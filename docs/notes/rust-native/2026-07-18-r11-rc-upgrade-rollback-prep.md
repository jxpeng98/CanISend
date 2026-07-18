# R11.3 upgrade, rollback, and uninstall preparation

**Date:** 2026-07-18

**Roadmap item:** R11.3 preparation

**Status:** Version-neutral controls qualified on five targets; signed Beta/RC qualification pending

## Boundary

The binary and workspace are different rollback surfaces. Replacing a standalone executable is reversible, but a
new binary may append a Rust-era SQLite migration when it opens a workspace. An older binary must reject a future
schema without mutation. CanISend therefore does not advertise in-place database downgrade or migration-record
deletion.

The new [upgrade, rollback, and uninstall guide](../../guides/upgrade-and-rollback.md) requires a checked and verified
pre-upgrade backup, explicit new-binary `version`/`doctor`, post-open workspace checks, host-pack regeneration, and
restore into a new path when the old binary cannot accept an upgraded workspace. Uninstall removes the binary and
notices but leaves user-owned workspaces and backups untouched.

## Automated archive control

`scripts/smoke_release_archive.sh` already proves exact binary equality, release contents, renderer health, the
documented workflow, and host-agent integration. It now also:

1. copies the extracted executable and release notices into an isolated user-install directory;
2. runs `version` and `doctor` from that installed copy;
3. creates and checks a workspace outside the installation directory;
4. deletes the complete installed binary/notice directory; and
5. requires the workspace configuration and private state directory to remain.

This control is portable Bash and will run in every five-target release matrix. It does not claim to prove a package
manager lifecycle or a Beta-to-RC migration by itself.

## Local validation

- `bash -n scripts/smoke_release_archive.sh` passes.
- `cargo run -p xtask --locked -- release check` requires and validates all eight user guides.
- A fresh macOS arm64 release package passed the exact extracted-archive smoke, including the installed-copy
  `version`/`doctor`, external workspace initialization and check, install-directory removal, and retained workspace
  assertions.
- The existing documented quick-start, host-agent workflow, renderer health, bundle inventory, and expected-binary
  equality checks still run before the new lifecycle assertions.

## Native preparation evidence

GitHub Actions native release run `29637471699` passed at exact source commit
`43c43dc502a0be09eb84b70a025255bebbc3f589`. All five target jobs passed target tests and the exact extracted-archive
smoke, including isolated install, installed-copy `version`/`doctor`, external workspace creation, removal of the
installation directory, and retained workspace assertions. Source gates, assembly, and attestation also passed.

The qualification ledger records this as `prepared-native`, not `passed`: the run used Alpha archives and cannot
substitute for the signed RC-stage documentation/uninstall matrix required by Stable.

## Remaining qualification

- After signed Beta and RC archives exist, build a representative workspace with the Beta binary, back it up, open it
  with RC, and check the expected same-schema or advanced-schema behavior with both versions.
- Restore the pre-upgrade backup into a new directory with Beta and retain exact version/run evidence.
- Run clean Homebrew, Scoop, and WinGet install, upgrade, and uninstall matrices from signed candidates.
- Run the complete clean-tag RC release matrix twice and publish version-specific release/rollback notes.

No R11.3 roadmap item is complete until that signed version-pair evidence exists.
