# pq-error-proof

The concrete Groth16/arkworks proof-of-commitment-opening implementation, registered into pq-proof-registry as a ProofScheme (existing public API preserved).

## Architecture

**Owns:** The Groth16 circuit and its Params/Attestation types.

**Category:** proof · **Source:** extraction:pq-error-proof/src/lib.rs

## Public API

`Params`, `Attestation`, `attest`, `verify`

## Dependencies

Workspace crates:

- `pq-proof-types`

External crates:

- `ark-bls12-381`
- `ark-ed-on-bls12-381`
- `ark-groth16`
- `ark-r1cs-std`
- `ark-relations`
- `ark-serialize`
- `ark-snark`
- `ark-std`
- `sha2`

## Testing

Planned test kinds: unit, tamper, cross-setup, serialization, unlinkability.

## Status

`planned` in `docs/crates.toml` -- see
`docs/CRATE_GENERATION_PLAN.md` for the implementation batch schedule.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
