//! Branch-target address computation (relative/absolute addressing) split out of condition evaluation.
//!
//! Owns: compute_branch_target.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `execution` category. Planned public API: compute_branch_target.
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
