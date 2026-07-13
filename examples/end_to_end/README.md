# End-to-End Local Workflow Example

This fixture demonstrates the intended local workflow without private data, network access, scraping, or real LLM calls.

It contains:

```text
examples/end_to_end/
  jobs_ac_uk_sample.xml
  full_job_advert.md
  fake_llm_provider.py
  profile/
    profile.yaml
    typst/
      cv.typ
      cover_letter_base.typ
      research_statement.typ
      teaching_statement.typ
    generated/
      .gitkeep
```

## Run Manually

Installed users can run the whole packaged example with:

```bash
canisend run-example --workspace /tmp/canisend-example --overwrite
```

The same fake workspace can exercise the accepted resumable Decision Spine without a provider or platform API:

```bash
JOB=jobs/2026-06-15_example-university_lecturer-in-applied-economics
canisend agent context --workspace /tmp/canisend-example --job "$JOB" --format json
canisend stage status --workspace /tmp/canisend-example --job "$JOB" --format json
canisend stage run --workspace /tmp/canisend-example --job "$JOB" --stage evidence --mode deterministic --format json
canisend stage run --workspace /tmp/canisend-example --job "$JOB" --stage parse --mode deterministic --format json
canisend stage run --workspace /tmp/canisend-example --job "$JOB" --stage confirm --mode deterministic --format json
canisend stage run --workspace /tmp/canisend-example --job "$JOB" --stage match --mode deterministic --format json
canisend decision status --workspace /tmp/canisend-example --job "$JOB" --format json
canisend run --workspace /tmp/canisend-example --job "$JOB" --no-git-add-materials
```

This adds strict Tier 2 `criteria.json`, private `evidence_catalog.json`, and body-minimized Tier 2
`criterion_matches.json` plus immutable run evidence under the job's `workflow/` directory. Match classifications
are proposed review results, not application decisions. The fake profile stays inside the workspace and its
generated Typst evidence carries a source-hash receipt.

After recording a current confirmed `decision=apply` through `decision init|update`, the same fake job can exercise
Brief/document planning without a provider or platform API:

```bash
canisend brief status --workspace /tmp/canisend-example --job "$JOB" --format json
canisend brief init --workspace /tmp/canisend-example --job "$JOB" --confirm-user-owned-write --format json
canisend stage run --workspace /tmp/canisend-example --job "$JOB" --stage brief --mode deterministic --format json
```

Use `brief update` with one strict patch and the latest revision/hash to confirm fields, the current document
requirements basis, and document choices. `application_brief.yaml` and `required_document_plan.json` are Tier 2;
Agent status remains body-free. An empty parsed list is not `confirmed_empty`, and unresolved, `required + omit`, or
orphaned choices remain blockers. Brief/document planning alone does not claim Draft or package readiness.

The release smoke automates the complete fake-data slice through confirmed Brief, guarded host-agent Draft,
deterministic Review, compatibility projection, and fail-closed package checking:

```bash
python scripts/smoke_decision_spine.py \
  --canisend canisend \
  --workspace /tmp/canisend-stage3-smoke
```

It constructs one schema-valid fake Claim candidate in private scratch only. For normal work, the active host agent
creates that candidate after the user approves `read-private-draft-inputs`; the agent then uses `stage submit` and
`stage apply`, never a direct authoritative write.

The three YAML files remain manual user-owned Tier 2 ask-first inputs: `confirmed_corrections.yaml`,
`application_decision.yaml`, and `application_brief.yaml`. Users may edit them directly against their schemas.
Agents instead use body-free status, one bounded private patch, the latest revision/hash CAS baseline, and explicit
consent; they never replace a whole user YAML file or race a manual editor save.

The deterministic `run` after current Match uses the workspace-configured profile to project the same proposed graph
into `02_fit_report.md`, `05_criteria_checklist.md`, structured checks in `07_material_review_checklist.md`, and the
`typst/application_package_content.json` and `typst/application_package.typ` projections. If structured state is
stale, drifted/tampered, graph-invalid, or parsed against a different advert view—or if `--profile-dir` overrides the
configured profile—the command safely uses legacy deterministic generation. `--llm-drafts` keeps provider-generated
drafts. A current validated structured Draft plus blocker-free deterministic Review also supplies
`03_cover_letter_draft.md`, Cover Letter/package content JSON, and both Typst views, with every Claim rendered once.
Missing/blocked/stale/tampered Draft or Review uses the same safe fallback. In every path, Match classifications,
Draft, Review, and compatibility projections remain proposals, not application decisions or readiness results.

The manual sequence below is useful when developing the project or debugging individual steps.

From the repository root:

```bash
WORKDIR=/tmp/canisend-example
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"
cp -R examples/end_to_end/profile "$WORKDIR/profile"

uv run canisend fetch-jobs-ac-uk \
  --rss-file examples/end_to_end/jobs_ac_uk_sample.xml \
  --output "$WORKDIR/job_leads/jobs_ac_uk.json" \
  --include economics

uv run canisend new-job-from-lead \
  --leads-file "$WORKDIR/job_leads/jobs_ac_uk.json" \
  --lead-index 0 \
  --institution "Example University" \
  --deadline "2026-06-15" \
  --jobs-dir "$WORKDIR/jobs"

JOB="$WORKDIR/jobs/2026-06-15_example-university_lecturer-in-applied-economics"
cp examples/end_to_end/full_job_advert.md "$JOB/job_advert.md"

uv run canisend extract-profile-evidence --profile-dir "$WORKDIR/profile"

ACADEMIC_PREP_LLM_PROVIDER=command \
ACADEMIC_PREP_LLM_COMMAND="python examples/end_to_end/fake_llm_provider.py" \
uv run canisend run \
  --job "$JOB" \
  --profile-dir "$WORKDIR/profile" \
  --llm-parser \
  --llm-drafts
```

Expected outputs include:

```text
$JOB/parsed_job.json
$JOB/02_fit_report.md
$JOB/03_cover_letter_draft.md
$JOB/05_criteria_checklist.md
$JOB/typst/cover_letter_content.json
$JOB/typst/cover_letter.typ
$JOB/typst/application_package_content.json
$JOB/typst/application_package.typ
```

The fake provider is deterministic. It exists only so Codex, Claude Code, IDE agents, and tests can exercise the file contracts without API keys.
