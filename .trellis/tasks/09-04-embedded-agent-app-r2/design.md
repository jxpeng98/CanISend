# R2 technical design

> 2026-09-06 closeout: App-first execution is superseded by CLI-first delivery under
> [ADR-RN-0023](../../../docs/architecture/rust-native/decisions/0023-prioritize-cli-first-local-agent-workflows.md). R0/R1 source completion is retained; R2 is partial
> and unaccepted; unfinished R2-R6 scope is deferred. Historical checklists below are not the
> active queue. The master roadmap owns the next CLI-first slice.

Status: R2a bound-server foundation implemented; remaining design pending acceptance
Updated: 2026-09-05

## 1. Existing owners

```text
existing Agent view -> bounded R1 App Server client
                    -> required Application-bound CanISend MCP
                    -> existing canisend-app consent/preview/commit/verify
                    -> Store revision + audit + Blob authority
```

Extend these owners in place. Session transport, the product facade, and storage keep separate
responsibilities. This plan adds no alternate mutation path, provider abstraction, or event store.

## 2. R2a protocol and configuration gate

The [2026-09-05 executable probe](research/r2a-boundary-evidence.md#real-app-server-probe--2026-09-05)
proves named `permissions` (experimental capability required), canonical-path reads, required MCP
startup failure, and explicit MCP elicitation on 0.152.0. Preserve the remaining integration gate:
a separate account-only probe confirms existing account recognition; the unauthenticated local
model fixture does not establish signed-in inference or all inherited-configuration controls. Do not fall back to the broader legacy sandbox on any failure.


Inspect the exact supported CLI's generated App Server/config schemas before selecting configuration
fields. Prove effective process policy before initialization and preserve it through start, resume,
and turn dispatch. R1's controlled directory and `sandbox = "read-only"` do not restrict all reads
or disable already-permitted commands. Declining a request only protects operations that ask.

The required policy permits provider communication and the selected CanISend stdio MCP process,
but not direct model file/command tools or unrelated MCP/plugins/hooks. Verify inherited global,
project, and thread settings cannot expand that policy. Use supported per-process configuration and
authentication reuse; do not alter global files, copy credentials, or assume changing `cwd` isolates
configuration. Do not copy newer read-access fields into the 0.152.0 protocol without evidence.

The deterministic fixture must distinguish the actual MCP approval request from arbitrary
`item/tool/requestUserInput`, elicitation, and host approval. The supported version must prove
required-server readiness, per-tool prompting, exact item correlation, denial behavior, and a user
reviewer. An inherited automatic reviewer is incompatible with the explicit in-App consent contract.
If an effective restriction or correlation is unavailable, private context/tools and commits stay
disabled; the provider diagnostic names the missing capability.

The MCP configuration uses `desktop_cli_source_path()` and the validated Workspace/Application.
Pass the same binding at start and resume. Do not store project/global Codex configuration. A scope
switch revokes all pending approvals and starts/resumes only a thread registered to the new scope.

## 3. MCP binding and tool classification

Add one optional bound Application to `CanISendMcpServer` and route all Application-ID entry paths
through one instance guard. Today `application.show`, `profile.association.list`, and
`evidence.association.list` bypass `parse_application_id`; converting only that parser is incomplete.

Inventory every current tool's request and response, including no-ID Workspace lists. Bound mode
returns only selected-Application, explicitly associated, permitted fields; otherwise omit/disable
the tool. In particular, full `StoredApplicationModel` output is not a metadata allowlist. Preserve
the unbound external server's current contract. Reuse existing bounded reads; add a new product read
only if a required user journey cannot obtain its explicitly consented data through an existing one.

Classify tools by actual returned content and side effect, not just `read_only_hint`. Keep metadata
reads automatic only when their output is safe for the selected scope. Private-read tools and
private preview inputs require exact context/provider-send consent even when no record changes.
The `writes` default may remain a base policy, but explicit private-tool overrides and user response
correlation must enforce the stronger requirement before dispatch.

## 4. Consent and approval lifecycle

The observed 0.152.0 MCP request is `mcpServer/elicitation/request` with thread/turn/server and
`_meta.codex_approval_kind=mcp_tool_call` plus `tool_params`. It lacks a structured item ID/tool name.
Correlate only to exactly one active, allowed MCP item with matching scope and arguments; missing,
concurrent, ambiguous or stale matches fail closed. Human-readable question text is not authority.
Only after this state is wired and tested may the embedded MCP flow use `on-request` with the user
reviewer. Existing transport-only sessions retain their denial behavior until then.


Reuse `PrivateReadConsent`, provider-send/export consent, and the single-use mutation broker.
The model cannot establish user consent by setting a boolean. The desktop displays the selected
content categories/identities, destination provider, operation, and affected Application. Bind the
response to current revision/content digest and thread/turn/item; changing any binding invalidates
it. Use existing grant mechanisms where possible, adding only a bounded ephemeral correlation value
if the proven transport needs one. Never persist raw grants/tokens or grant a whole session implicitly.

For a guarded commit, retain the preview operation/token/digest/expiry in memory and verify the
matching tool and commit arguments before showing the sanitized approval card. Resolve once through
a bounded response channel. Timeout, denial, cancellation, switch, crash, unknown shape, or replay
fails closed. The MCP broker still validates revision/digest and consumes the token at commit.
Host command/file/extra-network permission requests are separately labelled and denied in the MVP;
they never offer a CanISend approval action.

Private result bodies may reach the provider only under the matching active consent. UI rendering,
logs, and durable metadata must not silently widen that transfer. Redact default diagnostics. After
a completed mutation, report the verified product result even if later trace persistence fails;
never retry a mutation to repair its history label.

## 5. R2b durable origin

Registry v3 retains bounded session metadata and transient read/denial/tool correlation; v1/v2 migrate
losslessly. Cache eviction or explicit session deletion is allowed. Committed provenance belongs
beside product audit and must not depend on that cache.

Use a small append-only origin relation in the same Store migration as file snapshots, keyed by the
canonical `audit_event_id`. It contains only Application/revision/snapshot binding, desktop and
provider session/turn/item IDs, tool/operation, preview digest/approval outcome, optional output
digest, and timestamp. No prompt, result body, evidence, token, or credential is stored there.

A narrow facade reconciliation operation attaches an observed origin to an already verified product
receipt/audit event. Resolve the canonical ID in Store from the receipt's exact binding; if existing
typed receipts lack an unambiguous reference, expose the minimum canonical field through the owning
contract. Never select a convenient audit row by approximate timestamp or accept model-invented IDs.
Verify Workspace/Application/revision/digest/operation and committed outcome. The canonical audit ID
is unique: an identical attachment is idempotent; a conflicting attachment is rejected.

This is audit metadata, not a product commit bypass or a second event stream. Post-commit attachment
failure leaves a visible trace gap; unattached historical events remain valid with unknown origin.
Existing database backup/restore carries durable origin rows. Session deletion removes session cache,
not product audit. Denied/read-only attempts have only the explicitly bounded session retention.

The history read joins immutable Application revisions, canonical audit identity, and optional durable
origin. Keep it bounded and body-free; do not reload provider conversations to reconstruct audit.

## 6. Exact file snapshots and write ordering

Use the next additive migration, expected 0021; resolve its number at implementation time.

```text
application_file_snapshots_v1
  id, application_id, predecessor_id, generation_kind,
  application_revision, application_snapshot_sha256,
  pack_id, pack_version, pack_digest, generator_version, generator_build_identity,
  manifest_sha256, actor, reason, created_at

application_file_snapshot_entries_v1
  snapshot_id, relative_path, file_kind, media_type, byte_count, sha256

application_agent_origins_v1
  canonical audit_event_id + bounded origin fields from section 5
```

The entry key is `(snapshot_id, relative_path)`. Paths are safe logical output paths relative to the
generation root, not destination-prefixed Workspace paths. Exporting `cv.pdf` under two chosen
directories compares the same logical file; the existing export audit retains physical location.
Do not mix projection and export predecessor chains. Record actual packaged source/build identity,
including an explicit development identity when release provenance is unavailable; semver alone
cannot distinguish generators that all report Beta.1.

Sort and validate the metadata manifest (at most 256 entries) and check its digest on read. Register
exact bytes through existing BlobStore and ordinary `blob_references`. Reuse a snapshot only if the
current same-kind head has identical Application/revision/Pack/generator binding and manifest. Do
not add a global unique constraint on `(Application, kind, manifest digest)`: A -> B -> A must retain
a new link, and a changed generator must retain provenance even when bytes are equal.

Projection ordering:

1. Generate and validate the complete batch; store exact bytes in BlobStore.
2. In the existing immediate transaction, revalidate the Application and insert/reuse snapshot,
   entries/references, and pending projection state bound to that snapshot.
3. Publish files and update observations. Interrupted publication remains repair-required.
4. Repair and fresh-path restore of these snapshots use verified retained Blobs. Do not call the
   current serializer to reproduce old bytes. Keep unsnapshotted legacy recovery explicitly separate.

Export ordering:

1. Render/validate all bytes and store them in BlobStore.
2. Create the all-or-cleaned export batch using the existing create-new helper.
3. In one immediate transaction, revalidate and insert/reuse snapshot, references, and export audit.
4. On database failure remove only the new batch. Surface a failed cleanup as recovery-required;
   never remove pre-existing destination files. Preserve existing unreferenced-Blob cleanup policy.

No directory scan discovers snapshot inputs. Preserve existing transaction and recovery owners.

## 7. Desktop read services and limits

Store supplies bounded history, manifest metadata, and selected verified entry pairs. `canisend-app`
owns presentation-neutral comparison types. Tauri exposes versioned history, manifest-compare, and
selected-file-compare leaves; TypeScript renders escaped typed results without another diff engine.

Manifest A/M/D/U comparison never reads Blob bodies. Selected text comparison verifies Application
ownership and both requested Blobs, then uses `similar` 3.2.0 Myers with exact whitespace/line endings,
three context lines, 4 MiB and 20,000 lines per side, one path, 2,000 output rows, and 250 ms.
Binary/non-UTF-8 or limited results carry metadata and opaque preview handles; never imply a complete
text diff when a limit is reached. Missing/corrupt Blobs fail integrity checks. Equal digests need
no body read. Keep these historical private-body operations desktop-only.

## 8. Rollback and scope

Disable embedded MCP/private context and new history entry points; retain R1 conversation only when
its own capability gate passes, external handoff, and manual product operations. The additive schema
uses the existing future-schema refusal, so an older executable requires a compatible backup rather
than an unsafe database downgrade. R3 presentation and provider-owned history loading are separate
consumers of the contracts proven here.
