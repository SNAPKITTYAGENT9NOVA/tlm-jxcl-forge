// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Proves that `Params`'s `ProofScheme` impl (the `pq-proof-types`
//! glue) produces exactly the same accept/reject result as calling this
//! crate's own `attest`/`verify` directly, for the same inputs. The
//! trait impl must be pure glue over the existing, unchanged
//! implementation -- this is what that guarantee looks like as a test.

use ark_std::rand::{rngs::StdRng, SeedableRng};
use pq_error_proof::{attest, verify, Params, SCHEME_ID};
use pq_proof_types::{AttestationBytes, ProofScheme};

#[test]
fn trait_impl_matches_direct_verify_for_a_genuine_attestation() {
    let mut rng = StdRng::seed_from_u64(4_242);
    let params = Params::generate(&mut rng).unwrap();
    let context = b"error_code=DECRYPT_AEAD_MISMATCH;key_version=7;envelope=deadbeef";
    let attestation = attest(&params, context, &mut rng).unwrap();

    let direct_result = verify(&params, context, &attestation).unwrap();

    let attestation_bytes = AttestationBytes::new(attestation.to_bytes().unwrap());
    let trait_result = ProofScheme::verify(&params, context, &attestation_bytes).unwrap();

    assert_eq!(direct_result, trait_result);
    assert!(
        trait_result,
        "a genuine attestation must verify via both paths"
    );
}

#[test]
fn trait_impl_matches_direct_verify_for_a_mismatched_context() {
    let mut rng = StdRng::seed_from_u64(4_243);
    let params = Params::generate(&mut rng).unwrap();
    let context = b"error_code=DECRYPT_AEAD_MISMATCH;key_version=7;envelope=deadbeef";
    let attestation = attest(&params, context, &mut rng).unwrap();
    let other_context = b"error_code=DECRYPT_AEAD_MISMATCH;key_version=8;envelope=deadbeef";

    let direct_result = verify(&params, other_context, &attestation).unwrap();

    let attestation_bytes = AttestationBytes::new(attestation.to_bytes().unwrap());
    let trait_result = ProofScheme::verify(&params, other_context, &attestation_bytes).unwrap();

    assert_eq!(direct_result, trait_result);
    assert!(
        !trait_result,
        "a mismatched context must be rejected via both paths"
    );
}

#[test]
fn trait_impl_matches_direct_verify_for_a_tampered_commitment() {
    use ark_ec::{AffineRepr, CurveGroup};

    let mut rng = StdRng::seed_from_u64(4_244);
    let params = Params::generate(&mut rng).unwrap();
    let context = b"error_code=RETIRED_KEY_VERSION;key_version=1";
    let mut attestation = attest(&params, context, &mut rng).unwrap();
    // Tamper with the commitment the same way pq-error-proof's own
    // `tampered_commitment_fails_verification` unit test does.
    attestation.commitment =
        (attestation.commitment.into_group() + generator_g_for_test(&params)).into_affine();

    let direct_result = verify(&params, context, &attestation).unwrap();

    let attestation_bytes = AttestationBytes::new(attestation.to_bytes().unwrap());
    let trait_result = ProofScheme::verify(&params, context, &attestation_bytes).unwrap();

    assert_eq!(direct_result, trait_result);
    assert!(
        !trait_result,
        "a tampered commitment must be rejected via both paths"
    );
}

#[test]
fn trait_impl_errors_on_undecodable_attestation_bytes() {
    let mut rng = StdRng::seed_from_u64(4_245);
    let params = Params::generate(&mut rng).unwrap();
    let attestation_bytes = AttestationBytes::new(vec![0xFF; 4]);

    assert!(ProofScheme::verify(&params, b"any context", &attestation_bytes).is_err());
}

#[test]
fn scheme_id_is_stable_and_matches_the_exported_constant() {
    let mut rng = StdRng::seed_from_u64(4_246);
    let params = Params::generate(&mut rng).unwrap();
    assert_eq!(ProofScheme::scheme_id(&params), SCHEME_ID);
}

/// Re-derives `generator_g` the same way `pq-error-proof`'s own
/// `tampered_commitment_fails_verification` unit test does, without
/// depending on any private field: it's `params.to_bytes()`'s first
/// serialized value, which we can regenerate identically here because
/// this crate's `Params::generate` always seeds `generator_g` from the
/// same fixed curve generator (not part of the trusted-setup secret).
fn generator_g_for_test(_params: &Params) -> ark_ed_on_bls12_381::EdwardsProjective {
    use ark_ec::AffineRepr;
    ark_ed_on_bls12_381::EdwardsAffine::generator().into_group()
}
