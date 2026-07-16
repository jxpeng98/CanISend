# Stage 4 Discovery Ecosystem Implementation Plan

**Status:** In progress — Tasks 0–7 accepted; Task 8 local gates passed and remote exit acceptance is next

**Date:** 2026-07-15

**Branch:** `feat/discovery-ecosystem`

**Baseline:** Stage 3 `0.3.0b1`, post-release evidence commit `cd4f2124669076133faa11469470aa10c5e9a70c`

**Candidate milestone:** `0.6.0b1` after Stage 4 exit acceptance; no publication is implied by this plan

## Goal

Expand CanISend discovery from compatible RSS/Atom lists into a deterministic, source-neutral, agent-consumable
ecosystem without changing the Python/local-workspace platform, duplicating application workflow logic, crawling
arbitrary pages, or weakening direct URL and PDF intake.

## Fixed Decisions

- Accept ADR-023 as the identity, provenance, transport, privacy, and read-only adapter boundary.
- Keep `JobLead`'s six original fields, both existing feed commands, list-shaped lead files, and legacy index
  selection readable.
- Add stable `--lead-id` selection before merged catalogs refresh in place.
- Keep discovery results local under `job_leads/`; treat all imported descriptions/snippets as untrusted.
- Use a strict core model and normalized host envelope. Vendor-specific fields end inside adapters.
- Use public documented GET interfaces only. No account, portal, application POST, upload, private API emulation,
  adjacent-page crawl, or scheduled background service belongs to Stage 4.
- Preserve the existing explicit one-URL HTML/PDF fetch and local `.pdf`, `.md`, or `.txt` advert paths as peers.
- Treat CSV, JSON, EML, and MBOX as discovery-candidate imports only. They do not import application materials,
  create a full advert, or authorize application actions.
- Require offline fixture tests; CI and release acceptance must not depend on live job boards.
- Version schemas and public file contracts independently from `canisend.agent/v1`; additions to agent capabilities
  remain additive.

## Public File And CLI Shape

Compatibility commands continue to write a top-level JSON list of v2 entries:

```bash
canisend fetch-jobs-ac-uk --feed-url ...
canisend fetch-job-feed --source-name ... --feed-url ...
canisend new-job-from-lead --leads-file ... --lead-id lead_...
canisend new-job-from-lead --leads-file ... --lead-index 0  # compatibility
```

The Stage 4 discovery namespace will add:

```bash
canisend discovery import --input leads.csv --source-name ...
canisend discovery import-search --input normalized-search.json
canisend discovery merge --input source-a.json --input source-b.json
canisend discovery refresh --sources discovery-sources.yaml
```

`discovery refresh` writes one versioned `job_leads/catalog.json`, complete per-source batches, validator-only cache
records, and a body-free partial-failure report. `new-job-from-lead` accepts either a legacy list or the catalog's
`leads` member.

## Delivery Sequence

### Task 0: Contract And Boundary Freeze

- [x] Audit RSS/Atom, `new-job-from-lead`, URL/PDF intake, workspace resources, and agent capabilities.
- [x] Accept ADR-023.
- [x] Freeze the Stage 4 task graph, privacy boundary, compatibility promises, and exit matrix.

### Task 1: Lead v2 And Stable Selection

- [x] Add strict Pydantic `JobLeadV2`, provenance, and match-reason models.
- [x] Implement redacted URL canonicalization and source-ID/URL/fingerprint identity precedence.
- [x] Upgrade feed output to additive v2 entries and extract RSS GUID/Atom ID where available.
- [x] Write lead lists atomically and package the generated Lead v2 schema.
- [x] Add `--lead-id`, alias lookup, catalog/list loading, and exact-one-selector validation.
- [x] Prove old list files and index workflows remain usable.

### Task 2: Deterministic Catalog, Dedupe, And Ranking

- [x] Add union-based deduplication across namespaced source IDs, canonical URLs, and fallback fingerprints.
- [x] Merge provenance and aliases deterministically without losing the six compatibility fields.
- [x] Add deterministic include/exclude evaluation, source preference, stable tie-breaking, and structured reasons.
- [x] Add inspectable exclusion records and repeat-refresh/no-duplicate fixtures.
- [x] Add `discovery merge` text and `canisend.agent/v1` JSON surfaces.

### Task 3: Atomic Multi-Source Refresh And Shared Transport

