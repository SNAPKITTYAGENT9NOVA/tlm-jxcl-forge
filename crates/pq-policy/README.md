# pq-policy

Consolidates scattered policy decisions (TLS-required-in-production, minimum seed/key length) into one crate with a real Policy trait, replacing duplicated ad hoc checks in photo-cache-service and pq-crypto.

## Architecture

**Owns:** The Policy trait and the concrete TlsRequiredInProduction/MinimumSeedLength policies.

**Category:** crypto · **Source:** new

## Public API

- `Policy` - Core trait for policy enforcement
- `PolicyError` - Error type for policy violations
- `PolicyContext` - Context information for policy checks
- `TlsRequiredInProduction` - Built-in policy for TLS requirement in production
- `MinimumSeedLength` - Built-in policy for minimum key/seed length enforcement
- `CompositePolicy` - Composite policy that combines multiple policies

## Usage

```rust
use pq_policy::{Policy, PolicyContext, TlsRequiredInProduction, MinimumSeedLength};

let tls_policy = TlsRequiredInProduction;
let seed_policy = MinimumSeedLength::recommended();

let context = PolicyContext::new(
    true,   // is_production
    64,     // key_length in bytes
    true,   // tls_enabled
);

tls_policy.check(&context)?;
seed_policy.check(&context)?;
```

## Dependencies

Workspace crates:

- `jxcl-errors`

External crates:

*(none)*

## Testing

Implemented test kinds: unit (12 tests), boundary.

## Status

Implemented in Batch F (independent crypto crates).
