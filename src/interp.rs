use std::collections::HashMap;

use crate::ast::{Expr, Stmt};
use crate::error::{LangError, Result};
use crate::lexer::Token;

/// The program's memory: variable name -> value.
#[derive(Default)]
pub struct Environment {
    vars: HashMap<String, f64>,
}

impl Environment {
    pub fn get(&self, name: &str) -> Result<f64> {
        self.vars
            .get(name)
            .copied()
            .ok_or_else(|| LangError::UndefinedVariable(name.to_string()))
    }

    pub fn set(&mut self, name: String, value: f64) {
        self.vars.insert(name, value);
    }
}

/// Evaluate an expression to a value (tree-walking, recursive like the AST).
fn eval(expr: &Expr, env: &Environment) -> Result<f64> {

    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Variable(name) => env.get(name),
        Expr::Unary { op, operand } => {
            let v = eval(operand, env)?;

            match op {
                Token::Minus => Ok(-v),
                _ => unreachable!("parser only emits '-' as a unary operator"),
            }
        }
        Expr::Binary { op, left, right } => {
            let l = eval(left, env)?;
            let r = eval(right, env)?;

            match op {
                Token::Plus => Ok(l + r),
                Token::Minus => Ok(l - r),
                Token::Star => Ok(l * r),
                Token::Slash if r == 0.0 => Err(LangError::DivisionByZero),
                Token::Slash => Ok(l / r),
                _ => unreachable!("parser does not emit this binary operator"),
            }
        }
    }
}

/// Execute a statement. A `var` binding yields nothing; an expression yields its value.
pub fn exec(stmt: &Stmt, env: &mut Environment) -> Result<Option<f64>> {

    match stmt {
        Stmt::Let { name, value } => {
            let v = eval(value, env)?;
            env.set(name.clone(), v);
            Ok(None)
        }
        Stmt::Expr(expr) => Ok(Some(eval(expr, env)?)),
    }
}
