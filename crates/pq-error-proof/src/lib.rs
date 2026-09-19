//! Zero-knowledge proofs of honestly-opened error attestations.
//!
//! # Why this exists
//!
//! The original ask was an FFI-based error-handling layer proved correct
//! "down to circuit level with circom." Circom's toolchain lives on
//! GitHub, which this environment's egress policy blocks, so this crate
//! is a real substitute rather than that literal request: a genuine
//! Groth16 zk-SNARK circuit, built entirely from crates.io (arkworks),
//! over BLS12-381 with the Jubjub curve embedded in its scalar field
//! (the standard SNARK-friendly pairing used by, e.g., Zcash Sapling).
//! Same underlying cryptography (R1CS + Groth16) that a circom circuit
//! would compile down to -- just authored directly in Rust instead of in
//! circom's DSL, so it works within this environment's constraints.
//!
//! # What is actually proven
//!
//! A service that wants to record "an error of this kind happened" in a
//! way that can later be independently checked, without exposing
//! whatever secret material informed the error, does this:
//!
//! 1. Derive a public `message` field element deterministically from the
//!    plaintext error context (error code, key version, envelope
//!    identifier, etc. -- see [`derive_message`]). Anyone can recompute
//!    this from the same context; it is not a secret.
//! 2. Pick a fresh secret `randomness` field element.
//! 3. Compute the Pedersen commitment `commitment = message*G +
//!    randomness*H` over the Jubjub curve, and record `commitment`
//!    alongside the error context.
//! 4. Produce a Groth16 proof of "I know a `randomness` that opens
//!    `commitment` for this public `message`" ([`attest`]).
//!
//! Anyone holding the public [`Params`] can later run [`verify`] against
//! the recorded error context and commitment to confirm the attestation
//! really was honestly derived by someone holding an opening
//! `randomness` for it, without that randomness ever being revealed.
//! This is the "provable/verifiable error state" building block: a
//! forgeable-only-by-the-secret-holder, publicly-checkable receipt for
//! an error event.
//!
//! # What is *not* proven
//!
//! This does **not** prove that the underlying error genuinely occurred
//! inside `pq-crypto`/`pq-cache` (e.g. that an AES-GCM tag really failed
//! to verify). Modeling AEAD/KEM decryption itself as an R1CS circuit is
//! a much larger undertaking and out of scope here. What this proves is
//! narrower and still useful: that the party who published `commitment`
//! for a given error context held a genuine opening for it -- a
//! non-repudiation/integrity property for attestation records, not a
//! proof about the internals of the failure itself.
//!
//! # Setup is a local secret, not a public ceremony
//!
//! [`Params::generate`] runs a single-party "trusted setup" for this
//! circuit (standard Groth16 requirement). The resulting [`Params`] must
//! be generated once and distributed (its serialized bytes, via
//! [`Params::to_bytes`]/[`Params::from_bytes`]) to every party that
//! needs to attest or verify. This is appropriate for an internal
//! attestation system where one organization controls both roles; it is
//! **not** a public, trustless setup like a real multi-party ceremony
//! (e.g. Zcash's), and must not be presented as one.
#![forbid(unsafe_code)]

use ark_bls12_381::{Bls12_381, Fr};
use ark_ec::{AffineRepr, CurveGroup, PrimeGroup};
use ark_ed_on_bls12_381::{constraints::EdwardsVar, EdwardsAffine, EdwardsProjective, Fq};
use ark_ff::{PrimeField, UniformRand};
use ark_groth16::{prepare_verifying_key, Groth16, Proof, ProvingKey, VerifyingKey};
use ark_r1cs_std::{fields::fp::FpVar, prelude::*};
use ark_relations::gr1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::rand::{CryptoRng, RngCore};
use sha2::{Digest, Sha256};
use std::fmt;

/// Domain-separation tag for hashing an error context into the circuit's
/// public `message` field element. Bumping the version would deliberately
/// make old attestations unverifiable against a new [`Params`], the same
/// tradeoff `pq-crypto`'s `HKDF_INFO` documents.
const MESSAGE_DOMAIN: &[u8] = b"pq-error-proof/message/v1";

