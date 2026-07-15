# ADR-023: Version Discovery Identity, Provenance, And Read-Only Sources

**Status:** Accepted for Stage 4

**Date:** 2026-07-15

## Context

The existing RSS and Atom workflow stores a JSON list whose entries have six useful compatibility fields, but list
position is the only selection handle. Positions change when sources refresh, filters change, or lists merge. The
records also lack a versioned identity, normalized URL, source-native identifier, fetch timestamps, and an explicit
explanation of why a lead was retained or excluded. Adding more sources on that basis would make duplicate handling
and host-agent handoff unreliable.

Discovery is an input service, not an application portal. A source adapter must not inherit authority to create an
account, submit an application, upload a document, emulate a private API, or crawl adjacent pages. A direct user-
supplied HTML/PDF URL and a local PDF/text advert remain peer intake paths and must continue through the existing
bounded single-advert importer.

## Decision

Introduce strict `JobLeadV2` and `LeadBatchV1` contracts while retaining the six legacy lead fields and the legacy
top-level JSON-list format produced by `fetch-jobs-ac-uk` and `fetch-job-feed`. A v2 lead adds:

- a content-derived `lead_id` and its identity method;
- a redacted canonical HTTP(S) URL and optional source-native record ID;
- institution, location, and deadline when supplied by a source;
- first-seen, last-seen, and fetched timestamps;
- ordered provenance records and alternate lead IDs created by deterministic merging; and
- a score plus structured match reasons that explain filtering and ranking.

Identity is derived from a namespaced source-native record ID first, a canonical URL second, and a normalized
title/institution/deadline fingerprint last. Source-native IDs are always namespaced by the normalized source label.
Merge additionally compares canonical URLs and fallback fingerprints. The deterministic survivor retains all merged
IDs as aliases, so `--lead-id` can resolve an earlier source identity after deduplication. Legacy `--lead-index`
remains available, but exactly one selector is required.

Canonicalization lowercases the scheme and host, removes fragments, default ports, known tracking parameters, and
credential-like query parameters, and produces deterministic query ordering. Feed provenance redacts query values.
No credential, API token, raw request header, exported-email sender/recipient address, connector session identifier,
or absolute local source path may be written to a lead, batch, report, cache validator record, ordinary log, or
AgentResponse. A public contact address already present inside a published job description remains untrusted advert
content rather than provenance.

The discovery service boundary is:

```text
source adapter -> bounded transport -> normalization -> filter -> deduplicate -> rank -> atomic batch write
```

The shared transport alone owns public-address validation before a request and after redirects, response byte and
media-type limits, conditional requests, retry/backoff, `Retry-After`, and per-host throttling. It exposes response
bodies only to the selected adapter. Cache metadata contains only a source ID, redacted URL, ETag, Last-Modified,
timestamp, and content hash. A `304` reuses the last complete source batch. A failed source preserves its previous
complete batch as explicitly stale while successful sources advance; it never causes a partially written source or
catalog file.

Local CSV/JSON and exported `.eml`/`.mbox` alerts are explicit imports. Email import extracts only normalized public
job links and user-visible link text; it does not persist the original message, sender address, recipients, message
ID, or unrelated links. Host agents import one normalized, versioned search-result envelope rather than vendor-
specific payloads. Unknown vendor fields fail closed at that boundary.

Greenhouse and Lever adapters are read-only implementations over their documented public job-posting GET
interfaces. They accept an explicit board/site identifier, use no stored credentials, follow no pagination link or
job URL outside the documented response contract, and expose no application endpoint. Every adapter must pass the
same offline conformance fixtures before it is registered.

Discovery leads and catalogs are untrusted imported data. Creating a job from a lead writes a lead-only advert stub;
the full advert must still be supplied by local text/PDF, explicit bounded URL fetch, or manual paste before parsing
and application generation can be considered current.

## Consequences

- repeated refresh and reordering can use stable identifiers instead of array positions;
- multiple sources can be merged without losing source attribution or hiding exclusions;
- Codex, Claude, CLI, and future local protocol hosts can exchange one normalized discovery artifact;
- a failed source is inspectable without discarding unrelated successful work;
- existing jobs.ac.uk lists, `--lead-index`, direct URLs, and local PDFs remain readable; and
- adding a source cannot expand CanISend into account, portal, upload, or submission behavior.

## Rejected Alternatives

- Use list index as durable identity: rejected because refresh and ranking reorder lists.
- Hash only title text: rejected because common titles collide across institutions and dates.
- Put vendor response objects directly in the core model: rejected because vendor drift would leak into every host
  contract and could retain undocumented private fields.
- Store raw responses to make `304` work: rejected because the previous validated source batch is sufficient and raw
  responses increase privacy and migration risk.
- Scrape arbitrary job-result pages: rejected because page structure is unstable and a search result does not grant
  crawling authority.
- Let source failure abort the whole refresh: rejected because it discards valid independent source batches.
- Replace direct URL/PDF intake with discovery: rejected because user-supplied adverts are an equally important,
  bounded intake channel.

## Revisit When

Revisit before authenticated enterprise connectors, server-side shared catalogs, background scheduling, mutable
remote state, portal assistance, or a Lead v3 that changes the six compatibility fields or identity precedence.
