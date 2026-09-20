// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Environment-variable configuration parsing helpers.
//!
//! Extracted and generalized from two independent, near-duplicate
//! implementations:
//! - `photo-cache-service/src/lib.rs`'s `env_or`/`env_or_u16`/
//!   `env_or_u64` (typed env-var lookup with a default), and its
//!   `decode_hex_seed` (hex-decode a fixed-length seed, used to build a
//!   stable `pq_crypto::KeyPair` across replicas from `PQ_KEM_SEED`).
//! - `pq-crypto/src/lib.rs`'s `SEED_LEN` constant, which
//!   `photo-cache-service`'s `decode_hex_seed` is sized against
//!   (`SEED_LEN * 2` hex characters). This crate's `decode_hex_seed` is
//!   generic over the seed length instead of hardcoding 64 bytes, so it
//!   isn't tied to `pq-crypto` specifically (a foundation crate must not
//!   depend on a crypto crate) while still covering the exact same real
//!   use case.
//!
//! `env_or_u16`/`env_or_u64` are unified here into one generic
//! `env_or_parse::<T>`, since they were byte-for-byte identical apart
//! from the type parameter.
#![forbid(unsafe_code)]

use std::str::FromStr;

use jxcl_errors::Error;

/// Read `name` from the environment, or fall back to `default`.
///
/// Deliberately does *not* log the resolved value: some config (e.g. a
/// database URL) can embed a credential, and a generic helper is the
/// wrong place to decide what's safe to print (see
/// `jxcl-security::redact_secret_bearing_string`, outside this batch,
/// for the shared redaction convention callers should apply first).
pub fn env_or(name: &str, default: &str) -> String {
    match std::env::var(name) {
        Ok(v) if !v.is_empty() => v,
        _ => default.to_string(),
    }
}

/// Read `name` from the environment and parse it as `T`, or fall back to
/// `default` if the variable is unset, empty, or fails to parse.
///
/// Generalizes `photo-cache-service`'s `env_or_u16`/`env_or_u64` (which
/// were identical apart from the numeric type) to any `FromStr` type.
pub fn env_or_parse<T: FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Decode a `2 * N`-hex-character string into an `N`-byte seed.
///
/// This is the generic form of `photo-cache-service::decode_hex_seed`
/// (there, hardcoded to `pq_crypto::SEED_LEN == 64` bytes / 128 hex
/// characters): reads exactly `2 * N` hex characters and nothing else --
/// a too-short, too-long, or non-hex input is rejected outright rather
/// than truncated or padded, matching the original's "refuse to start
/// with an ambiguous key configuration" philosophy (see
/// `photo-cache-service::keypair_from_env`'s doc comment).
pub fn decode_hex_seed<const N: usize>(name: &'static str, hex: &str) -> Result<[u8; N], Error> {
    if hex.len() != N * 2 {
        return Err(Error::InvalidConfig {
            name,
            reason: format!(
                "expected {} hex characters ({} bytes), got {}",
                N * 2,
                N,
                hex.len()
            ),
        });
    }
    let mut seed = [0u8; N];
    for (i, byte) in seed.iter_mut().enumerate() {
        let digits = hex
            .get(i * 2..i * 2 + 2)
            .ok_or_else(|| Error::InvalidConfig {
                name,
                reason: "hex string ended unexpectedly".to_string(),
            })?;
        *byte = u8::from_str_radix(digits, 16).map_err(|_| Error::InvalidConfig {
            name,
            reason: format!("{:?} is not valid hex", digits),
        })?;
    }
    Ok(seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // std::env::set_var/remove_var mutate global process state; these
    // tests serialize on a lock (rather than relying on cargo test's
    // default parallelism to somehow not collide) exactly because
    // `photo-cache-service`'s own test suite calls out this same hazard
    // for `check_redis_tls_requirement_for` and sidesteps it by keeping
    // the env-touching logic in a pure function -- `env_or`/
    // `env_or_parse` *are* the env-touching layer here, so the lock is
    // this crate's equivalent safeguard.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn env_or_returns_default_when_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("JXCL_CONFIG_TEST_UNSET");
        assert_eq!(env_or("JXCL_CONFIG_TEST_UNSET", "fallback"), "fallback");
    }

    #[test]
    fn env_or_returns_default_when_empty() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("JXCL_CONFIG_TEST_EMPTY", "");
        assert_eq!(env_or("JXCL_CONFIG_TEST_EMPTY", "fallback"), "fallback");
        std::env::remove_var("JXCL_CONFIG_TEST_EMPTY");
    }

    #[test]
    fn env_or_returns_set_value() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("JXCL_CONFIG_TEST_SET", "hello");
        assert_eq!(env_or("JXCL_CONFIG_TEST_SET", "fallback"), "hello");
        std::env::remove_var("JXCL_CONFIG_TEST_SET");
    }

    #[test]
    fn env_or_parse_parses_typed_values() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("JXCL_CONFIG_TEST_PORT", "8080");
        assert_eq!(env_or_parse::<u16>("JXCL_CONFIG_TEST_PORT", 0), 8080);
        std::env::remove_var("JXCL_CONFIG_TEST_PORT");
    }

    #[test]
    fn env_or_parse_falls_back_on_unparseable_value() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("JXCL_CONFIG_TEST_BADNUM", "not-a-number");
        assert_eq!(env_or_parse::<u64>("JXCL_CONFIG_TEST_BADNUM", 42), 42);
        std::env::remove_var("JXCL_CONFIG_TEST_BADNUM");
    }

    #[test]
    fn env_or_parse_falls_back_when_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("JXCL_CONFIG_TEST_MISSING_NUM");
        assert_eq!(env_or_parse::<u64>("JXCL_CONFIG_TEST_MISSING_NUM", 7), 7);
    }

    #[test]
    fn decode_hex_seed_accepts_exact_length_hex() {
        let hex = "ab".repeat(64);
        let seed: [u8; 64] = decode_hex_seed("SEED", &hex).unwrap();
        assert_eq!(seed, [0xabu8; 64]);
    }

    #[test]
    fn decode_hex_seed_rejects_wrong_length() {
        let err = decode_hex_seed::<64>("SEED", "ab").unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { name: "SEED", .. }));
    }

    #[test]
    fn decode_hex_seed_rejects_non_hex() {
        let hex = "zz".repeat(64);
        assert!(decode_hex_seed::<64>("SEED", &hex).is_err());
    }

    #[test]
    fn decode_hex_seed_boundary_one_char_short() {
        let hex = "ab".repeat(64);
        let short = &hex[..hex.len() - 1];
        assert!(decode_hex_seed::<64>("SEED", short).is_err());
    }

    #[test]
    fn decode_hex_seed_boundary_one_char_long() {
        let mut hex = "ab".repeat(64);
        hex.push('0');
        assert!(decode_hex_seed::<64>("SEED", &hex).is_err());
    }

    #[test]
    fn decode_hex_seed_works_for_other_widths_too() {
        // Generic over N, unlike the pre-expansion pq-crypto-specific
        // version -- exercise a width pq-crypto doesn't use.
        let seed: [u8; 16] = decode_hex_seed("OTHER_SEED", &"11".repeat(16)).unwrap();
        assert_eq!(seed, [0x11u8; 16]);
    }
}
