# Activate Beta.1 feature freeze

## Goal

Activate the existing fail-closed 1.0 feature-freeze transaction against an exact protected
repository baseline that already contains the qualified Beta.1 evidence and local package-channel
candidates. Keep the published Beta artifact source identity distinct from the repository freeze
baseline, and preserve normal Trellis task/journal bookkeeping without exempting executable or
policy-bearing Trellis files from freeze review.

## Confirmed background

- `v1.0.0-beta.1` is public and qualified from artifact source
  `6e1397b79031cad54e794ccdc9edca2153f23b3e`, candidate run `33281162734`.
- `M4-LEDGER-001` / Issue #75 is Verified through merge
  `43dc80b0fb5e3accc602795c8e3b706e0bce8fea`.
- `M4-CHANNEL-001` / Issue #76 is Verified through merge
  `1c9f36d94a567975b2a8318af9138b9cda7320ba`.
- The checked-in ledger is canonical `beta-qualifying` with qualified signed Beta evidence; both
  feature-freeze records remain `planned`, have a null baseline, and contain no exceptions.
- The existing `xtask release activate-feature-freeze FULL_HEAD_COMMIT [--write]` command requires
  a full lowercase commit equal to the current clean `HEAD` in write mode and renders exactly
  `release/qualification-ledger.json` plus `release/feature-freeze-exceptions.json`.
- A read-only preflight against exact HEAD `29c009182bba8d22e5d758373770935771d5bcde`
  rendered those two files without mutation. That is research evidence, not the final baseline.
- Protected GitHub merges add a merge commit. Any nonautomatic preparation change must therefore
  land before the final baseline is resolved; otherwise the post-merge freeze history would reject
  the merge commit.

## Requirements

### R1 — Prepare the control boundary before resolving the baseline

- Use a first protected preparation PR for all nonautomatic changes required by the current
  Trellis-managed release process.
- Treat only `.trellis/tasks/` and `.trellis/workspace/` as automatic post-baseline control records.
  Keep `.trellis/scripts/`, `.trellis/spec/`, `.trellis/workflow.md`, platform adapters, product
  source, workflows, and every other Trellis path subject to an exact freeze exception.
- Extend the existing feature-freeze policy regression; add no new test module, dependency,
  abstraction, schema, command, or alternate activation path.
- Clarify in the release guidance that the qualified Beta artifact source and the feature-freeze
  repository baseline are separate exact identities.
- Do not activate the feature freeze in the preparation PR.

### R2 — Bind activation to the protected preparation merge

- Merge the preparation PR after focused checks, one source gate, and protected Fast CI pass.
- Fast-forward local `main`, create the activation branch from that exact protected merge, and
  resolve the new full `HEAD` with Git. That commit is the only permitted activation baseline.
- Run dry-run first, then `--write` from the same clean HEAD. Require matching baseline, file paths,
  and before/after digests, with only mode and `writes_performed` changing.
- Require both records to become `frozen`, carry the exact same non-null baseline, retain the
  canonical allowed change classes, and retain an empty exception list at activation.
- The write may change only the two existing machine records.

### R3 — Reconcile the frozen state without widening release authority

- Commit the two activation files as their own reviewable change before later automatic docs and
  Trellis task-record updates.
- Add one dated body-free evidence note and reconcile README, RELEASE, the Master Roadmap, Trellis
  project control, parent/current task state, and Issue #77 through protected review.
- Mark `M3-EVID-005` / Issue #70 Ready only after the activation PR is protected and merged.
- Do not create or move a tag, publish a release or package channel, authorize Stable, start RC.1,
  invite users, or claim cohort evidence.

### R4 — Use minimum-sufficient verification

- Preparation PR: Rust format, the existing focused feature-freeze regression, affected xtask
  Clippy, one final `release check`, `git diff --check`, Trellis validation, and protected Fast CI.
- Activation PR: compare dry-run/write reports, inspect the exact two-file transaction, run one
  final `release check`, `git diff --check`, Trellis validation, and protected Fast CI.
- Do not run the local full workspace suite, rebuild native artifacts, repeat public asset
  verification, rerun provider dogfood, execute package-manager lifecycles, or run extended
  assurance.

## Acceptance criteria

- [x] The preparation policy accepts only `.trellis/tasks/` and `.trellis/workspace/` as automatic
      Trellis records and still rejects executable, spec, workflow, adapter, and product paths.
- [ ] The preparation PR is protected and merged before the final activation baseline is resolved.
- [ ] The final baseline is the exact protected preparation merge commit, while the ledger retains
      Beta artifact source `6e1397b79031cad54e794ccdc9edca2153f23b3e` unchanged.
- [ ] Dry-run and write reports bind the same baseline, two paths, and before/after digests; write
      mode changes only the two canonical machine records.
- [ ] Both feature-freeze records are `frozen` at the same full commit with zero initial
      exceptions and unchanged allowed change classes.
- [ ] The source gate and protected checks pass on the exact activation PR head before Issue #77
      becomes Verified and Issue #70 becomes Ready.
- [ ] No external publication, user outreach, RC transition, Stable authorization, native rebuild,
      or unsupported evidence claim occurs.

## Out of scope

- Invited or cumulative cohort execution, consent, private bodies, provider sends, and retention.
- RC.1 or Stable stage preparation, tagging, building, promotion, qualification, or publication.
- External package-manager repository mutation or lifecycle qualification.
- Broad `.trellis/` freeze exemption, new release tooling, a schema change, or compatibility work.

## Blocking open questions

None. Existing ADRs, machine records, the audited activation command, protected merge behavior,
the active Roadmap, and Ready Issue #77 determine the bounded outcome and sequencing.
