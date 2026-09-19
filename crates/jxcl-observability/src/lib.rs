//! Metrics/span conventions (a RequestSpan helper, standard metric names) built atop jxcl-logging, consumed by jxcl-service.
//!
//! Owns: RequestSpan and the metric-naming convention.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `security` category. Planned public API: RequestSpan.
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
