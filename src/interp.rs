use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{BinaryOp, Expr, Function, LogicalOp, Stmt, UnaryOp};
use crate::error::{LangError, Result};
use crate::lexer::tokenize;
use crate::map::PlayMap;
use crate::parser::Parser;
use crate::trace::{EventKind, TraceEvent};
use crate::value::{Closure, Value};

/// Builtins reachable as plain names (unless shadowed by a user binding).
const BUILTIN_NAMES: &[&str] = &[
    "print", "getOperations", "getOperationsLimit", "str", "len", "array", "push", "has", "keys",
    "remove",
];

/// Default operation budget for one `run`.
pub const DEFAULT_FUEL_LIMIT: u64 = 1_000_000;

/// Cap on recorded trace events. Fuel bounds compute, but a fuel-exhausting
/// run can still produce ~100k events; recording stops here so the trace
/// stays serializable and renderable. Execution continues past the cap.
const TRACE_LIMIT: usize = 10_000;

/// Entering a function costs more than a plain node: a call sets up a scope,
/// binds arguments, tears everything down. First entry in the cost table.
const FUEL_CALL_COST: u64 = 10;

/// I/O is expensive — the Leek Wars lesson: `debug()` in a loop is how
/// leeks die. Charged on top of the call cost.
const FUEL_PRINT_COST: u64 = 100;

/// Maximum call depth. Fuel bounds *time*; this bounds *space* — recursion
/// grows the host's stack, which would overflow long before 1M ops run out.
/// The value assumes a normal (~8 MiB) native stack: each playscript call
/// nests several Rust frames, so on a much smaller stack this guard may fire
/// too late. Debug builds are the binding case (measured: 256 levels need
/// 2-3 MiB in debug, under 512 KiB in release — see examples/stack_probe.rs).
/// Removing the assumption entirely would mean an explicit heap call-stack.
const MAX_CALL_DEPTH: usize = 256;

/// Deterministic operation budget. Every AST node visited costs fuel, so
/// execution is finite by construction — `while (true) {}` included.
/// The count doubles as a logical clock: "after N ops" is a reproducible
/// instant, which is what a future replay/time-travel reader will seek on.
struct Fuel {
    used: u64,
    limit: u64,
}

impl Fuel {
    fn tick(&mut self, cost: u64) -> Result<()> {
        self.used += cost;

        if self.used > self.limit {
            return Err(LangError::OutOfFuel { limit: self.limit });
        }

        Ok(())
    }
}

/// The program's memory: a stack of lexical scopes, innermost last.
/// The global scope sits at the bottom and never pops.
struct Environment {
    scopes: Vec<HashMap<String, Value>>,
}

impl Default for Environment {
    fn default() -> Self {
        Environment { scopes: vec![HashMap::new()] }
    }
}

impl Environment {
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
        debug_assert!(!self.scopes.is_empty(), "the global scope must survive");
    }

    /// Read a variable, innermost scope first. Values clone cheaply:
    /// numbers and bools copy, strings bump a refcount.
    fn get(&self, name: &str) -> Result<Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .cloned()
            .ok_or_else(|| LangError::UndefinedVariable(name.to_string()))
    }

    /// `var` — declare in the innermost scope. Shadowing an outer binding is allowed.
    fn declare(&mut self, name: String, value: Value) {
        self.scopes
            .last_mut()
            .expect("the global scope always exists")
            .insert(name, value);
    }

    /// `=` — update an existing binding, innermost scope first.
    /// Assigning to a name that was never declared is an error.
    fn assign(&mut self, name: &str, value: Value) -> Result<()> {

        for scope in self.scopes.iter_mut().rev() {

            if let Some(slot) = scope.get_mut(name) {
                *slot = value;
                return Ok(());
            }
        }

        Err(LangError::UndefinedVariable(name.to_string()))
    }
}

