# Beta.1 package-channel contract research

## Existing authority

- ADR-RN-0010 requires `xtask release channels TAG ASSETS OUTPUT` to re-run the complete release
  verifier, reject an existing output directory, derive deterministic Homebrew/Scoop/WinGet files,
  and exact-compare the written tree.
- `candidate-source.json` is canonical `canisend.channel-candidate-source/v1` with
  `candidate_only: true` and `publication_authorized: false`.
- `check_channel_candidates` scans every checked-in version, rejects symlinks and unknown files,
  regenerates all manifest bodies, and preserves the qualified native Alpha baseline.
- The existing focused regression owns archive choice, nested executable paths, platform hashes,
  and historical license behavior.

## Exact Beta.1 input

- Tag: `v1.0.0-beta.1`
- Source: `6e1397b79031cad54e794ccdc9edca2153f23b3e`
- Candidate run: `33281162734`
- Manifest SHA-256:
  `2435c335f2edd31e1a59afd4065380112f4e24924f68f76a26be84acef0041f8`
- Qualification merge: `43dc80b0fb5e3accc602795c8e3b706e0bce8fea`
- Qualification evidence: `docs/notes/rust-native/2026-08-30-beta1-qualification.md`

The qualified release already binds fresh verification for 20 public assets and 20 attestations.
The channel generator still re-runs byte, checksum, manifest, signing-record, and archive-layout
verification before writing.

## Current metadata defect

`render_channel_manifest_files` currently hard-codes `Prepare evidence-backed academic job
applications with agent hosts` for Homebrew, Scoop, and WinGet, plus the WinGet tag
`academic-jobs`. Historical 0.7 candidate trees contain those exact bytes and are intentionally
immutable. ADR-RN-0018 makes `1.0.0-alpha.6` the generic-framework boundary, and the current CLI
already describes the product as `Evidence-backed application preparation`.

The smallest safe repair is a version-aware metadata selector parallel to `channel_license`:
retain historical text before Alpha.6 and use one generic description/tag set from Alpha.6 onward.
One existing renderer test can prove both sides. No generated file is hand-edited.

## Minimum verification

Because one Rust renderer changes, use format, the existing focused renderer regression, xtask
Clippy, and one final source gate. The generator itself exact-validates the new output. Protected
Fast CI owns the workspace suite; native rebuilds, external package-manager validation, and
lifecycle qualification belong to later gates.

## Roadmap header contract caught by the source gate

The first final source-gate pass rejected a wording-only Roadmap change because
`check_active_release_truth_for_version` requires the exact
`**Next intended checkpoint:** \`v1.0.0-beta.1\`` marker before any action description. This was an
implicit documentation-to-validator contract, not a release-state defect. The marker was restored
and the executable rule was added to the backend quality spec; no new test is needed because the
existing source gate already failed closed.
