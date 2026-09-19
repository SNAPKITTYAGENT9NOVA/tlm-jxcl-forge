//! The symbol table type (name -> address/section) shared by the
//! assembler, object format, linker, and disassembler.
//!
//! Owns: [`SymbolTable`] and [`Symbol`] -- every other toolchain crate in
//! this batch (`jxcl-object`, `jxcl-linker`, `jxcl-assembler`,
//! `jxcl-disassembler`) stores and resolves symbols exclusively through
//! this type, so there is exactly one notion of "what does this name
//! resolve to" across the toolchain.
//!
//! A `SymbolTable` uses a [`BTreeMap`] internally (not a `HashMap`) so
//! that iteration order -- and therefore the byte layout `jxcl-object`
//! produces when it serializes a table -- is deterministic across runs,
//! which matters for reproducible builds and golden-file tests.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

/// Which section a symbol's address is relative to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolSection {
    /// The code (text) section.
    Code,
    /// The data section.
    Data,
}

impl SymbolSection {
    /// A stable one-byte tag, used by `jxcl-object`'s wire format.
    pub const fn tag(self) -> u8 {
        match self {
            SymbolSection::Code => 0,
            SymbolSection::Data => 1,
        }
    }

    /// Inverse of [`SymbolSection::tag`].
    pub fn from_tag(tag: u8) -> Option<SymbolSection> {
        match tag {
            0 => Some(SymbolSection::Code),
            1 => Some(SymbolSection::Data),
            _ => None,
        }
    }
}

/// A single named symbol: where it lives, and in which section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    /// Byte offset. Within a single object file this is a *section-local*
    /// offset; once the linker has placed the section, it is rewritten to
    /// an absolute address (see `jxcl-linker`).
    pub address: u64,
    pub section: SymbolSection,
}

impl Symbol {
    pub fn new(name: impl Into<String>, address: u64, section: SymbolSection) -> Self {
        Symbol {
            name: name.into(),
            address,
            section,
        }
    }
}

/// What can go wrong manipulating a [`SymbolTable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolErrorKind {
    /// A symbol with this name was already defined.
    DuplicateSymbol,
    /// No symbol with this name is defined.
    UndefinedSymbol,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolError {
    pub kind: SymbolErrorKind,
    pub reason: String,
}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "symbol error: {:?}: {}", self.kind, self.reason)
    }
}
impl std::error::Error for SymbolError {}

fn dup(name: &str) -> SymbolError {
    SymbolError {
        kind: SymbolErrorKind::DuplicateSymbol,
        reason: format!("symbol {:?} is already defined", name),
    }
}

fn undef(name: &str) -> SymbolError {
    SymbolError {
        kind: SymbolErrorKind::UndefinedSymbol,
        reason: format!("symbol {:?} is not defined", name),
    }
}

