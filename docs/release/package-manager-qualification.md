# Package-manager Qualification Policy

[`release/package-manager-qualification-policy.json`](../../release/package-manager-qualification-policy.json) is
the machine authority for proving package-manager behavior before Stable. It does not publish a Cask, Scoop bucket,
WinGet manifest, release, or repository change.

## Release pair

Qualification uses a public, signed Beta as the installed version and a public, signed RC as the upgrade target.
Both versions must belong to the same release line, and both checked-in candidate sets must remain
`candidate_only: true` with `publication_authorized: false`. Alpha archives cannot satisfy this contract.

## Required records

One qualification run must produce four independently bound records:

| Record | Native environment | Mandatory native validation |
|---|---|---|
| `homebrew-aarch64-apple-darwin` | Apple Silicon macOS | `brew style`, strict Cask audit, lifecycle |
| `homebrew-x86_64-apple-darwin` | Intel macOS | `brew style`, strict Cask audit, lifecycle |
| `scoop-x86_64-pc-windows-msvc` | Windows x86_64 | local-bucket lifecycle |
| `winget-x86_64-pc-windows-msvc` | Windows x86_64 and Windows Sandbox | `winget validate`, Sandbox install, lifecycle |

Every record binds the Beta and RC candidate-source SHA-256 digests, GitHub run ID, runner/architecture, tool version,
exact version observations, and each lifecycle result. All 12 named checks must be exactly `true`; unknown fields,
skipped validators, tolerated failures, mixed run IDs, mixed candidate digests, or a noncanonical file set fail.

The lifecycle creates a workspace outside the package-manager installation root, upgrades the executable, removes
the installed executable, and proves the user-owned workspace remains. It never publishes externally and never uses
real application data.

## Evidence boundary

The qualification ledger may change `package_managers.status` from `candidates-only` to `passed` only after:

1. the four records validate against the policy;
2. their run ID and exact Beta/RC tags are independently inspected;
3. the signed release manifests and candidate-source digests match the public assets; and
4. the run contains no skipped or tolerated failure.

The manual `package-manager-prequalification` workflow is read-only with respect to external package repositories.
It verifies both public releases, runs Homebrew on both macOS architectures, runs Scoop on Windows 2025, validates
both WinGet manifests, and produces a Windows Sandbox kit. The workflow intentionally stops with three records;
WinGet becomes the fourth only after the bundled lifecycle runs in a fresh Sandbox according to the
[Sandbox guide](winget-sandbox-qualification.md). The final Stable publication remains a separate authorized action
after two clean RC release matrices.

Before running a package lifecycle, download both complete public release asset sets and verify the candidate pair:

```bash
cargo run -p xtask --locked -- release verify-package-candidates \
  v0.7.0-beta.1 BETA_ASSETS v0.7.0-rc.1 RC_ASSETS
```

This command re-verifies every checksum and manifest field in both historical releases, including mandatory
non-Alpha signing records, then binds each checked-in candidate source and channel artifact to the corresponding
public manifest bytes. Release assembly and tag publication remain restricted to the current workspace version;
only the read-only verifier accepts historical tags.

After collecting the four JSON records, verify them with:

```bash
cargo run -p xtask --locked -- release verify-package-evidence \
  v0.7.0-beta.1 v0.7.0-rc.1 EVIDENCE_DIRECTORY
```

The verifier independently enforces the Beta-to-RC stage pair, same release line, exact record environments, shared
run ID, shared but distinct candidate-source digests, exact observed versions, and all policy checks.

After independently inspecting the complete run, fresh-Sandbox record, signed public asset bindings, and absence of
skipped/tolerated failures, preview the only permitted ledger mutation from a clean current-RC checkout:

```bash
cargo run -p xtask --locked -- release record-package-qualification \
  v0.7.0-beta.1 v0.7.0-rc.1 EVIDENCE_DIRECTORY
```

The command reruns the strict four-record verifier, requires the ledger's qualified Beta tag and an already
successful matrix for the exact RC tag, and prints before/after ledger hashes. Nothing changes without `--write`.
It updates only `package_managers`; the preparation workflow's three hosted records or a WinGet record from another
run cannot qualify the release.

The successful record also retains canonical `beta_tag`, `rc_tag`, `run_id`, and four-record count fields. Stable
assembly refuses prose-only or hand-extended evidence, so its published channel assets can name the exact native
qualification they depend on.

## Stable GitHub release publication

Only a fully qualified Stable ledger causes `release assemble` to add these six supplemental assets:

- `canisend-VERSION-channel-publication.json`;
- one Homebrew Cask and one Scoop manifest;
- WinGet version, locale, and installer manifests.

The publication record binds the final archive hashes, exact package qualification, final RC matrix, canonical
external repository paths, and each manifest digest. All six files are listed in the release manifest, covered by
`SHA256SUMS`, attested by the release workflow, and uploaded with the Stable GitHub release. `release verify`
regenerates every byte and rejects missing, renamed, unknown, or modified channel assets.

This authorization is deliberately scoped to GitHub release assets. The record sets
`external_index_submission: false`; opening a pull request or pushing to a Homebrew tap, Scoop bucket, or
`winget-pkgs` requires a separate explicit maintainer action and the target repository's review.
