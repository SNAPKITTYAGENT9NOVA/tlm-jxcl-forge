# Hardware/RTL Subsystem: What Is and Isn't Verified

The 8 Hardware/RTL crates (`jxcl-hdl`, `jxcl-rtl`, `jxcl-verilog`,
`jxcl-vhdl`, `jxcl-netlist`, `jxcl-synthesis`, `jxcl-hardware`,
`jxcl-hardware-test`) are real Rust code with real tests. This document
states plainly what "real" does and doesn't mean here, so nobody
downstream mistakes the level of verification for more than it is.

## What is available in this environment

Checked via `which iverilog verilator yosys` at baseline (see
`docs/BASELINE.md`): **none of the three are installed.** There is no
Verilog/VHDL simulator and no logic synthesizer available to this
session.

## What this subsystem actually does

- `jxcl-hdl` defines a real, backend-agnostic hardware-description AST
  (modules, ports, signals, always-blocks, case statements) in Rust.
- `jxcl-rtl` builds that AST for jxcl's core datapath (opcode decoder,
  ALU, register file) **generated directly from `jxcl-opcodes`,
  `jxcl-constants`, and `jxcl-alu`'s real tables/enums** -- not
  hand-copied numbers. A test asserts that every opcode in
  `jxcl-opcodes::OPCODE_TABLE` produces exactly one case arm in the
  generated decoder module, and that register/opcode field widths in
  the generated module match `jxcl-constants`. This is what makes the
  RTL "mechanically tied to the ISA source of truth" per
  `docs/RTL_CONTRACT.md`, without needing a simulator: it's a structural
  fact about the generated AST, checked in Rust.
- `jxcl-verilog` and `jxcl-vhdl` render that AST to syntactically valid
  Verilog-2001 / VHDL text. Tests check the rendered text against
  checked-in golden files (`jxcl-hardware-test`) and re-parse basic
  structural properties (module/port declarations present, case arms
  present for every opcode) -- not full HDL grammar parsing, and
  certainly not simulation.
- `jxcl-synthesis` lowers a subset of the AST (combinational
  case-statement logic) into a `jxcl-netlist` structural
  gate/wire representation, documented explicitly as a toy/educational
  pass, not a production synthesis tool.

## What this subsystem does **not** do

- It does **not** simulate the generated RTL. No test claims that the
  generated Verilog, if fed to a real simulator, would produce outputs
  matching `jxcl-alu`'s Rust reference implementation for arbitrary
  inputs. That would require `iverilog`/`verilator`, which aren't
  available here.
- It does **not** synthesize to a real gate library or verify timing,
  area, or power. `jxcl-synthesis`'s output is a structural
  demonstration, not something you'd tape out.
- It does **not** claim the generated Verilog/VHDL is free of syntax
  errors a real toolchain would catch -- only that it round-trips
  through this crate's own text-structure checks and matches its golden
  files.

## If a real simulator/synthesizer becomes available

`jxcl-hardware-test` is the place to add real simulation-based
conformance: feed the generated Verilog to `iverilog`/`verilator`
alongside `jxcl-golden`'s existing vectors, and assert the two traces
match cycle-for-cycle. Until then, treat this subsystem's guarantees as
"structurally consistent with the ISA source of truth," not
"hardware-verified."
