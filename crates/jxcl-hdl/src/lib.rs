//! A backend-agnostic hardware-description intermediate representation:
//! modules, ports, signals, always-blocks, case-statements.
//!
//! # Scope and honesty boundary
//!
//! This crate defines a Rust **AST** for a small, explicitly scoped
//! subset of RTL-style hardware description. It does not simulate
//! anything and it does not talk to any external EDA tool. See
//! `docs/HARDWARE_LIMITATIONS.md` at the workspace root for the full
//! statement of what the Hardware/RTL crate batch does and does not do
//! (no `iverilog`/`verilator`/`yosys` is available in this environment,
//! or assumed to be available by anything in this crate).
//!
//! `jxcl-verilog` and `jxcl-vhdl` render this same [`Module`] AST to two
//! different text backends, which is the concrete proof that the AST
//! itself is backend-agnostic. `jxcl-synthesis` lowers a small subset of
//! it to a structural gate netlist ([`jxcl-netlist`]).
#![forbid(unsafe_code)]

/// Port direction, as seen from outside the module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    In,
    Out,
}

/// A module port: a named, directional, fixed-width signal that is part
/// of the module's external interface.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Port {
    pub name: String,
    pub direction: Direction,
    /// Width in bits. `1` is a scalar (`wire`/`std_logic`); anything
    /// greater renders as a vector (`[width-1:0]` / `std_logic_vector`).
    pub width: u32,
}

impl Port {
    pub fn new(name: impl Into<String>, direction: Direction, width: u32) -> Self {
        Port {
            name: name.into(),
            direction,
            width,
        }
    }

    pub fn input(name: impl Into<String>, width: u32) -> Self {
        Port::new(name, Direction::In, width)
    }

    pub fn output(name: impl Into<String>, width: u32) -> Self {
        Port::new(name, Direction::Out, width)
    }
}

/// An internal signal (a `wire`/`reg` in Verilog terms, a `signal` in
/// VHDL terms) that is not part of the module's port list.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Signal {
    pub name: String,
    pub width: u32,
}

impl Signal {
    pub fn new(name: impl Into<String>, width: u32) -> Self {
        Signal {
            name: name.into(),
            width,
        }
    }
}

/// An expression that can appear on the right-hand side of an
/// assignment, or as a `case`/`always` selector.
///
/// This is intentionally a small subset: identifiers, fixed-width
/// literals, and a fixed-range bit slice of another expression. It is
/// enough to express opcode-field extraction, register-file addressing,
/// and simple combinational selection, which is all this crate's
/// consumers (`jxcl-rtl`) need.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    /// A reference to a port or signal by name.
    Ident(String),
    /// A fixed-width numeric literal, e.g. the Verilog `8'h10` /
    /// VHDL `"00010000"`.
    Literal { width: u32, value: u64 },
    /// `base[high:low]`, inclusive on both ends, `high >= low`.
    BitSlice {
        base: Box<Expr>,
        high: u32,
        low: u32,
    },
}

impl Expr {
    pub fn ident(name: impl Into<String>) -> Self {
        Expr::Ident(name.into())
    }

    pub fn literal(width: u32, value: u64) -> Self {
        Expr::Literal { width, value }
    }

    pub fn bit_slice(base: Expr, high: u32, low: u32) -> Self {
        Expr::BitSlice {
            base: Box::new(base),
            high,
            low,
        }
    }
}

/// The selector value matched by one arm of a [`Statement::Case`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CaseValue {
    /// A specific fixed-width value (Verilog `8'h10:`, VHDL
    /// `when "00010000" =>`).
    Literal { width: u32, value: u64 },
    /// The catch-all arm (Verilog `default:`, VHDL `when others =>`).
    Default,
}

impl CaseValue {
    pub fn literal(width: u32, value: u64) -> Self {
        CaseValue::Literal { width, value }
    }
}

/// The sensitivity of an `always`/`process` block.
///
/// Only combinational sensitivity (`always @(*)` / `process(...)` with
/// every read signal listed) is modeled; this crate's consumers only
/// need to describe combinational decode/select logic, not clocked
/// sequential logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sensitivity {
    Combinational,
}

/// One statement inside a module body or a process/case-arm body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// A (continuous or procedural, depending on context) assignment:
    /// `target = expr;`.
    Assign { target: String, expr: Expr },
    /// A combinational process block: `always @(*) begin ... end` /
    /// `process(...) begin ... end process;`.
    Always {
        sensitivity: Sensitivity,
        body: Vec<Statement>,
    },
    /// A `case`/`case...is` statement: one arm per matched selector
    /// value, each arm holding its own statement body.
    Case {
        selector: Expr,
        arms: Vec<(CaseValue, Vec<Statement>)>,
    },
}

impl Statement {
    pub fn assign(target: impl Into<String>, expr: Expr) -> Self {
        Statement::Assign {
            target: target.into(),
            expr,
        }
    }

    pub fn always(sensitivity: Sensitivity, body: Vec<Statement>) -> Self {
        Statement::Always { sensitivity, body }
    }

    pub fn case_stmt(selector: Expr, arms: Vec<(CaseValue, Vec<Statement>)>) -> Self {
        Statement::Case { selector, arms }
    }
}

/// A complete hardware module: a name, an interface (ports), internal
/// state (signals), and behavior (statements).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    pub ports: Vec<Port>,
    pub signals: Vec<Signal>,
    pub statements: Vec<Statement>,
}

impl Module {
    pub fn new(name: impl Into<String>) -> Self {
        Module {
            name: name.into(),
            ports: Vec::new(),
            signals: Vec::new(),
            statements: Vec::new(),
        }
    }

