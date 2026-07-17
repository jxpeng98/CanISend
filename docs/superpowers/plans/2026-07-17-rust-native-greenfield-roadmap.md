# CanISend Rust-Native Greenfield Rebuild Roadmap

**Status:** In progress — R0 through R4 complete; R5 discovery ecosystem active

**Date:** 2026-07-17

**Delivery model:** Greenfield Rust rebuild in the existing repository

**Implementation branch:** `rewrite/rust-native`

**Archived Python baseline:** `archive/python-v0.6.0b1-final` at `18ab40815fa7976e2ce4ab9ef91e3f4826689af3`

**First planned release:** `canisend 0.7.0-alpha.1`

**New workspace format:** `canisend.workspace/v2`

**New agent protocol:** `canisend.agent/v2`

## Progress Log

- 2026-07-17: Approved the greenfield replacement model. Archived the final active Python implementation at
  `archive/python-v0.6.0b1-final` and created `rewrite/rust-native` from the same commit. R0 began with the roadmap as
  the execution authority.
- 2026-07-17: Accepted eight Rust-native ADRs covering the greenfield boundary, crate graph, SQLite/blob authority,
  generated contracts, agent protocol v2, embedded resources/rendering, privacy/consent, and native releases.
- 2026-07-17: Added a synthetic Lecturer in Economics v2 specification with source inputs, normalized criteria,
  evidence matches, document plan, Cover Letter candidate, review findings, validation error, and CLI transcript.
- 2026-07-17: The local native dependency spike passed bundled SQLite, schema generation/validation, embedded Typst,
  PDF generation/extraction, and Rustls HTTPS on macOS. Added a native Ubuntu/macOS/Windows spike matrix; R0 remains
  open until that matrix passes.
- 2026-07-17: Added the root Cargo workspace, six product crates, Rust `xtask`, protocol v2 envelope, truthful
  capability registry, embedded resource manifest, and native version/doctor/capabilities commands. Seven Rust tests,
  Clippy, schema/resource checks, and the first release build passed; Python deletion still awaits R0 native CI.
- 2026-07-17: GitHub Actions run `29608591519` passed the locked dependency spike on native Ubuntu, macOS, and
  Windows, including bundled SQLite, schema validation, embedded Typst/PDF extraction, and Rustls HTTPS. R0 exit
  criteria are satisfied; R1 repository cutover is authorized by the roadmap sequence.
- 2026-07-17: Replaced the active Python product, Pytest suite, legacy schemas/resources, and Python distribution
  workflows with the Rust workspace and Rust-only CI. GitHub Actions run `29609526692` passed the Python-file guard,
  formatting, Clippy, seven Rust tests, generated-contract/resource checks, release build, and packaged-binary smoke
  in 25 seconds with no annotations. R1 exit criteria are satisfied; R2 is active.
- 2026-07-17: Implemented validated v2 primitives and domain/task contracts, schema-first plus semantic candidate
  validation, 15 deterministic public schemas, and 21 typed embedded resources. Added body-free agent context,
  schema/resource diagnostics, stable error/exit mapping, and JSON snapshots. GitHub Actions run `29610852669`
  passed 19 Rust tests, Clippy, drift checks, release build, and packaged-binary smoke in 1 minute 59 seconds with no
  annotations. R2 exit criteria are satisfied; R3 is active.
- 2026-07-17: Established workspace v2 on bundled SQLite and immutable SHA-256 blobs, including migrations,
  transaction-bound audit events, exact artifact dependencies, recursive stale propagation, projection repair, and
  verified backup/restore. GitHub Actions run `29612319788` passed 27 Rust tests, Clippy, 18-schema/24-resource drift
  checks, release build, and packaged-binary workspace/backup/restore smoke in 1 minute 48 seconds. R3 exit criteria
  are satisfied; R4 is active.
- 2026-07-17: Completed transactional job lifecycle plus Markdown, text, public URL, HTML, and text-PDF intake. Added
  DNS-pinned Rustls requests, per-redirect SSRF checks, bounded MIME sniffing, structured HTML normalization,
  page-separated PDF extraction, and typed encrypted/malformed/image-only failures. GitHub Actions runs `29614087317`
  and `29614367500` passed 36 Rust tests, generated-contract checks, release builds, and packaged-binary job import
  smoke; the cached final run completed in 1 minute 10 seconds. R4 exit criteria are satisfied; R5 is active.

## 1. Executive Decision

CanISend will be rebuilt as a Rust-native product. This is not a Python-to-Rust compatibility migration and not a
line-by-line port.

The existing implementation is retained only in Git history as product research and behavioral reference. The new
implementation may reuse product ideas, wording, prompt content, templates, and test scenarios, but it does not have
to read old workspaces, reproduce old serialized bytes, preserve the old command tree, or run the old Pytest suite.

The finished product must satisfy these conditions:

1. End users install one platform-specific `canisend` executable and do not install Python.
2. Developers use Cargo and Rust-native test tools; Pytest and Python are absent from the active build and CI.
3. The binary accepts local links, supplied URLs, Markdown, text, JSON, CSV, and text-based PDF job adverts.
4. The binary exports agent instructions and a stable JSON protocol for Codex, Claude, IDE agents, and custom hosts.
5. CanISend remains local-first and prepares application materials; it does not submit applications.
6. Structured claims remain traceable to user evidence.
7. The initial supported workflow covers job intake, evidence, matching, planning, drafting, review, packaging, and
   PDF rendering.
8. All templates, schemas, prompts, and platform bridge assets required by the core workflow are embedded in the
   executable.
9. The normal workflow has no required Python, Node.js, Java, or separately installed Typst runtime.
10. Network access is explicit and limited to user-requested URL fetches, discovery sources, and configured model
    providers.

## 2. Fixed Decisions

These decisions are part of the rebuild definition and should not be reopened during implementation without a new
architecture decision record.

### 2.1 Language and runtime

- The product implementation is Rust.
- The repository uses a Cargo workspace.
- The stable Rust toolchain is pinned in `rust-toolchain.toml` and updated deliberately.
- The distributable product is a native executable named `canisend`.
- Rust tests replace Pytest completely.
- The active repository contains no required Python scripts, Python package metadata, or Python CI jobs.

### 2.2 Compatibility

- There is no automatic migration from a Python-era workspace.
- There is no promise of command-line compatibility with CanISend `0.6.x`.
- There is no promise of wire-format compatibility with `canisend.agent/v1`.
- There is no byte-for-byte output comparison with the Python implementation.
- The new implementation starts with workspace format v2 and agent protocol v2.
- Any useful old resource is copied only after it is reviewed against the new design.

### 2.3 Product boundary

- CanISend prepares materials but never submits an application.
- Account creation, portal automation, uploads, equality monitoring, right-to-work declarations, health data,
  criminal-record questions, and final submission remain outside the product.
- Job discovery is source-adapter based and user-invoked. There is no uncontrolled crawler.
- OCR is not required for the first alpha. Image-only PDFs return a structured `pdf_text_unavailable` result with
  remediation guidance.
- Configured LLM providers are optional. Host-agent execution through Codex, Claude, or another agent is a first-class
  path.

### 2.4 Storage

- SQLite is the authoritative metadata and workflow-state store.
- SQLite is linked into the application so users install no database service.
- Immutable content is stored in a SHA-256 content-addressed blob store.
- Markdown, Typst, JSON, and PDF files exposed in job folders are projections or exports, not the authoritative state
  database.
- Rust structs are the source of truth for machine contracts. JSON Schemas are generated from those structs.
- Machine state uses JSON. User configuration uses TOML. User-facing documents use Markdown, Typst, and PDF.

### 2.5 Agent integration

- Agents interact with CanISend through a versioned JSON CLI protocol.
- Agent hosts never write internal database rows, blob paths, task leases, or authoritative workflow state directly.
- `task prepare` declares the exact inputs, required consent, schema, and allowed output scope.
- `task complete` validates a candidate and commits it atomically in one operation.
- A failed candidate validation changes no authoritative state.
- Platform assets are exported from the binary; the installed binary does not depend on repository source files.
- A long-lived MCP or JSON-RPC server may be added later, but it is not required for the first usable release.

### 2.6 Rendering and distribution

- Typst source generation is native to the product.
- The release goal is embedded Typst PDF compilation using pinned Typst library crates.
- A known font subset is embedded for deterministic default output; system fonts may be opt-in additions.
- Releases are built independently for each supported OS and CPU architecture.
- GitHub Release archives and checksums are the first distribution channel.
- Homebrew and Scoop/WinGet are added after binary release validation.
- PyPI is not a distribution channel for the Rust-native product.

