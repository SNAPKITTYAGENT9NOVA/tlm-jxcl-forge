// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Restricted Rust verification language: a small, deliberately
//! limited subset of Rust syntax (`i64`/`bool` values, `if`/`while`,
//! `let`/assignment, function calls), a parser for it, a MIR-like IR
//! it lowers to, and a contract system (`requires`/`ensures` on
//! functions, `invariant`/`decreases` on loops).
//!
//! ## What this crate is (and isn't)
//!
//! This is a *frontend* only: lexing, parsing, and lowering to an
//! explicit control-flow graph. It does not check that a function's
//! contracts actually hold -- turning this IR plus its contracts into
//! proof obligations (and discharging them, whether through
//! `vf-kernel`'s own proof terms or an external oracle) is later
//! crates' job (`vf-smt`/`vf-kani`, next in verification-forge's
//! implementation order). Nothing here should be read as having
//! verified anything by itself: a program that parses and lowers
//! cleanly has only been shown to be *syntactically well-formed*, not
//! correct.
//!
//! ## Known, documented scope limits (not bugs)
//!
//! - Only two types: `i64` and `bool`. No structs, arrays, references,
//!   or generics.
//! - Function calls are lowered but not interprocedurally analyzed: a
//!   call's result is modeled as an arbitrary (unconstrained) value of
//!   type `i64`, and its own `requires`/`ensures` are not connected to
//!   the call site. Wiring that up is exactly the kind of thing a
//!   verification backend (not this frontend) would need to do
//!   deliberately, not something to fake here.
//! - `decreases` measures are parsed and carried through to the MIR
//!   (`mir::Function::loop_decreases`) but nothing in this crate
//!   checks that they actually decrease -- there is no termination
//!   checker here, only a place to attach the measure a future one
//!   would consume.
#![forbid(unsafe_code)]

pub mod ast;
pub mod lexer;
pub mod lower;
pub mod mir;
pub mod parser;

pub use lower::{lower_program, LowerError};
pub use parser::{parse_program, ParseError};

/// Parse and lower `src` in one step -- the common case for a caller
/// that doesn't need the intermediate surface AST.
pub fn compile(src: &str) -> Result<mir::Program, CompileError> {
    let program = parse_program(src)?;
    let mir = lower_program(&program)?;
    Ok(mir)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    Parse(ParseError),
    Lower(LowerError),
}

impl From<ParseError> for CompileError {
    fn from(e: ParseError) -> Self {
        CompileError::Parse(e)
    }
}
impl From<LowerError> for CompileError {
    fn from(e: LowerError) -> Self {
        CompileError::Lower(e)
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Parse(e) => write!(f, "parse error: {}", e.message),
            CompileError::Lower(e) => write!(f, "lowering error: {e}"),
        }
    }
}
impl std::error::Error for CompileError {}

#[cfg(test)]
mod tests {
    use super::*;
    use mir::Terminator;

    #[test]
    fn compiles_a_simple_function() {
        let program = compile("fn add(x: i64, y: i64) -> i64 { return x + y; }").unwrap();
        assert_eq!(program.functions.len(), 1);
        let f = &program.functions[0];
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
    }

    #[test]
    fn if_else_lowers_to_a_diamond_shaped_cfg() {
        let program =
            compile("fn f(x: i64) -> i64 { if x < 0 { return 0; } else { return x; } }").unwrap();
        let f = &program.functions[0];
        // The entry block (block 0) ends in a SwitchInt over the two branches.
        let entry = f.block(mir::BlockId(0));
        assert!(matches!(entry.terminator, Terminator::SwitchInt { .. }));
    }

    #[test]
    fn while_loop_creates_a_back_edge_and_records_its_invariant() {
        let program = compile(
            "fn f(n: i64) -> i64 { let mut i = 0; while i < n invariant i >= 0 { i = i + 1; } return i; }",
        )
        .unwrap();
        let f = &program.functions[0];
        assert_eq!(f.loop_invariants.len(), 1);
        let (_, invariants) = f.loop_invariants.iter().next().unwrap();
        assert_eq!(invariants.len(), 1);

        // The loop body's block must Goto back to the header (a back-edge).
        let has_back_edge = f.blocks.iter().any(|b| {
            matches!(b.terminator, Terminator::Goto(target) if f.loop_invariants.contains_key(&target.0))
        });
        assert!(
            has_back_edge,
            "expected some block to Goto back to the loop header"
        );
    }

    #[test]
    fn decreases_measure_is_recorded_on_the_loop_header() {
        let program =
            compile("fn f(n: i64) -> i64 { let mut i = 0; while i < n decreases n - i { i = i + 1; } return i; }")
                .unwrap();
        let f = &program.functions[0];
        assert_eq!(f.loop_decreases.len(), 1);
    }

    #[test]
    fn requires_and_ensures_are_carried_through_to_the_mir() {
        let program =
            compile("fn abs(x: i64) -> i64 requires x != 0 ensures result >= 0 { return x; }")
                .unwrap();
        let f = &program.functions[0];
        assert_eq!(f.requires.len(), 1);
        assert_eq!(f.ensures.len(), 1);
    }

    #[test]
    fn an_unbound_variable_is_a_lowering_error_not_a_panic() {
        let err = compile("fn f() -> i64 { return y; }").unwrap_err();
        assert_eq!(
            err,
            CompileError::Lower(LowerError::UnboundVariable("y".to_string()))
        );
    }

    #[test]
    fn a_call_to_an_undeclared_function_is_a_lowering_error() {
        let err = compile("fn f() -> i64 { return g(); }").unwrap_err();
        assert_eq!(
            err,
            CompileError::Lower(LowerError::UnknownFunction("g".to_string()))
        );
    }

    #[test]
    fn a_call_to_a_function_declared_later_in_the_same_program_is_accepted() {
        // Unlike vf-axioms's Registry (which enforces a strict
        // declare-before-use DAG for proof soundness), ordinary
        // function calls in a program are mutually visible --
        // `known_functions` is collected before any function is
        // lowered specifically to allow this.
        compile("fn f() -> i64 { return g(); } fn g() -> i64 { return 0; }").unwrap();
    }

    #[test]
    fn result_outside_a_contract_is_a_lowering_error() {
        let err = compile("fn f() -> i64 { return result; }").unwrap_err();
        assert_eq!(err, CompileError::Lower(LowerError::ResultOutsideContract));
    }

    #[test]
    fn duplicate_function_names_are_rejected() {
        let err = compile("fn f() -> i64 { return 0; } fn f() -> i64 { return 1; }").unwrap_err();
        assert_eq!(
            err,
            CompileError::Lower(LowerError::DuplicateFunction("f".to_string()))
        );
    }

    #[test]
    fn a_syntax_error_is_reported_as_a_parse_error() {
        let err = compile("fn f( -> i64 { return 0; }").unwrap_err();
        assert!(matches!(err, CompileError::Parse(_)));
    }
}
