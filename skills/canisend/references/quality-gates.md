# Quality Gates

Use this reference before saying generated application materials are ready for user review, Typst rendering, or manual submission.

## Ready Claim Gate

Do not use ready, final, complete, or submission-ready for generated materials until the relevant gates below have been checked against the current job folder. If a gate is not checked, say what remains unchecked instead.

## Profile Evidence Gate

- `profile/profile.yaml` exists.
- `profile/generated/` contains current evidence files.
- Strong claims cite normalized evidence, not private Typst prose directly.
- Every citation points to an existing generated evidence file, section, and item when an item ID exists.
- New agent output should use item-level citations such as `profile/generated/file.evidence.md#Section/item-id`.
- Item-level citations are preferred; section-level citations are compatibility only for older outputs or manually reviewed fallbacks.
- Missing evidence is reported as a gap.
- Typst-backed generated evidence has a current source-hash receipt. Re-extract older receiptless output or output
  made stale by a changed raw profile source.

## Evidence Catalog Gate

- Deterministic Evidence succeeded against the current manifest, raw-source receipts, generated evidence, schema,
  and versioned input limits.
- `evidence_catalog.json` is `available`, a deliberately reviewed valid `empty`, or an explicitly explained
  `unavailable`; these states are not interchangeable.
- A valid empty catalog does not mean the applicant lacks all evidence. An unavailable catalog must be refreshed
  before relying on Match.
- Stable Evidence IDs are content-and-kind-derived; legacy path/section/item labels remain display locators only.
- Evidence snapshots, candidates, and the catalog remain private body-bearing data-plane artifacts and are not quoted
  or staged merely to prove workflow status.

## Job Advert Gate

- Feed-created jobs no longer rely only on RSS or Atom description.
- `job_advert.md` contains the full advert or the user has explicitly accepted partial processing.
- `parsed_job.json` title, institution, deadline, required documents, essential criteria, and source URL match the advert.
- Missing advert fields are left empty or unknown; do not invent.

## Criterion Match Gate

- Confirm and Evidence are current, and Match records the current `criteria.json` and `evidence_catalog.json` hashes.
- Every criterion has exactly one deterministic `strong`, `partial`, `weak`, `missing`, or `unknown` classification.
- Every supported match resolves an opaque current Evidence ID; `missing` and `unknown` records carry an explicit
  privacy-safe gap.
- An empty catalog and an unavailable catalog produce distinct `unknown` gaps; an available catalog with no relevant
  support produces `missing`.
- `criterion_matches.json` contains no evidence bodies, private headings, legacy item labels, or private evidence
  kinds.
- Every classification is `review_state=proposed`. Do not treat Match as a user-owned application Decision, claim
  confirmation, or package-readiness signal.

## User-Owned Corrections And Decision Gate

- Unknown criteria extraction is not treated as `confirmed_empty`; an empty corrections template proves nothing.
- Each semantic correction was based on current Parse and Confirm, and Confirm reran before any next correction.
- Agent-assisted mutations used status, one strict scoped patch, current revision/hash CAS, and explicit consent;
  agents did not replace a user YAML file directly.
- A missing or `undecided` Decision is not treated as apply, hold, or skip. Match did not infer the value.
- For a confirmed Decision, `decision status` reports a current Criteria/Match basis. A preserved stale value remains
  review-required until the user explicitly reconfirms it.
- User YAML/private candidates/corrected Criteria remain Tier 2. Tier 1 receipts and all control/Agent output contain
  no correction text or rationale.

## Application Brief And Required-Document Plan Gate

- Brief work began from a current confirmed `decision=apply`; a changed Decision basis preserved the Brief and was
  explicitly reconfirmed rather than silently rewritten.
- `brief status` returned only body-free paths, hashes, field/count state, reasons, and actions. Any agent read of
  `application_brief.yaml` or `required_document_plan.json` received separate Tier 2 approval.
- Agent-assisted Brief writes used one bounded strict patch, the latest raw-byte revision/hash, and explicit consent;
  no agent replaced `application_brief.yaml` directly.
