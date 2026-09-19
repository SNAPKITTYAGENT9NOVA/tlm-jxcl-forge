//! Resolves symbols and applies relocations across one or more object
//! files into a single linked binary container.
//!
//! New functionality (see `jxcl-object`'s module docs for why there is
//! no pre-expansion ground truth to extract this from -- the original
//! `jxcl` assembler always went straight from source to a finished
//! binary in one step, with no separate-compilation/linking phase at
//! all).
//!
//! ## Layout
//!
//! [`link`] concatenates every object file's code section, in the order
//! given, into one combined code section, and likewise for data
//! sections. Matching `jxcl-binary`'s convention, the combined data
//! section is placed immediately after the combined code section, so a
//! `Data`-section symbol's final absolute address is
//! `total_code_size + <that object's data section's start within the
//! combined data section> + <the symbol's original section-local
//! address>`.
//!
//! Every object's [`jxcl_symbols::SymbolTable`] entries are re-based to
//! these final absolute addresses and merged into one combined table; a
//! name defined in more than one input object is a
//! [`LinkErrorKind::DuplicateSymbol`] error (linking never silently
//! picks one definition over another). Every relocation is then applied,
//! via [`jxcl_relocations::apply_relocation`], against the combined
//! buffer at its correctly re-based absolute offset; a relocation whose
//! symbol resolves in no object is a [`LinkErrorKind::UndefinedSymbol`]
//! error.
//!
//! At most one input object may set
//! [`jxcl_object::ObjectFile::entry_symbol`]; if none do, the linked
//! program's entry point defaults to `0` (matching the assembler's own
//! default).
#![forbid(unsafe_code)]

use std::fmt;

use jxcl_binary::BinaryContainer;
use jxcl_object::{ObjectFile, RelocationTarget};
use jxcl_relocations::{apply_relocation, Relocation};
use jxcl_symbols::SymbolTable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkErrorKind {
    /// The same symbol name was defined by more than one input object.
    DuplicateSymbol,
    /// A relocation (or the entry point) referenced a symbol no input
    /// object defines.
    UndefinedSymbol,
    /// More than one input object set `entry_symbol`.
    MultipleEntryPoints,
    /// A relocation's field didn't fit the resolved address (e.g. a
    /// `PcRelative32` displacement that overflowed 32 bits).
    RelocationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkError {
    pub kind: LinkErrorKind,
    pub reason: String,
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "link error: {:?}: {}", self.kind, self.reason)
    }
}
impl std::error::Error for LinkError {}

fn err(kind: LinkErrorKind, reason: impl Into<String>) -> LinkError {
    LinkError {
        kind,
        reason: reason.into(),
    }
}

