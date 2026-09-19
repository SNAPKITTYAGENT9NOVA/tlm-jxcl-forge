//! A toy (explicitly documented, non-production) structural synthesis
//! pass lowering a narrow, honestly-scoped subset of the `jxcl-hdl` AST
//! into primitive-gate `jxcl-netlist` form.
//!
//! # Scope and honesty boundary
//!
//! This is **not** a production synthesis tool and does not attempt to
//! be one. There is no `iverilog`/`verilator`/`yosys` in this
//! environment (see `docs/HARDWARE_LIMITATIONS.md` at the workspace
//! root), so nothing here is checked by feeding it to a real logic
//! synthesizer — [`synthesize`] is a small, hand-written structural
//! transformation and its only "correctness" guarantee is this crate's
//! own unit tests plus the [`jxcl_netlist::Netlist::validate`] structural
//! check on its output.
//!
//! [`synthesize`] recognizes **exactly one shape**: a module whose one
//! top-level `case` statement
//!
//! - selects on a single 1-bit identifier (e.g. `sel`),
//! - has exactly two arms, for selector values `0` and `1` (no
//!   `default` arm), and
//! - each arm's body is a set of plain `target = identifier;`
//!   assignments, both arms assigning to exactly the same set of
//!   targets.
//!
//! That shape is precisely "a bank of independent 2:1 multiplexers" —
//! `jxcl-hdl`'s own canonical `mux2` example
//! (`jxcl_hdl::examples::mux2_module`) is the smallest instance of it,
//! and is this crate's primary test fixture. Any module outside this
//! shape (a wider selector, a `default` arm, a non-identifier
//! right-hand side, mismatched targets between arms, more than one
//! statement of a different kind in an arm) is rejected with a
//! descriptive [`SynthesisError`] rather than silently producing
//! something that looks plausible but isn't actually equivalent to the
//! source module.
#![forbid(unsafe_code)]

use jxcl_hdl::{CaseValue, Expr, Module, Statement};
use jxcl_netlist::{Gate, Netlist, Wire};
use std::collections::BTreeMap;

/// Why [`synthesize`] refused a module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SynthesisError {
    /// The module has no top-level `case` statement at all.
    NoCaseStatement,
    /// The case statement's selector wasn't a bare 1-bit identifier
    /// (e.g. it was a literal or a bit-slice expression).
    UnsupportedSelector,
    /// The case statement didn't have exactly two arms (this pass only
    /// lowers a 2:1 mux shape).
    UnsupportedArity(usize),
    /// One of the two arms used [`CaseValue::Default`] rather than an
    /// explicit `0`/`1` literal, or a literal value other than 0 or 1,
    /// or a literal wider than 1 bit.
    UnsupportedCaseValue,
    /// The case didn't have exactly one arm for selector value 0 and
    /// one for value 1.
    MissingArm(u64),
    /// An arm's body contained something other than
    /// `target = identifier;` assignments (e.g. a nested `case`, or an
    /// assignment whose right-hand side is a literal or bit-slice).
    UnsupportedStatement,
    /// The two arms didn't assign exactly the same set of targets, so
    /// there is no well-defined single mux per output.
    MismatchedTargets,
}

impl std::fmt::Display for SynthesisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SynthesisError::NoCaseStatement => {
                write!(f, "module has no top-level case statement to synthesize")
            }
            SynthesisError::UnsupportedSelector => write!(
                f,
                "this toy synthesis pass only supports a bare 1-bit identifier as the case selector"
            ),
            SynthesisError::UnsupportedArity(n) => write!(
                f,
                "this toy synthesis pass only supports exactly 2 case arms (a 2:1 mux shape), found {n}"
            ),
            SynthesisError::UnsupportedCaseValue => write!(
                f,
                "this toy synthesis pass only supports case arms with 1-bit literal values 0 and 1, no default arm"
            ),
            SynthesisError::MissingArm(v) => {
                write!(f, "case statement is missing the arm for selector value {v}")
            }
            SynthesisError::UnsupportedStatement => write!(
                f,
                "this toy synthesis pass only supports `target = identifier;` assignments inside a case arm"
            ),
            SynthesisError::MismatchedTargets => write!(
                f,
                "the two case arms must assign exactly the same set of output targets"
            ),
        }
    }
}

impl std::error::Error for SynthesisError {}

