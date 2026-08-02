# GF3-PACK-001 — Built-in academic reference Pack

**Date:** 2026-08-02

**Status:** Implemented locally; GF3 adapter/compatibility/UI bindings, work-item linkage,
committed source-gate inspection, and native qualification remain roadmap steps.

## Delivered

- Added the checked-in `org.canisend.academic-job` v1 Manifest as a typed embedded
  `workflow-pack` resource with exact content digest.
- Declared English/Simplified-Chinese vocabulary, the canonical ten-stage DAG, eight Requirement
  and Evidence categories, four ordered Deliverables, five prompts, two templates, five validators,
  registered Typst rendering, and seven intake/discovery capability references.
- Added a raw embedded bundle boundary and application-level verified loader using the same
  data-only byte, runtime, capability, resource, and digest checks as external candidates.
- Removed the duplicate two-stage academic Pack constructor from Workspace migration tests.
- Changed the Application v2→v3 migration facade to resolve the built-in Pack internally and made
  Store migration reject external same-ID bundles before mutation.
- Added the Pack manifest to ordinary embedded-resource verification and catalog export without
  introducing a new executable Pack-body resource kind.

## Focused evidence

- `academic_pack_preserves_the_legacy_stage_graph_and_modes` compares all ten stage IDs,
  dependencies, output classes, execution modes, and terminal stage with the canonical v2 graph.
- `academic_pack_owns_the_canonical_taxonomy_materials_and_resources` checks exact taxonomy,
  Deliverable order/cardinality/templates, prompts, validators, intake references, locales, Pack
  identity, and built-in origin.
- Existing Workspace v2→v3 migration tests now use the checked-in Pack and retain semantic
  inventory, Blob, backup, failure-atomicity, projection, old-binary, and recovery coverage.
- `external_same_id_pack_cannot_activate_v3_authority` proves declared identity alone cannot gain
  the built-in legacy compatibility boundary.

## Remaining boundary

GF3-ADAPTER-001 must bind declared intake capabilities to the optional adapter catalog.
GF3-COMPAT-001 must route Agent v2 and `job` operations through explicit academic-Pack mappings and
fail closed for generic Packs. GF3-UI-001 must resolve Pack vocabulary and forms in English and
Chinese. The generic starter Pack remains GF4 rather than being inferred from this domain Pack.