/// How a statement finished: fell through normally (carrying the value of an
/// expression statement, for the REPL), or hit `return` and is unwinding
/// through blocks and loops towards the nearest enclosing call.
enum Flow {
    Value(Option<Value>),
    Return(Value),
}

/// The engine: owns the program's memory and the fuel meter. Functions are
/// ordinary values living in the environment.
pub struct Interpreter {
    env: Environment,
    fuel: Fuel,
    depth: usize,
    /// Source line of the statement currently executing; stamped onto trace
    /// events so the visualizer can highlight the matching line.
    current_line: usize,
    /// `Some` when tracing is enabled. Recording never ticks fuel, so a
    /// traced run and a plain run produce identical results and op counts.
    trace: Option<Vec<TraceEvent>>,
    /// Set when recording stopped at `TRACE_LIMIT`.
    trace_truncated: bool,
}

impl Interpreter {
    pub fn new(fuel_limit: u64) -> Self {
        Interpreter {
            env: Environment::default(),
            fuel: Fuel { used: 0, limit: fuel_limit },
            depth: 0,
            current_line: 1,
            trace: None,
            trace_truncated: false,
        }
    }

    /// Parse and execute a whole program; return the last expression's value.
    /// The environment and function registry persist across calls (that is
    /// what makes the REPL stateful); the fuel budget resets on every call.
    pub fn run(&mut self, src: &str) -> Result<Option<Value>> {
        let (tokens, lines) = tokenize(src)?;
        let program = Parser::new(tokens, lines).parse_program()?;

        self.fuel.used = 0;
        self.depth = 0;
        self.current_line = 1;
        self.trace_truncated = false;

        if let Some(trace) = self.trace.as_mut() {
            trace.clear();
        }

        let mut last = None;

        for stmt in &program {

            match self.exec(stmt)? {
                Flow::Value(value) => last = value,
                Flow::Return(_) => return Err(LangError::ReturnOutsideFunction),
            }
        }

        Ok(last)
    }

    /// Operations consumed by the last `run`.
    pub fn fuel_used(&self) -> u64 {
        self.fuel.used
    }

    /// Turn on execution tracing. The next `run` records the meaningful steps
    /// (assignments, calls, returns, branch decisions), each stamped with the
    /// op-clock value and the call depth.
    pub fn enable_tracing(&mut self) {
        self.trace = Some(Vec::new());
    }

    /// The trace recorded by the last `run`, if tracing is enabled.
    pub fn trace(&self) -> Option<&[TraceEvent]> {
        self.trace.as_deref()
    }

    /// Whether the last run hit `TRACE_LIMIT` and stopped recording events.
    pub fn trace_truncated(&self) -> bool {
        self.trace_truncated
    }

    fn tracing(&self) -> bool {
        self.trace.is_some()
    }

    /// Append an event, stamped with the current op-clock and depth.
    /// Recording never ticks fuel — a trace observes, it does not perturb.
    fn push_event(&mut self, kind: EventKind) {
        if !self.tracing() {
            return;
        }

        // Stop recording (but not executing) once the trace is full, so the
        // trace stays bounded even when the program runs to the fuel limit.
        if self.trace.as_ref().unwrap().len() >= TRACE_LIMIT {
            self.trace_truncated = true;
            return;
        }

        self.trace.as_mut().unwrap().push(TraceEvent {
            op: self.fuel.used,
            depth: self.depth,
            line: self.current_line,
            kind,
        });
    }

