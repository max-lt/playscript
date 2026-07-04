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
    Mod,
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
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
        }
    }
}

/// Logical operators — kept apart from `BinaryOp` because they
/// short-circuit: the right operand may never be evaluated at all.
#[derive(Debug, Clone, Copy)]
pub enum LogicalOp {
    And,
    Or,
}

impl LogicalOp {
    /// Source-level symbol, used in error messages.
    pub fn symbol(self) -> &'static str {

        match self {
            LogicalOp::And => "&&",
            LogicalOp::Or => "||",
        }
    }
}

/// An expression: evaluates to a value.
#[derive(Debug)]
pub enum Expr {
    Literal(Value),
    Variable(String),
    /// `callee(a, b)` — the callee is any expression: a variable, an array
    /// element, another call's result...
    Call { callee: Box<Expr>, args: Vec<Expr> },
    /// `x => expr` or `(a, b) => { ... }` — an anonymous function literal.
    Lambda(Rc<Function>),
    /// `[a, b, c]` — array literal.
    Array(Vec<Expr>),
    /// `target[index]` — read access; chains left-to-right (`m[i][j]`).
    Index { target: Box<Expr>, index: Box<Expr> },
    Unary { op: UnaryOp, operand: Box<Expr> },
    Binary { op: BinaryOp, left: Box<Expr>, right: Box<Expr> },
    Logical { op: LogicalOp, left: Box<Expr>, right: Box<Expr> },
}

/// A statement: executed for its effect.
#[derive(Debug)]
/// Statements that map to a source line carry it (`line`), so the interpreter
/// can stamp trace events with the line the visualizer should highlight.
pub enum Stmt {
    /// `var x = expr` — declare in the current scope.
    Let { name: String, value: Expr, line: usize },
    /// `x = expr` — update an existing binding, innermost scope first.
    Assign { name: String, value: Expr, line: usize },
    /// `x[i] = expr` — write one element of an array variable (copy-on-write).
    IndexAssign { name: String, index: Expr, value: Expr, line: usize },
    /// `{ ... }` — runs in its own scope.
    Block(Vec<Stmt>),
    /// Branches are always blocks (or another `If` for `else if`).
    If { condition: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>>, line: usize },
    /// `while (cond) { ... }` — the body is always a block.
    While { condition: Expr, body: Box<Stmt>, line: usize },
    /// `function name(params) { ... }` — registers the function.
    /// `Rc` so the definition can outlive the AST it was parsed from.
    Function(Rc<Function>),
    /// `return expr` — unwinds to the nearest enclosing call.
    Return { value: Expr, line: usize },
    Expr { expr: Expr, line: usize },
}

/// A user-defined function. The body is always a block; lambdas have no name.
#[derive(Debug)]
pub struct Function {
    pub name: Option<String>,
    pub params: Vec<String>,
    pub body: Stmt,
}
