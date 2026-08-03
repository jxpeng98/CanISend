# M1 shared approval broker implementation record

Date: 2026-08-03

## Outcome

M1-APPR-001 and the source implementation portion of M1-TEST-001 are complete. MCP and the
desktop job-intake, discovery, workflow-rerun, and task-completion preview families now use the
same app-owned approval broker. The previous adapter-owned timestamp/counter stores were removed.

This is source evidence for the M1A approval gate. It is not an Alpha.6 qualification event and
does not close M1-OP-003 semantic parity, MSRV/CI, clean-candidate, or native packaged-binary gates.

## Broker contract

The shared broker in `canisend-app` enforces one policy for every migrated adapter:

- a fixed initial ten-minute lifetime measured by an injectable monotonic clock;
- client-visible Unix expiry metadata derived separately from wall time;
- 256-bit operating-system CSPRNG tokens with collision rejection and no replacement;
- exact operation-kind, canonical Workspace identity/path, Pack id/version/digest, Application,
  and source revision or snapshot bindings;
- atomic single-use `take`, so at most one concurrent commit receives a grant;
- default consumption for expiry, replay, wrong kind, wrong Workspace/Pack, stale state,
  validation failure, and permanent IO failure;
- `ApprovalDisposition::{Consume, RestoreSameApproval}` independent of the legacy transport
  `retryable` flag;
- same-token restoration only for explicitly transient SQLite busy/locked or transient IO errors,
  with the original monotonic deadline preserved;
- a 16-entry bound covering both waiting and in-flight grants, with expiry swept before admission
  and no eviction of a valid approval;
- deterministic and idle background sweeping of expired private payloads; and
- process-local state, so restart invalidates every token.

The broker grant is tied to its creating broker. A taken grant retains its capacity reservation
until it is consumed, restored, or expires; a concurrent preview therefore cannot steal the slot
needed to restore an explicitly transient failure.

## Surface migration

- MCP Application approval, job intake, and task completion share
  `ApprovalBroker<PendingMutation>` and expose `expires_at_unix_ms` plus
  `remaining_ttl_seconds`.
- Tauri manages exactly one `DesktopApprovalStore` for discovery import/refresh, job intake,
  workflow rerun, and task completion. Commit requests supply current Workspace and operation
  context for fail-closed binding checks.
- The Svelte bridge and call sites carry the Workspace/kind fields required by commit and expose
  expiry metadata for all four preview read models.
- Workflow rerun commits now verify the previewed job revision inside the same immediate Store
  transaction before any downstream invalidation or audit mutation.

## Regression evidence

Focused tests cover clock rollback, expiry, collision, live-entry and in-flight capacity, idle and
deterministic sweep, wrong kind, cross-Workspace, wrong Pack digest, stale revision, validation,
replay, restart, concurrent exactly-once take, SQLite busy, transient IO, permanent IO, explicit
disposition, preserved-deadline restoration, and no-mutation behavior.

Adapter evidence covers:

- MCP Application preview/commit/stale/replay;
- MCP stdio job and task preview/commit/replay;
- every former desktop preview-store family using the single shared store; and
- Svelte bridge command shapes plus the production frontend build.

The release source gate now runs `cargo run -p xtask --locked -- approvals check`. It rejects a
reintroduced duplicate preview-store type, predictable adapter token prefixes, missing shared
broker use, multiple desktop stores, or missing preview expiry fields.

Verified on 2026-08-03:

- `cargo test --workspace --locked --no-fail-fast` — all executed workspace and doc tests passed;
- `cargo test -p canisend-app --locked` — 100 unit tests with 99 passed and one network test
  ignored, plus the focused integration suites;
- `cargo test -p canisend-mcp --locked` — 3 passed;
- `cargo test -p canisend-gui --lib --locked --no-fail-fast` — 39 passed;
- `cargo test -p canisend-cli --locked --test mcp_protocol` — 5 passed;
- `cargo test -p xtask --locked` — 73 passed;
- focused Clippy for Store, app, MCP, and GUI with warnings denied;
- desktop Svelte check — zero errors and warnings;
- desktop Vitest — 13 files and 72 tests passed;
- desktop production build; and
- `cargo run -p xtask --locked -- approvals check`; and
- `cargo run -p xtask --locked -- release check`.

## Remaining ordered work

The next P0 item is M1-OP-003/GF5-PARITY-001: derive the two-Pack semantic outcome matrix from the
canonical operation registry, execute every required read/mutation and preview/commit outcome,
and machine-list uncovered leaves. The first usable Alpha remains unproven until that and the
remaining M1A/M1F/native qualification gates pass on the exact candidate commit.
