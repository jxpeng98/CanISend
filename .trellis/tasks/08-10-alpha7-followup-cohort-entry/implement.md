# Implementation plan: Alpha.8 checkpoint and cohort entry

Do not start this parent. Execute and archive its children in order.

## Child 1 — Alpha.8 checkpoint qualification

- [ ] Merge the Trellis project-control branch so task/spec authority exists on `main`.
- [ ] Create the Alpha.8 Roadmap row, GitHub milestone, and `M3-ALPHA8-001` Issue; move Issue #70
      and conditional rerun Issue #68 to the new milestone without rewriting Alpha.7 evidence.
- [ ] Review PR #174 at its exact head, mark it ready only if scope remains bounded, and merge it
      through protected Fast CI.
- [ ] From updated `main`, repair sequential-Alpha document handling and eligible-v4-Alpha Beta
      authority in `xtask` and `refresh_beta_readiness.sh`.
- [ ] Add the smallest positive/negative regressions and update active release documentation.
- [ ] Run focused checks, `bash -n scripts/refresh_beta_readiness.sh`, Alpha.8 transition dry-run,
      and one final `cargo run -p xtask --locked -- release check`; merge through protected CI.
- [ ] Review and apply the Alpha.8 transition from a clean worktree, inspect every controlled
      digest, commit the generated stage change, and merge its green source gate.
- [ ] At an explicit release-operator gate, build once, qualify the supported native matrix, tag,
      promote without rebuilding, download, and independently reverify Alpha.8.
- [ ] Rerun Codex, Claude Code, Claude Desktop, and bounded MCP-host dogfood on exact Alpha.8; commit
      only body-free provider evidence and mark `M3-ALPHA8-001` Verified.

## Child 2 — Alpha.8 cohort and Beta evidence

- [ ] Confirm the exact public Alpha.8/provider identity before inviting users.
- [ ] Run the Roadmap cohort matrix and record only consented aggregate/body-free outcomes.
- [ ] Triage every failure; activate Issue #68 only when changed bytes require an affected rerun.
- [ ] Commit the reviewed cohort note and exact measured JSON record.
- [ ] Refresh Beta readiness dry-run first, review Issue #70 and all denominators/exclusions, then
      write only from a clean worktree after explicit authorization.
- [ ] Mark Issue #70 Verified only when every threshold passes, then hand off to `M4-READY-001`.

## Parent exit

- [ ] Both children are archived as Verified or the programme is explicitly blocked/deferred with
      the last valid public checkpoint retained.
- [ ] Roadmap, GitHub, Trellis, release JSON, tags, and public artifacts agree on the exact state.
