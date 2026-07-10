# Discovery And Application Workflow V2 Design

> This remains the detailed discovery, gate, and rerun-safety design. The agent-native delivery order and current
> phase sequencing are defined by `2026-07-10-agent-native-workflow-roadmap.md`.

## Goal

Expand academic and professional job discovery without introducing brittle page crawling, and make every application
stage observable, reviewable, and safe to rerun.

CanISend remains an application-preparation tool. It does not create accounts, submit applications, upload files, or
answer sensitive declarations.

## Product Decisions

1. Keep the application workflow in its own `application` namespace. Academic-research artifacts may be imported as
   evidence, but research lifecycle stage IDs must not be reused for job-application stages.
2. Keep the deterministic pipeline as the core execution path. Agent orchestration is a review and specialist-work
   layer until it can verify every declared output and enforce write boundaries.
3. Prefer stable feeds and documented read-only APIs. Arbitrary search-result scraping and background crawling remain
   out of scope.
4. Preserve the existing jobs.ac.uk command, lead JSON list, and job status values while adding versioned fields and
   derived workflow state incrementally.
5. Treat `packaged` as generation completed, never as proof that the application is ready to submit.

## Discovery Architecture

### First-Class Intake Channels

Source expansion does not replace direct user input. CanISend keeps three peer entry paths:

1. discovery through RSS, Atom, or documented read-only APIs;
2. one user-supplied HTTP(S) job URL returning HTML or PDF;
3. one user-supplied local `.pdf`, `.md`, or `.txt` advert.

All three normalize into `job.yaml` plus a reviewable `job_advert.md`, then use the same parse, match, draft, package,
and readiness stages. Direct URL intake remains bounded and explicit; it does not authorize crawling adjacent pages.

### Source Tiers

| Tier | Source type | Product treatment |
|---|---|---|
| 1 | RSS and Atom | Built-in generic feed ingestion; jobs.ac.uk remains the compatibility preset. |
| 2 | Documented public job-board APIs | Add explicit adapters, beginning with read-only Greenhouse and Lever endpoints. |
| 3 | Official portals without a stable public API | Accept user-exported JSON, CSV, email alerts, or saved links; do not emulate private APIs. |
| 4 | Arbitrary search-result pages | Not a discovery source. A user may still explicitly import one supplied HTML or PDF advert URL under bounded-fetch rules. |

Greenhouse documents unauthenticated GET access to published job-board data:
<https://developers.greenhouse.io/job-board>. Lever documents a public postings API for published jobs:
<https://github.com/lever/postings-api>. Those interfaces are suitable for source-specific adapters; their application
submission endpoints are not.

### Compatibility Layer

The first implementation keeps:

- `JobLead` and its six existing fields;
- `parse_jobs_ac_uk_rss()` as a wrapper;
- `fetch-jobs-ac-uk` and its default `job_leads/jobs_ac_uk.json` output;
- `new-job-from-lead --lead-index` for existing workspaces.

The generic entrypoint is:

```bash
canisend fetch-job-feed \
  --workspace <workspace> \
  --source-name "<source label>" \
  --feed-url "<rss-or-atom-url>"
```

### Target Discovery Model

The next contract revision should add optional fields without removing the existing six:

- `schema_version`
- `lead_id`
- `source_record_id`
- `canonical_url`
- `institution`
- `location`
- `deadline`
- `fetched_at`
- `match_reasons`
- `provenance`

Stable identity should use source-native ID first, canonical URL second, and a normalized
title/institution/deadline fingerprint last. `--lead-id` should be added before lead files begin merging or refreshing
in place.

### Discovery Service Boundary

A future `discovery` package should separate:

```text
source adapter -> bounded transport -> normalization -> filter -> deduplicate -> rank -> atomic batch write
```

The transport owns URL validation, redirect revalidation, response limits, content type, conditional requests,
retry/backoff, and redaction. Source adapters never persist credentials; configuration stores only environment-variable
names.

One failing source must not discard successful results from other sources. Batch output should preserve source errors
and provenance alongside the usable leads.

## Application Workflow

### Derived Stages

The existing CLI remains compatible while the workflow gains an explicit derived stage model:

