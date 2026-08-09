# Semantic parity v1

Status: Implemented source contract

Format: `canisend.semantic-parity/v1`

Authority: `crates/canisend-contracts/semantic-parity-v1.json`

## Purpose

This contract turns adapter parity from a transport-shape claim into an executable outcome
matrix. It binds the typed operation registry to bounded local fixtures for both built-in Packs
and the CLI, Tauri, and MCP surfaces. A fixture is accepted only when its exact source path and
test marker still exist.

The matrix is a minimum release-source qualification contract. It does not declare every
adapter-only command semantically equivalent and it does not replace exact packaged-binary or
real-host qualification.

## Required outcomes

The closed outcome vocabulary is:

- `success` for the intended bounded operation;
- `stale` for revision-bound mutation rejection;
- `replay` for single-use approval rejection;
- `wrong-pack` for exact Pack admission failure;
- `wrong-context` for approval or preview binding failure;
- `no-mutation` for every rejected mutating path; and
- `recovery` for an explicitly recoverable preview/commit failure.

Every shared operation must have success coverage. Every shared mutation must prove a rejected
path leaves authority unchanged, and every revision-bound shared operation must prove stale
rejection. Revision-bound Tauri-only operations and approval commits remain qualified through the
separate revision and preview/commit matrices.

## Qualified matrix

The current source contract covers:

| Dimension | Required inventory |
|---|---:|
| Built-in Pack/surface cases | 6 |
| Shared operations | 31 |
| Revision-bound operations | 13 |
| Preview/commit families | 12 |
| Read families | 4 |
| Qualified adapter bindings | 86 |

The six Pack/surface cases are the Cartesian product of `generic-application` and
`academic-job` with CLI, Tauri, and MCP. The clean CLI qualifies Workspace recovery and neutral
Application create/list/show for both Packs; MCP qualifies the same Pack-neutral reads. Tauri
fixtures run the full create, resume, plan, compose, review, approve, and export lifecycle and
still exercise the bounded academic compatibility families. Cross-linked fixtures prove the Pack
boundary without treating retired CLI mutations as supported operations.

The eleven preview/commit families include Application review, v4 desktop Application intake,
desktop discovery and workflow rerun, Profile/Evidence association, Requirement confirmation,
Plan proposal/confirmation, and Deliverable draft/revision. They share the app-owned approval
broker and its exact Workspace, Pack, operation, source, expiry, replay, and recovery rules.
Desktop Application intake additionally qualifies pasted text, local text/PDF, and URL previews
against one neutral Workspace and either built-in Pack.

## Explicitly uncovered bindings

An operation binding absent from the qualified minimum is never silently treated as equivalent.
The validator permits only the typed `canonical-leaf`, `compatibility-alias`, and `adapter-only`
classes to remain explicitly uncovered. Shared leaves may not be uncovered. The current inventory
machine-lists 95 such bindings with surface, leaf, operation, class, and Pack scope.

Run:

```text
cargo run -p xtask --locked -- semantics uncovered
```

The uncovered inventory is work planning data, not a claim that those leaves do not need their
own focused tests. A new shared leaf, missing fixture marker, unregistered Pack/surface case,
missing revision-bound operation, incomplete preview pair, or unclassified uncovered leaf fails
the gate.

## Source gate

Run:

```text
cargo run -p xtask --locked -- semantics check
```

The command validates the policy against the compiled typed operation registry and the current
fixture sources. It is also part of `cargo run -p xtask --locked -- release check`.

This closes the M1-OP-003 / GF5-PARITY-001 minimum source implementation. Native candidate,
packaged-binary, Agent-host, accessibility, migration, and target-user evidence remain separate
release gates.
