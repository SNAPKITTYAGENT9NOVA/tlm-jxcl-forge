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

use jxcl::isa::constants::DEFAULT_EXECUTION_LIMIT;

fn usage() -> &'static str {
    "\
jxcl — TLM JXCL ISA forge CLI

USAGE:
    jxcl asm <input.jxcl> -o <output.jxc>
    jxcl disasm <program.jxc>
    jxcl run <program.jxc> [--trace] [--limit N]
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
    let binary = match jxcl::assembler::assemble(&source) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(3);
        }
    };
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
    let program = match jxcl::binary::parse(&bytes) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(3);
        }
    };
    match jxcl::disassembler::disassemble(program.code, program.data, program.header.entry_point) {
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
    match jxcl::validator::validate(&bytes) {
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
    let program = match jxcl::binary::parse(&bytes) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(3);
        }
    };
    println!("version:      {}", program.header.version);
    println!("architecture: {:#06x}", program.header.architecture);
    println!("entry_point:  {:#x}", program.header.entry_point);
    println!("code_offset:  {:#x}", program.header.code_offset);
    println!("code_size:    {} bytes", program.header.code_size);
    println!("data_offset:  {:#x}", program.header.data_offset);
    println!("data_size:    {} bytes", program.header.data_size);
    match jxcl::encoding::decoder::decode_all(program.code) {
        Ok(instrs) => println!("instructions: {}", instrs.len()),
        Err(e) => println!("instructions: <decode error: {}>", e),
    }
    ExitCode::SUCCESS
}

fn cmd_run(args: &[String]) -> ExitCode {
    let mut path = None;
    let mut trace = false;
    let mut limit = DEFAULT_EXECUTION_LIMIT;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--trace" => trace = true,
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
            eprintln!("usage: jxcl run <program.jxc> [--trace] [--limit N]");
            return ExitCode::from(1);
        }
    };

    let bytes = match read_file(&path) {
        Ok(b) => b,
        Err(code) => return code,
    };
    if let Err(e) = jxcl::validator::validate(&bytes) {
        eprintln!("error: {}", e);
        return ExitCode::from(3);
    }
    let program = jxcl::binary::parse(&bytes).expect("already validated above");

    // Total machine memory: code + data, plus headroom for the stack.
    let total_mem = program.header.code_size + program.header.data_size + (1 << 20);
    let mem = match jxcl::memory::Memory::with_code_region(total_mem, 0, program.header.code_size) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: could not construct machine memory: {}", e);
            return ExitCode::from(3);
        }
    };
    let mut state = jxcl::machine::MachineState::new(mem);
    if let Err(e) = state.memory.loader_write(0, program.code) {
        eprintln!("error: could not load code: {}", e);
        return ExitCode::from(3);
    }
    if let Err(e) = state
        .memory
        .loader_write(program.header.code_size, program.data)
    {
        eprintln!("error: could not load data: {}", e);
        return ExitCode::from(3);
    }
    state.registers.pc = program.header.entry_point;

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let mut steps = 0u64;
    loop {
        if steps >= limit {
            eprintln!(
                "error: execution limit of {} steps reached without halting",
                limit
            );
            return ExitCode::from(4);
        }
        if trace {
            let entry = jxcl::debugger::trace_step(&mut state);
            let _ = write!(out, "{}", entry);
            steps += 1;
            match entry.result {
                jxcl::execution::StepResult::Continued
                | jxcl::execution::StepResult::Signaled(_) => continue,
                jxcl::execution::StepResult::Halted => break,
                jxcl::execution::StepResult::Faulted(_) => return ExitCode::from(4),
            }
        } else {
            steps += 1;
            match jxcl::execution::step(&mut state) {
                jxcl::execution::StepResult::Continued
                | jxcl::execution::StepResult::Signaled(_) => continue,
                jxcl::execution::StepResult::Halted => break,
                jxcl::execution::StepResult::Faulted(f) => {
                    eprintln!("fault: {}", f);
                    return ExitCode::from(4);
                }
            }
        }
    }

    let _ = writeln!(out, "halted after {} cycle(s)", state.cycle_count);
    for (i, v) in state.registers.general_registers().iter().enumerate() {
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
