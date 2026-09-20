// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Integration tests for the jxcl CLI: verifying all subcommands (asm, disasm,
//! run, inspect, validate) work end-to-end with realistic programs.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to get the path to the jxcl binary.
fn jxcl_bin() -> PathBuf {
    // In integration tests, the binary is in target/debug/ or target/release/
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let target_dir = PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join(profile);
    target_dir.join("jxcl")
}

/// A simple test program that adds two numbers and halts.
fn test_program_add() -> String {
    r#"
    MOVI R1, 10
    MOVI R2, 20
    ADD  R1, R2
    HALT
"#
    .to_string()
}

#[test]
fn test_asm_basic() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("test.jxcl");
    let output = temp.path().join("test.jxc");

    fs::write(&input, test_program_add()).unwrap();

    let output_asm = Command::new(jxcl_bin())
        .args([
            "asm",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run jxcl asm");

    assert!(
        output_asm.status.success(),
        "asm failed: {}",
        String::from_utf8_lossy(&output_asm.stderr)
    );
    assert!(output.exists(), "output file not created");
    let binary = fs::read(&output).unwrap();
    assert!(!binary.is_empty(), "output binary is empty");
}

#[test]
fn test_asm_missing_output() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("test.jxcl");
    fs::write(&input, test_program_add()).unwrap();

    let output_asm = Command::new(jxcl_bin())
        .args(["asm", input.to_str().unwrap()])
        .output()
        .expect("failed to run jxcl asm");

    assert!(
        !output_asm.status.success(),
        "asm should fail with missing output"
    );
    assert!(String::from_utf8_lossy(&output_asm.stderr).contains("usage:"));
}

