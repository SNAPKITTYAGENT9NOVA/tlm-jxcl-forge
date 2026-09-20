# cloud-forge

![tests](https://img.shields.io/badge/tests-94%20passing-brightgreen)
![license](https://img.shields.io/badge/license-AGPLv3%20%2F%20Commercial-blue)
![unsafe](https://img.shields.io/badge/unsafe-forbidden%20in%2012%2F12%20crates-brightgreen)
![rust](https://img.shields.io/badge/rust-2021%20edition-orange)
![status](https://img.shields.io/badge/status-phase%201%20of%2046-yellow)

A from-first-principles cloud-resource substrate: the primitives every
AWS-shaped service (compute, storage, database, messaging, …) would
compose from, built for real before any service-shaped crate exists.
This is a **separate Cargo workspace** from the rest of this
repository's 100-crate `jxcl`/`pq-*` stack and from `verification-forge`
— nothing here depends on either, and neither depends on this.

> **The non-negotiable rule:** do not create one crate per AWS service.
> Build the primitives once, then compose services from those
> primitives. See [`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md)
> for the full reasoning, the layering, and what's deliberately not
> built yet.

## What's here: Phase 1, the primitive kernel

This workspace currently implements **Phase 1** of a much larger
roadmap (a control plane, a service layer, and eventual AWS-shaped
services like compute/storage/database come later, in that order — see
the architecture doc's phase table). Phase 1 is the resource model
every later phase composes from:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-errors`](./crates/cloud-errors) | `CloudError`, the structured error every crate returns | 5 |
| [`cloud-types`](./crates/cloud-types) | `ResourceId`, `ResourceType`, `AccountId`, `RegionId`, `AzId`, `Arn`, `Timestamp` | 27 |
| [`cloud-tags`](./crates/cloud-tags) | A validated tag map: length limits, a reserved key prefix, a max tag count | 9 |
| [`cloud-lifecycle`](./crates/cloud-lifecycle) | The `Creating→Active→Updating→Deleting→Deleted`/`Failed` state machine | 8 |
| [`cloud-resource`](./crates/cloud-resource) | `Resource<T>`: id/type/region/owner/lifecycle/version/tags/timestamps + payload `T` | 7 |
| [`cloud-region`](./crates/cloud-region) | A region → availability-zone registry | 5 |
| [`cloud-account`](./crates/cloud-account) | `Account` + a registry rejecting duplicate account ids | 5 |
| [`cloud-identity`](./crates/cloud-identity) | A minimal `Principal` (`User`/`Role`/`Service`) | 5 |
| [`cloud-policy`](./crates/cloud-policy) | Allow/Deny evaluation; **explicit deny always dominates** | 9 |
| [`cloud-events`](./crates/cloud-events) | An append-only, strictly-ordered event log | 6 |
| [`cloud-quota`](./crates/cloud-quota) | Limit/usage tracking; a rejected reservation commits nothing | 7 |
| [`cloud-core`](./crates/cloud-core) | A facade re-exporting all of the above, plus an integration test | 1 |

**94 tests pass** (`cargo test --workspace --release` from this
directory), all `cargo clippy --workspace --all-targets -- -D
warnings` clean, all `cargo fmt --all -- --check` clean.

## Deliberately not in Phase 1

- **`cloud-metadata`** isn't a separate crate — its proposed fields are
  exactly `cloud-resource`'s `created_at`/`updated_at`/`version` and
  `cloud-tags`'s `Tags`.
- **`cloud-runtime`** is deferred to Phase 2 (the control plane) — a
  "runtime" has nothing real to run until a scheduler or reconciler
  exists to run something on.
- **The full IAM surface** (credentials, sessions, federation, JSON
  policy documents) is Phase 13 — `cloud-identity` only has enough of a
  `Principal` for `cloud-policy` to evaluate against.
- **No AWS-named crate exists anywhere in this workspace.** `services/`
  cannot start honestly until the primitives it would compose (Phase
  2's control plane, then compute/storage/network primitives) are
  themselves real.

## See it compose

`cloud-core`'s own test,
[`provisioning_a_resource_composes_every_phase_1_primitive`](./crates/cloud-core/src/lib.rs),
is the best single read for how these primitives fit together: it
registers a region and an account, authors a policy (and proves an
explicit deny beats a broader allow), creates a tagged resource and
walks it through its lifecycle while checking the version invariant at
every step, formats its internal ARN, records a creation event, and
hits a quota limit on a second reservation — all using only the
crates in the table above.

## Building and testing

```
cd cloud-forge
cargo build --workspace
cargo test --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Per-crate, e.g.:

```
cargo test -p cloud-policy --release
```

This workspace is independent of the root `Cargo.toml` and of
`verification-forge/` — building or testing either of those does not
touch `cloud-forge`, and vice versa. See the [repository root
README](../README.md) for how this fits into the rest of the project,
and [`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for
the full roadmap beyond Phase 1.

## License

This workspace is dual-licensed, identically to the repository root:
the GNU Affero General Public License v3.0
([`LICENSE-AGPL`](./LICENSE-AGPL)), or a separately negotiated
commercial license ([`LICENSE-COMMERCIAL`](./LICENSE-COMMERCIAL), a
non-binding draft template). See [`LICENSE-NOTICE`](./LICENSE-NOTICE),
[`COPYRIGHT.md`](./COPYRIGHT.md), and [`TRADEMARKS.md`](./TRADEMARKS.md)
for the copyright holder, licensing contacts, and trademark terms. This
workspace carries its own copy of the license set so it can be
distributed independently of the root repository.
