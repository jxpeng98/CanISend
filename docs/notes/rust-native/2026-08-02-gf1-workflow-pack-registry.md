# GF1 workflow-pack verified registry implementation record

**Date:** 2026-08-02

**Roadmap task:** GF1-REG-001 registry foundation; partial foundation for GF1-TRUST-001 and
M1F-PACK-001

**State:** Implemented and source-gate verified in this change. Persistent Workspace binding,
explicit external installation, publisher trust, and linked work-item inspection remain required
before the full roadmap task becomes Verified.

## Implemented boundary

- Added a pure `canisend-core` verified-bundle boundary that consumes the existing typed
  `canisend.workflow-pack/v1` manifest instead of introducing another format.
- Bound a manifest to the exact declared resource set, sizes, individual SHA-256 values, and a
  domain-separated canonical bundle digest.
- Added current kernel, Agent, and Workspace semantic-version compatibility checks.
- Added a kernel-owned registry for intake-adapter, renderer, and validator capability IDs. Pack
  data can select registered identifiers but cannot add executable behavior.
- Added deterministic `canisend.workflow-pack-snapshot/v1` values containing pack identity,
  origin, content digest, canonical manifest hash, and a sorted resource inventory.
- Added an in-memory registry keyed by exact pack ID and version. Resolution also requires the
  expected digest; no latest-version or implicit-upgrade API exists.
- Made identical registration idempotent while rejecting same-version substitution and defensive
  same-digest/different-content collisions.

This boundary accepts already-read manifest JSON and bounded resource bytes. It performs no file,
database, process, credential, or network access and does not enable external Pack installation.

## Digest and snapshot invariant

The bundle digest uses the `canisend.workflow-pack-bundle/v1\0` domain, a recursively key-sorted
compact manifest with `content_digest` zeroed, length-prefixed tagged segments, and resources in
ascending portable path order. Exact resource bytes are hashed; declared paths, sizes, individual
hashes, and the aggregate digest must all agree.

The registry never resolves only by ID or by an inferred newest version. A stored Application can
therefore bind to `(pack ID, version, content digest)` without a same-version replacement or a new
version silently changing its workflow.

## Defensive tests

- valid external bundle and deterministic snapshot;
- resource-byte tampering and aggregate-digest mismatch;
- missing and undeclared resources;
- incompatible kernel version and unavailable capability;
- repeated registration, exact digest resolution, and multiple-version coexistence;
- same-version substitution rejection; and
- digest stability across manifest object-key serialization order.

## Verification

```console
cargo test -p canisend-core workflow_pack --locked
cargo clippy -p canisend-core --all-targets --all-features --locked -- -D warnings
cargo test -p canisend-contracts -p canisend-core -p canisend-resources --locked
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- release check
```

## Remaining boundary

GF1-REG-001 is not a license to load arbitrary directories. The GF1-TRUST byte verifier now bounds
already-supplied Manifest/resource bytes and reports their data-only status, but a later app/storage
slice must still add a symlink-safe regular-file reader, explicit user install/update action,
trusted-source policy, atomic immutable storage, persisted Application snapshot binding, and
reopen verification. Pack migration remains separately user-approved and backup-backed. The
additive GF1-DAG-001 compiler consumes a verified Manifest's Pack-qualified stages, but Workspace
v3 graph execution remains disabled until the neutral persistence boundary exists.

## Rollback

Revert the core module, dependency-lock update, contract clarification, implementation record, and
roadmap evidence row together. Existing Agent v2, Workspace v2, fixed workflow, and stored data are
unchanged because no current runtime path consumes the new registry.
