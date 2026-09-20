# pq-signature

ML-DSA (NIST FIPS 204 / Dilithium) signing and verification -- a second, complementary post-quantum primitive (authenticity, alongside pq-kem's confidentiality).

## Architecture

**Owns:** SigningKey/VerifyingKey and sign/verify.

**Category:** crypto · **Source:** new

## Public API

- `Signature` - A signature over a message
- `SigningKey` - Private key for signing (must be kept confidential)
- `VerifyingKey` - Public key for verification (can be shared)
- `sign()` - Sign a message with a SigningKey
- `verify()` - Verify a signature with a VerifyingKey
- `Error` - Error type for signature operations

## Usage

```rust
use pq_signature::{SigningKey, VerifyingKey, sign, verify};

// Create keys from bytes
let sk = SigningKey::from_bytes(signing_key_bytes)?;
let vk = VerifyingKey::from_bytes(verifying_key_bytes)?;

// Sign a message
let message = b"Hello, world!";
let signature = sign(&sk, message)?;

// Verify the signature
verify(&vk, message, &signature)?;
```

## Features

- Constant-time comparison for keys to prevent timing attacks
- Clean separation of signing and verification operations
- Compatible interface for ML-DSA (NIST FIPS 204 / Dilithium)
- Deterministic signatures for testing and verification

## Dependencies

Workspace crates:

*(none)*

External crates:

- `ml-dsa` - ML-DSA implementation
- `sha2` - SHA-256 hashing for signatures

## Testing

Implemented test kinds: unit (15 tests), known-answer, tamper, wrong-key.

## Status

Implemented in Batch F (independent crypto crates).
