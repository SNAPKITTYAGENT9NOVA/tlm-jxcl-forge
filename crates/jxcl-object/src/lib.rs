//! A relocatable object-file format (sections + symbol table +
//! relocation table) distinct from the final linked binary container
//! (`jxcl-binary`).
//!
//! New functionality: the pre-expansion `jxcl` crate had no notion of
//! separate compilation, so there is no ground-truth file to extract
//! this from -- `jxcl-assembler`'s single-file workflow always produced
//! a finished `jxcl-binary::BinaryContainer` directly. This crate gives
//! the toolchain a genuinely separate, relocatable intermediate format:
//! an [`ObjectFile`] carries its own code/data bytes, an *unresolved*
//! [`jxcl_symbols::SymbolTable`] (addresses are section-local), and a
//! list of [`jxcl_relocations::Relocation`]s recording every place a
//! symbol reference was left unresolved. `jxcl-linker` is the only
//! consumer that resolves symbols and applies those relocations to
//! produce a final `jxcl-binary::BinaryContainer`.
//!
//! ## Wire format
//!
//! ```text
//! offset  size  field
//! 0       4     MAGIC          b"JXOB"
//! 4       2     VERSION        (u16 LE, currently 1)
//! 6       2     RESERVED       (must be 0)
//! 8       4     CODE_LEN       (u32 LE)
//! 12      4     DATA_LEN       (u32 LE)
//! 16      4     SYMBOL_COUNT   (u32 LE)
//! 20      4     RELOC_COUNT    (u32 LE)
//! 24      1     HAS_ENTRY      (0 or 1)
//! 25      2     ENTRY_NAME_LEN (u16 LE, 0 when HAS_ENTRY == 0)
//! 27      *     ENTRY_NAME     (UTF-8 bytes, ENTRY_NAME_LEN of them)
//!         *     CODE           (CODE_LEN bytes)
//!         *     DATA           (DATA_LEN bytes)
//!         *     SYMBOLS        (SYMBOL_COUNT entries, ascending name order)
//!         *     RELOCATIONS    (RELOC_COUNT entries)
//! ```
//!
//! Each symbol entry is `name_len: u16 LE, name bytes, address: u64 LE,
//! section_tag: u8` (tag per [`jxcl_symbols::SymbolSection::tag`]).
//!
//! Each relocation entry is `offset: u64 LE, target_tag: u8 (0 = code,
//! 1 = data), kind_tag: u8 (0 = Absolute64, 1 = PcRelative32),
//! symbol_name_len: u16 LE, symbol_name bytes`.
#![forbid(unsafe_code)]

use std::fmt;

use jxcl_bytes::{ByteCursor, ByteCursorMut};
use jxcl_relocations::{Relocation, RelocationKind};
use jxcl_symbols::{SymbolSection, SymbolTable};

const MAGIC: [u8; 4] = *b"JXOB";
const VERSION: u16 = 1;

/// Which section of the object file a relocation's field lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocationTarget {
    Code,
    Data,
}

impl RelocationTarget {
    const fn tag(self) -> u8 {
        match self {
            RelocationTarget::Code => 0,
            RelocationTarget::Data => 1,
        }
    }

    fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(RelocationTarget::Code),
            1 => Some(RelocationTarget::Data),
            _ => None,
        }
    }
}

/// One pending fixup, tagged with which section's bytes it patches. This
/// wraps a [`jxcl_relocations::Relocation`] (which only knows a
/// buffer-local byte offset) with the extra bit of information an object
/// file needs -- *which* buffer -- since an object file has two
/// (code and data), not one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetedRelocation {
    pub target: RelocationTarget,
    pub relocation: Relocation,
}

impl TargetedRelocation {
    pub fn new(target: RelocationTarget, relocation: Relocation) -> Self {
        TargetedRelocation { target, relocation }
    }
}

/// A relocatable object file: not yet runnable, but ready for
/// `jxcl-linker` to combine with others and resolve into a
/// `jxcl-binary::BinaryContainer`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObjectFile {
    pub code: Vec<u8>,
    pub data: Vec<u8>,
    /// Section-local (not yet linked/absolute) symbol addresses.
    pub symbols: SymbolTable,
    pub relocations: Vec<TargetedRelocation>,
    /// The name of the symbol this object file nominates as the program
    /// entry point, if any (only meaningful when this object is linked
    /// alone, or is the object the linker is told holds `main`).
    pub entry_symbol: Option<String>,
}

impl ObjectFile {
    pub fn new(code: Vec<u8>, data: Vec<u8>) -> Self {
        ObjectFile {
            code,
            data,
            symbols: SymbolTable::new(),
            relocations: Vec::new(),
            entry_symbol: None,
        }
    }

