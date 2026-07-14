use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Not, Rem, Sub};
use std::rc::Rc;

use crate::ast::Function;
use crate::error::{LangError, Result};
use crate::map::PlayMap;

/// A function value: shared code plus the locals it captured — by value —
/// when it was created. Globals are not captured; they stay live, which is
/// what keeps top-level recursion working.
#[derive(Debug)]
pub struct Closure {
    pub(crate) function: Rc<Function>,
    pub(crate) captures: HashMap<String, Value>,
}

/// A runtime value — the dynamic type of the language.
/// Numbers and bools are plain; strings are immutable and reference-counted,
/// so cloning any `Value` stays cheap (at most a refcount bump).
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Str(Rc<str>),
    /// Value semantics with copy-on-write: sharing the `Rc` is an invisible
    /// optimization, `Rc::make_mut` copies at the first shared write.
    /// No aliasing is ever observable, so no cycles can exist.
    Array(Rc<Vec<Value>>),
    /// Insertion-ordered map, value semantics via copy-on-write (like arrays).
    Map(Rc<PlayMap>),
    /// First-class user function (immutable, so sharing is invisible).
    Function(Rc<Closure>),
    /// A host builtin referenced as a value, e.g. `var p = print`.
    Builtin(&'static str),
}

// Equality is total and strict: values of different types are simply not
// equal. Numbers, bools, strings and arrays compare structurally; functions
// compare by identity (two lambdas with identical code are still distinct).
impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {

        match (self, other) {
            (Value::Number(l), Value::Number(r)) => l == r,
            (Value::Bool(l), Value::Bool(r)) => l == r,
            (Value::Str(l), Value::Str(r)) => l == r,
            (Value::Array(l), Value::Array(r)) => l == r,
            (Value::Map(l), Value::Map(r)) => l == r,
            (Value::Function(l), Value::Function(r)) => Rc::ptr_eq(l, r),
            (Value::Builtin(l), Value::Builtin(r)) => l == r,
            _ => false,
        }
    }
}

impl Value {
    /// Human-readable type name, used in error messages.
    pub fn type_name(&self) -> &'static str {

        match self {
            Value::Number(_) => "number",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Map(_) => "map",
            Value::Function(_) | Value::Builtin(_) => "function",
        }
    }

    /// If this is a map, its entries in insertion order. Lets code outside the
    /// crate (the wasm bridge) read maps without `PlayMap` being public.
    pub fn map_entries(&self) -> Option<impl Iterator<Item = (&Value, &Value)>> {

        match self {
            Value::Map(map) => Some(map.iter()),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Array(items) => {
                write!(f, "[")?;

                for (i, item) in items.iter().enumerate() {

                    if i > 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{item}")?;
                }

                write!(f, "]")
            }
            Value::Map(map) => {
                write!(f, "{{")?;

                for (i, (key, value)) in map.iter().enumerate() {

                    if i > 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{key}: {value}")?;
                }

                write!(f, "}}")
            }
            Value::Function(closure) => {

                match &closure.function.name {
                    Some(name) => write!(f, "<function {name}>"),
                    None => write!(f, "<lambda>"),
                }
            }
            Value::Builtin(name) => write!(f, "<builtin {name}>"),
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
            // `+` concatenates two strings — and only two strings; mixing
            // types requires an explicit str() conversion.
            (Value::Str(l), Value::Str(r)) => {
                let mut s = String::with_capacity(l.len() + r.len());
                s.push_str(&l);
                s.push_str(&r);
                Ok(Value::Str(s.into()))
            }
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

impl Rem for Value {
    type Output = Result<Value>;

    fn rem(self, rhs: Value) -> Result<Value> {

        match (self, rhs) {
            (Value::Number(_), Value::Number(r)) if r == 0.0 => Err(LangError::DivisionByZero),
            // f64 remainder: the result takes the sign of the dividend,
            // like JS: -7 % 3 == -1.
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l % r)),
            (l, r) => Err(invalid_binary("%", l, r)),
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

// Numbers order numerically, strings lexicographically. Anything else yields
// `None`, which the interpreter surfaces as a type error. (Equality is
// separate: `PartialEq` above makes values of different types not equal.)
impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {

        match (self, other) {
            (Value::Number(l), Value::Number(r)) => l.partial_cmp(r),
            (Value::Str(l), Value::Str(r)) => l.partial_cmp(r),
            _ => None,
        }
    }
}
