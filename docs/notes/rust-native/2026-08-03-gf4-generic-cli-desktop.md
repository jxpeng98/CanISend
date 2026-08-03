# GF4-UI-001 generic CLI and desktop journey

**Date:** 2026-08-03

**Roadmap item:** GF4-UI-001; contributes to M1F-SURFACE and the Alpha.7 dual-Pack checkpoint

## Outcome

New Workspaces now select an exact built-in Pack. Generic selection activates canonical v3
authority and opens a Pack-driven Application journey; academic selection retains the v2
compatibility journey. The CLI and Tauri/Svelte desktop can create, list, resume, plan, compose,
privately review, approve, and locally export a generic Application without submission.

## Shared surface

- `workspace init --pack generic-application` is the CLI default;
  `--pack academic-job` is explicit compatibility.
- `application v3-list`, `v3-show`, and the `generic-*` commands use the same App facade as the
  desktop. JSON candidates are regular non-symlink files bounded to 4 MiB.
- Tauri commands wrap the shared operations and keep private-read and private-export consent
  explicit. Resume status is body-free; Deliverable bodies appear only in the consented review.
- The Svelte view derives fields, bilingual categories, stages, and Deliverable choices from the
  exact verified Pack presentation. UTF-8 Requirement spans are calculated against the reviewed
  source bytes, including non-ASCII text.
- Workspace migration is previewed before mutation and requires its exact plan digest plus a new
  verified backup. Migrated v3 Workspaces retain the academic Pack presentation rather than being
  inferred as generic from the Workspace format.

## Accessibility and failure closure

Forms use existing semantic shadcn-svelte controls with explicit labels, descriptions, alert/live
regions, native keyboard behavior, and consent checkboxes. Mutations carry expected revisions;
stale requests remain atomic. Routine lists and status are body-free, review requires private-read
consent, export requires private-export consent, and every export reports no external submission.

Focused verification covers Rust flow review/resume, Pack presentation selection, Workspace Pack
identity across fresh and migrated v3 activation, typed Tauri envelopes, localized UTF-8 spans,
screen-reader/keyboard source contracts, Svelte type checking, frontend tests, production build,
formatting, Clippy, and the source release gate. Native five-target release qualification remains
owned by the release matrix.

## Remaining boundary

GF4-AGENT-001 must expose generic context and guarded operations through Agent v3/MCP.
GF4-EXAMPLE-001 must add the four synthetic scenario families. Direct file/PDF/URL conversion to a
canonical v3 create request and cross-surface semantic parity remain GF5 work. Governance linkage
and independent committed-evidence inspection are still required before the roadmap item becomes
Verified.