## 3. Goals and Non-Goals

### 3.1 Product goals

- A user can download a binary, initialize a workspace, import a job, and produce editable application materials.
- A user can give CanISend a URL or PDF without configuring an external API.
- A Codex or Claude session can discover CanISend capabilities and complete bounded workflow tasks.
- Every factual claim in generated material can reference a normalized evidence record.
- A changed source invalidates only the downstream artifacts that depend on it.
- A failed or interrupted command can be retried without corrupting the workspace.
- Private source bodies do not appear in normal logs or body-free agent responses.
- Installation and ordinary use do not depend on a development environment.

### 3.2 Engineering goals

- Clear crate boundaries and acyclic dependencies.
- One authoritative state model instead of coordinated mutable JSON/YAML files.
- Fast unit tests and focused integration tests.
- A normal pull-request gate below five minutes on Linux.
- Cross-platform release smoke tests use the packaged binary, not `cargo run`.
- Reproducible dependency locking and auditable release inputs.
- Structured error codes stable within agent protocol v2.

### 3.3 Non-goals for the first stable Rust release

- Reading or migrating Python-era workspaces.
- Reproducing every legacy command.
- Supporting every job board.
- Browser automation or authenticated scraping.
- OCR for scanned documents.
- A graphical desktop application.
- A cloud synchronization service.
- Background daemons.
- Dynamic Python plugins.
- Automatic application submission.
- Byte-identical PDFs across all operating systems when users opt into system fonts.

## 4. Product Scope

The first stable Rust release contains the following product slices.

### 4.1 Workspace

- Initialize and inspect a local workspace.
- Store user configuration and provider references.
- Create, archive, list, and inspect jobs.
- Preserve immutable audit events and artifact revisions.
- Export and repair user-visible projections.

### 4.2 Job intake

- Import `.md` and `.txt` files.
- Import text-based `.pdf` files.
- Fetch one user-supplied `https://` or `http://` URL.
- Detect HTML and PDF responses by validated MIME type and bounded content sniffing.
- Retain original source bytes and normalized text as distinct artifacts.
- Record source URL, retrieval time, content hash, and redirect chain.

### 4.3 Discovery

- Import local CSV and JSON lead batches.
- Import normalized host-agent search results.
- Read explicitly configured RSS/Atom feeds.
- Support jobs.ac.uk, Greenhouse, and Lever through compiled source adapters.
- Normalize and deduplicate leads without deleting source records.
- Promote a selected lead into a job workspace.

### 4.4 Profile evidence

- Import evidence from Markdown, JSON, plain text, and selected local documents.
- Normalize evidence into typed records.
- Assign stable IDs inside the new workspace.
- Track source hashes and evidence revisions.
- Allow user confirmation, correction, exclusion, and sensitivity labels.

### 4.5 Application workflow

- Parse job criteria.
- Confirm or correct criteria.
- Match criteria to evidence.
- Record application decision and strategy.
- Build a required-document plan.
- Draft supported materials.
- Review evidence use, missing claims, consistency, and placeholders.
- Record user dispositions for findings.
- Package editable sources and render PDFs.
- Derive readiness without claiming that an application was submitted.

### 4.6 Agent collaboration

- Export Codex, Claude, and generic agent assets.
- Return machine-readable capability and context responses.
- Prepare bounded tasks with declared privacy requirements.
- Accept candidates through stdin or a regular local file.
- Validate and commit candidates atomically.
- Return body-free artifact references, IDs, hashes, status, and next actions.

## 5. Target Architecture

```text
┌──────────────────────────────────────────────────────────────────────┐
│ User / Codex / Claude / IDE / custom host                           │
└──────────────────────────────┬───────────────────────────────────────┘
                               │ CLI + canisend.agent/v2 JSON
┌──────────────────────────────▼───────────────────────────────────────┐
│ canisend-cli                                                        │
│ command parsing · output mode · exit policy · stdin/stdout safety   │
└──────────────────────────────┬───────────────────────────────────────┘
                               │ application services
┌──────────────────────────────▼───────────────────────────────────────┐
│ canisend-core                                                       │
│ domain model · workflow graph · task service · artifact policies    │
└───────────────┬──────────────────────┬───────────────────────────────┘
                │                      │
┌───────────────▼────────────┐  ┌──────▼──────────────────────────────┐
│ canisend-store            │  │ canisend-io                         │
│ SQLite · blobs · events   │  │ URL · HTML · PDF · feeds · provider │
│ revisions · transactions  │  │ discovery adapters · render         │
└───────────────┬────────────┘  └──────┬──────────────────────────────┘
                │                      │
┌───────────────▼──────────────────────▼──────────────────────────────┐
│ canisend-contracts + canisend-resources                            │
│ typed contracts · schema export · templates · prompts · bridges    │
└──────────────────────────────────────────────────────────────────────┘
```

### 5.1 Dependency direction

The dependency graph must remain acyclic:

```text
canisend-contracts
      ▲
      ├── canisend-store
      ├── canisend-resources
      └── canisend-core
               ▲
               ├── canisend-io
               └── canisend-cli
```

If an integration needs to call into core behavior, the interface belongs in `canisend-core` and the implementation
belongs in the outer crate. `canisend-core` must not depend on `clap`, terminal presentation, a particular HTTP
client, or a particular model provider.

## 6. Repository Layout

```text
CanISend/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── rustfmt.toml
├── clippy.toml
├── deny.toml
├── README.md
├── LICENSE
├── SECURITY.md
├── CONTRIBUTING.md
├── crates/
│   ├── canisend-cli/
│   │   ├── Cargo.toml
│   │   ├── src/main.rs
│   │   ├── src/commands/
│   │   └── tests/
│   ├── canisend-contracts/
│   │   ├── src/agent/
│   │   ├── src/artifact/
│   │   ├── src/job/
│   │   ├── src/workflow/
│   │   └── tests/
│   ├── canisend-core/
│   │   ├── src/services/
│   │   ├── src/stages/
│   │   ├── src/policies/
│   │   └── tests/
│   ├── canisend-store/
│   │   ├── migrations/
│   │   ├── src/sqlite/
│   │   ├── src/blob/
│   │   └── tests/
│   ├── canisend-io/
│   │   ├── src/intake/
│   │   ├── src/discovery/
│   │   ├── src/provider/
│   │   ├── src/render/
│   │   └── tests/
│   └── canisend-resources/
│       ├── build.rs
│       ├── src/lib.rs
│       └── resources/
│           ├── agent/
│           ├── prompts/
│           ├── schemas/
│           ├── templates/
│           └── examples/
├── fixtures/
│   ├── adverts/
│   ├── discovery/
│   ├── pdf/
│   ├── providers/
│   └── workflows/
├── fuzz/
├── xtask/
├── docs/
│   ├── architecture/
│   ├── contracts/
│   ├── decisions/
│   ├── guides/
│   └── superpowers/plans/
└── .github/workflows/
```

`xtask` is optional but recommended for repository-owned automation such as schema generation, resource manifests,
release checks, and fixture verification. It must be Rust, not a replacement Python script directory.

## 7. Crate Responsibilities

### 7.1 `canisend-contracts`

Owns pure types and versioned external contracts:

- Strong IDs for workspace, job, source, evidence, artifact, task, run, and audit event.
- Agent request and response envelopes.
- Job lead and normalized job advert types.
- Evidence and criterion types.
- Workflow stage names and statuses.
- Task descriptor and task candidate types.
- Artifact metadata and dependency receipts.
- Error code taxonomy.
- JSON Schema generation.

This crate should have minimal dependencies and no filesystem, database, HTTP, or CLI behavior.

### 7.2 `canisend-core`

Owns product behavior:

- Workspace and job application services.
- Workflow graph and invalidation rules.
- Stage input/output contracts.
- Task preparation and completion policy.
- Consent requirements.
- Evidence-to-claim validation.
- Package readiness rules.
- Port traits for storage, network intake, providers, clock, and rendering.

Core tests use in-memory or deterministic fake ports and should account for most of the test count.

### 7.3 `canisend-store`

Owns durable local state:

