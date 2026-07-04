//! playscript — a minimal, deterministic, fully-metered scripting language
//! with a tree-walking interpreter.
//!
//! The public surface is deliberately tiny: build an [`Interpreter`], feed it
//! source with [`Interpreter::run`], and inspect the result plus the
//! operations consumed. Everything else is a crate-internal implementation
//! detail (lexer, parser, AST).

mod ast;
mod error;
mod interp;
mod lexer;
mod parser;
mod trace;
mod value;

pub use error::LangError;
pub use interp::{DEFAULT_FUEL_LIMIT, Interpreter};
pub use trace::{EventKind, TraceEvent};
pub use value::Value;
