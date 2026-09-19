# jxcl-simulator

An embeddable top-level simulation API (load + run in one call) for using jxcl as a library from other Rust programs/services.

## Architecture

**Owns:** Simulator::new/run.

**Category:** debug · **Source:** new

## Public API

`Simulator`, `RunResult`

## Dependencies

Workspace crates:

- `jxcl-loader`
- `jxcl-machine`
- `jxcl-execution`
- `jxcl-memory-map`

External crates:

*(none)*

## Testing

19 unit + integration tests, including an end-to-end integration test
(`loop_sums_one_to_five_end_to_end`) that hand-encodes a small
countdown-loop program to raw bytes and runs it to completion through
`Simulator::new`/`run`.

## Implementation notes

`jxcl-loader`, `jxcl-machine`, `jxcl-execution` and `jxcl-memory-map`
are still scaffolded placeholders as of this batch, with no public API
to compose yet. `Simulator` therefore embeds a real fetch/decode/
execute loop of its own (`src/isa.rs`, `src/decode.rs`, `src/alu.rs`,
`src/state.rs`, `src/exec.rs`), ported from `crates/jxcl`'s real
opcode table, decoder, ALU and execution engine for a genuine (if
partial -- 30 of the full registry's 47 mnemonics) instruction subset.
See the module-level doc comment in `src/lib.rs` for exactly which
mnemonics are covered and what changes once the four real dependency
crates land. The workspace path dependencies on those four crates stay
declared in `Cargo.toml` (matching `docs/crates.toml`) but are not yet
imported by this crate's code.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
This crate's own implementation (above) is complete for this batch;
`docs/crates.toml`'s `status` field is out of this batch's scope to
edit.
