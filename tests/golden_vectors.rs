//! Golden architectural test vectors (spec §38): fixed program bytes with
//! an expected final machine state (registers, flags, PC, halt/fault).
//! These are exactly the vectors a future RTL simulator would replay
//! against real hardware to confirm it implements the same architecture
//! (spec §36-§38's reference/RTL contract).

use jxcl::binary;
use jxcl::execution::{run_until_halt, StepResult};
use jxcl::isa::constants::DEFAULT_EXECUTION_LIMIT;
use jxcl::machine::MachineState;
use jxcl::memory::Memory;

fn assemble_and_load(src: &str) -> MachineState {
    let bin = assemble_checked(src);
    let program = binary::parse(&bin).unwrap();
    jxcl::validator::validate(&bin).expect("golden vector program must validate");
    let mem = Memory::with_code_region(1 << 16, 0, program.header.code_size).unwrap();
    let mut state = MachineState::new(mem);
    state.memory.loader_write(0, program.code).unwrap();
    state.memory.loader_write(program.header.code_size, program.data).unwrap();
    state.registers.pc = program.header.entry_point;
    state
}

fn assemble_checked(src: &str) -> Vec<u8> {
    jxcl::assembler::assemble(src).unwrap_or_else(|e| panic!("assembly failed: {}\nsource:\n{}", e, src))
}

/// A golden vector: assemble+run `source` and check the machine's final
/// architectural state against every expectation given.
struct Golden {
    name: &'static str,
    source: &'static str,
    expect_halted: bool,
    expect_fault: Option<&'static str>, // Debug-formatted ExecutionFault, or None
    expect_registers: &'static [(u8, u64)],
    expect_flags: Option<u64>,
}

const VECTORS: &[Golden] = &[
    Golden {
        name: "add_two_constants",
        source: "MOVI R1, 10\nMOVI R2, 20\nADD R1, R2\nHALT\n",
        expect_halted: true,
        expect_fault: None,
        expect_registers: &[(1, 30), (2, 20)],
        expect_flags: None,
    },
    Golden {
        name: "add_overflow_wraps_and_sets_zc",
        source: "MOVI R1, 18446744073709551615\nMOVI R2, 1\nADD R1, R2\nHALT\n",
        expect_halted: true,
        expect_fault: None,
        expect_registers: &[(1, 0)],
        // Z=1 (bit0), N=0, C=1 (bit2), V=0 -> 0b0101 = 5
        expect_flags: Some(0b0101),
    },
    Golden {
        name: "sum_one_to_five_via_loop",
        source: "\
    MOVI R1, 0
    MOVI R2, 5
loop:
    ADD  R1, R2
    DEC  R2
    CMP  R2, R0
    JG   loop
    HALT
",
        expect_halted: true,
        expect_fault: None,
        expect_registers: &[(1, 15), (2, 0)],
        expect_flags: None,
    },
    Golden {
        name: "stack_push_pop_roundtrip",
        source: "MOVI R1, 0xABCD\nPUSH R1\nMOVI R1, 0\nPOP R1\nHALT\n",
        expect_halted: true,
        expect_fault: None,
        expect_registers: &[(1, 0xABCD)],
        expect_flags: None,
    },
    Golden {
        name: "divide_by_zero_faults",
        source: "MOVI R1, 5\nMOVI R2, 0\nDIV R1, R2\nHALT\n",
        // A fault stops the machine just as HALT does (`state.halted`
        // is a generic "execution has stopped" flag) — `expect_fault`
        // below is what actually distinguishes this from a clean halt.
        expect_halted: true,
        expect_fault: Some("DivideByZero"),
        expect_registers: &[(1, 5)], // R1 unmodified: the fault preempts the writeback
        expect_flags: None,
    },
    Golden {
        name: "cas_success_then_failure",
        source: "\
    MOVI R5, 4096
    MOVI R1, 100
    STORE [R5], R1
    MOVI R2, 100
    MOVI R3, 200
    CAS  R2, [R5], R3
    MOVI R4, 999
    CAS  R4, [R5], R3
    HALT
",
        expect_halted: true,
        expect_fault: None,
        // First CAS: expected(100)==mem(100) -> success, mem becomes 200, R2 stays 100.
        // Second CAS: expected(999)!=mem(200) -> failure, R4 becomes 200 (actual mem value).
        expect_registers: &[(2, 100), (4, 200)],
        expect_flags: None,
    },
];

#[test]
fn golden_vectors_match_expected_final_state() {
    for v in VECTORS {
        let mut state = assemble_and_load(v.source);
        let result = run_until_halt(&mut state, DEFAULT_EXECUTION_LIMIT);

        assert_eq!(state.halted, v.expect_halted, "[{}] halted mismatch", v.name);
        match (result, v.expect_fault) {
            (StepResult::Faulted(f), Some(expected)) => {
                assert_eq!(format!("{:?}", f), expected, "[{}] fault mismatch", v.name);
            }
            (StepResult::Faulted(f), None) => panic!("[{}] unexpected fault {:?}", v.name, f),
            (_, Some(expected)) => panic!("[{}] expected fault {} but got {:?}", v.name, expected, result),
            (_, None) => {}
        }
        for (reg, expected) in v.expect_registers {
            let actual = state.registers.read(*reg).unwrap();
            assert_eq!(actual, *expected, "[{}] R{} mismatch: expected {:#x}, got {:#x}", v.name, reg, expected, actual);
        }
        if let Some(expected_flags) = v.expect_flags {
            assert_eq!(state.registers.flags, expected_flags, "[{}] flags mismatch", v.name);
        }
    }
}