- SQLite connection setup and migrations.
- Transaction boundaries.
- Content-addressed blob store.
- Artifact revision storage.
- Audit event append.
- Task lease and cancellation state.
- Projection manifests.
- Workspace lock and concurrent writer policy.
- Backup, integrity check, and recovery commands.

### 7.4 `canisend-io`

Owns external formats and side effects:

- Bounded HTTP fetch.
- Redirect and address validation.
- HTML normalization.
- PDF text extraction.
- CSV, JSON, RSS, and Atom parsing.
- Discovery source adapters.
- Configured LLM provider calls.
- Embedded Typst rendering.
- System font discovery when explicitly enabled.

### 7.5 `canisend-resources`

Owns build-time resources:

- Prompts.
- Typst templates.
- JSON Schemas generated from Rust types.
- Codex, Claude, and generic agent instructions.
- Example workspace inputs.
- A generated resource manifest containing path, version, and SHA-256.

Every resource is accessible through a typed resource API. Core code must not construct resource paths with arbitrary
strings.

### 7.6 `canisend-cli`

Owns user and host interaction:

- `clap` command tree.
- Human and JSON output modes.
- Stable process exit policy.
- Stdin candidate handling.
- Secret-safe logging setup.
- Conversion between application results and agent response envelopes.
- Shell completion generation.

## 8. Authoritative Data Architecture

### 8.1 Workspace layout

```text
workspace/
├── canisend.toml
├── .canisend/
│   ├── state.sqlite3
│   ├── blobs/
│   │   └── sha256/<prefix>/<digest>
│   ├── locks/
│   ├── tmp/
│   └── backups/
├── profile/
│   ├── sources/
│   └── exports/
├── jobs/
│   └── <job-slug>/
│       ├── source/
│       ├── workspace/
│       ├── exports/
│       └── pdf/
└── agent/
    ├── codex/
    ├── claude/
    └── generic/
```

`.canisend/` is internal and must not be edited by agents or users. User-editable inputs live outside it. Exported
files include a projection manifest so the core can detect an edit without treating the projection as authoritative.

### 8.2 SQLite tables

The initial migration should define at least:

- `workspace_metadata`
- `jobs`
- `sources`
- `source_revisions`
- `evidence_items`
- `evidence_revisions`
- `artifacts`
- `artifact_revisions`
- `artifact_dependencies`
- `workflow_runs`
- `stage_executions`
- `tasks`
- `task_inputs`
- `task_results`
- `consents`
- `audit_events`
- `projection_manifests`
- `discovery_sources`
- `job_leads`
- `provider_invocations`

Private bodies should be blobs referenced by hash rather than repeated in event rows. Audit rows contain IDs, hashes,
status, timestamps, actor kind, and reason codes.

### 8.3 Blob invariants

- A blob path is derived only from a validated lowercase SHA-256 digest.
- Bytes are written to a workspace-local temporary file, flushed, renamed, and verified before the database references
  the digest.
- Existing blobs are immutable.
- A hash collision or size mismatch fails closed.
- Garbage collection is explicit and never runs as part of an ordinary command.
- The first release may retain unreferenced blobs rather than risk deleting evidence.

### 8.4 Transaction model

- One command opens one application-level unit of work.
- SQLite transactions protect state transitions and audit append.
- Task completion validates the candidate before beginning the commit transaction whenever possible.
- A committed state transition and its audit event occur in the same transaction.
- Blob publication precedes the transaction; an interrupted unused blob is safe and may be collected later.
- Projection generation occurs after the authoritative commit and records success or repair-required state.
- A projection failure never rolls back already committed authoritative workflow data.

### 8.5 Versioning

- Workspace version and database migration number are distinct.
- Agent protocol uses a semantic protocol identifier.
- Each externally accepted candidate declares its schema ID and version.
- Artifact revisions are monotonic per artifact identity.
- Timestamps are UTC RFC 3339.
- IDs use UUIDv7 or another selected time-sortable random identifier.
- Integrity digests use SHA-256.
- Canonical signed/hashed JSON follows one documented canonicalization algorithm.

## 9. New Agent Protocol v2

### 9.1 Response envelope

Every JSON-mode command returns exactly one JSON object on stdout:

```json
{
  "protocol": "canisend.agent/v2",
  "operation": "task.prepare",
  "ok": true,
  "status": "prepared",
  "data": {},
  "artifacts": [],
  "required_consents": [],
  "warnings": [],
  "next_actions": [],
  "error": null
}
```

Logs and progress messages go to stderr. JSON stdout must never be mixed with human text.

### 9.2 Error envelope

Errors contain:

- Stable `code`.
- Human-readable `message`.
- Optional safe `details`.
- `retryable` boolean.
- Optional remediation action.
- No private source body.

Process exits are grouped rather than assigned uniquely to every error:

- `0`: operation completed.
- `2`: invalid CLI usage.
- `3`: validation or workflow blocker.
- `4`: workspace or state conflict.
- `5`: external I/O or provider failure.
- `6`: internal invariant failure.

### 9.3 Task lifecycle

```text
prepared ──complete(valid)──> committed
    │
    ├──complete(invalid)───> prepared + validation report
    ├──cancel──────────────> cancelled
    └──input changed───────> stale
```

The new task API intentionally removes the old separate submit/apply compatibility boundary.

`task prepare`:

- Resolves current stage inputs.
- Creates a task record and short-lived lease.
- Returns safe artifact references and input hashes.
- Returns the candidate JSON Schema.
- Declares required consent and private-read scope.

`task complete`:

- Accepts candidate JSON from stdin or a regular file.
- Rechecks task status and input hashes.
- Validates schema and semantic invariants.
- Stores candidate bytes as an immutable blob.
- Commits the stage result, dependencies, audit event, and invalidation in one transaction.
- Returns new artifact references and next actions.

### 9.4 Host assets

The binary exports host-specific assets:

```text
canisend agent assets export --host codex --workspace .
canisend agent assets export --host claude --workspace .
canisend agent assets export --host generic --workspace .
```

Each pack explains:

- How to call `agent capabilities` and `agent context`.
- How to obtain consent before reading private task inputs.
- How to produce a schema-valid candidate.
- Which files an agent may edit.
- Why internal `.canisend/` state must not be edited.
- How to handle validation errors and stale tasks.
- That readiness is not submission evidence.

## 10. Proposed CLI Surface

The CLI is redesigned around nouns and explicit operations.

```text
canisend version
canisend doctor

canisend workspace init
canisend workspace status
canisend workspace check
canisend workspace backup

canisend profile source add
canisend profile evidence import
canisend profile evidence list
canisend profile evidence confirm

canisend discovery source add
canisend discovery refresh
canisend discovery import
canisend discovery list
canisend discovery promote

canisend job create
canisend job import --file <path>
canisend job import --url <url>
canisend job list
canisend job show
canisend job archive

canisend workflow status
canisend workflow run
canisend workflow invalidate

canisend task prepare
canisend task complete
canisend task cancel
canisend task show

canisend artifact list
canisend artifact show
canisend artifact export
canisend artifact reconcile

canisend package check
canisend package export
canisend render

canisend agent capabilities
canisend agent context
canisend agent assets export
```

Every relevant command supports `--json`. Commands intended for agent use make JSON the default when stdout is not a
terminal.

## 11. Workflow Model

### 11.1 Stage graph

```text
intake
  └── parse
        └── criteria-confirm
              ├── evidence
              │     └── match
              └──────────┘
                     └── plan
                           ├── draft:cover-letter
                           ├── draft:research-statement
                           ├── draft:teaching-statement
                           └── draft:cv-notes
                                  └── review:<document>
                                         └── package-review
                                                └── package
                                                       └── render
```

The graph is data-driven. A stage declares:

- Input artifact kinds.
- Output artifact kinds.
- Whether it is deterministic, host-agent, configured-provider, or user-decision work.
- Candidate schema.
- Required consent scopes.
- Invalidation propagation.
- Projection outputs.

### 11.2 Artifact freshness

Every artifact revision records exact upstream artifact revision IDs and hashes. An artifact is current only when:

- Its own blob exists and matches its digest.
- Every declared dependency revision still exists.
- Current upstream heads equal the recorded dependency revisions.
- The producing stage implementation version is accepted by policy.
- Required user decisions remain current.

The engine marks descendants stale by graph query after an upstream revision changes. It does not recursively rerun
work without a user or agent command.

### 11.3 Stage execution modes

