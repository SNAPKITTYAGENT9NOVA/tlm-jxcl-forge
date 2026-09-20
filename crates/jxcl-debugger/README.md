# jxcl-debugger

Interactive/trace-mode debugger: step, breakpoint, and inspect a running machine.

## Architecture

**Owns:** The Debugger driver.

**Category:** debug · **Source:** extraction:jxcl/src/debugger.rs

## Public API

`Debugger`

## Dependencies

Workspace crates:

- `jxcl-machine`
- `jxcl-execution`
- `jxcl-trace`

External crates:

*(none)*

## Testing

8 unit tests covering stepping, history recording, breakpoints, the
run loop's three outcomes, and trace-step rendering, using a small
in-crate mock `Steppable` machine.

## Implementation notes

`jxcl-machine` and `jxcl-execution` are still scaffolded placeholders
as of this batch, with no `MachineState`/`step` API yet to drive.
`Debugger` is therefore generic over a `Steppable` trait defined in
this crate (execute one instruction, report the resulting
`jxcl-trace::TraceStep`), rather than depending on either crate's
type directly. It uses `jxcl-trace`'s real `TraceStep`/`TraceLog` as
its trace-record and history types, per this batch's instruction to
build on `jxcl-trace` rather than duplicate `jxcl::debugger::TraceEntry`.
See `src/lib.rs`'s module doc comment for how a real `MachineState`
plugs in once `jxcl-machine`/`jxcl-execution` land.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
This crate's own implementation (above) is complete for this batch;
`docs/crates.toml`'s `status` field is out of this batch's scope to
edit.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
