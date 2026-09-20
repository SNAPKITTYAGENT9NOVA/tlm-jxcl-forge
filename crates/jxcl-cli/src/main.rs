// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The `jxcl` command-line tool (spec §28): `asm`, `disasm`, `run`,
//! `inspect`, `validate`.
//!
//! No argument-parsing framework is used (spec §40: "Do not introduce
//! unnecessary frameworks") — the surface is five subcommands with a
//! handful of flags each, which a ~100-line hand-rolled parser covers
//! completely and deterministically.
//!
//! ## Exit codes
//! - `0` — success.
//! - `1` — usage error (unknown subcommand, missing/malformed argument).
//! - `2` — I/O error reading or writing a file.
//! - `3` — assembler, decode, or validation error in the input.
//! - `4` — the program ran to a fault rather than `HALT` (only for `run`).

use std::io::Write as _;
use std::process::ExitCode;

use jxcl_assembler::assemble_to_binary;
use jxcl_binary::BinaryContainer;
use jxcl_constants::DEFAULT_EXECUTION_LIMIT;
use jxcl_decoding::decode_all;
use jxcl_disassembler::disassemble;
use jxcl_simulator::Simulator;

fn usage() -> &'static str {
    "\
jxcl — TLM JXCL ISA forge CLI

USAGE:
    jxcl asm <input.jxcl> -o <output.jxc>
    jxcl disasm <program.jxc>
    jxcl run <program.jxc> [--limit N]
    jxcl inspect <program.jxc>
    jxcl validate <program.jxc>
"
}

fn read_file(path: &str) -> Result<Vec<u8>, ExitCode> {
    std::fs::read(path).map_err(|e| {
        eprintln!("error: could not read {:?}: {}", path, e);
        ExitCode::from(2)
    })
}

fn write_file(path: &str, bytes: &[u8]) -> Result<(), ExitCode> {
    std::fs::write(path, bytes).map_err(|e| {
        eprintln!("error: could not write {:?}: {}", path, e);
        ExitCode::from(2)
    })
}

fn cmd_asm(args: &[String]) -> ExitCode {
    let mut input = None;
    let mut output = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                match args.get(i) {
                    Some(v) => output = Some(v.clone()),
                    None => {
                        eprintln!("error: -o requires a path");
                        return ExitCode::from(1);
                    }
                }
            }
            other => input = Some(other.to_string()),
        }
        i += 1;
    }
    let (input, output) = match (input, output) {
        (Some(i), Some(o)) => (i, o),
        _ => {
            eprintln!("usage: jxcl asm <input.jxcl> -o <output.jxc>");
            return ExitCode::from(1);
        }
    };

    let source = match std::fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read {:?}: {}", input, e);
            return ExitCode::from(2);
        }
    };
    let container = match assemble_to_binary(&source) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(3);
        }
    };
    let binary = container.to_bytes();
    if let Err(code) = write_file(&output, &binary) {
        return code;
    }
    ExitCode::SUCCESS
}

fn cmd_disasm(args: &[String]) -> ExitCode {
    let path = match args.first() {
        Some(p) => p,
        None => {
            eprintln!("usage: jxcl disasm <program.jxc>");
            return ExitCode::from(1);
        }
    };
    let bytes = match read_file(path) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let container = match BinaryContainer::from_bytes(&bytes) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(3);
        }
    };
    match disassemble(&container.code, &container.data, container.entry_point) {
        Ok(text) => {
            print!("{}", text);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(3)
        }
    }
}

fn cmd_validate(args: &[String]) -> ExitCode {
    let path = match args.first() {
        Some(p) => p,
        None => {
            eprintln!("usage: jxcl validate <program.jxc>");
            return ExitCode::from(1);
        }
    };
    let bytes = match read_file(path) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let container = match BinaryContainer::from_bytes(&bytes) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(3);
        }
    };
    match decode_all(&container.code) {
        Ok(instrs) => {
            println!("OK: {} instruction(s) validated", instrs.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(3)
        }
    }
}

fn cmd_inspect(args: &[String]) -> ExitCode {
    let path = match args.first() {
        Some(p) => p,
        None => {
            eprintln!("usage: jxcl inspect <program.jxc>");
            return ExitCode::from(1);
        }
    };
    let bytes = match read_file(path) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let container = match BinaryContainer::from_bytes(&bytes) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(3);
        }
    };
    println!("entry_point:  {:#x}", container.entry_point);
    println!("code_size:    {} bytes", container.code.len());
    println!("data_size:    {} bytes", container.data.len());
    match decode_all(&container.code) {
        Ok(instrs) => println!("instructions: {}", instrs.len()),
        Err(e) => println!("instructions: <decode error: {}>", e),
    }
    ExitCode::SUCCESS
}

fn cmd_run(args: &[String]) -> ExitCode {
    let mut path = None;
    let mut limit = DEFAULT_EXECUTION_LIMIT;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--limit" => {
                i += 1;
                match args.get(i).and_then(|s| s.parse::<u64>().ok()) {
                    Some(n) => limit = n,
                    None => {
                        eprintln!("error: --limit requires a number");
                        return ExitCode::from(1);
                    }
                }
            }
            other => path = Some(other.to_string()),
        }
        i += 1;
    }
    let path = match path {
        Some(p) => p,
        None => {
            eprintln!("usage: jxcl run <program.jxc> [--limit N]");
            return ExitCode::from(1);
        }
    };

    let bytes = match read_file(&path) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let container = match BinaryContainer::from_bytes(&bytes) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(3);
        }
    };

    // Validate the program before running
    if let Err(e) = decode_all(&container.code) {
        eprintln!("error: {}", e);
        return ExitCode::from(3);
    }

    // Merge code and data sections with headroom
    let mut program_bytes = container.code.clone();
    program_bytes.extend_from_slice(&container.data);

    let mut simulator = Simulator::new(program_bytes);

    let result = simulator.run_with_limit(limit);

    if let Some(fault) = &result.fault {
        eprintln!("fault: {}", fault);
        return ExitCode::from(4);
    }
    if !result.halted {
        eprintln!(
            "error: execution limit of {} steps reached without halting",
            limit
        );
        return ExitCode::from(4);
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "halted after {} cycle(s)", result.cycles);
    for (i, v) in result.registers.iter().enumerate() {
        if *v != 0 {
            let _ = writeln!(out, "  R{} = {:#x}", i, v);
        }
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        eprint!("{}", usage());
        return ExitCode::from(1);
    };
    let rest = &args[1..];
    match command.as_str() {
        "asm" => cmd_asm(rest),
        "disasm" => cmd_disasm(rest),
        "run" => cmd_run(rest),
        "inspect" => cmd_inspect(rest),
        "validate" => cmd_validate(rest),
        "-h" | "--help" | "help" => {
            print!("{}", usage());
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("error: unknown subcommand {:?}\n", other);
            eprint!("{}", usage());
            ExitCode::from(1)
        }
    }
}
