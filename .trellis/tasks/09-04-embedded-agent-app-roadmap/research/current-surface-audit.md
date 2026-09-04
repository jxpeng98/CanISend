# Current Desktop and Agent Surface Audit

Date: 2026-09-04

## Finding

CanISend already has the correct product-data boundary and exact-output preview. It lacks a durable
agent-session boundary and has accumulated a frontend information architecture that exposes internal
subsystems more prominently than the user's working loop.

## Current architecture

- `docs/architecture/rust-native/decisions/0015-replace-egui-with-tauri-svelte.md` accepts Tauri 2 +
  Svelte 5, keeps Node at build time, and requires UI calls to cross the `canisend-app` facade.
- `docs/architecture/rust-native/decisions/0019-current-product-graph.md` assigns native window,
  dialog, and IPC ownership to the Tauri shell.
- `docs/superpowers/plans/2026-07-30-stage-4g-connected-agent-workspace-plan.md` intentionally makes
  the App the control plane and external Codex/Claude hosts the reasoning plane. A rich embedded
  client was deferred, so changing that direction requires an explicit authority update.

## Agent gap

`crates/canisend-desktop/src/agent_runtime.rs` starts `codex exec --json` or `claude -p` for one turn
and waits for the child to finish. It is not an App Server client and cannot provide a durable,
steerable, resumable session with live permissions. Its runtime catalog is still keyed by
`selected_job_id` and calls the legacy `Application::job_detail` operation when that value is
present.

The handoff method in `crates/canisend-app/src/agent.rs` already uses Workspace v4 and has a focused
regression. The surrounding Agent screen still wires legacy capability/context controls and the
job-scoped runtime catalog, so the reported preparation failure requires a cross-layer reproduction
rather than another isolated handoff-method fix. The clean v4 screen must stop invoking those legacy
paths.

## Frontend gap

The current snapshot has a large root coordinator and several large workflow pages:

- `apps/canisend-desktop/src/App.svelte`: roughly 2,971 lines;
- `apps/canisend-desktop/src/lib/bridge.ts`: roughly 3,334 lines and 123 invoke call sites;
- `apps/canisend-desktop/src/lib/views/AgentView.svelte`: roughly 1,645 lines;
- `GenericApplicationsView.svelte`: roughly 1,690 lines;
- `WorkflowView.svelte`: roughly 1,175 lines; and
- `DeliveryView.svelte`: roughly 821 lines.

Line count alone is not a defect. The relevant problem is that user state and callbacks are composed
centrally while Agent, Workflow, and Delivery appear as separate conceptual destinations. The new
Workbench should compose existing behavior around the active Application before any broad deletion.

## Existing assets to reuse

- `docs/contracts/operation-registry-v1.md` records 129 registered Tauri operation leaves. Replacing
  the shell would require a new transport and security boundary for that surface.
- `crates/canisend-desktop/src/delivery.rs` returns verified PDF bytes through a raw Tauri response.
- `apps/canisend-desktop/src/lib/views/DeliveryView.svelte` previews those bytes through a Blob URL
  and iframe and offers the system viewer.
- `apps/canisend-desktop/src/lib/delivery-render-preview.test.ts` owns the preview consent/Blob
  regression.
- `docs/architecture/typst-template-preview-execution-plan.md` requires preview and export to use the
  same exact bytes.
- `docs/superpowers/plans/2026-07-25-1.0-release-roadmap.md` records successful native PDF preview
  qualification under WKWebView, WebView2, and WebKitGTK in run `30742363439`.
- `docs/guides/shadcn-svelte-ui-governance.md` and `apps/canisend-desktop/src/app.css` already define
  the neutral semantic visual system, density, locale, theme, and accessibility expectations.

## File-version finding

The product owner clarified that traceability must include files, not only record metadata. The
current store provides most of the required foundation:

- `application_v4_revisions` retains immutable Application snapshots with actor, reason, timestamp,
  and snapshot digest;
- `BlobStore` stores verified content-addressed bytes and deduplicates identical content;
- historical Deliverable content remains referenced from retained Application revisions; and
- `generate_projection_bytes_from_database` can reconstruct managed projection bytes from a named
  Application revision and its content Blobs.

The current projection manifest is not an exact historical file-tree ledger. Its schema makes
`relative_path` unique, and `record_pending_projection` updates that row on path conflict. It
therefore describes the current managed projection rather than retaining every published path/digest
pair. Regenerating an old JSON or rendered file with a newer serializer, template, font, or renderer
would also not prove the bytes that existed at the original build.

The smallest exact solution is an append-only build/file manifest that binds each retained build or
Application revision to relative path, media type, size, digest, and an existing BlobStore object.
Comparison then classifies added, modified, and deleted paths by manifest and displays bounded text
diffs from the two verified Blobs; binary files use digest/size and preview comparison. Git is not
required for that mechanism.

The product owner resolved the file boundary on 2026-09-04: retain only files written and registered
by CanISend's managed projection/export pipeline. Do not scan or watch arbitrary user-managed files
elsewhere in the Workspace.

## Git-style diff evaluation

The requirement is Git-like review clarity, not repository behavior.

| Candidate | Evidence | Result |
| --- | --- | --- |
| System `git diff --no-index` | Git documents that it compares two filesystem paths outside a repository. | Rejected: requires a Git installation, materialized paths, a child process, output parsing, and temporary-file policy. |
| `git2`/libgit2 or `gix` | These are repository/object-model implementations; `git2` also introduces the libgit2 native boundary. | Rejected: unused repository semantics and greater build/release surface. |
| Rust `similar` 3.2.0 | Provides line diffs, grouped/unified output, practical Myers, exact whitespace/newline handling, and timeout configuration; its default build is dependency-free and Apache-2.0. | Selected, subject to the repository's normal dependency qualification. |
| `imara-diff` | Provides a lower-level high-performance Git-like Histogram/Myers implementation. | Deferred: add only if the selected library fails the accepted maximum-input benchmark. |

Sources checked on 2026-09-04:

- [Git `diff` documentation](https://git-scm.com/docs/git-diff)
- [`similar` crate overview](https://docs.rs/crate/similar/latest)
- [`similar::TextDiffConfig`](https://docs.rs/similar/latest/similar/struct.TextDiffConfig.html)
- [`similar::Algorithm`](https://docs.rs/similar/latest/similar/enum.Algorithm.html)
- [`git2` crate documentation](https://docs.rs/git2/latest/git2/)
- [`imara-diff` crate documentation](https://docs.rs/imara-diff/latest/imara_diff/)

The efficient path is therefore: compare the two small managed manifests by path and digest; load no
body for unchanged files; load and verify only the selected old/new Blob pair; run a bounded Myers
line diff; return typed hunks; render escaped rows in Svelte. No derived patch is persisted.

## Consequence

The smallest sufficient product change is:

1. fix clean Workspace v4 preparation;
2. add a direct Codex App Server session manager around the existing facade/MCP surface;
3. introduce one Workbench that composes current product operations; and
4. retain exact snapshots of registered managed outputs and compare selected text Blobs with a
   bounded in-process line diff; and
5. retain Tauri and the exact-PDF preview until an objective requirement disproves them.

A simultaneous Electron migration, protocol replacement, domain rewrite, and frontend rewrite would
multiply risk without proving the missing user loop sooner.
