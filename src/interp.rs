use std::cmp::Ordering;
use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, Stmt, UnaryOp};
use crate::error::{LangError, Result};
use crate::lexer::tokenize;
use crate::parser::Parser;
use crate::value::Value;

/// Default operation budget for one `run`.
pub const DEFAULT_FUEL_LIMIT: u64 = 1_000_000;

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

/// The engine: owns the program's memory and the fuel meter.
pub struct Interpreter {
    env: Environment,
    fuel: Fuel,
}

impl Interpreter {
    pub fn new(fuel_limit: u64) -> Self {
        Interpreter {
            env: Environment::default(),
            fuel: Fuel { used: 0, limit: fuel_limit },
        }
    }

    /// Parse and execute a whole program; return the last expression's value.
    /// The environment persists across calls (that is what makes the REPL
    /// stateful); the fuel budget resets on every call.
    pub fn run(&mut self, src: &str) -> Result<Option<Value>> {
        let tokens = tokenize(src)?;
        let program = Parser::new(tokens).parse_program()?;

        self.fuel.used = 0;

        let mut last = None;

        for stmt in &program {
            last = self.exec(stmt)?;
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

    fn exec(&mut self, stmt: &Stmt) -> Result<Option<Value>> {
        self.fuel.tick(1)?;

        match stmt {
            Stmt::Let { name, value } => {
                let v = self.eval(value)?;
                self.env.declare(name.clone(), v);
                Ok(None)
            }
            Stmt::Assign { name, value } => {
                let v = self.eval(value)?;
                self.env.assign(name, v)?;
                Ok(None)
            }
            Stmt::Block(stmts) => {
                self.env.push_scope();
                // Pop the scope even when a statement fails, so a REPL session
                // is not left inside a half-executed block.
                let result = stmts.iter().try_for_each(|stmt| self.exec(stmt).map(|_| ()));
                self.env.pop_scope();
                result?;
                Ok(None)
            }
            Stmt::If { condition, then_branch, else_branch } => {

                if self.eval_condition(condition)? {
                    self.exec(then_branch)?;
                } else if let Some(else_branch) = else_branch {
                    self.exec(else_branch)?;
                }

                Ok(None)
            }
            Stmt::While { condition, body } => {

                // The condition is re-evaluated (and re-charged) on every
                // iteration, so even an empty loop burns fuel — that is the
                // whole safety argument.
                while self.eval_condition(condition)? {
                    self.exec(body)?;
                }

                Ok(None)
            }
            Stmt::Expr(expr) => Ok(Some(self.eval(expr)?)),
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
