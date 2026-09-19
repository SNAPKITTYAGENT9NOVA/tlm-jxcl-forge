//! A structural gate-level netlist intermediate representation (primitive gates + wires).
//!
//! Owns: The Netlist/Gate/Wire types.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `hardware` category. Planned public API: Netlist, Gate, Wire.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit.
    }
}
