# R2 CanISend MCP and safe review loop

> 2026-09-06 closeout: App-first execution is superseded by CLI-first delivery under
> [ADR-RN-0023](../../../docs/architecture/rust-native/decisions/0023-prioritize-cli-first-local-agent-workflows.md). R0/R1 source completion is retained; R2 is partial
> and unaccepted; unfinished R2-R6 scope is deferred. Historical checklists below are not the
> active queue. The master roadmap owns the next CLI-first slice.

Status: In progress — R2a bound-server foundation; effective provider/consent gate pending
Date: 2026-09-04

## Goal and authority

Connect one selected Workspace v4 Application to the embedded Codex session through the existing
CanISend MCP/facade, then retain exact managed-file history and durable body-free action origins.
The [master roadmap](../../../docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md#33-approved-app-first-delivery-sequence)
owns ordering. The owner requested this plan revision and direct development without Trellis skills;
the subsequent execution instruction authorizes source implementation, not a release. See the
[execution checklist](implement.md) for completed work, evidence, and remaining gates.

R0/R1 source and recorded local checks exist at the reviewed branch head
`d7c0abfea46aa8d7b9f63d9b1ac7bd7fac93abea`. R1 proves session transport, not private-data isolation,
MCP consent, or restored conversation display. Keep the active feature-freeze protocol.

## R2a — Bound and consented product work

### Prove the effective host policy first

- Qualify the exact installed Codex version, generated schema, and deterministic fake-server
  request sequence. Planning inspected `codex-cli 0.152.0`; newer documentation is not proof that
  this version implements a capability.
- Configure isolation before process initialization, then preserve it on start, resume, and every
  turn. A controlled working directory, `read-only` sandbox, and denial of approval requests alone
  do not prevent already-allowed file reads or commands.
- Prove that unrelated user/project MCP servers, plugins, hooks, tools, and configuration cannot be
  inherited as authority. Reuse supported Codex authentication without reading or copying credentials
  and without changing the user's global configuration.
- Use only disposable local sentinel files and an unrelated fake MCP server to prove no direct
  Workspace/private-file reads, command execution, or extra filesystem/network authority. Provider
  transport and the explicitly approved CanISend MCP process remain the intended capabilities.
- If the tested provider cannot enforce this boundary, keep private context, private tools, and
  mutations disabled. Continue independent history work; do not claim R2a complete.

### Bind every product-tool path

- Add optional `--application` to `canisend mcp serve`. One server-instance guard validates the
  selected ID before any Application-scoped facade access, including handlers currently using
  `validate_application_id` directly rather than `parse_application_id`.
- Inventory the complete tool catalog and returned fields. In bound mode, constrain or disable
  Workspace-wide listing tools, including no-ID calls. `application.list/show` and association/source
  lists are not automatically safe metadata merely because their tools are read-only.
- Inject one required stdio `canisend` server for the selected Workspace/Application using the unified
  executable on start and resume. Missing selection or required-server startup failure is fail-closed.
  Leave explicitly unbound external MCP behavior unchanged.

### Separate consent from mutation approval

| Operation class | Required authority |
| --- | --- |
| Allowlisted non-private metadata | Selected Application binding; no extra prompt |
| Private content read and provider transmission | Explicit user consent for the exact selected content/revision, operation, provider, and current thread/turn |
| Product commit | Existing single-use preview token, digest/revision binding, and explicit in-App approval |
| Export | Existing review/readiness checks, separate private-export consent, and guarded export approval |

`default_tools_approval_mode = "writes"` is not sufficient: `deliverable.audit`, `review.inspect`,
and other read-only tools can return private bodies. A model-supplied `confirmed_private_read: true`
is not a user grant. Use the existing consent types/broker with schema-qualified per-tool prompting
and correlated user responses; no blanket session consent, inherited automatic approver, or prompt
instruction may supply missing authority. A user-initiated context selection may grant its displayed
scope; unrelated content still requires consent.

The App must distinguish **private context consent**, **CanISend change approval**, and denied
**Codex host permission** requests. Match the exact MCP item, tool, Application, operation, selected
revision/content digest, provider thread/turn, and preview token/digest where applicable. An arbitrary
model question is not an approval request. Keep raw bodies and tokens only in bounded process memory.
Denial, timeout, cancellation, scope switch, stale content, replay, malformed correlation, or process
exit revokes pending authority. A failed verification never becomes a successful commit label or an
automatic mutation retry. The existing `canisend-app` broker remains the final mutation authority.

## R2b — Durable origin and exact output history

- Keep session registry v3 bounded and body-free, with lossless v1/v2 reads. Its cache may expire;
  committed action origins must survive cache eviction, session deletion, and Workspace backup/restore.
- Attach minimum origin identifiers to canonical product audit identity through the existing
  facade/Store. Derive them only from observed CanISend MCP items and verified typed receipts. An
  attachment failure after a real commit reports a trace gap without retrying the product mutation.
- Expose bounded Application revision/audit history with optional durable origin. Existing revisions
  without an origin remain valid and are labelled accordingly; never infer an origin from prose.
- Use one additive Store migration (0021 if still next) for immutable file snapshots/entries and the
  minimum audit-origin relation. Snapshot bytes use existing BlobStore and `blob_references`.
- Snapshot only generated in-memory projection/export batches, at most 256 safe logical paths relative
  to their generation root. Keep export destination in the existing export record, outside path
  identity. Bind Application revision/digest, Pack, generation kind, actual generator build/source
  identity, predecessor of the same kind, manifest digest, actor, reason, and time.
- Reuse only an identical current head with the same complete generation binding. Same bytes under a
  different build/revision retain provenance; A -> B -> A retains the new predecessor link. Blob
  deduplication is independent of history identity.
- Projection snapshots commit with pending publication state; repair/restore of newly snapshotted
  files reads their verified retained Blobs, not today's serializer. Legacy unsnapshotted files keep
  their explicit existing recovery policy and are not labelled exact historical output.
- Export records/snapshots commit after create-new output publication succeeds. A later database
  failure cleans only that new batch; cleanup failure is surfaced through existing recovery handling.
- Compare manifests without bodies, then one selected verified Blob pair on demand. `similar` 3.2.0
  remains a qualified candidate until implementation locks and rechecks it. Use bounded Myers text
  hunks: exact whitespace/line endings, three context lines, 4 MiB and 20,000 lines per side, 2,000
  emitted rows, 250 ms. Return explicit limited/binary states and opaque preview handles.
- Expose history and comparison as desktop read operations; historical private bodies are not MCP
  tools. Do not add Git, directory scans, a transcript store, or persisted patches.

## Acceptance and evidence

- [ ] R2a effective isolation passes on start/resume/turn, including hostile inherited configuration
      and denied local sentinel access, before any private data or product mutation is enabled.
- [ ] Every ID-bearing tool rejects another Application; no-ID/list calls cannot expose unrelated
      Applications or unassociated private sources. Unbound external behavior retains its tests.
- [ ] Both Packs complete metadata read, explicitly consented private read/provider send, and a
      representative preview -> approve -> commit -> verify flow through the existing facade.
- [ ] Model-asserted consent, inherited auto-approval, denied/stale/replayed/scope-switched requests,
      unknown approval shapes, timeout/cancel, and process exits grant no unintended authority.
- [ ] A committed body-free origin remains available after registry eviction/deletion and fresh-path
      restore. An attachment failure reports the actual committed state and an explicit trace gap.
- [ ] Exact output history handles generator upgrades, different export destinations, identical
      consecutive builds, A -> B -> A, Blob corruption, interrupted publication, and restore/repair.
- [ ] Manifest comparison does not read bodies; selected-file comparison enforces ownership, digest,
      text/binary distinction, and every input/compute/output bound without opening unrelated files.
- [ ] Each invariant has one primary focused positive/negative regression and adapter wiring coverage;
      changed-language checks and one final applicable source gate pass, with protected CI separate.
- [ ] A disposable signed-in Codex smoke proves the actual MCP discovery, consent, and approval path;
      only version, scenario, outcome, and safe identifiers are retained as evidence.

## Handoff and rollback

[implement.md](implement.md) is the single execution checklist; [design.md](design.md) records the
contracts that need more detail. R2a and R2b are acceptance units within R2, not new process stages or
mandatory task directories. Independent R2b preparation may continue while a provider gate is blocked.
R3 integrates only proven contracts. R2 product work requires an implementation instruction; there is
no additional Trellis activation or repeated approval ceremony.

Disable embedded private tools and history entry points to roll back the feature. Preserve external
handoff and manual product operations. Additive schema upgrades retain the normal future-schema
refusal for older binaries; binary downgrade alone is not Workspace rollback. Use the existing
backup/restore policy when an older binary needs pre-migration data.
