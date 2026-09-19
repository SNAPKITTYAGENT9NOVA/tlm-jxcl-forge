//! The symbol table type (name -> address/section) shared by the assembler, object format, linker, and disassembler.
//!
//! Owns: SymbolTable.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `toolchain` category. Planned public API: SymbolTable, Symbol.
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
