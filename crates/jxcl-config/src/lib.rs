//! Environment-variable configuration parsing helpers (typed env_or, hex-seed decoding) shared by the CLI and services.
//!
//! Owns: env_or/env_or_parse/decode_hex_seed and the 'log the resolved value only if not secret' convention.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `foundation` category. Planned public API: env_or, env_or_parse, decode_hex_seed.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, boundary.
    }
}