    fn eval(&mut self, expr: &Expr) -> Result<Value> {
        // Every node visited costs one operation. Charging here, at the top,
        // means no expression — however deeply nested — escapes metering.
        self.fuel.tick(1)?;

        match expr {
            Expr::Literal(value) => Ok(value.clone()),
            Expr::Variable(name) => {

                if let Ok(value) = self.env.get(name) {
                    return Ok(value);
                }

                // Unshadowed builtins resolve as values: `var p = print`.
                BUILTIN_NAMES
                    .iter()
                    .find(|&&builtin| builtin == name)
                    .map(|&builtin| Value::Builtin(builtin))
                    .ok_or_else(|| LangError::UndefinedVariable(name.to_string()))
            }
            Expr::Lambda(function) => self.make_closure(function),
            Expr::Call { callee, args } => {

                match self.eval(callee)? {
                    Value::Function(closure) => self.call_closure(&closure, args),
                    Value::Builtin(builtin) => {
                        self.fuel.tick(FUEL_CALL_COST)?;
                        self.call_builtin(builtin, args)
                    }
                    other => {
                        Err(LangError::InvalidUnaryOp { op: "()", operand: other.type_name() })
                    }
                }
            }
            Expr::Array(items) => {
                // Building an array costs one op per element, on top of the
                // per-element evaluation — allocations are paid for.
                self.fuel.tick(items.len() as u64)?;

                let mut values = Vec::with_capacity(items.len());

                for item in items {
                    values.push(self.eval(item)?);
                }

                Ok(Value::Array(values.into()))
            }
            Expr::Map(entries) => {
                // One op per entry, like array literals.
                self.fuel.tick(entries.len() as u64)?;

                let mut map = PlayMap::default();

                for (key_expr, value_expr) in entries {
                    let key = self.eval(key_expr)?;
                    let value = self.eval(value_expr)?;
                    check_map_key(&key)?;
                    map.insert(key, value);
                }

                Ok(Value::Map(map.into()))
            }
            Expr::Index { target, index } => {
                let target = self.eval(target)?;
                let index = self.eval(index)?;

                match target {
                    Value::Array(items) => {
                        let i = as_index(&index, items.len())?;
                        Ok(items[i].clone())
                    }
                    Value::Map(map) => {
                        check_map_key(&index)?;
                        map.get(&index)
                            .cloned()
                            .ok_or_else(|| LangError::MissingKey(index.to_string()))
                    }
                    other => {
                        Err(LangError::InvalidUnaryOp { op: "[]", operand: other.type_name() })
                    }
                }
            }
            Expr::Unary { op, operand } => {
                let v = self.eval(operand)?;

                // The `Neg`/`Not` impls on `Value` do the type checking.
                match op {
                    UnaryOp::Neg => -v,
                    UnaryOp::Not => !v,
                }
            }
            Expr::Logical { op, left, right } => {
                let left = self.eval(left)?;

                let Value::Bool(left) = left else {
                    return Err(LangError::InvalidUnaryOp {
                        op: op.symbol(),
                        operand: left.type_name(),
                    });
                };

                // Short-circuit: when the left side decides, the right side
                // is never evaluated — and never charged.
                match (op, left) {
                    (LogicalOp::And, false) => return Ok(Value::Bool(false)),
                    (LogicalOp::Or, true) => return Ok(Value::Bool(true)),
                    _ => {}
                }

                match self.eval(right)? {
                    Value::Bool(right) => Ok(Value::Bool(right)),
                    other => Err(LangError::InvalidUnaryOp {
                        op: op.symbol(),
                        operand: other.type_name(),
                    }),
                }
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval(left)?;
                let r = self.eval(right)?;

                match op {
                    // The `std::ops` impls on `Value`: each returns a `Result`.
                    BinaryOp::Add => {
                        let result = (l + r)?;

                        // Concatenation allocates: charge one op per byte of
                        // the result, so fuel bounds memory too. Otherwise a
                        // doubling loop (s = s + s) would OOM the host well
                        // within an ops budget.
                        if let Value::Str(s) = &result {
                            self.fuel.tick(s.len() as u64)?;
                        }

                        Ok(result)
                    }
                    BinaryOp::Sub => l - r,
                    BinaryOp::Mul => l * r,
                    BinaryOp::Div => l / r,
                    BinaryOp::Mod => l % r,
                    // Equality is total: values of different types are not equal.
                    BinaryOp::Eq => Ok(Value::Bool(l == r)),
                    BinaryOp::Ne => Ok(Value::Bool(l != r)),
                    BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                        ordered(*op, l, r)
                    }
                }
            }
        }
    }

    /// Build a function value: snapshot the visible locals BY VALUE (cheap —
    /// values clone by refcount). Globals are not captured, they stay live.
    /// Capture-by-value keeps the no-aliasing invariant: a closure can never
    /// share mutable state with its surroundings.
    fn make_closure(&mut self, function: &Rc<Function>) -> Result<Value> {
        let mut captures = HashMap::new();

        for scope in &self.env.scopes[1..] {

            for (name, value) in scope {
                captures.insert(name.clone(), value.clone());
            }
        }

        // Creating a closure pays one op per captured binding.
        self.fuel.tick(captures.len() as u64)?;

        Ok(Value::Function(Rc::new(Closure { function: Rc::clone(function), captures })))
    }

    fn call_closure(&mut self, closure: &Rc<Closure>, args: &[Expr]) -> Result<Value> {
        self.fuel.tick(FUEL_CALL_COST)?;
        let function = &closure.function;

        if args.len() != function.params.len() {
            return Err(wrong_arity(&function_name(function), function.params.len(), args.len()));
        }

        // Arguments are evaluated in the caller's environment, before the switch.
        let mut values = Vec::with_capacity(args.len());

        for arg in args {
            values.push(self.eval(arg)?);
        }

        if self.tracing() {
            self.push_event(EventKind::Call {
                name: function_name(function),
                args: values.clone(),
            });
        }

        // The body moves `current_line` around as it runs; save the caller's
        // line to restore once the call returns.
        let caller_line = self.current_line;
        self.depth += 1;

        if self.depth > MAX_CALL_DEPTH {
            self.depth -= 1;
            return Err(LangError::CallDepthExceeded { limit: MAX_CALL_DEPTH });
        }

        // Lexical scoping: the body sees the globals, its captures and its
        // own scopes. The caller's locals are set aside for the duration of
        // the call — leaving them visible would be dynamic scoping, the
        // classic tree-walker trap.
        let caller_locals = self.env.scopes.split_off(1);

        // Fresh per-call copy of the captures; the function's own name binds
        // to itself so named functions can always recurse.
        let mut captures = closure.captures.clone();

        if let Some(name) = &function.name {
            captures.insert(name.clone(), Value::Function(Rc::clone(closure)));
        }

        self.env.scopes.push(captures);
        self.env.push_scope(); // parameters (may shadow captures)

        for (param, value) in function.params.iter().zip(values) {
            self.env.declare(param.clone(), value);
        }

        let outcome = self.exec(&function.body);

        self.env.scopes.truncate(1);
        self.env.scopes.extend(caller_locals);
        self.depth -= 1;

        let value = match outcome? {
            Flow::Return(value) => value,
            // No null in the language: a call must produce a value.
            Flow::Value(_) => {
                return Err(LangError::NoReturnValue { function: function_name(function) });
            }
        };

        // The Return event uses the callee's return line (current_line as the
        // return statement left it); then restore the caller's line.
        if self.tracing() {
            self.push_event(EventKind::Return {
                name: function_name(function),
                value: value.clone(),
            });
        }

        self.current_line = caller_line;
        Ok(value)
    }

    /// Native functions provided by the host. In the closed-world design
    /// these are the only doors out of the sandbox, so each one is an
    /// explicit, hand-carved case — deny by default.
    fn call_builtin(&mut self, name: &str, args: &[Expr]) -> Result<Value> {

        match name {
            "print" => {
                self.fuel.tick(FUEL_PRINT_COST)?;

                let [arg] = args else {
                    return Err(wrong_arity(name, 1, args.len()));
                };

                let value = self.eval(arg)?;
                println!("{value}");
                // No null: print passes its value through, so it can wrap
                // any expression (like Rust's dbg!).
                Ok(value)
            }
            "getOperations" => {
                let [] = args else {
                    return Err(wrong_arity(name, 0, args.len()));
                };

                Ok(Value::Number(self.fuel.used as f64))
            }
            "getOperationsLimit" => {
                let [] = args else {
                    return Err(wrong_arity(name, 0, args.len()));
                };

                Ok(Value::Number(self.fuel.limit as f64))
            }
            "str" => {
                let [arg] = args else {
                    return Err(wrong_arity(name, 1, args.len()));
                };

                // Explicit conversion — the strict counterpart of coercion:
                // "n = " + str(n) instead of JS's silent "n = " + n.
                match self.eval(arg)? {
                    already @ Value::Str(_) => Ok(already),
                    other => Ok(Value::Str(other.to_string().into())),
                }
            }
            "len" => {
                let [arg] = args else {
                    return Err(wrong_arity(name, 1, args.len()));
                };

                match self.eval(arg)? {
                    // Unicode scalar count, not bytes: len("héllo") == 5.
                    Value::Str(s) => Ok(Value::Number(s.chars().count() as f64)),
                    Value::Array(items) => Ok(Value::Number(items.len() as f64)),
                    Value::Map(map) => Ok(Value::Number(map.len() as f64)),
                    other => Err(LangError::InvalidUnaryOp { op: "len", operand: other.type_name() }),
                }
            }
            "array" => {
                let [count, fill] = args else {
                    return Err(wrong_arity(name, 2, args.len()));
                };

                let count = self.eval(count)?;
                let fill = self.eval(fill)?;

                let Value::Number(n) = count else {
                    return Err(LangError::InvalidIndex(count.type_name().to_string()));
                };

                if n.fract() != 0.0 || n < 0.0 {
                    return Err(LangError::InvalidIndex(n.to_string()));
                }

                // Pay for the allocation before making it.
                self.fuel.tick(n as u64)?;

                Ok(Value::Array(vec![fill; n as usize].into()))
            }
            "push" => {
                let [array, item] = args else {
                    return Err(wrong_arity(name, 2, args.len()));
                };

                let array = self.eval(array)?;
                let item = self.eval(item)?;

                let Value::Array(items) = array else {
                    return Err(LangError::InvalidUnaryOp { op: "push", operand: array.type_name() });
                };

                // Value semantics: push returns a new array. Building one
                // element at a time is O(n²) ops — array(n, fill) plus index
                // writes is the cheap way to build big.
                self.fuel.tick(items.len() as u64 + 1)?;

                let mut items = items.as_ref().clone();
                items.push(item);
                Ok(Value::Array(items.into()))
            }
            "has" => {
                let [map, key] = args else {
                    return Err(wrong_arity(name, 2, args.len()));
                };

                let map = self.eval(map)?;
                let key = self.eval(key)?;
                check_map_key(&key)?;

                match map {
                    Value::Map(map) => Ok(Value::Bool(map.get(&key).is_some())),
                    other => Err(LangError::InvalidUnaryOp { op: "has", operand: other.type_name() }),
                }
            }
            "keys" => {
                let [map] = args else {
                    return Err(wrong_arity(name, 1, args.len()));
                };

                match self.eval(map)? {
                    Value::Map(map) => {
                        // Building the key array costs one op per key.
                        self.fuel.tick(map.len() as u64)?;
                        let keys: Vec<Value> = map.iter().map(|(k, _)| k.clone()).collect();
                        Ok(Value::Array(keys.into()))
                    }
                    other => Err(LangError::InvalidUnaryOp { op: "keys", operand: other.type_name() }),
                }
            }
            "remove" => {
                let [map, key] = args else {
                    return Err(wrong_arity(name, 2, args.len()));
                };

                let map = self.eval(map)?;
                let key = self.eval(key)?;
                check_map_key(&key)?;

                match map {
                    Value::Map(map) => {
                        // Value semantics: remove returns a new map, one op per
                        // surviving entry.
                        self.fuel.tick(map.len() as u64)?;
                        Ok(Value::Map(map.without(&key).into()))
                    }
                    other => {
                        Err(LangError::InvalidUnaryOp { op: "remove", operand: other.type_name() })
                    }
                }
            }
            _ => Err(LangError::UndefinedFunction(name.to_string())),
        }
    }

    fn exec(&mut self, stmt: &Stmt) -> Result<Flow> {
        self.fuel.tick(1)?;

        match stmt {
            Stmt::Let { name, value, line } => {
                self.current_line = *line;
                let v = self.eval(value)?;

                if self.tracing() {
                    self.push_event(EventKind::Assign { target: name.clone(), value: v.clone() });
                }

                self.env.declare(name.clone(), v);
                Ok(Flow::Value(None))
            }
            Stmt::Assign { name, value, line } => {
                self.current_line = *line;
                let v = self.eval(value)?;

                if self.tracing() {
                    self.push_event(EventKind::Assign { target: name.clone(), value: v.clone() });
                }

                self.env.assign(name, v)?;
                Ok(Flow::Value(None))
            }
            Stmt::IndexAssign { name, index, value, line } => {
                self.current_line = *line;
                let index = self.eval(index)?;
                let value = self.eval(value)?;

                // Capture the trace data before `value` is moved into the
                // array, and before the mutable borrow below (which would
                // conflict with recording). Dropped untouched on any error.
                let traced = self
                    .tracing()
                    .then(|| (format!("{name}[{index}]"), value.clone()));

                // Locate the binding like `=` does, innermost scope first.
                let slot = self
                    .env
                    .scopes
                    .iter_mut()
                    .rev()
                    .find_map(|scope| scope.get_mut(name))
                    .ok_or_else(|| LangError::UndefinedVariable(name.clone()))?;

                // Copy-on-write for both containers: writing to a shared value
                // pays for the copy it triggers; an unshared one is a plain store.
                match slot {
                    Value::Array(items) => {
                        let i = as_index(&index, items.len())?;
                        let cost = if Rc::strong_count(items) > 1 { items.len() as u64 } else { 1 };
                        self.fuel.tick(cost)?;
                        Rc::make_mut(items)[i] = value;
                    }
                    Value::Map(map) => {
                        check_map_key(&index)?;
                        let cost = if Rc::strong_count(map) > 1 { map.len() as u64 } else { 1 };
                        self.fuel.tick(cost)?;
                        Rc::make_mut(map).insert(index.clone(), value);
                    }
                    other => {
                        return Err(LangError::InvalidUnaryOp { op: "[]=", operand: other.type_name() });
                    }
                }

                if let Some((target, value)) = traced {
                    self.push_event(EventKind::Assign { target, value });
                }

                Ok(Flow::Value(None))
            }
            Stmt::Function(function) => {
                // A function statement is just a variable binding to a
                // function value; redefining is allowed (REPL-friendly).
                let value = self.make_closure(function)?;
                let name = function.name.clone().expect("function statements are named");
                self.env.declare(name, value);
                Ok(Flow::Value(None))
            }
            Stmt::Return { value, line } => {
                self.current_line = *line;
                let result = self.eval(value)?;
                Ok(Flow::Return(result))
            }
            Stmt::Block(stmts) => {
                self.env.push_scope();

                let mut flow = Flow::Value(None);

                for stmt in stmts {

                    match self.exec(stmt) {
                        // A `return` stops the block and keeps unwinding.
                        Ok(Flow::Return(value)) => {
                            flow = Flow::Return(value);
                            break;
                        }
                        Ok(Flow::Value(_)) => {}
                        Err(e) => {
                            // Pop even on failure, so a REPL session is not
                            // left inside a half-executed block.
                            self.env.pop_scope();
                            return Err(e);
                        }
                    }
                }

                self.env.pop_scope();
                Ok(flow)
            }
            Stmt::If { condition, then_branch, else_branch, line } => {
                self.current_line = *line;
                let flag = self.eval_condition(condition)?;

                if self.tracing() {
                    self.push_event(EventKind::Branch { construct: "if", value: flag });
                }

                if flag {
                    self.exec(then_branch)
                } else if let Some(else_branch) = else_branch {
                    self.exec(else_branch)
                } else {
                    Ok(Flow::Value(None))
                }
            }
            Stmt::While { condition, body, line } => {

                // The condition is re-evaluated (and re-charged) on every
                // iteration, so even an empty loop burns fuel — that is the
                // whole safety argument.
                loop {
                    self.current_line = *line;
                    let flag = self.eval_condition(condition)?;

                    if self.tracing() {
                        self.push_event(EventKind::Branch { construct: "while", value: flag });
                    }

                    if !flag {
                        break;
                    }

                    if let Flow::Return(value) = self.exec(body)? {
                        return Ok(Flow::Return(value));
                    }
                }

                Ok(Flow::Value(None))
            }
            Stmt::Expr { expr, line } => {
                self.current_line = *line;
                let value = self.eval(expr)?;

                if self.tracing() {
                    self.push_event(EventKind::Expr { value: value.clone() });
                }

                Ok(Flow::Value(Some(value)))
            }
        }
    }

    /// Conditions are strict: anything but a bool is a type error.
    fn eval_condition(&mut self, condition: &Expr) -> Result<bool> {

        match self.eval(condition)? {
            Value::Bool(flag) => Ok(flag),
            other => Err(LangError::InvalidCondition { got: other.type_name() }),
        }
    }
}

