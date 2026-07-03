use crate::lexer::Token;

/// An expression: evaluates to a value.
#[derive(Debug)]
pub enum Expr {
    Number(f64),
    Variable(String),
    Unary { op: Token, operand: Box<Expr> },
    Binary { op: Token, left: Box<Expr>, right: Box<Expr> },
}

/// A statement: executed for its effect.
#[derive(Debug)]
pub enum Stmt {
    Let { name: String, value: Expr },
    Expr(Expr),
}
