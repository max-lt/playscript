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
        }
    }
}

impl std::error::Error for LangError {}