- The requirement-set basis is current and explicitly `confirmed` or `confirmed_empty`. Empty parser output,
  ambiguity, absence, or failure was not interpreted as `confirmed_empty`. Every non-empty requirement resolves to a
  complete positive source member; conditional, alternative, qualified, truncated, or continuation context is unknown.
- Deterministic Brief produced current `required_document_plan.json`; no one edited the core-owned plan directly.
- Every current normalized requirement has one task. An unconfirmed set or field, unresolved choice,
  `required + omit`, required document without preparation, or orphaned choice is a blocker for later Draft/Verify.
- Brief/plan/candidates remain Tier 2. Claims, receipts, manifests, errors, ordinary output, and AgentResponse contain
  no motivation, exclusions, language/style values, advert source text, or document labels.
- Stage 2 local acceptance is not treated as Draft, application-package, or submission readiness.

## Draft Gate

- Each current `cover_letter_draft.json` or `research_statement_draft.json` binds exact current Parsed Job, Criteria,
  Evidence, Match, Decision, Brief, and document-plan hashes, targets its exact stable confirmed `prepare` document
  ID/kind, and remains `review_state=proposed`.
- Every applicant-facing block is one Claim with a recomputed stable ID. Strong/partial factual claims resolve to
  current Evidence; unsupported facts carry `claim.unsupported`; partial facts carry `claim.partial_support`.
- Current deterministic `review_findings.json` exists for the Draft. Unsupported claims, confirmed Brief exclusion
  conflicts, and missing opening/body/closing sections are blockers. Supported factual wording retains an explicit
  semantic-support review item until inspected; every non-factual Claim kind also remains open for semantic review.
- A current Research Statement uses `research_statement_review_findings.json`; missing `research_overview`,
  `research_contributions`, or `future_agenda` is a blocker. Its findings use the independent
  `research_statement_review_dispositions.yaml`; compatibility rendering and package readiness are not implemented
  for this executor.
- The selected document's current disposition YAML binds its exact Draft/Review hashes. Every current non-blocker
  finding is `accepted`; none is unresolved or `revision_required`. Blockers cannot be accepted or waived, and a
  changed Review requires an explicit reset to the new basis rather than carrying decisions forward by position or
  message. Cover Letter uses `review_dispositions.yaml`; Research Statement uses
  `research_statement_review_dispositions.yaml`.
- Rejected/stale Draft candidates left authoritative Draft, user YAML, compatibility Markdown, Typst, and profile
  bytes unchanged. The host or provider wrote no declared run path directly. A configured-provider run had explicit
  Tier 3 consent, persisted no raw output, and used the same current-basis validator/promotion boundary.
- When structured Match views were used, Match and its upstream stages are still current and free of output drift;
  `02_fit_report.md`, `05_criteria_checklist.md`, `07_material_review_checklist.md`,
  `typst/application_package_content.json`, and `typst/application_package.typ` represent the same proposed graph.
- A legacy fallback caused by stale/tampered state, parsed-view or profile-provenance mismatch, or `--llm-drafts` is
  identified as such; it is not presented as current structured Match evidence.
- `02_fit_report.md` separates strong fit, partial fit, and gaps.
- If `typst/cover_letter_content.json` declares `projection.source=cover_letter_draft.json`, its Draft and Review
  hashes still match the current authoritative files, Review has no blocker findings, and each structured Claim
  appears exactly once in `03_cover_letter_draft.md` and both Typst views.
- Draft and Review remain `review_state=proposed`; reviewed status is a derived document-readiness projection. Missing,
  stale, incomplete, or revision-required dispositions keep `requires_human_review=true`. A missing, blocked, stale,
  drifted, or invalid Draft/Review uses the legacy/provider view rather than mixed provenance.
- Research Statement `reviewed` remains a per-document control result only; it does not enter compatibility rendering
  or the package gate in this slice.
