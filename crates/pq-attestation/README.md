# pq-attestation

Combines sealing a value (pq-envelope) with a verifiable attestation (pq-proof-types) that it was sealed under a specific, named key version, in one call.

## Architecture

**Owns:** seal_with_attestation / open_with_attestation.

**Category:** crypto · **Source:** new

## Public API

- `seal_with_attestation()` - Seal a value with attestation context
- `open_with_attestation()` - Open and verify an attested envelope
- `AttestedEnvelope` - Envelope combined with attestation context
- `AttestationContext` - Metadata for an attestation
- `AttestationError` - Error type for attestation operations

## Usage

```rust
use pq_attestation::{seal_with_attestation, open_with_attestation, AttestationContext};
use pq_envelope::EncapsulationKey;

// Create an attestation context
let mut context = AttestationContext::new(42); // key_version
context = context.with_error_code(500).with_state_id(vec![1, 2, 3]);

// Seal with attestation
let attested = seal_with_attestation(&encapsulation_key, b"secret data", context)?;

// Open and verify
let plaintext = open_with_attestation(&decapsulation_key, &attested)?;
```

## Features

- Structured attestation context encoding with key version, error code, and state
- Integration with pq-envelope for sealed values
- Verification that attestation context matches the sealed envelope
- Serialization/deserialization of attested envelopes
- Deterministic encoding for reproducible attestations

## Dependencies

Workspace crates:

- `pq-envelope` - Sealed envelope format
- `pq-proof-types` - Attestation types

External crates:

*(none)*

## Testing

Implemented test kinds: unit (14 tests), integration.

## Status

Implemented in Batch F (independent crypto crates).
