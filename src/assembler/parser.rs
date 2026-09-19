//! Assembler parser (spec §16): turns one line's tokens into a `Stmt`.
//! Label/constant references are kept symbolic (`ValueExpr::Name`) here;
//! resolving them against the symbol table is `assembler::mod`'s job
//! (spec §16 "The assembler must resolve labels deterministically.").

use crate::assembler::lexer::Token;
use crate::errors::{AssemblerError, AssemblerErrorKind};

/// A value that is either already known (`Number`) or must be resolved
/// against the symbol table (`Name` — a label or a `.const`).
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Empty,
    SwitchToCode,
    SwitchToData,
    ConstDef { name: String, value: ValueExpr },
    EntryDef { value: ValueExpr },
    Data { width: DataWidth, values: Vec<ValueExpr> },
    Instruction(InstrNode),
}

fn err(kind: AssemblerErrorKind, line: usize, reason: impl Into<String>) -> AssemblerError {
    AssemblerError { kind, line, reason: reason.into() }
}

/// Parse a register token's text (`R0`..`R31`, case-insensitive).
pub fn parse_register_text(text: &str, line: usize) -> Result<u8, AssemblerError> {
    let bytes = text.as_bytes();
    if bytes.len() < 2 || !(bytes[0] == b'R' || bytes[0] == b'r') {
        return Err(err(AssemblerErrorKind::UnknownRegister, line, format!("{:?} is not a register", text)));
    }
    let digits = &text[1..];
    let n: u32 = digits
        .parse()
        .map_err(|_| err(AssemblerErrorKind::UnknownRegister, line, format!("{:?} is not a register", text)))?;
    if n >= crate::isa::constants::NUM_GP_REGISTERS as u32 {
        return Err(err(AssemblerErrorKind::UnknownRegister, line, format!("register index {} out of range", n)));
    }
    Ok(n as u8)
}

fn looks_like_register(text: &str) -> bool {
    let bytes = text.as_bytes();
    (bytes.first() == Some(&b'R') || bytes.first() == Some(&b'r'))
        && bytes.len() > 1
        && bytes[1..].iter().all(u8::is_ascii_digit)
}

/// Parse an optional leading sign followed by a `Number` or `Ident`,
/// starting at `tokens[*pos]`, advancing `*pos` past what it consumes.
fn parse_value(tokens: &[Token], pos: &mut usize, line: usize) -> Result<ValueExpr, AssemblerError> {
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
        _ => Err(err(AssemblerErrorKind::MalformedOperand, line, "expected a number, label, or constant")),
    }
}

/// Parse `[Rbase]`, `[Rbase+disp]`, or `[Rbase-disp]`. `tokens[*pos]` must
/// be the `[`.
fn parse_mem_operand(tokens: &[Token], pos: &mut usize, line: usize) -> Result<OperandNode, AssemblerError> {
    debug_assert_eq!(tokens.get(*pos), Some(&Token::LBracket));
    *pos += 1;
    let base_text = match tokens.get(*pos) {
        Some(Token::Ident(s)) => s.clone(),
        _ => return Err(err(AssemblerErrorKind::MalformedOperand, line, "expected base register after '['")),
    };
    *pos += 1;
    let base = parse_register_text(&base_text, line)?;

    let disp = match tokens.get(*pos) {
        Some(Token::RBracket) => ValueExpr::Number(0),
        Some(Token::Plus) | Some(Token::Minus) => parse_value(tokens, pos, line)?,
        _ => return Err(err(AssemblerErrorKind::MalformedOperand, line, "expected '+', '-', or ']' in memory operand")),
    };
    match tokens.get(*pos) {
        Some(Token::RBracket) => {
            *pos += 1;
            Ok(OperandNode::Mem { base, disp })
        }
        _ => Err(err(AssemblerErrorKind::MalformedOperand, line, "expected closing ']'")),
    }
}

fn parse_operand(tokens: &[Token], pos: &mut usize, line: usize) -> Result<OperandNode, AssemblerError> {
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
        _ => Err(err(AssemblerErrorKind::MalformedOperand, line, "expected an operand")),
    }
}

fn parse_operand_list(tokens: &[Token], pos: &mut usize, line: usize) -> Result<Vec<OperandNode>, AssemblerError> {
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
            _ => return Err(err(AssemblerErrorKind::UnexpectedToken, line, "expected ',' or end of line")),
        }
    }
    Ok(ops)
}

fn parse_value_list(tokens: &[Token], pos: &mut usize, line: usize) -> Result<Vec<ValueExpr>, AssemblerError> {
    let mut values = Vec::new();
    loop {
        values.push(parse_value(tokens, pos, line)?);
        match tokens.get(*pos) {
            Some(Token::Comma) => {
                *pos += 1;
            }
            None => break,
            _ => return Err(err(AssemblerErrorKind::UnexpectedToken, line, "expected ',' or end of line")),
        }
    }
    Ok(values)
}

