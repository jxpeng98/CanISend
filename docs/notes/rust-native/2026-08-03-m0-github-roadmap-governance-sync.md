# M0 GitHub Roadmap governance synchronization

**Date:** 2026-08-03

**Roadmap tasks:** M0-GOV-001, M0-GOV-002, M1-CI-001, M1-DEP-001

## Outcome

The repository governance state now matches the active 1.0 Roadmap without creating release
authority. GitHub contains seven open milestones covering M0, the combined M1 foundation, Alpha.6,
Alpha.7, Beta, RC, and Stable. Ninety-two Issues map the Roadmap's complete P0/P1 inventory one to
one: every Issue has one exact Roadmap ID, priority, owner, state, and milestone association.

No milestone has a due date. The Roadmap contains duration ranges but explicitly does not make a
release promise, so converting those ranges into calendar deadlines would invent authority.

## Milestone chain

| GitHub milestone | Roadmap scope | Entry boundary |
|---|---|---|
| M0 — Truth and generic scope | §8 / M0 | Published Alpha.5 plus audited post-tag source |
| M1 — Framework foundation | §9 / M1F, M1A, M1B | M0 governance and truth dependencies |
| Alpha.6 — Migration checkpoint | §10 / M2 | Verified M0 and all M1 streams |
| Alpha.7 — Dual-pack validation | §11 / M3 | Published and publicly reverified Alpha.6 |
| Beta — Contract freeze | §12 / M4 | Verified Alpha.7 Agent/user evidence |
| RC — Release qualification | §13 / M5 | Qualified Beta.1 and active freeze |
| Stable — 1.0 | §14 / M6 | Two clean RC matrices and complete Stable ledger |

## Protected-ref state

Repository ruleset `20284730` protects the default branch with no bypass actors. It requires a
pull request, resolved review threads, strict up-to-date results from `desktop-ui`,
`browser-keyboard-accessibility`, `core-linux`, `core-windows`, `macos-quality`, and `macos-tests`,
and blocks deletion and non-fast-forward updates. The approving-review count remains zero because
the Roadmap explicitly requires solo maintenance to remain possible.

Repository ruleset `20284731` applies to `refs/tags/v*`, has no bypass actors, and blocks deletion,
update, and non-fast-forward mutation after tag creation. It does not authorize tag creation; the
Roadmap's candidate inspection and explicit release authorization remain separate prerequisites.

A disposable-branch probe received GitHub `GH013`, requiring a pull request and all six expected
checks. The probe branch was then deleted and removed from the ruleset target. This proves the
policy without weakening `main` or manufacturing a release tag.

`policy-and-deny` remains path-scoped rather than an unconditional required status check. Making a
path-filtered workflow globally required would leave unrelated pull requests permanently waiting.
Dependency changes, policy authority, `xtask`, and the named reachability-sensitive sources still
trigger the pinned dependency-assurance workflow.

## Exact replacement CI evidence

Draft PR [#95](https://github.com/jxpeng98/CanISend/pull/95) at
`e9c4b955721571ea90473c951c1459ffdaaec4d0` provides the current committed runner evidence:

- Fast CI run [30802993463](https://github.com/jxpeng98/CanISend/actions/runs/30802993463)
  passed all six protected jobs;
- dependency-assurance run
  [30802993936](https://github.com/jxpeng98/CanISend/actions/runs/30802993936) passed
  `policy-and-deny`;
- `core-windows` passed the fresh-checkout Markdown LF and embedded Pack digest boundary; and
- `macos-tests` passed the explicit academic Pack plus bounded UUID smoke path.

The PR remains Draft and unmerged. M1-CI-001 therefore remains In progress until the protected
integration lands, while source work already present on `main` may use this exact current-commit
matrix as its missing remote evidence.

## State reconciliation policy

Issues are marked Verified only where the implementation was already committed on `main`, the
task-specific source evidence exists, and the replacement source/dependency gates satisfy the
Roadmap's smallest applicable verification tier. Future-stage, exact-candidate, dogfood, user,
promotion, and public-verification Issues remain Planned or Ready. No Issue state in this
synchronization changes the workspace version, qualification ledger, tag set, or public release.

## Rollback

Roadmap metadata can be restored by reopening the affected Issue and returning its single state
label/body field to the prior value. Milestone descriptions and ruleset display names can be
reverted independently. Do not delete historical run evidence, rewrite published tags, add a
bypass actor, or lower a required Fast CI check to make governance appear green.
