//! Tokenizes assembly source text.
//!
//! Owns: the [`Token`] type and [`lex`]/[`lex_line`] -- extracted, near
//! verbatim, from `jxcl`'s original `src/assembler/lexer.rs`. JXCL
//! assembly is line-oriented -- every statement (label, directive, or
//! instruction) occupies exactly one line -- which keeps the grammar
//! simple and error locations exact.
//!
//! `docs/crates.toml` lists `jxcl-errors` as this crate's one dependency
//! (for the crate-wide error type). At the time this crate was
//! implemented, `jxcl-errors` was still a scaffolded placeholder with no
//! public items, so [`LexError`] is a small local error type scoped to
//! lexing, matching the shape (`kind`/`line`/`reason`) of what
//! `jxcl-errors`'s `AssemblerErrorKind` variants would produce. Once
//! `jxcl-errors` lands, this can become a thin wrapper (or direct
//! re-export) instead of an independent type.

#![forbid(unsafe_code)]

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// A bare word: mnemonic, register name, label, or constant reference.
    Ident(String),
    /// A directive keyword, without its leading `.` (e.g. `.data` -> `"data"`).
    Directive(String),
    /// A numeric literal (decimal, `0x` hex, or `0b` binary), already parsed.
    Number(i128),
    Comma,
    Colon,
    LBracket,
    RBracket,
    Plus,
    Minus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexErrorKind {
    MalformedImmediate,
    UnexpectedToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub line: usize,
    pub reason: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "lex error at line {}: {:?}: {}",
            self.line, self.kind, self.reason
        )
    }
}
impl std::error::Error for LexError {}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn parse_number(text: &str, line: usize) -> Result<i128, LexError> {
    let bad = |reason: String| LexError {
        kind: LexErrorKind::MalformedImmediate,
        line,
        reason,
    };
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        i128::from_str_radix(hex, 16)
            .map_err(|e| bad(format!("invalid hex literal {:?}: {}", text, e)))
    } else if let Some(bin) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
        i128::from_str_radix(bin, 2)
            .map_err(|e| bad(format!("invalid binary literal {:?}: {}", text, e)))
    } else {
        text.parse::<i128>()
            .map_err(|e| bad(format!("invalid decimal literal {:?}: {}", text, e)))
    }
}

/// Tokenize one line of JXCL assembly source. `;` begins a line comment
/// that runs to end of line.
pub fn lex_line(line_text: &str, line: usize) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = line_text.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\r' => {
                i += 1;
            }
            ';' => break, // rest of line is a comment
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            ':' => {
                tokens.push(Token::Colon);
                i += 1;
            }
            '[' => {
                tokens.push(Token::LBracket);
                i += 1;
            }
            ']' => {
                tokens.push(Token::RBracket);
                i += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '.' => {
                let start = i + 1;
                let mut j = start;
                while j < chars.len() && is_ident_continue(chars[j]) {
                    j += 1;
                }
                if j == start {
                    return Err(LexError {
                        kind: LexErrorKind::UnexpectedToken,
                        line,
                        reason: "'.' not followed by a directive name".into(),
                    });
                }
                tokens.push(Token::Directive(chars[start..j].iter().collect()));
                i = j;
            }
            c if c.is_ascii_digit() => {
                let start = i;
                let mut j = i + 1;
                while j < chars.len() && is_ident_continue(chars[j]) {
                    j += 1;
                }
                let text: String = chars[start..j].iter().collect();
                tokens.push(Token::Number(parse_number(&text, line)?));
                i = j;
            }
            c if is_ident_start(c) => {
                let start = i;
                let mut j = i + 1;
                while j < chars.len() && is_ident_continue(chars[j]) {
                    j += 1;
                }
                tokens.push(Token::Ident(chars[start..j].iter().collect()));
                i = j;
            }
            other => {
                return Err(LexError {
                    kind: LexErrorKind::UnexpectedToken,
                    line,
                    reason: format!("unexpected character {:?}", other),
                });
            }
        }
    }

    Ok(tokens)
}

/// Tokenize a complete multi-line source file, one line at a time.
/// Returns one entry per source line, in order, as `(1-based line number,
/// that line's tokens)`. A blank or comment-only line yields an empty
/// token vector rather than being skipped, so line numbers in the result
/// always match the source file exactly.
pub fn lex(source: &str) -> Result<Vec<(usize, Vec<Token>)>, LexError> {
    let mut out = Vec::new();
    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx + 1;
        out.push((line_no, lex_line(raw_line, line_no)?));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_instruction_line() {
        let toks = lex_line("    MOVI R1, 10", 1).unwrap();
        assert_eq!(
            toks,
            vec![
                Token::Ident("MOVI".into()),
                Token::Ident("R1".into()),
                Token::Comma,
                Token::Number(10),
            ]
        );
    }

    #[test]
    fn tokenizes_label() {
        let toks = lex_line("start:", 1).unwrap();
        assert_eq!(toks, vec![Token::Ident("start".into()), Token::Colon]);
    }

    #[test]
    fn tokenizes_hex_and_binary() {
        let toks = lex_line("0x1A 0b1010", 1).unwrap();
        assert_eq!(toks, vec![Token::Number(0x1A), Token::Number(0b1010)]);
    }

    #[test]
    fn strips_comments() {
        let toks = lex_line("HALT ; stop here", 1).unwrap();
        assert_eq!(toks, vec![Token::Ident("HALT".into())]);
    }

    #[test]
    fn tokenizes_memory_operand() {
        let toks = lex_line("[R2+8]", 1).unwrap();
        assert_eq!(
            toks,
            vec![
                Token::LBracket,
                Token::Ident("R2".into()),
                Token::Plus,
                Token::Number(8),
                Token::RBracket
            ]
        );
    }

    #[test]
    fn tokenizes_directive() {
        let toks = lex_line(".data", 1).unwrap();
        assert_eq!(toks, vec![Token::Directive("data".into())]);
    }

    #[test]
    fn lex_whole_source_preserves_line_numbers() {
        let src = "MOVI R1, 1\n\nHALT\n";
        let lines = lex(src).unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].0, 1);
        assert_eq!(lines[1].0, 2);
        assert!(lines[1].1.is_empty());
        assert_eq!(lines[2].0, 3);
        assert_eq!(lines[2].1, vec![Token::Ident("HALT".into())]);
    }

    #[test]
    fn rejects_unexpected_character() {
        let err = lex_line("MOVI R1, @", 1).unwrap_err();
        assert_eq!(err.kind, LexErrorKind::UnexpectedToken);
    }

    #[test]
    fn rejects_malformed_number() {
        let err = lex_line("0xZZ", 1).unwrap_err();
        assert_eq!(err.kind, LexErrorKind::MalformedImmediate);
    }
}
