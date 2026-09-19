//! A staged fetch/decode/execute/writeback pipeline-stage model used to report realistic per-instruction cycle costs, distinct from the (non-pipelined) reference execution semantics.
//!
//! Owns: PipelineStage and the stage-advance state machine.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `execution` category. Planned public API: PipelineStage, Pipeline.
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
