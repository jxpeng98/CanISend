# Native release stage transitions

[`release/stage-transition-policy.json`](../../release/stage-transition-policy.json) defines the only supported
forward transitions for the 0.7 Rust-native line: Alpha to Beta, Beta to RC, and RC to Stable. The transition tool
changes current product state without rewriting the immutable Alpha readiness, contract-freeze, feedback, or
package-candidate evidence that explains how the release reached that state.

## Preview first

The command is read-only unless the final `--write` flag is present:

```console
cargo run -p xtask --locked -- release prepare-stage v0.7.0-beta.1
```

It prints `canisend.stage-transition-plan/v1` JSON containing the source and target stages plus the before/after
SHA-256 digest of every controlled file. Review the complete file set. A transition cannot skip a stage, change the
0.7 release line, attach build metadata, or use a target other than the first Beta/RC version.

Before the Alpha-to-Beta write, refresh [`release/beta-readiness.json`](../../release/beta-readiness.json), run the
ordinary release source gate, and complete the name-only signing configuration audit described in
[the signing runbook](signing-operations.md). Do not put any credential value in the repository or transition plan.

## Apply intentionally

After the preview is reviewed, rerun it from a clean worktree:

```console
cargo run -p xtask --locked -- release prepare-stage v0.7.0-beta.1 --write
cargo run -p xtask --locked -- release check
git diff --check
```

Write mode updates the workspace version, exact internal dependency versions, workspace package entries in
`Cargo.lock`, qualification-ledger stage/status, and release-note heading as one prevalidated file set. The Stable
transition also publishes the already-reviewed support-policy document and records explicit Stable authorization.
The tool refuses RC without a qualified signed Beta and active feature freeze, and refuses Stable authorization
unless the qualification ledger already proves every other Stable evidence class. It never creates a tag, starts a
workflow, publishes a release, or changes a package-manager repository.

## Evidence that must remain historical

The following sources intentionally retain earlier version identifiers:

- `release/beta-readiness.json` identifies the public native Alpha used for blocker review;
- `release/beta-contract-freeze.json` binds the Beta contract to the qualified Alpha surface;
- `release/feedback-snapshot.json` records the release actually observed at capture time;
- `packaging/candidates/alpha` preserves nonpublishing candidates generated from exact Alpha assets.

An unrestricted replacement of `0.7.0-alpha.1` would corrupt those records and is not an acceptable transition.