fn wrong_arity(function: &str, expected: usize, got: usize) -> LangError {
    LangError::WrongArity { function: function.to_string(), expected, got }
}

fn function_name(function: &Function) -> String {
    function.name.clone().unwrap_or_else(|| "<lambda>".to_string())
}

/// Map keys must be primitives (number, bool, string) and not NaN, so that
/// equality and hashing are total and deterministic.
fn check_map_key(key: &Value) -> Result<()> {

    match key {
        Value::Number(n) if n.is_nan() => Err(LangError::InvalidMapKey { got: "NaN" }),
        Value::Number(_) | Value::Bool(_) | Value::Str(_) => Ok(()),
        other => Err(LangError::InvalidMapKey { got: other.type_name() }),
    }
}

/// Validate an evaluated index against an array length: it must be a
/// non-negative integer number, strictly below `len`.
fn as_index(value: &Value, len: usize) -> Result<usize> {
    let Value::Number(n) = value else {
        return Err(LangError::InvalidIndex(value.type_name().to_string()));
    };

    if n.fract() != 0.0 || *n < 0.0 {
        return Err(LangError::InvalidIndex(n.to_string()));
    }

    let index = *n as usize;

    if index >= len {
        return Err(LangError::IndexOutOfBounds { index, len });
    }

    Ok(index)
}

/// Ordering comparisons. `PartialOrd` on `Value` yields `None` for anything
/// but two numbers; we surface that as a type error.
fn ordered(op: BinaryOp, l: Value, r: Value) -> Result<Value> {
    let Some(ord) = l.partial_cmp(&r) else {
        return Err(LangError::InvalidBinaryOp {
            op: op.symbol(),
            lhs: l.type_name(),
            rhs: r.type_name(),
        });
    };

    let result = match op {
        BinaryOp::Lt => ord == Ordering::Less,
        BinaryOp::Le => ord != Ordering::Greater,
        BinaryOp::Gt => ord == Ordering::Greater,
        BinaryOp::Ge => ord != Ordering::Less,
        _ => unreachable!("ordered() is only called for ordering operators"),
    };

    Ok(Value::Bool(result))
}
