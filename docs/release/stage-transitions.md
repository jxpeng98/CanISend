# Native release stage transitions

[`release/stage-transition-policy.json`](../../release/stage-transition-policy.json) defines the only supported
forward transitions for the active 1.0 Rust-native line: sequential Alpha iteration, Alpha to
Beta, Beta to RC, sequential RC iteration, and RC to Stable. The transition tool
changes current product state without rewriting the immutable Alpha readiness, contract-freeze, feedback, or
package-candidate evidence that explains how the release reached that state.

## Preview first

The command is read-only unless the final `--write` flag is present:

```console
cargo run -p xtask --locked -- release prepare-stage v1.0.0-alpha.6
cargo run -p xtask --locked -- release prepare-stage v1.0.0-beta.1
```

It prints `canisend.stage-transition-plan/v1` JSON containing the source and target stages plus the before/after
SHA-256 digest of every controlled file. Review the complete file set. A transition cannot skip a
stage, change the 1.0 release line, attach build metadata, or skip an iteration. Sequential Alpha
and RC targets must increase exactly by one. Alpha iteration updates Cargo/internal pins/locks,
Tauri/npm/fallback versions, the CLI/GUI parity scope, Alpha package asset names, release workflow
default, active source-version claims, and release-note heading in one plan. It resets stale Beta
readiness, contract-freeze, and feedback identities to canonical pending state for the new Alpha;
Git history retains the prior evidence. Once RC.1 evidence is committed,
`prepare-stage v1.0.0-rc.2` is allowed; RC iteration preserves the
qualification ledger's earlier clean-tag records. Beta same-stage iteration and RC number skipping are rejected.
Any explicit release-notes review is reset during sequential RC iteration: the earlier review still exists in Git
history, but it cannot authorize a candidate whose manifest, assets, issues, or package-channel state may differ.

Before the Alpha-to-Beta write, refresh [`release/beta-readiness.json`](../../release/beta-readiness.json), run the
ordinary release source gate, and complete the name-only signing configuration audit described in
[the signing runbook](signing-operations.md). Write mode rejects a readiness snapshot older than 24 hours or more
than five minutes in the future. Do not put any credential value in the repository or transition plan.

Refresh is also dry-run first:

```console
./scripts/refresh_beta_readiness.sh jxpeng98/CanISend
./scripts/refresh_beta_readiness.sh jxpeng98/CanISend --write
```

The script queries only public issue number/state and public release identity; it never downloads issue titles,
bodies, comments, attachments, or private application data. Any open issue stops the refresh for manual blocker
triage. With none open, the candidate preserves reviewed per-class evidence, updates only audit time/counts, and must
pass `xtask release verify-beta-readiness` before an explicitly requested clean-worktree write.

## Apply intentionally

After the preview is reviewed, rerun it from a clean worktree:

```console
cargo run -p xtask --locked -- release prepare-stage v1.0.0-beta.1 --write
cargo run -p xtask --locked -- release check
git diff --check
```

Write mode transactionally updates the workspace version, exact internal dependency versions,
workspace package entries in `Cargo.lock`, the standalone fuzz workspace's exact dependencies and
lockfile package entries,
qualification-ledger stage/status, and release-note heading as one prevalidated file set. The Stable transition
also publishes the already-reviewed support-policy document and records explicit Stable authorization.
The tool refuses RC without a qualified signed Beta and active feature freeze, and refuses Stable authorization
unless the qualification ledger already proves every other Stable evidence class. It never creates a tag, starts a
workflow, publishes a release, or changes a package-manager repository.

Only the release-note heading changes during a stage transition. The body is deliberately stage-neutral and enforced
by `release/release-notes-policy.json`, so Alpha-only wording cannot leak into Beta, RC, or Stable. This mechanical
guarantee does not replace the policy-required final RC content review against real issues, assets, limitations, and
package-channel status.

## Review RC feedback before Stable

After the final public RC, capture only public issue number/state and release asset/download metadata. The refresher
is dry-run-first and never reads issue titles, bodies, comments, attachments, or private product data:

```console
./scripts/refresh_release_feedback.sh jxpeng98/CanISend v1.0.0-rc.2
./scripts/refresh_release_feedback.sh jxpeng98/CanISend v1.0.0-rc.2 --write
```

The reviewed write changes the feedback snapshot stage to `rc`, generates the measured roadmap block from the same
counts, and changes the next roadmap from `Draft` to `Reviewed`. Maintainers must review candidate priorities and
qualification findings before commit. Only the qualified RC-to-Stable `prepare-stage` transition may atomically
change the snapshot and roadmap markers from `Reviewed` to `Published`; it preserves all issue, download, release,
and engineering-finding evidence bytes.

## Evidence that must remain historical

The following sources intentionally retain earlier version identifiers:

- `release/beta-readiness.json` identifies the public native Alpha used for blocker review;
- `release/beta-contract-freeze.json` binds the Beta contract to the qualified Alpha surface;
- `release/feedback-snapshot.json` records the release actually observed at capture time;
- `packaging/candidates/alpha` preserves nonpublishing candidates generated from exact Alpha
  assets; and
- `release/history/0.7` plus `packaging/candidates/v0.7.0-alpha.1` remain immutable previous-line
  evidence.

Historical 0.7 identifiers must remain only in their archived evidence. Active transition commands
and support guidance must use the current 1.0 line.