/// Lower `module`'s single top-level 2:1-mux-shaped `case` statement
/// (see the module-level docs for the exact shape this accepts) into a
/// [`Netlist`] of primitive [`Gate::mux`] gates, one per output target.
///
/// The module's own (non-port) [`jxcl_hdl::Signal`]s are carried over
/// as [`Wire`]s so a caller can see the full net list, but this pass
/// does not itself need them: a mux gate's inputs/outputs are just the
/// identifier names already used in the source `case` statement.
pub fn synthesize(module: &Module) -> Result<Netlist, SynthesisError> {
    let case = module
        .top_level_case()
        .ok_or(SynthesisError::NoCaseStatement)?;

    let (selector, arms) = match case {
        Statement::Case { selector, arms } => (selector, arms),
        _ => unreachable!("top_level_case only ever returns a Statement::Case"),
    };

    let sel_name = match selector {
        Expr::Ident(name) => name.clone(),
        _ => return Err(SynthesisError::UnsupportedSelector),
    };

    if arms.len() != 2 {
        return Err(SynthesisError::UnsupportedArity(arms.len()));
    }

    let mut by_value: BTreeMap<u64, &Vec<Statement>> = BTreeMap::new();
    for (value, body) in arms {
        let bit = match value {
            CaseValue::Literal { width: 1, value } if *value == 0 || *value == 1 => *value,
            _ => return Err(SynthesisError::UnsupportedCaseValue),
        };
        by_value.insert(bit, body);
    }
    let arm0 = by_value
        .get(&0)
        .ok_or(SynthesisError::MissingArm(0))?
        .as_slice();
    let arm1 = by_value
        .get(&1)
        .ok_or(SynthesisError::MissingArm(1))?
        .as_slice();

    let assigns0 = extract_ident_assigns(arm0)?;
    let assigns1 = extract_ident_assigns(arm1)?;

    let targets0: Vec<&String> = assigns0.iter().map(|(t, _)| t).collect();
    let targets1: Vec<&String> = assigns1.iter().map(|(t, _)| t).collect();
    if targets0.len() != targets1.len() {
        return Err(SynthesisError::MismatchedTargets);
    }

    let map1: BTreeMap<&String, &String> = assigns1.iter().map(|(t, s)| (t, s)).collect();

    let mut gates = Vec::with_capacity(assigns0.len());
    for (target, source0) in &assigns0 {
        let source1 = map1.get(target).ok_or(SynthesisError::MismatchedTargets)?;
        gates.push(Gate::mux(
            sel_name.clone(),
            source0.clone(),
            (*source1).clone(),
            target.clone(),
        ));
    }

    let wires = module
        .signals
        .iter()
        .map(|s| Wire::new(s.name.clone(), s.width))
        .collect();

    Ok(Netlist::new().with_gates(gates).with_wires(wires))
}

