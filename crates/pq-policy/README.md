# pq-policy

Consolidates scattered policy decisions (TLS-required-in-production, minimum seed/key length) into one crate with a real Policy trait, replacing duplicated ad hoc checks in photo-cache-service and pq-crypto.

## Architecture

**Owns:** The Policy trait and the concrete TlsRequiredInProduction/MinimumSeedLength policies.

**Category:** crypto · **Source:** new

## Public API

`Policy`, `TlsRequiredInProduction`, `MinimumSeedLength`

## Dependencies

Workspace crates:

- `jxcl-errors`

External crates:

*(none)*

## Testing

Planned test kinds: unit, boundary.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
