# File Contracts

Use this reference when reading, writing, or validating project files.

## Workspace Root

User workspaces are initialized with `canisend init-workspace --workspace <private-workspace>` and contain:

```text
canisend.yaml
.env.example
.gitignore
profile/
jobs/
job_leads/
prompts/
templates/
schemas/
agent-skills/
```

CLI commands read `canisend.yaml` from `--workspace`; configured relative paths are resolved inside that workspace so agents can run from any current directory.

Default config keys:

```yaml
profile_dir: profile
jobs_dir: jobs
job_leads_dir: job_leads
prompt_dir: prompts
template_dir: templates
schema_dir: schemas
agent_skills_dir: agent-skills
```

## Resource Overrides

Application prompts live in `prompts/`. Workspace-local prompts override packaged defaults; missing prompt files fall back to packaged copies.

Agent-readable project skills live in `agent-skills/`. Workspace-local skills are copied defaults that can be edited by the user.

Project-managed Typst templates live in `templates/typst/`. Job-specific generated Typst sources live under each job folder.

## Private Profile

Private local profile data lives in ignored `profile/`.

Expected Typst-first profile files:

```text
profile/profile.yaml
profile/typst/cv.typ
profile/typst/cover_letter_base.typ
profile/typst/research_statement.typ
profile/typst/teaching_statement.typ
profile/generated/*.evidence.md
```

Typst-backed generated evidence contains a `canisend-source-sha256` receipt for the corresponding raw source.
Files created by older versions must be regenerated before resumable Evidence can treat them as current.

Generated evidence citations use `profile/generated/file.evidence.md#Section`.

Generated evidence items should have stable local IDs:

```markdown
## Teaching

- [cv-001] `job`: position: Teaching Assistant, institution: University X
```

New materials should cite item-level evidence as `profile/generated/file.evidence.md#Section/item-id`, for example `profile/generated/cv.evidence.md#Teaching/cv-001`. Section-level citations remain a compatibility fallback, not the preferred new output.

## Discovery Store

All discovery artifacts live in ignored `job_leads/` and are untrusted candidate data, not applicant evidence,
complete adverts, application decisions, or submission records:

```text
job_leads/
  catalog.json
  batches/<source-id>--<adapter>.json
  cache/<source-id>--<adapter>.json
  imports/<source-id>.batch.json
  imports/<source-id>.report.json
  searches/<source-id>.batch.json
  refresh-report.json
```

- `catalog.json` uses `canisend.lead-catalog/v1`. It is the deterministic, order-independent union/ranking view and
  records retained leads, exclusions, aliases, match reasons, counts, policy, and input receipts.
- Every current lead uses `JobLeadV2`: a stable `lead_id`, identity method, original six compatibility fields,
  canonical URL, published metadata, first/last seen timestamps, bounded provenance, aliases, and rank reasons.
- Network and imported source snapshots use `canisend.lead-batch/v1`. Only a validated complete batch is promoted;
  an interrupted or invalid source cannot replace the previous complete batch.
- `cache/*.json` uses `canisend.discovery-cache/v1` and stores only validators, redacted source locator, content hash,
  and timestamps. It never stores response or email bodies.
- `refresh-report.json` uses `canisend.discovery-refresh-report/v1`; local import reports use
  `canisend.discovery-import-report/v1`. Reports contain private-safe counts, paths relative to the workspace, and
  reason codes rather than raw rejected content.
- Host-agent handoff input uses `canisend.discovery-search/v1`. The imported batch does not retain provider names,
  connector/session IDs, credentials, email addresses, private absolute paths, or arbitrary host metadata.
- `discovery-sources.yaml` uses `canisend.discovery-sources/v1`: RSS/Atom accepts one public URL; Greenhouse accepts
  only a public board token; Lever accepts only a public site ID and global/EU region.

Legacy top-level JSON lead lists and `--lead-index` remain readable. New selection should use
`new-job-from-lead --leads-file job_leads/catalog.json --lead-id <lead_id>` so source reordering cannot change the
selected candidate. Selecting a lead creates a lead-only job folder; the full advert still comes from user paste,
local Markdown/text/PDF, or an explicitly approved single HTML/PDF URL.

