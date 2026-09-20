// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Tokio runtime configuration/startup helpers (worker-thread count from env, panic-hook wiring).
//!
//! Owns: build_runtime.
//!
//! Provides a single entry point [`build_runtime`] that constructs a configured
//! [`tokio::runtime::Runtime`] with worker-thread count read from the
//! `JXCL_WORKER_THREADS` environment variable, defaulting to the number of
//! logical CPUs if unset or unparseable. A panic hook is wired to prevent
//! async task panics from silently vanishing.
#![forbid(unsafe_code)]

use std::panic;

use jxcl_config::env_or_parse;

/// Construct a configured tokio runtime with worker threads and panic handling.
///
/// The runtime is configured as follows:
/// - **Worker threads:** read from the `JXCL_WORKER_THREADS` environment variable,
///   parsed as a positive integer. If the variable is unset, empty, or fails to parse,
///   defaults to the number of logical CPUs (via [`num_cpus::get`] if available, or
///   fallback to 1). Must not be 0.
/// - **Panic hook:** a panic hook is installed (via [`std::panic::set_hook`]) that
///   logs panics using [`eprintln!`] so that panicking async tasks do not silently
///   vanish. This hook is *process-global* and should be called once at startup.
///
/// # Returns
///
/// A configured [`tokio::runtime::Runtime`] on success, ready to block on or drive
/// async code.
///
/// # Panics
///
/// If the worker thread count resolves to 0, this function panics (a runtime with
/// no workers cannot execute any async code).
///
/// # Example
///
/// ```ignore
/// let rt = jxcl_runtime::build_runtime();
/// rt.block_on(async {
///     println!("Running in the configured runtime");
/// });
/// ```
pub fn build_runtime() -> tokio::runtime::Runtime {
    // Install panic hook early so all panics are logged.
    install_panic_hook();

    // Read worker thread count from env, with sensible default.
    let worker_threads = read_worker_thread_count();

    // Build the runtime.
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
}

/// Install a panic hook that logs panics to stderr.
///
/// This ensures panics in async tasks are visible rather than silently
/// vanishing. Called once at startup by [`build_runtime`].
fn install_panic_hook() {
    // Capture the default panic hook to call it after logging.
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |panic_info| {
        // Log the panic to stderr.
        eprintln!("[PANIC] {}", panic_info);
        // Call the original panic hook to preserve standard behavior.
        default_hook(panic_info);
    }));
}

/// Read the desired worker thread count from the environment.
///
/// Reads the `JXCL_WORKER_THREADS` variable and parses it as a positive usize.
/// Falls back to the number of logical CPUs if unset, empty, or unparseable.
/// Panics if the result is 0.
fn read_worker_thread_count() -> usize {
    const ENV_VAR: &str = "JXCL_WORKER_THREADS";

    // Try to read and parse the env var; fall back to CPU count.
    let default_count = num_cpus();
    let worker_threads = env_or_parse::<usize>(ENV_VAR, default_count);

    // Panic if worker threads is 0; a runtime without workers is unusable.
    if worker_threads == 0 {
        panic!(
            "worker thread count must be > 0 (got {} from env var {})",
            worker_threads, ENV_VAR
        );
    }

    worker_threads
}

/// Get the number of logical CPUs.
///
/// Returns the result of [`std::thread::available_parallelism`] if available
/// (Rust 1.59+), or falls back to 1. This avoids adding an external dependency
/// for a simple system query.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize env-var tests to avoid race conditions with other tests
    // and background threads modifying the environment. Note: this lock
    // is NOT used for tests that call build_runtime, since that modifies
    // the global panic hook and can interfere with panics in other tests.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn read_worker_thread_count_default_cpu_count() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("JXCL_WORKER_THREADS");
        let count = read_worker_thread_count();
        // Should be the CPU count (at least 1).
        assert!(count >= 1);
    }

    #[test]
    fn read_worker_thread_count_parses_env_var() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("JXCL_WORKER_THREADS", "4");
        let count = read_worker_thread_count();
        assert_eq!(count, 4);
        std::env::remove_var("JXCL_WORKER_THREADS");
    }

    #[test]
    fn read_worker_thread_count_falls_back_on_invalid_value() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("JXCL_WORKER_THREADS", "not-a-number");
        let count = read_worker_thread_count();
        // Should fall back to CPU count, not panic.
        assert!(count >= 1);
        std::env::remove_var("JXCL_WORKER_THREADS");
    }

    #[test]
    fn read_worker_thread_count_falls_back_on_empty_var() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("JXCL_WORKER_THREADS", "");
        let count = read_worker_thread_count();
        // Empty falls back to default.
        assert!(count >= 1);
        std::env::remove_var("JXCL_WORKER_THREADS");
    }

    #[test]
    #[should_panic(expected = "worker thread count must be > 0")]
    fn read_worker_thread_count_panics_on_zero() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("JXCL_WORKER_THREADS", "0");
        let _count = read_worker_thread_count();
    }

    #[test]
    fn num_cpus_returns_positive() {
        let count = num_cpus();
        assert!(count > 0);
    }
}