- `deterministic`: pure Rust transformation.
- `host-agent`: Codex, Claude, or another host proposes the candidate.
- `configured-provider`: the binary calls a configured remote provider after explicit consent.
- `user-decision`: the user selects or edits a bounded value.
- `manual-import`: the user supplies an externally prepared artifact.

Each mode ultimately commits through the same stage validator and artifact service.

## 12. Intake and Discovery Design

### 12.1 URL security policy

- Accept only `http` and `https`.
- Reject embedded credentials.
- Normalize and validate the URL before the first request.
- Resolve and reject loopback, link-local, multicast, unspecified, and private addresses unless an explicit future
  local-network mode is designed.
- Apply the same validation to every redirect.
- Limit redirect count.
- Limit response headers, body size, and total time.
- Stream bytes into a bounded temporary file instead of unbounded memory.
- Do not execute JavaScript.
- Do not reuse browser cookies or authenticated sessions.
- Record the final URL and safe redirect metadata.

### 12.2 PDF policy

- The first implementation supports PDFs with extractable text.
- Original bytes are retained as a source artifact.
- Extracted text is a separate derived artifact with extractor version metadata.
- Page count, byte size, and decoded text size have hard limits.
- Encrypted, malformed, image-only, or extraction-empty files fail with specific error codes.
- OCR is a later optional feature and must not silently run or transmit documents.

Before selecting the PDF crate, build a corpus containing:

- Single-column adverts.
- Two-column adverts.
- Tables and bullet lists.
- Embedded subset fonts.
- University-branded PDFs.
- Malformed cross-reference tables.
- Password-protected PDFs.
- Scanned image-only PDFs.
- Very large and decompression-heavy PDFs.

### 12.3 Discovery adapter interface

Each adapter implements a bounded interface similar to:

```rust
trait DiscoveryAdapter {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> AdapterCapabilities;
    async fn refresh(&self, request: RefreshRequest) -> Result<LeadBatch, DiscoveryError>;
}
```

Adapters return normalized data and source metadata. They never write SQLite or workspace files directly.

## 13. Rendering Design

### 13.1 Rendering pipeline

```text
structured document artifact
        ↓
template projection
        ↓
Typst source artifact
        ↓
embedded Typst compiler
        ↓
PDF blob + exported PDF
```

### 13.2 Embedded resources

- Templates are embedded and versioned.
- A default font family with redistribution-compatible licensing is embedded.
- Template packages required by the default output are vendored or embedded; rendering must not download packages at
  runtime.
- User templates may be added from the workspace but are treated as untrusted inputs with bounded reads.
- The compiler world is restricted to declared resources and the job's render inputs.

### 13.3 Rendering acceptance

- Rendering succeeds without a separately installed `typst` command.
- Rendering succeeds on a clean supported machine with no network.
- Default output contains no unresolved placeholders.
- PDF files open in at least two independent PDF readers in release smoke testing.
- Font licensing and bundled resource notices are included in the release archive.
- A renderer panic is converted to an internal error and does not corrupt authoritative state.

## 14. Rust Dependency Strategy

Dependency choices must be pinned only after a small spike proves the required behavior. The likely categories are:

- CLI: `clap`.
- Serialization: `serde`, `serde_json`.
- Schema generation and external candidate validation: `schemars` plus a Draft 2020-12 validator.
- Async runtime: `tokio`.
- HTTP and TLS: `reqwest` with Rustls.
- URL parsing: `url`.
- HTML parsing: a maintained HTML5 parser and selector library.
- Feed parsing: a maintained RSS/Atom parser.
- SQLite: `rusqlite` with bundled SQLite or an equivalent proven embedded option.
- Hashing: `sha2`.
- IDs: `uuid` with v7 support.
- Time: `time`.
- Errors: `thiserror` internally and `miette` or an equivalent presentation layer in the CLI.
- Logging: `tracing`.
- Temporary files: `tempfile`.
- Testing: `assert_cmd`, `predicates`, `insta`, `proptest`, HTTP test server tooling, and fault-injection helpers.
- Rendering: pinned official Typst compiler libraries.

Selection rules:

- Prefer widely used, maintained crates with clear licenses.
- Avoid native dynamic-library requirements in the end-user binary.
- Disable default features that add unused protocols or platform dependencies.
- Record allowed licenses in `deny.toml`.
- Commit `Cargo.lock`.
- Run dependency advisory and license checks in CI.
- New dependencies require an explicit reason in review.
- Provider SDK crates are avoided if a small, bounded HTTP implementation is safer and easier to audit.

## 15. Rust-Native Testing Architecture

Pytest is removed. No Python test runner remains.

### 15.1 Test layers

#### Layer A: unit tests

- Live next to the Rust module they test.
- Cover domain validation, graph logic, IDs, canonicalization, readiness, and error mapping.
- Use fake clock, fake IDs, and in-memory ports.
- Must be deterministic and network-free.
- Target runtime: under 30 seconds for the full workspace unit layer.

#### Layer B: crate integration tests

- Live in each crate's `tests/` directory.
- Cover SQLite transactions, blob publication, CLI JSON envelopes, URL policy, parsers, and rendering adapters.
- Use unique temporary workspaces.
- Never share a mutable fixture directory.
- Target runtime: under two minutes on the primary CI runner.

#### Layer C: snapshot and contract tests

- Snapshot new v2 CLI JSON, generated schemas, help output, error codes, and selected document structures.
- Snapshots are contracts for the Rust product, not comparisons with Python.
- Snapshot review is explicit; CI never automatically accepts changes.
- Private source bodies are replaced with synthetic fixtures.

#### Layer D: property and state-machine tests

- Artifact invalidation is correct for generated acyclic graphs.
- Repeated task completion is idempotent or returns the declared conflict.
- Canonical JSON produces stable hashes for semantically equivalent maps where promised.
- Blob path construction cannot escape the workspace.
- Arbitrary invalid URLs never bypass address policy.
- Database transitions preserve invariants under generated command sequences.

#### Layer E: adversarial and fault-injection tests

- Process interruption after blob write and before SQLite commit.
- Process interruption after state commit and before projection export.
- Full disk, permission denial, and read-only workspace.
- Concurrent task completion.
- Stale task after source revision.
- Corrupt SQLite file, missing blob, mismatched blob digest, and edited projection.
- Symlink, hard-link, and path traversal attacks.
- Redirect to private address and decompression-heavy inputs.

#### Layer F: binary end-to-end tests

- Run the compiled binary through `assert_cmd` or equivalent.
- Initialize a clean workspace.
- Import local text, URL-served HTML/PDF, and discovery fixtures.
- Complete a host-agent task through stdin.
- Produce package exports and a PDF.
- Restart the process between meaningful workflow steps.
- Verify only public outputs and database invariants through supported commands.

#### Layer G: fuzzing

Fuzz targets include:

- Agent JSON request and candidate parsing.
- URL normalization.
- HTML-to-text normalization.
- PDF parser boundary wrapper.
- CSV and discovery batch parsing.
- Template variable projection.
- Canonical JSON and artifact manifest parsing.

Fuzzing runs on a schedule, not on every pull request.

