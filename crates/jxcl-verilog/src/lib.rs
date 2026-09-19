//! Renders a `jxcl-hdl` [`Module`] AST to syntactically plausible
//! Verilog-2001 text.
//!
//! # Honesty boundary
//!
//! This backend produces text that *looks like* Verilog-2001 and follows
//! its grammar for the subset of constructs `jxcl-hdl` can express
//! (module/port declarations, `always @(*)` combinational blocks,
//! `case`/`endcase`). It does **not** run the output through a real
//! Verilog parser, linter, or simulator -- there is no `iverilog`,
//! `verilator`, or `yosys` in this environment (see
//! `docs/HARDWARE_LIMITATIONS.md`). Correctness here means: matches the
//! checked-in golden files, and passes this crate's own lightweight
//! structural re-scan (module/endmodule balance, one `case`/`endcase`
//! per `always` block, one line per port). It does not mean "compiles
//! under a real toolchain."
#![forbid(unsafe_code)]

use jxcl_hdl::{CaseValue, Direction, Expr, Module, Sensitivity, Statement};
use std::collections::HashSet;
use std::fmt::Write as _;

/// Render a complete module to Verilog-2001 source text.
pub fn render_verilog(module: &Module) -> String {
    let regs = reg_targets(module);
    let mut out = String::new();

    writeln!(out, "module {}(", module.name).ok();
    let n = module.ports.len();
    for (i, p) in module.ports.iter().enumerate() {
        let dir = match p.direction {
            Direction::In => "input",
            Direction::Out => "output",
        };
        let kind = if regs.contains(&p.name) {
            "reg"
        } else {
            "wire"
        };
        let width = width_range(p.width);
        let comma = if i + 1 < n { "," } else { "" };
        writeln!(out, "    {dir} {kind}{width} {}{comma}", p.name).ok();
    }
    writeln!(out, ");").ok();

    if !module.signals.is_empty() {
        writeln!(out).ok();
        for s in &module.signals {
            let kind = if regs.contains(&s.name) {
                "reg"
            } else {
                "wire"
            };
            let width = width_range(s.width);
            writeln!(out, "    {kind}{width} {};", s.name).ok();
        }
    }

    if !module.statements.is_empty() {
        writeln!(out).ok();
        for stmt in &module.statements {
            render_statement(stmt, 1, &mut out);
        }
    }

    writeln!(out).ok();
    write!(out, "endmodule").ok();
    writeln!(out).ok();
    out
}

/// Verilog's port-declaration bit-range suffix: empty for a 1-bit
/// scalar, `[width-1:0]` otherwise.
fn width_range(width: u32) -> String {
    if width <= 1 {
        String::new()
    } else {
        format!(" [{}:0]", width - 1)
    }
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn render_expr(expr: &Expr) -> String {
    match expr {
        Expr::Ident(name) => name.clone(),
        Expr::Literal { width, value } => format!("{width}'d{value}"),
        Expr::BitSlice { base, high, low } => {
            format!("{}[{}:{}]", render_expr(base), high, low)
        }
    }
}

fn render_case_value(value: &CaseValue) -> String {
    match value {
        CaseValue::Literal { width, value } => format!("{width}'d{value}"),
        CaseValue::Default => "default".to_string(),
    }
}

fn render_statement(stmt: &Statement, level: usize, out: &mut String) {
    let pad = indent(level);
    match stmt {
        Statement::Assign { target, expr } => {
            writeln!(out, "{pad}{target} = {};", render_expr(expr)).ok();
        }
        Statement::Always { sensitivity, body } => {
            let sense = match sensitivity {
                Sensitivity::Combinational => "*",
            };
            writeln!(out, "{pad}always @({sense}) begin").ok();
            for s in body {
                render_statement(s, level + 1, out);
            }
            writeln!(out, "{pad}end").ok();
        }
        Statement::Case { selector, arms } => {
            writeln!(out, "{pad}case ({})", render_expr(selector)).ok();
            let arm_pad = indent(level + 1);
            for (value, body) in arms {
                let is_default = matches!(value, CaseValue::Default);
                if is_default {
                    writeln!(out, "{arm_pad}default: begin").ok();
                } else {
                    writeln!(out, "{arm_pad}{}: begin", render_case_value(value)).ok();
                }
                for s in body {
                    render_statement(s, level + 2, out);
                }
                writeln!(out, "{arm_pad}end").ok();
            }
            writeln!(out, "{pad}endcase").ok();
        }
    }
}

/// Every assignment target that appears anywhere inside an `always`
/// block must be declared `reg` in Verilog (it is a procedurally
/// assigned variable, not a continuously driven wire). Everything else
/// (module-level `assign`, or never assigned) stays `wire`.
fn reg_targets(module: &Module) -> HashSet<String> {
    let mut regs = HashSet::new();
    fn walk(stmts: &[Statement], inside_always: bool, regs: &mut HashSet<String>) {
        for s in stmts {
            match s {
                Statement::Assign { target, .. } => {
                    if inside_always {
                        regs.insert(target.clone());
                    }
                }
                Statement::Always { body, .. } => walk(body, true, regs),
                Statement::Case { arms, .. } => {
                    for (_, body) in arms {
                        walk(body, inside_always, regs);
                    }
                }
            }
        }
    }
    walk(&module.statements, false, &mut regs);
    regs
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_hdl::examples::mux2_module;

    #[test]
    fn renders_module_and_endmodule() {
        let text = render_verilog(&mux2_module());
        assert!(text.starts_with("module mux2("));
        assert!(text.trim_end().ends_with("endmodule"));
    }

    #[test]
    fn renders_input_and_output_ports_with_correct_kind() {
        let text = render_verilog(&mux2_module());
        assert!(text.contains("input wire sel"));
        assert!(text.contains("input wire a"));
        assert!(text.contains("input wire b"));
        // `y` is assigned inside an always block, so it must be `reg`.
        assert!(text.contains("output reg y"));
    }

    #[test]
    fn renders_multi_bit_port_with_bit_range() {
        let m =
            jxcl_hdl::Module::new("widths").with_ports(vec![jxcl_hdl::Port::input("opcode", 8)]);
        let text = render_verilog(&m);
        assert!(text.contains("input wire [7:0] opcode"));
    }

    #[test]
    fn renders_always_star_and_case_endcase() {
        let text = render_verilog(&mux2_module());
        assert!(text.contains("always @(*) begin"));
        assert!(text.contains("case (sel)"));
        assert!(text.contains("endcase"));
        assert!(text.contains("1'd0: begin"));
        assert!(text.contains("1'd1: begin"));
        assert!(text.contains("y = a;"));
        assert!(text.contains("y = b;"));
    }

    #[test]
    fn begin_and_bare_end_counts_match() {
        let text = render_verilog(&mux2_module());
        let begins = text.matches("begin").count();
        let bare_ends = text.lines().filter(|l| l.trim() == "end").count();
        assert_eq!(begins, bare_ends, "unbalanced begin/end in:\n{text}");
        assert_eq!(
            text.matches("case (").count(),
            text.matches("endcase").count()
        );
    }

    #[test]
    fn golden_mux2_verilog() {
        let text = render_verilog(&mux2_module());
        let expected = include_str!("../tests/golden/mux2.v");
        assert_eq!(
            text, expected,
            "rendered Verilog drifted from tests/golden/mux2.v"
        );
    }
}
