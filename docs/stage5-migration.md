# Stage 5 Runtime Migration, Recovery, And Rollback

Stage 5 moves normal application preparation behind one resumable, validated stage runtime. The upgrade preserves
the Stage 4 discovery catalog, legacy CLI entry points and output names, full-advert URL/PDF/text intake, user-owned
Decision Spine files, edited Typst sources, and private job data.

Migration is explicit and reversible. Reading an older job does not create `workflow/`, rewrite metadata, or treat
legacy generated files as successful stage evidence.

## Before Upgrading

1. Back up the private workspace, including `profile/`, `jobs/`, `job_leads/`, and locally edited defaults.
2. Record any active CanISend or agent task and stop concurrent writers for the job being migrated.
3. Upgrade the CLI and add packaged defaults without overwriting local edits.

```bash
canisend doctor --workspace <private-workspace>
canisend update-workspace --workspace <private-workspace>
```

The default update is additive. It installs current schemas, examples, skills, and this guide while preserving local
prompt, template, schema, skill, advert, profile, job, Markdown, Typst, and PDF bytes. Use `--overwrite` only after
reviewing local default-resource differences; it is not a user-data migration.

## Inspect Without Mutation

Inspect each existing job first:

```bash
canisend migration inspect \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --format json
```

Inspection is read-only. It creates no lock, plan, receipt, state file, or backup. It classifies pre-workflow,
prior-schema, current-unmigrated, applied, rolled-back, and blocked jobs and reports body-free counts and reason
codes. Candidate files and prepared-input bodies are excluded from the migration inventory.

Old jobs remain usable before migration. Do not infer current stages from the presence of legacy Markdown, Typst,
PDF, or JSON output alone.

## Apply Or Resume Migration

Apply only after reviewing the inspection:

```bash
canisend migration apply \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --format json
```

The service writes an immutable migration plan before changing runtime metadata, preserves exact backup bytes before
replacement, and records one plan-bound body-free receipt. Repeating the command after success is a cache hit.
Repeating it after interruption resumes only when each observed file still matches the plan's recorded before or
after hash through compare-and-swap checks; any unrelated change fails closed.

Migration may create or replace only recorded runtime metadata. It never deletes or rewrites profile sources,
adverts, source URLs, Decision Spine YAML, structured Draft/Review bodies, compatibility Markdown, editable Typst,
rendered PDFs, discovery catalogs, or lead imports.

## Conflict-Safe Rollback

Rollback is also explicit:

```bash
canisend migration rollback \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --format json
```

Rollback removes a migration-created file or restores a replaced file only while its current hash still equals the
migration's recorded after hash. Locally changed metadata is preserved and reported as a conflict. Every rollback
attempt has its own immutable receipt; a conflict is never resolved by deleting user/private content.

Reinstalling an older CLI is separate from metadata rollback. Back up first, roll back unchanged migration-owned
metadata if needed, then install the previous known-good version. A previous release may ignore Stage 5 runtime
records, but Stage 4 CSV/JSON/EML/MBOX imports, normalized searches, RSS/Atom feeds, Greenhouse/Lever candidates,
legacy lead selection, and explicit URL/PDF/text advert intake remain available.

## Resume And Retry

`canisend run --dry-run` is a read-only sequence plan. A normal `canisend run` resumes current work in registered
order, reuses valid receipts, and stops at user-owned, host-agent, provider-consent, review, or repair boundaries.
It never treats a process exit code or a legacy filename as promotion evidence.

```bash
canisend run \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --dry-run

canisend run \
  --workspace <private-workspace> \
  --job jobs/<job-slug>
```

For an interrupted prepared task, inspect `stage status`. Resume with the returned submit/apply action when the
candidate and TaskResult are valid, or cancel the exact active stage before preparing replacement work:

```bash
canisend stage cancel \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage <stage> \
  --document-id <document-id-if-required> \
  --format json
```

Do not delete immutable run records to make a stage appear retryable. A rejected candidate, cancellation, stale
input, output conflict, or terminal claim remains audit evidence.

## Explicit Projection Repair

Package and Render promote one strict JSON bundle each. Legacy Markdown/Typst/PDF files are derived projections, not
alternate authoritative stage outputs. Inspect projection repair without mutation:

```bash
canisend repair projection \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --stage package \
  --dry-run \
  --format json
```

Use `--stage render` for PDF projections. If the response reports `repairable`, rerun without `--dry-run`.
Projection repair replays only a current validated bundle. It restores missing/partial derived files and replaces an
invalid projection journal; it preserves an edited primary Typst file and writes a reviewable `*.generated.typ`
candidate instead of overwriting the edit.

Output drift in the authoritative bundle is not repaired automatically. Review or restore the authoritative file,
or produce a new accepted stage result.

## Explicit State Repair

Inspect state reconstruction separately:

```bash
canisend repair state \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --dry-run \
  --format json
```

Rerun without `--dry-run` only when the response reports `repairable`. State repair reconstructs
`workflow/state.json` from recoverable immutable TaskSpec, result, validation, claim, promotion, and manifest
evidence. It never rewrites an authoritative stage output, so tampering or output drift stays visible.

## Concurrency And Registered Orchestration

All cooperative mutations for one job share `workflow/job.lock`. The lock is crash-released by the operating system;
the persistent file is only a coordination inode. One job may have at most one active registered stage task.

For agent orchestration, use the packaged `examples/orchestration/registered-parse.example.yaml`. A task declaring
`registered_stage` must not declare its own `inputs`, `outputs`, `writes`, or profile edits. CanISend prepares the
immutable TaskSpec before worker dispatch, exposes only its allowed reads/core-service writes/required consents,
accepts one JSON candidate from stdout, and runs guarded submit/validate/apply. Exit zero alone cannot promote.

Generic orchestration tasks retain their declared-output behavior and are labelled `generic_declared_output`; they
are not registered stage promotion.

## Troubleshooting

- `stage.concurrent_run`: finish or cancel the active task; do not run two stage writers for one job.
- `stage.already_current`: no work is required. Reuse the current receipt instead of forcing a rerun.
- `stage.stale_input` or `stage.dependency_not_current`: refresh the true upstream source/stage, then inspect again.
- `stage.output_conflict`: the authoritative output changed after preparation or promotion. Review the drift; repair
  commands do not overwrite it.
- `repair.projection_*`: run projection inspection first and preserve edited Typst candidates.
- migration `blocked`: preserve all files and inspect the body-free reason code; do not edit the immutable plan or
  receipt to bypass a hash conflict.
- rollback conflicts: keep the changed metadata, compare it with the migration backup/receipt, and decide manually.
- an unchanged rerun writes work: inspect stage currentness and projection journal drift before deleting anything.

Normal errors and receipts contain paths, hashes, counts, IDs, states, and reason codes rather than private bodies.
Job adverts, profile evidence, Draft/Review content, candidates, projections, and PDFs remain Tier 2 private data.

## Post-Upgrade Checks

```bash
canisend doctor --workspace <private-workspace>
canisend migration inspect --workspace <private-workspace> --job jobs/<job-slug> --format json
canisend run --workspace <private-workspace> --job jobs/<job-slug> --dry-run
canisend stage status --workspace <private-workspace> --job jobs/<job-slug> --format json
```

Keep submission manual. Migration, recovery, Package/Verify/Render success, and rendered PDFs are not portal upload,
submission, or receipt evidence.
