//! Parses a token stream into an assembly AST (instructions, labels,
//! directives).
//!
//! Owns: the assembly AST ([`AstNode`]) and its parser ([`parse`]/
//! [`parse_line`]) -- extracted, near verbatim, from `jxcl`'s original
//! `src/assembler/parser.rs`. Label/constant references are kept symbolic
//! (`ValueExpr::Name`) here; resolving them against a symbol table is
//! `jxcl-assembler`'s job.
//!
//! ## Deviation from `docs/crates.toml`
//!
//! The registry lists `jxcl-instructions` and `jxcl-operands` as
//! dependencies (this crate's node for an instruction statement
//! conceptually "is" an instruction with operands). At the time this
//! crate was implemented those two crates were still scaffolded
//! placeholders with no public items, and the real pre-expansion parser
//! (`jxcl/src/assembler/parser.rs`) never depended on `isa::instruction`/
//! `isa::operand` either -- it produces its own lightweight
//! [`InstrNode`]/[`OperandNode`] tree of *unresolved* mnemonic text and
//! operand syntax (a register index, a bare value/label reference, or a
//! `[base+disp]` memory operand), deferring every opcode-format and
//! register-range check to the assembler's emission pass (which does
//! depend on `jxcl-opcodes`). So this crate keeps the real, dependency-free
//! shape faithfully rather than routing through `jxcl-instructions`/
//! `jxcl-operands`, which would gain nothing (those crates model *decoded*
//! operands with concrete widths, not the pre-resolution symbolic operands
//! a parser produces) and would only add a placeholder dependency that
//! could not compile against anything yet. `jxcl-lexer`/`jxcl-errors`
//! remain real dependencies, used as intended.
#![forbid(unsafe_code)]

use jxcl_errors::{AssemblerError, AssemblerErrorKind};
use jxcl_lexer::{lex, LexError, Token};

fn from_lex_error(e: LexError) -> AssemblerError {
    AssemblerError {
        kind: AssemblerErrorKind::UnexpectedToken,
        line: e.line,
        reason: e.reason,
    }
}

/// A value that is either already known (`Number`) or must be resolved
/// against a symbol table (`Name` -- a label or a `.const`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueExpr {
    Number(i128),
    Name(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperandNode {
    Reg(u8),
    Value(ValueExpr),
    Mem { base: u8, disp: ValueExpr },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrNode {
    pub mnemonic: String,
    pub operands: Vec<OperandNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataWidth {
    Byte,
    Word,
    Dword,
    Qword,
}

/// One parsed statement -- the assembly AST's node type (matches
/// `docs/crates.toml`'s `public_api` entry `AstNode`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstNode {
    Empty,
    SwitchToCode,
    SwitchToData,
    ConstDef {
        name: String,
        value: ValueExpr,
    },
    EntryDef {
        value: ValueExpr,
    },
    Data {
        width: DataWidth,
        values: Vec<ValueExpr>,
    },
    Instruction(InstrNode),
}

/// A single source line's parse result: its (optional) leading label, its
/// statement, and the 1-based line number it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLine {
    pub line: usize,
    pub label: Option<String>,
    pub node: AstNode,
}

fn err(kind: AssemblerErrorKind, line: usize, reason: impl Into<String>) -> AssemblerError {
    AssemblerError {
        kind,
        line,
        reason: reason.into(),
    }
}

/// The maximum register index this ISA revision defines (`R0..=R31`,
/// per `jxcl-constants::NUM_GP_REGISTERS`). Not taken as a direct
/// dependency here (a register range check is architecturally owned by
/// the same table `jxcl-assembler` already consults when emitting), but
/// the syntactic shape `R<digits>` is universal enough to recognize at
/// parse time regardless.
const MAX_PLAUSIBLE_REGISTER_DIGITS: usize = 10;

/// Parse a register token's text (`R0`..`R<n>`, case-insensitive). Range
/// validation against the real register count is deferred to the
/// assembler's emission pass, which owns `jxcl-opcodes`; this only checks
/// the syntactic shape and that the digits parse as a `u8`.
pub fn parse_register_text(text: &str, line: usize) -> Result<u8, AssemblerError> {
    let bytes = text.as_bytes();
    if bytes.len() < 2 || !(bytes[0] == b'R' || bytes[0] == b'r') {
        return Err(err(
            AssemblerErrorKind::UnknownRegister,
            line,
            format!("{:?} is not a register", text),
        ));
    }
    let digits = &text[1..];
    if digits.len() > MAX_PLAUSIBLE_REGISTER_DIGITS || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(err(
            AssemblerErrorKind::UnknownRegister,
            line,
            format!("{:?} is not a register", text),
        ));
    }
    digits.parse::<u8>().map_err(|_| {
        err(
            AssemblerErrorKind::UnknownRegister,
            line,
            format!("register index in {:?} out of range", text),
        )
    })
}

