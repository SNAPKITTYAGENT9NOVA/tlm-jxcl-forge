//! Tokenizer for the restricted Rust verification language.
//!
//! This is a genuinely separate lexer from `vf-lexer` (the `.vf`
//! theorem language's), not a reuse -- the two languages have nothing
//! in common syntactically (this one looks like a small subset of
//! Rust; `.vf` looks like a dependent type theory), and keeping them
//! separate means a change to one's token set can never silently
//! affect the other.
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Ident(String),
    IntLit(i64),

    // Keywords
    KwFn,
    KwLet,
    KwMut,
    KwIf,
    KwElse,
    KwWhile,
    KwReturn,
    KwTrue,
    KwFalse,
    KwRequires,
    KwEnsures,
    KwInvariant,
    KwDecreases,
    KwI64,
    KwBool,

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Semi,
    Arrow, // ->
    Eq,    // =
    EqEq,  // ==
    Ne,    // !=
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    AndAnd,
    OrOr,
    Bang,

    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.span.line, self.span.col, self.message)
    }
}
impl std::error::Error for LexError {}

fn keyword(ident: &str) -> Option<TokenKind> {
    Some(match ident {
        "fn" => TokenKind::KwFn,
        "let" => TokenKind::KwLet,
        "mut" => TokenKind::KwMut,
        "if" => TokenKind::KwIf,
        "else" => TokenKind::KwElse,
        "while" => TokenKind::KwWhile,
        "return" => TokenKind::KwReturn,
        "true" => TokenKind::KwTrue,
        "false" => TokenKind::KwFalse,
        "requires" => TokenKind::KwRequires,
        "ensures" => TokenKind::KwEnsures,
        "invariant" => TokenKind::KwInvariant,
        "decreases" => TokenKind::KwDecreases,
        "i64" => TokenKind::KwI64,
        "bool" => TokenKind::KwBool,
        _ => return None,
    })
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: u32,
    col: u32,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer {
            src: src.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.src.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn here(&self) -> (usize, u32, u32) {
        (self.pos, self.line, self.col)
    }

    fn span_from(&self, start: (usize, u32, u32)) -> Span {
        Span {
            start: start.0,
            end: self.pos,
            line: start.1,
            col: start.2,
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_whitespace() => {
                    self.advance();
                }
                Some(b'/') if self.peek2() == Some(b'/') => {
                    while let Some(c) = self.peek() {
                        if c == b'\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_trivia();
        let start = self.here();
        let Some(c) = self.peek() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: self.span_from(start),
            });
        };

        if c.is_ascii_alphabetic() || c == b'_' {
            let mut s = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_alphanumeric() || c == b'_' {
                    s.push(c as char);
                    self.advance();
                } else {
                    break;
                }
            }
            let kind = keyword(&s).unwrap_or(TokenKind::Ident(s));
            return Ok(Token {
                kind,
                span: self.span_from(start),
            });
        }

        if c.is_ascii_digit() {
            let mut s = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    s.push(c as char);
                    self.advance();
                } else {
                    break;
                }
            }
            let n: i64 = s.parse().map_err(|_| LexError {
                message: format!("integer literal {s:?} does not fit in an i64"),
                span: self.span_from(start),
            })?;
            return Ok(Token {
                kind: TokenKind::IntLit(n),
                span: self.span_from(start),
            });
        }

        let kind = match c {
            b'(' => {
                self.advance();
                TokenKind::LParen
            }
            b')' => {
                self.advance();
                TokenKind::RParen
            }
            b'{' => {
                self.advance();
                TokenKind::LBrace
            }
            b'}' => {
                self.advance();
                TokenKind::RBrace
            }
            b',' => {
                self.advance();
                TokenKind::Comma
            }
            b':' => {
                self.advance();
                TokenKind::Colon
            }
            b';' => {
                self.advance();
                TokenKind::Semi
            }
            b'+' => {
                self.advance();
                TokenKind::Plus
            }
            b'*' => {
                self.advance();
                TokenKind::Star
            }
            b'/' => {
                self.advance();
                TokenKind::Slash
            }
            b'%' => {
                self.advance();
                TokenKind::Percent
            }
            b'-' => {
                self.advance();
                if self.peek() == Some(b'>') {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            b'=' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::EqEq
                } else {
                    TokenKind::Eq
                }
            }
            b'!' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::Ne
                } else {
                    TokenKind::Bang
                }
            }
            b'<' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::Le
                } else {
                    TokenKind::Lt
                }
            }
            b'>' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::Ge
                } else {
                    TokenKind::Gt
                }
            }
            b'&' => {
                self.advance();
                if self.peek() == Some(b'&') {
                    self.advance();
                    TokenKind::AndAnd
                } else {
                    return Err(LexError {
                        message: "expected '&&' after '&'".to_string(),
                        span: self.span_from(start),
                    });
                }
            }
            b'|' => {
                self.advance();
                if self.peek() == Some(b'|') {
                    self.advance();
                    TokenKind::OrOr
                } else {
                    return Err(LexError {
                        message: "expected '||' after '|'".to_string(),
                        span: self.span_from(start),
                    });
                }
            }
            other => {
                self.advance();
                return Err(LexError {
                    message: format!("unrecognized character {:?}", other as char),
                    span: self.span_from(start),
                });
            }
        };
        Ok(Token {
            kind,
            span: self.span_from(start),
        })
    }
}

