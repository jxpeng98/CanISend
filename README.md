# Academic Application Preparation Copilot

Local-first CLI tooling for preparing academic job application materials from a job advert and a Markdown-based academic profile.

See `academic_application_prep_copilot_proposal.md` for the V1 engineering proposal.

## V1 Workflow

Initialize local profile files:

```bash
academic-prep init-profile
```

Fetch jobs.ac.uk RSS leads and apply local keyword filters:

```bash
# Copy a raw RSS Feed link from https://www.jobs.ac.uk/feeds/subject-areas,
# https://www.jobs.ac.uk/feeds/locations, or https://www.jobs.ac.uk/feeds/type-roles.
academic-prep fetch-jobs-ac-uk \
  --feed-url "https://www.jobs.ac.uk/path/to/raw/rss/feed" \
  --include economics \
  --include finance \
  --exclude phd
```

Create a job folder from a selected advert:

```bash
academic-prep new-job \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --deadline "2026-06-15"
```

Generate application preparation outputs:

```bash
academic-prep run --job jobs/2026-06-15_university-x_lecturer-in-economics
```

Optionally render Typst files:

```bash
academic-prep render-typst --job jobs/2026-06-15_university-x_lecturer-in-economics
```

## Typst Templates

The project uses the public Typst Universe templates:

- `@preview/modernpro-cv:1.3.0`
- `@preview/modernpro-coverletter:0.0.8`

Generated job-specific Typst files are written under each ignored `jobs/<job-slug>/typst/` folder.

## Privacy Defaults

This repository is intended to be open source. Personal application data should stay local:

- `profile/ is ignored by git` except for `.gitkeep`.
- `jobs/` generated job folders are ignored by git.
- `job_leads/` RSS outputs are ignored by git.
- API keys belong in local environment variables or `.env`, which is ignored by git.
