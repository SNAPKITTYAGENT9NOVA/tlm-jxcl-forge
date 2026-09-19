//! The golden-vector format, loader, and canonical vector files used as
//! the ISA conformance baseline.
//!
//! Owns: [`GoldenVector`] and `tests/vectors/`.
//!
//! Ported from `crates/jxcl/tests/golden_vectors.rs` (spec §38): a fixed
//! program plus an expected final machine state (registers, flags,
//! halt/fault), which a future RTL simulator would replay against real
//! hardware to confirm it implements the same architecture. This crate
//! turns that test file's inline `Golden` table into a real, reusable,
//! data-driven format plus a standalone loader, so any crate (not just
//! `jxcl`'s own test suite) can validate a machine against these vectors.
//!
//! ## Cross-batch integration note
//!
//! `docs/crates.toml` lists `jxcl-assembler` as a workspace dependency
//! (`Cargo.toml` declares it as a path dependency to match); it is still
//! a scaffolded placeholder as of this batch, with no `assemble` function
//! to turn assembly source text into bytes. `GoldenVector::program`
//! therefore holds hand-encoded instruction bytes directly (against
//! [`jxcl-simulator`]'s real opcode table -- see its module doc comment
//! for exactly which mnemonics that covers) rather than assembly source
//! text, exactly as this batch's instructions anticipate for
//! `jxcl-simulator` itself. Once `jxcl-assembler` lands, an
//! `assemble_vector(source: &str) -> Result<GoldenVector, ...>`
//! convenience can be added without changing `GoldenVector`'s shape --
//! `program` would simply come from `jxcl_assembler::assemble` instead of
//! a hex literal in a fixture file.
//!
//! ## Vector file format
//!
//! Each file in `tests/vectors/` (extension `.golden`) is one
//! [`GoldenVector`], not a `serde`-driven serialization format (there is
//! no format crate declared for this crate beyond `serde` itself -- see
//! `docs/crates.toml`'s `external_dependencies`). It's a small
//! dependency-free `key: value` text format, one field per line, parsed
//! by [`load_vectors`]/[`parse_vector`] and documented positively so any
//! vector file is not a required exercise:
//!
//! ```text
//! name: add_two_constants
//! program: 02 01 0A 00 00 00 00 00 00 00 02 02 14 00 00 00 00 00 00 00 10 01 02 60
//! halted: true
//! fault: none
//! registers: 1=30, 2=20
//! flags: none
//! ```
//!
//! `GoldenVector` itself derives `serde::{Serialize, Deserialize}` (this
//! crate's real use of the `serde` dependency `docs/crates.toml` lists)
//! so callers that already depend on a `serde` data-format crate of
//! their own (JSON, TOML, ...) can (de)serialize vectors through that
//! format without this crate needing to pick one for them.

#![forbid(unsafe_code)]

use jxcl_simulator::Simulator;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A golden vector: a fixed hand-encoded program plus every expectation
/// about the machine's state after running it to completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenVector {
    pub name: String,
    /// Raw instruction bytes, hand-encoded against `jxcl-simulator`'s
    /// real opcode table (see this module's "Cross-batch integration
    /// note").
    pub program: Vec<u8>,
    pub expect_halted: bool,
    /// `Some(fault_name)` (e.g. `"DivideByZero"`), matching
    /// `jxcl_simulator::RunResult::fault`'s `{:?}`-formatted text, or
    /// `None` if the vector expects a clean run.
    pub expect_fault: Option<String>,
    /// `(register_id, expected_value)` pairs; only the registers a
    /// vector cares about need to be listed.
    pub expect_registers: Vec<(u8, u64)>,
    pub expect_flags: Option<u64>,
}

/// One mismatch between a [`GoldenVector`]'s expectations and what
/// actually happened when it ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoldenFailure(pub String);

impl std::fmt::Display for GoldenFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for GoldenFailure {}

/// Run `vector.program` to completion on a fresh [`jxcl_simulator::Simulator`]
/// and check every expectation `vector` declares. Returns every mismatch
/// found (never just the first), or `Ok(())` if the run matched every
/// expectation.
pub fn run_vector(vector: &GoldenVector) -> Result<(), Vec<GoldenFailure>> {
    let mut sim = Simulator::new(vector.program.clone());
    let result = sim.run();
    let mut failures = Vec::new();

    if result.halted != vector.expect_halted {
        failures.push(GoldenFailure(format!(
            "[{}] halted mismatch: expected {}, got {}",
            vector.name, vector.expect_halted, result.halted
        )));
    }

    match (&result.fault, &vector.expect_fault) {
        (Some(actual), Some(expected)) if actual != expected => {
            failures.push(GoldenFailure(format!(
                "[{}] fault mismatch: expected {expected}, got {actual}",
                vector.name
            )));
        }
        (Some(actual), None) => {
            failures.push(GoldenFailure(format!(
                "[{}] unexpected fault {actual}",
                vector.name
            )));
        }
        (None, Some(expected)) => {
            failures.push(GoldenFailure(format!(
                "[{}] expected fault {expected} but the run completed cleanly",
                vector.name
            )));
        }
        _ => {}
    }

    for (reg, expected) in &vector.expect_registers {
        let actual = result.register(*reg);
        if actual != *expected {
            failures.push(GoldenFailure(format!(
                "[{}] R{reg} mismatch: expected {expected:#x}, got {actual:#x}",
                vector.name
            )));
        }
    }

    if let Some(expected_flags) = vector.expect_flags {
        if result.flags != expected_flags {
            failures.push(GoldenFailure(format!(
                "[{}] flags mismatch: expected {expected_flags:#06b}, got {:#06b}",
                vector.name, result.flags
            )));
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn parse_number(text: &str) -> Result<u64, String> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).map_err(|e| format!("bad hex number {text:?}: {e}"))
    } else if let Some(bin) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
        u64::from_str_radix(bin, 2).map_err(|e| format!("bad binary number {text:?}: {e}"))
    } else {
        text.parse::<u64>()
            .map_err(|e| format!("bad decimal number {text:?}: {e}"))
    }
}

