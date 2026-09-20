// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The per-step execution trace record format and its writer/reader,
//! reused by the profiler ([`jxcl-profiler`]) and replay ([`jxcl-replay`])
//! crates.
//!
//! This crate factors out the trace-record shape that previously lived
//! inline in `jxcl::debugger` (`TraceEntry`, spec §30) into a
//! standalone, dependency-free type ([`TraceStep`]) plus a log
//! container ([`TraceLog`]). `jxcl-debugger` depends on this crate for
//! its trace-mode output instead of defining its own copy.
//!
//! ## Cross-batch integration note
//!
//! `docs/crates.toml` lists `jxcl-machine` and `jxcl-instructions` as
//! this crate's workspace dependencies (both still scaffolded
//! placeholders as of this batch). `TraceStep` deliberately does not
//! need either crate's concrete types to be real and useful today: the
//! executed instruction is recorded as its rendered text (exactly how
//! `jxcl::debugger::TraceEntry::text` already works — see
//! `jxcl/src/debugger.rs`), and register/flag/stack-pointer/memory
//! deltas are plain integers and byte vectors. Once `jxcl-instructions`
//! lands, a convenience constructor taking a concrete `Instruction` and
//! rendering it can be added without changing `TraceStep`'s shape.

#![forbid(unsafe_code)]

/// One instruction's worth of observable architectural state change,
/// exactly the information `jxcl::debugger::trace_step` (spec §30)
/// already computes: the PC it executed at, the instruction's rendered
/// text, every general-register change, the flags/stack-pointer before
/// and after, and any memory writes it produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceStep {
    pub pc: u64,
    /// The program counter's value immediately after this instruction
    /// executed (i.e. where the *next* fetch reads from). Recorded
    /// explicitly rather than left to be inferred from the next
    /// `TraceStep`'s `pc`, because the last step in a log has no
    /// "next" step to infer it from -- callers that need "the machine
    /// state after the final recorded step" (e.g. `jxcl-replay`) need
    /// this to be self-contained per step.
    pub pc_after: u64,
    /// Canonical textual form of the instruction that executed (or a
    /// placeholder such as `"<undecodable>"` if it could not be
    /// decoded — the fault itself belongs in a real machine's own
    /// fault state, not in this record).
    pub instruction: String,
    /// `(register_index, old_value, new_value)` for every general
    /// register that changed.
    pub register_changes: Vec<(u8, u64, u64)>,
    pub flags_before: u64,
    pub flags_after: u64,
    pub sp_before: u64,
    pub sp_after: u64,
    /// Contiguous runs of changed memory, as `(address, old_bytes, new_bytes)`.
    pub memory_writes: Vec<(u64, Vec<u8>, Vec<u8>)>,
}

impl TraceStep {
    /// Construct a step record for an instruction that changed nothing
    /// but the program counter (the common case for e.g. `NOP`).
    pub fn unchanged(
        pc: u64,
        pc_after: u64,
        instruction: impl Into<String>,
        flags: u64,
        sp: u64,
    ) -> Self {
        TraceStep {
            pc,
            pc_after,
            instruction: instruction.into(),
            register_changes: Vec::new(),
            flags_before: flags,
            flags_after: flags,
            sp_before: sp,
            sp_after: sp,
            memory_writes: Vec::new(),
        }
    }

    /// True if this step changed no observable architectural state at
    /// all besides having executed.
    pub fn is_no_op(&self) -> bool {
        self.register_changes.is_empty()
            && self.flags_before == self.flags_after
            && self.sp_before == self.sp_after
            && self.memory_writes.is_empty()
    }

    /// Serialize to a compact, self-describing, length-prefixed byte
    /// format. No external `serde`-style crate is declared as a
    /// dependency for this crate (`docs/crates.toml`'s
    /// `external_dependencies` is empty for `jxcl-trace`), so this is a
    /// small hand-rolled binary encoding rather than a `serde` impl.
    /// All integers are little-endian, matching the ISA's own
    /// endianness convention (spec §10).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.pc.to_le_bytes());
        out.extend_from_slice(&self.pc_after.to_le_bytes());

