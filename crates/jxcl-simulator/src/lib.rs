//! An embeddable top-level simulation API (load + run in one call) for using jxcl as a library from other Rust programs/services.
//!
//! Owns: Simulator::new/run.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `debug` category. Planned public API: Simulator, RunResult.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, integration.
    }
}
