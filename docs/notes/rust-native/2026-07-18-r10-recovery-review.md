# R10.2 Recovery Review Checkpoint

**Date:** 2026-07-18

**Branch:** `rewrite/rust-native`

**Implementation commit:** `28ce812`

**CI evidence:** GitHub Actions run `29629649534`

## Outcome

R10.2 proves that CanISend either commits an authoritative transition completely or leaves a diagnosable,
recoverable state. Backups and restores now use unique sibling staging directories protected by automatic cleanup.
Restore verifies the source, reconstructs every missing deterministic projection from SQLite/blob authority inside
staging, and only then atomically publishes the destination.

Normal `workspace repair` now covers raw, Markdown, structured JSON, and Typst projection manifests. It observes the
filesystem before trusting stored status, preserves edited user projections, regenerates missing/repair-required
files, and rejects output when the current projection recipe no longer matches the recorded generated digest.

## Failure evidence

The recovery suite covers:

- partial blob streaming and temporary-file cleanup;
- immutable blob publication before a rejected SQLite transaction;
- migration rollback, corrupt SQLite, and bounded writer conflict;
- projection failure after authoritative commit and deterministic repair;
- backup failure caused by a missing referenced blob, with no partial destination;
- restoration of raw plus managed Markdown/JSON/Typst projections;
- missing and digest-mismatched referenced blobs;
- two concurrent host agents completing one lease, producing one commit and one idempotent replay;
- stale task completion after source revision changes.

The full operational mapping and restore procedure are recorded in the
[recovery and interruption matrix](../../recovery/interruption-matrix.md).

## Verification

- 82 Rust tests passed locally after the recovery additions.
- Targeted recovery, strict Clippy, format, schema/resource, and workflow YAML checks passed.
- `recovery-native` passed on Linux x86_64, macOS arm64, and Windows x86_64.
- GitHub Actions run `29629649534` also passed dependency policy, complete quality, and native rendering jobs.

This closes R10.2. The next active roadmap item is the release-profile performance baseline and regression gate.
