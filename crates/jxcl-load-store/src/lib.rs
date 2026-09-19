//! Typed, sign-extension-aware load/store helpers (u8/u16/u32/u64 and signed variants) between raw memory and the execution engine.
//!
//! Owns: load_u8/16/32/64, store_u8/16/32/64 and their signed counterparts.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `memory` category. Planned public API: load_u8, load_u16, load_u32, load_u64, store_u8, store_u16, store_u32, store_u64.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, property.
    }
}
