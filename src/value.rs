use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Not, Sub};

use crate::error::{LangError, Result};

/// A runtime value — the dynamic type of the language.
/// Everything is `Copy` for now; that ends the day we add strings or arrays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
}

impl Value {
    /// Human-readable type name, used in error messages.
    pub fn type_name(&self) -> &'static str {

        match self {
            Value::Number(_) => "number",
            Value::Bool(_) => "bool",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
        }
    }
}

fn invalid_binary(op: &'static str, l: Value, r: Value) -> LangError {
    LangError::InvalidBinaryOp { op, lhs: l.type_name(), rhs: r.type_name() }
}

// Arithmetic as real Rust operator overloads. `Output` is a `Result`
// because the guest language is dynamically typed: `true + 1` is a
// perfectly parseable program whose failure only exists at runtime.

impl Add for Value {
    type Output = Result<Value>;

    fn add(self, rhs: Value) -> Result<Value> {

        match (self, rhs) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
            (l, r) => Err(invalid_binary("+", l, r)),
        }
    }
}

impl Sub for Value {
    type Output = Result<Value>;

    fn sub(self, rhs: Value) -> Result<Value> {

        match (self, rhs) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l - r)),
            (l, r) => Err(invalid_binary("-", l, r)),
        }
    }
}

impl Mul for Value {
    type Output = Result<Value>;

    fn mul(self, rhs: Value) -> Result<Value> {

        match (self, rhs) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
            (l, r) => Err(invalid_binary("*", l, r)),
        }
    }
}

impl Div for Value {
    type Output = Result<Value>;

    fn div(self, rhs: Value) -> Result<Value> {

        match (self, rhs) {
            (Value::Number(_), Value::Number(r)) if r == 0.0 => Err(LangError::DivisionByZero),
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l / r)),
            (l, r) => Err(invalid_binary("/", l, r)),
        }
    }
}

impl Neg for Value {
    type Output = Result<Value>;

    fn neg(self) -> Result<Value> {

        match self {
            Value::Number(n) => Ok(Value::Number(-n)),
            v => Err(LangError::InvalidUnaryOp { op: "-", operand: v.type_name() }),
        }
    }
}

impl Not for Value {
    type Output = Result<Value>;

    fn not(self) -> Result<Value> {

        match self {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            v => Err(LangError::InvalidUnaryOp { op: "!", operand: v.type_name() }),
        }
    }
}

// Only numbers have an ordering. Anything else yields `None`, which the
// interpreter surfaces as a type error. (Equality is separate: `PartialEq`
// above makes values of different types simply not equal.)
impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {

        match (self, other) {
            (Value::Number(l), Value::Number(r)) => l.partial_cmp(r),
            _ => None,
        }
    }
}
