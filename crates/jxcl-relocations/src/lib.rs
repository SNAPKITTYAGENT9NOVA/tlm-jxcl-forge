//! Relocation entry types and the patch-in-place logic that fixes up an
//! encoded instruction once a symbol's final address is known.
//!
//! Owns: [`Relocation`] and [`apply_relocation`] -- the only place a
//! previously-encoded instruction's bytes are ever patched after the fact.
//! `jxcl-assembler` uses this to resolve same-file forward references and
//! `jxcl-linker` uses the identical function to resolve cross-object-file
//! references, so the two never diverge on what "patch this field" means.
//!
//! ## On `jxcl-bitops`
//!
//! This crate's `docs/crates.toml` entry lists `jxcl-bitops` as a
//! dependency (bitfield insertion is conceptually a bit-level operation).
//! At the time this crate was implemented, `jxcl-bitops` was still a
//! scaffolded placeholder with no public items, so the patch below is
//! implemented directly against byte slices (every JXCL relocatable field
//! is byte-aligned, so this needs no sub-byte bit manipulation, only
//! little-endian byte writes). Once `jxcl-bitops` lands with a real
//! `insert_bits`/`extract_bits`, `apply_relocation`'s field-write should be
//! refactored to go through it for consistency with the rest of the
//! encoding stack -- functionally nothing here would change, since the
//! fields being patched are already whole little-endian integers.

#![forbid(unsafe_code)]

use std::fmt;

/// What kind of field a [`Relocation`] patches, and how the replacement
/// value is computed from a symbol's resolved address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocationKind {
    /// A full 64-bit little-endian absolute address (e.g. `MOVI`'s
    /// immediate field, when the immediate is really "the address of a
    /// label").
    Absolute64,
    /// A 32-bit little-endian PC-relative displacement (e.g. a branch's
    /// `disp32` field), computed as
    /// `resolved_address - (field_offset + 4)` -- i.e. relative to the
    /// address of the byte immediately after the 4-byte field itself,
    /// matching the ISA's `pc_after_fetch` convention (the field is always
    /// the trailing bytes of the instruction it belongs to).
    PcRelative32,
}

impl RelocationKind {
    /// Width, in bytes, of the field this relocation kind patches.
    pub const fn field_len(self) -> usize {
        match self {
            RelocationKind::Absolute64 => 8,
            RelocationKind::PcRelative32 => 4,
        }
    }
}

/// One pending fixup: "the bytes at `offset` need to be patched once
/// `symbol` is resolved."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relocation {
    /// Byte offset (into whatever buffer `apply_relocation` is given --
    /// typically a section's raw bytes) where the field to patch begins.
    pub offset: u64,
    pub kind: RelocationKind,
    /// Name of the symbol whose resolved address supplies the value.
    pub symbol: String,
}

impl Relocation {
    pub fn new(offset: u64, kind: RelocationKind, symbol: impl Into<String>) -> Self {
        Relocation {
            offset,
            kind,
            symbol: symbol.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelocationErrorKind {
    /// The relocation's field does not fit within the given buffer.
    OutOfBounds,
    /// A `PcRelative32` displacement did not fit in 32 bits.
    DisplacementOutOfRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelocationError {
    pub kind: RelocationErrorKind,
    pub reason: String,
}

impl fmt::Display for RelocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "relocation error: {:?}: {}", self.kind, self.reason)
    }
}
impl std::error::Error for RelocationError {}

/// Patch `bytes[relocation.offset .. ]` in place so that it encodes
/// `resolved_address`, according to `relocation.kind`.
pub fn apply_relocation(
    bytes: &mut [u8],
    relocation: &Relocation,
    resolved_address: u64,
) -> Result<(), RelocationError> {
    let start = relocation.offset as usize;
    let len = relocation.kind.field_len();
    let end = start.checked_add(len).ok_or_else(|| RelocationError {
        kind: RelocationErrorKind::OutOfBounds,
        reason: "relocation offset + field length overflows usize".into(),
    })?;
    if end > bytes.len() {
        return Err(RelocationError {
            kind: RelocationErrorKind::OutOfBounds,
            reason: format!(
                "relocation field [{}, {}) exceeds buffer of length {}",
                start,
                end,
                bytes.len()
            ),
        });
    }

    match relocation.kind {
        RelocationKind::Absolute64 => {
            bytes[start..end].copy_from_slice(&resolved_address.to_le_bytes());
        }
        RelocationKind::PcRelative32 => {
            // Widen to i128 first: a naive `u64 as i64` cast silently
            // reinterprets bits (e.g. u64::MAX becomes -1) instead of
            // reporting that the address is out of range, which would
            // let a bogus resolved_address slip past the i32 fits-check
            // below undetected.
            let pc_after_fetch = end as i128;
            let disp = resolved_address as i128 - pc_after_fetch;
            let disp = i32::try_from(disp).map_err(|_| RelocationError {
                kind: RelocationErrorKind::DisplacementOutOfRange,
                reason: format!(
                    "displacement {} to {:#x} does not fit in 32 bits",
                    disp, resolved_address
                ),
            })?;
            bytes[start..end].copy_from_slice(&disp.to_le_bytes());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_absolute64_field() {
        let mut bytes = vec![0xAAu8; 9]; // e.g. opcode byte + 8-byte immediate
        let reloc = Relocation::new(1, RelocationKind::Absolute64, "buffer");
        apply_relocation(&mut bytes, &reloc, 0x0102_0304_0506_0708).unwrap();
        assert_eq!(bytes[0], 0xAA); // untouched opcode byte
        assert_eq!(&bytes[1..9], &0x0102_0304_0506_0708u64.to_le_bytes());
    }

    #[test]
    fn patches_pc_relative32_field_forward() {
        // Field occupies bytes [1, 5); pc_after_fetch = 5.
        let mut bytes = vec![0u8; 5];
        let reloc = Relocation::new(1, RelocationKind::PcRelative32, "target");
        apply_relocation(&mut bytes, &reloc, 15).unwrap();
        let disp = i32::from_le_bytes(bytes[1..5].try_into().unwrap());
        assert_eq!(disp, 10); // 15 - 5
    }

    #[test]
    fn patches_pc_relative32_field_backward() {
        let mut bytes = vec![0u8; 5];
        let reloc = Relocation::new(1, RelocationKind::PcRelative32, "target");
        apply_relocation(&mut bytes, &reloc, 0).unwrap();
        let disp = i32::from_le_bytes(bytes[1..5].try_into().unwrap());
        assert_eq!(disp, -5); // 0 - 5
    }

    #[test]
    fn rejects_out_of_bounds_offset() {
        let mut bytes = vec![0u8; 4];
        let reloc = Relocation::new(1, RelocationKind::Absolute64, "x");
        let err = apply_relocation(&mut bytes, &reloc, 1).unwrap_err();
        assert_eq!(err.kind, RelocationErrorKind::OutOfBounds);
    }

    #[test]
    fn rejects_displacement_too_large_for_32_bits() {
        let mut bytes = vec![0u8; 5];
        let reloc = Relocation::new(1, RelocationKind::PcRelative32, "far");
        let err = apply_relocation(&mut bytes, &reloc, u64::MAX).unwrap_err();
        assert_eq!(err.kind, RelocationErrorKind::DisplacementOutOfRange);
    }

    #[test]
    fn field_len_matches_kind() {
        assert_eq!(RelocationKind::Absolute64.field_len(), 8);
        assert_eq!(RelocationKind::PcRelative32.field_len(), 4);
    }
}
