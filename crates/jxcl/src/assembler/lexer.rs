//! Assembler lexer (spec §16): turns one source line into a token stream.
//! JXCL assembly is line-oriented — every statement (label, directive, or
//! instruction) occupies exactly one line, which keeps the grammar simple
//! and the error locations exact (spec §42: errors carry a line number).

use crate::errors::{AssemblerError, AssemblerErrorKind};

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

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn parse_number(text: &str, line: usize) -> Result<i128, AssemblerError> {
    let bad = |reason: String| AssemblerError {
        kind: AssemblerErrorKind::MalformedImmediate,
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
/// that runs to end of line (spec §16 "comments").
pub fn tokenize(line_text: &str, line: usize) -> Result<Vec<Token>, AssemblerError> {
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
                    return Err(AssemblerError {
                        kind: AssemblerErrorKind::UnexpectedToken,
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
                while j < chars.len() && (is_ident_continue(chars[j])) {
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
                return Err(AssemblerError {
                    kind: AssemblerErrorKind::UnexpectedToken,
                    line,
                    reason: format!("unexpected character {:?}", other),
                });
            }
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_instruction_line() {
        let toks = tokenize("    MOVI R1, 10", 1).unwrap();
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
        let toks = tokenize("start:", 1).unwrap();
        assert_eq!(toks, vec![Token::Ident("start".into()), Token::Colon]);
    }

    #[test]
    fn tokenizes_hex_and_binary() {
        let toks = tokenize("0x1A 0b1010", 1).unwrap();
        assert_eq!(toks, vec![Token::Number(0x1A), Token::Number(0b1010)]);
    }

    #[test]
    fn strips_comments() {
        let toks = tokenize("HALT ; stop here", 1).unwrap();
        assert_eq!(toks, vec![Token::Ident("HALT".into())]);
    }

    #[test]
    fn tokenizes_memory_operand() {
        let toks = tokenize("[R2+8]", 1).unwrap();
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
        let toks = tokenize(".data", 1).unwrap();
        assert_eq!(toks, vec![Token::Directive("data".into())]);
    }
}
