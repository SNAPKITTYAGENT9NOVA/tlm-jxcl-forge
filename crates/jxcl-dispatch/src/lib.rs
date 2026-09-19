//! Table-driven opcode-to-handler dispatch, extracted from the fetch/decode/execute loop's match statement so dispatch can be tested and extended independently of execution semantics.
//!
//! Owns: The dispatch table (Opcode -> handler fn pointer).
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `execution` category. Planned public API: DispatchTable, DispatchTable::lookup.
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
