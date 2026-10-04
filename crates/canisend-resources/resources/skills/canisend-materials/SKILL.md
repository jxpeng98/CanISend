---
name: canisend-materials
description: Assess fit, associate Evidence, propose or confirm a Plan, and draft or revise CanISend Deliverables. Use for evidence gaps and content changes; use Review/export for final dispositions and local files.
---

# CanISend Materials

Tasks: `fit-plan`, `drafting`. Use [Workspace](../canisend-workspace/SKILL.md) for
shared state/consent rules and exact Application binding.

## Fit and Plan

Read the verified Pack catalog, including every kind's minimum/maximum count, plus confirmed
Requirements and Evidence. Explain direct, partial, missing or ambiguous support without
inventing a score or probability. Identify the fact/change needed to resolve each gap and
claims the Evidence cannot support. Persist only schema-supported values.

Associate selected Evidence separately and refresh context before proposing the Plan.
Reuse the user's proceed/hold decision, asking only if missing. Use
`canisend_plan_propose_preview`/commit, then `canisend_plan_confirm_preview`/commit with
their authorization requests under Workspace's shared rules, including active Auto approval.
A saved proposal is not confirmation; hold forbids drafting.

Specify purpose, audience, outline, source-backed constraints and Requirement coverage for the
complete material set. Derive kinds/headings from the Pack and opportunity, not an assumed
academic workflow. Keep documents complementary and facts consistent; distinguish completed
work, team contributions and future intentions. Do not strengthen a claim beyond its Evidence.

## Draft and revise

- Ground factual claims in confirmed, associated Evidence. Mark missing support explicitly;
  unresolved placeholders are not final-ready content.
- Initial `canisend_deliverable_draft_preview` takes the **complete** Plan/Pack material set,
  not one appended document. Include cover letter and CV together when both are required.
  Draft fields are `kind`, `title`, `media_type`, `content`; do not invent IDs or legacy claim
  fields. Check candidate support yourself: `canisend_deliverable_audit` reads stored drafts.
- For ready-to-review content use `media_type: application/vnd.canisend.deliverable+json`.
  `content` is a JSON string matching resource `schema.v3.deliverable-document`:
  `format: canisend.deliverable-document/v3`, ordered `fields` and `sections`, and
  `unresolved_fields`. Each field has `key` and `value`; each section has `id`, `heading`
  (string or null) and `body`. Each value/body has `text`, `role`, `evidence`, `requirements`.
  Text is literal; put section headings in `heading`, not Markdown markers in `text`.
  Roles are `evidence-bound` (exact associated Evidence ID/revision/SHA-256 citations),
  `requirement-bound` (confirmed Requirement ID/revision citations), `intent`, or `non-factual`.
  Intent and non-factual text have empty citation arrays. Never relabel a factual assertion
  to avoid a missing citation. Use the actual discovered references, not invented values.
  Academic Pack 1.0.3 requires evidence-bound `candidate-name` and `email` fields. Include
  reviewed contact/recipient fields supported by the bound template. Do not infer identity
  from a title or import unrelated Profile fields automatically.
- Legacy `text/plain` and `text/markdown` remain readable/editable drafts; Markdown is literal.
  Packs with readiness validators require structured revision before approval/export.
  An unresolved field or `[TODO`, `[TBD`, `[PLACEHOLDER` marker blocks final readiness.
- For a Submitted local task, `canisend_local_task_draft_preview` binds its exact generation
  and candidate digest, with private-read consent. The same complete-set rule applies.
- Use `canisend_deliverable_draft_commit` for the initial set. Once materials exist, use
  `canisend_deliverable_revise_preview`/commit for the existing UUID. Follow each current
  schema and applicable user authorization; then audit the stored result with the required read consent.

## Recover corrected inputs

Proposed Requirements go to [Intake](../canisend-intake/SKILL.md) for decisions first.
Reassess fit, then rebuild a `stale` Plan through the existing proposal route, preserving
identity. Confirm the new Plan; preserve existing material kinds/counts because changing the
material set is not supported by this recovery route. Report that gap if it is requested.

Revise each affected material using its existing UUID and current Application revision.
Reassess old content with the required read consent, retain supported/accepted content outside
the change, and route new facts through Evidence confirmation. Materials stay stale until
updated and reviewed again; old exports remain historical. Hand current references and
remaining gaps to [Review/export](../canisend-review-export/SKILL.md).
