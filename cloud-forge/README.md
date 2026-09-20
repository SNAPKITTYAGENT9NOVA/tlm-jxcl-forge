# cloud-forge

![tests](https://img.shields.io/badge/tests-254%20passing-brightgreen)
![license](https://img.shields.io/badge/license-AGPLv3%20%2F%20Commercial-blue)
![unsafe](https://img.shields.io/badge/unsafe-forbidden%20in%2031%2F31%20crates-brightgreen)
![rust](https://img.shields.io/badge/rust-2021%20edition-orange)
![status](https://img.shields.io/badge/status-phase%208%20of%2046-yellow)

A from-first-principles cloud-resource substrate: the primitives every
AWS-shaped service (compute, storage, database, messaging, …) would
compose from, built for real before any service-shaped crate existed —
and, as of Phase 8, the first one (`cloud-compute`) actually composed
from them. This is a **separate Cargo workspace** from the rest of this
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

## What's here: Phase 2, the control plane

Phase 2 adds the pipeline that turns Phase 1's independent data types
into an actual "create a resource" operation — authorized, quota
checked, placed, constructed, and recorded, atomically:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-scheduler`](./crates/cloud-scheduler) | Least-loaded-AZ placement over a region's registered AZs (Phase 4 adds a capacity-aware variant) | 11 |
| [`cloud-service-registry`](./crates/cloud-service-registry) | Named control-plane services and their health (`Healthy`/`Unhealthy`/`Unknown`) | 7 |
| [`cloud-reconciler`](./crates/cloud-reconciler) | BFS over `cloud-lifecycle`'s transition graph: shortest path from a current to a desired state | 7 |
| [`cloud-provisioner`](./crates/cloud-provisioner) | The `AUTHORIZE→VALIDATE→PLAN→APPLY→VERIFY→AUDIT` pipeline; **rolls back every earlier stage's reservation if a later stage fails** | 6 |
| [`cloud-control-plane`](./crates/cloud-control-plane) | `ControlPlane<T>`: registries + policy + quota + events + a resource store, with real `create`/`get`/`list`/`delete` | 8 |

## What's here: Phase 3, AWS-like resource identification

Phase 3 gives every provisioned resource a canonical, globally
resolvable name — an `Arn` — and a registry that resolves it back to
the resource it names:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-resource-registry`](./crates/cloud-resource-registry) | An `Arn` ↔ `ResourceId` registry: rejects a second registration of an already-used ARN, and a resource claiming a second ARN | 7 |

The roadmap names four crates for this phase
(`cloud-arn`, `cloud-resource-id`, `cloud-resource-parser`,
`cloud-resource-registry`); three of the four already existed since
Phase 1 as `cloud_types::Arn`, `cloud_types::ResourceId`, and `Arn`'s
own `FromStr` impl — see
[`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for why.
`cloud-control-plane::create()` now computes and registers each new
resource's `Arn` right after provisioning succeeds, with the same
rollback discipline as every other stage: a failed ARN registration
releases the quota and placement reservations `cloud-provisioner`
already made. `resolve_arn()`/`arn_of()` expose the mapping in both
directions, and a resource's ARN stays resolvable even after it's
deleted (consistent with the CLOUD-I003 no-id-reuse invariant).

## What's here: Phase 4, compute primitives

Phase 4 builds the three primitives a compute-shaped resource needs
that nothing before it required: something to track whether it's
actually running, a pool of capacity to run it on, and something to
boot it from.

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-runtime`](./crates/cloud-runtime) | `RuntimeState`: the execution-state machine (`Pending`/`Running`/`Stopping`/`Stopped`/`Terminating`/`Terminated`) — deliberately separate from `cloud-lifecycle::Lifecycle` | 9 |
| [`cloud-capacity`](./crates/cloud-capacity) | Per-AZ vCPU/memory-MiB capacity; reservations fail closed against an unregistered AZ, unlike `cloud-quota`'s default-unlimited caps | 12 |
| [`cloud-image`](./crates/cloud-image) | A registry of machine images to launch from: validated id/name/size/architecture, immutable once registered | 7 |

`cloud-scheduler` also gains `place_least_loaded_with_capacity`,
composing `cloud-capacity` into placement so a resource is never
assigned to an AZ without room for it — the one real composition this
phase adds, kept generic (any resource type can ask for
capacity-aware placement) rather than folded into a compute-specific
crate that doesn't exist yet. See
[`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for why
`cloud-runtime` and `cloud-lifecycle` are two state machines rather
than one, and why `cloud-provisioner`/`cloud-control-plane` are
untouched by this phase.

## What's here: Phase 5, storage primitives

Compute (Phase 4) needed something to execute, a pool to run it in,
and something to boot it from; storage needs an entirely different
set of concepts, built with the same discipline:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-checksum`](./crates/cloud-checksum) | `Checksum`: a from-scratch CRC-32/ISO-HDLC content-integrity checksum, streaming or one-shot — anchored to the algorithm's own published check value, not just internal self-consistency | 8 |
| [`cloud-redundancy`](./crates/cloud-redundancy) | `RedundancyScheme`: replication or erasure coding, shard counts, reconstruction threshold, max tolerable loss | 9 |
| [`cloud-attachment`](./crates/cloud-attachment) | `AttachmentState`: the attach/detach state machine for a volume-like resource — a third state machine alongside `Lifecycle` and `RuntimeState`, with **no terminal state** (every state has a path back to `Detached`) | 9 |

`cloud-checksum` has zero dependencies, like every crate in this
workspace — CRC-32/ISO-HDLC is implemented directly rather than
pulled in from crates.io. See
[`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for why
that matters here specifically, why `AttachmentState` is a genuinely
different state machine from `Lifecycle`/`RuntimeState` rather than a
rename, and why no storage-capacity or storage-service crate exists
yet.

## What's here: Phase 6, database primitives

Compute needed execution state, capacity, and images (Phase 4);
storage needed integrity, durability, and attachment state (Phase 5);
database-shaped resources need a third, disjoint set of concepts:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-consistency`](./crates/cloud-consistency) | `ConsistencyLevel` (`Eventual`/`BoundedStaleness`/`Strong`): a total order (via `#[derive(Ord)]`, not a hand-rolled comparison) plus `satisfies()` against a caller's minimum requirement | 7 |
| [`cloud-migration`](./crates/cloud-migration) | `MigrationLedger`: enforces that schema migrations apply strictly sequentially and without gaps, starting at version 1 | 7 |
| [`cloud-retention`](./crates/cloud-retention) | `RetentionPolicy`: which backups/snapshots are eligible for deletion by age, with a floor that protects the most recent N regardless of age | 8 |

`cloud-migration` and `cloud-retention` deliberately don't depend on
each other — schema shape over time and data lifetime are unrelated
questions. See
[`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for why,
and for why no `cloud-database`, query engine, or storage engine
exists yet.

## What's here: Phase 7, messaging primitives

Compute, storage, and database (Phases 4-6) each needed their own
disjoint set of concepts; messaging-shaped resources need a fourth —
the last of the four service categories this README names before
AWS-shaped service composition begins:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-delivery`](./crates/cloud-delivery) | `DeliverySemantics` (`AtMostOnce`/`AtLeastOnce`/`ExactlyOnce`): a genuine **partial order** (two independent axes — loss, duplication), not a total one like `cloud-consistency` | 6 |
| [`cloud-visibility`](./crates/cloud-visibility) | `MessageLease`: the visibility-timeout mechanism that actually implements at-least-once delivery, plus a dead-letter threshold on receive count | 8 |
| [`cloud-fanout`](./crates/cloud-fanout) | `FanoutRegistry`: the topic-to-subscriber pub/sub topology | 8 |

`AtMostOnce` and `AtLeastOnce` are **incomparable** — neither
satisfies the other, since each permits something the other forbids —
which is why `cloud-delivery` checks both axes explicitly instead of
deriving `Ord` the way `cloud-consistency` (Phase 6) correctly does
for its own, genuinely total, order. See
[`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for the
full reasoning, and for why no `cloud-queue`/`cloud-topic` or FIFO
ordering primitive exists yet.

## What's here: Phase 8, `cloud-compute` — the first real composed service

Phases 4-7 built sixteen primitive crates and, at every single one,
deferred composing them into anything. Phase 8 ends that deferral for
compute, the category with the most complete primitive set:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-compute`](./crates/cloud-compute) | `ComputeService`: `launch`/`transition_runtime`/`terminate`, composing eleven existing crates — no new primitive | 13 |

`ComputeService` deliberately does **not** reuse
`cloud-provisioner`/`cloud-control-plane` (Phase 2): those pipelines
place with `cloud_scheduler::place_least_loaded`, which knows nothing
about vCPU/memory, and teaching a generic pipeline about compute
capacity would be exactly the one-off special-casing this workspace's
rule exists to prevent. Instead `cloud-compute` runs its own
`AUTHORIZE -> VALIDATE -> PLAN -> APPLY`-shaped pipeline directly
against `cloud-capacity`/`cloud-image`/`cloud-runtime`, with the same
rollback discipline: a capacity-exhaustion failure at `PLAN` rolls back
the `VALIDATE`-stage quota reservation, tested directly. `RuntimeState`
lives in its own store, separate from `Resource<InstanceSpec>` — never
re-coupling what `cloud-runtime` (Phase 4) deliberately kept
independent. See
[`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for the
full reasoning and for why storage/database/messaging remain
primitives without a composed service for now.

**254 tests pass** (`cargo test --workspace --release` from this
directory), all `cargo clippy --workspace --all-targets -- -D
warnings` clean, all `cargo fmt --all -- --check` clean.

## Deliberately skipped or deferred

- **`cloud-metadata`** isn't a separate crate — its proposed fields are
  exactly `cloud-resource`'s `created_at`/`updated_at`/`version` and
  `cloud-tags`'s `Tags`.
- **`resource-manager`, `lifecycle-manager`, `quota-manager`,
  `policy-engine`, `account-manager`, `region-manager` are not separate
  crates** — each would be a thin wrapper forwarding to a Phase 1
  primitive, exactly the one-crate-per-name anti-pattern the
  non-negotiable rule exists to prevent. `cloud-control-plane` holds
  those primitives directly.
- **`control-api`** (a REST/gRPC layer) is deferred — the roadmap's own
  execution order puts the API layer well after the control plane it
  would expose.
- **Image deregistration is not implemented** — whether an in-use
  image can be safely removed depends on resource-to-image references
  `cloud-image` alone cannot see.
- **No `cloud-storage-capacity`, object/blob-storage, or `cloud-volume`
  crate exists yet** — no composition, or the primitive it would need,
  exists until a real storage-provisioning phase needs it (`cloud-compute`,
  Phase 8, is the one service category this reasoning no longer applies
  to).
- **No erasure-coding encoder/decoder** — `cloud-redundancy` models
  the arithmetic a real erasure code guarantees, not the encoding
  itself.
- **No `cloud-database` (or similarly named) crate, and no
  `cloud-table`/`cloud-index`/`cloud-query` crates.** Same reasoning as
  storage above — no composition until it's real.
- **No query language, execution engine, or storage engine** — this
  phase answers what guarantee a database offers and what data may be
  discarded, not how data is stored or queried.
- **`cloud-migration` does not run migrations** — only the
  version-ordering invariant; executing a migration's actual contents
  belongs to whatever future service calls the ledger.
- **No `cloud-queue`/`cloud-topic` (or similarly named) crate.** Same
  reasoning as storage/database above — no composition
  until it's real.
- **`cloud-fanout` does not deliver anything** — only the
  topic-to-subscriber mapping; fanning a message out (and tracking
  each subscriber's own `MessageLease`) belongs to a future service.
- **No message ordering (FIFO) primitive** — ordering interacts with
  partitioning and fanout in ways that need a concrete queue/topic
  shape to be meaningful.
- **`cloud-compute` has no instance resize, no attached volumes, and
  no HTTP/gRPC API surface** — `terminate` releases exactly what
  `launch` reserved; nothing about an instance's lifecycle beyond
  launch/run/stop/terminate is modeled yet, and `control-api` remains
  deferred from Phase 2's own reasoning.
- **The full IAM surface** (credentials, sessions, federation, JSON
  policy documents) is Phase 13 — `cloud-identity` only has enough of a
  `Principal` for `cloud-policy` to evaluate against.
- **No AWS-named crate exists anywhere in this workspace.** `services/`
  cannot start honestly until the primitives it would compose (compute,
  storage, network — later phases) are themselves real.

## See it compose

Two tests are the best reads for how these primitives fit together:

- `cloud-core`'s
  [`provisioning_a_resource_composes_every_phase_1_primitive`](./crates/cloud-core/src/lib.rs)
  — registers a region and an account, authors a policy (and proves an
  explicit deny beats a broader allow), creates a tagged resource and
  walks it through its lifecycle while checking the version invariant
  at every step, formats its internal ARN, records a creation event,
  and hits a quota limit on a second reservation, all using only Phase
  1 primitives directly.
- `cloud-provisioner`'s
  [`apply_failure_rolls_back_both_quota_and_placement`](./crates/cloud-provisioner/src/lib.rs)
  — the sharpest single test of Phase 2's actual claim: a pipeline
  built from atomic parts is not itself atomic unless someone wires
  the rollback, and this proves it's wired, not just asserted.
- `cloud-compute`'s
  [`exhausting_capacity_fails_the_launch_and_reserves_no_quota`](./crates/cloud-compute/src/lib.rs)
  — the same rollback claim one layer up, now across eleven composed
  crates instead of `cloud-provisioner`'s five: a capacity-exhaustion
  failure at `PLAN` still rolls back the `VALIDATE`-stage quota
  reservation, even though neither of those stages is `cloud-provisioner`'s
  own code anymore.

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
