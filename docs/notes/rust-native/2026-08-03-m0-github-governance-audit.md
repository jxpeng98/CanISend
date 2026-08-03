# M0-GOV-001/002 — GitHub governance audit

Date: 2026-08-03

## Read-only result

The GitHub API was queried for `jxpeng98/CanISend` without changing repository state:

- milestones, including closed milestones: none;
- roadmap-, priority-, milestone-, M-stage-, Alpha-, Beta-, RC-, or Stable-prefixed labels: none;
- open or closed Issues: none;
- `main` branch protection: GitHub returned `404 Branch not protected`;
- repository rulesets, including inherited rulesets: none;
- remote `main`: `cb2db0f772ff1931c84427becd4674c59acf9028`;
- local `main` at audit time, after the Host Agent smoke fix:
  `92ed8bb3bdd4974b6f817714226d03c6ed9c525e`, 36 commits ahead.

M0-GOV-001 and M0-GOV-002 are therefore not complete. Passing local tests cannot substitute for
these external controls, and historical run `30742363439` is bound to remote `main`, not the current
local source.

## Reviewed desired state

Repository-governance application should create:

- milestones for M0 truth/scope, M1 framework foundation, Alpha.6 migration checkpoint, Alpha.7
  dual-Pack validation, Beta, RC, and Stable;
- priority and roadmap-ID labels that keep P0/P1 state machine-readable;
- critical-path Issues whose milestone, owner, status, and Roadmap ID cover every unfinished
  release-blocking item;
- a `main` rule that requires the exact fast-CI jobs, blocks force-push and deletion, preserves solo
  maintenance, and does not require a nonexistent second reviewer; and
- a release-tag rule that permits reviewed tag creation but blocks update, force-push, and deletion.

The required fast-CI contexts are `desktop-ui`, `browser-keyboard-accessibility`, `core-linux`,
`core-windows`, `macos-quality`, and `macos-tests`. Rules must be verified with a disposable test
branch before M0 is marked complete.

## Authorization boundary

Creating milestones, labels, Issues, or protected-ref rules and pushing the local commit series are material
GitHub mutations. This audit intentionally performed none of them. Apply them only after explicit
repository-governance and push authority; inspect the resulting API state before changing any
Roadmap checkbox.
