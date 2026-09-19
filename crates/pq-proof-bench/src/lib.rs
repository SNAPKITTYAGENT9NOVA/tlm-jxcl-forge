//! Criterion benchmarks measuring `pq-error-proof`'s real
//! `attest()`/`verify()` throughput.
//!
//! Owns: The benchmark harness (dev-only, not a library dependency of
//! anything). The actual `#[[bench]]` target lives in `benches/attest_verify.rs`;
//! this module holds small, real (unit-tested) setup helpers shared by
//! that file, kept out of the bench file itself so setup logic is
//! covered by `cargo test`, not only exercised implicitly by
//! `cargo bench`.
//!
//! # Which path is benchmarked, and why
//!
//! `attest()` is benchmarked by calling `pq-error-proof::attest`
//! directly. `pq-proof-types::ProofScheme` only defines `verify` (a
//! registry needs to check attestations against many possible schemes;
//! it never needs to mint one), so there is no registry path for
//! `attest` to go through -- calling `pq-error-proof` directly is the
//! only option, not a shortcut taken because the registry path was
//! awkward.
//!
//! `verify()` is benchmarked two ways: directly against
//! `pq-error-proof::verify` (the existing, unchanged function), and
//! through `pq-proof-registry`/`pq-proof-verifier`'s
//! `verify_attestation` facade (the path other crates actually use once
//! `pq-error-proof` registers itself as a `ProofScheme`). Comparing the
//! two shows the registry/trait-object indirection's overhead is
//! negligible next to the Groth16 pairing check itself.
#![forbid(unsafe_code)]

use ark_std::rand::{rngs::StdRng, SeedableRng};
use pq_error_proof::Params;
use pq_proof_registry::ProofRegistry;

/// Fixed seed for the one-time benchmark setup RNG (trusted-setup
/// generation). Individual `attest` calls inside a benchmark loop use
/// their own advancing draws from this same RNG, exactly as any real
/// caller would -- this only fixes the *starting point* so setup cost
/// is reproducible across runs.
pub const BENCH_SETUP_SEED: u64 = 0xC0FF_EE00;

/// A fresh, deterministically-seeded RNG for benchmark setup.
pub fn bench_rng() -> StdRng {
    StdRng::seed_from_u64(BENCH_SETUP_SEED)
}

/// Register `params` into a fresh [`ProofRegistry`] under
/// `pq_error_proof::SCHEME_ID`, for benchmarks (or callers generally)
/// that want to exercise the registry/`pq-proof-verifier` path rather
/// than calling `pq-error-proof` directly.
pub fn registry_with_pq_error_proof(params: Params) -> ProofRegistry {
    let mut registry = ProofRegistry::new();
    registry.register(pq_error_proof::SCHEME_ID, Box::new(params));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use pq_error_proof::attest;
    use pq_proof_types::AttestationBytes;
    use pq_proof_verifier::verify_attestation;

    #[test]
    fn registry_wiring_matches_pq_error_proofs_own_verify() {
        let mut rng = bench_rng();
        let params = Params::generate(&mut rng).unwrap();
        let context = b"bench-setup-context";
        let attestation = attest(&params, context, &mut rng).unwrap();
        let attestation_bytes = AttestationBytes::new(attestation.to_bytes().unwrap());

        let registry = registry_with_pq_error_proof(params);

        assert!(verify_attestation(
            &registry,
            pq_error_proof::SCHEME_ID,
            context,
            &attestation_bytes
        )
        .unwrap());
    }
}
