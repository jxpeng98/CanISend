# Design: body-free Beta.1 cohort qualification

## Boundary

This task validates the frozen public product; it does not add a cohort subsystem. The repository
receives only one aggregate machine record and one reviewed body-free note. Consent coordination
and any participant contact remain outside the repository and require explicit authorization.

## Authority and identity

- Public artifact identity: qualified `v1.0.0-beta.1` source and release runs.
- Repository freeze identity: protected baseline
  `acf25dc483643ca9be0210320775708da116b715` plus exact approved exceptions.
- Work and acceptance: Master Roadmap and Issue #70.
- Execution memory: this Trellis task.

`release/beta-readiness.json` remains the immutable pre-Beta readiness record with zero users. The
post-Beta cohort uses a separate `release/cohort-evidence.json` record so RC qualification cannot
silently reinterpret historical Beta-entry evidence.

## Flow

1. The validation owner confirms a bounded window and consent for 5–8 invited users. Names,
   contacts, and private bodies are not committed.
2. Before each window, the operator verifies the public Beta.1 tag/source identity and supplies a
   body-free coverage matrix derived from Issue #70.
3. Participants run supported App, CLI, and Codex paths. Results are reduced immediately to
   aggregate counts, coverage tokens, exclusions, and minimum-safe blocker links.
4. Product failures stay in the applicable denominator. Only withdrawal and documented
   external-host outage are excluded.
5. The owner reviews one dated note, hashes it, and writes the aggregate JSON through the owning
   validator from a clean worktree.
6. Protected CI checks the record and RC-entry binding. Issue #70 becomes Verified only after the
   checked-in record, note, metrics, and blocker dispositions agree.

## Aggregate contract

The future record needs only fields that RC entry must prove:

- schema, status, exact Beta.1 tag/source/candidate/public-verification runs, and feature-freeze
  baseline;
- contract, Pack, and Skill digests already owned by release authorities;
- invited and cumulative user counts plus completed-flow count;
- body-free coverage tokens for Packs, mixed Workspace, scenario families, languages, inputs,
  Deliverable sets, recovery, legacy refusal, accessibility, hosts, and no-submission
  understanding;
- explicit numerators/denominators for each required metric;
- exclusions and blocker Issue numbers using minimum-safe metadata; and
- reviewed evidence-note path and SHA-256 digest.

Unknown fields, private-content-shaped fields, stale identities, synthetic users, incomplete
coverage, invalid denominators, unsupported claims, or unresolved P0/P1 blockers must fail the
validator.

## Privacy and consent

The checked-in evidence contains aggregates, controlled tokens, Issue numbers, exact public build
identity, and a note digest. It never contains a participant roster, Application content, local
paths, transcripts, prompts, credentials, or provider tokens. Consent is an entry condition, not
an implied side effect of starting the Trellis task.

## Compatibility and rollback

No historical schema or Beta.1 artifact changes. A product-byte fix stops affected collection and
moves later observations to the next qualified prerelease without rewriting Beta.1 evidence. A
planning error is reverted as one protected documentation PR.

## Deliberate simplifications

- No participant-management feature or external tracker.
- No Claude requirement for the Codex-first gate.
- No full native rebuild or duplicate workspace suite.
- No final JSON schema implementation until consent and a real cohort window make the record
  necessary.
