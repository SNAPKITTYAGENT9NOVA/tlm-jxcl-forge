# jxcl-conformance

The mechanical spec-vs-code conformance suite: checks docs/ISA_SPEC.md and docs/RTL_CONTRACT.md's stated facts against jxcl-isa-schema and jxcl-hardware's generated RTL.

## Architecture

**Owns:** The final mechanical verification layer tying software ISA and hardware RTL to their documentation.

**Category:** security · **Source:** new

## Public API

`(test-only crate)`

## Dependencies

Workspace crates:

- `jxcl-isa-schema`
- `jxcl-hardware`
- `jxcl-golden`

External crates:

*(none)*

## Testing

Planned test kinds: conformance.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
