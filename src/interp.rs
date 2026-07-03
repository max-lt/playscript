use std::cmp::Ordering;
use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, Stmt, UnaryOp};
use crate::error::{LangError, Result};
use crate::value::Value;

/// The program's memory: a stack of lexical scopes, innermost last.
/// The global scope sits at the bottom and never pops.
pub struct Environment {
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
    pub fn get(&self, name: &str) -> Result<Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .copied()
            .ok_or_else(|| LangError::UndefinedVariable(name.to_string()))
    }

    /// `var` — declare in the innermost scope. Shadowing an outer binding is allowed.
    pub fn declare(&mut self, name: String, value: Value) {
        self.scopes
            .last_mut()
            .expect("the global scope always exists")
            .insert(name, value);
    }

    /// `=` — update an existing binding, innermost scope first.
    /// Assigning to a name that was never declared is an error.
    pub fn assign(&mut self, name: &str, value: Value) -> Result<()> {

        for scope in self.scopes.iter_mut().rev() {

            if let Some(slot) = scope.get_mut(name) {
                *slot = value;
                return Ok(());
            }
        }

        Err(LangError::UndefinedVariable(name.to_string()))
    }
}

/// Evaluate an expression to a value (tree-walking, recursive like the AST).
fn eval(expr: &Expr, env: &Environment) -> Result<Value> {

    match expr {
        Expr::Literal(value) => Ok(*value),
        Expr::Variable(name) => env.get(name),
        Expr::Unary { op, operand } => {
            let v = eval(operand, env)?;

            // The `Neg`/`Not` impls on `Value` do the type checking.
            match op {
                UnaryOp::Neg => -v,
                UnaryOp::Not => !v,
            }
        }
        Expr::Binary { op, left, right } => {
            let l = eval(left, env)?;
            let r = eval(right, env)?;

            match op {
                // The `std::ops` impls on `Value`: each returns a `Result`.
                BinaryOp::Add => l + r,
                BinaryOp::Sub => l - r,
                BinaryOp::Mul => l * r,
                BinaryOp::Div => l / r,
                // Equality is total: values of different types are not equal.
                BinaryOp::Eq => Ok(Value::Bool(l == r)),
                BinaryOp::Ne => Ok(Value::Bool(l != r)),
                BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => ordered(*op, l, r),
            }
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

/// Execute a statement. A `var` binding yields nothing; an expression yields its value.
pub fn exec(stmt: &Stmt, env: &mut Environment) -> Result<Option<Value>> {

    match stmt {
        Stmt::Let { name, value } => {
            let v = eval(value, env)?;
            env.declare(name.clone(), v);
            Ok(None)
        }
        Stmt::Assign { name, value } => {
            let v = eval(value, env)?;
            env.assign(name, v)?;
            Ok(None)
        }
        Stmt::Block(stmts) => {
            env.push_scope();
            // Pop the scope even when a statement fails, so a REPL session
            // is not left inside a half-executed block.
            let result = stmts.iter().try_for_each(|stmt| exec(stmt, env).map(|_| ()));
            env.pop_scope();
            result?;
            Ok(None)
        }
        Stmt::If { condition, then_branch, else_branch } => {
            let cond = eval(condition, env)?;

            let Value::Bool(flag) = cond else {
                return Err(LangError::InvalidCondition { got: cond.type_name() });
            };

            if flag {
                exec(then_branch, env)?;
            } else if let Some(else_branch) = else_branch {
                exec(else_branch, env)?;
            }

            Ok(None)
        }
        Stmt::Expr(expr) => Ok(Some(eval(expr, env)?)),
    }
}
