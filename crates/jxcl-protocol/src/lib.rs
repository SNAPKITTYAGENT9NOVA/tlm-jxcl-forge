// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! A serde-serializable request/response protocol for remote jxcl-machine control:
//! assemble, run, return trace/final state.
//!
//! # Purpose
//!
//! This crate defines the wire protocol types (`Request`, `Response`) used to control
//! a jxcl-machine remotely over a transport layer (e.g., HTTP, gRPC). Clients serialize
//! `Request`s to JSON (or another serde-compatible format), send them to a server, and
//! deserialize `Response`s to learn the execution outcome and final machine state.
//!
//! # Public API
//!
//! - [`Request`]: An enum of remote control operations: `AssembleProgram`, `RunProgram`,
//!   `GetState`.
//! - [`Response`]: An enum covering success (with final machine state) and error cases.

#![forbid(unsafe_code)]

use jxcl_simulator::RunResult;
use serde::{Deserialize, Serialize};

/// A request sent by a client to control a remote jxcl-machine.
///
/// Each variant specifies one operation: loading a program, executing it, or
/// querying the current state. The server processes the request sequentially
/// and returns a `Response`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Request {
    /// Assemble (load) a raw bytecode image into the machine's memory at address 0.
    /// The bytecode is the raw hand-encoded or assembler-produced instruction bytes.
    /// On success, the machine is ready to run; the response carries no state yet.
    AssembleProgram {
        /// Raw JXCL instruction bytes (no loader container or header).
        bytecode: Vec<u8>,
        /// Optional explicit memory size in bytes. If not provided, defaults to
        /// the bytecode size plus a fixed stack reserve (4 KiB).
        #[serde(default)]
        memory_size: Option<u64>,
    },

    /// Execute the loaded program to completion (or until an instruction limit is hit).
    /// The machine must have a program loaded via `AssembleProgram` first.
    /// Returns the final machine state (PC, registers, flags, stack pointer, etc.)
    /// and optionally a cycle-by-cycle execution trace if `enable_trace` is set.
    RunProgram {
        /// Maximum number of instructions to execute. If not specified, the default
        /// limit (currently 1 million) is used.
        #[serde(default)]
        instruction_limit: Option<u64>,
        /// If true, collect a step-by-step trace of all register/flag/memory changes
        /// and include it in the response. Currently returns `None` as trace collection
        /// is not yet implemented; this is reserved for future use.
        #[serde(default)]
        enable_trace: bool,
    },

    /// Query the current machine state without further execution.
    /// Returns the same fields as `RunProgram`, but does not execute any instructions.
    GetState,
}

/// The final architectural state of a machine after a run, serializable to JSON.
/// This is a serde-compatible wrapper around the `RunResult` type from `jxcl-simulator`,
/// inlining all its fields for serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineState {
    /// Whether the machine halted (HALT instruction executed) or is still running.
    pub halted: bool,
    /// If a fault occurred during execution, the fault type name (e.g., "DivideByZero",
    /// "InvalidOpcode"). `None` if execution completed cleanly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fault: Option<String>,
    /// Total number of instructions executed.
    pub cycles: u64,
    /// The program counter (address of the next instruction to fetch).
    pub pc: u64,
    /// The stack pointer.
    pub sp: u64,
    /// The current flags register value (zero flag, sign flag, etc.).
    pub flags: u64,
    /// The 32 general-purpose register values (R0-R31), each 64 bits.
    pub registers: [u64; 32],
}

impl MachineState {
    /// Construct from a `RunResult` by copying all its fields.
    pub fn from_run_result(result: &RunResult) -> Self {
        MachineState {
            halted: result.halted,
            fault: result.fault.clone(),
            cycles: result.cycles,
            pc: result.pc,
            sp: result.sp,
            flags: result.flags,
            registers: result.registers,
        }
    }
}

/// A response sent by the server in reply to a `Request`.
///
/// Each variant indicates success or failure. Success responses include the final
/// `MachineState` (architectural state: PC, registers, flags, memory pointer, halt/fault
/// status, cycle count). The `MachineState` is boxed to avoid large size differences
/// between enum variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum Response {
    /// The request completed successfully. Contains the final state of the machine
    /// after the requested operation (execute or state query).
    #[serde(rename = "success")]
    Success {
        /// The final architectural state: halt flag, any fault, cycle count,
        /// program counter, stack pointer, flags register, and all general-purpose
        /// register values.
        state: Box<MachineState>,
    },

    /// The request failed. The error message describes what went wrong
    /// (e.g., invalid bytecode size, memory allocation failure, invalid state transition).
    #[serde(rename = "error")]
    Error {
        /// Human-readable error description.
        message: String,
    },
}

impl Response {
    /// Construct a success response from a `RunResult`.
    pub fn success(result: &RunResult) -> Self {
        Response::Success {
            state: Box::new(MachineState::from_run_result(result)),
        }
    }