### 15.2 Test commands

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --lib
cargo test --workspace --tests
cargo test --workspace --doc
cargo run -p xtask -- schemas check
cargo run -p xtask -- resources check
cargo run -p xtask -- release check
```

`cargo-nextest` may be adopted for CI scheduling and reports, but every test must remain runnable through standard
`cargo test` unless a documented exception is approved.

### 15.3 Performance budgets

- `canisend version`: under 100 ms warm on the reference machine.
- `canisend agent capabilities --json`: under 150 ms warm.
- Workspace open and status for 100 jobs: under 500 ms.
- Import a 1 MB HTML advert: under two seconds excluding network time.
- Import a 50-page text PDF: benchmarked with an explicit budget after the PDF spike.
- Normal PR CI: under five minutes.
- Full Linux test suite: under ten minutes.
- Cross-platform release workflow: under 30 minutes excluding signing queue time.

Performance budgets are regression gates after a stable baseline exists. They must not encourage removal of security
checks.

## 16. Security and Privacy Architecture

### 16.1 Data classifications

- `public`: product capabilities, version, adapter names, schema IDs.
- `private-local`: job advert bodies, profile evidence, drafts, reviews, provider responses.
- `provider-bound`: the exact subset explicitly approved for transmission.
- `secret`: API tokens and credentials.

### 16.2 Rules

- Secrets are read from environment variables or an approved OS credential integration; they are not stored in
  workspace TOML or SQLite.
- Normal logs never contain private bodies.
- Provider calls require explicit per-operation consent unless a future persistent policy is deliberately designed.
- The task descriptor declares the exact provider-bound artifacts.
- Crash reports and telemetry are off by default.
- Workspace internal files receive private permissions where the platform supports them.
- Exported documents are considered user-managed after export.
- `workspace check` reports insecure permissions and unexpected internal symlinks.
- Network requests have timeouts, byte limits, redirect limits, and safe user-agent identification.
- Release artifacts include provenance, checksums, and dependency/license evidence.

### 16.3 Threat model topics

- Malicious job URL attempts SSRF.
- Malformed PDF exploits parser behavior or resource exhaustion.
- Prompt injection in job adverts attempts to control a host agent.
- Candidate JSON attempts path escape or undeclared artifact writes.
- A second process races task completion.
- Edited projection is mistaken for an authoritative reviewed artifact.
- Provider output invents evidence IDs.
- A compromised release archive replaces embedded skills or templates.

Each topic requires at least one control and one automated test before beta.

## 17. CI Architecture

### 17.1 Pull request gate

Runs on one current Ubuntu image:

1. Formatting.
2. Clippy with warnings denied.
3. Unit tests.
4. Integration tests excluding scheduled fuzz and extended rendering corpus.
5. Schema and embedded-resource drift checks.
6. Dependency license and advisory checks.
7. One binary workflow smoke.

### 17.2 Main branch gate

Adds:

- Full Linux suite.
- All feature combinations that are supported.
- Release-profile build.
- Extended PDF and rendering corpus.
- Database migration and recovery scenarios.
- Benchmark comparison with allowed thresholds.

### 17.3 Scheduled gate

Adds:

- Fuzzing time budget.
- Dependency freshness report.
- Sanitizer runs where supported.
- Large-corpus PDF and HTML tests.
- Long concurrency and interruption tests.

### 17.4 Release gate

Builds and tests packaged binaries on:

- macOS ARM64.
- macOS x86_64 or a validated universal artifact.
- Linux x86_64 GNU.
- Linux x86_64 musl.
- Windows x86_64 MSVC.

Linux ARM64 is added when a native or reliable emulated runner is available.

Each target must:

- Run `version`, `doctor`, `workspace init`, and `agent capabilities`.
- Import a local advert.
- Execute a synthetic host-agent workflow.
- Render a PDF without external Typst.
- Export Codex/Claude assets.
- Prove the packaged executable has no Python runtime dependency.
- Produce archive checksums and manifest metadata.

## 18. Release and Installation Design

### 18.1 Artifact naming

```text
canisend-v0.7.0-alpha.1-aarch64-apple-darwin.tar.gz
canisend-v0.7.0-alpha.1-x86_64-apple-darwin.tar.gz
canisend-v0.7.0-alpha.1-x86_64-unknown-linux-gnu.tar.gz
canisend-v0.7.0-alpha.1-x86_64-unknown-linux-musl.tar.gz
canisend-v0.7.0-alpha.1-x86_64-pc-windows-msvc.zip
```

### 18.2 Release stages

- `alpha`: architecture and workspace may change; aimed at project dogfooding.
- `beta`: command and protocol v2 changes require release notes and migration inside the Rust era.
- `rc`: feature freeze; only blockers and release evidence changes.
- `stable`: supported installation channels and documented workspace backup policy.

### 18.3 Supply-chain requirements

- Reproducible source tag.
- Locked dependencies.
- Build provenance.
- SHA-256 checksums.
- SBOM.
- macOS code signing and notarization before stable.
- Windows code signing before stable if certificates are available.
- Signed release manifest.
- License notices for embedded fonts, SQLite, Typst, and other redistributed assets.

## 19. Roadmap Overview

The estimates below are planning ranges for one experienced engineer working primarily on this project. They are not
release promises.

| Phase | Outcome | Estimate | Depends on |
|---|---|---:|---|
| R0 | Architecture decisions and executable specifications | 3–5 days | roadmap approval |
| R1 | Rust-only repository and CI foundation | 4–7 days | R0 |
| R2 | Contracts, CLI envelope, resources | 1–2 weeks | R1 |
| R3 | SQLite, blob store, transactions, workspace | 2–3 weeks | R2 |
| R4 | File, URL, HTML, PDF intake | 2–3 weeks | R3 |
| R5 | Discovery ecosystem | 2 weeks | R4 |
| R6 | Agent protocol and host assets | 2–3 weeks | R2, R3 |
| R7 | Workflow kernel and evidence/match/plan | 3–4 weeks | R3, R4, R6 |
| R8 | Draft, review, package | 3–4 weeks | R7 |
| R9 | Embedded Typst rendering | 2–3 weeks | R3, R8 |
| R10 | Security, recovery, performance hardening | 2–3 weeks | R4–R9 |
| R11 | Cross-platform alpha/beta release | 2–3 weeks | R10 |

Expected total: approximately 21–33 engineer-weeks. Scope reduction can shorten this; keeping PDF extraction,
discovery, agent integration, evidence-backed drafting, embedded rendering, and five release targets makes a much
shorter estimate unrealistic.

## 20. Detailed Execution Plan

### Phase R0 — Architecture and executable specifications

**Objective:** Convert this roadmap into fixed implementation decisions and new Rust-native behavioral examples.

#### R0.1 Archive the Python era

- [x] Confirm the current repository has no uncommitted user changes before the roadmap document was created.
- [x] Record the final Python-era commit in the annotated `archive/python-v0.6.0b1-final` tag.
- [x] Record the old release and documentation location in `docs/history/python-era.md`.
- [x] State clearly that old workspaces are unsupported by the Rust-native binary.
- [x] Decide that the Pytest suite will not be copied into the Rust implementation.

#### R0.2 Write architecture decision records

- [x] ADR-RN-0001: greenfield Rust rebuild and no legacy compatibility.
- [x] ADR-RN-0002: Cargo workspace and crate boundaries.
- [x] ADR-RN-0003: SQLite plus content-addressed blobs.
- [x] ADR-RN-0004: Rust types as schema source of truth.
- [x] ADR-RN-0005: agent protocol v2 and atomic task completion.
- [x] ADR-RN-0006: embedded resources and Typst compiler.
- [x] ADR-RN-0007: privacy classifications and consent.
- [x] ADR-RN-0008: supported platform targets and release packaging.

#### R0.3 Create executable examples

- [x] Write one synthetic academic job advert fixture.
- [x] Write one synthetic profile evidence fixture.
- [x] Define expected normalized criteria and evidence records.
- [x] Define expected match and document plan structures.
- [x] Define one Cover Letter candidate and review result.
- [x] Define the expected agent error envelope for an invalid candidate.
- [x] Define the expected successful end-to-end CLI transcript.

#### R0.4 Dependency spikes

- [x] Compare candidate SQLite integration options and prove a bundled build on the local target.
- [x] Compare `pdf-extract` and direct `lopdf` extraction against the generated initial PDF.
- [x] Prove a minimal embedded Typst compilation with bundled fonts and no network.
- [x] Prove Rustls HTTPS on macOS, Linux, and Windows native GitHub runners in run `29608591519`.
- [x] Confirm selected JSON Schema tooling supports generated schemas and Draft 2020-12 runtime validation.

**Deliverables:** Eight ADRs, synthetic v2 fixtures, dependency spike notes, final crate graph. Complete.

**Exit criteria:** Satisfied. No unresolved decision blocks repository scaffolding, storage, agent tasks, PDF intake,
or rendering, and the dependency families passed their required native matrix.

### Phase R1 — Rust-only repository foundation

**Objective:** Replace the active Python project structure with a compiling, tested Rust workspace.

#### R1.1 Repository cutover

- [x] Create the dedicated `rewrite/rust-native` branch.
- [x] Remove `src/canisend/`, `tests/`, Python scripts, `pyproject.toml`, and Python lock files from the active branch.
- [x] Remove Python build, wheel, PyPI, TestPyPI, and Pytest workflows.
- [x] Preserve only deliberately reviewed product resources and documentation.
- [x] Add a CI check that rejects new required `.py` files and Python package metadata.

#### R1.2 Cargo workspace

- [x] Add root `Cargo.toml` with workspace dependency versions and release profiles.
- [x] Add `Cargo.lock`.
- [x] Add pinned `rust-toolchain.toml` with `rustfmt` and `clippy` components.
- [x] Scaffold all six crates and `xtask`.
- [x] Establish the accepted inward crate dependency direction in manifests.
- [x] Add repository metadata, license, authorship, and binary name.

#### R1.3 Engineering policy

- [x] Add `rustfmt.toml`.
- [x] Add Clippy policy and deny warnings in Rust CI.
- [x] Add dependency license/advisory configuration.
- [x] Add `CONTRIBUTING.md`, `SECURITY.md`, and Rust development commands.
- [x] Define unsafe-code policy; default to `#![forbid(unsafe_code)]` where dependencies and FFI boundaries allow.
- [x] Define minimum supported Rust version or explicitly use the pinned stable toolchain only for alpha.

