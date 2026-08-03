# GF3-UI-001 Pack-backed desktop presentation

**Date:** 2026-08-03

**Roadmap items:** GF3-UI-001, partial M1F-SURFACE-001

## Outcome

The principal academic desktop journey now receives its vocabulary, form field label, stage
labels, ordered Deliverable labels, and legacy draft choices from the verified academic Workflow
Pack. Shared Svelte views no longer contain a four-kind academic document label table or a branch
that enumerates the four academic draft operations.

## Implementation

- `canisend-app` builds a Pack-bound presentation read model from the shared localization,
  topological stage-graph, and Deliverable-catalog runtimes.
- The Tauri host exposes the read-only `workflow_pack_presentation` command with a closed `en` or
  `zh-CN` request.
- The desktop reloads presentation metadata when the host language changes and discards superseded
  responses.
- Workflow stage cards, Deliverable workspaces, rendered PDF controls, accessible iframe titles,
  and the academic compatibility form use resolved Pack labels.
- Academic v2 draft operations remain explicit Pack compatibility metadata; missing mappings are
  omitted rather than inferred.
- Unknown Pack stage and Deliverable IDs use their stable local IDs as a recoverable display
  fallback.

## Defensive invariants

The owned component is the CanISend presentation adapter. Its defensive invariant is that only a
fully verified embedded Pack can influence UI metadata and that presentation reads cannot mutate a
Workspace. Locale fallback uses the Pack default, task mappings are exact, and the UI does not
interpret labels as identities.

## Verification performed

- `cargo test -p canisend-app workflow_pack_presentation --locked`
- `cargo test -p canisend-gui workflow_pack_presentation --locked`
- `pnpm --dir apps/canisend-desktop check`
- `pnpm --dir apps/canisend-desktop test`

The focused frontend run passed 12 test files and 67 tests. The complete workspace source gate and
release check are run before committing this implementation record.

## Remaining boundary

This completes GF3-UI-001 source implementation for the built-in academic reference Pack. It does
not complete M1F-SURFACE-001: canonical v3 CLI/MCP surfaces, installed Pack selection, generic field
submission, and generic Application UI remain scheduled in GF4 and GF5.
