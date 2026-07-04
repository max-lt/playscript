use std::rc::Rc;

use crate::value::Value;

/// Unary operators.
#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// Binary operators.
#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl BinaryOp {
    /// Source-level symbol, used in error messages.
    pub fn symbol(self) -> &'static str {

        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
        }
    }
}

/// An expression: evaluates to a value.
#[derive(Debug)]
pub enum Expr {
    Literal(Value),
    Variable(String),
    /// `f(a, b)` — functions are not values (yet), the callee is a name.
    Call { name: String, args: Vec<Expr> },
    /// `[a, b, c]` — array literal.
    Array(Vec<Expr>),
    /// `target[index]` — read access; chains left-to-right (`m[i][j]`).
    Index { target: Box<Expr>, index: Box<Expr> },
    Unary { op: UnaryOp, operand: Box<Expr> },
    Binary { op: BinaryOp, left: Box<Expr>, right: Box<Expr> },
}

/// A statement: executed for its effect.
#[derive(Debug)]
pub enum Stmt {
    /// `var x = expr` — declare in the current scope.
    Let { name: String, value: Expr },
    /// `x = expr` — update an existing binding, innermost scope first.
    Assign { name: String, value: Expr },
    /// `x[i] = expr` — write one element of an array variable (copy-on-write).
    IndexAssign { name: String, index: Expr, value: Expr },
    /// `{ ... }` — runs in its own scope.
    Block(Vec<Stmt>),
    /// Branches are always blocks (or another `If` for `else if`).
    If { condition: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>> },
    /// `while (cond) { ... }` — the body is always a block.
    While { condition: Expr, body: Box<Stmt> },
    /// `function name(params) { ... }` — registers the function.
    /// `Rc` so the definition can outlive the AST it was parsed from.
    Function(Rc<Function>),
    /// `return expr` — unwinds to the nearest enclosing call.
    Return(Expr),
    Expr(Expr),
}

/// A user-defined function. The body is always a block.
#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Stmt,
}
