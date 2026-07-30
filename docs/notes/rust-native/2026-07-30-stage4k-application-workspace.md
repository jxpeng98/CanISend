# Stage 4K Application Workspace evidence

**Date:** 2026-07-30

**Source state:** Working source after commit `541603d`. This record does not authorize or claim
another commit, tag, package, push, or public release.

## Outcome

CanISend now presents application preparation as one selected Application Workspace instead of
three independent Applications, Workflow, and Documents & delivery destinations. The primary
sidebar contains Today, Opportunities, Application workspace, Profile, Agent integration,
Workspaces, and Settings.

The selected workspace and application remain global context. The Application Workspace projects
existing routes into five user-facing sections:

| Section | Existing authoritative surface |
| --- | --- |
| Overview | Dossier, Content Library, sources, and reviewed intake |
| Job & criteria | workflow graph, advert analysis, criteria, and Agent tasks |
| Evidence & fit | evidence, matches, gaps, prohibited claims, and application plan |
| Materials | accepted document set and exact document revisions |
| Review & export | findings, readiness, projections, render, and local export |

Workflow and Delivery components remain the owners of their existing interactions. Stage 4K adds
no database migration, workspace record, artifact kind, private-body cache, Rust service, or
second mutable state model.

## Persistent context and navigation

The fixed application context bar now shows:

- registered workspace and selected application controls;
- Dossier lifecycle state, deadline, current stage, and completion percentage;
- the first relevant blocker and the authoritative next action;
- five keyboard-reachable workspace sections; and
- the last successful action with its resumable route.

Internal Workflow decision/task tabs and Delivery document/review/package/render tabs update the
global detail route. Navigation memory therefore restores the exact workspace, application,
workspace section, internal detail, and bounded successful-action receipt after a normal restart.
Supporting Profile and Agent surfaces retain the same selected application without being folded
into CanISend-owned chat state.

Fixed navigation uses the existing small z-index scale. Deep-link targets reserve 256 pixels of
scroll margin so the macOS minimum-height window does not hide their headings. The context controls
use a two-row layout at the supported 960-pixel minimum width and a single-row layout when space
permits.

## Receipt continuation

Successful mutations expose an **Open result** action in the visible receipt and retain a
**Resume** action in the context bar. Newly created and promoted applications are selected before
their route is recorded, so the continuation cannot point at the previously selected job.

Existing source intake, profile, workflow, task, review, package, and render mutations continue to
use their revision-bound routes. Content Catalog results reuse the same route projection.
Read-only refreshes do not acquire a stale mutation route.

## Loading and performance

Workflow, Delivery, Agent, and Content Library views load only when opened. Every lazy boundary
provides a semantic loading state, a reduced-motion-safe spinner, and a bounded retry path if the
bundled module cannot load.

The production build emits:

- a 462.10 kB minified main JavaScript chunk;
- a 28.42 kB Workflow chunk;
- a 20.17 kB Delivery chunk;
- a 38.98 kB Agent chunk; and
- a 16.03 kB Content Library chunk.

The build completes without a chunk-size warning.

## Verification

- Frontend tests: 6 files and 32 tests passed.
- Navigation coverage includes all five workspace sections, legacy detail projection, selected-job
  memory, exact internal detail restoration, and successful receipt restoration.
- Svelte check: 0 errors and 0 warnings.
- Production Svelte build passed without chunk-size warnings.
- `git diff --check` passed.
- `cargo fmt --all -- --check` passed.
- `cargo run -p xtask --locked -- release check` passed with 40 schemas, migrations frozen through
  13, 37 implemented CLI/GUI operations, and 37/37 Svelte parity.

No native package qualification was run because Stage 4K is a source slice, not an authorized
release checkpoint.

## Next slice

Stage 4L can build contextual external-Agent assistance on the Dossier, Content Catalog identities,
and this five-section route model. It should generate body-free context packets and
revision-bound proposals while leaving Codex/Claude sessions, plugins, search, and credentials in
their native hosts.
