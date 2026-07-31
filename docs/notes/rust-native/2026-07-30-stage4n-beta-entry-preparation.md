# Stage 4N Beta-entry preparation evidence

Date: 2026-07-30

Source version: `1.0.0-alpha.5`

Public checkpoint: `v1.0.0-alpha.4`

Scope: source and operator-runbook preparation only; no Beta write, feature-freeze activation,
tag, package, push, provider request, or release

## Outcome

Stage 4N establishes a truthful handoff from the completed product source to the separately
authorized Beta qualification process. It does not treat source completeness, a local dry-run, or
historical Alpha.4 evidence as qualification for the changed Alpha.5 tree.

The active release documentation previously retained `v0.7.0-beta.1` and `v0.7.0-rc.*` command
examples after the repository authority had moved to 1.0. The support guide also described the
old 0.7 line and excluded GUI operation even though the current product ships an Apple Silicon
macOS desktop application. Those instructions could direct an operator to an invalid transition
or misstate the supported product.

This slice:

- updates stage transition, qualification-ledger, package-manager, upgrade, documentation/uninstall,
  signing, and support runbooks to the active 1.0 release line;
- distinguishes the five standalone CLI targets from the current Apple Silicon desktop boundary;
- preserves `release/history/0.7` and its referenced candidate tree as immutable historical
  evidence;
- adds a release-check contract over seven active runbooks, with current-line examples derived
  from the workspace version; and
- makes stale active `v0.7.0-beta.*` or `v0.7.0-rc.*` examples fail the source gate.

## Read-only Beta checks

The clean source at commit `a37d0ae` passed:

```console
cargo run -p xtask --locked -- release prepare-stage v1.0.0-beta.1
./scripts/audit_community_signing_configuration.sh
./scripts/check_signing_readiness.sh beta
./scripts/refresh_beta_readiness.sh jxpeng98/CanISend
```

The transition dry-run produced `canisend.stage-transition-plan/v1` from
`1.0.0-alpha.5` to `1.0.0-beta.1`, listed 15 controlled files, and reported
`writes_performed: false`. Community signing requires no paid credential and remains fail-closed
on native runners.

The public readiness refresh was also dry-run-only. At `2026-07-30T19:43:04Z` it found zero public
issues, zero open issues, reverified the published `v1.0.0-alpha.4` prerelease identity, and
validated the candidate readiness JSON. It did not replace the checked-in readiness record.

## Remaining authorization boundary

The qualification ledger remains `pre-beta`, workspace stage remains `alpha`, Beta status remains
`pending`, and the feature-freeze baseline remains `null` with no exceptions.

Before any Beta write:

1. run separately consented real-provider Codex and Claude dogfood;
2. qualify and independently verify the exact clean-tag Alpha.5 native candidate;
3. refresh and commit the Beta-readiness record from the reviewed clean checkpoint;
4. explicitly authorize and apply the `v1.0.0-beta.1` stage transition; and
5. build, publish, download, and record the exact signed Beta qualification.

Feature-freeze activation happens only after step 5. It binds the qualified Beta source commit;
it cannot use `a37d0ae`, this documentation slice, or the dry-run transition as a substitute.

## Verification

| Check | Result |
| --- | --- |
| `cargo test -p xtask --locked` | passed: 58 |
| active release runbook contract | passed: 7 runbooks |
| Alpha.5 → Beta.1 transition dry-run | passed: 15 files, no writes |
| community-signing audit and Beta readiness | passed |
| public Beta-readiness refresh dry-run | passed: 0 issues, candidate validated |
| `cargo clippy -p xtask --all-targets --locked -- -D warnings` | passed |
| `cargo run -p xtask --locked -- release check` | passed |
| `cargo fmt --all -- --check` and `git diff --check` | passed |

Native packages, provider calls, Alpha.5 publication, Beta transition write, feature freeze, and
external release actions remain outside this source batch.