## Job Folder

Each application task lives in ignored `jobs/<job-slug>/` and contains:

```text
job.yaml
job_advert.md
parsed_job.json
criteria.json
evidence_catalog.json         # core-owned private data plane
criterion_matches.json        # core-owned body-minimized Tier 2 proposed projection
confirmed_corrections.yaml    # optional Tier 2 user-owned input
application_decision.yaml     # optional Tier 2 user-owned input
application_brief.yaml        # optional Tier 2 user-owned input
required_document_plan.json   # core-owned Tier 2 deterministic projection
cover_letter_draft.json       # core-owned Tier 2 proposed structured Draft
research_statement_draft.json # core-owned Tier 2 proposed structured Draft
review_findings.json          # deterministic Cover Letter Review
research_statement_review_findings.json # deterministic Research Statement Review
package_review_findings.json  # deterministic aggregate Package Review
review_dispositions.yaml      # user-owned Cover Letter finding decisions
research_statement_review_dispositions.yaml # user-owned Research Statement finding decisions
package_review_dispositions.yaml # user-owned aggregate finding decisions
workflow/
  state.json                 # rebuildable view
  user-mutations/
    claims/<artifact>/<baseline>.json
    events/<mutation-id>/candidate.yaml  # immutable Tier 2 private body
    events/<mutation-id>/receipt.json    # immutable Tier 1 body-free receipt
  runs/<run-id>/
    inputs/evidence-snapshot.json  # Evidence runs only; immutable private data plane
    task-spec.json           # immutable task contract
    preparation.json         # immutable TaskSpec integrity anchor
    submission.json          # guarded candidate/TaskResult receipt
    candidates/<artifact>.json
    tasks/<task-id>/result.json
    validation/report.json
    terminal-claim.json
    promotion.json
    manifest.json            # terminal run evidence
00_preparation_questions.md
01_job_summary.md
02_fit_report.md
03_cover_letter_draft.md
04_cv_tailoring_notes.md
05_criteria_checklist.md
06_final_application_package.md
07_material_review_checklist.md
08_research_statement.md     # conditional reviewed standalone projection
typst/
  cover_letter.typ
  application_package.typ
  research_statement.typ     # conditional reviewed standalone projection
```

Discovery lead outputs live in ignored `job_leads/`.

## Output Contracts

- `job.yaml`: lightweight tracking fields, including `title`, `institution`, `deadline`, `source_url`, `status`, `english_variant`, `writing_style`, `created_at`, and `updated_at`.
- `job_advert.md`: full advert text. Feed-created jobs start with lead metadata and require manual full advert paste or
  an explicit one-URL import.
- `parsed_job.json`: structured advert data. Missing fields should remain empty or unknown; do not invent.
- `criteria.json`: core-owned, regenerable Stage 2 projection with stable criterion IDs, source spans, extraction
  confidence, confirmation state, unresolved IDs, and orphaned corrections with privacy-safe reason codes. Do not
  edit it directly. It is Tier 2 and may contain user-corrected wording; an agent asks before reading its body.
- `evidence_catalog.json`: strict, core-owned Evidence v1 projection with `available`, valid `empty`, or `unavailable`
  state; manifest/raw-source/generated-evidence receipts; stable content-and-kind-derived Evidence IDs; display
  locators; and normalized evidence bodies. This is a private data-plane artifact. Do not edit it directly.
- `criterion_matches.json`: strict, core-owned deterministic Match v1 projection. It records current Criteria and
  Evidence catalog hashes, matcher strategy/version, one proposed classification and explicit gaps per criterion,
  and opaque `evidence_catalog.json#items/<evidence-id>` references. It never contains evidence bodies, private
  headings, legacy item locators, or private evidence kinds. It remains a Tier 2 job artifact, so an agent asks before
  reading its body. `review_state=proposed` is not Decision or readiness.