| Stage | Required input | Primary output | Exit gate |
|---|---|---|---|
| Discovery | source query or local feed | normalized leads | source and provenance recorded |
| Intake | selected lead or manual advert | `job.yaml`, complete `job_advert.md` | advert is not a lead-only stub |
| Evidence | profile sources | `profile/generated/*.evidence.md` | evidence exists and is current |
| Parse | complete advert | validated `parsed_job.json` | required structure and source text valid |
| Match | parsed criteria and evidence | structured criterion matches | every essential criterion classified |
| Draft | validated matches and preferences | application drafts | citations resolve; gaps remain explicit |
| Package | reviewed drafts | Markdown and Typst package | generated files exist; manual edits preserved |
| Verify | current package | gate report | blockers, warnings, and manual decisions recorded |
| Render | verified Typst sources | optional PDFs | rendering succeeds for the current source hashes |
| Submit | user-controlled portal work | outside CanISend | always manual |

The legacy `status` field remains readable. New workflow state should be additive under a namespaced mapping such as:

```yaml
workflow:
  contract_version: 2
  phase: generated
  readiness: review_required
  last_run_id: 20260709T120000Z
```

### Executable Application Gates

Application gates use `APP-Q*` identifiers rather than research-workflow gate IDs:

- `APP-Q1`: advert integrity and provenance
- `APP-Q2`: evidence freshness and claim traceability
- `APP-Q3`: parsed and generated artifact completeness
- `APP-Q4`: human review, unresolved blockers, and presentation-source integrity

A gate report currently records `PASS` or `FAIL`, evidence paths, timestamps, and the hashes of inputs it applies to.
A pipeline rerun marks an existing report `STALE`. `WARN` and `BLOCKED` rollups remain part of the later stage-runner
contract.

### Rerun Safety

Generated Markdown may be regenerated, but job-specific Typst files are documented as editable sources of truth.
Reruns therefore need a generated-baseline hash:

- unchanged Typst source: update safely;
- user-edited Typst source: preserve it and write a `*.generated.typ` candidate;
- adopted candidate: recognize its recorded hash on the next run;
- never require a destructive reset to recover an edited source.

The full pipeline should later write to a staging run directory, validate outputs, and atomically promote them. Its run
manifest records input and output hashes, package version, flags, prompt/template versions, provider/model identifiers,
consent mode, warnings, and blockers without storing secrets.

## Skill Routing

The distributed skill pack should distinguish three workflow seams:

- `canisend-job-intake`: source selection, lead import, and full-advert readiness;
- `canisend-application-package`: coordinated construction of the complete application dossier;
- `canisend-submission-readiness`: strict final gate review before the user submits manually.

Material-specific skills remain responsible for fit, criteria, cover letters, CV tailoring, research statements,
teaching statements, email, interviews, humanization, and material review.

## Delivery Plan

### Slice 1: Compatibility Foundation

- Generic RSS/Atom parsing and `fetch-job-feed`.
- Strict package checks for real Typst sources, parsed-job structure, advert stubs, and review blockers.
- Machine-readable application gate report on explicit request.
- Typst rerun protection through generated hashes and candidate files.
- Focused intake, package, and submission-readiness skills.

### Slice 2: Trustworthy Multi-Source Discovery

- Shared bounded HTTP transport.
- Stable lead IDs and `--lead-id` selection.
- Lead schema version, deterministic deduplication, provenance merge, and explainable ranking.
- Atomic writes, ETag/Last-Modified cache, per-host throttling, and partial-failure reports.
- Local CSV/JSON/email-alert import before any additional network adapter.

### Slice 3: Documented API Adapters

- Greenhouse job-board adapter.
- Lever postings adapter.
- Adapter conformance tests shared by every source.
- Explicit organization/board configuration; no global crawling or application POST calls.

### Slice 4: Deterministic Stage Runner

- Stage contracts wrapping the existing parser, matcher, material generator, Typst renderer, and verifier.
- Run manifest, hashes, staging, atomic promotion, stale-state detection, resume, and selective rerun.
- Structured criterion-match data shared by all drafts and reviewers.

### Slice 5: Review Orchestration And Research Bridge

- Verify all declared orchestrator outputs and write scopes before marking tasks successful.
- Add retry/resume and per-attempt artifacts.
- Import selected research artifacts as read-only, provenance-preserving profile evidence without renaming or copying
  their original lifecycle stages.

## Acceptance Criteria

- Existing jobs.ac.uk workflows and lead files remain usable.
- A generic RSS or Atom source can be ingested without source mislabelling.
- Negative limits and invalid source labels fail clearly.
- A lead-only advert, incomplete parsed job, missing Typst source, or explicit review blocker cannot pass the strict
  package check.
- A repeated pipeline run never silently overwrites user-edited Typst.
- The tool never stores source credentials in config, output, logs, or provenance.
- No new source adapter performs account actions or application submission.
- All behavior changes have offline tests; CI does not depend on a live job board.
