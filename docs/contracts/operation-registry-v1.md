# Operation registry v1

Status: Implemented source contract  
Format: `canisend.operation-registry/v1`  
Authority: `crates/canisend-contracts/operation-registry-v1.json`

## Purpose

The registry is the single leaf-level identity authority for the compiled CLI, Tauri, and MCP
adapters. It separates callable leaves from presentation families and records the exact boundary
between canonical generic v3 operations, bounded academic v2 compatibility aliases, and
adapter-only behavior.

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
| CLI | Compiled Clap command tree | 86 |
| Tauri | `tauri::generate_handler!` | 111 |
| MCP | `#[tool_router]` `canisend_*` methods | 22 |

Every leaf is listed. An unoverridden leaf receives a deterministic adapter-prefixed
`adapter-only` ID. Overrides may target only a declared canonical operation or academic v2
compatibility alias, so an arbitrary or Pack-incompatible shared mapping cannot be introduced.

Canonical Agent v3 capability output resolves its nine MCP tool names from this registry. The
legacy Rust compatibility enum is also tested one-for-one against the registry's 19 alias and
canonical-target pairs.

## Source gate

Run:

```text
cargo run -p xtask --locked -- operations check
```

The check validates the typed registry, derives the Clap leaves, extracts every registered Tauri
handler and MCP router tool, verifies the Tauri declarations, and requires exact set equality.
It is also part of `release check`.

The operation registry proves identity, classification, Pack scope, and adapter coverage. It does
not itself prove semantic outcome parity; stale/replay/no-mutation and two-Pack outcome fixtures
belong to GF5-PARITY-001 / M1-OP-003.