/// Pull `target = identifier;` pairs out of a case-arm body, rejecting
/// anything else this toy pass doesn't understand.
fn extract_ident_assigns(body: &[Statement]) -> Result<Vec<(String, String)>, SynthesisError> {
    body.iter()
        .map(|stmt| match stmt {
            Statement::Assign {
                target,
                expr: Expr::Ident(source),
            } => Ok((target.clone(), source.clone())),
            _ => Err(SynthesisError::UnsupportedStatement),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_hdl::examples::mux2_module;
    use jxcl_hdl::{CaseValue as CV, Expr as E, Module as M, Port, Sensitivity, Statement as S};
    use jxcl_netlist::GateKind;

    #[test]
    fn mux2_synthesizes_to_a_single_mux_gate() {
        let netlist = synthesize(&mux2_module()).expect("mux2 is in scope");
        assert_eq!(netlist.gates.len(), 1);
        let gate = &netlist.gates[0];
        assert_eq!(gate.kind, GateKind::Mux);
        assert_eq!(gate.inputs, vec!["sel", "a", "b"]);
        assert_eq!(gate.output, "y");
        assert!(netlist.wires.is_empty());
        netlist
            .validate()
            .expect("synthesized netlist is well-formed");
    }

    #[test]
    fn module_with_no_case_statement_is_rejected() {
        let m = M::new("no_case").with_ports(vec![Port::input("a", 1), Port::output("y", 1)]);
        assert_eq!(synthesize(&m), Err(SynthesisError::NoCaseStatement));
    }

    #[test]
    fn wide_selector_is_rejected() {
        // A 2-bit-wide selector case (four arms) is out of this toy
        // pass's documented 2:1-mux-only scope.
        let m = M::new("wide").with_statements(vec![S::always(
            Sensitivity::Combinational,
            vec![S::case_stmt(
                E::ident("sel"),
                vec![
                    (CV::literal(2, 0), vec![S::assign("y", E::ident("a"))]),
                    (CV::literal(2, 1), vec![S::assign("y", E::ident("b"))]),
                    (CV::literal(2, 2), vec![S::assign("y", E::ident("c"))]),
                    (CV::literal(2, 3), vec![S::assign("y", E::ident("d"))]),
                ],
            )],
        )]);
        assert_eq!(synthesize(&m), Err(SynthesisError::UnsupportedArity(4)));
    }

    #[test]
    fn non_identifier_selector_is_rejected() {
        let m = M::new("bad_sel").with_statements(vec![S::case_stmt(
            E::bit_slice(E::ident("word"), 0, 0),
            vec![
                (CV::literal(1, 0), vec![S::assign("y", E::ident("a"))]),
                (CV::literal(1, 1), vec![S::assign("y", E::ident("b"))]),
            ],
        )]);
        assert_eq!(synthesize(&m), Err(SynthesisError::UnsupportedSelector));
    }

    #[test]
    fn default_arm_is_rejected() {
        let m = M::new("with_default").with_statements(vec![S::case_stmt(
            E::ident("sel"),
            vec![
                (CV::literal(1, 0), vec![S::assign("y", E::ident("a"))]),
                (CV::Default, vec![S::assign("y", E::ident("b"))]),
            ],
        )]);
        assert_eq!(synthesize(&m), Err(SynthesisError::UnsupportedCaseValue));
    }

    #[test]
    fn literal_rhs_in_arm_is_rejected() {
        let m = M::new("literal_rhs").with_statements(vec![S::case_stmt(
            E::ident("sel"),
            vec![
                (CV::literal(1, 0), vec![S::assign("y", E::literal(1, 0))]),
                (CV::literal(1, 1), vec![S::assign("y", E::ident("b"))]),
            ],
        )]);
        assert_eq!(synthesize(&m), Err(SynthesisError::UnsupportedStatement));
    }

    #[test]
    fn mismatched_targets_between_arms_is_rejected() {
        let m = M::new("mismatched").with_statements(vec![S::case_stmt(
            E::ident("sel"),
            vec![
                (CV::literal(1, 0), vec![S::assign("y", E::ident("a"))]),
                (CV::literal(1, 1), vec![S::assign("z", E::ident("b"))]),
            ],
        )]);
        assert_eq!(synthesize(&m), Err(SynthesisError::MismatchedTargets));
    }

    #[test]
    fn multiple_outputs_per_arm_synthesize_to_multiple_mux_gates() {
        let m = M::new("dual").with_statements(vec![S::case_stmt(
            E::ident("sel"),
            vec![
                (
                    CV::literal(1, 0),
                    vec![
                        S::assign("y1", E::ident("a1")),
                        S::assign("y2", E::ident("a2")),
                    ],
                ),
                (
                    CV::literal(1, 1),
                    vec![
                        S::assign("y1", E::ident("b1")),
                        S::assign("y2", E::ident("b2")),
                    ],
                ),
            ],
        )]);
        let netlist = synthesize(&m).expect("dual-output 2:1 mux is in scope");
        assert_eq!(netlist.gates.len(), 2);
        netlist.validate().expect("well-formed");
        let outputs: std::collections::BTreeSet<_> =
            netlist.gates.iter().map(|g| g.output.clone()).collect();
        assert_eq!(
            outputs,
            ["y1", "y2"].iter().map(|s| s.to_string()).collect()
        );
    }

    #[test]
    fn module_signals_carry_over_as_wires() {
        let m = M::new("with_signal")
            .with_signals(vec![jxcl_hdl::Signal::new("internal", 4)])
            .with_statements(vec![S::case_stmt(
                E::ident("sel"),
                vec![
                    (CV::literal(1, 0), vec![S::assign("y", E::ident("a"))]),
                    (CV::literal(1, 1), vec![S::assign("y", E::ident("b"))]),
                ],
            )]);
        let netlist = synthesize(&m).expect("in scope");
        assert_eq!(netlist.wires.len(), 1);
        assert_eq!(netlist.wires[0].name, "internal");
        assert_eq!(netlist.wires[0].width, 4);
    }
}
