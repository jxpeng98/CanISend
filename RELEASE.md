# CanISend 1.0 release policy

## Current state

- Checked-in source: `1.0.0-beta.2`. This source is not a published checkpoint.
- Latest public checkpoint: [`v1.0.0-beta.1`](https://github.com/jxpeng98/CanISend/releases/tag/v1.0.0-beta.1),
  built once from `6e1397b79031cad54e794ccdc9edca2153f23b3e`, independently reverified, and
  recorded as qualified against candidate run `33281162734`.
- Local package-channel candidates: deterministic Homebrew, Scoop, and WinGet review files derived
  from the exact qualified public archives, with publication explicitly unauthorized.
- Release-path ordering after qualified Beta.1: feature freeze is active, the private Beta.2
  source is prepared, and one nonpublishing native candidate has passed independent verification
  at protected source `2ae2b507b953eef3101aa9689bd60f91a0046605`. Beta.2 remains untagged,
  unpublished, and unqualified; the exact identities are retained in the
  [private candidate note](docs/notes/rust-native/2026-09-04-beta2-private-candidate.md). The
  consented body-free invited cohort is the next evidence gate before RC.1. The active step is
  derived from the qualification ledger and the
  [1.0 Roadmap](docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md). The ledger retains the
  qualified Beta.1 artifact source `6e1397b79031cad54e794ccdc9edca2153f23b3e`.
- License for current CanISend-authored source and future releases: `GPL-3.0-only`. Historical tags
  retain their original license facts.
- Machine stage: Beta / `beta-qualifying`; feature-freeze status is independently recorded in
  `release/qualification-ledger.json`. RC and Stable remain unauthorized until their gates pass.

PyPI and TestPyPI are not release channels for the Rust product. A source build, local GUI preview,
or manually dispatched candidate is not a published release.

## Supported public package scope

Public 1.0 checkpoints contain standalone CLI archives for:

- `aarch64-apple-darwin`;
- `x86_64-apple-darwin`;
- `x86_64-unknown-linux-gnu`;
- `x86_64-unknown-linux-musl`; and
- `x86_64-pc-windows-msvc`.

The public desktop package is Apple Silicon macOS ZIP and DMG. Intel macOS, Windows, and Linux
desktop builds are nonpublishing qualification candidates and do not broaden the support claim.

## Source and candidate gates

Use the smallest verification tier that proves a change. The complete source gate is:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --locked
cargo run -p xtask --locked -- dependencies check
cargo deny check advisories bans licenses sources
cargo run -p xtask --locked -- release status --json
cargo run -p xtask --locked -- release check
```

`release status --json` is a read-only projection. It reports expected source-ahead or stale-stage
evidence separately from hard contradictions and does not authorize publication.

The release workflow builds a future tag once as a nonpublishing candidate. The exact five CLI
archives, macOS ZIP/DMG, manifest, checksums, SBOM, notices, qualification records, and GitHub build
provenance must all bind one reviewed commit. Lifecycle, migration, accessibility, and Agent
evidence are collected against those exact candidate bytes. Promotion later reuses the cached
candidate without recompilation. GitHub build provenance remains part of the exact release unit.

## Community signing and integrity

Community signing is not a publicly trusted publisher identity; it provides artifact-integrity
evidence only. Alpha macOS applications use ad-hoc signatures. Where required by the stage policy,
standalone macOS executables use ad-hoc signatures and Windows uses an ephemeral self-signed
Authenticode certificate. These controls do not provide Developer ID, Apple notarization, a public
timestamp, or SmartScreen reputation.

Users must verify `SHA256SUMS`, the exact release manifest, signing evidence, and GitHub build
provenance. Gatekeeper, Unknown Publisher, or SmartScreen warnings may still occur. Never disable
an operating-system security control globally; use only normal per-application approval after
independent artifact verification. See the
[signing operations guide](docs/release/signing-operations.md).

## Stage gates

- **Alpha.6:** Pack v1, both built-in Pack digests, Agent/Workspace v3 migration, dual-Pack
  semantic parity, retained Academic v2 compatibility, five CLI targets, and Apple Silicon GUI
  must pass exact source, native, lifecycle, accessibility, Agent, integrity, and public-download
  verification.
- **Alpha.7:** Published Workspace v4, Agent v4, new Skills/resources, both Packs in one Workspace,
  connected intake, App/CLI/MCP parity, headless operation, and unsupported-legacy no-mutation
  passed the exact-package gates.
- **Alpha.8:** Published the approved bootstrap usability fixes and repaired sequential-Alpha/Beta
  authority on the same clean v4 boundary. Exact public bytes and provider hosts passed
  independent verification; invited-user evidence remains pending.
- **Alpha.9:** Published the architecture checkpoint with the Store-to-IO production edge removed.
  Exact public bytes, both Packs, render/recovery coverage, and provider hosts passed independent
  verification; invited-user evidence remains pending.
- **Beta:** readiness must be fresh from qualified public Alpha.10 within 24 hours; Workspace v4,
  Agent v4, Skills/resource, operation, approval, and both Pack digests must freeze; and the
  signed/integrity matrix must pass. User cohort evidence starts on public Beta.1 before RC.1.
- **Release candidate:** the feature freeze is active, the current RC is recorded before preparing
  another RC, the Beta-to-RC upgrade and package-manager matrices pass, and final notes/feedback
  bind the latest recorded RC.
- **Stable:** two distinct qualified RC matrices, lifecycle and documentation evidence, reviewed
  support policy, latest-RC feedback, and explicit Stable authorization are complete.

Publication uses an annotated `vVERSION` tag only after the candidate is qualified. A manual
workflow dispatch never publishes. The tag promotion must locate the same unexpired candidate and
must not rebuild product bytes.

## Consumer verification and feedback

Follow the [release verification guide](docs/guides/release-verification.md) before extraction.
CanISend has no default telemetry. Public feedback collection uses only sanitized Issue metadata
and release asset counts; never attach a Workspace, backup, application body, provider payload,
private path, token, certificate, or credential.