#### R1.4 Minimal binary

- [x] Implement `canisend version`.
- [x] Implement a foundation `canisend doctor` with JSON output and resource verification.
- [x] Add build version, Git revision, target triple, Rust compiler, and resource version to version output.
- [x] Add CLI binary contract tests.
- [x] Add and verify release-profile compilation.

**Deliverables:** Rust-only compiling repository, minimal binary, fast CI. Complete.

**Exit criteria:** Satisfied by GitHub Actions run `29609526692`. A clean checkout ran the active-file guard,
formatting, Clippy, all tests, generated-contract/resource checks, `cargo build --release`, and packaged-binary smoke
without Python in 25 seconds.

### Phase R2 — Contracts, CLI envelope, and embedded resources

**Objective:** Establish the new domain language and agent-visible contract before implementing persistence.

#### R2.1 Strong types

- [x] Implement version, ID, digest, revision, timestamp, and artifact-kind types.
- [x] Reject unvalidated path strings at contract boundaries.
- [x] Define job, source, evidence, criterion, match, plan, document, finding, and readiness types.
- [x] Define actor and execution-mode types.
- [x] Define privacy classification and consent-scope types.

#### R2.2 Agent protocol v2

- [x] Implement response and error envelopes.
- [x] Define safe artifact references.
- [x] Define capabilities and context payloads.
- [x] Define task descriptor and completion request.
- [x] Define stable error code registry.
- [x] Implement JSON stdout and stderr separation.
- [x] Implement exit-code mapping.

#### R2.3 Schema generation

- [x] Generate schemas from Rust contract types.
- [x] Give each public schema an ID and semantic version.
- [x] Sort and format generated schema output deterministically.
- [x] Add a drift check through `xtask`.
- [x] Add runtime external-candidate validation.
- [x] Add semantic validation after structural validation.

#### R2.4 Resource system

- [x] Define typed resource IDs.
- [x] Embed initial schemas, prompts, templates, examples, and host assets.
- [x] Generate a resource SHA-256 manifest during build.
- [x] Add resource lookup and export APIs.
- [x] Reject missing, duplicate, or undeclared resources at build/test time.

#### R2.5 Initial CLI contract

- [x] Implement `agent capabilities --json`.
- [x] Implement schema and resource listing commands for development diagnostics.
- [x] Implement JSON snapshot tests.
- [x] Document protocol v2 without referring to protocol v1 compatibility.

**Deliverables:** Contract crate, generated schemas, embedded resource API, agent envelope. Complete.

**Exit criteria:** Satisfied by GitHub Actions run `29610852669`. Generated schema/resource checks and committed JSON
snapshots are deterministic; the packaged binary exposes every capability marked available and returns grouped,
stable JSON failures without mixing stderr into stdout.

### Phase R3 — Workspace, SQLite, blobs, and recovery

**Objective:** Build the authoritative local-first state system.

#### R3.1 Workspace bootstrap

- [x] Implement workspace discovery and explicit `--workspace` resolution.
- [x] Implement `workspace init` and `canisend.toml`.
- [x] Create internal directories with private permissions where supported.
- [x] Refuse unsafe internal symlinks and non-directory collisions.
- [x] Implement `workspace status` and `workspace check`.

#### R3.2 SQLite foundation

- [x] Add embedded SQLite and initial migrations.
- [x] Enable foreign keys and configure a documented journal mode and busy timeout.
- [x] Implement transaction helper and typed repositories.
- [x] Store migration state and workspace identity.
- [x] Add database integrity checks.
- [x] Test opening, concurrent readers, writer conflict, migration failure, and corrupt files.

#### R3.3 Blob store

- [x] Implement bounded streaming writes with SHA-256.
- [x] Implement atomic publication and post-write verification.
- [x] Implement immutable reads and digest verification.
- [x] Implement reference recording.
- [x] Implement unreferenced-blob audit without automatic deletion.
- [x] Add traversal, symlink, collision, interruption, and permission tests.

#### R3.4 Artifact and event service

- [x] Create artifact identities and monotonic revisions.
- [x] Record exact dependency edges.
- [x] Append audit events in the same transaction as state transitions.
- [x] Implement freshness queries and stale propagation.
- [x] Implement projection manifests.
- [x] Implement repair of derived projections.

#### R3.5 Backup and recovery

- [x] Implement consistent SQLite backup.
- [x] Include referenced blobs and configuration in backup manifests.
- [x] Verify a backup before declaring success.
- [x] Implement restore into a new empty directory.
- [x] Test interruption after blob publication, transaction commit, and projection failure.

**Deliverables:** Durable workspace, state store, blob store, audit events, backup/restore. Complete.

**Exit criteria:** Satisfied by GitHub Actions run `29612319788`. Fault-injection tests leave pre-transaction blobs
auditable without partial database state, projection failure preserves authoritative artifacts and records a repair
manifest, and SQLite concurrency tests prove concurrent readers plus bounded writer conflict behavior. The packaged
release binary completes workspace initialization, integrity check, verified backup, restore, and post-restore check.

### Phase R4 — Job intake

**Objective:** Import the job source formats required for a useful standalone binary.

#### R4.1 Local file intake

- [x] Implement Markdown and text import with encoding policy.
- [x] Implement safe regular-file reads with size limits.
- [x] Retain original bytes and normalized text separately.
- [x] Create source revisions and intake artifacts.
- [x] Add duplicate-content reuse without merging distinct source identities.

#### R4.2 HTTP intake

- [x] Implement bounded HTTP client and Rustls configuration.
- [x] Implement safe URL and redirect policy.
- [x] Implement timeouts, maximum sizes, and content type checks.
- [x] Implement HTML-to-text normalization.
- [x] Record final URL and retrieval metadata.
- [x] Add local test-server cases for redirect, timeout, truncation, misleading MIME, and private addresses.

#### R4.3 PDF intake

- [x] Select the PDF extraction implementation from R0 evidence.
- [x] Enforce page, byte, decode, and time budgets.
- [x] Extract page-separated normalized text.
- [x] Return typed results for encrypted, malformed, and image-only documents.
- [x] Preserve original PDF bytes.
- [x] Add the complete PDF fixture corpus and regression tests.

#### R4.4 Job commands

- [x] Implement `job create`.
- [x] Implement `job import --file`.
- [x] Implement `job import --url`.
- [x] Implement `job list`, `job show`, and `job archive`.
- [x] Implement human and JSON outputs.

**Deliverables:** Complete direct URL/file/PDF job intake. Complete.

**Exit criteria:** Satisfied by GitHub Actions runs `29614087317` and `29614367500`. A clean release binary creates a
job, imports a committed Markdown advert, reads it back through the public body-free contract, passes workspace
integrity, and backs up/restores the result without Python or an external service. Offline HTTP and PDF fixtures cover
the network/parser boundaries without making CI depend on a live website.

### Phase R5 — Discovery ecosystem

**Objective:** Expand job sources without coupling adapters to core storage.

#### R5.1 Discovery domain

- [ ] Define lead, source, batch, identity, freshness, and promotion contracts.
- [ ] Define adapter capabilities and refresh policy.
- [ ] Implement normalized organization, title, location, deadline, and URL fields.
- [ ] Preserve source-specific metadata under bounded typed extensions.

#### R5.2 Local imports

- [ ] Implement CSV mapping with explicit headers and diagnostics.
- [ ] Implement JSON batch schema.
- [ ] Implement normalized host-agent result import.
- [ ] Add dry-run and row-level error reporting.

#### R5.3 Network adapters

- [ ] Implement RSS/Atom.
- [ ] Implement jobs.ac.uk.
- [ ] Implement Greenhouse public boards.
- [ ] Implement Lever public boards.
- [ ] Add fixture-based adapter tests with no live-network dependency in CI.