- `confirmed_corrections.yaml`: optional strict user-owned overlay keyed by stable criterion ID. Confirm may read it;
  Parse and Confirm must never silently create, normalize, overwrite, or delete it. Manual edits remain valid against
  `schemas/confirmed-corrections.schema.json`. Agent writes use `corrections status|init|update` with one strict
  scoped patch, the current revision/hash, and explicit consent. An empty overlay is not `confirmed_empty`.
- `application_decision.yaml`: strict user-owned Decision. `undecided` is distinct from apply, hold, or skip. Its
  accepted value survives a changed Criteria/Match basis byte for byte; `decision status` derives review-required
  state without writing staleness into the file. Agent writes use `decision status|init|update`.
- `application_brief.yaml`: strict Tier 2 user-owned Brief for confirmed language, writing style, motivation,
  emphasis, exclusions, advert-document requirement-set state, and document choices. It requires a current confirmed
  apply Decision. `brief status` is body-free; Agent writes use `brief status|init|update`, one strict patch, the
  current revision/hash, and explicit consent. A changed Decision basis preserves the Brief for reconfirmation.
- `required_document_plan.json`: deterministic core-owned Tier 2 Brief-stage projection. It binds current Decision,
  advert/Parsed Job requirements, Criteria/Match receipts, and the raw Brief hash; exposes one task per normalized
  requirement; and records unresolved, blocking, and orphaned IDs. Do not edit it directly. An empty Parsed Job list
  remains `unconfirmed` unless the current Brief explicitly records `confirmed_empty` against its basis.
- `cover_letter_draft.json`: strict core-owned Tier 2 Cover Letter Draft. Every applicant-facing prose block is one
  explicit Claim with a stable content-derived ID, kind, support state, current semantic references, and blocker
  codes. It binds exact Parsed Job, Criteria, Evidence, Match, Decision, Brief, and document-plan hashes. It is
  promoted only from guarded host-agent or configured-provider candidate JSON and always remains
  `review_state=proposed`. Configured-provider output contributes only section/Claim semantics; core derives the
  trusted envelope, basis, IDs, generation metadata, and review state.
- `research_statement_draft.json`: strict core-owned Tier 2 Research Statement Draft using the same current-basis,
  Claim-ID, evidence-support, candidate-validation, and guarded-promotion rules. It is host-agent-only in this slice,
  uses `schemas/research-statement-draft.schema.json`, and remains `review_state=proposed`. A blocker-free completeness
  Review requires `research_overview`, `research_contributions`, and `future_agenda` sections.
- `review_findings.json`: deterministic core-owned Tier 2 Review projection for the current structured Draft. It
  records stable blocker/review/warning findings without changing the Draft, user YAML, compatibility views, or
  profile. Unsupported claims, missing required sections, and confirmed Brief exclusion conflicts are blockers;
  structurally supported factual claims and every non-factual Claim-kind classification retain explicit
  semantic-review findings. A later compatible `run` may read this current Review to project a blocker-free Draft.
- `research_statement_review_findings.json`: deterministic Tier 2 Review projection for the current Research
  Statement Draft. It uses the shared Review Findings schema with strategy
  `deterministic.research_statement_review`; it feeds only Research Statement dispositions/readiness, never Cover
  Letter dispositions, compatibility projection, or package readiness.
- `package_review_findings.json`: deterministic core-owned Tier 2 aggregate Review. It binds the exact Parsed Job,
  Brief, Required Document Plan, derived execution plan, and every observed Draft/Review/disposition/readiness hash.
  Missing required documents and exact repeated-assertion receipt disagreements are blockers; semantic alignment is
  explicit review work. Correction proposals target a document and Claim set and require a new guarded Draft
  candidate. This file is not user-owned package readiness or submission state.
- `review_dispositions.yaml`: strict user-owned Tier 2 finding decisions bound to the exact Draft and Review hashes.
  It is the Cover Letter artifact and defaults missing `document_kind` to `cover_letter` for backward readability.
