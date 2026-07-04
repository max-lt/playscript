use std::fmt;

/// Every way a program can fail, from lexing to evaluation.
#[derive(Debug)]
pub enum LangError {
    UnexpectedChar(char),
    InvalidNumber(String),
    UnterminatedString,
    InvalidEscape(char),
    UnexpectedToken { expected: &'static str, found: String },
    UnexpectedEnd { expected: &'static str },
    UndefinedVariable(String),
    DivisionByZero,
    InvalidBinaryOp { op: &'static str, lhs: &'static str, rhs: &'static str },
    InvalidUnaryOp { op: &'static str, operand: &'static str },
    InvalidCondition { got: &'static str },
    OutOfFuel { limit: u64 },
    UndefinedFunction(String),
    WrongArity { function: String, expected: usize, got: usize },
    NoReturnValue { function: String },
    ReturnOutsideFunction,
    CallDepthExceeded { limit: usize },
}

/// Crate-wide result alias: `Result<T>` == `Result<T, LangError>`.
pub type Result<T> = std::result::Result<T, LangError>;

impl fmt::Display for LangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {
            LangError::UnexpectedChar(c) => write!(f, "unexpected character: {c:?}"),
            LangError::InvalidNumber(s) => write!(f, "invalid number: {s}"),
            LangError::UnterminatedString => write!(f, "unterminated string literal"),
            LangError::InvalidEscape(c) => write!(f, "invalid escape sequence: \\{c}"),
            LangError::UnexpectedToken { expected, found } => {
                write!(f, "expected {expected}, found '{found}'")
            }
            LangError::UnexpectedEnd { expected } => {
                write!(f, "expected {expected}, but reached end of input")
            }
            LangError::UndefinedVariable(name) => write!(f, "undefined variable: {name}"),
            LangError::DivisionByZero => write!(f, "division by zero"),
            LangError::InvalidBinaryOp { op, lhs, rhs } => {
                write!(f, "invalid operands to '{op}': {lhs} and {rhs}")
            }
            LangError::InvalidUnaryOp { op, operand } => {
                write!(f, "invalid operand to '{op}': {operand}")
            }
            LangError::InvalidCondition { got } => {
                write!(f, "condition must be a bool, got {got}")
            }
            LangError::OutOfFuel { limit } => {
                write!(f, "operation limit exceeded ({limit} ops)")
            }
            LangError::UndefinedFunction(name) => write!(f, "undefined function: {name}"),
            LangError::WrongArity { function, expected, got } => {
                write!(f, "function '{function}' takes {expected} argument(s), got {got}")
            }
            LangError::NoReturnValue { function } => {
                write!(f, "function '{function}' ended without returning a value")
            }
            LangError::ReturnOutsideFunction => write!(f, "'return' outside of a function"),
            LangError::CallDepthExceeded { limit } => {
                write!(f, "call depth limit exceeded ({limit})")
            }
        }
    }
}

impl std::error::Error for LangError {}
