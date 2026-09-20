// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Shared tracing/logging initialization.
//!
//! Extracted from the one real precedent in the pre-expansion workspace:
//! `photo-cache-service/src/lib.rs`'s `init_tracing()`, which every
//! binary in that crate calls at startup. `jxcl/src/main.rs` (the other
//! source this crate is nominally extracted from, per `docs/crates.toml`)
//! has *no* logging setup at all -- its hand-rolled CLI (spec §40: "Do
//! not introduce unnecessary frameworks") reports errors with plain
//! `eprintln!`, deliberately. So there is only one real implementation to
//! extract; this crate generalizes it into a reusable `init_logging()`
//! so a future `jxcl-cli` (this batch's `jxcl` facade + follow-on
//! toolchain batch) can opt into the same convention without duplicating
//! it, rather than inventing a second logging setup from nothing.
#![forbid(unsafe_code)]

use tracing_subscriber::EnvFilter;

/// Environment variable honored for the log filter directive, matching
/// `tracing_subscriber`'s own convention and `photo-cache-service`'s
/// doc comment ("Honors `RUST_LOG`").
pub const LOG_FILTER_ENV_VAR: &str = "RUST_LOG";

/// Default filter directive used when `RUST_LOG` is unset or empty.
pub const DEFAULT_LOG_FILTER: &str = "info";

/// Initialize structured logging for the current process. Honors
/// `RUST_LOG` (defaulting to `info`) so operators can turn up verbosity
/// without a redeploy, exactly as `photo-cache-service::init_tracing`
/// did.
///
/// Unlike the original (which called `.init()` and would panic if a
/// global subscriber were already installed), this uses `try_init()`
/// and silently no-ops on failure. That's a deliberate, minor hardening
/// over the extracted behavior: a shared library-level `init_logging()`
/// can plausibly be called from more than one place (a binary's `main`
/// *and* a test harness, or twice during a graceful-restart sequence),
/// and "logging was already configured" is not a condition any caller
/// should crash over.
pub fn init_logging() {
    let filter = EnvFilter::try_new(effective_filter_directive())
        .unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// The filter directive `init_logging` will use, read from
/// [`LOG_FILTER_ENV_VAR`] with a fallback to [`DEFAULT_LOG_FILTER`].
pub fn effective_filter_directive() -> String {
    effective_filter_directive_from(std::env::var(LOG_FILTER_ENV_VAR).ok())
}

/// The pure (environment-independent) logic behind
/// [`effective_filter_directive`], split out so it's testable without
/// mutating the process-global environment -- which would race against
/// other tests running in parallel in the same process, exactly the
/// concern `photo-cache-service::check_redis_tls_requirement_for`
/// documents for the same reason.
fn effective_filter_directive_from(value: Option<String>) -> String {
    match value {
        Some(v) if !v.is_empty() => v,
        _ => DEFAULT_LOG_FILTER.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_info_when_unset() {
        assert_eq!(effective_filter_directive_from(None), "info");
    }

    #[test]
    fn defaults_to_info_when_empty() {
        assert_eq!(effective_filter_directive_from(Some(String::new())), "info");
    }

    #[test]
    fn honors_an_explicit_directive() {
        assert_eq!(
            effective_filter_directive_from(Some("debug".to_string())),
            "debug"
        );
        assert_eq!(
            effective_filter_directive_from(Some("jxcl=trace,warn".to_string())),
            "jxcl=trace,warn"
        );
    }

    #[test]
    fn init_logging_does_not_panic_even_when_called_more_than_once() {
        init_logging();
        init_logging();
    }
}
