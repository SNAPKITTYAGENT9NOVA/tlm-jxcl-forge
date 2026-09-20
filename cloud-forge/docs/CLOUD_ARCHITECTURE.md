# cloud-forge architecture

## The non-negotiable rule

> Do not create one crate per AWS service. Build the primitives once,
> then compose services from those primitives.

`cloud-forge` is a from-first-principles attempt at the substrate
underneath an AWS-shaped cloud platform: resource identity, lifecycle,
ownership, policy, tagging, events, and quota — the handful of concepts
that every cloud service (compute, storage, database, messaging, …)
reuses rather than reinvents. No crate in this workspace is named after
an AWS product, and none will be until it is a thin composition of
already-real primitive crates, not a container for logic that belongs
in one of them.

This mandate carries the same weight as the root workspace's "100
crates, none of them fake" rule (see the repository root
[`docs/CRATE_ARCHITECTURE.md`](../../docs/CRATE_ARCHITECTURE.md)) and
`verification-forge`'s hard invariants: a crate exists here only when
it owns a real, independently-testable invariant.

## Why a third workspace

`tlm-jxcl-forge` already separates two independent Cargo workspaces:
the root workspace (the TLM JXCL ISA, post-quantum crypto/storage, one
reference service) and `verification-forge` (a from-scratch proof
kernel). Neither is a natural home for cloud-resource modeling:

- The root workspace's `docs/crates.toml` registry is machine-checked
  at exactly 100 crates with its own dependency-DAG rules
  (`tools/check_workspace.py`); folding an open-ended, many-phase cloud
  platform into it would either blow past that validated count or
  force cloud crates to misrepresent themselves as ISA/crypto crates.