    /// Construct an error response.
    pub fn error(message: impl Into<String>) -> Self {
        Response::Error {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a register array with default values (all zeros).
    fn default_registers() -> [u64; 32] {
        [0; 32]
    }

    /// Helper to create a register array with some non-zero values.
    fn test_registers() -> [u64; 32] {
        let mut r = [0; 32];
        r[0] = 0;
        r[1] = 1;
        r[2] = 2;
        r[3] = 3;
        r[4] = 4;
        r[5] = 5;
        r[6] = 6;
        r[7] = 7;
        r
    }

    // ---- Unit tests: Request construction ----

    #[test]
    fn request_assemble_program_minimal() {
        let req = Request::AssembleProgram {
            bytecode: vec![0x60], // HALT instruction
            memory_size: None,
        };
        assert_eq!(req, req.clone());
    }

    #[test]
    fn request_assemble_program_with_memory_size() {
        let req = Request::AssembleProgram {
            bytecode: vec![0x02, 0x01, 1, 0, 0, 0, 0, 0, 0, 0], // MOVI R1, 1
            memory_size: Some(8192),
        };
        assert_eq!(
            req,
            Request::AssembleProgram {
                bytecode: vec![0x02, 0x01, 1, 0, 0, 0, 0, 0, 0, 0],
                memory_size: Some(8192),
            }
        );
    }

    #[test]
    fn request_run_program_minimal() {
        let req = Request::RunProgram {
            instruction_limit: None,
            enable_trace: false,
        };
        assert_eq!(
            req,
            Request::RunProgram {
                instruction_limit: None,
                enable_trace: false,
            }
        );
    }

    #[test]
    fn request_run_program_with_limit_and_trace() {
        let req = Request::RunProgram {
            instruction_limit: Some(100_000),
            enable_trace: true,
        };
        assert_eq!(
            req,
            Request::RunProgram {
                instruction_limit: Some(100_000),
                enable_trace: true,
            }
        );
    }

    #[test]
    fn request_get_state() {
        let req = Request::GetState;
        assert_eq!(req, Request::GetState);
    }

    // ---- Unit tests: MachineState construction ----

    #[test]
    fn machine_state_from_run_result() {
        let result = RunResult {
            halted: true,
            fault: None,
            cycles: 5,
            pc: 34,
            sp: 8192,
            flags: 0,
            registers: test_registers(),
        };
        let state = MachineState::from_run_result(&result);
        assert!(state.halted);
        assert_eq!(state.cycles, 5);
        assert_eq!(state.pc, 34);
        assert_eq!(state.registers[1], 1);
    }

    #[test]
    fn machine_state_with_fault() {
        let result = RunResult {
            halted: true,
            fault: Some("DivideByZero".to_string()),
            cycles: 1,
            pc: 0,
            sp: 4096,
            flags: 0,
            registers: default_registers(),
        };
        let state = MachineState::from_run_result(&result);
        assert_eq!(state.fault, Some("DivideByZero".to_string()));
    }

    // ---- Unit tests: Response construction ----

    #[test]
    fn response_success_from_run_result() {
        let result = RunResult {
            halted: true,
            fault: None,
            cycles: 10,
            pc: 20,
            sp: 2048,
            flags: 1,
            registers: test_registers(),
        };
        let resp = Response::success(&result);
        match resp {
            Response::Success { state } => {
                assert!(state.halted);
                assert_eq!(state.cycles, 10);
            }
            _ => panic!("expected Success variant"),
        }
    }

    #[test]
    fn response_error() {
        let resp = Response::error("memory size too small");
        match resp {
            Response::Error { message } => {
                assert_eq!(message, "memory size too small");
            }
            _ => panic!("expected Error variant"),
        }
    }

    // ---- Serialization tests: Request ----

    #[test]
    fn serialize_request_assemble_program() {
        let req = Request::AssembleProgram {
            bytecode: vec![0x60],
            memory_size: None,
        };
        let json = serde_json::to_string(&req).expect("serialization failed");
        assert!(json.contains("AssembleProgram"));
        assert!(json.contains("\"bytecode\":[96]")); // 0x60 = 96
    }

    #[test]
    fn deserialize_request_assemble_program() {
        let json = r#"{
            "type": "AssembleProgram",
            "data": {
                "bytecode": [96],
                "memory_size": null
            }
        }"#;
        let req: Request = serde_json::from_str(json).expect("deserialization failed");
        assert_eq!(
            req,
            Request::AssembleProgram {
                bytecode: vec![96],
                memory_size: None,
            }
        );
    }

    #[test]
    fn roundtrip_request_assemble_with_memory_size() {
        let original = Request::AssembleProgram {
            bytecode: vec![0x02, 0x01, 1, 0, 0, 0, 0, 0, 0, 0],
            memory_size: Some(16384),
        };
        let json = serde_json::to_string(&original).expect("serialization failed");
        let deserialized: Request = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn serialize_request_run_program() {
        let req = Request::RunProgram {
            instruction_limit: Some(50_000),
            enable_trace: true,
        };
        let json = serde_json::to_string(&req).expect("serialization failed");
        assert!(json.contains("RunProgram"));
        assert!(json.contains("50000"));
        assert!(json.contains("true"));
    }

    #[test]
    fn roundtrip_request_run_program() {
        let original = Request::RunProgram {
            instruction_limit: Some(100),
            enable_trace: false,
        };
        let json = serde_json::to_string(&original).expect("serialization failed");
        let deserialized: Request = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn serialize_request_get_state() {
        let req = Request::GetState;
        let json = serde_json::to_string(&req).expect("serialization failed");
        assert!(json.contains("GetState"));
    }

    #[test]
    fn roundtrip_request_get_state() {
        let original = Request::GetState;
        let json = serde_json::to_string(&original).expect("serialization failed");
        let deserialized: Request = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(original, deserialized);
    }

    // ---- Serialization tests: Response ----

    #[test]
    fn serialize_response_success() {
        let result = RunResult {
            halted: true,
            fault: None,
            cycles: 42,
            pc: 64,
            sp: 4096,
            flags: 0,
            registers: test_registers(),
        };
        let resp = Response::success(&result);
        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("success"));
        assert!(json.contains("42")); // cycles
        assert!(json.contains("64")); // pc
    }

    #[test]
    fn roundtrip_response_success() {
        let result = RunResult {
            halted: true,
            fault: None,
            cycles: 10,
            pc: 20,
            sp: 2048,
            flags: 1,
            registers: test_registers(),
        };
        let original = Response::success(&result);
        let json = serde_json::to_string(&original).expect("serialization failed");
        let deserialized: Response = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn serialize_response_error() {
        let resp = Response::error("instruction limit exceeded");
        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("error"));
        assert!(json.contains("instruction limit exceeded"));
    }

    #[test]
    fn roundtrip_response_error() {
        let original = Response::error("divide by zero");
        let json = serde_json::to_string(&original).expect("serialization failed");
        let deserialized: Response = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn roundtrip_response_success_with_large_register_values() {
        let result = RunResult {
            halted: false,
            fault: Some("DivideByZero".to_string()),
            cycles: 999_999,
            pc: u64::MAX - 1,
            sp: u64::MAX,
            flags: 0b11111111,
            registers: [u64::MAX; 32],
        };
        let original = Response::success(&result);
        let json = serde_json::to_string(&original).expect("serialization failed");
        let deserialized: Response = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn serialize_response_success_with_fault() {
        let result = RunResult {
            halted: true,
            fault: Some("InvalidOpcode".to_string()),
            cycles: 1,
            pc: 0,
            sp: 8192,
            flags: 0,
            registers: default_registers(),
        };
        let resp = Response::success(&result);
        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("InvalidOpcode"));
    }

    // ---- Cross-variant roundtrip tests ----

    #[test]
    fn multiple_requests_roundtrip_in_sequence() {
        let reqs = vec![
            Request::AssembleProgram {
                bytecode: vec![0x60],
                memory_size: None,
            },
            Request::RunProgram {
                instruction_limit: Some(1000),
                enable_trace: false,
            },
            Request::GetState,
        ];

        for req in reqs {
            let json = serde_json::to_string(&req).expect("serialization failed");
            let deserialized: Request =
                serde_json::from_str(&json).expect("deserialization failed");
            assert_eq!(req, deserialized);
        }
    }

    #[test]
    fn multiple_responses_roundtrip_in_sequence() {
        let result = RunResult {
            halted: true,
            fault: None,
            cycles: 5,
            pc: 10,
            sp: 4096,
            flags: 0,
            registers: default_registers(),
        };

        let resps = vec![Response::success(&result), Response::error("test error")];

        for resp in resps {
            let json = serde_json::to_string(&resp).expect("serialization failed");
            let deserialized: Response =
                serde_json::from_str(&json).expect("deserialization failed");
            assert_eq!(resp, deserialized);
        }
    }

    #[test]
    fn machine_state_serialization_roundtrip() {
        let mut regs = default_registers();
        regs[0] = 10;
        regs[1] = 20;
        regs[2] = 30;
        regs[3] = 40;
        regs[4] = 50;
        regs[5] = 60;
        regs[6] = 70;
        regs[7] = 80;

        let original = MachineState {
            halted: true,
            fault: Some("TestFault".to_string()),
            cycles: 100,
            pc: 512,
            sp: 8192,
            flags: 0x0F,
            registers: regs,
        };
        let json = serde_json::to_string(&original).expect("serialization failed");
        let deserialized: MachineState =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(original, deserialized);
    }
}
