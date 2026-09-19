# Production Hardening

This covers the `pq-crypto` / `pq-cache` / `photo-cache-service` /
`pq-sql-vault` / `pq-error-proof` crates (the Redis caching demo and its
post-quantum, SQL-backed, and attestation extensions). See
`docs/ISA_SPEC.md` and `docs/RTL_CONTRACT.md` for the `jxcl` ISA forge,
which has no network surface and a different hardening posture
(determinism and fault handling rather than transport/secrets
concerns).

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
  deployment. `server-cached` enforces `rediss://` when `APP_ENV=production`
  (see "Runtime hardening checklist" below) so this isn't just a
  recommendation left to the deployer to remember.
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

## Key rotation and envelope versioning

`pq-crypto::KeyRing` (and `pq-cache`/`pq-sql-vault`'s use of it) supports
rotating the active key without breaking previously-sealed values:

- Every `Envelope` is stamped with the `key_version` that sealed it (the
  wire format's leading `u32`), so `open()` can find the right key even
  after the active version has moved on.
- Each `KeyRing` entry has a lifecycle: `Active` (seals new values and
  opens old ones), `DecryptOnly` (no longer used to seal, but still
  opens values sealed under it — the rotation's grace period), and
  `Retired` (refuses to open at all; `Error::RetiredKeyVersion`, not a
  silent decryption failure, so an operator can tell "deliberately
  retired" apart from "corrupt entry").
- `KeyRing::set_active(new_version)` promotes one version and demotes
  any other `Active` entry to `DecryptOnly` — a rotation never retires
  the old key immediately, since doing so instantly would turn every
  not-yet-expired cache entry sealed under it into an unreadable miss.
  Retire the old version explicitly once its grace period (at least the
  cache's TTL) has passed.
- `Error::UnknownKeyVersion` is returned for an envelope whose
  `key_version` isn't registered in the ring at all — distinct from
  `RetiredKeyVersion` — which matters operationally: the former usually
  means a config/deployment mismatch (a replica missing a key another
  replica has), the latter means rotation is working as intended.

This is exercised end-to-end in `pq-crypto`'s
`rotation_keeps_old_entries_readable_until_retired` test: seal under v1,
rotate to v2, confirm both versions are still readable during the grace
period, retire v1, confirm v1 now fails closed while v2 is unaffected.

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
- [x] **TLS to Redis is enforced in production.** `APP_ENV=production`
  makes `server-cached` refuse to start unless `REDIS_URL` uses the
  `rediss://` scheme (`check_redis_tls_requirement` in
  `photo-cache-service`, checked and logged before any connection is
  attempted, exiting the process with a clear error rather than silently
  running over plaintext). Outside production (`APP_ENV` unset or any
  other value) plaintext `redis://` is still allowed, since local/dev
  Redis instances rarely have TLS configured. Verified live against the
  release binary: `APP_ENV=production REDIS_URL=redis://...` logs the
  refusal and exits `1` before touching the network.

## SQL Server vault (`pq-sql-vault`)

An alternative backing store to Redis: `pq-sql-vault::SqlVault` connects
to SQL Server (via `tiberius`, a pure-Rust TDS client — no ODBC/native
driver needed) and stores only `pq-crypto`-sealed ciphertext, never
plaintext or key material. It reuses `pq-crypto::KeyRing` for the same
rotation semantics as `pq-cache` (see "Key rotation" above).

Design principle: **policy is enforced in T-SQL, not just in the Rust
client.** A client bug or a compromised application identity should not
be able to violate the schema's invariants:

- `sql/001_schema.sql` defines `pq.KeyVersion`, `pq.EncryptedObject`, and
  `pq.AuditLog`, with a **filtered unique index**
  (`UX_KeyVersion_OneActive ... WHERE Status = N'Active'`) that makes it
  impossible for more than one key version to be `Active` at the
  database level — not just an application-level convention.
- `sql/002_procedures.sql` is the only write path
  (`sp_upsert_encrypted_object`, `sp_register_key_version`,
  `sp_set_active_key_version`, `sp_retire_key_version`,
  `sp_purge_expired_objects`): `sp_upsert_encrypted_object` validates the
  target key version isn't `Retired` before the `MERGE`, and
  `sp_retire_key_version` refuses to retire the currently `Active`
  version, both via `THROW` rather than leaving it to the caller to
  check first.
- `sql/003_row_level_security.sql` and `sql/004_signed_procedures.sql`
  are optional, opt-in hardening for multi-tenant deployments: row-level
  security predicates scoping which rows a given database principal can
  see, and certificate-signed procedures so an application login can be
  granted `EXECUTE` on the procedures without being granted direct table
  access (least privilege).
- `ExpiresAtUnixSeconds` is a plain `BIGINT`, not `DATETIME2`, so the
  Rust side never needs `tiberius`'s `chrono`/`time` feature just to
  read a TTL back — one less dependency surface.

**Not live-tested.** Unlike the Redis integration tests (run against a
real `redis-server`), `pq-sql-vault` was **not** exercised against a
live SQL Server in this environment — starting one requires a container
runtime, and starting the Docker daemon itself was denied by this
sandbox's containment policy. `crates/pq-sql-vault/tests/live_vault.rs`
holds two `#[ignore]`d integration tests, gated on
`PQ_SQL_VAULT_TEST_CONNECTION_STRING`, that exercise a set/get roundtrip
and the same rotation grace-period scenario as `pq-crypto`'s unit test.
Run them explicitly against a real SQL Server before relying on this
crate in production:

```sh
PQ_SQL_VAULT_TEST_CONNECTION_STRING="Server=tcp:host,1433;Database=db;User Id=sa;Password=...;TrustServerCertificate=true;" \
    cargo test -p pq-sql-vault --test live_vault -- --ignored --nocapture
```

The crate's unit tests (connection-string building and the
`redact_ado_connection_string` password redaction, verified to be
case-insensitive on the `password=` key and correct when no password is
present) do run in normal CI, same as everything else.

## Verifiable error attestations (`pq-error-proof`)

The original ask here was an FFI error-handling layer verified "down to
circuit level with circom." Circom's toolchain lives on GitHub, which
this environment's egress policy blocks outright (confirmed: a plain
HTTPS `HEAD` to `github.com` came back `403` through the proxy). The
substitute, `pq-error-proof`, delivers the same underlying request —
provable/verifiable error states — with a real Groth16 zk-SNARK circuit
built entirely from crates.io via `arkworks` (BLS12-381 with the Jubjub
curve embedded in its scalar field, the same construction family used by
Zcash Sapling), authored directly in Rust rather than compiled from
circom's DSL.

**What it proves:** that a service which published an error-attestation
commitment for a specific, publicly-known error context (error code, key
version, envelope id, etc.) really held a secret "opening randomness"
for that commitment — a Pedersen-commitment knowledge-of-opening proof —
without revealing that randomness. Concretely: `commitment = message*G +
randomness*H` over Jubjub, where `message` is a public value anyone can
recompute (`derive_message`, SHA-256 of the error context) and
`randomness` is a private witness. `Params::generate` runs the (local,
single-party) Groth16 setup; `attest` produces a commitment + proof for
an error context; `verify` checks a commitment + proof against a
recomputed `message`.

**What it does *not* prove:** that the underlying error actually
happened inside `pq-crypto`/`pq-cache` — e.g. it is not a circuit that
models AES-GCM tag verification. That would require encoding the entire
AEAD/KEM decryption path as an R1CS circuit, a substantially larger
undertaking left out of scope. What's delivered is a narrower,
honestly-scoped non-repudiation building block: a publicly-checkable,
forgeable-only-by-the-secret-holder receipt for an error event, not a
proof about the failure's internal mechanics.

**Setup is a local secret, not a public ceremony.** Unlike a real
multi-party trusted setup (e.g. Zcash's), `Params::generate` is a
single-party operation — whoever runs it can, in principle, forge
proofs. This is fine for an internal attestation system where one
organization controls both the attesting service and the verifier,
generates `Params` once, and distributes its serialized bytes
(`Params::to_bytes`/`from_bytes`) to both; it must not be presented as,
or relied on as, a trustless/public-verifiability guarantee.

All 9 of the crate's tests are genuine cryptographic assertions run
against real proof generation/verification (no mocks): a full
attest-then-verify roundtrip, rejection of a mismatched error context,
rejection across two independent `Params` setups, rejection of a
tampered commitment, `Params`/`Attestation` byte-serialization
roundtrips (including truncated-input rejection), and unlinkability of
two attestations over the same error context (fresh randomness produces
different commitments each time).

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

`cargo audit` (RustSec advisory database) was run against the full
workspace lockfile (354 packages after adding `pq-sql-vault` and
`pq-error-proof`). It first found 3 real vulnerabilities
(RUSTSEC-2026-0098/0099/0104: name-constraint and CRL-parsing bugs in
`rustls-webpki 0.101.7`), all pulled in transitively by `tiberius`'s
`rustls` feature — directly relevant to `pq-sql-vault` since that stack
is exactly what would negotiate TLS to SQL Server. **Fixed**: switched
`pq-sql-vault`'s `tiberius` dependency from the `rustls` feature to
`native-tls` (wraps the system TLS library instead of vendoring an
outdated pure-Rust one), which drops that dependency chain entirely; see
the comment on the `tiberius` line in `crates/pq-sql-vault/Cargo.toml`.
Re-running `cargo audit` afterward found zero vulnerabilities. One
unmaintained-crate *warning* remains (`derivative`, RUSTSEC-2024-0388,
pulled in by `ark-crypto-primitives`) — not a vulnerability, and fixing
it would mean patching arkworks itself, so it's accepted and documented
rather than silently ignored.

## CI

`.github/workflows/ci.yml` runs, on every push and pull request, across
the whole workspace: `cargo build`, `cargo test`, `cargo clippy -- -D
warnings`, and `cargo fmt --check`. The `pq-cache` integration tests
spin up a real `redis-server` in the CI runner (via the `redis-server`
apt package) rather than mocking Redis. The runner also installs
`libssl-dev` for `pq-sql-vault`'s `tiberius` dependency, which links
against the system OpenSSL (see "Security review" above for why
`native-tls` was chosen over `rustls` here).
