# jxcl-rpc

A minimal RPC server/client implementing jxcl-protocol over line-delimited JSON on TCP.

## Architecture

**Owns:** RpcServer/RpcClient.

**Category:** network · **Source:** new

## Public API

`RpcServer`, `RpcClient`

## Dependencies

Workspace crates:

- `jxcl-protocol`
- `jxcl-network`

External crates:

- `tokio`

## Testing

Planned test kinds: unit, integration.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.