#[test]
fn test_disasm_basic() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("test.jxcl");
    let output = temp.path().join("test.jxc");

    fs::write(&input, test_program_add()).unwrap();

    // First assemble
    let asm_status = Command::new(jxcl_bin())
        .args([
            "asm",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run jxcl asm")
        .status;

    assert!(asm_status.success(), "asm failed");

    // Then disassemble
    let output_disasm = Command::new(jxcl_bin())
        .args(["disasm", output.to_str().unwrap()])
        .output()
        .expect("failed to run jxcl disasm");

    assert!(
        output_disasm.status.success(),
        "disasm failed: {}",
        String::from_utf8_lossy(&output_disasm.stderr)
    );
    let output_text = String::from_utf8_lossy(&output_disasm.stdout);
    assert!(!output_text.is_empty(), "disasm output is empty");
    // Check that disassembly contains some instruction mnemonic (uppercase or lowercase)
    let lower_output = output_text.to_lowercase();
    assert!(
        lower_output.contains("movi")
            || lower_output.contains("add")
            || lower_output.contains("halt"),
        "disasm output doesn't contain expected mnemonics: {}",
        output_text
    );
}

#[test]
fn test_disasm_missing_file() {
    let output_disasm = Command::new(jxcl_bin())
        .args(["disasm", "/nonexistent/file.jxc"])
        .output()
        .expect("failed to run jxcl disasm");

    assert!(
        !output_disasm.status.success(),
        "disasm should fail with missing file"
    );
    assert!(String::from_utf8_lossy(&output_disasm.stderr).contains("could not read"));
}

#[test]
fn test_validate_basic() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("test.jxcl");
    let output = temp.path().join("test.jxc");

    fs::write(&input, test_program_add()).unwrap();

    // First assemble
    let asm_status = Command::new(jxcl_bin())
        .args([
            "asm",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run jxcl asm")
        .status;

    assert!(asm_status.success(), "asm failed");

    // Then validate
    let output_validate = Command::new(jxcl_bin())
        .args(["validate", output.to_str().unwrap()])
        .output()
        .expect("failed to run jxcl validate");

    assert!(
        output_validate.status.success(),
        "validate failed: {}",
        String::from_utf8_lossy(&output_validate.stderr)
    );
    let output_text = String::from_utf8_lossy(&output_validate.stdout);
    assert!(output_text.contains("OK:") && output_text.contains("instruction(s) validated"));
}

#[test]
fn test_validate_missing_file() {
    let output_validate = Command::new(jxcl_bin())
        .args(["validate", "/nonexistent/file.jxc"])
        .output()
        .expect("failed to run jxcl validate");

    assert!(
        !output_validate.status.success(),
        "validate should fail with missing file"
    );
}

#[test]
fn test_inspect_basic() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("test.jxcl");
    let output = temp.path().join("test.jxc");

    fs::write(&input, test_program_add()).unwrap();

    // First assemble
    let asm_status = Command::new(jxcl_bin())
        .args([
            "asm",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run jxcl asm")
        .status;

    assert!(asm_status.success(), "asm failed");

    // Then inspect
    let output_inspect = Command::new(jxcl_bin())
        .args(["inspect", output.to_str().unwrap()])
        .output()
        .expect("failed to run jxcl inspect");

    assert!(
        output_inspect.status.success(),
        "inspect failed: {}",
        String::from_utf8_lossy(&output_inspect.stderr)
    );
    let output_text = String::from_utf8_lossy(&output_inspect.stdout);
    assert!(output_text.contains("entry_point:"));
    assert!(output_text.contains("code_size:"));
    assert!(output_text.contains("data_size:"));
    assert!(output_text.contains("instructions:"));
}

#[test]
fn test_inspect_missing_file() {
    let output_inspect = Command::new(jxcl_bin())
        .args(["inspect", "/nonexistent/file.jxc"])
        .output()
        .expect("failed to run jxcl inspect");

    assert!(
        !output_inspect.status.success(),
        "inspect should fail with missing file"
    );
}

#[test]
fn test_run_basic() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("test.jxcl");
    let output = temp.path().join("test.jxc");

    fs::write(&input, test_program_add()).unwrap();

    // First assemble
    let asm_status = Command::new(jxcl_bin())
        .args([
            "asm",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run jxcl asm")
        .status;

    assert!(asm_status.success(), "asm failed");

    // Then run
    let output_run = Command::new(jxcl_bin())
        .args(["run", output.to_str().unwrap()])
        .output()
        .expect("failed to run jxcl run");

    assert!(
        output_run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output_run.stderr)
    );
    let output_text = String::from_utf8_lossy(&output_run.stdout);
    assert!(output_text.contains("halted after"));
}

#[test]
fn test_run_with_limit() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("test.jxcl");
    let output = temp.path().join("test.jxc");

    fs::write(&input, test_program_add()).unwrap();

    // First assemble
    let asm_status = Command::new(jxcl_bin())
        .args([
            "asm",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run jxcl asm")
        .status;

    assert!(asm_status.success(), "asm failed");

    // Run with a specific limit
    let output_run = Command::new(jxcl_bin())
        .args(["run", output.to_str().unwrap(), "--limit", "10000"])
        .output()
        .expect("failed to run jxcl run");

    assert!(
        output_run.status.success(),
        "run with limit failed: {}",
        String::from_utf8_lossy(&output_run.stderr)
    );
    let output_text = String::from_utf8_lossy(&output_run.stdout);
    assert!(output_text.contains("halted after"));
}

#[test]
fn test_run_missing_file() {
    let output_run = Command::new(jxcl_bin())
        .args(["run", "/nonexistent/file.jxc"])
        .output()
        .expect("failed to run jxcl run");

    assert!(
        !output_run.status.success(),
        "run should fail with missing file"
    );
}

#[test]
fn test_run_invalid_limit() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("test.jxcl");
    let output = temp.path().join("test.jxc");

    fs::write(&input, test_program_add()).unwrap();

    // First assemble
    let asm_status = Command::new(jxcl_bin())
        .args([
            "asm",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run jxcl asm")
        .status;

    assert!(asm_status.success(), "asm failed");

    // Run with an invalid limit
    let output_run = Command::new(jxcl_bin())
        .args(["run", output.to_str().unwrap(), "--limit", "not_a_number"])
        .output()
        .expect("failed to run jxcl run");

    assert!(
        !output_run.status.success(),
        "run should fail with invalid limit"
    );
}

#[test]
fn test_help() {
    let output = Command::new(jxcl_bin())
        .args(["--help"])
        .output()
        .expect("failed to run jxcl --help");

    assert!(output.status.success(), "help should succeed");
    let output_text = String::from_utf8_lossy(&output.stdout);
    assert!(output_text.contains("USAGE:"));
    assert!(output_text.contains("jxcl asm"));
    assert!(output_text.contains("jxcl disasm"));
    assert!(output_text.contains("jxcl run"));
    assert!(output_text.contains("jxcl inspect"));
    assert!(output_text.contains("jxcl validate"));
}

#[test]
fn test_unknown_subcommand() {
    let output = Command::new(jxcl_bin())
        .args(["unknown"])
        .output()
        .expect("failed to run jxcl unknown");

    assert!(!output.status.success(), "unknown subcommand should fail");
    let output_text = String::from_utf8_lossy(&output.stderr);
    assert!(output_text.contains("unknown subcommand"));
}
