# Operation registry v1

Status: Implemented source contract  
Format: `canisend.operation-registry/v1`  
Authority: `crates/canisend-contracts/operation-registry-v1.json`

## Purpose

The registry is the single leaf-level identity authority for the compiled CLI, Tauri, and MCP
adapters. It separates callable leaves from presentation families and records the exact boundary
between clean Workspace v4 operations, the remaining desktop-only academic compatibility
aliases, and adapter-only behavior.

The older `cli-gui-parity-v1.json` and `svelte-parity-v1.json` files remain UI evidence ledgers.
Their composite and wildcard families are not callable operation IDs and cannot replace this
registry.

## Typed model

`canisend-contracts` exposes validated `OperationId`, `OperationStatus`, `OperationClass`,
`OperationPackScope`, `OperationSurface`, and `OperationRegistry` types. The status registry is
closed and declares the classes allowed for each status:

- `implemented`: canonical leaves, shared leaves, composites, wildcard aliases, and adapter-only
  leaves;
- `deprecated`: compatibility aliases only; and
- `deferred-beta`: canonical, shared, or adapter-only leaves that are deliberately unavailable in
  the current release line.

Adding an enum status without completing its exhaustive Rust match fails compilation. Omitting a
status policy, duplicating it, or assigning an operation to a disallowed class fails registry
validation.

## Operation classes

- `canonical-leaf` is a Pack-qualified operation exported by at most one current adapter.
- `shared-leaf` has the same semantic operation on at least two adapters.
- `compatibility-alias` is deprecated, bounded to the academic Pack, and points to one canonical
  v3 target.
- `adapter-only` is a real exported adapter command or tool without a claimed cross-adapter
  semantic equivalent.
- `composite` and `wildcard-alias` are non-callable presentation groupings with exact registered
  members.

Two leaves on the same adapter may not resolve to one operation ID. Cross-adapter sharing must be
declared as `shared-leaf`; a falsely shared canonical leaf fails validation. A binding's Pack
scope must exactly equal its operation or compatibility-alias scope.

## Exact adapter inventories

The built-in registry currently owns:

| Adapter | Derived source | Registered leaves |
|---|---|---:|
| CLI | Compiled Clap command tree | 31 |
| Tauri | `tauri::generate_handler!` | 129 |
| MCP | `#[tool_router]` `canisend_*` methods | 36 |

Every leaf is listed. An unoverridden leaf receives a deterministic adapter-prefixed
`adapter-only` ID. Overrides may target only a declared canonical operation. The Alpha.7 registry
exports no compatibility aliases, so an arbitrary, legacy, or Pack-incompatible shared mapping
cannot be introduced.

The CLI graph contains only product inspection, Schema/resource inspection, Workspace v4
lifecycle, Pack-bound Application create/list/show/archive and Requirement/Plan/Deliverable reads, clean Agent v4 host
setup/status/remove, Workspace Profile Source import/list, Application-scoped Profile/Evidence link
inventories, and the guarded MCP server. Host setup installs only manifest-owned Skills
and prepares deterministic MCP configuration; it does not rewrite host configuration. The MCP
router contains only Workspace status/check, Application list/show, body-free Workspace Profile
Source listing, Application-scoped Profile/Evidence link inventories, Pack-bound
Requirement/Plan/Deliverable reads, private Deliverable audit with explicit consent, and
single-use guarded association and Application-mutation preview/commit pairs. Alpha.6-era CLI families
are refused before parsing or Workspace discovery and have no compiled command implementation.
The Alpha.7 registry contains zero compatibility aliases. The six transitional read-only Tauri
bindings have joined the already retired Alpha.6 mutation and preview bindings: legacy Agent, Job,
Task, and Workflow inputs fail closed before mutation and direct users to clean Workspace v4
initialization. Profile Source import/list use strict neutral Workspace v4
`profile-source.import` / `profile-source.list` operations across the shared facade. Their frontend
entry points reject locally before invoking Tauri, and their internal
facades remain only as historical regression fixtures rather than public product surfaces.
The v2-to-v3 migration preview and commit handlers are retired on the same boundary, so Alpha.7
does not expose a hidden Workspace migration path.

The clean-v4 Tauri inventory now also binds the body-free
`profile.association.list` / `evidence.association.list` reads and their exact preview/commit
pairs. These operations are canonical v4 leaves, not compatibility aliases; they bind one selected
Application, exact resource revisions and digests, and explicit private-read consent where needed.
Requirement confirmation, Plan propose/confirm, and Deliverable draft/revise add five more guarded
preview/commit pairs on Tauri and MCP. Each preview is bound to the exact Workspace, Pack,
Application revision, snapshot and proposed bytes; each commit requires explicit approval and
consumes its token on denial, success, stale context, or replay. The standalone CLI does not
advertise these mutations until it has an equally explicit safe approval interaction.
The desktop now uses those reviewed v4 operations directly, so the former direct
`application.plan` and `application.compose` Tauri writes are no longer registered product
surfaces.

## Source gate

Run:

```text
cargo run -p xtask --locked -- operations check
```

The check validates the typed registry, derives the Clap leaves, extracts every registered Tauri
handler and MCP router tool, verifies the Tauri declarations, and requires exact set equality.
It is also part of `release check`.

The operation registry proves identity, classification, Pack scope, and adapter coverage. Semantic
outcome qualification is a separate source contract in
[Semantic parity v1](semantic-parity-v1.md), which binds stale, replay, wrong Pack/context,
no-mutation, recovery, and two-Pack fixtures to this registry.
