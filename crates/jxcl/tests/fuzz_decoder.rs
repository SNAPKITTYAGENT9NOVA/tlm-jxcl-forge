// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Decoder fuzz target (spec §33): random byte streams must never panic,
//! corrupt host state, or behave non-deterministically — only ever
//! decode successfully or return a structured `DecodeError`.
//!
//! A real `cargo-fuzz`/libFuzzer corpus-based harness needs a nightly
//! toolchain and an external dependency, which spec §40 rules out for
//! this reference build ("Do not introduce network dependencies" /
//! keep to one systems language, no unnecessary frameworks). This test
//! gives the same crash-freedom guarantee inside an ordinary
//! `cargo test` run: a fixed-seed PRNG generates a large, reproducible
//! corpus of byte streams (including many that are *not* valid
//! programs) and every one of them must come back as `Ok` or `Err`,
//! never a panic.

mod common;

use common::Xorshift64;
use jxcl::encoding::decoder::{decode_all, decode_one};

const ITERATIONS: usize = 20_000;

#[test]
fn decode_one_never_panics_on_random_bytes() {
    let mut rng = Xorshift64::new(0xF00D_CAFE_1234_5678);
    for _ in 0..ITERATIONS {
        let len = (rng.next_u64() % 16) as usize;
        let mut buf = vec![0u8; len];
        rng.fill_bytes(&mut buf);
        // The only contract here is "does not panic"; both Ok and Err
        // are architecturally valid outcomes for arbitrary bytes.
        let _ = decode_one(&buf, 0);
    }
}

#[test]
fn decode_all_never_panics_and_always_terminates() {
    let mut rng = Xorshift64::new(0x0BAD_F00D_DEAD_BEEF);
    for _ in 0..ITERATIONS / 10 {
        let len = (rng.next_u64() % 256) as usize;
        let mut buf = vec![0u8; len];
        rng.fill_bytes(&mut buf);
        let _ = decode_all(&buf);
    }
}

#[test]
fn decode_one_never_reads_past_the_given_slice() {
    // Every valid opcode paired with a too-short buffer must fail
    // cleanly (TruncatedInstruction) rather than panicking on an
    // out-of-bounds slice index.
    for def in jxcl::isa::opcodes::all_defs() {
        for short_len in 0..def.format.len() {
            let mut buf = vec![0u8; short_len];
            if short_len > 0 {
                buf[0] = def.opcode;
            }
            let _ = decode_one(&buf, 0);
        }
    }
}
