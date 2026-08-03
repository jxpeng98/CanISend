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
path leaves authority unchanged, every revision-bound shared operation must prove stale rejection,
and the generic approval commit must cover the complete outcome vocabulary.

## Qualified matrix

The current source contract covers:

| Dimension | Required inventory |
|---|---:|
| Built-in Pack/surface cases | 6 |
| Shared operations | 8 |
| Revision-bound operations | 7 |
| Preview/commit families | 5 |
| Read families | 5 |
| Qualified adapter bindings | 71 |

The six Pack/surface cases are the Cartesian product of `generic-application` and
`academic-job` with CLI, Tauri, and MCP. Generic fixtures run the canonical create, resume, plan,
compose, review, approve, and export lifecycle where the surface supports it. Academic fixtures
exercise the bounded v2 compatibility families. Cross-linked fixtures prove both directions of
the Pack boundary: canonical generic operations fail on the academic Pack, and academic
compatibility operations fail on the generic Pack without mutation.

The five preview/commit families are generic Application review, academic job intake, academic
task completion, desktop discovery, and desktop workflow rerun. They share the app-owned approval
broker and its exact Workspace, Pack, operation, source, expiry, replay, and recovery rules.

## Explicitly uncovered bindings

An operation binding absent from the qualified minimum is never silently treated as equivalent.
The validator permits only the typed `canonical-leaf`, `compatibility-alias`, and `adapter-only`
classes to remain explicitly uncovered. Shared leaves may not be uncovered. The current inventory
machine-lists 148 such bindings with surface, leaf, operation, class, and Pack scope.

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
