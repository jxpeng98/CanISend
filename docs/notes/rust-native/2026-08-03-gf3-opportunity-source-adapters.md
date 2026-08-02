# GF3-ADAPTER-001 — Pack-qualified Opportunity-source adapters

**Date:** 2026-08-03

**Status:** Implemented locally; work-item linkage, committed source-gate inspection, shared v3
surface routing, and native qualification remain roadmap steps.

## Delivered

- Added one typed host registry that maps the four bounded discovery source kinds to exact
  workflow-Pack capability IDs and their existing capability descriptors.
- Added a Pack-qualified adapter catalog bound to exact Pack ID, version, and content digest.
- Filtered catalog visibility by the verified Manifest's `capabilities.intake_adapters` selection.
- Required Pack declaration before network refresh preview and repeated the check from the
  normalized report before Workspace commit.
- Kept the existing academic discovery command and desktop surface as explicit compatibility
  wrappers that resolve the verified built-in academic Pack.
- Preserved all existing consent, destination, redirect, content-type, byte/item limit,
  provenance, refresh-history, audit, and promotion controls.

## Focused evidence

- IO registry parity proves an exact one-to-one mapping for RSS/Atom, jobs.ac.uk, Greenhouse, and
  Lever, while local CSV has no network adapter capability.
- Provider policy regressions reject HTTP provider endpoints, cross-provider hosts, wrong API path
  shapes, and a missing Lever JSON mode.
- A 1,001-item Greenhouse response fails at the same 1,000-item limit advertised by every
  registration.
- A verified Pack with the RSS/Atom declaration removed hides the adapter and rejects both preview
  and commit before network or Workspace access.
- Existing Store coverage proves deterministic refresh, exact source/cursor provenance, removed
  history, expiration/freshness behavior, audit writes, and idempotent promotion.

## Verification

```console
cargo test -p canisend-app discovery::tests --locked -- --test-threads=1
cargo test -p canisend-io discovery::adapters::tests --locked
cargo test -p canisend-store discovery::tests --locked -- --test-threads=1
cargo clippy -p canisend-io -p canisend-app -p canisend-cli -p canisend-gui \
  --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- release check
```

## Remaining boundary

GF3-COMPAT-001 must bind Agent v2 and `job` direct-intake operations to explicit academic-Pack
mappings and fail closed for non-academic Packs. GF3-UI-001 must render the Pack-qualified catalog
and academic labels from metadata. GF4/GF5 own generic Pack selection and canonical v3
CLI/MCP/desktop parity; this change does not infer or install an external Pack.