        let instr_bytes = self.instruction.as_bytes();
        out.extend_from_slice(&(instr_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(instr_bytes);

        out.extend_from_slice(&(self.register_changes.len() as u32).to_le_bytes());
        for (idx, old, new) in &self.register_changes {
            out.push(*idx);
            out.extend_from_slice(&old.to_le_bytes());
            out.extend_from_slice(&new.to_le_bytes());
        }

        out.extend_from_slice(&self.flags_before.to_le_bytes());
        out.extend_from_slice(&self.flags_after.to_le_bytes());
        out.extend_from_slice(&self.sp_before.to_le_bytes());
        out.extend_from_slice(&self.sp_after.to_le_bytes());

        out.extend_from_slice(&(self.memory_writes.len() as u32).to_le_bytes());
        for (addr, old, new) in &self.memory_writes {
            out.extend_from_slice(&addr.to_le_bytes());
            out.extend_from_slice(&(old.len() as u32).to_le_bytes());
            out.extend_from_slice(old);
            out.extend_from_slice(&(new.len() as u32).to_le_bytes());
            out.extend_from_slice(new);
        }
        out
    }

    /// Inverse of [`TraceStep::to_bytes`]. Returns `Err` with a short
    /// message describing what went wrong (truncated input or an
    /// internal length that runs past the end of the buffer) rather
    /// than panicking, so malformed trace files fail closed.
    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), String> {
        let mut cursor = Cursor::new(bytes);
        let pc = cursor.read_u64()?;
        let pc_after = cursor.read_u64()?;

        let instr_len = cursor.read_u32()? as usize;
        let instruction = String::from_utf8(cursor.read_bytes(instr_len)?.to_vec())
            .map_err(|e| format!("instruction text was not valid UTF-8: {e}"))?;

        let reg_count = cursor.read_u32()? as usize;
        let mut register_changes = Vec::with_capacity(reg_count);
        for _ in 0..reg_count {
            let idx = cursor.read_u8()?;
            let old = cursor.read_u64()?;
            let new = cursor.read_u64()?;
            register_changes.push((idx, old, new));
        }

        let flags_before = cursor.read_u64()?;
        let flags_after = cursor.read_u64()?;
        let sp_before = cursor.read_u64()?;
        let sp_after = cursor.read_u64()?;

        let write_count = cursor.read_u32()? as usize;
        let mut memory_writes = Vec::with_capacity(write_count);
        for _ in 0..write_count {
            let addr = cursor.read_u64()?;
            let old_len = cursor.read_u32()? as usize;
            let old = cursor.read_bytes(old_len)?.to_vec();
            let new_len = cursor.read_u32()? as usize;
            let new = cursor.read_bytes(new_len)?.to_vec();
            memory_writes.push((addr, old, new));
        }

        Ok((
            TraceStep {
                pc,
                pc_after,
                instruction,
                register_changes,
                flags_before,
                flags_after,
                sp_before,
                sp_after,
                memory_writes,
            },
            cursor.pos,
        ))
    }
}

/// A minimal bounds-checked byte cursor used only by
/// [`TraceStep::from_bytes`]. Kept private and tiny rather than pulling
/// in `jxcl-bytes` (not a declared dependency of this crate).
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Cursor { bytes, pos: 0 }
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| "length overflow while decoding trace step".to_string())?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| "truncated trace step data".to_string())?;
        self.pos = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_bytes(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_le_bytes(b.try_into().unwrap()))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let b = self.read_bytes(8)?;
        Ok(u64::from_le_bytes(b.try_into().unwrap()))
    }
}

/// An ordered log of [`TraceStep`]s recorded across a run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceLog {
    steps: Vec<TraceStep>,
}

impl TraceLog {
    pub fn new() -> Self {
        TraceLog::default()
    }

    /// Append one recorded step to the end of the log.
    pub fn push(&mut self, step: TraceStep) {
        self.steps.push(step);
    }

    /// Iterate the recorded steps in execution order.
    pub fn iter(&self) -> std::slice::Iter<'_, TraceStep> {
        self.steps.iter()
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Borrow the step at index `n`, if it exists.
    pub fn get(&self, n: usize) -> Option<&TraceStep> {
        self.steps.get(n)
    }

    /// Serialize the entire log: a little-endian step count followed by
    /// each step's [`TraceStep::to_bytes`] encoding, back to back.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(self.steps.len() as u32).to_le_bytes());
        for step in &self.steps {
            out.extend_from_slice(&step.to_bytes());
        }
        out
    }

    /// Inverse of [`TraceLog::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut cursor = Cursor::new(bytes);
        let count = cursor.read_u32()? as usize;
        let mut steps = Vec::with_capacity(count);
        for _ in 0..count {
            let remaining = &bytes[cursor.pos..];
            let (step, consumed) = TraceStep::from_bytes(remaining)?;
            cursor.pos += consumed;
            steps.push(step);
        }
        Ok(TraceLog { steps })
    }
}

