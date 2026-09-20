//! Recursive-descent parser for the restricted Rust verification
//! language.
//!
//! ```text
//! program    := function*
//! function   := 'fn' IDENT '(' (param (',' param)*)? ')' '->' ty
//!               contract* block
//! param      := IDENT ':' ty
//! ty         := 'i64' | 'bool'
//! contract   := 'requires' expr | 'ensures' expr
//! block      := '{' stmt* '}'
//! stmt       := 'let' 'mut'? IDENT (':' ty)? '=' expr ';'
//!             | IDENT '=' expr ';'
//!             | 'if' expr block ('else' block)?
//!             | 'while' expr loop_contract* block
//!             | 'return' expr? ';'
//!             | expr ';'
//! loop_contract := 'invariant' expr | 'decreases' expr
//! expr       := logic_or
//! logic_or   := logic_and ('||' logic_and)*
//! logic_and  := equality ('&&' equality)*
//! equality   := comparison (('==' | '!=') comparison)*
//! comparison := additive (('<' | '<=' | '>' | '>=') additive)*
//! additive   := multiplicative (('+' | '-') multiplicative)*
//! multiplicative := unary (('*' | '/' | '%') unary)*
//! unary      := ('-' | '!') unary | primary
//! primary    := INT | 'true' | 'false' | 'result'
//!             | IDENT ('(' (expr (',' expr)*)? ')')?
//!             | '(' expr ')'
//! ```
use crate::ast::{BinOp, Block, Expr, Function, Param, Program, Stmt, Ty, UnOp};
use crate::lexer::{lex, LexError, Span, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        ParseError {
            message: e.message,
            span: e.span,
        }
    }
}

