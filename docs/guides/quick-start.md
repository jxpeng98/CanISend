# Quick start

This path imports an academic job advert and prepares it for a user or agent host. Commands use human-readable output;
add `--json` when a program needs the stable `canisend.agent/v2` envelope.

## 1. Verify and initialize

```console
canisend doctor
canisend --workspace ./applications workspace init
```

The workspace contains private application material. Keep it in a user-controlled directory and do not sync it to a
public repository.

## 2. Create a job

```console
canisend --workspace ./applications job create \
  --title "Lecturer in Economics" \
  --institution "University X"
```

Copy the printed Job ID for later commands. In an agent integration, read `data.id` from the JSON response instead of
scraping human text.

## 3. Import one or more sources

Local Markdown, UTF-8 text, and text-based PDF:

```console
canisend --workspace ./applications job import JOB_ID --file ./job-advert.md
canisend --workspace ./applications job import JOB_ID --file ./person-specification.pdf
```

A user-supplied public HTTP(S) link is also supported:

```console
canisend --workspace ./applications job import JOB_ID \
  --url https://jobs.example.edu/vacancy/123
```

Each import creates a distinct source identity and advances the job revision. URL fetches reject credentials,
non-HTTP(S) schemes, private/non-public addresses, unsafe redirects, misleading content types, oversized responses,
and excessive redirect chains.

Scanned or image-only PDFs are intentionally unsupported. If the command reports `pdf_text_unavailable`, obtain a
text-based advert or use a trusted OCR tool, review the extracted text, save it as Markdown/plain text, and import
that reviewed file. CanISend does not run OCR automatically.

## 4. Start preparation

```console
canisend --workspace ./applications job show JOB_ID
canisend --workspace ./applications workflow start --job JOB_ID
canisend --workspace ./applications workflow status --job JOB_ID
```

The workflow advances through intake, parsing, criteria confirmation, evidence normalization/matching, the explicit
apply/hold/skip decision, structured drafting, review, package readiness, and local PDF rendering. It never submits
the application.

For interactive reasoning, continue with the [agent integration guide](agent-integration.md). For every meaningful
session, finish with:

```console
canisend --workspace ./applications workspace check
canisend --workspace ./applications workspace backup ./applications-backup
```
