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
canisend stage run --workspace /tmp/canisend-example --job "$JOB" --stage evidence --mode deterministic --format json
canisend stage run --workspace /tmp/canisend-example --job "$JOB" --stage parse --mode deterministic --format json
canisend stage run --workspace /tmp/canisend-example --job "$JOB" --stage confirm --mode deterministic --format json
canisend stage run --workspace /tmp/canisend-example --job "$JOB" --stage match --mode deterministic --format json
```

This adds strict `criteria.json`, private `evidence_catalog.json`, and privacy-safe `criterion_matches.json` plus
immutable run evidence under the job's `workflow/` directory. Match classifications are proposed review results, not
application decisions. The fake profile stays inside the workspace and its generated Typst evidence carries a
source-hash receipt.

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
