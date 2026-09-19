//! Surface-syntax AST for the `.vf` theorem language.
//!
//! Binders here carry ordinary string names, unlike `vf-core::Term`'s
//! locally-nameless (de Bruijn + interned free variables)
//! representation. `vf-elab` is the (untrusted) bridge between the
//! two: it resolves names to either a de Bruijn index (a binder in
//! scope) or a `vf-core::Const` (a global declaration), and reports a
//! name-resolution error for anything else -- it never guesses.
#![forbid(unsafe_code)]

use vf_lexer::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortLit {
    Prop,
    /// `Type` (bare) is `Type(0)`; `Type n` is `Type(n)`.
    Type(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Ident(String, Span),
    Sort(SortLit, Span),
    NatLit(u64, Span),
    App(Box<Expr>, Box<Expr>, Span),
    Lam {
        name: String,
        ty: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    Pi {
        /// `None` for the `A -> B` non-dependent sugar, `Some(name)`
        /// for an explicit `forall (name : A), B`.
        name: Option<String>,
        domain: Box<Expr>,
        codomain: Box<Expr>,
        span: Span,
    },
    Let {
        name: String,
        ty: Box<Expr>,
        value: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    Eq(Box<Expr>, Box<Expr>, Span),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Ident(_, s)
            | Expr::Sort(_, s)
            | Expr::NatLit(_, s)
            | Expr::App(_, _, s)
            | Expr::Lam { span: s, .. }
            | Expr::Pi { span: s, .. }
            | Expr::Let { span: s, .. }
            | Expr::Eq(_, _, s) => *s,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decl {
    Axiom {
        name: String,
        ty: Expr,
        span: Span,
    },
    Def {
        name: String,
        ty: Expr,
        value: Expr,
        span: Span,
    },
    Theorem {
        name: String,
        ty: Expr,
        proof: Expr,
        span: Span,
    },
}

impl Decl {
    pub fn name(&self) -> &str {
        match self {
            Decl::Axiom { name, .. } | Decl::Def { name, .. } | Decl::Theorem { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Module {
    pub decls: Vec<Decl>,
}