/// Domain-separation tag for deriving the second Pedersen generator `H`
/// by hashing to a curve point (try-and-increment), so nobody -- including
/// whoever runs [`Params::generate`] -- knows a discrete-log relationship
/// between `G` and `H`. Without that, the commitment would not be binding.
const GENERATOR_H_DOMAIN: &[u8] = b"pq-error-proof/pedersen-h/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Circuit setup (proving/verifying key generation) failed.
    Setup,
    /// Proof generation failed.
    Proving,
    /// Proof verification itself errored (distinct from a valid proof
    /// that simply doesn't verify -- see [`verify`]'s `Ok(false)`).
    Verification,
    /// Serializing a [`Params`] or [`Attestation`] failed.
    Serialization,
    /// Deserializing a [`Params`] or [`Attestation`] failed: truncated,
    /// corrupt, or not produced by this crate.
    Deserialization,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Error {}

/// Hash `error_context` (e.g. `b"error_code=...;key_version=...;envelope=..."`)
/// into the circuit's public `message` field element. Deterministic and
/// public: anyone can recompute it from the same context bytes.
pub fn derive_message(error_context: &[u8]) -> Fq {
    let mut hasher = Sha256::new();
    hasher.update(MESSAGE_DOMAIN);
    hasher.update(error_context);
    Fq::from_le_bytes_mod_order(&hasher.finalize())
}

/// Hash a fixed domain string to a curve point via try-and-increment,
/// then clear the cofactor so the result lands in the prime-order
/// subgroup. Used once, at [`Params::generate`] time, to derive `H`.
fn hash_to_curve(domain: &[u8]) -> EdwardsProjective {
    let mut counter: u32 = 0;
    loop {
        let mut hasher = Sha256::new();
        hasher.update(domain);
        hasher.update(counter.to_le_bytes());
        if let Some(point) = EdwardsAffine::from_random_bytes(&hasher.finalize()) {
            return point.clear_cofactor().into_group();
        }
        counter += 1;
    }
}

/// R1CS circuit proving knowledge of `randomness` such that
/// `message*G + randomness*H == commitment`, for public `message` and
/// `commitment`, and private `randomness`.
#[derive(Clone)]
struct CommitmentOpeningCircuit {
    generator_g: EdwardsProjective,
    generator_h: EdwardsProjective,
    message: Option<Fq>,
    randomness: Option<Fq>,
    commitment: Option<EdwardsProjective>,
}

