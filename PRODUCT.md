# CanISend Product

Status: Approved CLI-first direction; previous App-first execution superseded
Date: 2026-09-06

## Purpose

CanISend is a local-first Rust framework for preparing evidence-bound applications and submissions.
A domain-neutral Workspace holds independent Applications bound to exact declarative workflow
Packs. Academic jobs are the first reference Pack; the generic application Pack remains supported.

## Primary experience

The user installs the native CLI and verified product resources, then works through a selected
external Agent using the existing CLI/MCP tools and Skills. Core installation, configuration,
application preparation, approval, export, and recovery must work without a CanISend App window.
The Host owns its model, account, and conversation. CanISend owns product records and policy.

1. Create or select a Workspace and Application.
2. Import bounded sources and explicitly associate confirmed Evidence.
3. Inspect Requirements, missing facts, and Pack-defined Deliverables.
4. Generate a candidate with source references and exact input versions.
5. Review the candidate and explicitly approve the final bound preview through a verified path.
6. Commit through the shared facade, verify the receipt, and render/export reviewed output.
7. Resume from canonical product state in another local session; back up and restore independently.

First prove one complete Host journey. Then add same-device task claims, isolated candidates,
exact-candidate reviews, reliable final submission to the local store, and conflict/crash recovery.
At least two real running sessions must demonstrate collaboration before it is advertised.
A second Agent's agreement does not confirm a user's experience or authorize a product mutation.

## Product invariants

- Keep the existing Rust CLI, MCP server, `canisend-app` domain facade, and Store/Blob authority.
- Preserve Workspace v4, Application-level Pack binding, both Packs, and verified resources.
- Facts require user-confirmed Evidence; missing information remains unknown.
- Private read/provider-send, product commit, and export have separate authority. A role prompt,
  model boolean, broad Host auto-allow, or preview token alone cannot establish human consent.
- Active approval belongs to its Broker instance. Prefer final preview/confirmation/commit in one
  instance; restart invalidates active grants. Other sessions read persisted results and receipts.
- Bind candidates and reviews to their exact scope, inputs, revisions, and digests. Revalidate at
  commit, reject stale/conflicting results, and recover from actual receipts without duplicate writes.
- Model execution does not hold a database write transaction. Coordination does not become a
  second product database, workflow engine, or approval system.
- Agents never write `.canisend` directly. Diagnostics and retained test evidence remain body-free.
- Git owns source and release provenance; private product content does not require Git.
- Uninstall and repair preserve user Workspaces. Restore and schema rollback follow existing policy.
- CanISend prepares and exports; it never uploads or submits to a third-party portal for the user.

## Existing App and deferred scope

R0/R1 source completion and partial R2 evidence remain available. The App-first execution path is
closed as superseded, with unfinished R2-R6 work deferred rather than accepted. Keep the existing
GUI and supported maintenance/retention/accessibility behavior. Private embedded tools remain off
until their original boundary is proven; the CLI direction does not relax that control.

Defer custom chat/Workbench expansion, ACP, React/Electron migration, automatic model workers,
provider credential management, and cross-device synchronization. A local coordinator is optional
only if a proven cross-connection need cannot use the existing process/facade. Extra Host support
requires per-Host evidence; same-device operation does not automatically cover WSL, VMs, or containers.

## Delivery and qualification

[ADR-RN-0023](docs/architecture/rust-native/decisions/0023-prioritize-cli-first-local-agent-workflows.md)
owns this decision. The [master roadmap](docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md)
owns ordering: audit, independent CLI delivery, trusted approval, one Host and resumption, then local
collaboration. Qualify the standalone baseline separately from collaboration. Build isolation,
clean-machine runtime, business acceptance, protected CI, and exact release qualification are
separate claims. Historical release identities, freeze rules, and user-evidence thresholds remain.
