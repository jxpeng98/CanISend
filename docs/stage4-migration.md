# Stage 4 Discovery Migration And Rollback

This guide upgrades an existing private CanISend workspace to the Stage 4 discovery contracts. The migration is
additive: it does not move or rewrite `profile/`, `jobs/`, existing lead files, or application artifacts.

## Before Upgrading

1. Back up the private workspace, especially `profile/`, `jobs/`, `job_leads/`, and locally edited defaults.
2. Finish or record any active application work. Discovery outputs are candidates, not submission state.
3. Upgrade the installed CLI, then inspect the workspace before replacing any local defaults.

```bash
canisend doctor --workspace <private-workspace>
canisend update-workspace --workspace <private-workspace>
```

The default update preserves local prompts, templates, schemas, and skills. Use `--overwrite` only after reviewing
local changes that should be replaced by packaged Stage 4 defaults. The update adds discovery schemas, examples,
this guide, and current agent instructions; it does not delete private data.

## Existing Lists And Index Selection

Existing jobs.ac.uk and generic RSS/Atom JSON lists remain readable. Existing automation may continue using the
zero-based selector:

```bash
canisend new-job-from-lead \
  --workspace <private-workspace> \
  --leads-file job_leads/existing-list.json \
  --lead-index 0 \
  --institution "<institution>"
```

Stage 4 derives Lead v2 identity for a legacy record when possible. New workflows should merge lists into the
catalog and select by stable ID, because an ID survives source reordering:

```bash
canisend discovery merge \
  --workspace <private-workspace> \
  --input job_leads/existing-list.json

canisend new-job-from-lead \
  --workspace <private-workspace> \
  --leads-file job_leads/catalog.json \
  --lead-id <lead_id> \
  --institution "<institution>"
```

Do not provide `--lead-index` and `--lead-id` together. Old list files are not rewritten by merge.

## New Discovery Storage

Stage 4 keeps discovery data inside ignored `job_leads/`:

```text
job_leads/
  catalog.json                    # current deterministic union/ranking view
  batches/                        # complete source snapshots
  cache/                          # validators and private-safe transport metadata
  imports/                        # local CSV/JSON/EML/MBOX batches and reports
  searches/                       # normalized host-agent search batches
  refresh-report.json             # latest private-safe refresh report
```

The catalog is the normal selection surface. Complete batches, imports, and normalized search batches are durable
inputs from which it can be rebuilt. Cache validators and refresh reports are operational metadata; they are not job
applications and contain no retained response bodies.

CSV, JSON, EML, MBOX, host-agent search, RSS/Atom, Greenhouse, and Lever inputs all stop at candidate discovery.
Selecting a lead creates one job folder, but the user must still provide the complete advert before parsing or
drafting. Existing intake paths remain unchanged:

```bash
canisend new-job --advert-file <advert.md|advert.txt|advert.pdf> ...
canisend new-job --source-url <one-user-supplied-html-or-pdf-url> --fetch-url ...
```

## Source Configuration And Cache Recovery

Start from `examples/discovery/discovery-sources.example.yaml`. Replace placeholders and keep the configuration free
of credentials. Greenhouse accepts a public `board_token`; Lever accepts a public `site_id` and `global` or `eu`
region. Neither adapter accepts arbitrary API URLs, headers, auth, apply endpoints, or upload behavior.

If a cache entry is invalid, stale, or from an older prerelease, preserve the source configuration and rerun
`discovery refresh`. A successful complete batch can be reused after a later source failure. If cache recovery is
not possible, remove only the affected rebuildable cache entry after backing it up; do not delete complete batches,
imports, searches, `catalog.json`, jobs, or profile data as a first response.

## Rollback

To roll back the CLI, reinstall the previous known-good version and keep the private workspace backup. The previous
CLI can continue using legacy JSON lists, `--lead-index`, direct manual job creation, explicit single-URL HTML/PDF
intake, and local PDF/text intake. It may ignore newer catalog metadata and Stage 4 directories.

If a previous version cannot read `catalog.json`, use an untouched legacy list or export the catalog's `leads` array
to a separate JSON list with a reviewed local tool. Never overwrite the catalog or delete Stage 4 batches during
rollback. Discovery caches are rebuildable; selected job folders and application artifacts are not.

Rollback does not reverse a user decision, submit an application, or remove private data. After returning to Stage
4, run `update-workspace` without `--overwrite`, validate the source configuration, and rebuild the catalog.
