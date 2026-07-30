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

## Desktop workflow memory

The desktop persists a small body-free navigation record in its WebView storage so reopening the App can restore the
active screen, selected workspace/application, exact sub-screen, and most recent successful action. It may contain a
canonical workspace path, public job ID, operation name, short action-receipt summary, route, and timestamp. It never
contains advert/profile bodies, draft text, task inputs, provider prompts or responses, transcripts, tokens, or
credentials.

This record is a convenience pointer, not authoritative state. The App validates and bounds it before use, ignores a
job that no longer belongs to the selected workspace, and reloads the current job/workflow from the Rust facade.
Switching workspaces does not offer a resume action from another workspace. Clearing WebView storage removes the
navigation memory without deleting or changing a CanISend workspace.

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

External-host handoff is the recommended desktop integration. Codex or Claude owns its session, transcript, search,
plugins, connectors, and approvals; CanISend owns application state. The optional in-App bridge stores only a
body-free external session binding and remains read-only. The guarded MCP adapter exposes typed inspection and
preview-confirm operations but does not grant arbitrary filesystem or shell writes.

Cancelling an in-App bridge turn terminates only the exact workspace/runtime/job-scoped local process. CanISend does
not parse or persist partial output, and a cancelled new turn cannot replace the last successful external session
binding. The host may still retain provider-side activity according to its own policy if transmission occurred
before cancellation.

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
