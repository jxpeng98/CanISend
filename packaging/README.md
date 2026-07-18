# Package-manager channel candidates

This directory contains deterministic review and test candidates derived from verified native CanISend release
assets. A checked-in candidate is not evidence that a public Homebrew, Scoop, or WinGet channel exists. Publication
requires the later release-stage gates described below.

## Generate a candidate set

Start from one complete release directory that already passes the repository verifier, then choose a new output
directory whose final component is the release tag:

```console
cargo run -p xtask -- release verify TAG VERIFIED_RELEASE_DIRECTORY
cargo run -p xtask -- release channels TAG VERIFIED_RELEASE_DIRECTORY OUTPUT_DIRECTORY
```

`release channels` runs the full verifier again before writing anything. It records the release-manifest digest,
source commit, archive names, sizes, and SHA-256 digests in `candidate-source.json`, then generates:

- a Homebrew Cask with Apple Silicon and Intel archive selection;
- a Scoop bucket manifest for Windows x86_64;
- a three-file WinGet 1.12 manifest for the portable executable nested inside the release ZIP.

`cargo run -p xtask -- release check` reconstructs every checked-in candidate from its source record, rejects
symlinks and unknown files, and requires exact byte equality. Candidate source records set `candidate_only: true`
and `publication_authorized: false`; changing either invariant fails closed.

## Why Homebrew uses a Cask

CanISend distributes upstream-built and, after the signing milestone, notarized executables. Current Homebrew policy
places upstream precompiled binaries in Casks rather than source-building Formulae. The Cask uses Homebrew's `arch`,
architecture-specific `sha256`, and `binary` stanzas, and points to the actual versioned directory inside each
release archive.

References:

- [Homebrew Cask Cookbook](https://docs.brew.sh/Cask-Cookbook)
- [Homebrew acceptable Formulae policy](https://docs.brew.sh/Acceptable-Formulae)
- [Scoop app manifest reference](https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests)
- [WinGet manifest authoring](https://learn.microsoft.com/en-us/windows/package-manager/package/manifest)
- [WinGet manifest validation](https://learn.microsoft.com/en-us/windows/package-manager/winget/validate)

## Publication gates

The current Alpha-derived set proves generation and review shape only. It is intentionally not submitted to an
external package repository. For the version that will be published:

1. Regenerate from the final signed/notarized release assets; never edit hashes or URLs by hand.
2. Run `brew style` and `brew audit --strict --cask`, then install, execute `canisend version --json`, upgrade, and
   uninstall on both Apple Silicon and Intel macOS.
3. Install the Scoop manifest in a clean Windows account, execute `canisend version --json` and `canisend doctor
   --json`, upgrade, and uninstall without deleting user workspaces.
4. Run `winget validate MANIFEST_DIRECTORY`, followed by Windows Sandbox install, execution, upgrade, and uninstall.
5. Confirm all channel downloads match the final signed release archive hashes and preserve disabled default
   telemetry.
6. Publish only when the roadmap's release-stage authorization and rollback evidence are both complete.

Historical candidate sets remain checked in as auditable derivations. After the two clean release-candidate
matrices and every Stable ledger gate pass, release assembly generates canonical Homebrew, Scoop, and WinGet files
from the final signed archives. Those files, their scoped publication record, release-manifest entries,
`SHA256SUMS`, and GitHub build provenance are published as one Stable GitHub release unit.

The publication record authorizes only `github-release-assets` and explicitly leaves `external_index_submission`
false. Submitting the recorded repository paths to a Homebrew tap, Scoop bucket, or `winget-pkgs` remains a separate
maintainer action because it changes another repository and may require its own review or credentials.
