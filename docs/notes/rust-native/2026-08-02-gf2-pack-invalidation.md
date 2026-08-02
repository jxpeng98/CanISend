# GF2-INVALID-001 — Exact Pack migration and scoped invalidation

**Date:** 2026-08-02

**Status:** Implemented locally; installed-Pack registry integration, shared surfaces, work-item
linkage, committed source-gate inspection, and native qualification remain roadmap steps.

## Delivered

- Added append-only schema migration 17 and an immutable Application Pack-migration ledger.
- Added deterministic preview/commit services over already verified source and target Pack bundles.
- Required an exact current source binding, the same Pack ID, a greater target version, a changed
  content digest, and an explicit migration declaration from the source version.
- Validated taxonomy/field mappings, target required metadata, field types/options, and current
  Application compatibility before mutation.
- Rebound every entity to the target Pack while staling only dependency-reached Plan and
  Deliverable state; stale outputs retain their historical inputs and content.
- Reported old projection rows as superseded derived state, excluded them from ordinary repair,
  and rebound same-path manifests only when the current revision is explicitly projected.
- Bound commit to the reviewed head, snapshot, manifests, semantic impact, and projection set, then
  wrote revision, dependencies, head, audit, and ledger in one immediate transaction.

## Focused evidence

- `label_only_upgrade_rebinds_without_invalidating_outputs_and_supersedes_old_projections`
  covers no-impact migration and projection supersession.
- `template_change_invalidates_only_the_affected_deliverable_kind` covers local Deliverable impact.
- `requirement_contract_change_stales_the_plan_and_its_materialized_outputs` covers dependency
  closure while preserving historical inputs.
- `stale_preview_and_ledger_failure_are_atomic_and_retryable` covers concurrency, rollback, retry,
  and replay rejection.
- `wrong_source_id_digest_and_missing_migration_fail_without_mutation` covers exact binding,
  same-ID, changed-digest, greater-version, and declared-predecessor gates.

## Remaining boundary

GF1-REG-001 still owns installation/persistence and resolution of verified Pack bundles.
M1F-SURFACE-001 owns app, CLI, MCP, desktop, and canonical Agent v3 migration controls. Cross-Pack
conversion remains intentionally unsupported; it requires a separately reviewed import/clone
contract instead of weakening same-Pack migration.
