# R11 release qualification ledger

**Date:** 2026-07-18

**Roadmap items:** R11.2, R11.3, and R11.4

**Status:** Pre-Beta ledger implemented; external and clean-tag evidence pending

## Why this ledger exists

The release roadmap previously described Beta signing, two RC matrices, migration/rollback, uninstall, package
managers, and Stable notes in prose and checkboxes. A version bump could therefore pass the source tests without one
machine authority proving that the earlier stage evidence had been retained.

`release/qualification-ledger.json` now has an exact status for each workspace release stage. `xtask release check`
validates current-stage alignment, the three package-manager channels, feature-freeze policy, documentation paths,
and the prerelease prohibition on Stable authorization.

For a Stable workspace version, the gate additionally requires a frozen commit, one qualified signed Beta, two
distinct successful clean-tag RC runs, passed upgrade/restore, five-target documentation/uninstall, all three package
manager lifecycles, final notes, and explicit authorization. Run IDs and commits are structurally validated, but
maintainers must still inspect their external evidence rather than trusting the ledger alone.

## Current evidence level

Only the local macOS arm64 archive lifecycle smoke is recorded as preparation. It is not promoted to five-target or
RC evidence. The Beta, RC, upgrade, package-manager, and Stable fields remain pending or candidate-only, consistent
with the missing Apple/Azure repository configuration.
