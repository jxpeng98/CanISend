# R8.4 guarded package readiness note

## Outcome

R8.4 adds a deterministic package boundary after Review. No agent or configured provider computes readiness. The
Rust core freezes the current application plan, evidence catalog, profile revision, document set, every member
document, and the current review findings before it writes a `PackageManifestRecord`.

The public surface is:

```text
canisend package check --job JOB_ID --json
canisend package show --job JOB_ID --json
```

`check` is idempotent while every frozen input remains current. `show` returns the body-free manifest and typed
reason records; it does not return draft bodies.

## Readiness model

The core produces three active states:

- `blocked` when a deterministic or revision-integrity reason exists;
- `needs-review` when only open human-review findings remain;
- `ready-to-export` when no unresolved reason remains.

Reason records have a closed enum code plus optional artifact, document-kind, or finding identity. They have no
message or prose field. Current codes cover missing/stale documents, document-plan mismatch, review-set mismatch,
open deterministic findings, pending human findings, and mixed plan, evidence, or profile revisions.

`exported` remains reserved for the R8.5 projection lifecycle.

## Revision and storage boundary

SQLite migration 11 adds `package_heads`, a derived current-head projection tied to one workflow run. The manifest
artifact records exact dependency rows for the plan, match set, evidence catalog, document set, every structured
document, and review findings. The manifest also records the exact profile revision consumed by the evidence chain.

Package heads and artifacts are invalidated by profile, evidence, plan, document, review, job, or workflow rerun
changes. A repeated check returns the same manifest artifact while its complete input context is unchanged.

## Render and submission guard

Completing Package does not automatically open Render. The workflow gate reads the exact current `package_heads`
row and opens Render only for `ready-to-export` or the later `exported` state. Blocked and needs-review packages
produce stable body-free workflow blockers.

The manifest contains `submission_performed: false`, and semantic validation rejects `true`. This is an explicit
contract assertion: readiness can authorize file projection, but it cannot authorize, perform, or claim an
application submission.

## Agent-host integration

`package.lifecycle` and the Package stage are now marked available. Workflow status points hosts to `package check`.
The Codex, Claude, and generic guides explain the readiness boundary, and their self-contained pack now includes the
package-manifest schema. The pack contains 26 files.

## Verification

Local verification passed:

- formatting and full-workspace Clippy with warnings denied;
- 66 Rust tests, including the two loopback transport tests;
- 35 generated public-schema checks and 45 embedded-resource checks;
- package contract validation that structurally rejects free-form readiness prose and submission claims;
- store integration coverage for exact input freezing, deterministic blockers, accepted human-risk disposition,
  idempotent package checks, Render gating, and upstream invalidation;
- locked release compilation and packaged host-agent smoke.

The preceding R8.3 remote checkpoint also passed GitHub Actions run `29625243713`.
