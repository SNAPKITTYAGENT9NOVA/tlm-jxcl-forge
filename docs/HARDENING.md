# Production Hardening

This covers the `pq-crypto` / `pq-cache` / `photo-cache-service` crates
(the Redis caching demo). See `docs/ISA_SPEC.md` and
`docs/RTL_CONTRACT.md` for the `jxcl` ISA forge, which has no network
surface and a different hardening posture (determinism and fault
handling rather than transport/secrets concerns).

## Threat model for the post-quantum envelope encryption

`pq-crypto` (ML-KEM-768 + HKDF-SHA256 + AES-256-GCM) protects the
**confidentiality of cached values at rest**, against anyone who can
read Redis's storage or memory but does not hold the service's
decapsulation key — including, per ML-KEM's design goal, an attacker
with a cryptographically relevant quantum computer.

It explicitly does **not**:

- **Secure the network path to Redis.** The connection between the
  service and Redis is whatever the `redis` crate's connection string
  gives you (plaintext TCP by default, or `rediss://` for TLS if your
  Redis supports it). Encrypting values before they're sent means a
  network eavesdropper still can't read the *plaintext* photo data even
  over an unencrypted connection, but they can see connection metadata,
  timing, and — if you don't also use `rediss://`/stunnel/a private
  network — could tamper with the TCP stream (Redis itself has no
  built-in transport integrity beyond what `rediss://` provides). Run
  Redis on a private network or behind `rediss://`/mTLS in any real
  deployment.
- **Authenticate who wrote a cache entry.** AES-GCM gives you
  integrity/authenticity of the ciphertext (tampering is detected and
  decryption fails), but that only proves the entry was sealed by
  *someone holding the encapsulation key* — the encapsulation key is
  public by design (that's the point of a KEM), so anyone with network
  access to write to Redis can seal and insert their own valid-looking
  entries. This scheme is for confidentiality at rest, not for
  authenticating cache writers; don't treat a successfully-decrypting
  entry as "verified to have come from this service" in a context where
  that distinction matters.
- **Protect against a compromised process.** If an attacker gets code
  execution in the service itself, they have the decapsulation key (or
  the seed) in memory and everything is moot. This is a data-at-rest
  control, not a runtime sandboxing one.

## Key management

`KeyPair::generate()` makes a fresh, random key pair with no way to
recreate it later. That's fine for a single instance, but it has a real
consequence for horizontal scaling: **two replicas that each call
`generate()` independently cannot decrypt each other's cache entries.**
A read on replica B of a key replica A wrote looks like a permanent miss
(safe — `pq-cache` degrades to a miss rather than erroring — but it
defeats the point of a shared cache).

Fix: set `PQ_KEM_SEED` to the same 64-byte hex-encoded value (128 hex
characters) on every replica that should share a cache. All of them will
derive an identical key pair via `MlKem768::from_seed`
(`KeyPair::from_seed` in `pq-crypto`) and can read each other's entries.
Generate a seed once with, e.g., `openssl rand -hex 64`, and treat it
with the same confidentiality as a private key — anyone with the seed
can decrypt every cache entry sealed under it. Store it as a secret
(secret manager / orchestrator secret), never in source control or a
plain config file checked into git.

If `PQ_KEM_SEED` is unset, the service logs a `warn`-level line
explaining the consequence and proceeds with a random per-process key —
this is a safe default (nothing breaks, the cache just doesn't share
across replicas), not a silent footgun.

## Secrets in configuration

`REDIS_URL` can embed a password (`redis://user:pass@host:port`). The
service never logs it verbatim: `redact_url_credentials` (in
`photo-cache-service`) strips any `user:pass@` userinfo before the
resolved config is logged at startup, replacing it with `***@`. This is
covered by unit tests (`redacts_password_from_redis_url` etc. in
`crates/photo-cache-service/src/lib.rs`) and was verified against a
live, password-protected `redis-server` — see the PR description for
the reproduction command.

`PQ_KEM_SEED` itself is never logged (only *that* it was set).

## Runtime hardening checklist

- [x] **Structured logging** via `tracing`, level controlled by
  `RUST_LOG` (defaults to `info`).
- [x] **Timeouts** on the outbound upstream HTTP fetch (5s) so a hung
  upstream can't hold a request — or, in the cached server, the shared
  cache mutex — open indefinitely.
- [x] **Graceful shutdown** on Ctrl-C/SIGTERM (`axum::serve`'s
  `with_graceful_shutdown`), so in-flight requests finish before exit
  under a process supervisor or container orchestrator.
- [x] **No panics on bad input or upstream/cache failure.** Every
  fallible path returns a typed `AppError` mapped to a specific HTTP
  status (502 upstream, 503 cache, 500 serialization); a corrupted or
  foreign cache entry degrades to a miss rather than erroring or
  panicking (tested against a live Redis in
  `crates/pq-cache/tests/redis_integration.rs`).
- [x] **Config via environment variables** with logged, non-secret
  defaults (`PORT`, `CACHE_TTL_SECONDS`) and redacted secret-bearing
  ones (`REDIS_URL`).
- [x] **Health checks** (`GET /healthz`) — the cached server's variant
  actually pings Redis, so it reflects real dependency health rather
  than just "the process is alive."
- [ ] **Rate limiting / backpressure** on `/photos` is not implemented.
  The upstream API (`jsonplaceholder.typicode.com`) is a public demo
  service with no auth of its own; a production deployment fronting a
  real, rate-limited upstream should add a rate limiter (e.g.
  `tower::limit` or an API gateway) — left out here as out of scope for
  a like-for-like port of the original demo, not an oversight to be
  quietly ignored.
- [ ] **TLS to Redis** is not configured by default (see "Threat model"
  above) — pass a `rediss://` URL and a TLS-capable Redis to get it; the
  `redis` crate supports it, this repo just doesn't assume a specific
  deployment's TLS setup.

## Security review

A security-focused review pass (input validation, crypto/secrets
management, injection, data exposure) was run against the `pq-crypto`,
`pq-cache`, and `photo-cache-service` diff. Findings and how they were
handled:

- Confirmed no static nonce/key reuse in the AES-GCM envelope (every
  `seal()` does a fresh KEM encapsulation, so the derived AES key is
  different every time even before the random nonce is considered).
- Confirmed `Envelope::from_bytes` bounds-checks every length prefix
  against the actual buffer before allocating or slicing, so
  attacker-controlled bytes read back from Redis can't cause an
  out-of-bounds panic or an oversized allocation.
- Confirmed no secret (the `PQ_KEM_SEED` value, the Redis password) is
  ever logged, including on error paths.
- **Fixed**: `AppError`'s HTTP response bodies were forwarding the raw
  `Display` of internal errors (e.g. a `redis` crate connection error,
  potentially naming internal hosts) straight to an unauthenticated
  caller of `/photos` during a transient Redis/upstream outage. Now
  returns a fixed, generic message per status code (mirroring the
  `healthz` endpoint's existing "redis unavailable" pattern); the
  detailed error still goes to the server's own `tracing::error!` log.

`cargo audit` (RustSec advisory database, 1251 advisories) was run
against the full workspace lockfile (230 dependencies): no known
vulnerabilities found.

## CI

`.github/workflows/ci.yml` runs, on every push and pull request, across
the whole workspace: `cargo build`, `cargo test`, `cargo clippy -- -D
warnings`, and `cargo fmt --check`. The `pq-cache` integration tests
spin up a real `redis-server` in the CI runner (via the `redis-server`
apt package) rather than mocking Redis.
