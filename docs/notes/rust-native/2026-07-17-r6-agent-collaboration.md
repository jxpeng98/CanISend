# R6 Agent Collaboration Implementation Note

**Date:** 2026-07-17

**Phase:** R6

**Status:** Complete

## Boundary delivered

R6 makes CanISend useful as a durable local state owner paired with Codex, Claude, or a generic agent host. The host
receives body-free context and bounded task descriptors, reads private content only after explicit consent, creates a
candidate outside internal state, and returns it through a versioned JSON completion contract. Only the CanISend
binary reads or writes `.canisend/`.

This is an agent collaboration boundary, not an autonomous application-submission system. Readiness and task success
never authorize sending an application.

## Context and capability registry

- `agent capabilities` is generated from compiled capability, stage, and discovery-adapter registries.
- `agent context --job` derives workspace/job counts, blockers, and next actions from SQLite without reading source
  bodies.
- A regression imports a private advert containing a sentinel phrase and proves the serialized context omits it.
- `task.lifecycle` is advertised as available; later workflow stages remain planned until their own phases pass.

## Task lifecycle

SQLite migration 4 extends tasks with their subject job revision, operation, actor/mode, allowed output, candidate
schema, unique lease, frozen descriptor, and completion/cancellation timestamps. `task prepare` freezes every current
normalized source artifact revision and SHA-256 plus the job revision under a 15-minute lease.

`task complete` accepts at most 4 MiB from stdin or a regular non-symlink `.json` file. It performs public JSON Schema
validation before typed semantic validation. It then rechecks the lease, job revision, exact task inputs, and current
artifact heads inside an immediate SQLite transaction. A successful candidate is canonicalized and committed as an
immutable artifact with dependencies, task result, task state, blob reference, and audit event in one authoritative
transaction. Identical replay returns the existing artifact; a different replay conflicts.

Lease expiry, cancellation, a newer artifact head, or a changed job revision cannot silently commit. Expired or
changed work becomes durably stale and returns prepare-again remediation. Validation failures leave the task prepared
and return stable violations with JSON pointers.

## Consent and external work directory

Prepared descriptors request `read-private-inputs` for an exact artifact scope. `task inputs` refuses to create an
export unless `--allow-private-read` explicitly confirms that consent. The successful path writes only declared
artifacts and `canisend-task-inputs.json` to a new or empty external directory with private Unix permissions. It also
records the consent-manifest digest and an audit event.

Candidate creation and correction therefore occur outside `.canisend/`; agent hosts do not need direct database or
blob access.

## Host packs

`agent assets export` creates separate Codex, Claude, and generic packs. Each contains:

- `AGENTS.md`, `CLAUDE.md`, or `README.md`;
- the bounded criterion prompt;
- a completion example;
- criterion, task-descriptor, and task-completion schemas;
- `canisend-agent-pack.json` with product/protocol/resource versions and per-file ID, version, size, and SHA-256.

Exports require a new or empty non-symlink directory outside `.canisend/` and refuse overwrites. All assets are
compiled into the native executable and covered by the existing resource integrity gate.

## Verification

Local acceptance passed:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- 54 Rust tests across contracts, CLI, IO, resources, store, and core;
- deterministic checks for 20 public schemas and 26 embedded resources;
- `cargo build --release --locked`;
- `scripts/smoke_host_agent.sh` against the release binary.

The packaged smoke exports a Codex pack, proves consent refusal creates no input directory, exports scoped inputs,
returns semantic remediation for an invalid external candidate, commits and replays a valid candidate, makes a second
task stale by importing another source revision, and finishes with a healthy workspace. The same smoke is wired into
normal CI and the Linux, macOS, and Windows native preview matrix.

GitHub Actions run `29618142639` repeated the full clean-checkout gate, release build, and packaged host-agent smoke
in 1 minute 39 seconds. R6 is accepted and R7 is active.
