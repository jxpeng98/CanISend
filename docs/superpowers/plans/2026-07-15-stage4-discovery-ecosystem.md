# Stage 4 Discovery Ecosystem Implementation Plan

**Status:** In progress — Tasks 0–3 locally accepted; Task 4 is next

**Date:** 2026-07-15

**Branch:** `feat/discovery-ecosystem`

**Baseline:** Stage 3 `0.3.0b1`, post-release evidence commit `cd4f2124669076133faa11469470aa10c5e9a70c`

**Candidate milestone:** `0.6.0a1` after Stage 4 exit acceptance; no publication is implied by this plan

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

- [ ] Add strict CSV field aliases and row-level validation/reporting.
- [ ] Accept JSON lists and versioned CanISend lead batches without silently accepting vendor-specific envelopes.
- [ ] Add `.eml` and `.mbox` alert extraction without persisting raw headers, bodies, local absolute paths, or
  unrelated links.
- [ ] Route every record through the same normalization, dedupe, ranking, and atomic writer.

### Task 5: Host-Agent Search Import

- [ ] Add a strict `canisend.discovery-search/v1` envelope and schema.
- [ ] Normalize connector/search results into Lead v2 without storing provider credentials or opaque sessions.
- [ ] Expose additive discovery operations/capabilities and body-free result counts/paths through
  `canisend.agent/v1`.
- [ ] Add Codex/Claude/generic-host fixtures proving identical imported catalogs.

### Task 6: Adapter Conformance, Greenhouse, And Lever

- [ ] Freeze adapter protocol and a shared offline conformance corpus before registering network adapters.
- [ ] Implement Greenhouse published-job GET mapping from an explicit board token.
- [ ] Implement Lever published-posting GET mapping from an explicit site identifier.
- [ ] Reject malformed identifiers, undocumented response roots, auth/credential options, non-GET behavior, and
  application/submission endpoints.
- [ ] Document the exact official read-only endpoints and date of verification.

### Task 7: Compatibility, Skills, And Migration

- [ ] Update README, changelog, file contracts, privacy guidance, job-intake skill, canonical mirror, and examples.
- [ ] Add Stage 4 migration guidance for legacy lists, indexes, catalog selection, caches, and rollback.
- [ ] Prove explicit URL HTML/PDF and local PDF/text imports still enter the same job/application workflow.
- [ ] Add all public schemas and fixtures to workspace and wheel resource checks.

### Task 8: Stage 4 Exit Acceptance

- [ ] Run focused adversarial identity, privacy, transport, batch, import, adapter, and compatibility tests.
- [ ] Run the full supported Python 3.11-3.13 matrix plus the available development interpreter.
- [ ] Run source-tree, built-wheel, clean-install, schema/resource, Twine, mirror, and CLI/agent smoke gates.
- [ ] Require remote Linux/macOS/Windows CLI smoke before marking the stage complete.
- [ ] Record immutable evidence and decide separately whether to publish the `0.6.0a1` candidate.

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

Tasks 0–3 were locally accepted on 2026-07-15:

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

Remote CI, cross-version/cross-OS acceptance, local and host-agent imports, public API adapters, compatibility/docs,
and a Stage 4 release candidate remain later tasks; they are not claimed here.
