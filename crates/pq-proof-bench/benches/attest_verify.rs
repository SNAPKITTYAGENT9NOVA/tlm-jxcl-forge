// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Criterion benchmarks for `pq-error-proof`'s real Groth16
//! `attest()`/`verify()` throughput.
//!
//! Run with `cargo bench -p pq-proof-bench`. See `src/lib.rs`'s module
//! docs for which of these go through `pq-proof-registry` and why.

use ark_std::rand::RngCore;
use criterion::{criterion_group, criterion_main, Criterion};
use pq_error_proof::{attest, verify, Params};
use pq_proof_bench::{bench_rng, registry_with_pq_error_proof};
use pq_proof_types::AttestationBytes;
use pq_proof_verifier::verify_attestation;

const ERROR_CONTEXT: &[u8] = b"error_code=DECRYPT_AEAD_MISMATCH;key_version=7;envelope=deadbeef";

fn bench_attest(c: &mut Criterion) {
    let mut rng = bench_rng();
    let params = Params::generate(&mut rng).unwrap();

    // Groth16 proving needs fresh secret randomness per call, so the
    // RNG itself (not just the params) has to live across iterations;
    // `attest` draws from it each time exactly as any real caller would.
    let mut iter_rng = bench_rng();
    // Advance past the draws `Params::generate` above already made from
    // the *setup* rng's seed stream by re-seeding independently, so
    // `attest`'s own draws don't start from an already-consumed state.
    iter_rng.next_u64();

    c.bench_function("pq-error-proof: attest (direct)", |b| {
        b.iter(|| attest(&params, ERROR_CONTEXT, &mut iter_rng).unwrap())
    });
}

fn bench_verify_direct(c: &mut Criterion) {
    let mut rng = bench_rng();
    let params = Params::generate(&mut rng).unwrap();
    let attestation = attest(&params, ERROR_CONTEXT, &mut rng).unwrap();

    c.bench_function("pq-error-proof: verify (direct)", |b| {
        b.iter(|| verify(&params, ERROR_CONTEXT, &attestation).unwrap())
    });
}

fn bench_verify_via_registry(c: &mut Criterion) {
    let mut rng = bench_rng();
    let params = Params::generate(&mut rng).unwrap();
    let attestation = attest(&params, ERROR_CONTEXT, &mut rng).unwrap();
    let attestation_bytes = AttestationBytes::new(attestation.to_bytes().unwrap());

    let registry = registry_with_pq_error_proof(params);

    c.bench_function("pq-error-proof: verify (via pq-proof-registry)", |b| {
        b.iter(|| {
            verify_attestation(
                &registry,
                pq_error_proof::SCHEME_ID,
                ERROR_CONTEXT,
                &attestation_bytes,
            )
            .unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_attest,
    bench_verify_direct,
    bench_verify_via_registry
);
criterion_main!(benches);
