// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Recursive-descent parser: `.vf` source text -> [`vf_ast::Module`].
//!
//! Untrusted, like the rest of the front end (see `vf-lexer`'s crate
//! docs): a parser bug can produce the wrong AST, never an unsound
//! proof, since `vf-kernel` re-checks whatever `vf-elab` eventually
//! builds from that AST.
//!
//! Grammar (roughly, precedence lowest to highest):
//!
//! ```text
//! module   := decl*
//! decl     := 'axiom' IDENT ':' expr
//!           | 'def' IDENT ':' expr ':=' expr
//!           | 'theorem' IDENT ':' expr ':=' expr
//! expr     := arrow ('=' arrow)?
//! arrow    := app ('->' arrow)?                    -- right-associative
//! app      := atom+                                -- left-associative
//! atom     := IDENT | NAT | 'Prop' | 'Type' NAT?
//!           | '(' expr ')'
//!           | 'fun' '(' IDENT ':' expr ')' '=>' expr
//!           | 'forall' '(' IDENT ':' expr ')' ',' expr
//!           | 'let' IDENT ':' expr ':=' expr 'in' expr
//! ```
#![forbid(unsafe_code)]

use std::fmt;
use vf_ast::{Decl, Expr, Module, SortLit};
use vf_lexer::{lex, LexError, Span, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.span.line, self.span.col, self.message)
    }
}
impl std::error::Error for ParseError {}

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
        // `lex` always terminates with Eof and we never advance past
        // it, so this index is always in range.
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, kind: &TokenKind, what: &str) -> Result<Token, ParseError> {
        if &self.peek().kind == kind {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("expected {what}, found {:?}", self.peek().kind),
                span: self.peek().span,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Ident(name) => {
                self.advance();
                Ok((name, tok.span))
            }
            other => Err(ParseError {
                message: format!("expected an identifier, found {other:?}"),
                span: tok.span,
            }),
        }
    }

    fn parse_module(&mut self) -> Result<Module, ParseError> {
        let mut decls = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            decls.push(self.parse_decl()?);
        }
        Ok(Module { decls })
    }

    fn parse_decl(&mut self) -> Result<Decl, ParseError> {
        let start = self.peek().span;
        match &self.peek().kind {
            TokenKind::KwAxiom => {
                self.advance();
                let (name, _) = self.expect_ident()?;
                self.expect(&TokenKind::Colon, "':'")?;
                let ty = self.parse_expr()?;
                let span = join(start, ty.span());
                Ok(Decl::Axiom { name, ty, span })
            }
            TokenKind::KwDef => {
                self.advance();
                let (name, _) = self.expect_ident()?;
                self.expect(&TokenKind::Colon, "':'")?;
                let ty = self.parse_expr()?;
                self.expect(&TokenKind::ColonEq, "':='")?;
                let value = self.parse_expr()?;
                let span = join(start, value.span());
                Ok(Decl::Def {
                    name,
                    ty,
                    value,
                    span,
                })
            }
            TokenKind::KwTheorem => {
                self.advance();
                let (name, _) = self.expect_ident()?;
                self.expect(&TokenKind::Colon, "':'")?;
                let ty = self.parse_expr()?;
                self.expect(&TokenKind::ColonEq, "':='")?;
                let proof = self.parse_expr()?;
                let span = join(start, proof.span());
                Ok(Decl::Theorem {
                    name,
                    ty,
                    proof,
                    span,
                })
            }
            other => Err(ParseError {
                message: format!(
                    "expected a declaration ('axiom', 'def', or 'theorem'), found {other:?}"
                ),
                span: start,
            }),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_arrow()?;
        if self.peek().kind == TokenKind::Eq {
            self.advance();
            let rhs = self.parse_arrow()?;
            let span = join(lhs.span(), rhs.span());
            return Ok(Expr::Eq(Box::new(lhs), Box::new(rhs), span));
        }
        Ok(lhs)
    }

    fn parse_arrow(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_app()?;
        if self.peek().kind == TokenKind::Arrow {
            self.advance();
            let rhs = self.parse_arrow()?; // right-associative
            let span = join(lhs.span(), rhs.span());
            return Ok(Expr::Pi {
                name: None,
                domain: Box::new(lhs),
                codomain: Box::new(rhs),
                span,
            });
        }
        Ok(lhs)
    }

    fn starts_atom(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Ident(_)
                | TokenKind::Nat(_)
                | TokenKind::KwProp
                | TokenKind::KwType
                | TokenKind::LParen
                | TokenKind::KwFun
                | TokenKind::KwForall
                | TokenKind::KwLet
        )
    }

    fn parse_app(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_atom()?;
        while Self::starts_atom(&self.peek().kind) {
            let arg = self.parse_atom()?;
            let span = join(expr.span(), arg.span());
            expr = Expr::App(Box::new(expr), Box::new(arg), span);
        }
        Ok(expr)
    }

    fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Expr::Ident(name, tok.span))
            }
            TokenKind::Nat(n) => {
                self.advance();
                Ok(Expr::NatLit(n, tok.span))
            }
            TokenKind::KwProp => {
                self.advance();
                Ok(Expr::Sort(SortLit::Prop, tok.span))
            }
            TokenKind::KwType => {
                self.advance();
                if let TokenKind::Nat(n) = self.peek().kind {
                    let end = self.peek().span;
                    self.advance();
                    Ok(Expr::Sort(SortLit::Type(n as u32), join(tok.span, end)))
                } else {
                    Ok(Expr::Sort(SortLit::Type(0), tok.span))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(&TokenKind::RParen, "')'")?;
                Ok(inner)
            }
            TokenKind::KwFun => {
                self.advance();
                self.expect(&TokenKind::LParen, "'(' after 'fun'")?;
                let (name, _) = self.expect_ident()?;
                self.expect(&TokenKind::Colon, "':' in fun binder")?;
                let ty = self.parse_expr()?;
                self.expect(&TokenKind::RParen, "')' closing fun binder")?;
                self.expect(&TokenKind::FatArrow, "'=>' after fun binder")?;
                let body = self.parse_expr()?;
                let span = join(tok.span, body.span());
                Ok(Expr::Lam {
                    name,
                    ty: Box::new(ty),
                    body: Box::new(body),
                    span,
                })
            }
            TokenKind::KwForall => {
                self.advance();
                self.expect(&TokenKind::LParen, "'(' after 'forall'")?;
                let (name, _) = self.expect_ident()?;
                self.expect(&TokenKind::Colon, "':' in forall binder")?;
                let domain = self.parse_expr()?;
                self.expect(&TokenKind::RParen, "')' closing forall binder")?;
                self.expect(&TokenKind::Comma, "',' after forall binder")?;
                let codomain = self.parse_expr()?;
                let span = join(tok.span, codomain.span());
                Ok(Expr::Pi {
                    name: Some(name),
                    domain: Box::new(domain),
                    codomain: Box::new(codomain),
                    span,
                })
            }
            TokenKind::KwLet => {
                self.advance();
                let (name, _) = self.expect_ident()?;
                self.expect(&TokenKind::Colon, "':' in let binding")?;
                let ty = self.parse_expr()?;
                self.expect(&TokenKind::ColonEq, "':=' in let binding")?;
                let value = self.parse_expr()?;
                self.expect(&TokenKind::KwIn, "'in' after let binding")?;
                let body = self.parse_expr()?;
                let span = join(tok.span, body.span());
                Ok(Expr::Let {
                    name,
                    ty: Box::new(ty),
                    value: Box::new(value),
                    body: Box::new(body),
                    span,
                })
            }
            other => Err(ParseError {
                message: format!("expected an expression, found {other:?}"),
                span: tok.span,
            }),
        }
    }
}

