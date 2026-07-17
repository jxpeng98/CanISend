# R4 Job Intake Evidence

**Date:** 2026-07-17

**Status:** Complete

## Job and source authority

SQLite migration 2 adds the Rust-native job/source revision links required by intake. `job create`, `job import`, and
`job archive` append audit events in the same transaction as their state changes. An import publishes original and
normalized bytes first, then atomically creates the source identity, two artifact identities and revisions, exact
normalization dependency, blob references, source revision, job revision, and audit event.

Original bytes and normalized text are always separate artifacts. Equal bytes reuse the immutable content-addressed
blob, but distinct imports retain distinct source and artifact identities. `job show` exposes hashes, revisions,
content type, retrieval time, source/final URLs, and redirect chain without returning the private source body.

## Local text policy

Local inputs must be regular non-symlink files with `.md`, `.txt`, or `.pdf` extensions. Text inputs are bounded to 16
MiB and must be UTF-8, optionally with a UTF-8 BOM. Normalization converts line endings, removes trailing line
whitespace, rejects NUL and unsafe control characters, rejects empty content, and produces one final newline. File
metadata is checked before a bounded streaming read so sparse or changed oversized files still fail closed.

## URL and HTML policy

The HTTP client uses Reqwest 0.13.4 with Rustls, disables automatic redirects and environment proxies, and applies
connect/request timeouts plus a 16 MiB decoded-body limit. Only credential-free `http` and `https` URLs are accepted.
Each request resolves the host, rejects loopback/private/link-local/multicast/unspecified/documentation ranges, pins
Reqwest to the checked addresses, and repeats the process for every redirect. HTTPS cannot redirect down to HTTP.

Content-Length is an early limit only; a bounded body reader remains authoritative. Content type, UTF-8 charset, PDF
signature, and bounded HTML sniffing must agree. Unsupported content encodings and misleading MIME declarations fail
closed. `html2text` 0.17.1 parses HTML with scripting disabled; the result passes the same text normalization and
decoded-size policy as local text.

Ephemeral local-server fixtures cover successful HTML plus redirect metadata, redirect limits, timeout, declared
oversize, truncated bodies, misleading MIME, and production rejection of loopback addresses. CI never needs a live
third-party site.

## PDF policy

R0 selected `pdf-extract` 0.12.0 over the lower-level adapter for initial extraction quality; R4 wraps it with Lopdf
0.42.0 preflight. Imports require a PDF signature, no encryption, 1–100 pages, at most 16 MiB of input and normalized
text, and completion within the declared extraction budget. Output uses explicit `--- Page N ---` separators.

Failures distinguish `pdf.encrypted`, `pdf.malformed`, and `pdf_text_unavailable`. The last result covers scanned or
otherwise image-only PDFs and tells the user to supply a text-based PDF, Markdown, or plain text. The original PDF is
preserved unchanged even though normalized text is stored separately.

The offline corpus covers normal text, positioned two-column/table-like academic layouts, university branding text,
empty/image-only pages, malformed/truncated cross-reference data, encrypted documents, and a 101-page limit case.
The R0 embedded-Typst fixture additionally proves extraction from an embedded subset-font PDF on Ubuntu, macOS, and
Windows.

## CLI and verification evidence

Available commands are:

```text
canisend --workspace ./workspace job create --title TITLE --institution INSTITUTION --json
canisend --workspace ./workspace job import JOB_ID --file ./advert.md --json
canisend --workspace ./workspace job import JOB_ID --file ./advert.pdf --json
canisend --workspace ./workspace job import JOB_ID --url https://example.edu/advert --json
canisend --workspace ./workspace job list --json
canisend --workspace ./workspace job show JOB_ID --json
canisend --workspace ./workspace job archive JOB_ID --json
```

Local verification passed 36 Rust tests, Clippy with warnings denied, 18 generated schema checks, 24 embedded resource
checks, release compilation, and packaged-binary create/import/show/integrity smoke. GitHub Actions run `29614087317`
passed the cold clean-checkout gate in 3 minutes 52 seconds; run `29614367500` repeated the final regression state in
1 minute 10 seconds. `job.intake` is now truthfully `available`; R5 can reuse the same safe URL transport and promote
selected discovery leads through this intake boundary.