#### R5.4 Identity and refresh

- [ ] Implement deterministic exact-key matching.
- [ ] Implement bounded fuzzy candidate suggestions without automatic destructive merges.
- [ ] Record refresh receipts and source cursors.
- [ ] Preserve removed/expired leads as history.
- [ ] Implement promotion from lead to job.

**Deliverables:** Local and network discovery with deduplication and promotion.

**Exit criteria:** Offline fixtures cover every adapter and a user can promote a discovered lead into direct intake.

### Phase R6 — Agent collaboration

**Objective:** Make the Rust binary useful from Codex, Claude, and generic agent hosts.

#### R6.1 Context and capabilities

- [ ] Implement `agent capabilities` from the compiled stage and adapter registry.
- [ ] Implement body-free `agent context` from workspace/job state.
- [ ] Include protocol, workspace, supported stages, blockers, and next actions.
- [ ] Prove normal responses contain no private source body.

#### R6.2 Task service

- [ ] Implement `task prepare` with exact input revision hashes.
- [ ] Implement task lease, expiry, cancellation, and stale detection.
- [ ] Implement candidate input from stdin and regular files.
- [ ] Reject symlinked or oversized candidate files.
- [ ] Implement structural and semantic validation.
- [ ] Implement atomic `task complete`.
- [ ] Make repeated valid completion idempotent where the task/result hash is identical.

#### R6.3 Host assets

- [ ] Write Codex skill/instruction assets for protocol v2.
- [ ] Write Claude assets for protocol v2.
- [ ] Write generic Markdown integration instructions.
- [ ] Embed and export assets.
- [ ] Include version and resource manifest in each exported pack.

#### R6.4 Host smoke tests

- [ ] Add a scripted host-agent transcript using only the packaged CLI protocol.
- [ ] Verify task candidate creation outside internal state.
- [ ] Verify validation failure remediation.
- [ ] Verify stale task behavior after a source edit.
- [ ] Verify no direct internal writes are required.

**Deliverables:** Agent protocol v2 implementation and self-contained host asset packs.

**Exit criteria:** A new Codex or Claude workspace can follow exported instructions and complete a synthetic task.

### Phase R7 — Workflow kernel, evidence, match, and plan

**Objective:** Deliver the evidence-backed decision spine.

#### R7.1 Stage registry and graph executor

- [ ] Implement stage descriptors and dependency graph validation.
- [ ] Reject cycles and duplicate outputs at startup/test time.
- [ ] Implement status, readiness, blockers, and next-action derivation.
- [ ] Implement deterministic, host-agent, provider, and user-decision modes.
- [ ] Implement scoped rerun and invalidation.

#### R7.2 Parse and criteria confirmation

- [ ] Define parsed-job and criterion contracts.
- [ ] Implement host-agent parse task.
- [ ] Implement optional configured-provider parse through the same validator.
- [ ] Implement user correction and confirmation commands.
- [ ] Track criterion source spans and confidence.

#### R7.3 Evidence

- [ ] Implement profile source import.
- [ ] Define evidence record types and sensitivity labels.
- [ ] Implement evidence normalization tasks.
- [ ] Implement user confirmation, correction, exclusion, and revision.
- [ ] Prevent an agent from inventing source identities or evidence IDs.

#### R7.4 Match

- [ ] Implement typed criterion-to-evidence match proposals.
- [ ] Record support strength, gaps, and prohibited claims.
- [ ] Require cited evidence revision IDs.
- [ ] Recompute stale matches after criterion or evidence change.

#### R7.5 Decision and document plan

- [ ] Implement apply/hold/skip decision.
- [ ] Implement application strategy fields.
- [ ] Implement required/optional/omitted document plan.
- [ ] Derive unresolved blockers.
- [ ] Expose body-free workflow context.

**Deliverables:** Resumable intake-to-plan workflow.

**Exit criteria:** A synthetic job and profile produce confirmed criteria, evidence matches, decision, and document plan.

### Phase R8 — Draft, review, and package

**Objective:** Produce evidence-backed, reviewable application materials.

#### R8.1 Structured document model

- [ ] Define document section, claim, citation, placeholder, and generation metadata types.
- [ ] Require every factual claim category to declare evidence or an allowed non-evidence classification.
- [ ] Define supported document kinds for the first alpha.
- [ ] Implement document-specific semantic validators.

#### R8.2 Draft stages

- [ ] Implement Cover Letter host-agent draft.
- [ ] Implement Research Statement host-agent draft.
- [ ] Implement Teaching Statement host-agent draft if included in alpha scope.
- [ ] Implement CV tailoring notes.
- [ ] Add configured-provider support through the same task validation boundary.
- [ ] Bound provider input and output sizes.

#### R8.3 Review stages

- [ ] Implement evidence citation validation.
- [ ] Implement placeholder and unsupported-claim detection.
- [ ] Implement cross-document consistency checks.
- [ ] Define human-review findings separately from deterministic blockers.
- [ ] Implement user dispositions and finding revision tracking.

#### R8.4 Package readiness

- [ ] Verify required documents exist and are current.
- [ ] Verify reviews and dispositions reference exact document revisions.
- [ ] Detect mixed evidence/profile revisions.
- [ ] Produce body-free readiness reasons.
- [ ] Preserve the rule that readiness is not submission.

#### R8.5 Exports

- [ ] Project structured drafts into Markdown.
- [ ] Export structured JSON for agent inspection.
- [ ] Record projection hashes and edit status.
- [ ] Implement reconcile/replace/copy-as-new behavior for edited projections.

**Deliverables:** Reviewable structured materials and a guarded package.

**Exit criteria:** The synthetic workflow produces current, cited, reviewed, editable material exports.

### Phase R9 — Embedded Typst and PDF rendering

**Objective:** Complete the standalone binary promise by eliminating an external Typst runtime requirement.

#### R9.1 Compiler spike integration

- [ ] Pin the proven Typst library versions.
- [ ] Implement the restricted compiler world.
- [ ] Embed licensed default fonts and templates.
- [ ] Disable runtime package downloads.
- [ ] Add memory and time bounds where the API permits.

#### R9.2 Typst projection

- [ ] Project each supported structured document into Typst data and source.
- [ ] Escape user text safely.
- [ ] Detect unresolved template fields.
- [ ] Preserve editable source exports separately from authoritative structured artifacts.

#### R9.3 PDF output

- [ ] Compile PDFs entirely inside the process.
- [ ] Store the PDF as an artifact blob.
- [ ] Export PDF files and manifests.
- [ ] Capture safe diagnostics without private source leakage in normal logs.
- [ ] Validate generated PDFs in tests.

#### R9.4 Cross-platform rendering

- [ ] Test default fonts on all release targets.
- [ ] Test Unicode, mathematical text, URLs, lists, and page breaks.
- [ ] Test missing user font behavior.
- [ ] Measure binary size and render time.
- [ ] Include licenses and notices.

**Deliverables:** Offline embedded rendering and PDF exports.

**Exit criteria:** Release binaries render the full synthetic package without Python, Node, Java, network, or Typst CLI.

### Phase R10 — Hardening and performance

**Objective:** Prove the product is safe and recoverable under realistic failure conditions.

#### R10.1 Security review

- [ ] Complete the threat model.
- [ ] Audit URL and redirect validation.
- [ ] Audit archive/resource/path handling.
- [ ] Audit private logging and provider payload construction.
- [ ] Run dependency advisory and license checks.
- [ ] Resolve all high-severity findings or document release blockers.

#### R10.2 Recovery review

- [ ] Execute every planned interruption point.
- [ ] Restore from backup on every primary platform.
- [ ] Rebuild projections from authoritative state.
- [ ] Detect corrupt/missing blobs.
- [ ] Test concurrent host-agent tasks and stale completion.

#### R10.3 Performance

- [ ] Establish command startup benchmarks.
- [ ] Benchmark status for large workspaces.
- [ ] Benchmark HTML and PDF intake.
- [ ] Benchmark full synthetic workflow.
- [ ] Benchmark Typst render and binary size.
- [ ] Add regression thresholds to main/release gates.

#### R10.4 UX and documentation

- [ ] Review human command output and remediation messages.
- [ ] Complete installation, quick-start, privacy, agent, backup, and troubleshooting guides.
- [ ] Document unsupported scanned PDFs.
- [ ] Document provider consent and data boundaries.
- [ ] Test documentation from a clean machine.