- [x] Add a strict source-config and Lead Batch/report contract.
- [x] Centralize DNS-aware public-address checks, redirect checks, response limits, and media-type policy.
- [x] Add ETag/Last-Modified conditional requests and reuse of the last validated complete batch on `304`.
- [x] Add bounded retry/backoff, `Retry-After`, injectable clocks/sleep, and per-host throttling.
- [x] Atomically promote per-source batches and the merged catalog only after validation.
- [x] Preserve stale prior batches and report one source failure without discarding successful sources.

### Task 4: Local Export Ingestion

- [x] Add strict CSV field aliases and row-level validation/reporting.
- [x] Accept JSON lists and versioned CanISend lead batches without silently accepting vendor-specific envelopes.
- [x] Add `.eml` and `.mbox` alert extraction without persisting raw headers, bodies, local absolute paths, or
  unrelated links.
- [x] Route every record through the same normalization, dedupe, ranking, and atomic writer.

### Task 5: Host-Agent Search Import

- [x] Add a strict `canisend.discovery-search/v1` envelope and schema.
- [x] Normalize connector/search results into Lead v2 without storing provider credentials or opaque sessions.
- [x] Expose additive discovery operations/capabilities and body-free result counts/paths through
  `canisend.agent/v1`.
- [x] Add Codex/Claude/generic-host fixtures proving identical imported catalogs.

### Task 6: Adapter Conformance, Greenhouse, And Lever

- [x] Freeze adapter protocol and a shared offline conformance corpus before registering network adapters.
- [x] Implement Greenhouse published-job GET mapping from an explicit board token.
- [x] Implement Lever published-posting GET mapping from an explicit site identifier.
- [x] Reject malformed identifiers, undocumented response roots, auth/credential options, non-GET behavior, and
  application/submission endpoints.
- [x] Document the exact official read-only endpoints and date of verification.

### Task 7: Compatibility, Skills, And Migration

- [x] Update README, changelog, file contracts, privacy guidance, job-intake skill, canonical mirror, and examples.
- [x] Add Stage 4 migration guidance for legacy lists, indexes, catalog selection, caches, and rollback.
- [x] Prove explicit URL HTML/PDF and local PDF/text imports still enter the same job/application workflow.
- [x] Add all public schemas and fixtures to workspace and wheel resource checks.

### Task 8: Stage 4 Exit Acceptance

- [x] Run focused adversarial identity, privacy, transport, batch, import, adapter, and compatibility tests.
- [x] Run the full supported Python 3.11-3.13 matrix plus the available development interpreter.
- [x] Run source-tree, built-wheel, clean-install, schema/resource, Twine, mirror, and CLI/agent smoke gates.
- [ ] Require remote Linux/macOS/Windows CLI smoke before marking the stage complete.
- [ ] Record immutable evidence and decide separately whether to publish the `0.6.0b1` candidate.

## Exit Criteria

- refreshing the same or reordered multi-source inputs creates no duplicate stable leads;
- a failed source is reported and cannot corrupt or erase successful complete batches;
- every retained score, match reason, merge alias, and exclusion reason is inspectable;
- legacy list/index, jobs.ac.uk, generic RSS/Atom, explicit URL HTML/PDF, and local PDF/text intake pass unchanged;
- every adapter is source-neutral beyond its mapper, read-only, bounded, fixture-tested, and free of account,
  application, upload, portal, or private API behavior;
- ordinary CLI/AgentResponse, cache validators, and failure reports contain no raw private bodies, credentials,
  email addresses, absolute local paths, or connector session identifiers; and
- installed wheels include every discovery schema, skill update, fixture needed by runtime smoke, and migration
  document promised as a packaged resource.

## Validation Record

Tasks 0–7 were locally accepted on 2026-07-15:

- ADR-023 and this complete Stage 4 task graph freeze identity precedence, URL/provenance redaction, untrusted data,
  read-only source authority, partial-failure semantics, and continued direct URL/PDF intake.
- `JobLeadV2`, its generated `job-lead-v2.schema.json`, source-neutral provenance, RFC 3339 timestamps, stable
  source-ID/URL/fingerprint identity, private source-locator hashing, and atomic JSON-list replacement are
  implemented.
- RSS GUID, RSS 1.0 `rdf:about`, and Atom ID map to source-native identity when present. Existing six fields,
  jobs.ac.uk/generic feed commands, list-shaped output, example workflow, and `--lead-index` remain compatible.