- `research_statement_review_dispositions.yaml`: independent strict user-owned Tier 2 Research Statement finding
  decisions. It cannot share a CAS baseline, claim directory, or recovery receipt with Cover Letter dispositions.
  Agent writes for either file use `review-dispositions status|init|update --document-id <id>` with one finding, the
  current revision/hash, and explicit consent. `accepted` is valid only for non-blockers; `revision_required` keeps
  that document out of readiness. A changed Draft/Review preserves the file but requires
  `reset_for_current_review` before new findings are edited.
- `package_review_dispositions.yaml`: independent strict user-owned Tier 2 aggregate finding decisions bound to the
  exact package Review hash. Agent writes use `package-review status|init|update`, one package finding, the current
  revision/hash, and explicit consent. Aggregate blockers cannot be accepted; a changed package Review preserves
  the old file until `reset_for_current_package_review` explicitly rebinds it.
- `schemas/document-readiness.schema.json`: derived Cover Letter or Research Statement readiness contract. It is
  recomputed from the selected document's current Draft, Review, and dispositions; it is not a mutable approval file
  and does not establish whole-package readiness. Only Cover Letter readiness is embedded in current compatibility
  content and package checks.
- `schemas/document-execution-plan.schema.json`: body-free, read-only fan-out projection derived from the exact
  Required Document Plan hash. It distinguishes blocked, omitted, dispatchable, planned-unavailable, and
  unregistered document work without persisting a second workflow state or claiming package readiness.
- `schemas/application-package-readiness.schema.json`: body-free derived aggregate readiness contract. It binds the
  exact required-document receipts, package Review, and current package dispositions, and distinguishes `blocked`,
  `review_required`, `revision_required`, and `reviewed`. It is not a mutable approval file, rendering approval,
  submission readiness, or submission evidence.
- `workflow/user-mutations/`: private immutable candidates plus cooperative single-winner claims and immutable
  receipts. Candidate/YAML bodies and corrected Criteria are Tier 2. Claims and receipts never include correction
  text, rationale, Brief values, finding messages, or document source text; the receipt is Tier 1 and validates against
  `schemas/user-mutation-receipt.schema.json`.
  Candidates use private-file mode (0600 on POSIX) and persist after semantic reset/clear/withdraw for audit/recovery. Corrections history
  likewise retains old corrected bodies. Deleting selected events or the whole private job is a separate retention
  action; there is no automatic secure-erase guarantee.
- `workflow/runs/*/task-spec.json`: immutable task contract. `allowed_writes` is explicitly marked
  `write_authority: core_service`; a host supplies scratch candidate JSON through `stage submit` rather than writing
  candidate or result paths itself. Evidence TaskSpecs name only their own job-local immutable snapshot; Match
  TaskSpecs name only current `criteria.json` and `evidence_catalog.json`; Brief TaskSpecs name current job-local
  advert/Parsed Job, Criteria, Match, Decision, and Brief inputs. Draft TaskSpecs name the seven current Tier 2
  structured/user inputs and allow only core-owned candidate/result writes. Cover Letter configured-provider Draft uses the same
  exact TaskSpec paths with privacy tier 3 and consent `send-private-draft-inputs-to-provider`; raw provider output
  is not an artifact. Review TaskSpecs add the promoted Draft and remain deterministic. Package Review TaskSpecs
  name the exact existing aggregate inputs and remain body-free despite reading Tier 2 content locally. Non-document tasks keep the
  frozen 1.0 wire shape. Document-scoped Draft/Review tasks use backward-readable 1.1 records and bind TaskSpec,
  result, submission, validation, manifest, terminal claim, promotion receipt, and WorkflowState to the same stable
  Required Document Plan ID. The current plan's normalized kind selects the document-specific adapter, schema, and
  authoritative target. Workflow stage instances are unique by `(stage, document_id)`.
- `workflow/runs/*/inputs/evidence-snapshot.json`: immutable Evidence input written by the core during prepare. It may
  duplicate normalized profile evidence and remains until the user removes the private run or job. Resumable
  Evidence does not accept a workspace-external profile root.
