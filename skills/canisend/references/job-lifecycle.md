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

Regenerate evidence, then run the pipeline:

```bash
canisend extract-profile-evidence --workspace <private-workspace>
canisend run --workspace <private-workspace> --job jobs/<job-slug>
```

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
- Missing `parsed_job.json`: run `canisend run`.
- Existing generated outputs after advert/profile changes: rerun the pipeline and review diffs.
- A `typst/*.generated.typ` file after rerun: the editable `.typ` had user changes and was preserved; review and merge
  the candidate intentionally.
