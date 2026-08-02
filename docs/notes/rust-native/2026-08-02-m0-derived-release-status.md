# M0 derived release-status implementation record

Date: 2026-08-02

Roadmap task: `M0-STATE-001`

State: Implemented and source-gate verified in this change. The roadmap task remains Planned until
M0-GOV-001 creates its linked work item and the committed evidence is inspected.

## Defensive invariant

Release status must be a body-free projection over existing machine authorities. It must never
become a manually edited stage or publication record. A source/public, source/package,
source/ledger, operation-contract, or platform-set contradiction fails closed; expected work after
a public checkpoint is reported as drift rather than hidden or rewritten.

## Implemented scope

- `cargo run -p xtask --locked -- release status --json` emits
  `canisend.release-status/v1`.
- The projection reads the Cargo workspace identity, Git HEAD and reachable SemVer tags,
  qualification ledger, support and signing policies, release targets, Alpha package contract,
  CLI/GUI and Svelte parity contracts, Beta readiness/freeze, and feedback snapshot.
- The output distinguishes hard consistency from pending or stage-blocking drift and names the
  authority behind every fact.
- `release check` rebuilds the projection so a hard contradiction also fails the source gate.
- Historical readiness, freeze, feedback, tags, and package evidence remain unchanged.

## Bounded verification

Focused fixtures cover:

- a canonical consistent status;
- source commits ahead of the public checkpoint plus stale stage evidence;
- rejection of a public version newer than source;
- rejection of Cargo/ledger stage disagreement; and
- rejection of support/target platform-count disagreement.

Required commands:

```console
cargo fmt --all -- --check
cargo test -p xtask release_status_ --locked
cargo test -p xtask --locked
cargo run -p xtask --locked -- release status --json
cargo run -p xtask --locked -- release check
```

## Rollback

Revert the status command, its source-gate call, tests, and documentation together. Do not edit
release ledgers, public tags, archived 0.7 evidence, or readiness/freeze snapshots to make the
derived view appear clean.
