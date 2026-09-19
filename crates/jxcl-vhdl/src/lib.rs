//! Renders a `jxcl-hdl` [`Module`] AST to syntactically plausible VHDL
//! text, for the exact same AST `jxcl-verilog` renders to Verilog. That
//! two independent text backends can consume the same [`Module`] value
//! is the concrete proof that `jxcl-hdl`'s AST is backend-agnostic.
//!
//! # Honesty boundary
//!
//! As with `jxcl-verilog`: this produces text that follows VHDL's
//! grammar for the subset `jxcl-hdl` can express (entity/architecture,
//! port lists, `process` blocks, `case`/`end case`). It is not run
//! through a real VHDL analyzer or simulator -- none is available in
//! this environment (see `docs/HARDWARE_LIMITATIONS.md`). "Correct"
//! here means: matches the checked-in golden files and this crate's own
//! lightweight structural checks, not "analyzes clean under a real
//! toolchain."
#![forbid(unsafe_code)]

use jxcl_hdl::{CaseValue, Direction, Expr, Module, Sensitivity, Statement};
use std::fmt::Write as _;

/// Render a complete module to VHDL (entity + a single behavioral
/// architecture named `rtl`).
pub fn render_vhdl(module: &Module) -> String {
    let mut out = String::new();

    writeln!(out, "entity {} is", module.name).ok();
    if !module.ports.is_empty() {
        writeln!(out, "    port (").ok();
        let n = module.ports.len();
        for (i, p) in module.ports.iter().enumerate() {
            let dir = match p.direction {
                Direction::In => "in",
                Direction::Out => "out",
            };
            let ty = vhdl_type(p.width);
            let semi = if i + 1 < n { ";" } else { "" };
            writeln!(out, "        {} : {} {}{}", p.name, dir, ty, semi).ok();
        }
        writeln!(out, "    );").ok();
    }
    writeln!(out, "end entity {};", module.name).ok();
    writeln!(out).ok();

    writeln!(out, "architecture rtl of {} is", module.name).ok();
    for s in &module.signals {
        writeln!(out, "    signal {} : {};", s.name, vhdl_type(s.width)).ok();
    }
    writeln!(out, "begin").ok();
    for stmt in &module.statements {
        render_statement(stmt, 1, &mut out);
    }
    writeln!(out, "end architecture rtl;").ok();
    out
}

/// `std_logic` for scalars, `std_logic_vector(width-1 downto 0)` for
/// vectors -- the conventional VHDL mapping for a fixed-width signal.
fn vhdl_type(width: u32) -> String {
    if width <= 1 {
        "std_logic".to_string()
    } else {
        format!("std_logic_vector({} downto 0)", width - 1)
    }
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn render_expr(expr: &Expr) -> String {
    match expr {
        Expr::Ident(name) => name.clone(),
        Expr::Literal { width, value } => vhdl_literal(*width, *value),
        Expr::BitSlice { base, high, low } => {
            format!("{}({} downto {})", render_expr(base), high, low)
        }
    }
}

/// A fixed-width binary literal, VHDL-style: `"0101"` for width > 1,
/// `'0'`/`'1'` for a scalar.
fn vhdl_literal(width: u32, value: u64) -> String {
    if width <= 1 {
        format!("'{}'", value & 1)
    } else {
        let mut bits = String::with_capacity(width as usize);
        for i in (0..width).rev() {
            bits.push(if (value >> i) & 1 == 1 { '1' } else { '0' });
        }
        format!("\"{bits}\"")
    }
}

fn render_case_value(value: &CaseValue) -> String {
    match value {
        CaseValue::Literal { width, value } => vhdl_literal(*width, *value),
        CaseValue::Default => "others".to_string(),
    }
}

fn render_statement(stmt: &Statement, level: usize, out: &mut String) {
    let pad = indent(level);
    match stmt {
        Statement::Assign { target, expr } => {
            writeln!(out, "{pad}{target} <= {};", render_expr(expr)).ok();
        }
        Statement::Always { sensitivity, body } => {
            let sensitivity_list = match sensitivity {
                Sensitivity::Combinational => "all",
            };
            writeln!(out, "{pad}process ({sensitivity_list}) is").ok();
            writeln!(out, "{pad}begin").ok();
            for s in body {
                render_statement(s, level + 1, out);
            }
            writeln!(out, "{pad}end process;").ok();
        }
        Statement::Case { selector, arms } => {
            writeln!(out, "{pad}case {} is", render_expr(selector)).ok();
            let arm_pad = indent(level + 1);
            for (value, body) in arms {
                writeln!(out, "{arm_pad}when {} =>", render_case_value(value)).ok();
                for s in body {
                    render_statement(s, level + 2, out);
                }
            }
            writeln!(out, "{pad}end case;").ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_hdl::examples::mux2_module;

    #[test]
    fn renders_entity_and_architecture() {
        let text = render_vhdl(&mux2_module());
        assert!(text.contains("entity mux2 is"));
        assert!(text.contains("end entity mux2;"));
        assert!(text.contains("architecture rtl of mux2 is"));
        assert!(text.contains("end architecture rtl;"));
    }

    #[test]
    fn renders_port_directions_and_types() {
        let text = render_vhdl(&mux2_module());
        assert!(text.contains("sel : in std_logic"));
        assert!(text.contains("a : in std_logic"));
        assert!(text.contains("b : in std_logic"));
        assert!(text.contains("y : out std_logic"));
    }

    #[test]
    fn renders_vector_port_type() {
        let m =
            jxcl_hdl::Module::new("widths").with_ports(vec![jxcl_hdl::Port::input("opcode", 8)]);
        let text = render_vhdl(&m);
        assert!(text.contains("opcode : in std_logic_vector(7 downto 0)"));
    }

    #[test]
    fn renders_process_and_case_end_case() {
        let text = render_vhdl(&mux2_module());
        assert!(text.contains("process (all) is"));
        assert!(text.contains("case sel is"));
        assert!(text.contains("end case;"));
        assert!(text.contains("when '0' =>"));
        assert!(text.contains("when '1' =>"));
        assert!(text.contains("y <= a;"));
        assert!(text.contains("y <= b;"));
    }

    #[test]
    fn case_and_end_case_counts_match() {
        let text = render_vhdl(&mux2_module());
        assert_eq!(
            text.matches("case ").count(),
            text.matches("end case;").count()
        );
    }
}
