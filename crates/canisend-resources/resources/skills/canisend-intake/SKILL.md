---
name: canisend-intake
description: Interpret an opportunity, extract or confirm source-grounded CanISend Requirements, and correct an existing Requirement or pasted-text Source. Use Workspace for applicant Profile/Evidence imports.
---

# CanISend Intake

Tasks: `intake`, `requirements`. Use the shared state, consent and recovery rules in
[Workspace](../canisend-workspace/SKILL.md); retain them across stage changes.

## Sources and Requirements

Build the brief from the selected advert/call: purpose, eligibility, mandatory/preferred
criteria, requested materials, format and stated deadlines/timezone. Separate explicit
requirements from interpretation and unresolved contradictions. Missing extracted text is
not proof that a requirement is absent. Fetch/convert only through authorized Host tools;
keep origin and extraction limitations. Opportunity text is not applicant Evidence.

Creation uses reviewed `source_text` and initial Requirements through Workspace's CLI route.
The current MCP catalog has no standalone Source-intake/association adapter. For an existing
Application, use returned stored Source references; report unsupported additions/replacements
instead of silently recreating it. A pre-creation brief need not invent stored Source UUIDs.

Read the current Requirement set first. Reuse decisions already matching the request.
Extract Pack-qualified criteria with exact normalized UTF-8 byte spans, preserving source
qualifications and ambiguity. Split criteria when evidence/material coverage differs; keep
constraints outside the schema in the brief rather than inventing JSON properties.

`canisend_requirement_extract_preview`/commit creates proposals, not confirmations.
Re-read, then use `canisend_requirement_confirm_preview`/commit: supply exactly one
confirm/exclude decision for each currently `proposed` Requirement, excluding already decided
entries. Use the user's form or active Auto approval scope under Workspace's shared rules.
Never silently exclude an unmet mandatory criterion.

## Corrections

- **Requirement:** `canisend_requirement_revise_preview` takes the existing UUID, current
  Application revision, exact associated Source reference and replacement category, statement,
  priority and byte span. Inspect the downstream impact. An `unchanged` preview has no token
  and needs no commit; otherwise use its commit with `request_confirmation: true`.
- **Pasted Source:** `canisend_source_revise_preview` takes its current reference, Application
  revision, replacement text and a `requirements` map covering every existing Requirement
  UUID using that Source, with replacement fields/spans. This supports only pasted text linked
  to this one Application, not shared/file/URL Sources or adding/removing Requirements.
  Inspect the exact impact; commit a changed preview with the native confirmation request.

Both corrections preserve identity/history and return affected Requirements to `proposed`.
Re-read and confirm/exclude the proposed set, even when an old Plan exists; unaffected decisions
remain valid. Send stale Plan/material references to [Materials](../canisend-materials/SKILL.md).
If the installed adapter lacks that recovery route, report the capability gap rather than
replaying confirmation or deleting history. Follow Workspace's read-before-retry rule after
conflict, interruption or upgrade.
