# Project control

## Authority and scope

Accepted ADRs own decisions; `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md`
owns work ordering. `Cargo.toml`, `release/*.json`, `docs/contracts/*.json`, tags and
exact artifacts own version and release facts. Existing implementation plans record
scope, acceptance, evidence and the next action. Correct inconsistent projections;
never rewrite historical release evidence or treat a pending gate as passed.

ADR-RN-0023 sets CLI-first, same-device Agent workflows as the current direction.
Unfinished App work and cross-device work are deferred. Historical Beta.1 and private
Beta.2 qualification retain their original identities and limits in the roadmap.

## Execution and integration

Work directly on the authorized outcome. Ask only for a missing decision that changes
it or an action outside existing authority. Reuse the current branch and plan; use
isolation for conflicting parallel work or risky experiments. Integrate coherent
milestones with meaningful commits and a PR when remote protections require one.
Do not change protections or force-push to avoid their checks.

`.trellis/tasks/`, `.trellis/spec/` and `.trellis/workspace/` are ordinary retained
project documents. Task status, phase activation, dispatch and journals do not control
execution. The old project-local Trellis adapters have been removed; do not run
`trellis init` or `trellis update` unless the owner requests their restoration.
User-global tools and the CLI's shipped Agent Skills are separate from those adapters.

## Verification and release

Follow [quality guidelines](../backend/quality-guidelines.md): one invariant has one
primary test owner. Reuse existing protocol processes, dual-Pack fixtures and package
smokes. Repeat a passing check only after a relevant change or new failure.
`xtask source check` and Fast CI validate development; `xtask release check` retains
full candidate qualification, freeze, dependency and actual Host requirements.

The owner authorized npm testing prereleases independently of full native qualification.
Since 2026-09-08, `release.yml` can build from main and publish macOS ARM64 to `next`
through npm OIDC, then verify registry bytes and a fresh installation. Beta.6 succeeded
in run `34533668709` from `e9ba8ea7364dbad152482db7ba99a45a0273007d`.
See [installation](../../../docs/guides/installation.md) for the current invocation.
Only supplied platforms may be declared; this route does not qualify the full Beta
matrix, publish Cargo/GitHub releases, or renew dependency exceptions.

Fast CI and dependency assurance remain active. GUI checks and archived extended
workflows remain paused under the owner's decision; see
[workflow archive](../../../.github/workflow-archive/README.md) before restoring them.
Keep local checks, protected CI, exact artifact qualification and real-user acceptance
separate. Pending Host acceptance gates its candidate, not independent development.
