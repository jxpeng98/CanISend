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

From the repository root:

```bash
WORKDIR=/tmp/aap-example
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"
cp -R examples/end_to_end/profile "$WORKDIR/profile"

uv run academic-prep fetch-jobs-ac-uk \
  --rss-file examples/end_to_end/jobs_ac_uk_sample.xml \
  --output "$WORKDIR/job_leads/jobs_ac_uk.json" \
  --include economics

uv run academic-prep new-job-from-lead \
  --leads-file "$WORKDIR/job_leads/jobs_ac_uk.json" \
  --lead-index 0 \
  --institution "Example University" \
  --deadline "2026-06-15" \
  --jobs-dir "$WORKDIR/jobs"

JOB="$WORKDIR/jobs/2026-06-15_example-university_lecturer-in-applied-economics"
cp examples/end_to_end/full_job_advert.md "$JOB/job_advert.md"

uv run academic-prep extract-profile-evidence --profile-dir "$WORKDIR/profile"

ACADEMIC_PREP_LLM_PROVIDER=command \
ACADEMIC_PREP_LLM_COMMAND="python examples/end_to_end/fake_llm_provider.py" \
uv run academic-prep run \
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

The fake provider is deterministic. It exists only so Codex, Claude Code, Gemini, and tests can exercise the file contracts without API keys.
