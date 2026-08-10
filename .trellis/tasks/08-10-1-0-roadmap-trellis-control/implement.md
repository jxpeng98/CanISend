# Implementation plan: Roadmap-backed Trellis project control

Implementation is delegated to `08-10-roadmap-trellis-control-contract`; do not start this parent.

## 1. Establish the local control contract

- [x] Update `.trellis/spec/guides/project-control.md` with the management layers, just-in-time
      child rule, task metadata contract, lifecycle mapping, WIP limit, and drift handling.
- [x] Keep the existing authority order and verification-tier rules intact.

## 2. Reconcile the active Roadmap

- [x] Add a concise Trellis execution subsection to the Master Roadmap governance section.
- [x] State that GitHub owns the backlog/public projection while Trellis owns only active bounded
      execution; do not add another queue or duplicate milestone tables.
- [x] Reconcile the Roadmap definition-of-done statements that still describe completed Alpha.7
      qualification as future work, without rewriting historical records.

## 3. Normalize the current Trellis task

- [x] Create the programme parent and link `08-10-alpha7-followup-cohort-entry` as its child.
- [x] Add the required Roadmap/GitHub/evidence metadata to the current child.
- [x] Keep the programme parent as planning-only governance. Start the governance-contract child
      for these documentation changes and start the Alpha.7 child separately for its own evidence
      work after each child passes its review gate.
- [x] Do not create future milestone children.

## 4. Record projection drift

- [x] Record the observed future GitHub v2/v3 and "dual-pack" wording drift in the parent task or
      journal as a later authorized synchronization item.
- [x] Do not mutate GitHub in this task.

## 5. Verify

- [x] Run `git diff --check`.
- [x] Run `python3 ./.trellis/scripts/task.py list` and confirm one parent with the governance and
      current Roadmap delivery children.
- [x] Run `python3 ./.trellis/scripts/get_context.py` and confirm the governance child context.
- [x] Run `cargo run -p xtask --locked -- release check` once on the final head.
- [x] Review the diff for duplicate authority, stale current-state claims, private data, and
      unintended changes to user-owned files.

## Rollback points

- Revert documentation/spec changes independently of task metadata.
- Use `task.py remove-subtask` to undo only the parent/child link if the structure is rejected.
