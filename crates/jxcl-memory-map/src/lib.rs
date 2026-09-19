//! The concrete default memory layout (where code/stack/heap/MMIO live) consumed by the loader and machine setup.
//!
//! Owns: MemoryMap, the default layout constant.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `memory` category. Planned public API: MemoryMap, DEFAULT_MEMORY_MAP.
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