fn looks_like_register(text: &str) -> bool {
    let bytes = text.as_bytes();
    (bytes.first() == Some(&b'R') || bytes.first() == Some(&b'r'))
        && bytes.len() > 1
        && bytes[1..].iter().all(u8::is_ascii_digit)
}

/// Parse an optional leading sign followed by a `Number` or `Ident`,
/// starting at `tokens[*pos]`, advancing `*pos` past what it consumes.
fn parse_value(
    tokens: &[Token],
    pos: &mut usize,
    line: usize,
) -> Result<ValueExpr, AssemblerError> {
    let negative = match tokens.get(*pos) {
        Some(Token::Minus) => {
            *pos += 1;
            true
        }
        Some(Token::Plus) => {
            *pos += 1;
            false
        }
        _ => false,
    };
    match tokens.get(*pos) {
        Some(Token::Number(n)) => {
            *pos += 1;
            Ok(ValueExpr::Number(if negative { -n } else { *n }))
        }
        Some(Token::Ident(name)) if !negative => {
            *pos += 1;
            Ok(ValueExpr::Name(name.clone()))
        }
        _ => Err(err(
            AssemblerErrorKind::MalformedOperand,
            line,
            "expected a number, label, or constant",
        )),
    }
}

/// Parse `[Rbase]`, `[Rbase+disp]`, or `[Rbase-disp]`. `tokens[*pos]` must
/// be the `[`.
fn parse_mem_operand(
    tokens: &[Token],
    pos: &mut usize,
    line: usize,
) -> Result<OperandNode, AssemblerError> {
    debug_assert_eq!(tokens.get(*pos), Some(&Token::LBracket));
    *pos += 1;
    let base_text = match tokens.get(*pos) {
        Some(Token::Ident(s)) => s.clone(),
        _ => {
            return Err(err(
                AssemblerErrorKind::MalformedOperand,
                line,
                "expected base register after '['",
            ))
        }
    };
    *pos += 1;
    let base = parse_register_text(&base_text, line)?;

    let disp = match tokens.get(*pos) {
        Some(Token::RBracket) => ValueExpr::Number(0),
        Some(Token::Plus) | Some(Token::Minus) => parse_value(tokens, pos, line)?,
        _ => {
            return Err(err(
                AssemblerErrorKind::MalformedOperand,
                line,
                "expected '+', '-', or ']' in memory operand",
            ))
        }
    };
    match tokens.get(*pos) {
        Some(Token::RBracket) => {
            *pos += 1;
            Ok(OperandNode::Mem { base, disp })
        }
        _ => Err(err(
            AssemblerErrorKind::MalformedOperand,
            line,
            "expected closing ']'",
        )),
    }
}

fn parse_operand(
    tokens: &[Token],
    pos: &mut usize,
    line: usize,
) -> Result<OperandNode, AssemblerError> {
    match tokens.get(*pos) {
        Some(Token::LBracket) => parse_mem_operand(tokens, pos, line),
        Some(Token::Ident(name)) if looks_like_register(name) => {
            let r = parse_register_text(name, line)?;
            *pos += 1;
            Ok(OperandNode::Reg(r))
        }
        Some(Token::Ident(_)) | Some(Token::Number(_)) | Some(Token::Minus) | Some(Token::Plus) => {
            Ok(OperandNode::Value(parse_value(tokens, pos, line)?))
        }
        _ => Err(err(
            AssemblerErrorKind::MalformedOperand,
            line,
            "expected an operand",
        )),
    }
}

