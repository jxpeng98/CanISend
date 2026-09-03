# Implementation plan: private Beta.2 readiness

## Phase A — Policy and automated readiness

1. Add an ADR for sequential Beta iteration and update the stage-transition policy, transition
   guide, qualification-ledger guide, and active 1.0 roadmap wording. Do not edit accepted
   historical ADRs or measured Beta.1 evidence.
2. Extend `xtask/src/main.rs` in the existing shared paths:
   - recognize exact sequential Beta transitions;
   - validate and render `beta_history` plus the pending active slot;
   - allow Beta qualification from the canonical frozen sequential state;
   - keep RC/Stable/package/upgrade consumers bound to the active qualified Beta; and
   - report historical Beta qualification separately from the private active candidate.
3. Add focused positive coverage for Beta.1-to-Beta.2 preview/write/qualification state and
   negative coverage for skip, repeat, downgrade, cross-line, build metadata, unqualified source,
   malformed history, duplicate evidence, and transactional rollback.
4. Extend `scripts/smoke_agent_v4_mcp.sh` with project-scoped host setup/status and exact four-Skill
   ownership/digest/MCP-guidance assertions in its existing disposable App-closed journey.
5. Run formatting, relevant Clippy, focused `xtask`, CLI, MCP, resource, database, and smoke tests.
   Run `cargo run -p xtask --locked -- release check` once on the final PR head.
6. Commit the policy/readiness changes, then add their exact nonautomatic paths to
   `release/feature-freeze-exceptions.json` in a separate evidence commit. Exclude the existing
   cohort-task working-tree edits from both commits.
7. Open the policy/readiness PR, wait for Fast CI, review the exact diff, and merge it before source
   preparation.

## Phase B — Controlled Beta.2 source preparation

1. Start from the merged clean base and run the dry-run form of
   `cargo run -p xtask --locked -- release prepare-stage v1.0.0-beta.2`.
2. Verify the complete controlled path list and after-digests, including byte-preserved Beta.1
   history, pending active Beta.2 state, unchanged feature freeze, and unchanged Beta-entry and
   cohort evidence.
3. Run the same command with `--write`; run the focused transition tests, `git diff --check`, and
   the release source gate.
4. Commit only the controlled transition files. Add the exact nonautomatic paths and preceding
   commit ID to the feature-freeze exception ledger in a second commit, then rerun the release
   source gate.
5. Open the source-transition PR, wait for Fast CI, review the exact diff, and merge it.

## Phase C — Handoff

1. Rerun the packaged App-closed smoke against the merged Beta.2 source and record the exact
   automated commands and results.
2. Report any remaining native accessibility, real-host, consent, cohort, signing, or publication
   checks with the reason they cannot be synthetic.
3. Confirm that public Beta.1, its source and run IDs, tag, assets, and qualification evidence are
   unchanged; RC.1 and Beta.2 publication remain uncreated.

## Stop conditions

- Stop before mutation if the transition preview contains an uncontrolled path or rewrites
  historical evidence.
- Stop before merge if focused checks, the source gate, or Fast CI fail.
- Stop before any tag, workflow dispatch, qualification write, release, or package publication;
  those actions are outside this task and require separate authorization.
