//! A structural gate-level netlist intermediate representation
//! (primitive gates + wires).
//!
//! # Scope and honesty boundary
//!
//! This crate defines a plain Rust data structure for a netlist made of
//! four primitive gate kinds ([`GateKind::And`], [`GateKind::Or`],
//! [`GateKind::Not`], [`GateKind::Mux`]) connected by named wires. It is
//! a **structural representation only**: nothing here simulates gate
//! behavior, drives timing, or talks to a logic synthesizer. There is no
//! `iverilog`/`verilator`/`yosys` in this environment (see
//! `docs/HARDWARE_LIMITATIONS.md` at the workspace root); a [`Netlist`]
//! produced here is checked for *structural* well-formedness (gate
//! arity matches its kind, in [`Netlist::validate`]) — never for
//! electrical or functional correctness.
#![forbid(unsafe_code)]

/// The primitive gate kinds this netlist representation can express.
///
/// This is intentionally a minimal, fixed set (spec-equivalent to a
/// standard-cell library reduced to its four most basic combinational
/// primitives): enough for [`jxcl-synthesis`](../jxcl_synthesis/index.html)'s
/// toy lowering pass to target, not a real cell library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateKind {
    /// 2-input AND.
    And,
    /// 2-input OR.
    Or,
    /// 1-input inverter.
    Not,
    /// 2-to-1 multiplexer: `inputs = [select, when_0, when_1]`,
    /// `output = if select == 0 { when_0 } else { when_1 }`.
    Mux,
}

impl GateKind {
    /// The exact number of inputs a well-formed gate of this kind must
    /// have. Used by [`Netlist::validate`] to catch a malformed gate
    /// (e.g. a synthesis-pass bug) as a structural error rather than
    /// letting it render as nonsense text downstream.
    pub const fn arity(self) -> usize {
        match self {
            GateKind::And => 2,
            GateKind::Or => 2,
            GateKind::Not => 1,
            GateKind::Mux => 3,
        }
    }
}

/// One primitive gate instance: a kind, its ordered input wire/port
/// names, and the single named net it drives.
///
/// Input and output are plain `String` names rather than typed
/// references on purpose — a netlist is a flat structural graph, and
/// the names are resolved against a [`Netlist`]'s [`Wire`]s or against
/// the enclosing module's own ports (which this crate does not model;
/// that association is `jxcl-synthesis`'s job when it builds a
/// [`Netlist`] from a `jxcl-hdl` `Module`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    pub kind: GateKind,
    pub inputs: Vec<String>,
    pub output: String,
}

impl Gate {
    pub fn new(kind: GateKind, inputs: Vec<String>, output: impl Into<String>) -> Self {
        Gate {
            kind,
            inputs,
            output: output.into(),
        }
    }

    pub fn and(a: impl Into<String>, b: impl Into<String>, output: impl Into<String>) -> Self {
        Gate::new(GateKind::And, vec![a.into(), b.into()], output)
    }

    pub fn or(a: impl Into<String>, b: impl Into<String>, output: impl Into<String>) -> Self {
        Gate::new(GateKind::Or, vec![a.into(), b.into()], output)
    }

    pub fn not(a: impl Into<String>, output: impl Into<String>) -> Self {
        Gate::new(GateKind::Not, vec![a.into()], output)
    }

    /// `select` chooses between `when_0` (selected when `select == 0`)
    /// and `when_1` (selected when `select == 1`).
    pub fn mux(
        select: impl Into<String>,
        when_0: impl Into<String>,
        when_1: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Gate::new(
            GateKind::Mux,
            vec![select.into(), when_0.into(), when_1.into()],
            output,
        )
    }

    /// Does this gate's input count match what [`GateKind::arity`]
    /// requires for its kind?
    pub fn has_valid_arity(&self) -> bool {
        self.inputs.len() == self.kind.arity()
    }
}

/// A named internal net with a bit width, distinct from a module's
/// external ports (which the consumer that built this `Netlist` from a
/// `jxcl-hdl::Module` already knows about).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Wire {
    pub name: String,
    pub width: u32,
}

impl Wire {
    pub fn new(name: impl Into<String>, width: u32) -> Self {
        Wire {
            name: name.into(),
            width,
        }
    }
}

/// A structural error found by [`Netlist::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetlistError {
    /// A gate's `inputs.len()` didn't match `gate.kind.arity()`.
    WrongArity {
        gate_output: String,
        kind: GateKind,
        expected: usize,
        actual: usize,
    },
    /// Two gates claim to drive the same output net — not a real
    /// electrical short (this representation has no notion of drive
    /// strength), just a structural inconsistency this toy
    /// representation refuses to represent silently.
    MultiplyDrivenNet(String),
}