- `03_cover_letter_draft.md` application-facing English must not include unsupported claims.
- `04_cv_tailoring_notes.md` tells the user what to adjust in the private CV, but does not rewrite the CV unless asked.
- `05_criteria_checklist.md` covers all extracted essential criteria.
- `07_material_review_checklist.md` tracks cover letter draft and CV tailoring notes review actions before Typst rendering.
- Each non-missing criterion row cites item-level evidence when evidence exists.
- LLM-backed drafts with unknown citations fail validation; unknown citations fail validation by design.
- Every deterministic Match classification remains a proposal. Draft review does not convert it into Decision,
  confirmation, package readiness, or submission readiness.

## Typst Gate

- `typst/cover_letter.typ` directly contains the cover letter text and stable section markers.
- `typst/application_package.typ` directly contains the package text, criteria checklist, and remaining actions.
- Generated `.typ` files use `modernpro-coverletter` or `modernpro-cv` templates rather than Markdown-to-Typst conversion.
- A structured Draft projection uses stable section and Claim markers, escapes Claim text as text, and does not allow
  agent-controlled headings or Typst code to become structure.
- If a rerun writes `*.generated.typ`, the preserved editable source and candidate have been compared and reconciled.
- No `typst/*.generated.typ` candidate remains; pending candidates block package readiness and rendering.
- PDF rendering is optional and requires local Typst.

## Executable Gate Report

- `canisend check-package` checks `APP-Q1` advert integrity, `APP-Q2` evidence traceability, `APP-Q3` artifact
  completeness, and `APP-Q4` unresolved human-review blockers.
- `check-package --write-report` writes `application_gate_report.json`; without the flag the check is read-only.
- The report includes SHA-256 hashes under safe relative labels. Any later input or material edit changes those hashes
  and requires a fresh check.
- A reviewed structured Cover Letter projection binds `cover_letter_draft.json`, `review_findings.json`, and
  `review_dispositions.yaml` hashes and embeds the strict derived document-readiness contract. `check-package`
  independently re-derives that gate. Other missing documents or review blockers still fail package readiness.
- A later `canisend run` marks an existing report `STALE`; rerun `check-package --write-report` after reconciliation.

## Privacy Gate

- Real `profile/`, `jobs/`, `job_leads/`, PDFs, `.env`, and private source URLs are not staged.
- Sensitive declarations are left to the user.
- The final package is a preparation dossier, not proof of submission.
- Agent-assisted reviews report any full private sources read directly.
- LLM-backed CLI runs report which flags/providers were used.

## Profile Input Edit Gate

- Profile improvement ideas are recorded as suggestions before source files are changed.
- Original `profile/` inputs outside `profile/generated/` are edited only through `canisend orchestrate`.
- Any profile-input edit task declares `edits_profile_input: true`, uses privacy tier 2+, depends on a prior review task, and writes only the intended profile source file.
- The run includes both profile edit confirmations: `--confirm-profile-input-edit` and `--confirm-profile-input-edit-again`.

## Manual Submission Gate

- The tool stops at preparation; the user submits manually outside CanISend.
- Portal uploads, account creation, equality monitoring, visa, right-to-work, disability, health, criminal record, conflict, and other sensitive declarations remain user-only.
- Generated material can be described as reviewed only for the checks actually performed.

## Review Order

Review files in this order:

1. `job.yaml`
2. `job_advert.md`
3. `parsed_job.json`
4. `criteria.json`
5. Evidence state and receipts in `evidence_catalog.json` (read bodies only when needed and approved)
6. `criterion_matches.json` proposed classifications and gaps
7. Corrections status and `confirmed_corrections.yaml` only when its Tier 2 body is needed
8. Decision status and `application_decision.yaml` only when its Tier 2 rationale is needed
9. Brief status and `application_brief.yaml` only when its Tier 2 body is needed
10. Brief-stage status and `required_document_plan.json` only when its Tier 2 body is needed
11. `00_preparation_questions.md`
12. `05_criteria_checklist.md`
13. `02_fit_report.md`
14. `03_cover_letter_draft.md`
15. `04_cv_tailoring_notes.md`
16. `07_material_review_checklist.md`
17. `typst/cover_letter.typ`
18. `typst/application_package.typ`
19. `06_final_application_package.md`

Before editing prose, confirm `00_preparation_questions.md` has resolved US English vs UK English, the target writing style, specific motivation, emphasis, risk areas, and details to exclude.
