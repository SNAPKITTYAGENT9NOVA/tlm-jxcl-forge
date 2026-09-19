//! Parses a token stream into an assembly AST (instructions, labels, directives).
//!
//! Owns: The assembly AST and its parser.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `toolchain` category. Planned public API: parse, AstNode.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, error-path.
    }
}
