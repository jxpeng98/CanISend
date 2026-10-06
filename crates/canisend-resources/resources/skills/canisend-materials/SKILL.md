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

<!-- canisend:typst-preview:start -->
## Typst Preview templates

For user-owned standalone Typst files, import these exact published packages. No template
repository checkout or copied package implementation is needed. Keep the full version in every
import; change it only during a reviewed upgrade, then compile and inspect the affected PDFs.

| Material | Package import | Entry point |
| --- | --- | --- |
| Academic CV | `@preview/modernpro-cv:2.1.2` | `cv` |
| Cover letter | `@preview/modernpro-coverletter:1.0.3` | `coverletter` |
| Research or teaching statement | `@preview/modernpro-coverletter:1.0.3` | `statement` |

Define the same reviewed `profile` values inline in each document. These fictional values are
syntax examples, not applicant facts; replace them with user-reviewed information:

```typst
#let profile = (
  name: [Fictional Candidate],
  contacts: ((text: [candidate\@example.invalid], link: "mailto:candidate@example.invalid"),),
)
```

Use one of the following snippets in each file, after its profile definition. For a CV, prefer
one column for academic applications and reliable PDF reading order:

```typst
#import "@preview/modernpro-cv:2.1.2": cv, section, summary
#show: cv.with(profile: profile, theme: (font: "Libertinus Serif"), options: (last-updated: false))
#section("Research Profile")
#summary[Replace with a reviewed research summary.]
```

For a cover letter, supply reviewed recipient values and write short paragraphs:

```typst
#import "@preview/modernpro-coverletter:1.0.3": coverletter
#show: coverletter.with(
  profile: profile,
  recipient: (organization: [Fictional Institution], greeting: [Dear Search Committee,]),
  theme: (font: "Libertinus Serif"),
)
Replace with reviewed letter text.
```

Use the same package for each statement; set its title and headings to the actual requirements:

```typst
#import "@preview/modernpro-coverletter:1.0.3": statement
#show: statement.with(profile: profile, title: [Research Statement], theme: (font: "Libertinus Serif"))
= Research agenda
Replace with reviewed statement text.
```

Compile a user-owned file with `typst compile cv.typ cv.pdf` (or the corresponding letter or
statement paths) in the external Typst CLI. Its first import of a version requires a package
download; cached packages work offline. Ensure the selected font is available to that compiler.
Escape Typst syntax in user text, including `\@` in content blocks; omit optional icons and
photos unless requested. Inspect contacts, page breaks and extracted PDF text before delivery.

CanISend-managed Deliverables use the verified template in their exact bound Workflow Pack,
through structured content, review and guarded export. The embedded renderer works offline and
does not compile arbitrary Preview imports or user-authored Typst. A standalone PDF is not a
CanISend-reviewed export. Do not replace managed projections with these examples or treat template
placeholders as Evidence. Existing Applications retain their original Pack/template identity.

After upgrading the CLI, run `canisend --workspace PATH workspace upgrade` to refresh project
Skills, including this template guidance; global Skills use `host setup --host HOST --scope global`.
Reconnect the Host and reread the installed Materials Skill for current pins and APIs. Bundled
Host guides delegate authoring details to this Skill; a durable pointer in user-owned `AGENTS.md`
does not need new version numbers on each template upgrade. Exported Agent packs still contain
resource snapshots; use the selected Workspace/Host's managed Skills for ongoing work, and
regenerate a pack when its protocol or workflow instructions change. Never overwrite a user's
`AGENTS.md` or `CLAUDE.md`.

API references: [CV](https://typst.app/universe/package/modernpro-cv/),
[letters and statements](https://typst.app/universe/package/modernpro-coverletter/),
[package versions and caching](https://github.com/typst/packages/blob/main/README.md).
<!-- canisend:typst-preview:end -->