**Deliverables:** Threat model, recovery evidence, benchmark baseline, complete user documentation.

**Exit criteria:** No unresolved critical security, corruption, privacy, or installation blockers.

### Phase R11 — Alpha, beta, and stable release

**Objective:** Publish verified native binaries and graduate them through controlled release stages.

#### R11.1 Alpha

- [ ] Build all initial target archives.
- [ ] Run packaged-binary smokes.
- [ ] Publish checksums, SBOM, notices, and known limitations.
- [ ] Dogfood real job imports and application workflows.
- [ ] Collect issues without enabling default telemetry.

#### R11.2 Beta

- [ ] Resolve alpha data-loss, security, protocol, and rendering blockers.
- [ ] Freeze agent protocol v2 for the beta line.
- [ ] Freeze workspace v2 migrations inside the Rust era.
- [ ] Add Homebrew and Windows installation channel candidates.
- [ ] Complete macOS notarization and planned Windows signing.

#### R11.3 Release candidate

- [ ] Freeze features.
- [ ] Run the complete release matrix twice from clean tags.
- [ ] Verify upgrade between Rust beta/RC workspace migrations.
- [ ] Verify documentation and uninstall instructions.
- [ ] Produce release notes and rollback guidance.

#### R11.4 Stable

- [ ] Publish stable archives and package-manager manifests.
- [ ] Publish protocol, schema, and workspace support policy.
- [ ] Create the next roadmap from measured user feedback.

**Deliverables:** Signed, documented, platform-specific native releases.

**Exit criteria:** A supported user installs and completes the documented workflow without any development runtime.

## 21. Phase Dependency Graph

```text
R0
└── R1
    └── R2
        ├── R3
        │   ├── R4
        │   │   └── R5
        │   └── R6
        └────────┘
             └── R7
                 └── R8
                     └── R9
                         └── R10
                             └── R11
```

R5 may continue in parallel with R6 after R4 establishes the shared network and intake infrastructure. R9 may begin
its compiler integration spike earlier, but full rendering depends on the R8 structured document model.

## 22. Work Item and Pull Request Strategy

### 22.1 Work item sizing

- One work item should normally produce one externally observable behavior or one internal invariant.
- A work item must include tests and documentation needed for that behavior.
- Avoid phase-sized pull requests.
- Prefer vertical slices after the foundation exists.
- Database migrations and their repository code land together.
- Generated schemas and source contract changes land together.

### 22.2 Pull request rules

Every PR states:

- The invariant or user behavior introduced.
- Crates changed.
- New dependencies and why they are required.
- Tests run.
- Security/privacy impact.
- Workspace, schema, or agent protocol impact.
- Rollback behavior.

### 22.3 Suggested branch and release sequence

```text
archive/python-v0.6
rewrite/rust-native
release/0.7.0-alpha.1
release/0.7.0-beta.1
release/0.7.0-rc.1
main
```

The exact branching model may be simplified, but the archival Python tag must exist before the active files are
removed.

## 23. Definition of Done

The Rust-native rebuild is complete only when all conditions below are true.

### 23.1 Runtime independence

- [ ] Packaged binary runs on every supported clean OS image.
- [ ] No Python executable, Python library, virtual environment, or PyPI install is required.
- [ ] No external Typst command is required.
- [ ] No Node.js or Java runtime is required.
- [ ] SQLite and required resources are bundled appropriately.

### 23.2 Product workflow

- [ ] Initialize workspace.
- [ ] Import a local text advert.
- [ ] Import a text PDF advert.
- [ ] Import a supplied URL.
- [ ] Import and promote discovery leads.
- [ ] Build profile evidence.
- [ ] Parse and confirm criteria.
- [ ] Match evidence.
- [ ] Plan documents.
- [ ] Draft and review supported materials.
- [ ] Produce package readiness.
- [ ] Export Markdown, Typst, JSON, and PDF.

### 23.3 Agent workflow

- [ ] Export Codex assets.
- [ ] Export Claude assets.
- [ ] Discover capabilities and context.
- [ ] Prepare and complete a task through JSON/stdin.
- [ ] Reject invalid and stale candidates without mutation.
- [ ] Keep private bodies out of body-free responses.

### 23.4 Reliability

- [ ] Recovery tests cover every commit/projection interruption boundary.
- [ ] Backup and restore are verified.
- [ ] Concurrent writer behavior is documented and tested.
- [ ] Corrupt state is detected rather than silently accepted.
- [ ] Projections can be repaired from authoritative state.

### 23.5 Release evidence

- [ ] Formatting, Clippy, unit, integration, property, E2E, schema, resource, and release tests pass.
- [ ] Scheduled fuzz targets have no unresolved reproducible crash.
- [ ] Security and license checks pass.
- [ ] Cross-platform packaged-binary smokes pass.
- [ ] Checksums, SBOM, provenance, notices, and signatures are published.
- [ ] Installation and quick-start instructions are verified from clean machines.

## 24. Principal Risks and Mitigations

| Risk | Impact | Mitigation | Release gate |
|---|---|---|---|
| Embedded Typst integration is unstable | standalone PDF promise delayed | complete R0 compiler spike before workflow investment; pin versions | offline render smoke |
| Rust PDF extraction loses text | criteria quality degrades | corpus-based selection; retain source; explicit unsupported result; no silent OCR | PDF regression corpus |
| SQLite and blob commit diverge | orphan or missing content | immutable blobs first, transactional references, integrity audit, explicit GC | interruption tests |
| Binary becomes very large | install friction | measure per phase; strip/LTO carefully; audit embedded fonts/resources | size budget |
| Cross-platform filesystem differences | corruption or export failures | packaged native tests on macOS/Linux/Windows | release matrix |
| Agent prompt injection | unsafe reads/actions | task scope, explicit consent, candidate schema, no internal writes | adversarial host tests |
| Provider leaks private data | privacy breach | exact provider-bound manifest, consent, redacted logs | provider payload tests |
| Too many crates slow development | unnecessary abstraction | six initial crates only; split further only with demonstrated boundary | architecture review |
| Greenfield scope expands | release never converges | fixed alpha document types/adapters; defer OCR, GUI, portal automation | phase exit review |
| Removing Python too early loses reference | product behavior forgotten | archive tag and history docs before deletion; synthetic new specs | R0/R1 gate |
| Rust compile time grows | developer feedback slows | feature discipline, dependency review, cache CI, crate-level tests | PR time budget |
| No old workspace support surprises users | adoption friction | explicit breaking-release notes and separate workspace directory | release docs |

## 25. First Execution Iteration

The first implementation iteration should stop after a verified Rust-only skeleton. It should not begin job parsing,
PDF work, or workflow stages prematurely.

### Iteration objective

Produce a clean Rust workspace that builds `canisend`, exposes protocol v2 version/capabilities, embeds one test
resource, and is tested entirely through Cargo.

### Ordered tasks

1. Confirm and record the archival Python commit.
2. Add ADR-001 through ADR-003.
3. Create `rewrite/rust-native`.
4. Remove the active Python package, Pytest suite, Python build metadata, and Python CI.
5. Create the Cargo workspace and six crate manifests.
6. Pin the stable Rust toolchain.
7. Implement `canisend version` in human and JSON modes.
8. Implement the protocol v2 response/error envelope.
9. Implement `agent capabilities --json` with a minimal compiled registry.
10. Embed and verify one resource through `canisend-resources`.
11. Add unit, snapshot, and binary integration tests.
12. Add format, Clippy, Cargo test, resource, and release-build CI jobs.
13. Build one local release binary and verify it on a shell without Python activation.
14. Update README with the new development and installation contract.

### Iteration acceptance commands

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --release --locked
./target/release/canisend version --json
./target/release/canisend agent capabilities --json
```

### Iteration exit evidence

- Cargo commands pass from a clean checkout.
- No Pytest or Python job exists in active CI.
- `canisend` runs without an activated Python environment.
- JSON stdout contains exactly one protocol v2 envelope.
- The resource manifest is embedded and verified.
- The next implementation task is R3 storage foundation, not a return to the Python architecture.

## 26. Final Implementation Principle

The rebuild should preserve the product's purpose, not its accidental implementation history.

Rust is not only a packaging replacement here. The new architecture should use the rebuild to establish one
authoritative state store, immutable content, typed contracts, bounded agent tasks, embedded rendering, and a
Rust-native test/release system. Whenever an old Python mechanism conflicts with those goals, the new Rust design wins.