    pub fn with_ports(mut self, ports: Vec<Port>) -> Self {
        self.ports = ports;
        self
    }

    pub fn with_signals(mut self, signals: Vec<Signal>) -> Self {
        self.signals = signals;
        self
    }

    pub fn with_statements(mut self, statements: Vec<Statement>) -> Self {
        self.statements = statements;
        self
    }

    pub fn inputs(&self) -> impl Iterator<Item = &Port> {
        self.ports.iter().filter(|p| p.direction == Direction::In)
    }

    pub fn outputs(&self) -> impl Iterator<Item = &Port> {
        self.ports.iter().filter(|p| p.direction == Direction::Out)
    }

    /// Find the first top-level `case` statement, regardless of whether
    /// it is wrapped in an `always` block or bare. Returns `None` if
    /// there is no top-level case statement. Consumers that need to walk
    /// arbitrarily nested statements should match on [`Statement`]
    /// directly; this helper covers the common "one always-with-one-case
    /// module" shape used throughout `jxcl-rtl` and `jxcl-synthesis`.
    pub fn top_level_case(&self) -> Option<&Statement> {
        fn find(stmts: &[Statement]) -> Option<&Statement> {
            for s in stmts {
                match s {
                    Statement::Case { .. } => return Some(s),
                    Statement::Always { body, .. } => {
                        if let Some(found) = find(body) {
                            return Some(found);
                        }
                    }
                    Statement::Assign { .. } => {}
                }
            }
            None
        }
        find(&self.statements)
    }
}

/// Small hand-built example modules shared across this crate's own tests
/// and the downstream `jxcl-verilog`/`jxcl-vhdl` golden-file tests, so
/// every backend renders exactly the same AST instance.
pub mod examples {
    use super::*;

    /// A 2-to-1 multiplexer: `sel` selects between `a` and `b` on the
    /// single-bit output `y`. This is the canonical minimal example used
    /// to prove the AST is backend-agnostic (`jxcl-verilog`/`jxcl-vhdl`
    /// both render this exact module) and is also the shape
    /// `jxcl-synthesis`'s toy synthesis pass is scoped to handle.
    pub fn mux2_module() -> Module {
        Module::new("mux2")
            .with_ports(vec![
                Port::input("sel", 1),
                Port::input("a", 1),
                Port::input("b", 1),
                Port::output("y", 1),
            ])
            .with_statements(vec![Statement::always(
                Sensitivity::Combinational,
                vec![Statement::case_stmt(
                    Expr::ident("sel"),
                    vec![
                        (
                            CaseValue::literal(1, 0),
                            vec![Statement::assign("y", Expr::ident("a"))],
                        ),
                        (
                            CaseValue::literal(1, 1),
                            vec![Statement::assign("y", Expr::ident("b"))],
                        ),
                    ],
                )],
            )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use examples::mux2_module;

    #[test]
    fn module_structure_matches_hand_built_shape() {
        let m = mux2_module();
        assert_eq!(m.name, "mux2");
        assert_eq!(m.ports.len(), 4);
        assert_eq!(m.inputs().count(), 3);
        assert_eq!(m.outputs().count(), 1);
        assert!(m.signals.is_empty());
        assert_eq!(m.statements.len(), 1);
    }

    #[test]
    fn port_widths_and_directions_are_preserved() {
        let m = mux2_module();
        let sel = m.ports.iter().find(|p| p.name == "sel").unwrap();
        assert_eq!(sel.direction, Direction::In);
        assert_eq!(sel.width, 1);

        let y = m.ports.iter().find(|p| p.name == "y").unwrap();
        assert_eq!(y.direction, Direction::Out);
        assert_eq!(y.width, 1);
    }

    #[test]
    fn top_level_case_is_found_inside_always_block() {
        let m = mux2_module();
        let case = m.top_level_case().expect("module has a case statement");
        match case {
            Statement::Case { selector, arms } => {
                assert_eq!(*selector, Expr::ident("sel"));
                assert_eq!(arms.len(), 2);
                assert_eq!(arms[0].0, CaseValue::literal(1, 0));
                assert_eq!(arms[1].0, CaseValue::literal(1, 1));
                assert_eq!(arms[0].1, vec![Statement::assign("y", Expr::ident("a"))]);
                assert_eq!(arms[1].1, vec![Statement::assign("y", Expr::ident("b"))]);
            }
            other => panic!("expected a case statement, got {other:?}"),
        }
    }

    #[test]
    fn case_default_arm_is_distinct_from_any_literal() {
        assert_ne!(CaseValue::Default, CaseValue::literal(8, 0));
        assert_ne!(CaseValue::Default, CaseValue::literal(1, 0));
    }

    #[test]
    fn signals_can_be_added_alongside_ports() {
        let m = Module::new("with_signal")
            .with_ports(vec![Port::input("clk_unused", 1)])
            .with_signals(vec![Signal::new("internal", 8)]);
        assert_eq!(m.signals.len(), 1);
        assert_eq!(m.signals[0].width, 8);
    }

    #[test]
    fn bit_slice_expr_holds_its_range() {
        let e = Expr::bit_slice(Expr::ident("instr"), 15, 8);
        match e {
            Expr::BitSlice { base, high, low } => {
                assert_eq!(*base, Expr::ident("instr"));
                assert_eq!(high, 15);
                assert_eq!(low, 8);
            }
            other => panic!("expected BitSlice, got {other:?}"),
        }
    }
}
