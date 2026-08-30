# Qualify public Beta.1 with post-Beta cohort evidence

## Goal

Prove that consented target users can complete CanISend's frozen, Codex-first mixed-Application
workflow on exact public Beta.1, using only body-free aggregate evidence, before RC.1 planning.

## Confirmed facts

- Public `v1.0.0-beta.1` was built from
  `6e1397b79031cad54e794ccdc9edca2153f23b3e`, qualified from candidate run `33281162734`, and
  publicly reverified by run `33283530240`.
- Feature freeze is active at protected repository baseline
  `acf25dc483643ca9be0210320775708da116b715` with zero initial exceptions.
- Issue #70 / `M3-EVID-005` is Ready. It is a post-Beta M4 exit gate, not a Beta-entry gate, and
  must complete before RC.1 planning.
- `release/beta-readiness.json` intentionally retains the pre-Beta zero-user boundary and is not
  rewritten as cohort evidence.
- Synthetic maintainer and provider dogfood contributes zero users and zero user flows.

## Requirements

1. Invite 5–8 consented target users per bounded cohort window and continue until the cumulative
   evidence includes at least 8 users and 20 completed supported flows.
2. Cover both built-in Packs, at least one mixed-Application Workspace, two academic and three
   non-academic scenario families, English and Simplified Chinese, URL/pasted-text/local-file/
   text-PDF intake, keyboard-only navigation, VoiceOver, 200% text scale, and the no-submission
   boundary.
3. Include clean App and CLI-only initialization, App-closed Codex operation followed by App
   reconciliation, mixed-Application backup/restore to a new path, and evidence/association
   rejection cases without weakening their controls. Exercise at least two Deliverable sets,
   including one with neither a CV nor an academic statement, and prove unsupported legacy
   Workspace and old-Skill inputs are refused without mutation.
4. Use Codex as the primary validated Agent host. Claude Desktop and Claude Code are optional
   compatibility observations and must never be reported as passed when skipped or unauthenticated.
5. Record explicit numerators and denominators for unassisted supported-flow completion, audited
   claim traceability, measured backup/restore, unsupported claims, and no-submission understanding.
6. Count product, installation, navigation, rendering, data, and recovery failures in the
   unassisted denominator. Exclude only participant withdrawal or a documented external-host
   outage, and give every product failure a minimum-safe Issue link and disposition.
7. Commit one reviewed body-free note and one machine-validated aggregate JSON record bound to the
   exact Beta.1 identity. Retain no Application body, transcript, prompt, local path, credential,
   provider token, or participant identity.
8. Stop affected collection if a supported P0 blocker changes product bytes. Qualify the next
   permitted prerelease before resuming; never rewrite Beta.1 or historical evidence.

## Acceptance criteria

- [ ] The evidence binds tag `v1.0.0-beta.1`, source
      `6e1397b79031cad54e794ccdc9edca2153f23b3e`, candidate run `33281162734`, public-verification run
      `33283530240`, both Pack identities, frozen Agent/Workspace/Skill contracts, and the reviewed
      note digest.
- [ ] At least 8 cumulative consented target users and 20 completed supported flows are counted;
      no synthetic user or flow is counted.
- [ ] Both Packs, a mixed-Application Workspace, two academic and three non-academic scenario
      families, both languages, all four intake forms, two Deliverable sets, recovery, legacy
      refusal, keyboard/VoiceOver/200%-scale accessibility, and no-submission understanding meet
      the coverage contract.
- [ ] Unassisted completion is at least 80%; audited claim traceability and measured
      backup/restore are 100%; unsupported audited claims are zero; no unresolved P0/P1 cohort
      blocker remains.
- [ ] Every exclusion and product failure is body-free, denominator-correct, and linked to a
      minimum-safe disposition.
- [ ] The aggregate record and note contain no private body, participant identity, path,
      credential, transcript, prompt, or provider token.
- [ ] Issue #70 and the checked-in cohort evidence agree before RC.1 planning begins;
      `release/beta-readiness.json` remains unchanged.

## Out of scope

- Contacting or inviting participants before explicit consent and an owner schedule exist.
- Retaining a participant roster or per-user private material in the repository.
- Requiring Claude real-host validation for this Codex-first gate.
- Product feature work, package-index publication, RC construction, or Stable authorization.
- Re-running native release matrices that exact public Beta.1 has already passed.

## Risks and rollback

- A changed-byte P0 fix invalidates affected Beta.1 observations; stop, qualify the next permitted
  prerelease, and resume only the affected coverage on that exact build.
- Missing or inconsistent aggregate evidence blocks RC.1; it does not justify filling gaps with
  synthetic results or private data.
- Planning-only changes are reverted through their protected PR. Cohort evidence is append-only
  and historical Beta.1 records are never rewritten.

## References

- Issue #70 / `M3-EVID-005`
- `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md`
- `release/qualification-ledger.json`
- `release/feature-freeze-exceptions.json`
- `docs/notes/rust-native/2026-08-30-beta1-qualification.md`
- `docs/notes/rust-native/2026-08-30-beta1-feature-freeze.md`

Planning is converged. Execution remains gated on explicit participant consent and the validation
owner's bounded schedule; this plan does not authorize invitations, provider sends, or private-data
access.
