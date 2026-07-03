use std::fmt;

use crate::error::{LangError, Result};

/// A single lexical token produced from the source text.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Ident(String),
    Var, // keyword
    Equals,
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Semicolon,
}

// Used to build readable error messages, e.g. "expected '=', found '+'".
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {
            Token::Number(n) => write!(f, "{n}"),
            Token::Ident(name) => write!(f, "{name}"),
            Token::Var => write!(f, "var"),
            Token::Equals => write!(f, "="),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::Semicolon => write!(f, ";"),
        }
    }
}

/// Turn source text into a flat list of tokens.
pub fn tokenize(src: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = src.chars().peekable();

    while let Some(&c) = chars.peek() {

        match c {
            ' ' | '\t' | '\r' | '\n' => {
                chars.next();
            }
            '+' => {
                chars.next();
                tokens.push(Token::Plus);
            }
            '-' => {
                chars.next();
                tokens.push(Token::Minus);
            }
            '*' => {
                chars.next();
                tokens.push(Token::Star);
            }
            '/' => {
                chars.next();
                tokens.push(Token::Slash);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '=' => {
                chars.next();
                tokens.push(Token::Equals);
            }
            ';' => {
                chars.next();
                tokens.push(Token::Semicolon);
            }
            '0'..='9' | '.' => {
                let mut num = String::new();

                while let Some(&d) = chars.peek() {

                    if d.is_ascii_digit() || d == '.' {
                        num.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }

                let value = num
                    .parse::<f64>()
                    .map_err(|_| LangError::InvalidNumber(num.clone()))?;

                tokens.push(Token::Number(value));
            }
            c if c.is_alphabetic() || c == '_' => {
                let mut ident = String::new();

                while let Some(&d) = chars.peek() {

                    if d.is_alphanumeric() || d == '_' {
                        ident.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }

                // Reserved keyword, or plain identifier?
                let token = match ident.as_str() {
                    "var" => Token::Var,
                    _ => Token::Ident(ident),
                };

                tokens.push(token);
            }
            _ => return Err(LangError::UnexpectedChar(c)),
        }
    }

    Ok(tokens)
}