fn parse_operand_list(
    tokens: &[Token],
    pos: &mut usize,
    line: usize,
) -> Result<Vec<OperandNode>, AssemblerError> {
    let mut ops = Vec::new();
    if *pos >= tokens.len() {
        return Ok(ops);
    }
    loop {
        ops.push(parse_operand(tokens, pos, line)?);
        match tokens.get(*pos) {
            Some(Token::Comma) => {
                *pos += 1;
            }
            None => break,
            _ => {
                return Err(err(
                    AssemblerErrorKind::UnexpectedToken,
                    line,
                    "expected ',' or end of line",
                ))
            }
        }
    }
    Ok(ops)
}

fn parse_value_list(
    tokens: &[Token],
    pos: &mut usize,
    line: usize,
) -> Result<Vec<ValueExpr>, AssemblerError> {
    let mut values = Vec::new();
    loop {
        values.push(parse_value(tokens, pos, line)?);
        match tokens.get(*pos) {
            Some(Token::Comma) => {
                *pos += 1;
            }
            None => break,
            _ => {
                return Err(err(
                    AssemblerErrorKind::UnexpectedToken,
                    line,
                    "expected ',' or end of line",
                ))
            }
        }
    }
    Ok(values)
}

/// Parse one already-tokenized line into an [`AstNode`]. A line may begin
/// with a label (`name:`) followed by another statement on the same line
/// (e.g. `start: MOVI R1, 0`).
pub fn parse_line(
    tokens: &[Token],
    line: usize,
) -> Result<(Option<String>, AstNode), AssemblerError> {
    if tokens.is_empty() {
        return Ok((None, AstNode::Empty));
    }
    let mut pos = 0;
    let mut label = None;
    if let (Some(Token::Ident(name)), Some(Token::Colon)) = (tokens.first(), tokens.get(1)) {
        label = Some(name.clone());
        pos = 2;
    }
    if pos >= tokens.len() {
        return Ok((label, AstNode::Empty));
    }

    let node = match &tokens[pos] {
        Token::Directive(name) => {
            pos += 1;
            match name.as_str() {
                "code" | "text" => AstNode::SwitchToCode,
                "data" => AstNode::SwitchToData,
                "const" => {
                    let const_name = match tokens.get(pos) {
                        Some(Token::Ident(n)) => n.clone(),
                        _ => {
                            return Err(err(
                                AssemblerErrorKind::MalformedOperand,
                                line,
                                "expected constant name",
                            ))
                        }
                    };
                    pos += 1;
                    let value = parse_value(tokens, &mut pos, line)?;
                    AstNode::ConstDef {
                        name: const_name,
                        value,
                    }
                }
                "entry" => AstNode::EntryDef {
                    value: parse_value(tokens, &mut pos, line)?,
                },
                "byte" => AstNode::Data {
                    width: DataWidth::Byte,
                    values: parse_value_list(tokens, &mut pos, line)?,
                },
                "word" => AstNode::Data {
                    width: DataWidth::Word,
                    values: parse_value_list(tokens, &mut pos, line)?,
                },
                "dword" => AstNode::Data {
                    width: DataWidth::Dword,
                    values: parse_value_list(tokens, &mut pos, line)?,
                },
                "qword" => AstNode::Data {
                    width: DataWidth::Qword,
                    values: parse_value_list(tokens, &mut pos, line)?,
                },
                other => {
                    return Err(err(
                        AssemblerErrorKind::UnexpectedToken,
                        line,
                        format!("unknown directive '.{}'", other),
                    ))
                }
            }
        }
        Token::Ident(mnemonic) => {
            pos += 1;
            let operands = parse_operand_list(tokens, &mut pos, line)?;
            AstNode::Instruction(InstrNode {
                mnemonic: mnemonic.clone(),
                operands,
            })
        }
        _ => {
            return Err(err(
                AssemblerErrorKind::UnexpectedToken,
                line,
                "expected a label, directive, or mnemonic",
            ))
        }
    };

    Ok((label, node))
}

