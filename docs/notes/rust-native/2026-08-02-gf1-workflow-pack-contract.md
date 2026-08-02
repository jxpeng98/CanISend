# GF1 workflow-pack v1 contract implementation record

**Date:** 2026-08-02

**Roadmap tasks:** GF1-SCHEMA-001; partial foundation for M1F-PACK-001 and GF1-DAG-001

**State:** Implemented and focused-test verified in this change. Work-item linkage and committed
evidence inspection remain required before the roadmap task becomes Verified.

## Implemented boundary

- Added typed `canisend.workflow-pack/v1` manifest contracts without changing the current
  Agent v2, Workspace v2, Job, fixed-workflow, or runtime loading paths.
- Added strong pack, publisher, item, locale, and registered-capability identifiers.
- Added compatibility ranges, localized vocabulary and labels, metadata fields, Requirement and
  Evidence taxonomies, dynamic stage definitions, Deliverable kinds, capability references,
  validator instances, data-only resources, declarative migrations, and SHA-256 declarations.
- Added bounded validation for a 1 MiB manifest, depth 32, 20,000 JSON nodes, collection counts,
  resource sizes, safe paths, version requirements, locale coverage, duplicates, DAG cycles and
  terminal reachability, Deliverable references, and migration mappings.
- Structurally limited pack resources to prompts, templates, examples, and translations; script
  and executable extensions are rejected independently of their declared kind.
- Generated and embedded a separately versioned workflow-pack Schema without modifying the 40
  frozen Agent v2 public schemas.
- Extended `xtask schemas check/write` and embedded-resource verification to own the new Schema.

## Defensive invariant

This slice validates declarative shape and internal semantic consistency. It grants no capability
and does not install or execute a pack. Resource bytes, canonical bundle digest, registered
capability availability, immutable snapshots, trust decisions, and migration approvals remain
future GF1-REG/GF1-TRUST runtime gates.

## Focused verification

```console
cargo test -p canisend-contracts --locked
cargo test -p canisend-resources --locked
cargo test -p canisend-app catalog --locked
cargo test -p xtask --locked
cargo run -p xtask --locked -- schemas check
cargo run -p xtask --locked -- resources check
cargo clippy -p canisend-contracts -p canisend-resources -p xtask \
  --all-targets --all-features --locked -- -D warnings
cargo run -p xtask --locked -- release check
```

The negative fixtures cover unknown fields/resource kinds, malformed identities, unsafe paths,
invalid digests and compatibility ranges, missing locales, unselected capabilities, cycles,
terminal-disconnected stages, missing templates, script resources, oversized declarations, and
excessive JSON depth.

## Next boundary

GF1-REG-001 should define canonical pack-bundle hashing, verify actual resource bytes against the
manifest, match capability IDs against a kernel-owned registry, store immutable pack snapshots,
and reject substitution or silent update. It must consume this contract rather than adding a
second manifest shape.

## Rollback

Revert the additive workflow-pack types, generated Schema, resource declaration, schema tooling,
tests, and documentation together. Existing v2 schemas, workspaces, and runtime behavior require
no data rollback because this slice does not consume packs.
