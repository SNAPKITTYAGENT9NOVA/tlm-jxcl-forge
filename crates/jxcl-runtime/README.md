# jxcl-runtime

Tokio runtime configuration/startup helpers (worker-thread count from env, panic-hook wiring).

## Architecture

**Owns:** build_runtime.

**Category:** network · **Source:** new

## Purpose

Provides a single, configured entry point for constructing a tokio multi-threaded runtime suitable for the JXCL forge. The runtime is initialized with:

- **Worker thread count:** read from the `JXCL_WORKER_THREADS` environment variable (defaults to the number of logical CPUs if unset or unparseable).
- **Panic hook:** a process-global panic hook that logs panics to stderr, ensuring async task panics do not silently vanish.

## Public API

`build_runtime() -> tokio::runtime::Runtime`

## Implementation Notes

The crate exports a single public function, `build_runtime`, which:

1. Installs a panic hook early in startup using `std::panic::set_hook`, capturing the default panic hook and logging to stderr via `eprintln!` before calling the original hook.

2. Reads the `JXCL_WORKER_THREADS` environment variable using `jxcl_config::env_or_parse`, with a fallback to the number of logical CPUs via `std::thread::available_parallelism` (or 1 if unavailable).

3. Constructs a `tokio::runtime::Builder::new_multi_thread()` with the resolved thread count and `enable_all()` features enabled.

4. Panics if the worker thread count is 0, since a runtime without workers cannot execute any async code.

The implementation follows the patterns established in `jxcl-config` for environment-variable reading and error handling, and uses no unsafe code.

## Dependencies

Workspace crates:

- `jxcl-config`

External crates:

- `tokio` (version 1, with "rt" feature for runtime building)

## Testing

Implemented test kinds: unit (7 tests).

Tests cover:

- `build_runtime_succeeds`: Verifies that a runtime can be built and is usable for simple async tasks.
- `read_worker_thread_count_default_cpu_count`: Checks fallback to CPU count when env var is unset.
- `read_worker_thread_count_parses_env_var`: Validates parsing of an explicit thread count from the env var.
- `read_worker_thread_count_falls_back_on_invalid_value`: Ensures invalid/unparseable values fall back to CPU count.
- `read_worker_thread_count_falls_back_on_empty_var`: Confirms empty env var falls back to default.
- `read_worker_thread_count_panics_on_zero`: Validates that a thread count of 0 panics.
- `panic_hook_is_installed`: Verifies the panic hook machinery works.
- `num_cpus_returns_positive`: Confirms the CPU-count helper returns a positive value.

All env-var tests use a Mutex lock (`ENV_LOCK`) to serialize access and prevent race conditions.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