    pub fn with_entry_symbol(mut self, name: impl Into<String>) -> Self {
        self.entry_symbol = Some(name.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectErrorKind {
    TooShort,
    BadMagic,
    UnsupportedVersion,
    Truncated,
    InvalidUtf8,
    InvalidSectionTag,
    InvalidRelocationTag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectError {
    pub kind: ObjectErrorKind,
    pub reason: String,
}

impl fmt::Display for ObjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "object format error: {:?}: {}", self.kind, self.reason)
    }
}
impl std::error::Error for ObjectError {}

fn err(kind: ObjectErrorKind, reason: impl Into<String>) -> ObjectError {
    ObjectError {
        kind,
        reason: reason.into(),
    }
}

fn cursor_err(e: jxcl_errors::Error) -> ObjectError {
    err(ObjectErrorKind::Truncated, e.to_string())
}

fn write_name(out: &mut ByteCursorMut, name: &str) {
    let bytes = name.as_bytes();
    out.write_u16(bytes.len() as u16)
        .expect("ByteCursorMut::write_bytes never fails");
    out.write_bytes(bytes)
        .expect("ByteCursorMut::write_bytes never fails");
}

fn read_name<'a>(cur: &mut ByteCursor<'a>) -> Result<String, ObjectError> {
    let len = cur.read_u16().map_err(cursor_err)? as usize;
    let bytes = cur.read_bytes(len).map_err(cursor_err)?;
    String::from_utf8(bytes.to_vec()).map_err(|e| err(ObjectErrorKind::InvalidUtf8, e.to_string()))
}

/// A stable one-byte tag for [`RelocationKind`], local to this crate's
/// wire format (an inherent `impl` on `RelocationKind` itself is not
/// possible here -- it's a foreign type owned by `jxcl-relocations`).
const fn relocation_kind_tag(kind: RelocationKind) -> u8 {
    match kind {
        RelocationKind::Absolute64 => 0,
        RelocationKind::PcRelative32 => 1,
    }
}

fn relocation_kind_from_tag(tag: u8) -> Option<RelocationKind> {
    match tag {
        0 => Some(RelocationKind::Absolute64),
        1 => Some(RelocationKind::PcRelative32),
        _ => None,
    }
}

