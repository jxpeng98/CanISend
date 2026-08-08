# Connected intake v4

Connected intake turns reviewed source bytes into one Workspace Source, explicit Application link,
and proposed Requirements. It never performs portal submission.

## Pasted-text preview

The first v4 intake adapter accepts an exact Pack ID, Application title and metadata, pasted UTF-8
text, one Pack-declared Requirement category, and a proposed priority. It treats each non-empty
line as one Requirement proposal, trims only the outer whitespace from that line, and records the
exact byte span of the resulting statement in the unchanged pasted text.

Preview validates the selected Pack, category, metadata, title, Source bounds, Requirement count,
statement bounds, control characters, and every UTF-8 span. It returns the exact prepared
Application request plus Source and preview SHA-256 digests. Preview does not write SQLite, Blobs,
projections, or audit events.

## Commit

Commit receives the same bounded preview request and the SHA-256 digest the user reviewed. The
application facade reconstructs the preview deterministically and refuses a stale or mismatched
digest before mutation. A matching preview delegates to the same neutral Application creation
service used by other in-process and headless surfaces.

The Store then commits the Application snapshot, Workspace Source revision, exact Source link,
Requirement spans, Blob references, and body-free audit event as one authority transaction. The
result remains in proposed-Requirement state until the user explicitly confirms Requirements and
selects a Plan.

## Local file and text-PDF preview

The local adapter accepts `.txt`, `.md`, `.json`, and text-based `.pdf` files selected by the user.
Private-read consent is required before Workspace or path access and is required again when commit
re-reads the path. The bounded IO adapters reject symlinks, non-regular files, unsupported types,
oversized inputs, invalid UTF-8, unsafe text controls, encrypted or malformed PDFs, PDFs over the
page limit, and PDFs without extractable text.

Text files retain their original bytes and deterministic normalized UTF-8 text. PDFs retain the
original PDF bytes and a page-qualified normalized text projection; synthetic page markers are not
proposed as Requirements. Preview reports both digests, byte and line counts, PDF page count,
content type, Source kind, and body-free exact-revision duplicate signals. An existing duplicate is
informational and does not silently reuse or cross-link another Application.

Commit re-reads and re-normalizes the same path after consent, reconstructs the complete preview,
and rejects changed bytes before any Blob, Source, Application, association, or audit mutation. A
matching digest atomically stores the original and normalized Blobs, a `LocalFile` or `TextPdf`
Source revision, the `ReadPrivateInputs` consent-bound Application link, and exact normalized-text
Requirement spans.

## URL preview and commit

URL intake requires explicit user-supplied-URL fetch consent before Workspace or network access.
The existing bounded HTTP adapter permits only HTTP(S), strips fragments, rejects embedded
credentials and non-public DNS destinations, pins each request to validated resolved addresses,
disables ambient proxies and automatic redirects, forbids HTTPS downgrade, limits redirect count
and response bytes, rejects unsupported encodings and misleading content types, and accepts only
HTML, UTF-8 text, or text-based PDFs.

Preview preserves the validated source URL, final URL, redirect chain, canonical content type,
original response bytes, normalized text, both digests, PDF page count where applicable, exact
Requirement spans, and exact-revision duplicate signals. Commit refetches through the same policy
and rejects any changed bytes, normalized text, redirect provenance, duplicate review state, or
Application request before mutation. A match atomically records the URL Source and its
`FetchUserSuppliedUrl` consent-bound Application link. It never sends credentials, logs in,
uploads, or submits an Application.

## Remaining surface work

Desktop, CLI, and Agent v4 bindings remain separate surface work. Scanned-document OCR remains
outside the 1.0 scope.
