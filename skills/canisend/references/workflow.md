# Workflow

Use this reference for the normal installed-package workflow. From a development checkout, prefix commands with `uv run`.

## 1. Initialize Or Inspect Workspace

For a new user workspace:

```bash
canisend init-workspace --workspace <private-workspace>
canisend agent context --workspace <private-workspace> --format json
```

For an existing workspace, start with `agent context`. Use `doctor` when a human-readable environment diagnostic is
also useful, and resolve missing profile, prompt, skill, provider, or Typst items before generating
application-facing material.

After package upgrades:

```bash
canisend update-workspace --workspace <private-workspace>
canisend agent context --workspace <private-workspace> --format json
```

Use `--overwrite` only when the user intentionally wants packaged defaults to replace local prompt, template, or skill edits.

## 2. Prepare Profile Evidence

The user should keep real CV and statement sources in ignored `profile/`. In Typst-first workflows, these are already-written `modernpro-cv` and `modernpro-coverletter` sources.

Regenerate normalized evidence whenever profile sources change:

```bash
canisend extract-profile-evidence --workspace <private-workspace>
```

Typst-backed generated evidence now includes a source-hash receipt. If Evidence reports
`evidence.source_receipt_missing` for older generated output, or `evidence.source_receipt_stale` after its raw source
changes, rerun this command. Do not repair the receipt manually.

Agents should read generated evidence from `profile/generated/`, not directly rely on prose claims in the private CV. New claims should cite item-level citations such as `profile/generated/cv.evidence.md#Teaching/cv-001`.

If generated evidence is incomplete, first report the gap. Read raw profile sources only with user approval, because in agent-assisted mode the content read by the agent may be processed by the agent model provider. `extract-profile-evidence --llm-augment` must also be explicit opt-in; it rejects augmented items that do not cite a local source chunk.

## 3. Fetch And Select Leads

Fetch jobs.ac.uk RSS leads locally:

```bash
canisend fetch-jobs-ac-uk \
  --workspace <private-workspace> \
  --feed-url "<jobs-ac-uk-rss-url>" \
  --include "<keyword>" \
  --exclude "<keyword>"
```

Use the generic command for another stable RSS or Atom source:

```bash
canisend fetch-job-feed \
  --workspace <private-workspace> \
  --source-name "<source label>" \
  --feed-url "<rss-or-atom-url>"
```

Feed leads are discovery records, not full adverts. Ask the user to choose a lead index unless they already provided
one. Generic source files are written under `job_leads/<source-name>.json` and should be passed with `--leads-file`.

## 4. Create One Job Workspace

From a feed lead:

```bash
canisend new-job-from-lead \
  --workspace <private-workspace> \
  --lead-index <index> \
  --institution "<institution>" \
  --deadline "YYYY-MM-DD"
```

Manual job creation:

```bash
canisend new-job \
  --workspace <private-workspace> \
  --title "<job title>" \
  --institution "<institution>" \
  --deadline "YYYY-MM-DD" \
  --source-url "<source-url>"
```

Paste or import the full selected advert into `jobs/<job-slug>/job_advert.md` before relying on parser output. Direct
intake may use `new-job --advert-file <advert.pdf|advert.md|advert.txt>`.
`new-job --source-url` records metadata only. Use `--fetch-url` only when the user explicitly asks to import that one
supplied HTML or PDF advert; it does not authorize site crawling or search-result scraping.

After creating or selecting a job, any fresh host session can resume with:

```bash
canisend agent context \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --format json
```

The workspace is authoritative; no previous prompt or chat transcript is required.

## 5. Run Or Resume Evidence And Parse

Evidence and Parse are independent after intake and can run in either order. Run Evidence deterministically:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage evidence \
  --mode deterministic \
  --format json
```

Evidence prepares one immutable private
`workflow/runs/<run-id>/inputs/evidence-snapshot.json`; its TaskSpec names only that job-local input. It promotes a
strict `evidence_catalog.json` with stable content-derived IDs and `available`, `empty`, or `unavailable` state. The
snapshot, candidate, and catalog may contain normalized profile bodies and duplicate them until the user removes the
private run or job directory. Workflow state, receipts, manifests, errors, ordinary command output, and AgentResponse
extensions do not contain those bodies.

This resumable slice accepts only profiles inside the workspace. It rejects a workspace-external profile root,
unsafe manifest paths, symlinks, hard-link aliases, non-regular files, changing inputs, and versioned size-limit
violations. These restrictions do not redefine compatibility behavior of unrelated legacy commands.

Inspect Parse without writing state:

```bash
canisend stage status \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --format json
```

Use the deterministic executor when local parsing is appropriate:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage parse \
  --mode deterministic \
  --format json
```

For current-host parsing, first obtain explicit approval to read the full advert, then run `stage prepare --mode
host-agent`. Read its immutable TaskSpec, create candidate JSON in a fresh scratch file, and pass that file through
`stage submit --candidate-file`. The guarded submit service writes the declared candidate and TaskResult; after
review, pass the returned paths to `stage apply`. TaskSpec and receipts are read-only through their AgentResponse
references; never write or modify a declared run path or `parsed_job.json` directly.
Submit and Apply reject stale inputs, output drift, undeclared paths, path aliases, wrong hashes, invalid schemas, and
source receipts that do not resolve to the advert.

An unchanged deterministic rerun is a cache hit. Advert or relevant job metadata changes make Parse stale; profile
evidence, writing preferences, package status, and downstream artifacts do not.

