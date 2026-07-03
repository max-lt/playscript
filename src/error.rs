use std::fmt;

/// Every way a program can fail, from lexing to evaluation.
#[derive(Debug)]
pub enum LangError {
    UnexpectedChar(char),
    InvalidNumber(String),
    UnexpectedToken { expected: &'static str, found: String },
    UnexpectedEnd { expected: &'static str },
    UndefinedVariable(String),
    DivisionByZero,
    InvalidBinaryOp { op: &'static str, lhs: &'static str, rhs: &'static str },
    InvalidUnaryOp { op: &'static str, operand: &'static str },
    InvalidCondition { got: &'static str },
    OutOfFuel { limit: u64 },
}

/// Crate-wide result alias: `Result<T>` == `Result<T, LangError>`.
pub type Result<T> = std::result::Result<T, LangError>;

impl fmt::Display for LangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {
            LangError::UnexpectedChar(c) => write!(f, "unexpected character: {c:?}"),
            LangError::InvalidNumber(s) => write!(f, "invalid number: {s}"),
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
        }
    }
}

impl std::error::Error for LangError {}