fn parse_program(text: &str) -> Result<Vec<u8>, String> {
    text.split_whitespace()
        .map(|tok| {
            u8::from_str_radix(tok, 16).map_err(|e| format!("bad program byte {tok:?}: {e}"))
        })
        .collect()
}

fn parse_registers(text: &str) -> Result<Vec<(u8, u64)>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(Vec::new());
    }
    text.split(',')
        .map(|pair| {
            let pair = pair.trim();
            let (id, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("bad register entry {pair:?}: expected `id=value`"))?;
            let id: u8 = id
                .trim()
                .parse()
                .map_err(|e| format!("bad register id {id:?}: {e}"))?;
            let value = parse_number(value)?;
            Ok((id, value))
        })
        .collect()
}

/// Parse one [`GoldenVector`] from this crate's `.golden` text format
/// (see the module-level doc comment). Fields may appear in any order;
/// `name` and `program` are required, the rest default to "no
/// expectation" if omitted.
pub fn parse_vector(text: &str) -> Result<GoldenVector, String> {
    let mut name: Option<String> = None;
    let mut program: Option<Vec<u8>> = None;
    let mut expect_halted = true;
    let mut expect_fault: Option<String> = None;
    let mut expect_registers = Vec::new();
    let mut expect_flags: Option<u64> = None;

    for (lineno, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| format!("line {}: expected `key: value`, got {raw_line:?}", lineno + 1))?;
        let value = value.trim();
        match key.trim() {
            "name" => name = Some(value.to_string()),
            "program" => program = Some(parse_program(value)?),
            "halted" => {
                expect_halted = value
                    .parse::<bool>()
                    .map_err(|e| format!("line {}: bad boolean {value:?}: {e}", lineno + 1))?;
            }
            "fault" => {
                expect_fault = if value.eq_ignore_ascii_case("none") {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "registers" => expect_registers = parse_registers(value)?,
            "flags" => {
                expect_flags = if value.eq_ignore_ascii_case("none") {
                    None
                } else {
                    Some(parse_number(value)?)
                };
            }
            other => return Err(format!("line {}: unknown field {other:?}", lineno + 1)),
        }
    }

    Ok(GoldenVector {
        name: name.ok_or("missing required `name` field")?,
        program: program.ok_or("missing required `program` field")?,
        expect_halted,
        expect_fault,
        expect_registers,
        expect_flags,
    })
}

/// Load every `.golden` vector file in `dir`, sorted by file name for a
/// deterministic order. Returns an error naming the first file that
/// fails to parse.
pub fn load_vectors(dir: impl AsRef<Path>) -> Result<Vec<GoldenVector>, String> {
    let dir = dir.as_ref();
    let mut paths: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("failed to read vector directory {}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("golden"))
        .collect();
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            parse_vector(&text).map_err(|e| format!("{}: {e}", path.display()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_text() -> &'static str {
        "name: sample\nprogram: 02 01 0A 00 00 00 00 00 00 00 60\nhalted: true\nfault: none\nregisters: 1=10\nflags: none\n"
    }

    #[test]
    fn parse_vector_reads_every_field() {
        let v = parse_vector(sample_text()).unwrap();
        assert_eq!(v.name, "sample");
        assert_eq!(v.program, vec![0x02, 0x01, 10, 0, 0, 0, 0, 0, 0, 0, 0x60]);
        assert!(v.expect_halted);
        assert_eq!(v.expect_fault, None);
        assert_eq!(v.expect_registers, vec![(1, 10)]);
        assert_eq!(v.expect_flags, None);
    }

    #[test]
    fn parse_vector_rejects_missing_name() {
        let err = parse_vector("program: 60\n").unwrap_err();
        assert!(err.contains("name"));
    }

    #[test]
    fn parse_vector_rejects_unknown_field() {
        let err = parse_vector("name: x\nprogram: 60\nbogus: 1\n").unwrap_err();
        assert!(err.contains("bogus"));
    }

    #[test]
    fn run_vector_passes_for_a_correct_vector() {
        let v = parse_vector(sample_text()).unwrap();
        assert_eq!(run_vector(&v), Ok(()));
    }

    #[test]
    fn run_vector_reports_a_register_mismatch() {
        let mut v = parse_vector(sample_text()).unwrap();
        v.expect_registers = vec![(1, 999)];
        let failures = run_vector(&v).unwrap_err();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].0.contains("R1 mismatch"));
    }

    #[test]
    fn run_vector_reports_multiple_mismatches_at_once() {
        let mut v = parse_vector(sample_text()).unwrap();
        v.expect_halted = false;
        v.expect_registers = vec![(1, 999)];
        let failures = run_vector(&v).unwrap_err();
        assert_eq!(failures.len(), 2);
    }

    #[test]
    fn load_vectors_reads_the_checked_in_fixtures_in_order() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
        let vectors = load_vectors(&dir).expect("fixtures must parse");
        assert!(!vectors.is_empty());
        let names: Vec<_> = vectors.iter().map(|v| v.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "load_vectors must return a deterministic order");
    }

    #[test]
    fn every_checked_in_fixture_passes_its_own_expectations() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
        let vectors = load_vectors(&dir).expect("fixtures must parse");
        for v in &vectors {
            if let Err(failures) = run_vector(v) {
                panic!(
                    "vector {:?} failed:\n{}",
                    v.name,
                    failures
                        .iter()
                        .map(|f| f.0.clone())
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
        }
    }
}