impl<'a> IntoIterator for &'a TraceLog {
    type Item = &'a TraceStep;
    type IntoIter = std::slice::Iter<'a, TraceStep>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<TraceStep> for TraceLog {
    fn from_iter<I: IntoIterator<Item = TraceStep>>(iter: I) -> Self {
        TraceLog {
            steps: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_step(pc: u64) -> TraceStep {
        TraceStep {
            pc,
            pc_after: pc + 4,
            instruction: format!("MOVI R1, {pc}"),
            register_changes: vec![(1, 0, pc)],
            flags_before: 0,
            flags_after: 0b0001,
            sp_before: 256,
            sp_after: 256,
            memory_writes: vec![(64, vec![0, 0], vec![0x2A, 0x00])],
        }
    }

    #[test]
    fn trace_log_push_and_iter_preserve_order() {
        let mut log = TraceLog::new();
        assert!(log.is_empty());
        for pc in [0, 4, 8, 12] {
            log.push(sample_step(pc));
        }
        assert_eq!(log.len(), 4);
        let pcs: Vec<u64> = log.iter().map(|s| s.pc).collect();
        assert_eq!(pcs, vec![0, 4, 8, 12]);
        assert_eq!(log.get(2).unwrap().pc, 8);
        assert!(log.get(4).is_none());
    }

    #[test]
    fn trace_log_from_iterator() {
        let log: TraceLog = (0..3).map(sample_step).collect();
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn unchanged_step_is_a_no_op() {
        let step = TraceStep::unchanged(0, 4, "NOP", 0, 256);
        assert!(step.is_no_op());
        let mut changed = step.clone();
        changed.register_changes.push((0, 0, 1));
        assert!(!changed.is_no_op());
    }

    #[test]
    fn trace_step_serialization_roundtrips() {
        let step = sample_step(1024);
        let bytes = step.to_bytes();
        let (decoded, consumed) = TraceStep::from_bytes(&bytes).expect("decode must succeed");
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded, step);
    }

    #[test]
    fn trace_step_serialization_roundtrips_with_no_changes() {
        let step = TraceStep::unchanged(0, 0, "HALT", 0, 0);
        let bytes = step.to_bytes();
        let (decoded, _) = TraceStep::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, step);
    }

    #[test]
    fn trace_log_serialization_roundtrips() {
        let mut log = TraceLog::new();
        for pc in [0u64, 4, 8, 16, 1024] {
            log.push(sample_step(pc));
        }
        let bytes = log.to_bytes();
        let decoded = TraceLog::from_bytes(&bytes).expect("decode must succeed");
        assert_eq!(decoded, log);
    }

    #[test]
    fn empty_trace_log_serialization_roundtrips() {
        let log = TraceLog::new();
        let bytes = log.to_bytes();
        let decoded = TraceLog::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, log);
        assert!(decoded.is_empty());
    }

    #[test]
    fn from_bytes_fails_closed_on_truncated_input() {
        let step = sample_step(1);
        let bytes = step.to_bytes();
        // Truncate partway through the encoding; must return Err, never panic.
        let truncated = &bytes[..bytes.len() - 3];
        assert!(TraceStep::from_bytes(truncated).is_err());
    }

    #[test]
    fn from_bytes_fails_closed_on_empty_input() {
        assert!(TraceStep::from_bytes(&[]).is_err());
        assert!(TraceLog::from_bytes(&[]).is_err());
    }

    #[test]
    fn from_bytes_never_panics_on_arbitrary_short_buffers() {
        // A lightweight decoder-boundary fuzz check consistent with the
        // rest of this workspace's "never panic on malformed input"
        // convention (spec §33 / jxcl/tests/fuzz_decoder.rs).
        let mut state: u64 = 0xC0FFEE;
        for len in 0..64usize {
            let mut buf = vec![0u8; len];
            for b in buf.iter_mut() {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *b = (state & 0xFF) as u8;
            }
            let _ = TraceStep::from_bytes(&buf);
        }
    }
}