impl ConstraintSynthesizer<Fr> for CommitmentOpeningCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let g = EdwardsVar::new_constant(cs.clone(), self.generator_g)?;
        let h = EdwardsVar::new_constant(cs.clone(), self.generator_h)?;

        let message = FpVar::new_input(cs.clone(), || {
            self.message.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let commitment_x = FpVar::new_input(cs.clone(), || {
            self.commitment
                .ok_or(SynthesisError::AssignmentMissing)
                .map(|c| c.into_affine().x)
        })?;
        let commitment_y = FpVar::new_input(cs.clone(), || {
            self.commitment
                .ok_or(SynthesisError::AssignmentMissing)
                .map(|c| c.into_affine().y)
        })?;

        let randomness = FpVar::new_witness(cs.clone(), || {
            self.randomness.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let message_bits = message.to_bits_le()?;
        let randomness_bits = randomness.to_bits_le()?;

        let computed =
            g.scalar_mul_le(message_bits.iter())? + h.scalar_mul_le(randomness_bits.iter())?;

        computed.x.enforce_equal(&commitment_x)?;
        computed.y.enforce_equal(&commitment_y)?;

        Ok(())
    }
}

/// The circuit's public parameters: the two Pedersen generators and the
/// Groth16 proving/verifying key pair. Generate once with
/// [`Params::generate`], persist with [`Params::to_bytes`], and load
/// with [`Params::from_bytes`] everywhere [`attest`] or [`verify`] runs.
pub struct Params {
    generator_g: EdwardsProjective,
    generator_h: EdwardsProjective,
    proving_key: ProvingKey<Bls12_381>,
    verifying_key: VerifyingKey<Bls12_381>,
}

impl Params {
    /// Run this circuit's (single-party, local) setup. See the crate
    /// docs' "Setup is a local secret, not a public ceremony" section.
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Result<Self, Error> {
        let generator_g: EdwardsProjective = EdwardsAffine::generator().into_group();
        let generator_h = hash_to_curve(GENERATOR_H_DOMAIN);

        let setup_circuit = CommitmentOpeningCircuit {
            generator_g,
            generator_h,
            message: None,
            randomness: None,
            commitment: None,
        };

        let (proving_key, verifying_key) =
            Groth16::<Bls12_381>::circuit_specific_setup(setup_circuit, rng)
                .map_err(|_| Error::Setup)?;

        Ok(Params {
            generator_g,
            generator_h,
            proving_key,
            verifying_key,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut out = Vec::new();
        self.generator_g
            .serialize_compressed(&mut out)
            .map_err(|_| Error::Serialization)?;
        self.generator_h
            .serialize_compressed(&mut out)
            .map_err(|_| Error::Serialization)?;
        self.proving_key
            .serialize_compressed(&mut out)
            .map_err(|_| Error::Serialization)?;
        self.verifying_key
            .serialize_compressed(&mut out)
            .map_err(|_| Error::Serialization)?;
        Ok(out)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let mut reader = bytes;
        let generator_g = EdwardsProjective::deserialize_compressed(&mut reader)
            .map_err(|_| Error::Deserialization)?;
        let generator_h = EdwardsProjective::deserialize_compressed(&mut reader)
            .map_err(|_| Error::Deserialization)?;
        let proving_key = ProvingKey::<Bls12_381>::deserialize_compressed(&mut reader)
            .map_err(|_| Error::Deserialization)?;
        let verifying_key = VerifyingKey::<Bls12_381>::deserialize_compressed(&mut reader)
            .map_err(|_| Error::Deserialization)?;
        Ok(Params {
            generator_g,
            generator_h,
            proving_key,
            verifying_key,
        })
    }
}

/// A published error attestation: the Pedersen commitment (the public
/// "receipt" for the error context) and the proof that it was honestly
/// opened. Both fields are safe to store and transmit publicly -- the
/// secret opening randomness never appears here.
pub struct Attestation {
    pub commitment: EdwardsAffine,
    pub proof: Proof<Bls12_381>,
}

impl Attestation {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut out = Vec::new();
        self.commitment
            .serialize_compressed(&mut out)
            .map_err(|_| Error::Serialization)?;
        self.proof
            .serialize_compressed(&mut out)
            .map_err(|_| Error::Serialization)?;
        Ok(out)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let mut reader = bytes;
        let commitment = EdwardsAffine::deserialize_compressed(&mut reader)
            .map_err(|_| Error::Deserialization)?;
        let proof = Proof::<Bls12_381>::deserialize_compressed(&mut reader)
            .map_err(|_| Error::Deserialization)?;
        Ok(Attestation { commitment, proof })
    }
}

/// Attest to `error_context`: derive the public `message`, pick a fresh
/// secret opening randomness, compute the commitment, and prove
/// knowledge of that randomness. The randomness itself is discarded --
/// it never leaves this function.
pub fn attest<R: RngCore + CryptoRng>(
    params: &Params,
    error_context: &[u8],
    rng: &mut R,
) -> Result<Attestation, Error> {
    let message = derive_message(error_context);
    let randomness = Fq::rand(rng);

    let commitment = params.generator_g.mul_bigint(message.into_bigint())
        + params.generator_h.mul_bigint(randomness.into_bigint());

    let circuit = CommitmentOpeningCircuit {
        generator_g: params.generator_g,
        generator_h: params.generator_h,
        message: Some(message),
        randomness: Some(randomness),
        commitment: Some(commitment),
    };

    let proof = Groth16::<Bls12_381>::prove(&params.proving_key, circuit, rng)
        .map_err(|_| Error::Proving)?;

    Ok(Attestation {
        commitment: commitment.into_affine(),
        proof,
    })
}

/// Verify that `attestation` is a genuine, honestly-opened attestation
/// for `error_context` under `params`. `Ok(false)` means the proof is
/// well-formed but does not hold (wrong context, wrong commitment, or a
/// forged proof); `Err` means verification itself could not be
/// completed (e.g. a malformed proof).
pub fn verify(
    params: &Params,
    error_context: &[u8],
    attestation: &Attestation,
) -> Result<bool, Error> {
    let message = derive_message(error_context);
    let pvk = prepare_verifying_key(&params.verifying_key);

    let public_inputs = vec![message, attestation.commitment.x, attestation.commitment.y];

    Groth16::<Bls12_381>::verify_with_processed_vk(&pvk, &public_inputs, &attestation.proof)
        .map_err(|_| Error::Verification)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_std::rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn attest_then_verify_roundtrips() {
        let mut rng = StdRng::seed_from_u64(1);
        let params = Params::generate(&mut rng).unwrap();

        let context = b"error_code=DECRYPT_AEAD_MISMATCH;key_version=7;envelope=deadbeef";
        let attestation = attest(&params, context, &mut rng).unwrap();

        assert!(verify(&params, context, &attestation).unwrap());
    }

    #[test]
    fn verification_fails_for_a_different_error_context() {
        let mut rng = StdRng::seed_from_u64(2);
        let params = Params::generate(&mut rng).unwrap();

        let context = b"error_code=DECRYPT_AEAD_MISMATCH;key_version=7;envelope=deadbeef";
        let attestation = attest(&params, context, &mut rng).unwrap();

        let other_context = b"error_code=DECRYPT_AEAD_MISMATCH;key_version=8;envelope=deadbeef";
        assert!(!verify(&params, other_context, &attestation).unwrap());
    }

    #[test]
    fn verification_fails_against_a_different_params_setup() {
        let mut rng = StdRng::seed_from_u64(3);
        let params_a = Params::generate(&mut rng).unwrap();
        let params_b = Params::generate(&mut rng).unwrap();

        let context = b"error_code=KEY_RING_NO_ACTIVE_VERSION";
        let attestation = attest(&params_a, context, &mut rng).unwrap();

        // A proof made under one setup's proving key does not verify
        // under an unrelated setup's verifying key: the pairing check
        // simply fails closed rather than erroring.
        assert!(!verify(&params_b, context, &attestation).unwrap());
    }

    #[test]
    fn tampered_commitment_fails_verification() {
        let mut rng = StdRng::seed_from_u64(4);
        let params = Params::generate(&mut rng).unwrap();

        let context = b"error_code=RETIRED_KEY_VERSION;key_version=1";
        let mut attestation = attest(&params, context, &mut rng).unwrap();
        attestation.commitment =
            (attestation.commitment.into_group() + params.generator_g).into_affine();

        assert!(!verify(&params, context, &attestation).unwrap());
    }

    #[test]
    fn params_serialization_roundtrips_and_still_verifies() {
        let mut rng = StdRng::seed_from_u64(5);
        let params = Params::generate(&mut rng).unwrap();
        let context = b"error_code=UNKNOWN_KEY_VERSION";
        let attestation = attest(&params, context, &mut rng).unwrap();

        let params_bytes = params.to_bytes().unwrap();
        let reloaded_params = Params::from_bytes(&params_bytes).unwrap();

        assert!(verify(&reloaded_params, context, &attestation).unwrap());
    }

    #[test]
    fn attestation_serialization_roundtrips() {
        let mut rng = StdRng::seed_from_u64(6);
        let params = Params::generate(&mut rng).unwrap();
        let context = b"error_code=MALFORMED_ENVELOPE";
        let attestation = attest(&params, context, &mut rng).unwrap();

        let attestation_bytes = attestation.to_bytes().unwrap();
        let reloaded_attestation = Attestation::from_bytes(&attestation_bytes).unwrap();

        assert!(verify(&params, context, &reloaded_attestation).unwrap());
    }

    #[test]
    fn two_attestations_of_the_same_context_are_unlinkable() {
        // Fresh randomness each time -> different commitments and
        // proofs for an identical error context (hiding property).
        let mut rng = StdRng::seed_from_u64(7);
        let params = Params::generate(&mut rng).unwrap();
        let context = b"error_code=DECRYPT_AEAD_MISMATCH;key_version=7;envelope=deadbeef";

        let a1 = attest(&params, context, &mut rng).unwrap();
        let a2 = attest(&params, context, &mut rng).unwrap();

        assert_ne!(a1.commitment, a2.commitment);
        assert!(verify(&params, context, &a1).unwrap());
        assert!(verify(&params, context, &a2).unwrap());
    }

    #[test]
    fn from_bytes_rejects_truncated_params() {
        let mut rng = StdRng::seed_from_u64(8);
        let params = Params::generate(&mut rng).unwrap();
        let bytes = params.to_bytes().unwrap();
        assert!(Params::from_bytes(&bytes[..bytes.len() / 2]).is_err());
    }

    #[test]
    fn from_bytes_rejects_truncated_attestation() {
        let mut rng = StdRng::seed_from_u64(9);
        let params = Params::generate(&mut rng).unwrap();
        let attestation = attest(&params, b"error_code=X", &mut rng).unwrap();
        let bytes = attestation.to_bytes().unwrap();
        assert!(Attestation::from_bytes(&bytes[..4]).is_err());
    }
}
