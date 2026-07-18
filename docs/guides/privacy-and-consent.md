# Privacy and consent

CanISend is local-first, but local storage alone does not make every operation private. A host agent or configured
model provider may read or transmit data only at explicit, bounded consent points.

## Data classifications

| Classification | Examples | Default handling |
| --- | --- | --- |
| `public` | product versions, capabilities, schema IDs, public adapter names | safe for body-free inspection |
| `private-local` | adverts, CV/profile evidence, drafts, reviews, rendered PDFs | remains inside the workspace unless explicitly exported |
| `provider-bound` | the exact task input scope approved for a configured provider | may be exported only with private-read and provider-send consent |
| `secret` | API tokens, passwords, account credentials | never accepted as normal task or document content |

JSON success/error envelopes contain metadata, IDs, hashes, counts, blockers, and next actions by default. Commands
that need full bodies export a declared set to a new or empty external directory after consent.

## Host-agent mode

`--mode host-agent` means the already active Codex, Claude, or other host performs reasoning. Preparing a task does
not reveal bodies. To export only its declared inputs, the user must approve private reading:

```console
canisend --workspace ./applications task inputs TASK_ID \
  --destination ./agent-work/TASK_ID \
  --allow-private-read
```

The exported manifest freezes exact artifact IDs, revisions, and SHA-256 values. The host should read only that
directory and return candidate JSON through `task complete`.

## Configured-provider mode

`--mode configured-provider` adds a second boundary. Export requires both flags:

```console
canisend --workspace ./applications task inputs TASK_ID \
  --destination ./provider-work/TASK_ID \
  --allow-private-read \
  --allow-provider-send
```

`--allow-provider-send` confirms only the descriptor's frozen input scope. It is not blanket permission for the
workspace, future revisions, unrelated profile sources, or later tasks. CanISend does not silently discover provider
credentials or transmit undeclared workspace files. The external host/provider integration remains responsible for
its own retention, regional processing, model-training, and account policies.

## Public discovery versus private application data

RSS/Atom, jobs.ac.uk, Greenhouse, and Lever discovery adapters perform bounded, read-only fetches from exact public
hosts. Discovery requests do not include a CV, profile evidence, drafts, reviews, or workspace bodies. Promoting a
lead creates a local job; it does not send private data back to the discovery source.

User-supplied job URLs require explicit command invocation and are fetched through the same per-hop SSRF boundary.
Redirects are recorded as source metadata. Provider-specific discovery redirects must remain inside the adapter's
exact allowlist.

## Export boundaries

Editable application projections and PDFs require `--allow-private-export`. Destinations must be safe paths under
`jobs/JOB_ID/`, and unmanaged files or user edits are never overwritten implicitly. Export means local filesystem
publication, not application submission.

Backups contain authoritative private data. Store, copy, encrypt, retain, and delete them according to the same or
stronger policy as the workspace. CanISend does not upload backups or implement automatic secure erasure.