/// Parse a full `.vf` module from source text.
pub fn parse_module(src: &str) -> Result<Module, ParseError> {
    let tokens = lex(src)?;
    let mut parser = Parser { tokens, pos: 0 };
    let module = parser.parse_module()?;
    Ok(module)
}

/// Parse a single expression (mainly useful for tests/tools that
/// don't need a whole module). Fails if there is unconsumed input
/// after the expression.
pub fn parse_expr(src: &str) -> Result<Expr, ParseError> {
    let tokens = lex(src)?;
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.parse_expr()?;
    if parser.peek().kind != TokenKind::Eof {
        return Err(ParseError {
            message: format!("unexpected trailing token {:?}", parser.peek().kind),
            span: parser.peek().span,
        });
    }
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_bare_identifier() {
        let e = parse_expr("x").unwrap();
        assert!(matches!(e, Expr::Ident(name, _) if name == "x"));
    }

    #[test]
    fn parses_sorts() {
        assert!(matches!(
            parse_expr("Prop").unwrap(),
            Expr::Sort(SortLit::Prop, _)
        ));
        assert!(matches!(
            parse_expr("Type").unwrap(),
            Expr::Sort(SortLit::Type(0), _)
        ));
        assert!(matches!(
            parse_expr("Type 3").unwrap(),
            Expr::Sort(SortLit::Type(3), _)
        ));
    }

    #[test]
    fn application_is_left_associative() {
        let e = parse_expr("f a b").unwrap();
        // f a b == App(App(f, a), b)
        match e {
            Expr::App(inner, b, _) => {
                assert!(matches!(*b, Expr::Ident(ref n, _) if n == "b"));
                match *inner {
                    Expr::App(f, a, _) => {
                        assert!(matches!(*f, Expr::Ident(ref n, _) if n == "f"));
                        assert!(matches!(*a, Expr::Ident(ref n, _) if n == "a"));
                    }
                    other => panic!("expected nested App, got {other:?}"),
                }
            }
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn parenthesized_expression_overrides_application_grouping() {
        let e = parse_expr("f (a b)").unwrap();
        match e {
            Expr::App(_, arg, _) => {
                assert!(matches!(*arg, Expr::App(_, _, _)));
            }
            other => panic!("expected App, got {other:?}"),
        }
    }

    #[test]
    fn arrow_is_right_associative_and_desugars_to_a_non_dependent_pi() {
        let e = parse_expr("A -> B -> C").unwrap();
        match e {
            Expr::Pi {
                name: None,
                domain,
                codomain,
                ..
            } => {
                assert!(matches!(*domain, Expr::Ident(ref n, _) if n == "A"));
                assert!(matches!(*codomain, Expr::Pi { name: None, .. }));
            }
            other => panic!("expected Pi, got {other:?}"),
        }
    }

    #[test]
    fn parses_fun_lambda() {
        let e = parse_expr("fun (x : Prop) => x").unwrap();
        match e {
            Expr::Lam { name, ty, body, .. } => {
                assert_eq!(name, "x");
                assert!(matches!(*ty, Expr::Sort(SortLit::Prop, _)));
                assert!(matches!(*body, Expr::Ident(ref n, _) if n == "x"));
            }
            other => panic!("expected Lam, got {other:?}"),
        }
    }

    #[test]
    fn parses_forall_pi_with_explicit_binder_name() {
        let e = parse_expr("forall (x : Prop), x").unwrap();
        match e {
            Expr::Pi { name, .. } => assert_eq!(name.as_deref(), Some("x")),
            other => panic!("expected Pi, got {other:?}"),
        }
    }

    #[test]
    fn parses_let_expression() {
        let e = parse_expr("let x : Prop := y in x").unwrap();
        match e {
            Expr::Let {
                name,
                ty,
                value,
                body,
                ..
            } => {
                assert_eq!(name, "x");
                assert!(matches!(*ty, Expr::Sort(SortLit::Prop, _)));
                assert!(matches!(*value, Expr::Ident(ref n, _) if n == "y"));
                assert!(matches!(*body, Expr::Ident(ref n, _) if n == "x"));
            }
            other => panic!("expected Let, got {other:?}"),
        }
    }

    #[test]
    fn parses_equality() {
        let e = parse_expr("a = b").unwrap();
        assert!(matches!(e, Expr::Eq(_, _, _)));
    }

    #[test]
    fn equality_binds_looser_than_arrow_and_application() {
        // `f a = g b` must parse as `(f a) = (g b)`, not `f (a = g) b`
        // or similar -- equality is the loosest-binding operator.
        let e = parse_expr("f a = g b").unwrap();
        match e {
            Expr::Eq(lhs, rhs, _) => {
                assert!(matches!(*lhs, Expr::App(_, _, _)));
                assert!(matches!(*rhs, Expr::App(_, _, _)));
            }
            other => panic!("expected Eq, got {other:?}"),
        }
    }

    #[test]
    fn parses_a_full_axiom_declaration() {
        let m = parse_module("axiom foo : Prop").unwrap();
        assert_eq!(m.decls.len(), 1);
        match &m.decls[0] {
            Decl::Axiom { name, ty, .. } => {
                assert_eq!(name, "foo");
                assert!(matches!(ty, Expr::Sort(SortLit::Prop, _)));
            }
            other => panic!("expected Axiom, got {other:?}"),
        }
    }

    #[test]
    fn parses_a_full_def_declaration() {
        let m = parse_module("def id : Prop -> Prop := fun (x : Prop) => x").unwrap();
        assert_eq!(m.decls.len(), 1);
        assert_eq!(m.decls[0].name(), "id");
        assert!(matches!(m.decls[0], Decl::Def { .. }));
    }

    #[test]
    fn parses_a_full_theorem_declaration() {
        let m = parse_module("theorem t : Prop := fun (x : Prop) => x").unwrap();
        assert!(matches!(m.decls[0], Decl::Theorem { .. }));
    }

    #[test]
    fn parses_multiple_declarations_in_one_module() {
        let m = parse_module("axiom a : Prop\ndef b : Prop := a").unwrap();
        assert_eq!(m.decls.len(), 2);
    }

    #[test]
    fn empty_module_parses_to_zero_declarations() {
        let m = parse_module("").unwrap();
        assert_eq!(m.decls.len(), 0);
    }

    #[test]
    fn missing_colon_in_axiom_is_a_parse_error_not_a_panic() {
        let err = parse_module("axiom foo Prop").unwrap_err();
        assert!(err.message.contains("':'"));
    }

    #[test]
    fn unclosed_paren_is_a_parse_error() {
        let err = parse_expr("(a b").unwrap_err();
        assert!(err.message.contains("')'"));
    }

    #[test]
    fn trailing_garbage_after_an_expression_is_rejected() {
        let err = parse_expr("a b )").unwrap_err();
        assert!(err.message.contains("trailing"));
    }

    #[test]
    fn lexer_errors_propagate_as_parse_errors() {
        let err = parse_expr("a @ b").unwrap_err();
        assert!(err.message.contains('@'));
    }
}