impl ObjectFile {
    /// Serialize to the on-disk `.jxo` object format described in this
    /// crate's module docs.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = ByteCursorMut::new();
        out.write_bytes(&MAGIC).unwrap();
        out.write_u16(VERSION).unwrap();
        out.write_u16(0).unwrap(); // RESERVED
        out.write_u32(self.code.len() as u32).unwrap();
        out.write_u32(self.data.len() as u32).unwrap();
        out.write_u32(self.symbols.len() as u32).unwrap();
        out.write_u32(self.relocations.len() as u32).unwrap();
        match &self.entry_symbol {
            Some(name) => {
                out.write_u8(1).unwrap();
                write_name(&mut out, name);
            }
            None => {
                out.write_u8(0).unwrap();
                out.write_u16(0).unwrap();
            }
        }
        out.write_bytes(&self.code).unwrap();
        out.write_bytes(&self.data).unwrap();
        for sym in self.symbols.iter() {
            write_name(&mut out, &sym.name);
            out.write_u64(sym.address).unwrap();
            out.write_u8(sym.section.tag()).unwrap();
        }
        for tr in &self.relocations {
            out.write_u64(tr.relocation.offset).unwrap();
            out.write_u8(tr.target.tag()).unwrap();
            out.write_u8(relocation_kind_tag(tr.relocation.kind))
                .unwrap();
            write_name(&mut out, &tr.relocation.symbol);
        }
        out.into_vec()
    }

    /// Inverse of [`ObjectFile::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ObjectError> {
        let mut cur = ByteCursor::new(bytes);
        let magic = cur.read_bytes(4).map_err(cursor_err)?;
        if magic != MAGIC {
            return Err(err(ObjectErrorKind::BadMagic, "missing JXOB magic"));
        }
        let version = cur.read_u16().map_err(cursor_err)?;
        if version != VERSION {
            return Err(err(
                ObjectErrorKind::UnsupportedVersion,
                format!("expected version {}, found {}", VERSION, version),
            ));
        }
        let _reserved = cur.read_u16().map_err(cursor_err)?;
        let code_len = cur.read_u32().map_err(cursor_err)? as usize;
        let data_len = cur.read_u32().map_err(cursor_err)? as usize;
        let symbol_count = cur.read_u32().map_err(cursor_err)?;
        let reloc_count = cur.read_u32().map_err(cursor_err)?;
        let has_entry = cur.read_u8().map_err(cursor_err)? != 0;
        let entry_symbol = if has_entry {
            Some(read_name(&mut cur)?)
        } else {
            let len = cur.read_u16().map_err(cursor_err)?;
            if len != 0 {
                return Err(err(
                    ObjectErrorKind::Truncated,
                    "entry name length must be 0 when HAS_ENTRY is 0",
                ));
            }
            None
        };

        let code = cur.read_bytes(code_len).map_err(cursor_err)?.to_vec();
        let data = cur.read_bytes(data_len).map_err(cursor_err)?.to_vec();

        let mut symbols = SymbolTable::new();
        for _ in 0..symbol_count {
            let name = read_name(&mut cur)?;
            let address = cur.read_u64().map_err(cursor_err)?;
            let tag = cur.read_u8().map_err(cursor_err)?;
            let section = SymbolSection::from_tag(tag)
                .ok_or_else(|| err(ObjectErrorKind::InvalidSectionTag, format!("{}", tag)))?;
            symbols
                .define(name, address, section)
                .map_err(|e| err(ObjectErrorKind::Truncated, e.to_string()))?;
        }

        let mut relocations = Vec::with_capacity(reloc_count as usize);
        for _ in 0..reloc_count {
            let offset = cur.read_u64().map_err(cursor_err)?;
            let target_tag = cur.read_u8().map_err(cursor_err)?;
            let target = RelocationTarget::from_tag(target_tag).ok_or_else(|| {
                err(
                    ObjectErrorKind::InvalidRelocationTag,
                    format!("{}", target_tag),
                )
            })?;
            let kind_tag = cur.read_u8().map_err(cursor_err)?;
            let kind = relocation_kind_from_tag(kind_tag).ok_or_else(|| {
                err(
                    ObjectErrorKind::InvalidRelocationTag,
                    format!("{}", kind_tag),
                )
            })?;
            let symbol = read_name(&mut cur)?;
            relocations.push(TargetedRelocation::new(
                target,
                Relocation::new(offset, kind, symbol),
            ));
        }

        Ok(ObjectFile {
            code,
            data,
            symbols,
            relocations,
            entry_symbol,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_symbols::SymbolSection;

    #[test]
    fn empty_object_roundtrips() {
        let obj = ObjectFile::new(vec![], vec![]);
        let bytes = obj.to_bytes();
        let restored = ObjectFile::from_bytes(&bytes).unwrap();
        assert_eq!(obj, restored);
    }

    #[test]
    fn object_with_symbols_and_relocations_roundtrips() {
        let mut obj = ObjectFile::new(vec![0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0x60], vec![1, 2, 3, 4]);
        obj.symbols.define("start", 0, SymbolSection::Code).unwrap();
        obj.symbols
            .define("buffer", 0, SymbolSection::Data)
            .unwrap();
        obj.relocations.push(TargetedRelocation::new(
            RelocationTarget::Code,
            Relocation::new(2, RelocationKind::Absolute64, "buffer"),
        ));
        obj = obj.with_entry_symbol("start");

        let bytes = obj.to_bytes();
        let restored = ObjectFile::from_bytes(&bytes).unwrap();
        assert_eq!(obj, restored);
        assert_eq!(restored.entry_symbol.as_deref(), Some("start"));
        assert_eq!(restored.relocations.len(), 1);
        assert_eq!(restored.symbols.len(), 2);
    }

    #[test]
    fn rejects_bad_magic() {
        let obj = ObjectFile::new(vec![1], vec![]);
        let mut bytes = obj.to_bytes();
        bytes[0] = b'X';
        assert_eq!(
            ObjectFile::from_bytes(&bytes).unwrap_err().kind,
            ObjectErrorKind::BadMagic
        );
    }

    #[test]
    fn rejects_unsupported_version() {
        let obj = ObjectFile::new(vec![], vec![]);
        let mut bytes = obj.to_bytes();
        bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
        assert_eq!(
            ObjectFile::from_bytes(&bytes).unwrap_err().kind,
            ObjectErrorKind::UnsupportedVersion
        );
    }

    #[test]
    fn rejects_truncated_buffer() {
        let obj = ObjectFile::new(vec![1, 2, 3], vec![]);
        let mut bytes = obj.to_bytes();
        bytes.truncate(bytes.len() - 1);
        assert_eq!(
            ObjectFile::from_bytes(&bytes).unwrap_err().kind,
            ObjectErrorKind::Truncated
        );
    }

    #[test]
    fn boundary_empty_code_and_data_sections() {
        let mut obj = ObjectFile::new(vec![], vec![]);
        obj.symbols.define("only", 0, SymbolSection::Code).unwrap();
        let bytes = obj.to_bytes();
        let restored = ObjectFile::from_bytes(&bytes).unwrap();
        assert!(restored.code.is_empty());
        assert!(restored.data.is_empty());
        assert_eq!(restored.symbols.len(), 1);
    }

    #[test]
    fn symbol_iteration_order_is_preserved_through_serialization() {
        let mut obj = ObjectFile::new(vec![], vec![]);
        obj.symbols.define("zeta", 1, SymbolSection::Code).unwrap();
        obj.symbols.define("alpha", 2, SymbolSection::Code).unwrap();
        let bytes = obj.to_bytes();
        let restored = ObjectFile::from_bytes(&bytes).unwrap();
        let names: Vec<&str> = restored.symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "zeta"]);
    }
}
