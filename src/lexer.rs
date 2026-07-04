use std::fmt;

use crate::error::{LangError, Result};

/// A single lexical token produced from the source text.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Str(String),
    Ident(String),
    // Keywords
    Var,
    True,
    False,
    If,
    Else,
    While,
    Function,
    Return,
    // Operators and punctuation
    Equals,
    FatArrow,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    AndAnd,
    OrOr,
    Bang,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
}

// Used to build readable error messages, e.g. "expected '=', found '+'".
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match self {
            Token::Number(n) => write!(f, "{n}"),
            Token::Str(s) => write!(f, "\"{s}\""),
            Token::Ident(name) => write!(f, "{name}"),
            Token::Var => write!(f, "var"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::While => write!(f, "while"),
            Token::Function => write!(f, "function"),
            Token::Return => write!(f, "return"),
            Token::Equals => write!(f, "="),
            Token::FatArrow => write!(f, "=>"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::AndAnd => write!(f, "&&"),
            Token::OrOr => write!(f, "||"),
            Token::Bang => write!(f, "!"),
            Token::EqualEqual => write!(f, "=="),
            Token::BangEqual => write!(f, "!="),
            Token::Less => write!(f, "<"),
            Token::LessEqual => write!(f, "<="),
            Token::Greater => write!(f, ">"),
            Token::GreaterEqual => write!(f, ">="),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),
        }
    }
}

/// Turn source text into a flat list of tokens, plus the 1-based source line
/// each token starts on (parallel to `tokens`), so the parser can tag nodes.
pub fn tokenize(src: &str) -> Result<(Vec<Token>, Vec<usize>)> {
    let mut tokens = Vec::new();
    let mut lines = Vec::new();
    let mut line = 1usize;
    let mut chars = src.chars().peekable();

    while let Some(&c) = chars.peek() {
        let start_line = line;

        match c {
            ' ' | '\t' | '\r' => {
                chars.next();
            }
            '\n' => {
                chars.next();
                line += 1;
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

                if chars.peek() == Some(&'/') {

                    // Line comment: skip everything up to the end of line.
                    while let Some(&c) = chars.peek() {

                        if c == '\n' {
                            break;
                        }

                        chars.next();
                    }
                } else {
                    tokens.push(Token::Slash);
                }
            }
            '%' => {
                chars.next();
                tokens.push(Token::Percent);
            }
            '&' => {
                chars.next();

                if chars.peek() == Some(&'&') {
                    chars.next();
                    tokens.push(Token::AndAnd);
                } else {
                    return Err(LangError::UnexpectedChar('&'));
                }
            }
            '|' => {
                chars.next();

                if chars.peek() == Some(&'|') {
                    chars.next();
                    tokens.push(Token::OrOr);
                } else {
                    return Err(LangError::UnexpectedChar('|'));
                }
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '{' => {
                chars.next();
                tokens.push(Token::LBrace);
            }
            '}' => {
                chars.next();
                tokens.push(Token::RBrace);
            }
            '[' => {
                chars.next();
                tokens.push(Token::LBracket);
            }
            ']' => {
                chars.next();
                tokens.push(Token::RBracket);
            }
            ';' => {
                chars.next();
                tokens.push(Token::Semicolon);
            }
            ',' => {
                chars.next();
                tokens.push(Token::Comma);
            }
            // One- or two-character operators: the second char decides.
            '=' => {
                chars.next();

                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::EqualEqual);
                } else if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::FatArrow);
                } else {
                    tokens.push(Token::Equals);
                }
            }
            '!' => {
                chars.next();

                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::BangEqual);
                } else {
                    tokens.push(Token::Bang);
                }
            }
            '<' => {
                chars.next();

                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::LessEqual);
                } else {
                    tokens.push(Token::Less);
                }
            }
            '>' => {
                chars.next();

                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::GreaterEqual);
                } else {
                    tokens.push(Token::Greater);
                }
            }
            '"' => {
                chars.next(); // opening quote

                let mut s = String::new();

                loop {

                    match chars.next() {
                        None => return Err(LangError::UnterminatedString),
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('"') => s.push('"'),
                            Some('\\') => s.push('\\'),
                            Some(c) => return Err(LangError::InvalidEscape(c)),
                            None => return Err(LangError::UnterminatedString),
                        },
                        Some('\n') => {
                            s.push('\n');
                            line += 1;
                        }
                        Some(c) => s.push(c),
                    }
                }

                tokens.push(Token::Str(s));
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
                    "true" => Token::True,
                    "false" => Token::False,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "function" => Token::Function,
                    "return" => Token::Return,
                    _ => Token::Ident(ident),
                };

                tokens.push(token);
            }
            _ => return Err(LangError::UnexpectedChar(c)),
        }

        // Record the start line for whatever token this iteration produced
        // (whitespace and comments produce none).
        if tokens.len() > lines.len() {
            lines.push(start_line);
        }
    }

    Ok((tokens, lines))
}
