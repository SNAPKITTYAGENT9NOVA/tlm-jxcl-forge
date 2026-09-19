//! A deterministic external-interrupt injection mechanism (priority queue + mask flag) for embedding jxcl in a simulator or RPC host.
//!
//! Owns: InterruptController: queue/mask/deliver.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `execution` category. Planned public API: InterruptController.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, determinism.
    }
}
