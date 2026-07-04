use std::rc::Rc;

use crate::ast::{BinaryOp, Expr, Function, Stmt, UnaryOp};
use crate::error::{LangError, Result};
use crate::lexer::Token;
use crate::value::Value;

// Recursive descent. The grammar encodes operator precedence by call order:
//   program    := statement*
//   statement  := "var" IDENT "=" expr
//                | expr ("=" expr)?          (assignment iff expr is an lvalue)
//                | "function" IDENT "(" (IDENT ("," IDENT)*)? ")" block
//                | "return" expr
//                | "if" "(" expr ")" block ("else" (block | if))?
//                | "while" "(" expr ")" block
//                | block
//                | expr
//   block      := "{" statement* "}"
//   expr       := equality
//   equality   := comparison (("==" | "!=") comparison)*
//   comparison := term (("<" | "<=" | ">" | ">=") term)*
//   term       := factor (("+" | "-") factor)*
//   factor     := unary (("*" | "/") unary)*
//   unary      := ("-" | "!") unary | postfix
//   postfix    := primary ("[" expr "]")*
//   primary    := NUMBER | STRING | "true" | "false" | IDENT ("(" args ")")?
//                | "[" args "]" | "(" expr ")"
//   args       := (expr ("," expr)*)?

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

// Build an "expected X, found Y" error (or "reached end of input").
fn expected(what: &'static str, found: Option<Token>) -> LangError {

    match found {
        Some(tok) => LangError::UnexpectedToken { expected: what, found: tok.to_string() },
        None => LangError::UnexpectedEnd { expected: what },
    }
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        tok
    }

    /// Consume the next token if it is exactly `want`, error otherwise.
    fn expect(&mut self, want: Token, what: &'static str) -> Result<()> {

        match self.advance() {
            Some(tok) if tok == want => Ok(()),
            other => Err(expected(what, other)),
        }
    }

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>> {
        let mut stmts = Vec::new();

        while self.peek().is_some() {
            stmts.push(self.statement()?);

            // Optional semicolon between statements.
            if matches!(self.peek(), Some(Token::Semicolon)) {
                self.advance();
            }
        }

        Ok(stmts)
    }

    fn statement(&mut self) -> Result<Stmt> {

        match self.peek() {
            Some(Token::Var) => self.let_statement(),
            Some(Token::Function) => self.function_statement(),
            Some(Token::Return) => self.return_statement(),
            Some(Token::If) => self.if_statement(),
            Some(Token::While) => self.while_statement(),
            Some(Token::LBrace) => self.block(),
            _ => self.expr_or_assign_statement(),
        }
    }

    // Parse an expression; if '=' follows, reinterpret the expression as an
    // assignment target — a variable, or one indexing of a variable.
    fn expr_or_assign_statement(&mut self) -> Result<Stmt> {
        let expr = self.expr()?;

        if !matches!(self.peek(), Some(Token::Equals)) {
            return Ok(Stmt::Expr(expr));
        }

        self.advance(); // consume '='
        let value = self.expr()?;

        match expr {
            Expr::Variable(name) => Ok(Stmt::Assign { name, value }),
            Expr::Index { target, index } => {

                if let Expr::Variable(name) = *target {
                    return Ok(Stmt::IndexAssign { name, index: *index, value });
                }

                Err(LangError::InvalidAssignTarget)
            }
            _ => Err(LangError::InvalidAssignTarget),
        }
    }

    fn let_statement(&mut self) -> Result<Stmt> {
        self.advance(); // consume 'var'

        let name = match self.advance() {
            Some(Token::Ident(name)) => name,
            other => return Err(expected("a variable name", other)),
        };

        self.expect(Token::Equals, "'='")?;

        let value = self.expr()?;
        Ok(Stmt::Let { name, value })
    }

    fn block(&mut self) -> Result<Stmt> {
        self.advance(); // consume '{'

        let mut stmts = Vec::new();

        loop {

            match self.peek() {
                None => return Err(expected("'}'", None)),
                Some(Token::RBrace) => {
                    self.advance();
                    break;
                }
                _ => {
                    stmts.push(self.statement()?);

                    // Optional semicolon between statements.
                    if matches!(self.peek(), Some(Token::Semicolon)) {
                        self.advance();
                    }
                }
            }
        }

        Ok(Stmt::Block(stmts))
    }

    // Braces are mandatory around branches, which rules out the classic
    // "dangling else" ambiguity. `else if` chains by recursing into `if`.
    fn if_statement(&mut self) -> Result<Stmt> {
        self.advance(); // consume 'if'

        self.expect(Token::LParen, "'('")?;
        let condition = self.expr()?;
        self.expect(Token::RParen, "')'")?;

        let then_branch = match self.peek() {
            Some(Token::LBrace) => Box::new(self.block()?),
            _ => return Err(expected("'{'", self.advance())),
        };

        let else_branch = if matches!(self.peek(), Some(Token::Else)) {
            self.advance(); // consume 'else'

            match self.peek() {
                Some(Token::LBrace) => Some(Box::new(self.block()?)),
                Some(Token::If) => Some(Box::new(self.if_statement()?)),
                _ => return Err(expected("'{' or 'if'", self.advance())),
            }
        } else {
            None
        };

        Ok(Stmt::If { condition, then_branch, else_branch })
    }

    fn while_statement(&mut self) -> Result<Stmt> {
        self.advance(); // consume 'while'

        self.expect(Token::LParen, "'('")?;
        let condition = self.expr()?;
        self.expect(Token::RParen, "')'")?;

        let body = match self.peek() {
            Some(Token::LBrace) => Box::new(self.block()?),
            _ => return Err(expected("'{'", self.advance())),
        };

        Ok(Stmt::While { condition, body })
    }

    fn function_statement(&mut self) -> Result<Stmt> {
        self.advance(); // consume 'function'

        let name = match self.advance() {
            Some(Token::Ident(name)) => name,
            other => return Err(expected("a function name", other)),
        };

        self.expect(Token::LParen, "'('")?;

        let mut params = Vec::new();

        if !matches!(self.peek(), Some(Token::RParen)) {

            loop {

                match self.advance() {
                    Some(Token::Ident(param)) => params.push(param),
                    other => return Err(expected("a parameter name", other)),
                }

                match self.peek() {
                    Some(Token::Comma) => {
                        self.advance();
                    }
                    _ => break,
                }
            }
        }

        self.expect(Token::RParen, "')'")?;

        let body = match self.peek() {
            Some(Token::LBrace) => self.block()?,
            _ => return Err(expected("'{'", self.advance())),
        };

        Ok(Stmt::Function(Rc::new(Function { name, params, body })))
    }

    fn return_statement(&mut self) -> Result<Stmt> {
        self.advance(); // consume 'return'

        // No null in the language: `return` always carries a value.
        let value = self.expr()?;
        Ok(Stmt::Return(value))
    }

    fn expr(&mut self) -> Result<Expr> {
        self.equality()
    }

    // One precedence level: `match_op` decides which tokens belong to this
    // level, `next` parses the operands (the next, tighter level). Plain
    // function pointers — no captures needed.
    fn binary_level(
        &mut self,
        match_op: fn(&Token) -> Option<BinaryOp>,
        next: fn(&mut Self) -> Result<Expr>,
    ) -> Result<Expr> {
        let mut left = next(self)?;

        while let Some(op) = self.peek().and_then(match_op) {
            self.advance();
            let right = next(self)?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }

        Ok(left)
    }

    fn equality(&mut self) -> Result<Expr> {
        self.binary_level(
            |tok| match tok {
                Token::EqualEqual => Some(BinaryOp::Eq),
                Token::BangEqual => Some(BinaryOp::Ne),
                _ => None,
            },
            Self::comparison,
        )
    }

    fn comparison(&mut self) -> Result<Expr> {
        self.binary_level(
            |tok| match tok {
                Token::Less => Some(BinaryOp::Lt),
                Token::LessEqual => Some(BinaryOp::Le),
                Token::Greater => Some(BinaryOp::Gt),
                Token::GreaterEqual => Some(BinaryOp::Ge),
                _ => None,
            },
            Self::term,
        )
    }

    fn term(&mut self) -> Result<Expr> {
        self.binary_level(
            |tok| match tok {
                Token::Plus => Some(BinaryOp::Add),
                Token::Minus => Some(BinaryOp::Sub),
                _ => None,
            },
            Self::factor,
        )
    }

    fn factor(&mut self) -> Result<Expr> {
        self.binary_level(
            |tok| match tok {
                Token::Star => Some(BinaryOp::Mul),
                Token::Slash => Some(BinaryOp::Div),
                _ => None,
            },
            Self::unary,
        )
    }

    fn unary(&mut self) -> Result<Expr> {
        let op = match self.peek() {
            Some(Token::Minus) => Some(UnaryOp::Neg),
            Some(Token::Bang) => Some(UnaryOp::Not),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let operand = self.unary()?;
            return Ok(Expr::Unary { op, operand: Box::new(operand) });
        }

        self.postfix()
    }

    // Postfix indexing binds tighter than any operator: a[0][1], f(x)[2].
    fn postfix(&mut self) -> Result<Expr> {
        let mut expr = self.primary()?;

        while matches!(self.peek(), Some(Token::LBracket)) {
            self.advance(); // consume '['
            let index = self.expr()?;
            self.expect(Token::RBracket, "']'")?;
            expr = Expr::Index { target: Box::new(expr), index: Box::new(index) };
        }

        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr> {

        match self.advance() {
            Some(Token::Number(n)) => Ok(Expr::Literal(Value::Number(n))),
            Some(Token::Str(s)) => Ok(Expr::Literal(Value::Str(s.into()))),
            Some(Token::True) => Ok(Expr::Literal(Value::Bool(true))),
            Some(Token::False) => Ok(Expr::Literal(Value::Bool(false))),
            Some(Token::Ident(name)) => {

                // A name followed by '(' is a call, otherwise a variable.
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.advance(); // consume '('
                    let args = self.arguments()?;
                    return Ok(Expr::Call { name, args });
                }

                Ok(Expr::Variable(name))
            }
            Some(Token::LParen) => {
                let inner = self.expr()?;

                match self.advance() {
                    Some(Token::RParen) => Ok(inner),
                    other => Err(expected("')'", other)),
                }
            }
            Some(Token::LBracket) => {
                let mut items = Vec::new();

                if !matches!(self.peek(), Some(Token::RBracket)) {

                    loop {
                        items.push(self.expr()?);

                        match self.peek() {
                            Some(Token::Comma) => {
                                self.advance();
                            }
                            _ => break,
                        }
                    }
                }

                self.expect(Token::RBracket, "']'")?;
                Ok(Expr::Array(items))
            }
            other => Err(expected("a literal, a variable or '('", other)),
        }
    }

    /// Comma-separated argument list; the opening '(' is already consumed.
    fn arguments(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();

        if !matches!(self.peek(), Some(Token::RParen)) {

            loop {
                args.push(self.expr()?);

                match self.peek() {
                    Some(Token::Comma) => {
                        self.advance();
                    }
                    _ => break,
                }
            }
        }

        self.expect(Token::RParen, "')'")?;
        Ok(args)
    }
}
