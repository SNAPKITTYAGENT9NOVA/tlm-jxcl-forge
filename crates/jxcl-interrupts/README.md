# jxcl-interrupts

A deterministic external-interrupt injection mechanism (priority queue + mask flag) for embedding jxcl in a simulator or RPC host.

## Architecture

**Owns:** InterruptController: queue/mask/deliver.

**Category:** execution · **Source:** new

## Public API

`InterruptController`

## Dependencies

Workspace crates:

- `jxcl-flags`
- `jxcl-exceptions`

External crates:

*(none)*

## Testing

Planned test kinds: unit, determinism.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
