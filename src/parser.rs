use crate::ast::{Expr, Stmt};
use crate::error::{LangError, Result};
use crate::lexer::Token;

// Recursive descent. The grammar encodes operator precedence by call order:
//   program   := statement*
//   statement := "var" IDENT "=" expr | expr
//   expr      := term   (("+" | "-") term)*
//   term      := factor (("*" | "/") factor)*
//   factor    := "-" factor | primary
//   primary   := NUMBER | IDENT | "(" expr ")"

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

        if matches!(self.peek(), Some(Token::Var)) {
            self.advance(); // consume 'var'

            let name = match self.advance() {
                Some(Token::Ident(name)) => name,
                other => return Err(expected("a variable name", other)),
            };

            match self.advance() {
                Some(Token::Equals) => {}
                other => return Err(expected("'='", other)),
            }

            let value = self.expr()?;
            return Ok(Stmt::Let { name, value });
        }

        Ok(Stmt::Expr(self.expr()?))
    }

    fn expr(&mut self) -> Result<Expr> {
        let mut left = self.term()?;

        while matches!(self.peek(), Some(Token::Plus) | Some(Token::Minus)) {
            let op = self.advance().unwrap();
            let right = self.term()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }

        Ok(left)
    }

    fn term(&mut self) -> Result<Expr> {
        let mut left = self.factor()?;

        while matches!(self.peek(), Some(Token::Star) | Some(Token::Slash)) {
            let op = self.advance().unwrap();
            let right = self.factor()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }

        Ok(left)
    }

    fn factor(&mut self) -> Result<Expr> {

        if matches!(self.peek(), Some(Token::Minus)) {
            let op = self.advance().unwrap();
            let operand = self.factor()?;
            return Ok(Expr::Unary { op, operand: Box::new(operand) });
        }

        self.primary()
    }

    fn primary(&mut self) -> Result<Expr> {

        match self.advance() {
            Some(Token::Number(n)) => Ok(Expr::Number(n)),
            Some(Token::Ident(name)) => Ok(Expr::Variable(name)),
            Some(Token::LParen) => {
                let inner = self.expr()?;

                match self.advance() {
                    Some(Token::RParen) => Ok(inner),
                    other => Err(expected("')'", other)),
                }
            }
            other => Err(expected("a number, a variable or '('", other)),
        }
    }
}
