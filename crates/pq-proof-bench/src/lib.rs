//! Criterion benchmarks measuring attest()/verify() throughput for registered proof schemes.
//!
//! Owns: The benchmark harness (dev-only, not a library dependency of anything).
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `proof` category. Planned public API: (bench-only crate).
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // benchmark.
    }
}