/// Tokenize `src` in full, returning every token (including a
/// trailing [`TokenKind::Eof`]) or the first [`LexError`] encountered.
pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer::new(src);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token()?;
        let is_eof = tok.kind == TokenKind::Eof;
        tokens.push(tok);
        if is_eof {
            break;
        }
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn lexes_a_function_signature() {
        assert_eq!(
            kinds("fn add(x: i64, y: i64) -> i64"),
            vec![
                TokenKind::KwFn,
                TokenKind::Ident("add".to_string()),
                TokenKind::LParen,
                TokenKind::Ident("x".to_string()),
                TokenKind::Colon,
                TokenKind::KwI64,
                TokenKind::Comma,
                TokenKind::Ident("y".to_string()),
                TokenKind::Colon,
                TokenKind::KwI64,
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::KwI64,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_int_literals() {
        assert_eq!(
            kinds("0 42 1000"),
            vec![
                TokenKind::IntLit(0),
                TokenKind::IntLit(42),
                TokenKind::IntLit(1000),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn distinguishes_eq_eqeq_and_arrow_from_minus() {
        assert_eq!(
            kinds("= == -> - !="),
            vec![
                TokenKind::Eq,
                TokenKind::EqEq,
                TokenKind::Arrow,
                TokenKind::Minus,
                TokenKind::Ne,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_comparison_operators() {
        assert_eq!(
            kinds("< <= > >="),
            vec![
                TokenKind::Lt,
                TokenKind::Le,
                TokenKind::Gt,
                TokenKind::Ge,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lexes_logical_operators() {
        assert_eq!(
            kinds("&& || !"),
            vec![
                TokenKind::AndAnd,
                TokenKind::OrOr,
                TokenKind::Bang,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn skips_line_comments() {
        assert_eq!(
            kinds("let x = 1; // set x\nlet y = 2;"),
            vec![
                TokenKind::KwLet,
                TokenKind::Ident("x".to_string()),
                TokenKind::Eq,
                TokenKind::IntLit(1),
                TokenKind::Semi,
                TokenKind::KwLet,
                TokenKind::Ident("y".to_string()),
                TokenKind::Eq,
                TokenKind::IntLit(2),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn recognizes_all_contract_keywords() {
        assert_eq!(
            kinds("requires ensures invariant decreases"),
            vec![
                TokenKind::KwRequires,
                TokenKind::KwEnsures,
                TokenKind::KwInvariant,
                TokenKind::KwDecreases,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn a_lone_ampersand_is_a_lex_error_not_a_panic() {
        let err = lex("a & b").unwrap_err();
        assert!(err.message.contains("&&"));
    }

    #[test]
    fn an_unrecognized_character_is_a_lex_error() {
        let err = lex("a $ b").unwrap_err();
        assert!(err.message.contains('$'));
    }

    #[test]
    fn tracks_line_numbers_across_newlines() {
        let tokens = lex("let\nx").unwrap();
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[1].span.line, 2);
    }
}
