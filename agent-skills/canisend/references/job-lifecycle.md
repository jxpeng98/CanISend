# Job Lifecycle

Use this reference to decide the next action from current files and `job.yaml` status.

## Status Values

- `status: new`: job folder exists, but the advert may be empty or manually pending.
- `status: lead_imported`: job was created from an RSS or Atom feed lead. The full advert still needs manual paste or import.
- `status: advert_imported`: job was created with a local advert file.
- `status: packaged`: pipeline generated parsed data, reports, drafts, final package, and Typst sources.

## Next Action By State

### No workspace

Run:

```bash
canisend init-workspace --workspace <private-workspace>
canisend doctor --workspace <private-workspace>
```

### Workspace exists, no job

Fetch leads or create a job:

```bash
canisend fetch-jobs-ac-uk --workspace <private-workspace> --feed-url "<rss-url>"
canisend fetch-job-feed --workspace <private-workspace> --source-name "<source>" --feed-url "<feed-url>"
canisend new-job-from-lead --workspace <private-workspace> --lead-index <index> --institution "<institution>"
```

### `status: lead_imported`

Open `jobs/<job-slug>/job_advert.md`. If the `Full Advert` section still contains placeholder text, ask the user to paste or explicitly import the full advert. Do not rely on a feed description alone for final criteria matching.

### `status: new` or `status: advert_imported`

Regenerate evidence, then either run the compatible package pipeline or advance the resumable Decision Spine:

```bash
canisend extract-profile-evidence --workspace <private-workspace>
canisend run --workspace <private-workspace> --job jobs/<job-slug>
```

```bash
canisend stage run --workspace <private-workspace> --job jobs/<job-slug> --stage evidence --mode deterministic --format json
canisend stage run --workspace <private-workspace> --job jobs/<job-slug> --stage parse --mode deterministic --format json
canisend stage run --workspace <private-workspace> --job jobs/<job-slug> --stage confirm --mode deterministic --format json
canisend stage run --workspace <private-workspace> --job jobs/<job-slug> --stage match --mode deterministic --format json
```

Evidence and Parse are independent; Match requires both current Evidence and Confirm. Every Match result remains a
proposal for review, not an application Decision. Between Confirm and Match, use `corrections status` and the
explicit-consent init/update operations when confirmation is needed; empty initialization is fingerprint-neutral,
while every semantic correction requires a Confirm rerun before another patch. After Match, use `decision status` and
the explicit-consent init/update operations to record the
user's apply/hold/skip choice.

After a current confirmed apply Decision, continue through body-free Brief status and explicit-consent initialization,
then build the core-owned plan:

```bash
canisend brief status --workspace <private-workspace> --job jobs/<job-slug> --format json
canisend brief init --workspace <private-workspace> --job jobs/<job-slug> --confirm-user-owned-write --format json
canisend stage run --workspace <private-workspace> --job jobs/<job-slug> --stage brief --mode deterministic --format json
```

Use `brief update` with one strict patch and the latest revision/hash to resolve fields and document choices. The
Brief and plan bodies are Tier 2 ask-first; status is body-free. When the plan has one confirmed `prepare` Cover
Letter and no blockers, prepare a host-agent Draft task after Tier 2 approval. Submit strict candidate JSON through
the returned TaskSpec and apply it through the guarded CLI; then run deterministic `stage run --stage review`.
Draft and Review bodies remain Tier 2, and neither result alone is package readiness.

After current deterministic Match, `canisend run --workspace ...` with the configured workspace profile and no
`--llm-drafts` projects the proposed Match view consistently into `02_fit_report.md`, `05_criteria_checklist.md`,
structured checks in `07_material_review_checklist.md`, `typst/application_package_content.json`, and
`typst/application_package.typ`. If Match or upstream state is stale, a structured artifact is drifted/tampered, the
parsed view or profile provenance differs, or the run uses `--llm-drafts`, the pipeline keeps its compatible
legacy/provider path. A fallback does not turn a proposal into a Decision or establish readiness.

Use `extract-profile-evidence --llm-augment`, `--llm-parser`, or `--llm-drafts` only when provider config is ready and the user explicitly wants model-backed steps.

### `status: packaged`

Review quality gates before rendering:

1. Confirm `parsed_job.json` matches the advert.
2. Confirm criteria checklist covers essential criteria.
3. Confirm strong claims cite `profile/generated/` evidence.
4. Confirm `typst/cover_letter.typ` matches the edited cover letter.
5. Render Typst only when the user wants PDFs.

## Missing Or Inconsistent Files

- Missing `job.yaml`: recreate the job folder from a lead or manual job metadata.
- Missing `job_advert.md`: create it before parsing.
- Missing `profile/generated/*.evidence.md`: run `extract-profile-evidence`.
- Evidence reason `evidence.source_receipt_missing` or `evidence.source_receipt_stale`: rerun
  `extract-profile-evidence`; do not patch source-hash receipts manually.
- Workspace-external `profile_dir`: use a profile inside this workspace for resumable Evidence. Do not bypass the
  TaskSpec v1 boundary with parent or absolute paths.
- Missing `parsed_job.json`: run `canisend run`.
- Missing `criteria.json`: run resumable Parse, then Confirm.
- Missing `evidence_catalog.json`: run deterministic Evidence; do not create or edit the catalog directly.
- Missing `criterion_matches.json`: make Confirm and Evidence current, then run deterministic Match.
- `criterion_matches.json` exists: review every `review_state=proposed` classification and explicit gap before using
  it for strategy. Match does not create Decision, Brief, or readiness.
- Missing `confirmed_corrections.yaml`: status is read-only; initialize only with explicit consent. An empty file is
  not a confirmed-empty extraction.
- Active corrections changed: rerun Confirm before another semantic correction or Match; empty initialization alone
  is fingerprint-neutral.
- Missing `application_decision.yaml`: initialize an undecided record only with explicit consent after current Match.
- Decision basis is review-required: preserve the stored value, review current Criteria/Match, then reconfirm through
  a new scoped patch and current revision/hash. Do not edit a stale flag into the YAML.
- Missing `application_brief.yaml`: first require a current confirmed apply Decision, then initialize once with
  explicit consent. Concrete legacy language/style values bootstrap only at creation.
- Brief basis is review-required: preserve its bytes, review/reconfirm the current apply Decision, then use one
  `reconfirm_brief` patch against the latest Brief revision/hash.
- Missing or stale `required_document_plan.json`: rerun deterministic Brief; never edit the plan directly.
- Empty Parsed Job document requirements: keep the set `unconfirmed` unless the user explicitly confirms
  `confirmed_empty` against the current requirements-basis hash.
- Plan blockers: resolve an unconfirmed requirement set, unconfirmed Brief field/choice, `required + omit`, required
  document without a preparation action, or orphaned old choice before later Draft/Verify work.
- Missing or stale `cover_letter_draft.json`: make Brief/Match/Evidence current, prepare Draft in host-agent mode,
  submit strict Claim JSON from private scratch, and apply it through the guarded stage boundary.
- Missing or stale `review_findings.json`: make Draft current and run deterministic Review. Resolve blocker findings;
  do not treat open support-review findings as readiness.
- Mutation recovery requested: use `user-mutation recover` with the opaque accepted mutation ID and explicit
  consent; do not replay the private patch as a new write.
- Existing generated outputs after advert/profile changes: rerun the pipeline and review diffs.
- A `typst/*.generated.typ` file after rerun: the editable `.typ` had user changes and was preserved; review and merge
  the candidate intentionally.
