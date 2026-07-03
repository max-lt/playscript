// Shared helpers for the integration tests. Living under `common/` (not
// `common.rs`) keeps cargo from treating it as its own test binary.
#![allow(dead_code)]

pub use playscript::{DEFAULT_FUEL_LIMIT, Interpreter, LangError, Value};

/// Run a program with the default budget; return the raw outcome.
pub fn run(src: &str) -> Result<Option<Value>, LangError> {
    Interpreter::new(DEFAULT_FUEL_LIMIT).run(src)
}

/// Run a program expected to succeed with a value; panic otherwise.
pub fn eval(src: &str) -> Value {

    match run(src) {
        Ok(Some(value)) => value,
        Ok(None) => panic!("program produced no value: {src:?}"),
        Err(e) => panic!("program errored: {src:?} -> {e}"),
    }
}

/// Run a program expected to fail; return the error.
pub fn eval_err(src: &str) -> LangError {

    match run(src) {
        Err(e) => e,
        Ok(value) => panic!("expected an error from {src:?}, got {value:?}"),
    }
}

/// Operations consumed while running `src` (which must succeed).
pub fn ops(src: &str) -> u64 {
    let mut interp = Interpreter::new(DEFAULT_FUEL_LIMIT);
    interp.run(src).expect("program should succeed");
    interp.fuel_used()
}

pub fn num(n: f64) -> Value {
    Value::Number(n)
}

pub fn boolean(b: bool) -> Value {
    Value::Bool(b)
}