/// Link one or more relocatable object files into a single runnable
/// [`BinaryContainer`].
pub fn link(objects: &[ObjectFile]) -> Result<BinaryContainer, LinkError> {
    // Pass 1: compute each object's placement (running code/data offsets)
    // and the combined section sizes.
    let mut code_offsets = Vec::with_capacity(objects.len());
    let mut data_offsets = Vec::with_capacity(objects.len());
    let mut code_cursor: u64 = 0;
    let mut data_cursor: u64 = 0;
    for obj in objects {
        code_offsets.push(code_cursor);
        data_offsets.push(data_cursor);
        code_cursor += obj.code.len() as u64;
        data_cursor += obj.data.len() as u64;
    }
    let total_code_size = code_cursor;

    // Pass 2: build the combined symbol table at final absolute addresses.
    let mut combined_symbols = SymbolTable::new();
    for (i, obj) in objects.iter().enumerate() {
        for sym in obj.symbols.iter() {
            let abs_address = match sym.section {
                jxcl_symbols::SymbolSection::Code => code_offsets[i] + sym.address,
                jxcl_symbols::SymbolSection::Data => {
                    total_code_size + data_offsets[i] + sym.address
                }
            };
            combined_symbols
                .define(sym.name.clone(), abs_address, sym.section)
                .map_err(|_| {
                    err(
                        LinkErrorKind::DuplicateSymbol,
                        format!(
                            "symbol {:?} is defined in more than one object file",
                            sym.name
                        ),
                    )
                })?;
        }
    }

    // Pass 3: concatenate code and data into one combined buffer (code
    // first, data immediately after -- matching jxcl-binary's layout).
    let mut combined: Vec<u8> = Vec::with_capacity(
        objects.iter().map(|o| o.code.len()).sum::<usize>()
            + objects.iter().map(|o| o.data.len()).sum::<usize>(),
    );
    for obj in objects {
        combined.extend_from_slice(&obj.code);
    }
    for obj in objects {
        combined.extend_from_slice(&obj.data);
    }

    // Pass 4: apply every relocation at its re-based absolute offset.
    for (i, obj) in objects.iter().enumerate() {
        for tr in &obj.relocations {
            let abs_offset = match tr.target {
                RelocationTarget::Code => code_offsets[i] + tr.relocation.offset,
                RelocationTarget::Data => total_code_size + data_offsets[i] + tr.relocation.offset,
            };
            let resolved = combined_symbols
                .resolve(&tr.relocation.symbol)
                .ok_or_else(|| {
                    err(
                        LinkErrorKind::UndefinedSymbol,
                        format!("undefined symbol {:?}", tr.relocation.symbol),
                    )
                })?;
            let reloc =
                Relocation::new(abs_offset, tr.relocation.kind, tr.relocation.symbol.clone());
            apply_relocation(&mut combined, &reloc, resolved.address)
                .map_err(|e| err(LinkErrorKind::RelocationFailed, e.to_string()))?;
        }
    }

    // Pass 5: resolve the entry point, if any object nominated one.
    let mut entry_symbol: Option<&str> = None;
    for obj in objects {
        if let Some(name) = &obj.entry_symbol {
            if entry_symbol.is_some() {
                return Err(err(
                    LinkErrorKind::MultipleEntryPoints,
                    "more than one object file set an entry symbol",
                ));
            }
            entry_symbol = Some(name.as_str());
        }
    }
    let entry_point = match entry_symbol {
        Some(name) => {
            combined_symbols
                .resolve(name)
                .ok_or_else(|| {
                    err(
                        LinkErrorKind::UndefinedSymbol,
                        format!("entry symbol {:?} is undefined", name),
                    )
                })?
                .address
        }
        None => 0,
    };

    let (code, data) = combined.split_at(total_code_size as usize);
    Ok(BinaryContainer::new(
        entry_point,
        code.to_vec(),
        data.to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_object::TargetedRelocation;
    use jxcl_relocations::RelocationKind;
    use jxcl_symbols::SymbolSection;

    #[test]
    fn links_a_single_object_with_no_relocations() {
        let obj = ObjectFile::new(vec![0x60], vec![]).with_entry_symbol("start");
        let mut obj = obj;
        obj.symbols.define("start", 0, SymbolSection::Code).unwrap();
        let bin = link(&[obj]).unwrap();
        assert_eq!(bin.entry_point, 0);
        assert_eq!(bin.code, vec![0x60]);
    }

    #[test]
    fn defaults_entry_point_to_zero_when_unset() {
        let obj = ObjectFile::new(vec![0x60], vec![]);
        let bin = link(&[obj]).unwrap();
        assert_eq!(bin.entry_point, 0);
    }

    #[test]
    fn links_two_objects_with_a_cross_reference() {
        // Object A: a 5-byte "CALL helper" placeholder instruction
        // (1 opcode byte + 4-byte PC-relative displacement) followed by
        // a 1-byte HALT. It references "helper", defined in object B.
        let mut a = ObjectFile::new(vec![0xABu8, 0, 0, 0, 0, 0x60], vec![]);
        a.relocations.push(TargetedRelocation::new(
            jxcl_object::RelocationTarget::Code,
            jxcl_relocations::Relocation::new(1, RelocationKind::PcRelative32, "helper"),
        ));
        a = a.with_entry_symbol("main");
        a.symbols.define("main", 0, SymbolSection::Code).unwrap();

        // Object B: "helper" is a single HALT at its own offset 0.
        let mut b = ObjectFile::new(vec![0x60], vec![]);
        b.symbols.define("helper", 0, SymbolSection::Code).unwrap();

        let bin = link(&[a, b]).unwrap();
        // Object A occupies bytes [0, 6); object B is appended at 6.
        assert_eq!(bin.code.len(), 7);
        assert_eq!(bin.entry_point, 0);
        // The relocation field is bytes [1, 5); pc_after_fetch = 5;
        // helper's final absolute address is 6 (start of object B's code).
        let disp = i32::from_le_bytes(bin.code[1..5].try_into().unwrap());
        assert_eq!(disp, 1); // 6 - 5
        assert_eq!(bin.code[6], 0x60);
    }

    #[test]
    fn cross_referencing_a_data_symbol_resolves_past_the_combined_code_section() {
        // A single object whose code references a data-section symbol
        // via an absolute-64 relocation (like MOVI loading a label's
        // address). Exercises the code-size-relative data placement.
        let mut obj = ObjectFile::new(
            vec![0x02u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0x60], // MOVI-shaped + HALT
            vec![0xEFu8, 0xBE, 0, 0, 0, 0, 0, 0],
        );
        obj.relocations.push(TargetedRelocation::new(
            jxcl_object::RelocationTarget::Code,
            jxcl_relocations::Relocation::new(2, RelocationKind::Absolute64, "buffer"),
        ));
        obj.symbols
            .define("buffer", 0, SymbolSection::Data)
            .unwrap();

        let bin = link(&[obj]).unwrap();
        assert_eq!(bin.code.len(), 11);
        assert_eq!(bin.data.len(), 8);
        // "buffer"'s absolute address equals the total code size (11).
        let patched = u64::from_le_bytes(bin.code[2..10].try_into().unwrap());
        assert_eq!(patched, 11);
    }

    #[test]
    fn undefined_symbol_relocation_is_an_error() {
        let mut obj = ObjectFile::new(vec![0xABu8, 0, 0, 0, 0], vec![]);
        obj.relocations.push(TargetedRelocation::new(
            jxcl_object::RelocationTarget::Code,
            jxcl_relocations::Relocation::new(1, RelocationKind::PcRelative32, "nowhere"),
        ));
        let err = link(&[obj]).unwrap_err();
        assert_eq!(err.kind, LinkErrorKind::UndefinedSymbol);
    }

    #[test]
    fn duplicate_symbol_across_objects_is_an_error() {
        let mut a = ObjectFile::new(vec![0x60], vec![]);
        a.symbols.define("dup", 0, SymbolSection::Code).unwrap();
        let mut b = ObjectFile::new(vec![0x60], vec![]);
        b.symbols.define("dup", 0, SymbolSection::Code).unwrap();

        let err = link(&[a, b]).unwrap_err();
        assert_eq!(err.kind, LinkErrorKind::DuplicateSymbol);
    }

    #[test]
    fn multiple_entry_points_is_an_error() {
        let mut a = ObjectFile::new(vec![0x60], vec![]).with_entry_symbol("a_main");
        a.symbols.define("a_main", 0, SymbolSection::Code).unwrap();
        let mut b = ObjectFile::new(vec![0x60], vec![]).with_entry_symbol("b_main");
        b.symbols.define("b_main", 0, SymbolSection::Code).unwrap();

        let err = link(&[a, b]).unwrap_err();
        assert_eq!(err.kind, LinkErrorKind::MultipleEntryPoints);
    }

    #[test]
    fn undefined_entry_symbol_is_an_error() {
        let obj = ObjectFile::new(vec![0x60], vec![]).with_entry_symbol("ghost");
        let err = link(&[obj]).unwrap_err();
        assert_eq!(err.kind, LinkErrorKind::UndefinedSymbol);
    }

    #[test]
    fn relocation_displacement_out_of_range_is_an_error() {
        let mut obj = ObjectFile::new(vec![0xABu8, 0, 0, 0, 0], vec![]);
        obj.relocations.push(TargetedRelocation::new(
            jxcl_object::RelocationTarget::Code,
            jxcl_relocations::Relocation::new(1, RelocationKind::PcRelative32, "far"),
        ));
        // A symbol whose address is nowhere near the relocation site --
        // still representable as a u64, but its computed displacement
        // overflows i32.
        obj.symbols
            .define("far", u64::MAX / 2, SymbolSection::Code)
            .unwrap();
        let err = link(&[obj]).unwrap_err();
        assert_eq!(err.kind, LinkErrorKind::RelocationFailed);
    }

    #[test]
    fn empty_object_list_links_to_an_empty_binary() {
        let bin = link(&[]).unwrap();
        assert!(bin.code.is_empty());
        assert!(bin.data.is_empty());
        assert_eq!(bin.entry_point, 0);
    }
}