impl std::fmt::Display for NetlistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetlistError::WrongArity {
                gate_output,
                kind,
                expected,
                actual,
            } => write!(
                f,
                "gate driving `{gate_output}` ({kind:?}) expects {expected} input(s), got {actual}"
            ),
            NetlistError::MultiplyDrivenNet(name) => {
                write!(f, "net `{name}` is driven by more than one gate")
            }
        }
    }
}

impl std::error::Error for NetlistError {}

/// A structural gate-level netlist: a flat list of primitive [`Gate`]s
/// connected by named [`Wire`]s.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Netlist {
    pub gates: Vec<Gate>,
    pub wires: Vec<Wire>,
}

impl Netlist {
    pub fn new() -> Self {
        Netlist::default()
    }

    pub fn with_gates(mut self, gates: Vec<Gate>) -> Self {
        self.gates = gates;
        self
    }

    pub fn with_wires(mut self, wires: Vec<Wire>) -> Self {
        self.wires = wires;
        self
    }

    pub fn add_gate(&mut self, gate: Gate) {
        self.gates.push(gate);
    }

    pub fn add_wire(&mut self, wire: Wire) {
        self.wires.push(wire);
    }

    /// Structural well-formedness check: every gate has the right
    /// number of inputs for its kind, and no net is driven by more than
    /// one gate. This is the only "verification" this crate performs —
    /// it says nothing about whether the netlist behaves like the
    /// `jxcl-hdl` module it may have been synthesized from.
    pub fn validate(&self) -> Result<(), NetlistError> {
        for gate in &self.gates {
            if !gate.has_valid_arity() {
                return Err(NetlistError::WrongArity {
                    gate_output: gate.output.clone(),
                    kind: gate.kind,
                    expected: gate.kind.arity(),
                    actual: gate.inputs.len(),
                });
            }
        }
        let mut driven = std::collections::HashSet::new();
        for gate in &self.gates {
            if !driven.insert(gate.output.clone()) {
                return Err(NetlistError::MultiplyDrivenNet(gate.output.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_arity_constants_match_primitive_shapes() {
        assert_eq!(GateKind::And.arity(), 2);
        assert_eq!(GateKind::Or.arity(), 2);
        assert_eq!(GateKind::Not.arity(), 1);
        assert_eq!(GateKind::Mux.arity(), 3);
    }

    #[test]
    fn constructors_build_well_formed_gates() {
        let and = Gate::and("a", "b", "y");
        assert!(and.has_valid_arity());
        let not = Gate::not("a", "y");
        assert!(not.has_valid_arity());
        let mux = Gate::mux("sel", "a", "b", "y");
        assert!(mux.has_valid_arity());
        assert_eq!(mux.inputs, vec!["sel", "a", "b"]);
    }

    #[test]
    fn malformed_gate_fails_validation() {
        let bad = Gate::new(GateKind::And, vec!["only_one".into()], "y");
        assert!(!bad.has_valid_arity());
        let net = Netlist::new().with_gates(vec![bad]);
        let err = net.validate().unwrap_err();
        assert!(matches!(err, NetlistError::WrongArity { .. }));
    }

    #[test]
    fn well_formed_netlist_validates() {
        let net = Netlist::new()
            .with_gates(vec![Gate::mux("sel", "a", "b", "y")])
            .with_wires(vec![]);
        assert!(net.validate().is_ok());
    }

    #[test]
    fn two_gates_driving_the_same_net_is_rejected() {
        let net =
            Netlist::new().with_gates(vec![Gate::and("a", "b", "y"), Gate::or("c", "d", "y")]);
        let err = net.validate().unwrap_err();
        assert_eq!(err, NetlistError::MultiplyDrivenNet("y".to_string()));
    }

    #[test]
    fn wires_can_be_added_incrementally() {
        let mut net = Netlist::new();
        net.add_wire(Wire::new("internal", 8));
        net.add_gate(Gate::not("internal", "inv"));
        assert_eq!(net.wires.len(), 1);
        assert_eq!(net.wires[0].width, 8);
        assert_eq!(net.gates.len(), 1);
    }

    #[test]
    fn netlist_error_messages_are_descriptive() {
        let err = NetlistError::WrongArity {
            gate_output: "y".into(),
            kind: GateKind::Mux,
            expected: 3,
            actual: 1,
        };
        assert!(err.to_string().contains("Mux"));
        assert!(err.to_string().contains('y'));
    }
}