Host-agent execution currently applies only to Parse. Each prepared task has a separate immutable preparation
receipt. If its inputs, upstream dependencies, or protected output change before promotion, do not prepare a parallel
task: cancel the active one first, preserving its audit trail and candidate:

```bash
canisend stage cancel \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage <stage> \
  --format json
```

Cancellation releases the active-task slot but does not authorize overwriting a drifted authoritative output.

## 6. Project And Confirm Stable Criteria

After Parse is current, project its criteria into the Stage 2 semantic contract:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage confirm \
  --mode deterministic \
  --format json
```

Confirm writes `criteria.json` through the same candidate-validation and atomic-promotion boundary as Parse. Each
criterion has a stable ID, essential/desirable importance, categorical confidence, source-known state, and separate
user-confirmation state. Missing receipts remain unknown; ambiguous receipts expose candidate spans without silently
selecting one. The operation returns `review_required` while criteria remain unconfirmed or source-unknown, a
correction needs reconciliation, or an empty extraction remains unconfirmed. A technically successful run is not
evidence of user confirmation.

`confirmed_corrections.yaml` is an optional user-owned typed overlay. Confirm reads it; neither Parse nor Confirm
creates, rewrites, or deletes it. Until a scoped compare-and-swap update command is available, validate manual edits
against `schemas/confirmed-corrections.schema.json`, preserve the expected current file, and rerun Confirm. An agent
must not change the overlay without explicit instruction. If the advert source receipt changes, the old correction
remains present and Confirm reports a privacy-safe reconciliation reason instead of silently reassigning it.

A minimal unambiguous confirmation copies hashes from the current `criteria.json` record:

```yaml
schema_version: 1.0.0
job_id: <job-folder-name>
revision: 1
updated_at: 2026-07-11T12:00:00Z
criteria:
  - correction_id: correction_<32-lowercase-hex>
    criterion_id: criterion_<32-lowercase-hex>
    target_source_sha256: <source-span-text-sha256>
    target_criterion_sha256: <parsed-text-sha256>
    confirmation: confirmed
    record_state: active
    confirmed_at: 2026-07-11T12:00:00Z
```

Use `confirmation: corrected` plus `corrected_text` to replace the projected wording. For an ambiguous source, also
copy one candidate's paired `source_occurrence` and `source_anchor_sha256`; Confirm accepts it only when that context
anchor is unique in the current candidate set.

## 7. Propose Durable Criterion Matches

After Confirm and Evidence are current, run Match deterministically:

```bash
canisend stage run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage match \
  --mode deterministic \
  --format json
```

Match reads only `criteria.json` and `evidence_catalog.json`. It writes one canonical `strong`, `partial`, `weak`,
`missing`, or `unknown` classification per criterion, with matcher provenance, explicit gaps, and opaque
`evidence_catalog.json#items/<evidence-id>` references. It never copies evidence text, private headings, legacy item
labels, or evidence kinds into `criterion_matches.json`.

An empty Evidence catalog and unavailable Evidence produce different `unknown` gaps; an available catalog with no
relevant support produces `missing`. Unknown Criteria extraction blocks Match and routes back to Criteria review.
Every classification has `review_state=proposed`. Review the proposals; they are neither a user-owned application
Decision nor evidence that a package is ready. Editing either catalog directly is forbidden—rerun its owning stage.

## 8. Generate Draft Package

Deterministic baseline:

```bash
canisend run --workspace <private-workspace> --job jobs/<job-slug>
```

LLM-backed parse and draft generation require explicit opt-in and provider configuration:

```bash
canisend run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --llm-parser \
  --llm-drafts
```

Use only `--llm-parser` when the user wants structured parsing but not drafted prose. Use only `--llm-drafts` when deterministic parsing is sufficient.

Always ask before enabling LLM-backed flags or a command provider for a real workspace, because those modes can send selected private advert, profile, evidence, and draft context to the configured provider. If the user has not opted in, run the deterministic baseline and report any gaps for manual review.

## 9. Review Before Rendering

Review, in order:

1. `parsed_job.json`
2. `criteria.json`
3. Evidence state and receipts in `evidence_catalog.json` (read its private bodies only when needed and approved)
4. Proposed classifications and gaps in `criterion_matches.json`
5. `00_preparation_questions.md`
6. `05_criteria_checklist.md`
7. `02_fit_report.md`
8. `03_cover_letter_draft.md`
9. `04_cv_tailoring_notes.md`
10. `07_material_review_checklist.md`
11. `typst/cover_letter.typ`
12. `typst/application_package.typ`
13. `06_final_application_package.md`

Apply `quality-gates.md` before treating any output as usable.
In particular, check language/style confirmation, item-level citations, unsupported claims, required-document coverage, and private-file safety before presenting a package as ready.

Repeated generation protects edited Typst sources. If `run` reports a `*.generated.typ` candidate, compare it with the
preserved user-edited `.typ` file and merge intentionally before rendering.

Use `canisend check-package --write-report` when a machine-readable `application_gate_report.json` is useful. Without
that flag, the check remains read-only.

In agent-assisted mode, also report which private sources were read directly, which LLM-backed CLI flags were used, and which remaining claims need manual confirmation.

## 10. Optional Typst Rendering

Render only when the user asks for PDFs or needs local PDF review:

```bash
canisend render-typst --workspace <private-workspace> --job jobs/<job-slug>
```

Rendering requires a local `typst` binary. Source generation does not.

## 11. Manual Submission

The tool stops at preparation. The user manually handles portal upload, eligibility declarations, equality monitoring, right-to-work, disability, visa, conflict, criminal record, and other sensitive form answers.
