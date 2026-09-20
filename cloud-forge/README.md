# cloud-forge

![tests](https://img.shields.io/badge/tests-362%20passing-brightgreen)
![license](https://img.shields.io/badge/license-AGPLv3%20%2F%20Commercial-blue)
![unsafe](https://img.shields.io/badge/unsafe-forbidden%20in%2039%2F39%20crates-brightgreen)
![rust](https://img.shields.io/badge/rust-2021%20edition-orange)
![status](https://img.shields.io/badge/status-phase%2016%20of%2046-yellow)

A from-first-principles cloud-resource substrate: the primitives every
AWS-shaped service (compute, storage, database, messaging, …) would
compose from, built for real before any service-shaped crate existed —
and, as of Phase 11, all four (`cloud-compute`, Phase 8; `cloud-storage`,
Phase 9; `cloud-database`, Phase 10; `cloud-messaging`, Phase 11)
actually composed from them, with Phase 12 (`cloud-orchestration`)
composing two of those services *together* for the first time, Phase 13
building out the IAM surface (`cloud-credentials`, `cloud-session`,
`cloud-policy-document`) `cloud-identity` deferred all the way back in
Phase 1, Phase 14 (`cloud-iam`) composing those three IAM primitives
into a fifth real service, and now Phase 15 giving `cloud-orchestration`
a second cross-service composition — `cloud-compute` + `cloud-messaging`
— with this workspace's first rollback spanning two independent
services. This is a **separate Cargo workspace** from the rest of this
repository's 100-crate `jxcl`/`pq-*` stack and from `verification-forge`
— nothing here depends on either, and neither
depends on this.

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

## What's here: Phase 9, `cloud-storage` — the second composed service

Phase 8's closing note named exactly this: the other three primitive
categories built in Phases 5-7 remain uncomposed. Phase 9 composes the
second — storage — following `cloud-compute`'s own shape:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-storage`](./crates/cloud-storage) | `StorageService`: `create_volume`/`transition_attachment`/`delete_volume`, composing nine existing crates — no new primitive | 13 |

Unlike `cloud-compute`, `StorageService` places nothing onto a
specific AZ — Phase 5 never built a `cloud-storage-capacity`
equivalent to `cloud-capacity`, so a volume's size is tracked as an
account-level `cloud-quota` reservation instead, the same pattern
`cloud-compute` uses for instance count. `delete_volume` adds this
workspace's first cross-primitive gate at the service level: it
refuses unless the volume's `cloud-attachment::AttachmentState` is
`Detached`, checked before any lifecycle reconciliation or quota
release — a real business rule (an in-use volume can't be deleted),
not a restatement of something a single primitive already enforces on
its own. See
[`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for the
full reasoning, including why attachment targets aren't validated
against `cloud-compute`.

## What's here: Phase 10, `cloud-database` — the third composed service

The third of the four service categories gets composed, following
exactly the shape `cloud-compute` and `cloud-storage` established:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-database`](./crates/cloud-database) | `DatabaseService`: `create_database`/`apply_migration`/`create_snapshot`/`expire_snapshots`/`delete_database`, composing ten existing crates — no new primitive | 16 |

`cloud-retention` (Phase 6) finally gets a stateful caller: Phase 6
only computed which snapshots a policy would allow deleting, given a
list; `create_snapshot`/`expire_snapshots` are the first things in
this workspace that actually keep such a list and act on the answer —
resolving `cloud-storage`'s own Phase 9 deferral note. `delete_database`
refuses while any snapshot is still recorded, a cross-cutting gate like
`cloud-storage::delete_volume`'s, but reading this service's *own*
recorded state rather than a different primitive's state machine.
`apply_migration` is a thin pass-through to `MigrationLedger::apply`,
the same pattern `transition_runtime`/`transition_attachment` already
established. See
[`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for the
full reasoning.

## What's here: Phase 11, `cloud-messaging` — the fourth and last composed service

The fourth and final service category gets composed, completing the
set `cloud-compute`, `cloud-storage`, and `cloud-database` began:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-messaging`](./crates/cloud-messaging) | `MessagingService`: `create_queue`/`create_topic`/`subscribe`/`publish`/`enqueue`/`receive`/`delete_message`/`delete_queue`/`delete_topic`, composing eleven existing crates — no new primitive | 24 |

Unlike the other three, `MessagingService` has two top-level resource
kinds instead of one: a queue, which holds messages directly as
`cloud-visibility::MessageLease`s, and a topic, which holds none of its
own — only a fan-out list of subscriber queues via
`cloud-fanout::FanoutRegistry`. `publish` is exactly the composition
`cloud-fanout`'s own Phase 7 doc comment named as deferred: fanning a
message out to every subscriber and giving each one a real
`MessageLease`, so `receive` gets at-least-once redelivery and
dead-lettering for free regardless of whether a message arrived by
direct `enqueue` or by fan-out. `subscribe` refuses to link a queue to
a topic whose required `cloud-delivery::DeliverySemantics` the queue's
own semantics don't satisfy — the same cross-primitive gate shape
`cloud-storage::delete_volume` (Phase 9) used, applied here to a new
operation. `delete_queue`/`delete_topic` each refuse based on their own
recorded state (a non-empty inbox; a remaining subscriber) — the same
own-state gate shape `cloud-database::delete_database` (Phase 10) used,
applied here twice. See
[`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for the
full reasoning, including how `publish` avoids partial fan-out on a
message-id collision.

## What's here: Phase 12, `cloud-orchestration` — the first cross-service composition

Phases 8-11 each composed primitives *within* one service category.
Phase 12 composes two already-real **services** together for the first
time, resolving a deferral `cloud-storage` (Phase 9) stated in its own
words:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-orchestration`](./crates/cloud-orchestration) | `attach_volume`/`detach_volume`: free functions coordinating `ComputeService` and `StorageService` — no new state machine, no new primitive | 8 |

`attach_volume` takes a `&ComputeService` and a `&mut StorageService`
together, checks the target instance exists and isn't `Terminating`/
`Terminated`, and only then drives the volume's attachment transitions
— exactly the "orchestration concern for whatever future layer calls
both services" Phase 9's own doc comment named as deferred. An unknown
or terminated instance leaves the volume's `AttachmentState` completely
untouched. `cloud-orchestration` owns no resource store, quota, policy,
or event log of its own — it borrows both services for the duration of
one call and returns, the same shape `cloud-provisioner` (Phase 2)
already established for a pipeline with nothing to hold between calls.
`detach_volume` deliberately does not consult `compute` at all: force-detaching
a volume must stay possible even from an instance that's since been
terminated. See [`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md)
for the full reasoning, including why this phase, unlike every earlier
pipeline in this workspace, has no rollback path to write.

## What's here: Phase 13, the IAM surface

`cloud-identity`'s own Phase 1 doc comment named this explicitly:
"credentials, sessions, federation, policy documents ... is Phase 13
in the roadmap." Phase 13 builds that surface as three new primitive
crates — the same shape Phases 4-7 each used, primitives without a
service yet:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-credentials`](./crates/cloud-credentials) | `CredentialStore`: at most two `Active` credentials per principal, the real IAM rule AWS itself enforces | 9 |
| [`cloud-session`](./crates/cloud-session) | `SessionStore`: assumed-role and federated sessions, valid only while un-revoked and unexpired | 9 |
| [`cloud-policy-document`](./crates/cloud-policy-document) | `to_json`/`from_json`: a `cloud-policy::Policy` serialized to and parsed from JSON via a hand-rolled, schema-restricted parser | 11 |

`cloud-credentials` models a credential's lifecycle — id, principal,
`Active`/`Inactive` — with no real secret material at all, since secure
secret generation needs randomness this zero-dependency workspace
doesn't pull in; its one real business rule is AWS's own: a principal
may hold at most two `Active` credentials at once, and reactivating a
credential re-checks that same cap. `cloud-session`'s `Session::is_valid`
checks two independent invalidity paths — expiry and explicit revocation
— the same two-axis shape `cloud-visibility::MessageLease` (Phase 7)
already established; `SessionSource` covers both "sessions" and
"federation" from `cloud-identity`'s deferral in one type, since a
federated identity's session is still just a session with a different
origin. `cloud-policy-document` hand-rolls a JSON parser restricted to
exactly its fixed schema (strings, arrays, objects — no numbers,
booleans, or `null`), the same zero-dependency posture `cloud-checksum`
(Phase 5) upheld with hand-rolled CRC-32; `cloud-policy` itself gained
one small `statements()` accessor to make this possible, the same
precedent `cloud-scheduler` set gaining `place_least_loaded_with_capacity`
in Phase 4. See [`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md)
for the full reasoning, including why there's no `cloud-federation`
crate and no IAM service yet.

## What's here: Phase 14, `cloud-iam` — the fifth composed service

Phase 13 built the IAM surface as three disjoint primitives without a
service, the same shape Phases 4-7 used before their own service
phases arrived later. Phase 14 is that service:

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-iam`](./crates/cloud-iam) | `IamService`: `create_credential`/`assume_role`/`federate`/`revoke_session`/`authorize`/`load_policy_document`/`export_policy_document`, composing five existing crates — no new primitive | 12 |

Every composed service so far has called `Policy::authorize` before
mutating, but always to gate *another* service's resource operation.
`assume_role`/`federate` are the first operations anywhere in this
workspace where `cloud-policy` (Phase 1) governs something within the
IAM surface itself: a principal now needs permission to assume a role
or federate in at all, checked before any session is created, leaving
`cloud-session`'s store untouched on denial. `load_policy_document`
replaces the active policy immediately — every subsequent authorization
decision uses it — and a malformed document is a pure no-op, since the
assignment only happens after `cloud-policy-document::from_json`
already succeeded. Unlike every prior composed service, `IamService`
validates no account or region and reserves no quota, and registers no
`Arn`: real IAM is inherently global, and neither a credential nor a
session is region-scoped infrastructure with a canonical external name
— mirroring real IAM's own inconsistent resource model (a role has an
ARN; an access key does not), not a gap left to fill later. See
[`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for the
full reasoning.

## What's here: Phase 15, `cloud-orchestration`'s second composition

Phase 12 gave `cloud-orchestration` its first cross-service
composition, resolving `cloud-storage`'s own Phase 9 deferral. Phase
15 gives it a second, in the same crate, resolving a deferral
`cloud-messaging` itself named in its own Phase 11 closing note: "what
remains deferred is composing these services *together* (e.g. a
compute instance's logs delivered through a queue)."

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-orchestration`](./crates/cloud-orchestration) | Adds `transition_runtime_and_notify`/`terminate_and_notify` (`cloud-compute` + `cloud-messaging`) alongside Phase 12's `attach_volume`/`detach_volume` | 13 |

Both new functions enqueue a notification message into `cloud-messaging`
*before* attempting the compute mutation, then delete that message if
the mutation fails — unlike Phase 12's `attach_volume`, which needed no
rollback at all (its second step was unconditionally valid once the
first succeeded), a `RuntimeState` transition genuinely can fail, and
`RuntimeState` transitions aren't generally reversible the way
`cloud-attachment`'s are. This is the first rollback anywhere in this
workspace that spans two independent services rather than undoing
steps within one. See
[`docs/CLOUD_ARCHITECTURE.md`](./docs/CLOUD_ARCHITECTURE.md) for the
full reasoning, including why this composition still has no message
body, no notification on other operations, and no fan-out.

## What's here: Phase 16, `cloud-orchestration`'s third composition

Phase 15 composed `cloud-compute` with `cloud-messaging`. Phase 16
adds a third composition in the same crate: `cloud-database` +
`cloud-messaging`. [`apply_migration_and_notify`] and
[`delete_database_and_notify`] follow the same enqueue-first,
rollback-on-failure discipline as Phase 15, since database operations
like schema migrations can fail (an invalid version jump) and are
typically non-reversible (schema changes and deletions are one-way).

| Crate | Owns | Tests |
|---|---|---:|
| [`cloud-orchestration`](./crates/cloud-orchestration) | Adds `apply_migration_and_notify`/`delete_database_and_notify` alongside Phase 12's attach/detach and Phase 15's compute+messaging | 23 |

**367 tests pass** (`cargo test --workspace --release` from this
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
- **No `cloud-storage-capacity` crate exists yet** — no per-AZ
  byte-capacity primitive to feed it; `cloud-storage` (Phase 9) tracks
  volume size as an account-level `cloud-quota` reservation instead,
  same as `cloud-compute` does for instance count.
- **`cloud-orchestration` only covers attach/detach, not database or
  messaging cross-service composition** — this phase resolves exactly
  the one deferral `cloud-storage` stated; wiring a database or a queue
  to a compute instance is a different cross-service question with no
  equivalent stated deferral yet.
- **`cloud-storage` still has no snapshots or `cloud-checksum`/
  `cloud-retention` integration** — a volume has no content-integrity
  check or backup lifecycle; `cloud-database` (Phase 10) is where a
  snapshot concept first exists, since a database's point-in-time
  backups made more sense there than on a raw block volume.
- **No erasure-coding encoder/decoder** — `cloud-redundancy` models
  the arithmetic a real erasure code guarantees, not the encoding
  itself.
- **No `cloud-table`/`cloud-index`/`cloud-query` crates, no query
  language, and no execution/storage engine in `cloud-database`** —
  `apply_migration` records that a version was applied without any
  idea what SQL or transformation it represents, exactly as
  `cloud-migration` itself (Phase 6) does not run migrations.
- **`create_snapshot` does not use `cloud-checksum`** — a snapshot is
  just an id and a creation time; content-integrity checking would
  need the snapshot to hold real bytes, which this phase's model
  doesn't have.
- **`cloud-messaging` has no message body, and no per-message quota** —
  a message is just a `ResourceId`, exactly as a `cloud-database`
  snapshot (Phase 10) was just an id and a creation time; `"queues"`
  and `"topics"` are quota'd at creation, but messages flowing through
  them are not, the same pattern Phase 10 used for snapshots.
- **No message ordering (FIFO) guarantee** — `cloud-messaging` uses
  `BTreeMap`'s key order for deterministic tests and listings, not as a
  delivery-order promise.
- **`cloud-compute` has no instance resize, no attached volumes, and
  no HTTP/gRPC API surface** — `terminate` releases exactly what
  `launch` reserved; nothing about an instance's lifecycle beyond
  launch/run/stop/terminate is modeled yet, and `control-api` remains
  deferred from Phase 2's own reasoning.
- **All four originally-planned service categories are now composed,
  one cross-service composition (`cloud-compute` + `cloud-messaging`)
  was real as of Phase 15, and a second
  (`cloud-database` + `cloud-messaging`) is real as of Phase 16.**
  `cloud-database`/`cloud-compute` cross-service composition remains
  an open question with no equivalent stated deferral yet.
- **`cloud-orchestration`'s notifications carry no message body, fire
  on no operation besides a runtime transition or termination, and
  fan out to exactly one queue** — extending any of these needs a
  concrete motivating case, not speculation ahead of one.
- **`cloud-iam` links no credential to any session.** A long-term
  credential and a short-term session are tracked independently;
  exchanging one for the other the way real STS `GetSessionToken` does
  needs secret material `cloud-credentials` doesn't have (see below).
- **No policy versioning or multiple named policies in `cloud-iam`.**
  `load_policy_document` replaces the single active policy outright —
  no history, no policy identifiers, no per-principal policy
  attachment.
- **No real cryptographic secret material in `cloud-credentials`** — no
  secret bytes, signing, or verification; secure secret generation
  needs randomness this zero-dependency workspace doesn't pull in.
- **No `cloud-federation` crate** — federation is one `SessionSource`
  variant in `cloud-session`, not a fourth crate, since a federated
  identity's session is still just a session with a different origin.
- **`cloud-policy-document` doesn't support AWS's bare-string-or-array
  polymorphism** for `Principal`/`Action`/`Resource` — one shape per
  field (always an array, or the literal `"*"` for `Principal`) is all
  this phase needs.
- **No resource-based policies** — a policy document attached to a
  specific resource rather than a principal remains the Phase 13 (IAM)
  concept `cloud-resource`'s own Phase 1 doc comment named as not
  pulled forward into `Resource<T>`.
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
- `cloud-storage`'s
  [`delete_volume_refuses_an_attached_volume`](./crates/cloud-storage/src/lib.rs)
  — a different kind of composition test: not rollback, but a service
  reading one primitive's current state (`cloud-attachment`) to gate
  an operation on a different resource store entirely, and proving the
  refusal leaves the earlier quota reservation untouched.
- `cloud-database`'s
  [`expire_snapshots_removes_only_expired_ones_beyond_the_floor`](./crates/cloud-database/src/lib.rs)
  — proves `cloud-retention`'s pure `eligible_for_deletion` computation
  and a service's real, mutated snapshot list agree: the floor
  protects the newest snapshot even though the older one is well past
  its age window, and only the actually-expired one is removed from
  the store.
- `cloud-messaging`'s
  [`publish_with_a_colliding_message_id_leaves_no_partial_fanout`](./crates/cloud-messaging/src/lib.rs)
  — proves the validate-before-apply discipline holds across a fan-out
  to multiple subscribers, not just a single resource's creation: a
  message id already present in one subscriber's inbox rejects the
  whole `publish` call, and the *other* subscriber, which had room,
  still never receives it.
- `cloud-orchestration`'s
  [`attach_volume_rejects_a_terminated_instance`](./crates/cloud-orchestration/src/lib.rs)
  — the first test in this workspace of a composition spanning two
  independent services: it terminates a real `ComputeService` instance,
  then proves `attach_volume` refuses to touch a `StorageService`
  volume's `AttachmentState` at all on account of the *other* service's
  state, leaving it exactly `Detached`.
- `cloud-policy-document`'s
  [`multiple_statements_with_a_one_of_principal_round_trip`](./crates/cloud-policy-document/src/lib.rs)
  — builds a real `cloud-policy::Policy` with an `Allow`-all statement
  and a `Deny` scoped to specific principals, serializes it through the
  hand-rolled `to_json`, parses it back with `from_json`, and asserts
  full equality with the original — proving the parser and serializer
  agree on every field this schema has, not just the fields easiest to
  test.
- `cloud-iam`'s
  [`exported_policy_document_round_trips_through_load`](./crates/cloud-iam/src/lib.rs)
  — exports one service's active policy to JSON, loads it into a
  *second*, independently-created `IamService` that started out
  denying everything, and proves `assume_role` now succeeds there too —
  showing `cloud-policy-document`'s round trip and `cloud-iam`'s
  policy-gated session creation compose correctly together, not just
  each in isolation.
- `cloud-orchestration`'s
  [`transition_runtime_and_notify_rolls_back_the_message_on_an_invalid_transition`](./crates/cloud-orchestration/src/lib.rs)
  — attempts an invalid `RuntimeState` jump through the composed
  function, then proves the notification message it enqueued into a
  real `MessagingService` queue moments earlier is gone: `receive`
  on that exact message id comes back `NotFound`, showing the
  rollback actually reached across into the other service's own
  store rather than just returning an error.

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
