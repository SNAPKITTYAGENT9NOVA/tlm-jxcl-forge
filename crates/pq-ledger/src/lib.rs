//! A tamper-evident, hash-chained audit ledger recording proof attestations in order, built on pq-journal.
//!
//! Owns: Ledger::record/verify_chain.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `storage` category. Planned public API: Ledger.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, tamper.
    }
}