- `new-job-from-lead --lead-id` resolves primary IDs and aliases from v2 lists or catalog objects and can derive a
  stable ID for legacy lists. Exactly one selector is required. Intake remains a lead-only untrusted advert stub and
  records `source_lead_id` without scraping or fetching the advert.
- The final focused identity/RSS/job/example suite passed 88 tests. The wider schema/workspace/release-resource
  focused suite passed 148 tests before the final URL edge hardening; all affected tests were rerun afterward.
- The final full development-interpreter suite passed 1,120 tests in 872.77 seconds. Python bytecode compilation and
  `git diff --check` passed.
- An isolated source distribution and wheel build succeeded. The packaged-resource checker confirmed the new Lead
  v2 schema is present, and Twine accepted both artifacts. Nothing was uploaded or published.
- Task 2 adds the strict `canisend.discovery-catalog/v1` contract and packaged schema. Retained Lead v2 records use
  contiguous one-based ranks; exclusions preserve the normalized lead plus structured filter reasons; catalog IDs
  are derived from deterministic content rather than input order or local paths.
- Union-based merge groups resolve explicit aliases, namespaced native IDs, canonical URLs, and safe fallback
  fingerprints. Conflicting strong identities are not silently merged. Primary IDs survive repeated refreshes while
  provenance, alternate IDs, and compatible fields are merged deterministically.
- Include/exclude keywords, source preference, metadata completeness, multi-source evidence, score deltas, stable
  tie-breaking, and exclusion reasons are inspectable. Exclusions win, and every retained score equals the sum of
  its structured reasons.
- `canisend discovery merge` accepts legacy lists, Lead v2 lists, or strict catalogs and writes atomically. Its text
  and `canisend.agent/v1` JSON surfaces report only a relative artifact, hash, catalog ID, safe counts, and next
  action; lead titles, descriptions, absolute workspace paths, and source bodies do not enter the response.
- After final catalog re-filter recovery hardening, the discovery/job/agent/schema/resource suite passed 233 tests.
  The complete development-interpreter suite had passed 1,140 tests in 878.81 seconds before that isolated change;
  all affected suites were rerun afterward. Bytecode compilation and `git diff --check` passed.
- A new isolated wheel and source distribution built successfully. The packaged-resource checker and Twine accepted
  both artifacts. A Python 3.12 clean install from that wheel exposed `discovery merge`, loaded the packaged catalog
  schema, and completed an offline merge smoke test. Nothing was uploaded or published.
- Task 3 adds strict `canisend.discovery-sources/v1`, `canisend.discovery-batch/v1`,
  `canisend.discovery-cache/v1`, and `canisend.discovery-refresh-report/v1` contracts with generated, packaged
  schemas. Source configurations currently admit only explicit RSS/Atom GET sources; vendor API adapters remain
  gated behind Task 6 conformance work.
- One shared public GET transport now serves discovery refresh, existing RSS/Atom fetch, and explicit HTML/PDF
  intake. It validates public DNS results before requests and after redirects, bounds media and response size,
  performs conditional requests, honors bounded `Retry-After`/backoff, throttles per host, and supports injected
  clocks and waits without exposing response bodies or untrusted response-header values in stable errors.
- `discovery refresh` sorts enabled sources, writes validated complete batches and validator-only caches atomically,
  reuses the complete batch on `304`, and promotes the merged catalog after source promotion. A failed or invalid
  source reuses its prior complete batch as explicitly stale while unrelated sources advance; an unusable run or
  catalog write failure preserves the existing catalog.
- The text and `canisend.agent/v1` JSON refresh surfaces expose only relative catalog/report artifacts, hashes,
  stable IDs, counts, status codes, warnings, and next actions. Source bodies, lead descriptions, query values,
  absolute paths, and exception details stay out of ordinary responses and the body-free refresh report.
- The final discovery, transport, intake-compatibility, agent, schema, and resource acceptance group passed 271
  tests. The complete development-interpreter suite passed 1,179 tests in 801.41 seconds; Python bytecode
  compilation and `git diff --check` passed.
- An isolated source distribution and wheel built successfully. The packaged-resource checker and Twine accepted
  both artifacts. A Python 3.12.12 clean install exposed `discovery refresh`, loaded all four packaged contracts,
  returned a one-line body-free AgentResponse for invalid input, and completed an offline successful refresh smoke
  test. Nothing was uploaded or published.