fn join(a: Span, b: Span) -> Span {
    Span {
        start: a.start,
        end: b.end,
        line: a.line,
        col: a.col,
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn check(&self, kind: &TokenKind) -> bool {
        &self.peek().kind == kind
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, ParseError> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("expected {kind:?}, found {:?}", self.peek().kind),
                span: self.peek().span,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        match self.peek().kind.clone() {
            TokenKind::Ident(name) => {
                let span = self.peek().span;
                self.advance();
                Ok((name, span))
            }
            other => Err(ParseError {
                message: format!("expected an identifier, found {other:?}"),
                span: self.peek().span,
            }),
        }
    }

    fn parse_ty(&mut self) -> Result<Ty, ParseError> {
        match self.peek().kind {
            TokenKind::KwI64 => {
                self.advance();
                Ok(Ty::I64)
            }
            TokenKind::KwBool => {
                self.advance();
                Ok(Ty::Bool)
            }
            _ => Err(ParseError {
                message: format!(
                    "expected a type (i64 or bool), found {:?}",
                    self.peek().kind
                ),
                span: self.peek().span,
            }),
        }
    }

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut functions = Vec::new();
        while !self.check(&TokenKind::Eof) {
            functions.push(self.parse_function()?);
        }
        Ok(Program { functions })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let start = self.expect(TokenKind::KwFn)?.span;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let (pname, _) = self.expect_ident()?;
                self.expect(TokenKind::Colon)?;
                let ty = self.parse_ty()?;
                params.push(Param { name: pname, ty });
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Arrow)?;
        let ret = self.parse_ty()?;

        let mut requires = Vec::new();
        let mut ensures = Vec::new();
        loop {
            match self.peek().kind {
                TokenKind::KwRequires => {
                    self.advance();
                    requires.push(self.parse_expr()?);
                }
                TokenKind::KwEnsures => {
                    self.advance();
                    ensures.push(self.parse_expr()?);
                }
                _ => break,
            }
        }

        let body = self.parse_block()?;
        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map_or(start, |t| t.span);
        Ok(Function {
            name,
            params,
            ret,
            requires,
            ensures,
            body,
            span: join(start, end),
        })
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Block { stmts })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek().kind.clone() {
            TokenKind::KwLet => {
                let start = self.advance().span;
                let mutable = if self.check(&TokenKind::KwMut) {
                    self.advance();
                    true
                } else {
                    false
                };
                let (name, _) = self.expect_ident()?;
                let ty = if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(self.parse_ty()?)
                } else {
                    None
                };
                self.expect(TokenKind::Eq)?;
                let value = self.parse_expr()?;
                let end = self.expect(TokenKind::Semi)?.span;
                Ok(Stmt::Let {
                    name,
                    mutable,
                    ty,
                    value,
                    span: join(start, end),
                })
            }
            TokenKind::KwIf => {
                let start = self.advance().span;
                let cond = self.parse_expr()?;
                let then_branch = self.parse_block()?;
                let else_branch = if self.check(&TokenKind::KwElse) {
                    self.advance();
                    Some(self.parse_block()?)
                } else {
                    None
                };
                let end = self.tokens[self.pos.saturating_sub(1)].span;
                Ok(Stmt::If {
                    cond,
                    then_branch,
                    else_branch,
                    span: join(start, end),
                })
            }
            TokenKind::KwWhile => {
                let start = self.advance().span;
                let cond = self.parse_expr()?;
                let mut invariants = Vec::new();
                let mut decreases = None;
                loop {
                    match self.peek().kind {
                        TokenKind::KwInvariant => {
                            self.advance();
                            invariants.push(self.parse_expr()?);
                        }
                        TokenKind::KwDecreases => {
                            self.advance();
                            decreases = Some(self.parse_expr()?);
                        }
                        _ => break,
                    }
                }
                let body = self.parse_block()?;
                let end = self.tokens[self.pos.saturating_sub(1)].span;
                Ok(Stmt::While {
                    cond,
                    invariants,
                    decreases,
                    body,
                    span: join(start, end),
                })
            }
            TokenKind::KwReturn => {
                let start = self.advance().span;
                let value = if self.check(&TokenKind::Semi) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                let end = self.expect(TokenKind::Semi)?.span;
                Ok(Stmt::Return(value, join(start, end)))
            }
            TokenKind::Ident(_) if self.peek_is_assignment() => {
                let (name, start) = self.expect_ident()?;
                self.expect(TokenKind::Eq)?;
                let value = self.parse_expr()?;
                let end = self.expect(TokenKind::Semi)?.span;
                Ok(Stmt::Assign {
                    name,
                    value,
                    span: join(start, end),
                })
            }
            _ => {
                let e = self.parse_expr()?;
                self.expect(TokenKind::Semi)?;
                Ok(Stmt::Expr(e))
            }
        }
    }

    /// Whether the current `Ident` token is immediately followed by
    /// `=` (a plain assignment) as opposed to `(` (a call) or any
    /// binary operator (an expression statement) -- distinguishing
    /// `x = ...;` from `f(...);` without backtracking.
    fn peek_is_assignment(&self) -> bool {
        matches!(
            self.tokens.get(self.pos + 1).map(|t| &t.kind),
            Some(TokenKind::Eq)
        )
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_logic_or()
    }

    fn parse_logic_or(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_logic_and()?;
        while self.check(&TokenKind::OrOr) {
            self.advance();
            let rhs = self.parse_logic_and()?;
            let span = join(lhs.span(), rhs.span());
            lhs = Expr::Binary(BinOp::Or, Box::new(lhs), Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_logic_and(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_equality()?;
        while self.check(&TokenKind::AndAnd) {
            self.advance();
            let rhs = self.parse_equality()?;
            let span = join(lhs.span(), rhs.span());
            lhs = Expr::Binary(BinOp::And, Box::new(lhs), Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_comparison()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::Ne => BinOp::Ne,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_comparison()?;
            let span = join(lhs.span(), rhs.span());
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_additive()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_additive()?;
            let span = join(lhs.span(), rhs.span());
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_multiplicative()?;
            let span = join(lhs.span(), rhs.span());
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Rem,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_unary()?;
            let span = join(lhs.span(), rhs.span());
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().kind {
            TokenKind::Minus => {
                let start = self.advance().span;
                let e = self.parse_unary()?;
                let span = join(start, e.span());
                Ok(Expr::Unary(UnOp::Neg, Box::new(e), span))
            }
            TokenKind::Bang => {
                let start = self.advance().span;
                let e = self.parse_unary()?;
                let span = join(start, e.span());
                Ok(Expr::Unary(UnOp::Not, Box::new(e), span))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().kind.clone() {
            TokenKind::IntLit(n) => {
                let span = self.advance().span;
                Ok(Expr::IntLit(n, span))
            }
            TokenKind::KwTrue => {
                let span = self.advance().span;
                Ok(Expr::BoolLit(true, span))
            }
            TokenKind::KwFalse => {
                let span = self.advance().span;
                Ok(Expr::BoolLit(false, span))
            }
            TokenKind::Ident(name) => {
                let start = self.advance().span;
                if name == "result" {
                    return Ok(Expr::Result(start));
                }
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if self.check(&TokenKind::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    let end = self.expect(TokenKind::RParen)?.span;
                    Ok(Expr::Call(name, args, join(start, end)))
                } else {
                    Ok(Expr::Var(name, start))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            }
            other => Err(ParseError {
                message: format!("expected an expression, found {other:?}"),
                span: self.peek().span,
            }),
        }
    }
}

/// Parse a full program (zero or more function definitions).
pub fn parse_program(src: &str) -> Result<Program, ParseError> {
    let tokens = lex(src)?;
    let mut parser = Parser { tokens, pos: 0 };
    let program = parser.parse_program()?;
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_function_with_no_contracts() {
        let program = parse_program("fn add(x: i64, y: i64) -> i64 { return x + y; }").unwrap();
        assert_eq!(program.functions.len(), 1);
        let f = &program.functions[0];
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert!(f.requires.is_empty());
        assert!(f.ensures.is_empty());
    }

    #[test]
    fn parses_requires_and_ensures() {
        let src = "fn abs(x: i64) -> i64 requires x != 0 ensures result >= 0 { return x; }";
        let program = parse_program(src).unwrap();
        let f = &program.functions[0];
        assert_eq!(f.requires.len(), 1);
        assert_eq!(f.ensures.len(), 1);
        assert!(matches!(f.ensures[0], Expr::Binary(BinOp::Ge, _, _, _)));
    }

    #[test]
    fn parses_if_else() {
        let src = "fn f(x: i64) -> i64 { if x < 0 { return 0; } else { return x; } }";
        let program = parse_program(src).unwrap();
        let stmt = &program.functions[0].body.stmts[0];
        assert!(matches!(
            stmt,
            Stmt::If {
                else_branch: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn parses_while_with_invariant_and_decreases() {
        let src = "fn f(n: i64) -> i64 { let mut i = 0; while i < n invariant i >= 0 decreases n - i { i = i + 1; } return i; }";
        let program = parse_program(src).unwrap();
        let stmt = &program.functions[0].body.stmts[1];
        match stmt {
            Stmt::While {
                invariants,
                decreases,
                ..
            } => {
                assert_eq!(invariants.len(), 1);
                assert!(decreases.is_some());
            }
            other => panic!("expected a While statement, got {other:?}"),
        }
    }

    #[test]
    fn parses_let_mut_and_assignment() {
        let src = "fn f() -> i64 { let mut x = 1; x = x + 1; return x; }";
        let program = parse_program(src).unwrap();
        assert!(matches!(
            program.functions[0].body.stmts[0],
            Stmt::Let { mutable: true, .. }
        ));
        assert!(matches!(
            program.functions[0].body.stmts[1],
            Stmt::Assign { .. }
        ));
    }

    #[test]
    fn parses_a_call_expression_statement() {
        let src = "fn f() -> i64 { g(1, 2); return 0; }";
        let program = parse_program(src).unwrap();
        match &program.functions[0].body.stmts[0] {
            Stmt::Expr(Expr::Call(name, args, _)) => {
                assert_eq!(name, "g");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected a call expression statement, got {other:?}"),
        }
    }

    #[test]
    fn result_is_its_own_expr_variant_not_a_plain_var() {
        let src = "fn f() -> i64 ensures result == 0 { return 0; }";
        let program = parse_program(src).unwrap();
        assert!(matches!(
            program.functions[0].ensures[0],
            Expr::Binary(BinOp::Eq, _, _, _)
        ));
        match &program.functions[0].ensures[0] {
            Expr::Binary(_, lhs, _, _) => assert!(matches!(**lhs, Expr::Result(_))),
            _ => unreachable!(),
        }
    }

    #[test]
    fn operator_precedence_matches_arithmetic_conventions() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3), not (1 + 2) * 3.
        let e = {
            let program = parse_program("fn f() -> i64 { return 1 + 2 * 3; }").unwrap();
            match &program.functions[0].body.stmts[0] {
                Stmt::Return(Some(e), _) => e.clone(),
                other => panic!("expected a return expression, got {other:?}"),
            }
        };
        match e {
            Expr::Binary(BinOp::Add, lhs, rhs, _) => {
                assert!(matches!(*lhs, Expr::IntLit(1, _)));
                assert!(matches!(*rhs, Expr::Binary(BinOp::Mul, _, _, _)));
            }
            other => panic!("expected a top-level Add, got {other:?}"),
        }
    }

    #[test]
    fn logical_and_binds_tighter_than_logical_or() {
        let e = {
            let program =
                parse_program("fn f(a: bool, b: bool, c: bool) -> bool { return a || b && c; }")
                    .unwrap();
            match &program.functions[0].body.stmts[0] {
                Stmt::Return(Some(e), _) => e.clone(),
                other => panic!("expected a return expression, got {other:?}"),
            }
        };
        match e {
            Expr::Binary(BinOp::Or, lhs, rhs, _) => {
                assert!(matches!(*lhs, Expr::Var(_, _)));
                assert!(matches!(*rhs, Expr::Binary(BinOp::And, _, _, _)));
            }
            other => panic!("expected a top-level Or, got {other:?}"),
        }
    }

    #[test]
    fn missing_semicolon_is_a_parse_error_not_a_panic() {
        let err = parse_program("fn f() -> i64 { return 0 }").unwrap_err();
        assert!(err.message.contains("Semi") || err.message.contains(';'));
    }

    #[test]
    fn lexer_errors_propagate_as_parse_errors() {
        let err = parse_program("fn f() -> i64 { return $; }").unwrap_err();
        assert!(err.message.contains('$'));
    }
}
