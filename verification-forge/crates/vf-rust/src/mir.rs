// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A MIR-like IR: functions as a control-flow graph of basic blocks,
//! each a straight-line sequence of single-operation assignments
//! ending in one terminator -- deliberately modeled after rustc's MIR
//! shape (`Local`/`Place`-style operands, `BasicBlock`, `Terminator`),
//! but only as much of it as a small deductive verifier needs: no
//! borrows, no drops, no panics-as-control-flow. This is a lowering
//! *target*, not an execution format -- nothing in this crate
//! interprets it.
use crate::ast::{self, BinOp, Ty, UnOp};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Local(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDecl {
    pub ty: Ty,
    /// `None` for a compiler-introduced temporary (holds a
    /// subexpression's value on the way to somewhere else); `Some` for
    /// a local that came from a source-level parameter or `let`.
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Const {
    Int(i64),
    Bool(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Copy(Local),
    Const(Const),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rvalue {
    Use(Operand),
    UnaryOp(UnOp, Operand),
    BinaryOp(BinOp, Operand, Operand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Statement {
    Assign(Local, Rvalue),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminator {
    Goto(BlockId),
    SwitchInt {
        discr: Operand,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return,
    /// A block reachable only through dead code (e.g. statements
    /// lexically following an unconditional `return`). Never a panic
    /// at build time -- see `lower`'s doc comment.
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Local>,
    /// The distinguished "return place" (like rustc MIR's `_0`):
    /// assigned on every path before a `Terminator::Return`.
    pub ret_local: Local,
    pub locals: Vec<LocalDecl>,
    pub blocks: Vec<BasicBlock>,
    /// Preconditions, in surface-AST form (not lowered): a verifier
    /// consuming this IR needs them as *statements to prove*, not as
    /// more control flow to execute.
    pub requires: Vec<ast::Expr>,
    /// Postconditions; may reference `ast::Expr::Result`.
    pub ensures: Vec<ast::Expr>,
    /// Loop invariants, keyed by the loop header block's id.
    pub loop_invariants: BTreeMap<u32, Vec<ast::Expr>>,
    /// Loop termination measures, keyed by the loop header block's id.
    pub loop_decreases: BTreeMap<u32, ast::Expr>,
}

impl Function {
    pub fn local_decl(&self, local: Local) -> &LocalDecl {
        &self.locals[local.0 as usize]
    }

    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id.0 as usize]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Program {
    pub functions: Vec<Function>,
}
