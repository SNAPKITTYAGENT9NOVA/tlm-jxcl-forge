//! Key-rotation policy: the Active/DecryptOnly/Retired lifecycle and the set_active/retire transition rules, kept separate from the KeyRing data structure itself.
//!
//! Owns: KeyStatus and the rotation state-transition rules.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `crypto` category. Planned public API: KeyStatus, set_active, retire.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, rotation.
    }
}
