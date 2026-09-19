//! A direct-mapped/set-associative cache simulation layered over memory, for profiling hit/miss behavior.
//!
//! Owns: CacheModel and its hit/miss accounting.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `memory` category. Planned public API: CacheModel.
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
