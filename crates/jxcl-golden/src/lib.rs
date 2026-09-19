//! The golden-vector format, loader, and canonical vector files used as the ISA conformance baseline.
//!
//! Owns: GoldenVector and tests/vectors/.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `debug` category. Planned public API: GoldenVector, load_vectors, run_vector.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // golden, conformance.
    }
}
