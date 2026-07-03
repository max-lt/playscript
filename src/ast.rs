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
    /// `{ ... }` — runs in its own scope.
    Block(Vec<Stmt>),
    /// Branches are always blocks (or another `If` for `else if`).
    If { condition: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>> },
    Expr(Expr),
}
