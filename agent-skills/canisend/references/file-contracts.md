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
typst/
  cover_letter.typ
  application_package.typ
```

RSS and Atom lead outputs live in ignored `job_leads/`.

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
- `workflow/user-mutations/`: private immutable candidates plus cooperative single-winner claims and immutable
  receipts. Candidate/YAML bodies and corrected Criteria are Tier 2. Claims and receipts never include correction
  text or rationale; the receipt is Tier 1 and validates against `schemas/user-mutation-receipt.schema.json`.
  Candidates use private-file mode (0600 on POSIX) and persist after semantic reset/clear/withdraw for audit/recovery. Corrections history
  likewise retains old corrected bodies. Deleting selected events or the whole private job is a separate retention
  action; there is no automatic secure-erase guarantee.
- `workflow/runs/*/task-spec.json`: immutable task contract. `allowed_writes` is explicitly marked
  `write_authority: core_service`; a host supplies scratch candidate JSON through `stage submit` rather than writing
  candidate or result paths itself. Evidence TaskSpecs name only their own job-local immutable snapshot; Match
  TaskSpecs name only current `criteria.json` and `evidence_catalog.json`.
- `workflow/runs/*/inputs/evidence-snapshot.json`: immutable Evidence input written by the core during prepare. It may
  duplicate normalized profile evidence and remains until the user removes the private run or job. Resumable
  Evidence does not accept a workspace-external profile root.
- `00_preparation_questions.md`: grill-me checklist for confirming US English vs UK English, writing style, specific motivation, emphasis, risks, and excluded details before treating materials as final.
- `02_fit_report.md`, `03_cover_letter_draft.md`, `04_cv_tailoring_notes.md`, `05_criteria_checklist.md`: evidence-grounded Markdown review artifacts.
- `07_material_review_checklist.md`: management artifact for cover letter draft, CV tailoring notes, placeholders, item-level citations, and manual follow-up actions.
- `typst/cover_letter.typ`: editable Typst source for the final cover letter, with stable `// CANISEND: section ...` markers.
- `typst/application_package.typ`: editable Typst source for the final package, including remaining actions and review sections.
- `typst/.canisend-generated.json`: generated-hash metadata used to avoid overwriting user-edited Typst files.
- `typst/*.generated.typ`: candidate regeneration written only when the corresponding editable `.typ` has diverged
  from its generated baseline.
- `application_gate_report.json`: optional machine-readable `APP-Q*` report written only by an explicit
  `check-package --write-report` request.

The pipeline may emit content JSON compatibility/debug artifacts under `typst/`, but agents should treat the `.typ` files as the editing contract.

Evidence snapshots, Evidence candidates, and promoted Evidence catalogs are the private data plane and may contain
profile text. User mutation YAML, private candidates, and corrected Criteria may contain private correction/rationale
bodies. Workflow state, task/result and mutation receipts, mutation claims, validation and promotion records,
manifests, errors, ordinary CLI/AgentResponse output, and Match output are the control plane and must contain only
privacy-safe paths, hashes, semantic IDs, classifications, reason codes, and counts.
