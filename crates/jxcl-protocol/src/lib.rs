//! A serde-serializable request/response protocol for remote jxcl-machine control: assemble, run, return trace/final state.
//!
//! Owns: the Request/Response wire types.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `network` category. Planned public API: Request, Response.
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