- `00_preparation_questions.md`: grill-me checklist for confirming US English vs UK English, writing style, specific motivation, emphasis, risks, and excluded details before treating materials as final.
- `02_fit_report.md`, `03_cover_letter_draft.md`, `04_cv_tailoring_notes.md`, `05_criteria_checklist.md`:
  compatibility Markdown review artifacts. A workspace `run` projects a current, validated, blocker-free structured
  Cover Letter into `03_cover_letter_draft.md` only when the same run can use current structured Match, the parsed
  view and configured profile provenance agree, and `--llm-drafts` is absent. Otherwise the compatible legacy or
  provider path remains in effect.
- `07_material_review_checklist.md`: management artifact for cover letter draft, CV tailoring notes, placeholders, item-level citations, and manual follow-up actions.
- `08_research_statement.md`: conditional standalone compatibility view emitted only for an exact current reviewed
  Research Statement. It is not a required package file.
- `typst/cover_letter.typ`: editable Typst source for the final cover letter, with stable `// CANISEND: section ...` markers.
- `typst/application_package.typ`: editable Typst source for the final package, including remaining actions and review sections.
- `typst/research_statement.typ`: conditional standalone reviewed Research Statement with stable Claim markers. It
  is not embedded in the application-package source.
- `typst/.canisend-generated.json`: generated-hash metadata used to avoid overwriting user-edited Typst files.
- `typst/.canisend-research-generated.json`: separate generated-hash metadata for the optional standalone Research
  Statement, keeping its edit lifecycle outside package-gate inputs.
- `typst/*.generated.typ`: candidate regeneration written only when the corresponding editable `.typ` has diverged
  from its generated baseline.
- `application_gate_report.json`: optional machine-readable `APP-Q*` report written only by an explicit
  `check-package --write-report` request. APP-Q5 independently rederives aggregate package readiness and fails
  closed when its exact Review/decision receipts are absent, invalid, stale, incomplete, or changed.

The pipeline may emit content JSON compatibility/debug artifacts under `typst/`, but agents should treat the `.typ` files as the editing contract.

In deterministic workspace runs, a current validated Match graph supplies the proposal view used by
`02_fit_report.md` and `05_criteria_checklist.md`, the structured essential-criteria portion of
`07_material_review_checklist.md`, `typst/application_package_content.json`, and
`typst/application_package.typ`. The same proposal text may also appear in `06_final_application_package.md`. These
derived files do not change `review_state=proposed` into user confirmation,
Decision, or readiness. Stale or drifted/tampered structured artifacts, a mismatching parsed view, a profile override,
or `--llm-drafts` cause safe legacy/provider fallback rather than mixed-provenance output.

When that Match guard passes, a current validated Draft plus current deterministic Review with zero blocker findings
may also supply `03_cover_letter_draft.md`, `typst/cover_letter_content.json`, the Cover Letter portion of
`typst/application_package_content.json`, and both Typst sources. Each structured Claim is rendered once and the
content JSON records exact Draft/Review hashes and `requires_human_review`. Open review/warning findings remain open;
the projection is not a reviewed/final package. Missing, blocked, stale, drifted, or invalid Draft/Review artifacts,
direct library use without workspace provenance, profile override, or `--llm-drafts` use safe fallback instead.

When the same Match guard passes, an exact current Research Statement Draft, deterministic blocker-free Review, and
audited complete dispositions may supply standalone `08_research_statement.md` and
`typst/research_statement.typ`. The projection records exact Draft, Review, disposition, readiness, and Markdown
hashes and renders every Claim once. It remains outside application-package content, required package files, APP-Q
issues, and package input hashes. A prior projection that becomes ineligible is replaced by body-free unavailable
views; an edited Typst primary is preserved with a reconciliation candidate.

Evidence snapshots, Evidence candidates, promoted Evidence catalogs, user mutation YAML/private candidates,
corrected Criteria, Brief-stage candidates/plans, structured Draft candidates/artifacts, and Review findings are the
Tier 2 private data plane. Workflow
state, task/result and mutation receipts, mutation claims, validation and promotion records, manifests, errors,
ordinary CLI/AgentResponse output, and Match output are the control plane and must contain only privacy-safe paths,
hashes, semantic IDs, classifications, reason/blocker codes, and counts.
