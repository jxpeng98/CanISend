# Privacy and consent

CanISend is local-first, but local storage alone does not make every operation private. A host Agent
or configured model provider may read or transmit data only at explicit, bounded consent points.
The same policy applies to `org.canisend.generic-application` and
`org.canisend.academic-job`; Pack vocabulary never weakens a privacy classification.

## Data classifications

| Classification | Examples | Default handling |
| --- | --- | --- |
| `public` | product versions, capabilities, schema IDs, public adapter names | safe for body-free inspection |
| `private-local` | opportunity sources, Profile Evidence, drafts, reviews, rendered PDFs | remains inside the Workspace unless explicitly read or exported |
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

Codex, Claude Code, Claude Desktop, or another MCP host performs reasoning while CanISend remains
the local state authority. Start one version-matched Agent v4 MCP process:

```console
canisend --workspace ./applications mcp serve
```

Routine orientation is body-free. Private inspection, provider send, network fetch, and local
export require the exact consent declared by the v4 preview. Academic and generic Applications use
the same protocol while retaining separate Pack bindings and Application-scoped data access.

External-host handoff is the recommended desktop integration. Codex or Claude owns its session, transcript, search,
plugins, connectors, and approvals; CanISend owns application state. The optional in-App bridge stores only a
body-free external session binding and remains read-only. The guarded MCP adapter exposes typed inspection and
preview-confirm operations but does not grant arbitrary filesystem or shell writes. Its App-owned
approval broker issues cryptographically random single-use tokens with a ten-minute monotonic
lifetime. A token is bound to the canonical Workspace, exact Pack and operation, reviewed snapshot,
and expected revision. Expiry, replay, wrong Pack, wrong Workspace, or stale revision causes no
mutation; a token is approval for exactly one reviewed commit, not future work.

Cancelling an in-App bridge turn terminates only the exact Workspace/runtime/Application-scoped local process. CanISend does
not parse or persist partial output, and a cancelled new turn cannot replace the last successful external session
binding. The host may still retain provider-side activity according to its own policy if transmission occurred
before cancellation.

## Provider-send consent

`send-to-configured-provider` confirms only the exact preview's frozen input scope. It is not
blanket permission for the Workspace, future revisions, unrelated Profile Sources, or later
tasks. CanISend does not discover provider credentials or transmit undeclared Workspace files.
The external host/provider remains responsible for retention, regional processing, model training,
and account policies.

## Public discovery versus private Application data

RSS/Atom, jobs.ac.uk, Greenhouse, and Lever discovery adapters are Academic Pack opportunity
sources. They perform bounded, read-only fetches from exact public hosts and never include Profile
Evidence, drafts, reviews, or Workspace bodies. Agent v4 connected intake also accepts a reviewed
URL, pasted text, local file, or text PDF for an exact Application in either built-in Pack.

User-supplied job URLs require explicit command invocation and are fetched through the same per-hop SSRF boundary.
Redirects are recorded as source metadata. Provider-specific discovery redirects must remain inside the adapter's
exact allowlist.

## Export boundaries

Private projections and PDFs require explicit private-export consent and remain under safe
Application-scoped paths. Unmanaged files and user edits are never overwritten implicitly. Export
means local filesystem publication, not Application submission, and receipts record
`submission_performed: false`.

Backups contain authoritative private data. Store, copy, encrypt, retain, and delete them according to the same or
stronger policy as the workspace. CanISend does not upload backups or implement automatic secure erasure.
