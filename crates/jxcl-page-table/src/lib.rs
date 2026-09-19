//! A single-level virtual-to-physical page table with map/unmap/translate and page-fault on unmapped access.
//!
//! Owns: PageTable.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `memory` category. Planned public API: PageTable.
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
