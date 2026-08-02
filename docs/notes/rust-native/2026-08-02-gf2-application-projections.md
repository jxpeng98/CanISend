# GF2-PROJ-001 — Neutral Application projections

**Date:** 2026-08-02

**Status:** Implemented locally; work-item linkage, committed source-gate inspection, and native
release qualification remain roadmap governance steps.

## Delivered

- Added append-only database migration 16 and a dedicated v3 manifest that binds every projection
  to the immutable Application revision, snapshot digest, Pack identity, optional Deliverable
  revision, and generated/observed digests.
- Added deterministic `applications/APPLICATION_ID/` model, Deliverable metadata, and materialized
  content projections without reusing the v2 Artifact/`DocumentKind` schema.
- Added all-path preflight, Application-head recheck, managed missing/repair-required recovery,
  explicit replace, and edit-preserving copy-as-new behavior.
- Registered materialized v3 content during the authoritative Application revision transaction in
  the existing Blob-reference ledger, so verified backup and staged restore retain the authority
  required to rebuild it even before projection publication.
- Extended the existing workspace repair and restore entrypoint to both projection generations.
- Recognized migrated `jobs/JOB_ID/` projections only through migration links plus existing
  manifests; directory scans, inferred ownership, re-ownership, and legacy writes remain forbidden.

## Defensive invariants and focused evidence

- `generic_projections_preserve_edits_copy_replace_and_repair` proves deterministic paths, edit
  preservation, explicit replacement/copy, repair convergence, and unchanged Application authority.
- `unmanaged_missing_blob_and_symlink_paths_fail_before_projection_ownership` proves unmanaged,
  missing-authority, and symbolic-link fixtures fail before manifest ownership or external writes.
- `migrated_academic_legacy_projection_is_recognized_but_never_reowned` proves bounded legacy
  recognition excludes unmanaged files and preserves old bytes.
- `backup_restore_rebuilds_generic_projections_from_authoritative_content` proves content-reference
  inclusion, verified backup, staged restore, and idempotent repair.

## Remaining boundary

The Store contract is complete for GF2-PROJ-001. Pack-driven CLI, MCP, desktop, and Agent v3
projection operations remain part of M1F-SURFACE-001. Exact packaged-binary recovery qualification
remains a release gate rather than a source implementation claim.
