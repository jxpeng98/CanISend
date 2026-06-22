# Strict HR Review and Job Intake Design

## Goal

Extend CanISend in two focused ways:

- add a strict university HR review lens that checks whether generated materials are visibly aligned with the JD;
- expand job intake so users can start from a source URL, explicitly fetch a job page, or import a JD PDF.

The change must preserve the current local-first workflow, privacy boundaries, and existing `.md` / `.txt` advert
imports.

## Context

CanISend currently creates job folders from manual metadata, local `.md` or `.txt` adverts, or jobs.ac.uk RSS leads.
The generated material review checklist checks placeholders, citations, and manual follow-up actions, but it does not
act like a strict university shortlisting reviewer.

The current `new-job` behavior intentionally avoids scraping full job pages. That remains the default. The new behavior
adds an explicit opt-in flag for fetching URL content and a local PDF import path for users who already have the JD as a
file.

## Non-Goals

This design does not add:

- account creation, portal automation, form filling, uploads, or application submission;
- automatic fetching from URLs unless the user passes an explicit fetch flag;
- login, cookie, JavaScript-rendered page, or browser-session support;
- claims that fetched page text or PDF text is perfect without human review;
- broad web scraping beyond a single user-supplied job URL.

## Recommended Approach

Use a conservative V1.1 expansion:

1. Keep `new-job --source-url` as metadata-only by default.
2. Add `new-job --source-url <url> --fetch-url` to fetch one user-supplied URL and import readable page text.
3. Extend `new-job --advert-file` to accept `.pdf` in addition to `.md` and `.txt`.
4. Add strict university HR review guidance to the material-review skill and generated checklist.

This keeps old commands working while making network use explicit and auditable.

## Job Intake Behavior

### Metadata-Only URL

When the user runs:

```bash
canisend new-job \
  --workspace <workspace> \
  --title "<title>" \
  --institution "<institution>" \
  --deadline "<deadline>" \
  --source-url "<job-url>"
```

CanISend creates the job folder and writes `source_url` to `job.yaml`. `job_advert.md` remains a review stub that says
the URL was saved but the full advert still needs manual paste, PDF import, or explicit fetch before final parsing.

Status remains `new` because no advert has been imported.

### Explicit URL Fetch

When the user adds `--fetch-url`, CanISend fetches exactly the supplied `--source-url`, extracts readable text from the
HTML response, and writes it to `job_advert.md`.

Rules:

- `--fetch-url` requires a non-empty `--source-url`.
- Only `http://` and `https://` URLs are accepted.
- Redirects may be followed by the standard library HTTP client.
- Non-HTML responses fail with a clear error unless they are handled by a future import path.
- Failed network requests leave no partial job folder behind.
- Successful fetch sets `status: advert_imported`.
- `job.yaml.notes` records that the advert was fetched from the source URL and needs human review.

The extractor should be intentionally simple: remove scripts, styles, markup, and repeated whitespace, then preserve
enough text for the existing parser or LLM parser to work. It should not attempt to execute JavaScript or bypass access
controls.

### Local JD PDF Import

When the user passes a PDF:

```bash
canisend new-job \
  --workspace <workspace> \
  --title "<title>" \
  --institution "<institution>" \
  --deadline "<deadline>" \
  --advert-file jd.pdf
```

CanISend extracts text from the local PDF and writes it to `job_advert.md` with a short provenance header.

Rules:

- Existing `.md` and `.txt` imports remain byte-for-byte unchanged.
- `.pdf` import sets `status: advert_imported`.
- Empty or unreadable PDF text fails clearly.
- PDF extraction should use the smallest reasonable dependency if the current runtime does not already provide one.
- Extracted PDF text is treated as an imported advert that still requires human review before relying on criteria.

## Strict University HR Review

The strict review role should be visible in both:

- `skills/canisend-material-review/SKILL.md`;
- generated `07_material_review_checklist.md`.

The role is:

> Review like a strict university HR or shortlisting panel member checking whether the application materials clearly
> answer the advertised essential and desirable criteria.

The review checklist should add a dedicated section that checks:

- every essential criterion appears in the generated criteria checklist;
- each essential criterion has coverage marked strong, partial, weak, or missing;
- weak or missing essential criteria are blockers, not polish items;
- cover letter and CV tailoring notes make JD fit visible without generic claims;
- claims are proportional to evidence and avoid overclaiming;
- item-level evidence citations are present for strong claims when generated evidence exists;
- required documents are listed and unresolved document gaps are visible;
- wording is appropriate for a university HR reviewer who may scan for exact JD language.

The generated checklist should stay deterministic. It can use existing parsed criteria, generated materials, citation
presence, and placeholder checks. It should not pretend to know the true strength of private evidence beyond the data it
can inspect.

## Data Flow

```text
manual metadata, source URL, fetched HTML, or local JD PDF
        |
        v
new-job
        |
        v
jobs/<job-slug>/job.yaml + job_advert.md
        |
        v
canisend run
        |
        v
parsed_job.json + generated materials
        |
        v
strict HR material review checklist
```

## Error Handling

`new-job --fetch-url` should fail before writing a job folder when:

- `--source-url` is missing;
- the URL scheme is not HTTP or HTTPS;
- the response cannot be read;
- the response is not recognizable HTML;
- extracted text is empty.

`new-job --advert-file jd.pdf` should fail before writing a job folder when:

- the PDF cannot be read;
- no text can be extracted;
- the PDF parser dependency is unavailable.

Existing duplicate job-folder behavior remains unchanged: `mkdir(..., exist_ok=False)` prevents accidental overwrite.

## Testing

Use test-first implementation for behavior changes.

Required tests:

- `new-job --source-url` alone creates a metadata-only job stub and keeps `status: new`;
- `new-job --fetch-url` rejects missing source URL;
- `new-job --fetch-url` rejects non-HTTP schemes;
- URL fetching imports readable HTML text and sets `status: advert_imported`;
- failed URL fetching does not leave a partial job folder;
- `.md` and `.txt` advert imports remain unchanged;
- `.pdf` advert import writes extracted text and sets `status: advert_imported`;
- unreadable or empty PDF extraction fails clearly;
- material review checklist includes a strict university HR review section;
- strict review section flags weak or missing essential criteria as blockers;
- `canisend-material-review` skill names the strict HR role and keeps shared privacy and quality-gate references;
- resource distribution and package checks still include updated skills and any new dependency metadata.

Run at least:

```bash
uv run pytest tests/test_jobs.py tests/test_pipeline.py tests/test_skill_distribution.py -q
uv run pytest -q
```

## Documentation Updates

Update README and shared workflow references to state:

- `--source-url` alone records a job link but does not fetch it;
- `--fetch-url` explicitly downloads one user-supplied job page;
- local JD PDFs can be imported through `--advert-file`;
- imported HTML or PDF text must be reviewed before relying on parsed criteria;
- the material review skill now includes a strict university HR alignment review.

## Release Boundaries

The change must preserve:

- default no-scraping behavior;
- explicit approval for network-based URL fetching;
- existing jobs.ac.uk RSS workflow;
- existing `.md` and `.txt` advert import semantics;
- no submission, upload, account, or portal automation;
- no readiness claims without quality-gate review.
