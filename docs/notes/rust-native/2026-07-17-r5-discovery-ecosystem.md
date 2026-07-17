# R5 Discovery Ecosystem Evidence

**Date:** 2026-07-17

**Status:** Complete

## Boundary and contracts

R5 adds discovery as a bounded source-adapter layer rather than a crawler. `canisend-contracts` defines source kinds,
adapter capabilities, refresh policy, normalized lead candidates and records, freshness/status, batch imports,
row diagnostics, receipts, and non-destructive similarity suggestions. `canisend.discovery-batch/v2` and
`canisend.discovery-lead/v2` are generated from those Rust types, bringing the public catalog to 20 schemas and the
embedded resource catalog to 26 resources.

Normalized leads require a title, organization, and credential-free HTTP(S) URL. Optional location, ISO calendar
deadline, external ID, and summary fields are bounded. Source extensions accept typed strings, integers, booleans,
and bounded JSON with limits on entry count, key/value size, aggregate bytes, and nesting depth.

## Local and agent imports

CSV imports require explicit `title`, `organization`, and `url` headers. Optional fields are `external_id`,
`location`, `deadline`, and `summary`; `meta.*` headers are the only extension mapping. Duplicate, missing, or unknown
headers fail the batch, while invalid rows produce stable diagnostics without discarding valid rows.

JSON files use the versioned discovery-batch contract. Host-agent results must declare `source_kind: host-agent` and
are otherwise normalized through the same path. Regular non-symlink `.csv` and `.json` files are streamed under a
4 MiB limit. `--dry-run` validates without discovering, opening, or changing a workspace.

## Public network adapters

The compiled adapter registry contains RSS/Atom, jobs.ac.uk, Greenhouse, and Lever. The provider shapes follow the
[Greenhouse Job Board API](https://developers.greenhouse.io/job-board), the official
[Lever Postings API](https://github.com/lever/postings-api), and the jobs.ac.uk
[RSS guidance](https://www.jobs.ac.uk/media/pdf/about/rss_guide.pdf). Only documented public read endpoints are used;
CanISend never calls application-submission endpoints.

All adapters reuse the R4 transport. Requests disable proxies and automatic redirects, resolve and pin public
addresses on every hop, reject credentials/private ranges/HTTPS downgrade, and enforce time, redirect, content
encoding, and body limits. Discovery responses have the tighter 4 MiB limit and must be UTF-8 JSON or XML whose MIME
declaration agrees with content sniffing. Greenhouse and Lever enforce HTTPS plus their documented provider hosts and
paths. CI parses committed RSS, Atom, jobs.ac.uk, Greenhouse, and Lever fixtures and never depends on a live site.

## Identity, refresh, and history

SQLite migration 3 expands discovery sources and leads and adds refresh receipts. A source identity is the adapter
kind, source name, and endpoint. Within that source, an external ID is the preferred exact key; otherwise a
deterministic SHA-256 key is derived from normalized organization, title, location, and URL. Exact refreshes update
the existing record instead of silently multiplying it.

Each committed batch stores observed/inserted/updated/unchanged/removed/rejected counts, start/completion times, and
the source cursor. Snapshot adapters mark missing active leads removed; passed deadlines become expired; stale policy
updates freshness. Removed, expired, and promoted records remain queryable with `--include-history`.

Similarity is advisory only. It examines at most 200 recent active candidates, returns at most 20 results, uses a
deterministic weighted token score, and never merges or deletes a lead. Promotion is explicit and idempotent: it
creates a direct-intake job in one SQLite transaction, marks the lead promoted, writes audit events, and returns the
safe follow-up `job import JOB_ID --url URL` rather than fetching the advert implicitly.

## CLI and verification evidence

Available commands are:

```text
canisend discovery adapters --json
canisend discovery import --file BATCH.csv --source-name NAME --dry-run --json
canisend --workspace WORKSPACE discovery import --file BATCH.csv --source-name NAME --json
canisend --workspace WORKSPACE discovery import --file BATCH.json --json
canisend --workspace WORKSPACE discovery import --file AGENT.json --host-agent --json
canisend --workspace WORKSPACE discovery refresh --adapter ADAPTER --endpoint URL --source-name NAME --json
canisend --workspace WORKSPACE discovery sources --json
canisend --workspace WORKSPACE discovery list --include-history --json
canisend --workspace WORKSPACE discovery show LEAD_ID --json
canisend --workspace WORKSPACE discovery suggest LEAD_ID --limit 5 --json
canisend --workspace WORKSPACE discovery promote LEAD_ID --json
```

Local verification passed formatting, Clippy with warnings denied, 44 Rust tests, 20 generated-schema checks, 26
embedded-resource checks, release compilation, and a packaged-binary dry-run/import/promotion/backup/restore smoke.
GitHub Actions run `29616322777` repeated the clean-checkout gate in 1 minute 55 seconds. `discovery.refresh` is now
truthfully `available`; R6 can build richer agent context and task collaboration on this durable discovery state.
