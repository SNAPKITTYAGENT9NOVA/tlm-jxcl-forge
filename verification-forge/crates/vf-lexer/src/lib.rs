//! Tokenizer for the `.vf` theorem language surface syntax.
//!
//! This crate is untrusted (per `docs/TRUST_MODEL.md`): a lexer bug
//! can at worst cause a parse failure or an elaboration of the wrong
//! surface term, never an unsound proof, because `vf-kernel`
//! independently re-checks whatever `vf-elab` eventually produces.
//! That said, it still fails closed: an unrecognized character is a
//! reported [`LexError`], never silently skipped or panicked on.
#![forbid(unsafe_code)]

use std::fmt;

/// A byte-offset span into the source text, used for error messages
/// and (later) counterexample source locations.
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
    Nat(u64),

    // Keywords
    KwFun,
    KwForall,
    KwLet,
    KwIn,
    KwDef,
    KwAxiom,
    KwTheorem,
    KwProp,
    KwType,

    // Punctuation
    LParen,
    RParen,
    Colon,
    ColonEq,
    Comma,
    Eq,
    FatArrow, // =>
    Arrow,    // ->
    Plus,
    Star,

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
        "fun" => TokenKind::KwFun,
        "forall" => TokenKind::KwForall,
        "let" => TokenKind::KwLet,
        "in" => TokenKind::KwIn,
        "def" => TokenKind::KwDef,
        "axiom" => TokenKind::KwAxiom,
        "theorem" => TokenKind::KwTheorem,
        "Prop" => TokenKind::KwProp,
        "Type" => TokenKind::KwType,
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
                Some(b'-') if self.peek2() == Some(b'-') => {
                    // line comment: `-- ...` to end of line
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
                if c.is_ascii_alphanumeric() || c == b'_' || c == b'\'' {
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
            let n: u64 = s.parse().map_err(|_| LexError {
                message: format!("numeric literal {s:?} does not fit in a u64"),
                span: self.span_from(start),
            })?;
            return Ok(Token {
                kind: TokenKind::Nat(n),
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
            b',' => {
                self.advance();
                TokenKind::Comma
            }
            b'+' => {
                self.advance();
                TokenKind::Plus
            }
            b'*' => {
                self.advance();
                TokenKind::Star
            }
            b'=' => {
                self.advance();
                if self.peek() == Some(b'>') {
                    self.advance();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }
            b':' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::ColonEq
                } else {
                    TokenKind::Colon
                }
            }
            b'-' => {
                self.advance();
                if self.peek() == Some(b'>') {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    return Err(LexError {
                        message: "expected '->' after '-'".to_string(),
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
    fn lexes_identifiers_and_keywords_separately() {
        assert_eq!(
            kinds("foo fun forall"),
            vec![
                TokenKind::Ident("foo".to_string()),
                TokenKind::KwFun,
                TokenKind::KwForall,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_nat_literals() {
        assert_eq!(
            kinds("0 42"),
            vec![TokenKind::Nat(0), TokenKind::Nat(42), TokenKind::Eof]
        );
    }

    #[test]
    fn lexes_colon_vs_colon_eq() {
        assert_eq!(
            kinds(": :="),
            vec![TokenKind::Colon, TokenKind::ColonEq, TokenKind::Eof]
        );
    }

    #[test]
    fn lexes_arrow_and_fat_arrow() {
        assert_eq!(
            kinds("-> =>"),
            vec![TokenKind::Arrow, TokenKind::FatArrow, TokenKind::Eof]
        );
    }

    #[test]
    fn lexes_eq_alone_when_not_followed_by_gt() {
        assert_eq!(
            kinds("= x"),
            vec![
                TokenKind::Eq,
                TokenKind::Ident("x".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn skips_line_comments() {
        assert_eq!(
            kinds("foo -- this is a comment\nbar"),
            vec![
                TokenKind::Ident("foo".to_string()),
                TokenKind::Ident("bar".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn skips_whitespace_including_newlines() {
        assert_eq!(
            kinds("  foo\n\tbar  "),
            vec![
                TokenKind::Ident("foo".to_string()),
                TokenKind::Ident("bar".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn tracks_line_and_column() {
        let tokens = lex("foo\nbar").unwrap();
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[1].span.line, 2);
    }

    #[test]
    fn reports_unrecognized_character_as_a_lex_error_not_a_panic() {
        let err = lex("foo @ bar").unwrap_err();
        assert!(err.message.contains('@'));
    }

    #[test]
    fn reports_lone_dash_as_a_lex_error() {
        let err = lex("- x").unwrap_err();
        assert!(err.message.contains("->"));
    }

    #[test]
    fn full_declaration_lexes_to_the_expected_token_sequence() {
        let src = "axiom foo : Prop";
        assert_eq!(
            kinds(src),
            vec![
                TokenKind::KwAxiom,
                TokenKind::Ident("foo".to_string()),
                TokenKind::Colon,
                TokenKind::KwProp,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn empty_input_lexes_to_just_eof() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn identifiers_may_contain_underscores_and_primes_and_digits() {
        assert_eq!(
            kinds("add_comm x1 y'"),
            vec![
                TokenKind::Ident("add_comm".to_string()),
                TokenKind::Ident("x1".to_string()),
                TokenKind::Ident("y'".to_string()),
                TokenKind::Eof,
            ]
        );
    }
}