/// A name -> [`Symbol`] table. Insertion order is not preserved; iteration
/// is always in ascending name order (`BTreeMap`), which keeps
/// serialization and tests deterministic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolTable {
    symbols: BTreeMap<String, Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable {
            symbols: BTreeMap::new(),
        }
    }

    /// Define a new symbol. Errors with `DuplicateSymbol` if `name` is
    /// already defined -- a symbol table never silently overwrites an
    /// existing definition.
    pub fn define(
        &mut self,
        name: impl Into<String>,
        address: u64,
        section: SymbolSection,
    ) -> Result<(), SymbolError> {
        let name = name.into();
        if self.symbols.contains_key(&name) {
            return Err(dup(&name));
        }
        self.symbols
            .insert(name.clone(), Symbol::new(name, address, section));
        Ok(())
    }

    /// Look up a symbol by name.
    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Look up a symbol by name, or a `SymbolError::UndefinedSymbol`.
    pub fn resolve_or_err(&self, name: &str) -> Result<&Symbol, SymbolError> {
        self.resolve(name).ok_or_else(|| undef(name))
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Iterate all symbols in ascending name order.
    pub fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }

    /// Merge `other`'s symbols into `self`, applying `offset` to every
    /// merged symbol's address first (used by `jxcl-linker` to relocate an
    /// object file's section-local symbol table to its final placement in
    /// the combined section). Errors (without partially merging) if any
    /// name collides with one already present.
    pub fn merge_with_offset(
        &mut self,
        other: &SymbolTable,
        offset: u64,
    ) -> Result<(), SymbolError> {
        for name in other.symbols.keys() {
            if self.symbols.contains_key(name) {
                return Err(dup(name));
            }
        }
        for sym in other.symbols.values() {
            let shifted = Symbol::new(sym.name.clone(), sym.address + offset, sym.section);
            self.symbols.insert(shifted.name.clone(), shifted);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn define_and_resolve() {
        let mut t = SymbolTable::new();
        t.define("start", 0, SymbolSection::Code).unwrap();
        t.define("buffer", 16, SymbolSection::Data).unwrap();

        let s = t.resolve("start").unwrap();
        assert_eq!(s.address, 0);
        assert_eq!(s.section, SymbolSection::Code);

        let b = t.resolve("buffer").unwrap();
        assert_eq!(b.address, 16);
        assert_eq!(b.section, SymbolSection::Data);
    }

    #[test]
    fn resolve_unknown_is_none() {
        let t = SymbolTable::new();
        assert!(t.resolve("nope").is_none());
        assert_eq!(
            t.resolve_or_err("nope").unwrap_err().kind,
            SymbolErrorKind::UndefinedSymbol
        );
    }

    #[test]
    fn duplicate_definition_is_an_error() {
        let mut t = SymbolTable::new();
        t.define("start", 0, SymbolSection::Code).unwrap();
        let err = t.define("start", 4, SymbolSection::Code).unwrap_err();
        assert_eq!(err.kind, SymbolErrorKind::DuplicateSymbol);
        // The original definition must be untouched.
        assert_eq!(t.resolve("start").unwrap().address, 0);
    }

    #[test]
    fn iteration_is_in_ascending_name_order() {
        let mut t = SymbolTable::new();
        t.define("zeta", 0, SymbolSection::Code).unwrap();
        t.define("alpha", 4, SymbolSection::Code).unwrap();
        t.define("mid", 8, SymbolSection::Code).unwrap();
        let names: Vec<&str> = t.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn merge_with_offset_shifts_addresses() {
        let mut a = SymbolTable::new();
        a.define("main", 0, SymbolSection::Code).unwrap();

        let mut b = SymbolTable::new();
        b.define("helper", 4, SymbolSection::Code).unwrap();
        b.define("table", 0, SymbolSection::Data).unwrap();

        a.merge_with_offset(&b, 100).unwrap();
        assert_eq!(a.resolve("helper").unwrap().address, 104);
        assert_eq!(a.resolve("table").unwrap().address, 100);
        assert_eq!(a.resolve("main").unwrap().address, 0);
    }

    #[test]
    fn merge_rejects_colliding_names_without_partial_merge() {
        let mut a = SymbolTable::new();
        a.define("dup", 0, SymbolSection::Code).unwrap();

        let mut b = SymbolTable::new();
        b.define("dup", 4, SymbolSection::Code).unwrap();
        b.define("unique", 8, SymbolSection::Code).unwrap();

        let err = a.merge_with_offset(&b, 0).unwrap_err();
        assert_eq!(err.kind, SymbolErrorKind::DuplicateSymbol);
        // Nothing from `b` should have been merged in, including `unique`.
        assert!(a.resolve("unique").is_none());
    }

    #[test]
    fn section_tag_roundtrips() {
        for s in [SymbolSection::Code, SymbolSection::Data] {
            assert_eq!(SymbolSection::from_tag(s.tag()), Some(s));
        }
        assert_eq!(SymbolSection::from_tag(0xFF), None);
    }
}
