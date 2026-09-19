//! The per-step execution trace record format and its writer/reader, reused by the profiler and replay.
//!
//! Owns: TraceStep and TraceLog.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `debug` category. Planned public API: TraceStep, TraceLog.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, serialization.
    }
}
