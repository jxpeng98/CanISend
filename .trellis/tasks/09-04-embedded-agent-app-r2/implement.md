# R2 execution checklist

> 2026-09-06 closeout: App-first execution is superseded by CLI-first delivery under
> [ADR-RN-0023](../../../docs/architecture/rust-native/decisions/0023-prioritize-cli-first-local-agent-workflows.md). R0/R1 source completion is retained; R2 is partial
> and unaccepted; unfinished R2-R6 scope is deferred. Historical checklists below are not the
> active queue. The master roadmap owns the next CLI-first slice.

Status: Deferred — partial R2 source retained; isolation, consent and qualification gates remain open
Updated: 2026-09-05

This is the single R2 checklist under the [master roadmap](../../../docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md#33-approved-app-first-delivery-sequence).
Use the retained PRD/design as contracts; no new Trellis child, activation, dispatch, or journal is
required. R2a/R2b may be separate reviewable changes without creating another planning hierarchy.

## R2a — Protocol, binding, and consent

- [x] Inspect installed CLI 0.152.0 schemas; extend the existing fake App Server with body-free MCP
      item handling and rejection of unqualified input/elicitation/permission requests on start/resume.
- [x] Probe the real installed App Server with a deterministic local model/MCP: required startup
      and failure on start/resume, explicit read acceptance/denial, fresh prompting on later/resumed
      turns, inherited-server overrides, and named-profile positive/negative reads. See the
      [25-check report](research/r2a-codex-0.152.0-probe.json); this is not signed-in qualification.
- [x] Confirm installed-account recognition under process-level named permissions with an explicit
      account-only probe: no credential copy, global config edit, thread creation, or model turn.
      See the [availability result](research/r2a-installed-account-probe.json).
- [x] Integrate named permissions before initialization and on start/resume/turn, canonicalize
      executable/session paths, disable execution/hooks/plugins/apps by process overrides, and reject
      versions other than the locally qualified 0.152.0. Policy rejection has no legacy fallback.
- [x] Apply the owner's 2026-09-05 separate-login amendment: share dedicated child configuration/user
      directories between App Server and the Codex-owned browser login, discard login output, and
      retain provider state across session cleanup. Exclude ambient provider/configuration variables.
      Serialize login against session/turn workers and revoke send confirmation on every login attempt.
- [x] Prove the dedicated user directory excludes a synthetic external Skill and the separate
      `CODEX_HOME` ignores external required MCP changes after initialization on macOS arm64.
- [ ] Close inherited MCP and host skill discovery, then verify the full signed-in product flow
      without credential copying or global edits. The real probe confirms thread creation reloads
      MCP settings added after `config/read`; snapshot-and-disable is insufficient. Account recognition
      is proven separately; it does not qualify inference or product consent. A standalone Skill
      description still reaches the local model with the discovery skip flag enabled and its directory
      excluded from the read grant. Neither mechanism qualifies host-Skill isolation.
- [ ] Integrate required-server readiness, per-tool private-read prompting, user reviewer, and safe
      correlation of the actual MCP approval request. If unavailable, keep private tools/commits
      disabled and continue only independent work.
- [x] Inventory every tool's ID paths and returned fields. Add optional CLI `--application` and one
      server-instance guard; route all handlers through it. Constrain or disable Workspace-wide/no-ID
      enumeration. Preserve the unbound external contract.
- [x] Bind desktop catalog/start/turn/cancel and session persistence to an existing Application
      through the shared resolver, bridge and Agent view. Keep legacy Job identities separate;
      invalidate send confirmation and stale UI events on scope changes.
- [ ] Inject the selected Application's required stdio server on start/resume after the provider
      isolation gate passes. Session metadata binding alone grants no MCP authority.
- [ ] Add only the bounded pending-response state and one resolution command required by the proven
      protocol. Use existing consent/preview broker types; correlate private context/provider send,
      commit approval, and export separately. Model booleans and inherited auto-approval grant nothing.
- [x] Show denied command/file/extra-permission requests in the existing Agent Alert, with English/Chinese
      copy and scope/new-turn reset. This adds no approval authority.
- [ ] Use the existing Agent view for consent/approval cards; preserve English/Chinese, keyboard
      operation, focus, and cancellation.
- [ ] Prove a selected-Application metadata read, a consented private read/provider send, and
      preview/approve/commit/verify through the same facade for both Packs. Cover denial, stale/replay,
      scope mismatch/switch, cancellation/timeout, unknown requests, verification failure, and exit.
- [ ] Run one disposable signed-in smoke of the proven flow; record safe version/scenario/outcome
      evidence only. Mark R2a complete only after the effective boundary and approval loop pass.

## R2b — Durable origin and exact history

R2b design/fixture preparation may overlap a blocked provider proof once the receipt/binding contract
is stable. Keep one owner of the Store migration and integration; do not enable R2a tools early.

- [ ] Add the minimum canonical audit reference needed for verified receipt reconciliation, reusing
      existing contract/Store owners. Reject ambiguous or mismatched identity.
- [ ] Use one additive migration (expected 0021) for snapshot/entry rows and the small audit-origin
      relation. Register every retained file Blob with the existing backup/audit mechanism.
- [x] Keep registry v3 bounded and body-free with lossless v1/v2 metadata migration and distinct
      Workspace/Application/legacy Job keys. Origin fields and durable audit attachment are pending.
- [ ] Attach verified committed origins through the facade to canonical audit IDs; identical
      attachments reuse, conflicting ones fail.
- [ ] Prove durable origin survives registry eviction/deletion and restore. Inject a post-commit
      attachment failure and verify an explicit trace gap without mutation retry.
- [ ] Populate snapshots only from existing generated batches. Normalize logical paths relative to
      generation roots, retain actual build identity, and link same-kind predecessors.
- [ ] Reuse only an identical current head with full binding. Test same bytes/new generator, different
      export destinations, consecutive identical generation, and A -> B -> A with Blob deduplication.
- [ ] Integrate projection pending publication and Blob-backed repair/restore; preserve explicit
      legacy unsnapshotted behavior. Reuse export create-new cleanup and transaction owners.
- [ ] Prove interrupted publication, post-export transaction/cleanup failure, missing/corrupt Blobs,
      backup references, and exclusion of unmanaged files with the existing bounded fixtures.
- [ ] Add/lock `similar` 3.2.0 in `canisend-app` only with the real comparison implementation; recheck
      its actual lockfile source, license/advisory policy, and maximum-input behavior.
- [ ] Add manifest A/M/D/U and selected-file comparison through the facade and three desktop read
      leaves. Test no-body fast paths, ownership, exact newline/whitespace behavior, binary handling,
      and every byte/line/row/time bound. Do not add historical bodies to MCP.

## Verify and hand off

- [ ] Record one primary check per invariant in this checklist/evidence note, plus only the adapter
      wiring tests needed to prove reachability. Reuse current dual-Pack/protocol/recovery fixtures.
- [ ] Run changed-language formatting/static analysis and the affected focused tests. Protected Fast
      CI owns the full workspace suite; do not repeat it locally at each planning or integration step.
- [ ] For each final integration PR, run `cargo run -p xtask --locked -- release check` once after
      applicable contract/spec updates. Record exact source and the required freeze disposition when
      commits exist; never invent future hashes or use a local check as artifact qualification.
- [ ] Update evidence and remaining gates once. Overall R2 exits only when R2a and R2b pass. R3 can
      then compose the proven operations into complete user journeys.

## Ownership and rollback

- Runtime/consent: existing `canisend-desktop` runtime, MCP server/CLI, App consent/broker, and Agent
  view/bridge; no second client or mutation path.
- History: existing Store database/projection/export/Blob, App receipt/history, and desktop read
  adapters. Keep one migration owner across origin and snapshots.
- Disable the new entry points to roll back. Preserve authoritative commits, external handoff, and
  manual product operations; use existing backup/restore for older-binary schema compatibility.

The first source slice and its actual checks are recorded in [R2a boundary evidence](research/r2a-boundary-evidence.md).
Next: complete the dedicated browser sign-in and qualify effective managed/system configuration,
then inject the bound server and integrate uniquely correlated elicitation and approval UI before
enabling private tools. The separate-login decision is accepted and implemented locally. R2b remains pending. The [dated review](../../../docs/notes/rust-native/2026-09-04-roadmap-execution-review.md)
preserves the earlier planning baseline. No protected CI, signed-in smoke, or release qualification
is claimed by this source slice.