- Task 4 makes local exports an explicit discovery-only bridge: `.csv`, `.json`, `.eml`, and `.mbox` inputs become
  normalized Lead v2 candidates and never become application materials or full adverts. Selection still proceeds by
  stable lead ID, and complete advert intake still uses the existing URL, PDF, text, or manual-paste paths.
- CSV imports use a bounded alias table, reject ambiguous headers, ignore unknown column values, and record invalid
  rows only as stable row number/code/field tuples. JSON accepts lists of canonical/Lead v2 records or a strict
  `canisend.discovery-batch/v1` object; unversioned and vendor-specific envelopes fail closed.
- EML/MBOX imports parse the already hashed bytes, skip attachments, extract only credential-free public HTTP(S) job
  links and minimal user-visible link text, and ignore unrelated/footer links. Sender, recipient, subject, message ID,
  raw headers, bodies, local paths, and credential-like source IDs/locators never enter persisted artifacts or normal
  responses.
- `canisend.discovery-import-report/v1` provides a generated packaged schema, deterministic import ID, safe counts,
  bounded row issues, and relative batch/catalog paths. `discovery import` exposes the same information through text
  and `canisend.agent/v1` without returning lead content or source paths.
- Valid local imports are written as atomic complete Lead Batches under `job_leads/imports/`, deduplicated, filtered,
  ranked, and merged into the catalog. Repeating identical input reuses the complete batch; subsequent network
  refreshes continue to include current valid local batches.
- The final discovery, local-import, URL/PDF intake, agent, schema, and resource acceptance group passed 291 tests.
  The complete development-interpreter suite passed 1,199 tests in 843.85 seconds; Python bytecode compilation and
  `git diff --check` passed.
- An isolated source distribution and wheel built successfully. The packaged-resource checker and Twine accepted
  both artifacts. A Python 3.12.12 clean install exposed `discovery import`, loaded the packaged import/batch
  contracts, completed CSV and EML CLI imports, merged both leads, and passed a persisted-artifact privacy scan.
  Nothing was uploaded or published.
- Task 5 adds the strict, generated, and packaged `canisend.discovery-search/v1` envelope. It contains only a safe
  logical source, observation time, exact result count, and source-neutral public job fields. Unknown provider,
  host, query, header, cursor, session, or vendor fields fail closed before any artifact is written.
- `discovery import-search` normalizes every accepted result into Lead v2 with `host_agent` provenance and the fixed
  `host.search` adapter, removes tracking parameters, writes a complete batch atomically under
  `job_leads/searches/`, and enters the same catalog dedupe/filter/ranking pipeline. Network refresh continues to
  include current validated host-search batches.
- `discovery.search_import` is additive in `canisend.agent/v1`. Its response contains only relative catalog/batch
  paths, hashes, IDs, counts, and a next action; titles, snippets, raw envelopes, input paths, provider fields, and
  exception details remain outside the response.
- Codex, Claude, and generic-host fixtures vary result ordering and tracking parameters but produce byte-equivalent
  validated batch models and identical catalogs. Provider/session/cursor/header/query inputs, private locators,
  future timestamps, symlink escapes, invalid batches, and unexpected CLI failures have offline adversarial tests.
- The Task 5 discovery/agent/schema/resource acceptance group passed 209 tests. The complete
  development-interpreter suite passed 1,223 tests in 823.94 seconds; Python bytecode compilation and
  `git diff --check` passed.
- An isolated source distribution and wheel built successfully. The packaged-resource checker and Twine accepted
  both artifacts. A Python 3.12.12 clean install exposed `discovery.search_import`, imported a normalized host-search
  fixture, loaded the packaged search schema, produced two catalog leads, and passed a persisted-artifact scan for
  host names, tracking values, credentials, cursors, and private sentinels. Nothing was uploaded or published.
- Task 6 freezes one adapter protocol across RSS/Atom, Greenhouse, and Lever: each adapter owns one ID, one derived
  public GET URL, one redacted locator, one media contract, endpoint validation, and one Lead v2 mapper. Public API
  configuration accepts identifiers rather than arbitrary URLs or request options.
- The Greenhouse adapter uses the documented unauthenticated Job Board list endpoint with an explicit lowercase
  `board_token` and `content=true`. The Lever adapter uses the documented public Postings list endpoint with an
  explicit lowercase `site_id`, global/EU host selection, JSON mode, and one bounded result limit. Both endpoint
  contracts were verified against official documentation on 2026-07-15 and recorded in `docs/discovery-adapters.md`.
