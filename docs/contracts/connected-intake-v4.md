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

## Remaining adapters

Local file, text-based PDF, and URL adapters must reuse this prepared Source/Application boundary.
They additionally own bounded input parsing, provenance, duplicate signals, and the exact required
private-read or network-fetch consent. Scanned-document OCR remains outside the 1.0 scope.
