# R11.3 native archive upgrade qualification

**Date:** 2026-07-18

**Roadmap item:** R11.3 signed Beta-to-RC upgrade preparation

**Status:** Implementation complete; real signed Beta/RC evidence pending

## Outcome

CanISend now has one nonpublishing five-target workflow for the exact public signed Beta/RC archive pair. It covers
macOS arm64, macOS Intel, Linux GNU, Linux musl, and Windows MSVC. The policy, runner script, evidence verifier, and
ledger recorder are separate controls: no checked-in file or successful local fixture can replace the future public
run.

The workflow does not create tags, releases, package-manager publications, or external application data. Its
synthetic workspaces, SQLite databases, backups, host packs, paths, and command output are runner-local and are
deleted. Only a body-free canonical JSON summary is uploaded.

## Exact lifecycle

Every target record:

1. rebinds its archive bytes to the shared verified Beta and RC manifests;
2. installs the Beta executable and notices as one isolated unit;
3. verifies native `version` and `doctor`, creates a synthetic workspace and job, checks it, and creates a verified
   pre-upgrade backup;
4. replaces the installed unit with RC, verifies it, and opens/checks the workspace;
5. proves either same-schema Beta acceptance or future-schema `workspace.conflict` rejection without changing the
   SQLite bytes;
6. restores the Beta backup into a new directory and checks it with Beta;
7. regenerates a Codex host pack with RC; and
8. removes the installed binary/notices while retaining the workspace, backup, and restored workspace until runner
   cleanup.

## Evidence and promotion boundary

`release verify-upgrade-evidence` requires the exact five filenames and record/target/environment mapping. It
rejects mixed GitHub runs, mixed manifest pairs, reused archive digests, false or extra checks, mismatched observed
versions, invalid schema/old-binary combinations, non-UTC timestamps, symlinks, nested extras, and unknown JSON
fields.

`release record-upgrade-qualification` is dry-run-first. An explicit clean-worktree `--write` is accepted only when
the ledger already identifies the exact qualified Beta, frozen baseline, RC workspace state, and successful signed
matrix for the same RC tag. It updates only `upgrade_matrix`; documentation/uninstall and package-manager evidence
remain independently qualified Stable gates.

## Local verification

- `bash -n scripts/qualify_archive_upgrade.sh`
- `cargo test -p xtask --locked`: 36 passed, including four dedicated upgrade-policy/evidence/ledger tests
- `cargo run -p xtask --locked -- release check`: 40 schemas, 51 embedded resources, five upgrade records, signing,
  transition, support, feedback, qualification, and release contracts passed
- `git diff --check`

The archive lifecycle itself cannot run locally without the future public signed Beta and RC pair. The R11.3
roadmap checkbox therefore remains open. After those releases exist, the manual workflow and independent public
attestation review provide the only accepted evidence for ledger promotion.

Exact implementation commit `6e4fd41` passed all eight ordinary CI jobs in GitHub Actions run `29642790518`.