- Both adapters issue exactly one GET through the shared bounded public transport. They never expose or call an
  application POST, follow a response pagination/apply URL, accept credentials, or permit an arbitrary API host.
  Unknown record fields end inside the adapter; only documented published-job fields map into untrusted Lead v2.
- A shared offline conformance corpus covers valid vendor shapes, plain-text/HTML descriptions, IDs, locations,
  tracking removal, application-field exclusion, exact endpoints, EU Lever routing, GET/no-auth behavior, malformed
  identifiers, undocumented roots, redirects, record limits, one-request behavior, and stale complete-batch reuse.
- The final adapter/discovery/intake/agent/schema/resource acceptance group passed 253 tests. The complete
  development-interpreter suite passed 1,240 tests in 788.42 seconds; bytecode compilation and `git diff --check`
  passed.
- An isolated source distribution and wheel built successfully. The packaged-resource checker and Twine accepted
  both artifacts. A Python 3.12.12 clean install exposed `discovery refresh`, loaded the packaged source schema with
  all three adapter kinds, derived the exact Greenhouse/Lever GET URLs, and mapped both offline fixtures into four
  Lead v2 records. Nothing was uploaded or published.
- Task 7 adds one coherent Stage 4 operator surface across README, changelog, file contracts, privacy guidance,
  workflow references, the focused job-intake skill, canonical skill mirror, public synthetic examples, and an
  additive migration/rollback guide. Installed workspaces receive the discovery examples and migration guide while
  preserving local edits by default.
- Focused compatibility tests run local `.txt`, local PDF, explicitly supplied HTML URL, and explicitly supplied PDF
  URL intake through `new-job` and the existing application pipeline to Parsed Job plus compatibility materials.
  Legacy list/index, stable catalog/ID, jobs.ac.uk, and generic RSS/Atom coverage remains in the same acceptance
  group.
- The final Task 7 discovery/intake/agent/schema/resource group passed 374 tests; the focused documentation/resource
  group passed 68 tests. The complete development-interpreter suite passed 1,247 tests in 749.14 seconds; bytecode
  compilation, canonical-mirror check, and `git diff --check` passed.
- An isolated source distribution and wheel built successfully. The packaged-resource checker and Twine accepted
  both artifacts. A Python 3.12.12 clean install initialized 101 workspace defaults, reported them current, exposed
  `discovery refresh`, imported packaged CSV and host-search examples into one catalog, and selected a lead by stable
  ID while correctly blocking application work on the missing full advert. Nothing was uploaded or published.

Remote CI, cross-version/cross-OS exit acceptance, and the `0.6.0b1` Stage 4 release candidate remain Task 8; they are
not claimed here.

### Task 8 Local Candidate Evidence

The `0.6.0b1` candidate passed the local portion of Task 8 on 2026-07-15:

- Python 3.11 passed 1,248 tests in 825.95 seconds; Python 3.12 passed 1,248 in 930.29 seconds; Python 3.13 passed
  1,248 in 832.67 seconds; and the additional Python 3.14 development interpreter passed 1,248 in 749.16 seconds.
- The source-tree Stage 4 discovery smoke initialized a fresh workspace, validated every installed discovery schema,
  example and migration resource, mapped offline Greenhouse/Lever fixtures, imported CSV and normalized host search,
  deduplicated one catalog, selected by stable ID, retained the full-advert blocker, and passed the private-safe
  artifact scan.
- The `0.6.0b1` sdist and wheel built successfully; Twine and the wheel resource checker accepted both artifacts.
  A Python 3.12.12 clean install passed the Stage 4 discovery smoke and the complete Decision Spine smoke with 11
  successful stages and 20 mutation receipts.
- Canonical skill mirror, Bash syntax, bytecode compilation, release/resource contracts, and `git diff --check`
  passed. PyPI, TestPyPI, and GitHub returned no existing `0.6.0b1` release/tag before candidate preparation.
- CI and release workflows now run the offline Stage 4 discovery smoke from source, built wheel, and TestPyPI wheel;
  Linux, macOS, and Windows remote evidence is still required before the release tag can be created.

No tag was created and nothing was uploaded or published during local acceptance. The exact pushed candidate commit,
remote CI run, tag workflow, TestPyPI/PyPI artifacts, GitHub prerelease, and independent public-index install remain
open evidence.
