use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{BinaryOp, Expr, Function, Stmt, UnaryOp};
use crate::error::{LangError, Result};
use crate::lexer::tokenize;
use crate::parser::Parser;
use crate::value::Value;

/// Default operation budget for one `run`.
pub const DEFAULT_FUEL_LIMIT: u64 = 1_000_000;

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
/// too late. Removing the assumption would mean an explicit heap call-stack.
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

    /// Read a variable, innermost scope first.
    fn get(&self, name: &str) -> Result<Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .copied()
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

/// The engine: owns the program's memory, the fuel meter and the
/// function registry.
pub struct Interpreter {
    env: Environment,
    fuel: Fuel,
    functions: HashMap<String, Rc<Function>>,
    depth: usize,
}

impl Interpreter {
    pub fn new(fuel_limit: u64) -> Self {
        Interpreter {
            env: Environment::default(),
            fuel: Fuel { used: 0, limit: fuel_limit },
            functions: HashMap::new(),
            depth: 0,
        }
    }

    /// Parse and execute a whole program; return the last expression's value.
    /// The environment and function registry persist across calls (that is
    /// what makes the REPL stateful); the fuel budget resets on every call.
    pub fn run(&mut self, src: &str) -> Result<Option<Value>> {
        let tokens = tokenize(src)?;
        let program = Parser::new(tokens).parse_program()?;

        self.fuel.used = 0;
        self.depth = 0;

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

    fn eval(&mut self, expr: &Expr) -> Result<Value> {
        // Every node visited costs one operation. Charging here, at the top,
        // means no expression — however deeply nested — escapes metering.
        self.fuel.tick(1)?;

        match expr {
            Expr::Literal(value) => Ok(*value),
            Expr::Variable(name) => self.env.get(name),
            Expr::Call { name, args } => self.call(name, args),
            Expr::Unary { op, operand } => {
                let v = self.eval(operand)?;

                // The `Neg`/`Not` impls on `Value` do the type checking.
                match op {
                    UnaryOp::Neg => -v,
                    UnaryOp::Not => !v,
                }
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval(left)?;
                let r = self.eval(right)?;

                match op {
                    // The `std::ops` impls on `Value`: each returns a `Result`.
                    BinaryOp::Add => l + r,
                    BinaryOp::Sub => l - r,
                    BinaryOp::Mul => l * r,
                    BinaryOp::Div => l / r,
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

    fn call(&mut self, name: &str, args: &[Expr]) -> Result<Value> {
        self.fuel.tick(FUEL_CALL_COST)?;

        // User-defined functions are looked up first: shadowing a builtin
        // is allowed, like shadowing a variable.
        if let Some(function) = self.functions.get(name).cloned() {
            return self.call_user(&function, args);
        }

        self.call_builtin(name, args)
    }

    fn call_user(&mut self, function: &Function, args: &[Expr]) -> Result<Value> {

        if args.len() != function.params.len() {
            return Err(wrong_arity(&function.name, function.params.len(), args.len()));
        }

        // Arguments are evaluated in the caller's environment, before the switch.
        let mut values = Vec::with_capacity(args.len());

        for arg in args {
            values.push(self.eval(arg)?);
        }

        self.depth += 1;

        if self.depth > MAX_CALL_DEPTH {
            self.depth -= 1;
            return Err(LangError::CallDepthExceeded { limit: MAX_CALL_DEPTH });
        }

        // Lexical scoping: the body sees the globals and its own scope only.
        // The caller's locals are set aside for the duration of the call —
        // leaving them visible would be dynamic scoping, the classic
        // tree-walker trap.
        let caller_locals = self.env.scopes.split_off(1);
        self.env.push_scope();

        for (param, value) in function.params.iter().zip(values) {
            self.env.declare(param.clone(), value);
        }

        let outcome = self.exec(&function.body);

        self.env.scopes.truncate(1);
        self.env.scopes.extend(caller_locals);
        self.depth -= 1;

        match outcome? {
            Flow::Return(value) => Ok(value),
            // No null in the language: a call must produce a value.
            Flow::Value(_) => Err(LangError::NoReturnValue { function: function.name.clone() }),
        }
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
            _ => Err(LangError::UndefinedFunction(name.to_string())),
        }
    }

    fn exec(&mut self, stmt: &Stmt) -> Result<Flow> {
        self.fuel.tick(1)?;

        match stmt {
            Stmt::Let { name, value } => {
                let v = self.eval(value)?;
                self.env.declare(name.clone(), v);
                Ok(Flow::Value(None))
            }
            Stmt::Assign { name, value } => {
                let v = self.eval(value)?;
                self.env.assign(name, v)?;
                Ok(Flow::Value(None))
            }
            Stmt::Function(function) => {
                // Registration only; redefining is allowed (REPL-friendly).
                self.functions.insert(function.name.clone(), Rc::clone(function));
                Ok(Flow::Value(None))
            }
            Stmt::Return(expr) => {
                let value = self.eval(expr)?;
                Ok(Flow::Return(value))
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
            Stmt::If { condition, then_branch, else_branch } => {

                if self.eval_condition(condition)? {
                    self.exec(then_branch)
                } else if let Some(else_branch) = else_branch {
                    self.exec(else_branch)
                } else {
                    Ok(Flow::Value(None))
                }
            }
            Stmt::While { condition, body } => {

                // The condition is re-evaluated (and re-charged) on every
                // iteration, so even an empty loop burns fuel — that is the
                // whole safety argument.
                while self.eval_condition(condition)? {

                    if let Flow::Return(value) = self.exec(body)? {
                        return Ok(Flow::Return(value));
                    }
                }

                Ok(Flow::Value(None))
            }
            Stmt::Expr(expr) => Ok(Flow::Value(Some(self.eval(expr)?))),
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