/// Parse a complete source file into one [`ParsedLine`] per source line,
/// in order. Lexes with [`jxcl_lexer::lex`] first, then parses each
/// line's tokens.
pub fn parse(source: &str) -> Result<Vec<ParsedLine>, AssemblerError> {
    let lines = lex(source).map_err(from_lex_error)?;
    let mut out = Vec::with_capacity(lines.len());
    for (line_no, tokens) in lines {
        let (label, node) = parse_line(&tokens, line_no)?;
        out.push(ParsedLine {
            line: line_no,
            label,
            node,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_one(src: &str) -> (Option<String>, AstNode) {
        let toks = jxcl_lexer::lex_line(src, 1).unwrap();
        parse_line(&toks, 1).unwrap()
    }

    #[test]
    fn parses_plain_instruction() {
        let (label, node) = parse_one("MOVI R1, 10");
        assert_eq!(label, None);
        assert_eq!(
            node,
            AstNode::Instruction(InstrNode {
                mnemonic: "MOVI".into(),
                operands: vec![
                    OperandNode::Reg(1),
                    OperandNode::Value(ValueExpr::Number(10))
                ],
            })
        );
    }

    #[test]
    fn parses_label_and_instruction_together() {
        let (label, node) = parse_one("start: JMP start");
        assert_eq!(label, Some("start".into()));
        assert_eq!(
            node,
            AstNode::Instruction(InstrNode {
                mnemonic: "JMP".into(),
                operands: vec![OperandNode::Value(ValueExpr::Name("start".into()))],
            })
        );
    }

    #[test]
    fn parses_memory_operand_with_negative_displacement() {
        let (_, node) = parse_one("LOAD R1, [R2-8]");
        assert_eq!(
            node,
            AstNode::Instruction(InstrNode {
                mnemonic: "LOAD".into(),
                operands: vec![
                    OperandNode::Reg(1),
                    OperandNode::Mem {
                        base: 2,
                        disp: ValueExpr::Number(-8)
                    }
                ],
            })
        );
    }

    #[test]
    fn parses_bare_label_only() {
        let (label, node) = parse_one("start:");
        assert_eq!(label, Some("start".into()));
        assert_eq!(node, AstNode::Empty);
    }

    #[test]
    fn parses_data_directive() {
        let (_, node) = parse_one(".byte 1, 2, 0xFF");
        assert_eq!(
            node,
            AstNode::Data {
                width: DataWidth::Byte,
                values: vec![
                    ValueExpr::Number(1),
                    ValueExpr::Number(2),
                    ValueExpr::Number(0xFF)
                ],
            }
        );
    }

    #[test]
    fn parses_const_and_entry_directives() {
        let (_, node) = parse_one(".const FOO 42");
        assert_eq!(
            node,
            AstNode::ConstDef {
                name: "FOO".into(),
                value: ValueExpr::Number(42),
            }
        );
        let (_, node) = parse_one(".entry start");
        assert_eq!(
            node,
            AstNode::EntryDef {
                value: ValueExpr::Name("start".into()),
            }
        );
    }

    #[test]
    fn unknown_directive_is_an_error() {
        let toks = jxcl_lexer::lex_line(".bogus", 1).unwrap();
        let err = parse_line(&toks, 1).unwrap_err();
        assert_eq!(err.kind, AssemblerErrorKind::UnexpectedToken);
    }

    #[test]
    fn parse_whole_source_preserves_line_numbers_and_labels() {
        let src = "start:\n    MOVI R1, 1\n\n    JMP start\n";
        let lines = parse(src).unwrap();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].line, 1);
        assert_eq!(lines[0].label, Some("start".into()));
        assert_eq!(lines[0].node, AstNode::Empty);
        assert_eq!(lines[2].node, AstNode::Empty);
    }

    #[test]
    fn parse_propagates_lex_errors_with_correct_line() {
        let src = "MOVI R1, 1\nMOVI R2, @\n";
        let err = parse(src).unwrap_err();
        assert_eq!(err.line, 2);
    }

    #[test]
    fn malformed_base_register_in_memory_operand_is_an_error() {
        let toks = jxcl_lexer::lex_line("LOAD R1, [XY+8]", 1).unwrap();
        let err = parse_line(&toks, 1).unwrap_err();
        assert_eq!(err.kind, AssemblerErrorKind::UnknownRegister);
    }
}
