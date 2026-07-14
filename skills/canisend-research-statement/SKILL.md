---
name: canisend-research-statement
description: Use when revising, strengthening, tailoring, or reviewing a research statement for an academic or research job application with CanISend evidence, profile receipts, job criteria, or private workspace materials.
---

# CanISend Research Statement

Focus only on research statements. Do not prepare a full application package unless the user explicitly switches to the main `canisend` workflow.

## Boundaries

- Do not submit applications, fill portals, create accounts, upload materials, or answer sensitive declarations.
- Do not fabricate applicant evidence, publications, grants, methods, collaborations, or institutional fit.
- Ask before reading full private CVs, statements, references, full job adverts, source URLs, or enabling LLM-backed commands.
- Prefer generated evidence under `profile/generated/` and cite gaps instead of inventing support.

## Required References

Read only what the current task requires:

- `../canisend/references/privacy.md`: private-material handling and quoting limits.
- `../canisend/references/quality-gates.md`: evidence and readiness gates.
- `../canisend/references/file-contracts.md`: profile evidence and job artifact paths.
- `../canisend/references/typst-profile.md`: Typst-first profile source handling.

## Workflow

1. Identify the workspace, run `canisend agent context`, and inspect body-free `canisend documents status`.
2. If Research Statement is confirmed for `prepare`, obtain approval before reading the Tier 2
   `required_document_plan.json`, then copy the exact stable ID mapped to `normalized_kind=research_statement`. Never
   infer the ID from a label, position, output path, or prose.
3. Prepare the host-agent task with `canisend stage prepare --stage draft --mode host-agent --document-id <id>
   --format json`. Configured-provider execution is not supported for Research Statement.
4. After the returned `read-private-draft-inputs` consent is approved, read only TaskSpec-declared inputs. Prefer
   current Evidence IDs and relevant Criteria over raw private files.
5. Create fresh private scratch JSON matching `schemas/research-statement-draft.schema.json`. Use the exact TaskSpec
   job/document identity and fingerprint plus the seven declared input hashes as Draft basis. Every applicant-facing
   block is one Claim with a core-recomputable stable ID. Include `research_overview`, `research_contributions`, and
   `future_agenda`; map factual claims to current Evidence, and map future intent to Criteria or confirmed Brief
   emphasis. Unsupported or partial facts retain their declared blocker.
6. Pass scratch through `stage submit`, then pass its immutable TaskResult to `stage apply`. Never write the run
   directory or `research_statement_draft.json` directly.
7. Run `stage run --stage review --mode deterministic --document-id <same-id>`. Read finding bodies only after Tier 2
   approval. Missing required sections, unsupported facts, and confirmed exclusion conflicts are blockers; semantic
   support and Claim-kind findings require human review.
8. Use body-free `review-dispositions status --document-id <same-id>` before reading finding bodies. Initialize
   `research_statement_review_dispositions.yaml` only with explicit user-owned write consent, then submit one
   `set_finding_disposition` patch at a time with the latest revision/hash and the same ID. Blockers cannot be
   accepted; a changed Draft/Review requires an explicit `reset_for_current_review` patch.
9. Complete current acceptances derive Research Statement document readiness while Draft and Review remain
   `proposed`. Run the compatible pipeline only after readiness is `reviewed`; it may then project each Claim once to
   standalone `08_research_statement.md` and `typst/research_statement.typ`. Do not embed that projection in the
   application package or claim package readiness.
10. If a later run reports the Research Statement projection unavailable or creates
    `research_statement.generated.typ`, do not reuse the old generated wording as current. Reconcile an edited Typst
    primary explicitly before rendering.
11. Before presenting wording for human review, check `../canisend/references/quality-gates.md` and clearly label the
   Draft and Review as `proposed`.
