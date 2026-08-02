# GF1 workflow-pack localization implementation record

**Date:** 2026-08-02

**Roadmap task:** GF1-I18N-001 core localization foundation

**State:** Implemented and source-gate verified in this change. Workspace v3 locale persistence,
Pack-backed read models, desktop presentation binding, and end-to-end restart qualification remain
required before the full roadmap task becomes Verified.

## Implemented boundary

- Added `WorkflowPackLocalizationRuntime`, constructed from an exact
  `VerifiedWorkflowPackBundle`; it performs no filesystem, network, database, process, or
  installation operation.
- Bound locale selections to Pack ID, version, and content digest so selections cannot cross Pack
  or snapshot boundaries.
- Mirrored the existing desktop preference contract with serializable `en` and `zh-CN` host locale
  values.
- Resolved English as `en`, and Simplified Chinese in the explicit order `zh-CN`, `zh-Hans`, `zh`,
  then the declared Pack default.
- Resolved arbitrary locale IDs by exact match, declared primary language, then Pack default.
- Resolved individual localized labels through the selected locale and then their required Pack
  default, reporting the fallback class at both levels.
- Kept serialized locale selections body-free; vocabulary and labels remain in the verified Pack
  runtime.
- Added a shared maximum-locale constant and independently rechecked locale count/default presence
  at core compilation.

## Contract invariants

- Placeholders use balanced `{lowercase-kebab-case}` tokens; doubled braces represent literal
  braces.
- Every localized vocabulary field and label preserves the default locale's placeholder names and
  occurrence counts. Translation order may differ.
- Empty, oversized, ordinary control-bearing, malformed-placeholder, and bidi-formatting text
  fails semantic validation before bundle verification.
- Normal multilingual Unicode, combining marks, and right-to-left script characters remain valid;
  only embedded bidi marks, isolates, formatting, and overrides are rejected.
- Digest binding remains authoritative. Locale selection cannot reinterpret another verified Pack
  version, and localization never grants a Pack capability.

## Test coverage

- missing locale collection, Pack default, and per-label default locale fail closed;
- `en` exact selection and `zh-CN` to `zh-Hans` compatibility selection;
- selected-label and per-label Pack-default fallback;
- arbitrary unavailable locale fallback;
- persisted `zh-CN` serialization and deterministic resolver reconstruction;
- body-free selection serialization;
- cross-Pack and stale same-Pack digest selection rejection;
- matching and mismatched placeholder names/counts;
- invalid placeholder syntax and escaped literal braces; and
- valid multilingual/combining Unicode with bidi override rejection.

## Verification

```console
cargo test -p canisend-contracts -p canisend-core --locked
cargo clippy -p canisend-contracts -p canisend-core \
  --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- release check
```

## Remaining boundary

Workspace v3 must persist the user's selected host locale separately from the immutable Pack
snapshot and recreate the same digest-bound selection on reopen. Neutral read models should expose
Pack-resolved vocabulary and labels to CLI, MCP, and desktop adapters without storing translated
copies as authority. The desktop must then replace fixed academic labels at Pack-owned surfaces
while retaining host-owned safety, consent, recovery, and navigation copy.

Translation resources remain opaque verified UTF-8 data in v1. A later resource-format contract is
required before they can extend Manifest vocabulary; it must retain placeholder parity, byte
limits, digest binding, and default fallback.

## Rollback

Revert the localization runtime, shared semantic invariants/tests, documentation, and Roadmap
evidence row together. Existing Workspace and desktop v2 behavior requires no data rollback because
this slice does not persist locale state or replace the current UI message catalog.
