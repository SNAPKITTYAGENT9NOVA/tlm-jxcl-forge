//! Renders the same jxcl-hdl AST to VHDL text, proving the AST is backend-agnostic.
//!
//! Owns: The VHDL text backend.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `hardware` category. Planned public API: render_vhdl.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, golden.
    }
}
