# jxcl-fuzz

A structured fuzz harness for the decoder (and other parser/decoder boundaries) against malformed input.

## Architecture

**Owns:** The fuzz target(s).

**Category:** debug · **Source:** extraction:jxcl/tests/fuzz_decoder.rs

## Public API

`fuzz_decode_target`

## Dependencies

Workspace crates:

- `jxcl-decoding`
- `jxcl-instructions`

External crates:

*(none)*

## Testing

Planned test kinds: fuzz.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