- `verification-forge` is a general-purpose proof kernel with no
  cloud-specific concept in it at all — it will eventually *verify*
  invariants about `cloud-forge` (see "Relationship to
  verification-forge" below), which means it must stay independent of
  it, not host it.

So `cloud-forge/` is a third, independent top-level Cargo workspace,
with its own `Cargo.toml`, `crates/`, and quality gates — exactly the
same shape as `verification-forge/`.

## Layering

```mermaid
flowchart TB
    subgraph existing["Already real, in the root workspace"]
        isa["jxcl* — ISA, execution engine"]
        pq["pq* — PQ crypto, storage, ZK proofs"]
    end

    subgraph kernel["cloud-forge Phase 1: the primitive kernel (this pass)"]
        types["cloud-types<br/>ResourceId, Arn, AccountId, RegionId"]
        errors["cloud-errors<br/>shared CloudError"]
        tags["cloud-tags"]
        lifecycle["cloud-lifecycle<br/>state machine"]
        resource["cloud-resource<br/>Resource&lt;T&gt; wrapper"]
        region["cloud-region"]
        account["cloud-account"]
        identity["cloud-identity<br/>Principal (minimal)"]
        policy["cloud-policy<br/>Allow/Deny, deny dominates"]
        events["cloud-events"]
        quota["cloud-quota"]
        core["cloud-core<br/>facade"]

        types --> errors
        errors --> tags
        errors --> lifecycle
        types --> resource
        tags --> resource
        lifecycle --> resource
        types --> region
        types --> account
        types --> identity
        identity --> policy
        types --> policy
        errors --> policy
        types --> events
        types --> quota
        resource --> core
        region --> core
        account --> core
        identity --> core
        policy --> core
        events --> core
        quota --> core
    end

    subgraph future["Planned, later phases — not built yet"]
        control["Phase 2: control plane<br/>(provisioning, reconciliation, scheduler)"]
        services["Phase 4+: service layer<br/>(compute, storage, database, …)"]
    end

    existing -.underlies.-> kernel
    kernel --> future
```

Every arrow above is a real `[dependencies]` edge in a crate's own
`Cargo.toml` — there is no crate in this phase that depends on
something not yet built, and no crate that exists only to be depended
on later without doing anything itself today.

## The resource model (Phase 0 / Phase 1)

Every cloud resource, regardless of which future service creates it,
carries the same fields — this is the one model every service composes
from rather than redefining:

| Field | Type | Owning crate |
|---|---|---|
| `id` | `ResourceId` | `cloud-types` |
| `resource_type` | `ResourceType` | `cloud-types` |
| `region` | `RegionId` | `cloud-types` / `cloud-region` |
| `account` | `AccountId` | `cloud-types` / `cloud-account` |
| `owner` | `AccountId` | `cloud-types` |
| `lifecycle` | `Lifecycle` | `cloud-lifecycle` |
| `version` | `u64` | `cloud-resource` |
| `tags` | `Tags` | `cloud-tags` |
| `created_at` / `updated_at` | `u64` (unix millis) | `cloud-resource` |

`owner` here means *account ownership* (who a resource bills to and
belongs to) — distinct from `cloud-identity::Principal`, which
`cloud-policy` evaluates access for. A resource-based policy attaching
a `Principal` to a specific resource is a Phase 13 (IAM) concept, not
pulled forward into this phase's `Resource<T>`.

`cloud-resource::Resource<T>` combines all of these plus a
service-specific payload `T`, so a future `Instance` or `Bucket` type
is `Resource<InstanceSpec>` / `Resource<BucketSpec>`, not a
hand-rolled struct that has to remember to include `tags` and
`lifecycle` correctly on its own.

The internal ARN format `cloud-types::Arn` implements is exactly the
one specified in the roadmap:

```
jxcl:cloud:<partition>:<service>:<region>:<account>:<resource>
```

## What Phase 1 deliberately does not include

- **`cloud-metadata` is not a separate crate.** Its proposed fields
  (`created_at`/`updated_at`/tags/version) are exactly `cloud-resource`
  and `cloud-tags`'s fields; a separate crate would own nothing a
  caller couldn't already get from those two.
- **`cloud-runtime` is deferred to Phase 2.** A "runtime" is only
  meaningful once there's a scheduler or reconciler to run something
  on; before that exists, a `cloud-runtime` crate would have no real
  invariant to test — it would be a name, not a primitive. It's listed
  as a Phase 2 (control-plane) item instead.
- **`cloud-identity` is intentionally minimal in this phase**: a
  `Principal` enum (`User`/`Role`/`Service`, each carrying an id) —
  enough for `cloud-policy` to evaluate against. The full IAM surface
  (credentials, sessions, federation, policy documents-as-JSON) is
  Phase 13 in the roadmap and is not pulled forward.
- **No AWS-named crate exists anywhere in this phase.** `services/ec2`,
  `services/s3`, etc. cannot be started honestly until the primitives
  they'd compose (compute, storage, network primitives — later phases)
  are themselves real.

## Definition of done, per crate

Reusing the roadmap's own maturity levels, scoped to what a primitive
crate (not a service) can claim:

- **L1 (data model)** — the types exist, are validated at construction,
  and round-trip through their own `Display`/`FromStr` (or equivalent).
- **L2 (deterministic local implementation)** — the crate's actual
  logic (a state-machine transition, a policy evaluation, a quota
  check) is implemented and produces the same result for the same
  input every time, with no hidden clock/randomness/iteration-order
  dependency.
- **L8 (tests)** — every invariant claimed above has a passing test
  that would fail if the invariant were violated (not just a
  compiles-and-returns-something test).

Every crate in this phase targets L1+L2+L8. Persistence (L5),
distribution (L4), and formal verification (L9/L10, eventually via
`verification-forge`) are explicitly out of scope until the control
plane (Phase 2) gives them something real to attach to.

## Relationship to `verification-forge`

Per the roadmap: `verification-forge` should eventually verify
`cloud-forge`'s invariants (resource-id uniqueness, deny-dominates
policy evaluation, lifecycle transitions being total functions, quota
violations never committing), not become a runtime dependency of it.
That work is not started in this phase — it requires `cloud-forge`'s
own resource/lifecycle/policy model to exist and stabilize first, which
is exactly what this phase builds.

## Quality gates

Same discipline as the other two workspaces in this repository: no
crate is considered done until `cargo test -p <crate> --release`
passes, `cargo clippy -p <crate> --all-targets -- -D warnings` is
clean, and `cargo fmt -p <crate> -- --check` is clean — checked after
every crate, not batched at the end. `#![forbid(unsafe_code)]` in every
crate. No `unwrap()`/`expect()` on anything derived from a caller's
input -- a malformed resource id or an invalid policy input is exactly
the kind of thing this substrate has to reject cleanly (a `CloudError`)
rather than panic on. The only `expect()` calls in this phase
(`cloud-region`'s AZ-name construction, `cloud-types::AzId::region()`)
are on values built from a *already-validated* piece of the same type's
own state -- provably unreachable to fail, and each carries a comment
saying so -- never on anything a caller supplied that hasn't already
gone through that type's constructor.