/// Parse one already-tokenized line into a `Stmt`. A line may begin with
/// a label (`name:`) followed by another statement on the same line
/// (e.g. `start: MOVI R1, 0`).
pub fn parse_line(tokens: &[Token], line: usize) -> Result<(Option<String>, Stmt), AssemblerError> {
    if tokens.is_empty() {
        return Ok((None, Stmt::Empty));
    }
    let mut pos = 0;
    let mut label = None;
    if let (Some(Token::Ident(name)), Some(Token::Colon)) = (tokens.first(), tokens.get(1)) {
        label = Some(name.clone());
        pos = 2;
    }
    if pos >= tokens.len() {
        return Ok((label, Stmt::Empty));
    }

    let stmt = match &tokens[pos] {
        Token::Directive(name) => {
            pos += 1;
            match name.as_str() {
                "code" | "text" => Stmt::SwitchToCode,
                "data" => Stmt::SwitchToData,
                "const" => {
                    let const_name = match tokens.get(pos) {
                        Some(Token::Ident(n)) => n.clone(),
                        _ => return Err(err(AssemblerErrorKind::MalformedOperand, line, "expected constant name")),
                    };
                    pos += 1;
                    let value = parse_value(tokens, &mut pos, line)?;
                    Stmt::ConstDef { name: const_name, value }
                }
                "entry" => Stmt::EntryDef { value: parse_value(tokens, &mut pos, line)? },
                "byte" => Stmt::Data { width: DataWidth::Byte, values: parse_value_list(tokens, &mut pos, line)? },
                "word" => Stmt::Data { width: DataWidth::Word, values: parse_value_list(tokens, &mut pos, line)? },
                "dword" => Stmt::Data { width: DataWidth::Dword, values: parse_value_list(tokens, &mut pos, line)? },
                "qword" => Stmt::Data { width: DataWidth::Qword, values: parse_value_list(tokens, &mut pos, line)? },
                other => {
                    return Err(err(AssemblerErrorKind::UnexpectedToken, line, format!("unknown directive '.{}'", other)))
                }
            }
        }
        Token::Ident(mnemonic) => {
            pos += 1;
            let operands = parse_operand_list(tokens, &mut pos, line)?;
            Stmt::Instruction(InstrNode { mnemonic: mnemonic.clone(), operands })
        }
        _ => return Err(err(AssemblerErrorKind::UnexpectedToken, line, "expected a label, directive, or mnemonic")),
    };

    Ok((label, stmt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembler::lexer::tokenize;

    fn parse(src: &str) -> (Option<String>, Stmt) {
        let toks = tokenize(src, 1).unwrap();
        parse_line(&toks, 1).unwrap()
    }

    #[test]
    fn parses_plain_instruction() {
        let (label, stmt) = parse("MOVI R1, 10");
        assert_eq!(label, None);
        assert_eq!(
            stmt,
            Stmt::Instruction(InstrNode {
                mnemonic: "MOVI".into(),
                operands: vec![OperandNode::Reg(1), OperandNode::Value(ValueExpr::Number(10))],
            })
        );
    }

    #[test]
    fn parses_label_and_instruction_together() {
        let (label, stmt) = parse("start: JMP start");
        assert_eq!(label, Some("start".into()));
        assert_eq!(
            stmt,
            Stmt::Instruction(InstrNode {
                mnemonic: "JMP".into(),
                operands: vec![OperandNode::Value(ValueExpr::Name("start".into()))],
            })
        );
    }

    #[test]
    fn parses_memory_operand_with_negative_displacement() {
        let (_, stmt) = parse("LOAD R1, [R2-8]");
        assert_eq!(
            stmt,
            Stmt::Instruction(InstrNode {
                mnemonic: "LOAD".into(),
                operands: vec![
                    OperandNode::Reg(1),
                    OperandNode::Mem { base: 2, disp: ValueExpr::Number(-8) }
                ],
            })
        );
    }

    #[test]
    fn parses_bare_label_only() {
        let (label, stmt) = parse("start:");
        assert_eq!(label, Some("start".into()));
        assert_eq!(stmt, Stmt::Empty);
    }

    #[test]
    fn parses_data_directive() {
        let (_, stmt) = parse(".byte 1, 2, 0xFF");
        assert_eq!(
            stmt,
            Stmt::Data {
                width: DataWidth::Byte,
                values: vec![ValueExpr::Number(1), ValueExpr::Number(2), ValueExpr::Number(0xFF)],
            }
        );
    }
}
