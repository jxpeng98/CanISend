# Generate Beta.1 package-channel candidates

## Goal

Generate deterministic Homebrew, Scoop, and WinGet review candidates from the exact qualified
public `v1.0.0-beta.1` bytes. The checked-in output must describe the current domain-neutral
framework, remain explicitly candidate-only, and create no external package-channel mutation.

## Confirmed background

- `M4-LEDGER-001` / Issue #75 is Verified through protected PR #205 and merge
  `43dc80b0fb5e3accc602795c8e3b706e0bce8fea`.
- Qualified public `v1.0.0-beta.1` binds source
  `6e1397b79031cad54e794ccdc9edca2153f23b3e`, candidate run `33281162734`, and manifest SHA-256
  `2435c335f2edd31e1a59afd4065380112f4e24924f68f76a26be84acef0041f8`.
- ADR-RN-0010 and the existing `xtask release channels` command already own verified-byte
  derivation, deterministic rendering, output validation, and nonpublication semantics.
- The current renderer still emits the historical academic-job description and tag. That wording
  contradicts ADR-RN-0018 and the active generic-framework product boundary for 1.0, while the
  checked-in 0.7 candidate history must remain byte-identical.

## Requirements

### R1 — Correct the current package description at its owning renderer

- Preserve exact pre-`1.0.0-alpha.6` historical candidate rendering.
- Render `1.0.0-alpha.6` and later candidates with domain-neutral package metadata:
  `Prepare evidence-bound applications and submissions locally`, plus general application,
  Agent, and CLI tags.
- Reuse the existing version-aware renderer and one existing focused test; add no schema,
  dependency, alternate generator, compatibility adapter, or packaging framework.

### R2 — Revalidate the exact qualified public source

- Use the already independent public Beta.1 download when it remains available and exact;
  otherwise download the same 20 public release assets into a fresh temporary directory.
- Re-run `xtask release verify` before generation. Require exact tag, source, manifest digest,
  checksums, artifact set, signing records, and native archive identities.
- Do not rebuild artifacts or repeat provider dogfood, native matrices, or attestation work already
  bound by the qualified ledger.

### R3 — Generate one bounded candidate tree

- Require `packaging/candidates/v1.0.0-beta.1` not to exist before generation.
- Run the existing generator once and accept exactly `candidate-source.json`, one Homebrew Cask,
  one Scoop manifest, and three WinGet manifests.
- Require `candidate_only: true`, `publication_authorized: false`, exact Beta tag/source/manifest,
  and the exact Apple Silicon, Intel macOS, and Windows archive names, sizes, and SHA-256 values.
- Do not manually edit generated URLs, digests, nested executable paths, version, license, or
  publication fields.

### R4 — Reconcile bounded project truth

- Add one dated body-free evidence note and update README, RELEASE, the Master Roadmap, Trellis
  project control, parent/current task state, and Issue #76 through protected review.
- Mark package-channel candidates complete only after the generated tree and source gate pass.
- Keep feature-freeze activation, invited-user evidence, RC, Stable, native lifecycle
  qualification, and external index publication pending.

### R5 — Use minimum-sufficient verification

- Run Rust format, the existing focused channel-rendering regression, affected xtask Clippy, one
  final `release check`, `git diff --check`, Trellis validation, and protected Fast CI.
- Do not run the local full workspace suite, rebuild native assets, execute package-manager
  lifecycle matrices, or run extended assurance.

## Acceptance criteria

- [x] Historical 0.7 candidates still regenerate byte-identically, while 1.0 Beta candidates use
      domain-neutral description and tags with GPL-3.0-only metadata.
- [x] The exact public Beta.1 release directory passes the existing verifier before generation.
- [x] The new candidate source binds exact tag `v1.0.0-beta.1`, source `6e1397b...`, manifest
      `2435c335...`, and the three required native archives.
- [x] The generated six-file tree revalidates exactly and cannot authorize publication.
- [x] No external package index, release, tag, qualification ledger, feature-freeze state, or
      public artifact changes.
- [x] Focused checks, final source gate, and protected Fast CI pass on the exact PR head before
      Issue #76 becomes Verified.

## Out of scope

- Homebrew tap, Scoop bucket, winget-pkgs, or any other external repository mutation.
- Native install, upgrade, uninstall, notarization, Authenticode trust, or package-manager policy
  qualification owned by later release gates.
- Feature-freeze activation, cohort evidence, RC, Stable, product behavior, or historical candidate
  rewrites.

## Blocking open questions

None. The accepted ADRs, exact qualified release, active Roadmap, and Ready Issue #76 define the
product wording, source identity, output boundary, and verification level.
