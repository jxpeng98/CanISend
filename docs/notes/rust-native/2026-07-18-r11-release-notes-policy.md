# R11 release-notes and rollback policy

**Date:** 2026-07-18

**Roadmap item:** R11.3 release notes and rollback guidance preparation

**Status:** Stage-neutral structure enforced; exact RC final review pending

## Problem corrected

The stage-transition tool intentionally changed only the `RELEASE_NOTES.md` version heading. The earlier body said
“The alpha” and “This prerelease,” so a mechanically qualified Stable transition could have published stale stage
wording even though the heading and ledger were correct.

## Result

The release-note body is now stage-neutral and complete across seven ordered sections: highlights, compatibility,
install/verification, upgrade/rollback, security/privacy, known limitations, and feedback/support. It records the
standalone runtime boundary, five targets, Agent/workspace v2, the Python-era break, no-submission boundary, exact
release verification, backup-first rollback, no in-place downgrade, default-off telemetry, scoped limitations, and
safe issue reporting.

`release/release-notes-policy.json` is exact machine authority. `release check` now requires the current workspace
version exactly once in the heading, the complete ordered section set, required guidance, three repository guide
links, and no Alpha/Beta/RC/prerelease/Stable word in the body. A regression test proves a stage transition changes
only the heading and rejects reintroduced stage-specific body text.

## Qualification boundary

The rollback guide and final-ready note structure exist now, but R11.3 remains open. The exact RC notes still require
a maintainer content review against the real signed version, resolved issues, final limitations, archive set, and
package-channel state. Stage transition and checked-in policy cannot substitute for that release-time review.

## Local verification

- `cargo test -p xtask --locked`: 40 passed
- `cargo run -p xtask --locked -- release check`: seven stage-neutral sections passed
- `git diff --check`

Exact implementation commit `19577a6` passed all eight ordinary CI jobs in GitHub Actions run `29643209061`.
