# Application Package Skills and Job Feed Design

## Goal

Improve CanISend's application-package workflow while keeping the current jobs.ac.uk RSS workflow working.

The primary improvement is a more complete skill set for constructing, checking, and reviewing application
packages. The secondary improvement is a generic RSS/Atom lead-ingestion layer that keeps jobs.ac.uk as the
first-class source but makes future job sources easier to add.

## Context

CanISend currently has a local-first CLI workflow that can:

- initialize a private workspace;
- fetch and filter jobs.ac.uk RSS leads;
- create a local job folder from a selected lead;
- parse a full advert after the user manually adds it;
- generate fit reports, cover letters, CV notes, criteria checklists, material review checklists, and Typst
  package sources;
- export a reusable skill pack with focused skills for cover letters, CV tailoring, criteria checks, research
  statements, and material review.

The current gaps are:

- application-package construction is covered by the broad `canisend` workflow rather than a focused package
  skill;
- job intake is still described as jobs.ac.uk-specific even though the lead model is already generic enough for
  RSS/Atom feeds;
- submission-readiness review is implied by quality gates, but there is no dedicated skill for final package
  completeness and risk review.

## Non-Goals

This design does not add:

- full job-page scraping;
- account creation, portal automation, form filling, uploads, or application submission;
- private CV, statement, or advert reading without the existing explicit-approval boundaries;
- broad external-channel integrations that depend on unstable page HTML;
- new runtime dependencies unless implementation proves that the standard library is insufficient.

## Recommended Approach

Use a conservative V1.1 expansion:

1. Add focused application-package skills.
2. Add a generic feed parser and CLI while preserving `fetch-jobs-ac-uk`.
3. Update docs, package checks, and tests so the new skill pack and feed behavior are part of the release
   contract.

This keeps the product useful immediately for jobs.ac.uk users and creates a stable interface for later channels.

## Architecture

### Skill Pack

Add three skill folders under `skills/`:

- `canisend-application-package`: use when building or reviewing the full application package for one job.
- `canisend-job-intake`: use when turning jobs.ac.uk leads and manually supplied advert text into a complete job
  folder.
- `canisend-submission-readiness`: use when checking whether generated materials are complete enough for the
  user to manually submit.

Each skill has:

- `SKILL.md` with standard frontmatter;
- `agents/openai.yaml` with display metadata and default prompts;
- references to shared `../canisend/references/privacy.md` and `../canisend/references/quality-gates.md`;
- explicit boundaries that forbid submission, portal automation, fabricated evidence, and unapproved private
  reading.

Update `skills/canisend/SKILL.md` and `agent-skills/canisend/SKILL.md` so the main workflow routes focused package,
job-intake, and submission-readiness tasks to these skills when native skills are available.

### Feed Ingestion

Keep `JobLead` as the normalized lead type:

- `title`
- `source_url`
- `description`
- `published_at`
- `source`
- `source_feed`

Add a generic parser in `src/canisend/rss.py`:

- `parse_job_feed(xml_text: str, *, feed_url: str = "", source_name: str = "unknown") -> list[JobLead]`
- RSS support reads `./channel/item` with `title`, `link`, `description`, and `pubDate`.
- Atom support reads entries with `title`, `link`, `summary` or `content`, and `updated` or `published`.
- HTML descriptions are cleaned with the existing description-cleaning helper.

Keep `parse_jobs_ac_uk_rss()` as a compatibility wrapper:

- calls `parse_job_feed(..., source_name="jobs.ac.uk")`;
- preserves current test expectations and output fields.

### CLI

Keep the existing command:

```bash
canisend fetch-jobs-ac-uk --workspace <workspace> --feed-url "<jobs.ac.uk RSS URL>"
```

Add a generic command:

```bash
canisend fetch-job-feed \
  --workspace <workspace> \
  --source-name "<source label>" \
  --feed-url "<RSS or Atom URL>" \
  --output <optional output JSON> \
  --include <keyword> \
  --exclude <keyword>
```

Default output behavior:

- `fetch-jobs-ac-uk` keeps writing `job_leads/jobs_ac_uk.json`;
- `fetch-job-feed` writes `job_leads/<slugified-source-name>.json` when `--output` is not supplied.

Both commands use the same filter and writer functions.

### Job Folder Creation

Keep `new-job-from-lead` unchanged as the normalized entrypoint from lead JSON to job folder.

The lead-created advert remains a discovery stub:

- source metadata;
- RSS/Atom description;
- explicit placeholder asking the user to paste the full advert before relying on generated criteria or drafts.

This preserves the privacy and anti-scraping boundary.

## Data Flow

```text
jobs.ac.uk RSS or generic RSS/Atom feed
        |
        v
parse_job_feed / parse_jobs_ac_uk_rss
        |
        v
job_leads/*.json
        |
        v
new-job-from-lead
        |
        v
jobs/<job-slug>/
        |
        v
manual full advert paste or local advert import
        |
        v
canisend run
        |
        v
application package outputs and focused skills review
```

## Error Handling

Feed parsing should fail clearly when XML is invalid.

The generic CLI should reject calls that provide neither `--feed-url` nor `--rss-file`.

`--source-name` must be non-empty for the generic command. The implementation should use a deterministic slug for
the default output filename and reject source names that slugify to an empty value.

`new-job-from-lead` keeps its current validations for negative index, missing lead object, missing title, and missing
institution.

## Testing

Use test-first implementation for behavior changes.

Required tests:

- generic RSS parsing maps source name and feed URL into `JobLead`;
- Atom parsing handles title, link, summary or content, and updated or published dates;
- `fetch-job-feed` reads a local XML file, applies include/exclude filters, and writes a source-specific JSON file;
- `fetch-jobs-ac-uk` retains current behavior and output text;
- new skills have standard manifests and agent metadata;
- skill export includes the new skills;
- package resource checks include the new skill resources;
- main skill references the package, job-intake, and submission-readiness skills without losing existing privacy
  gates.

Run at least:

```bash
uv run pytest tests/test_rss.py tests/test_cli.py tests/test_skill_distribution.py tests/test_repository_contract.py -q
uv run pytest -q
```

## Documentation Updates

Update README and shared references to state:

- jobs.ac.uk remains the documented default workflow;
- `fetch-job-feed` is available for RSS/Atom sources;
- lead imports are discovery records, not full adverts;
- package-focused skills should be used for application-package construction and review;
- CanISend still prepares materials only and never submits applications.

## Release Boundaries

The change must preserve:

- local-first behavior;
- private path git-safety rules;
- current jobs.ac.uk command compatibility;
- no scraping of full job pages;
- no claim that materials are ready, final, complete, or submission-ready without quality-gate review.

## Open Decisions

The first implementation should not hard-code extra public job boards beyond jobs.ac.uk. Additional sources should be
added later only when they expose stable RSS, Atom, or documented APIs.
