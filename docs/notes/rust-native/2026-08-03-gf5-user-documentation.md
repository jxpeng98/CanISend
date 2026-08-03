# GF5-DOC-001 — Dual-Pack user paths and migration boundary

Date: 2026-08-03

## Outcome

CanISend now presents two usable, mutually exclusive user journeys before the first Workspace
mutation. The Generic quick start explicitly initializes `org.canisend.generic-application` and
runs the canonical v3 create, Plan, compose, approve, render, and local-export lifecycle. The
Academic quick start explicitly initializes `org.canisend.academic-job` and retains source intake,
workflow, Profile, Evidence, matching, materials, and review through the v2 compatibility surface.

The user guides no longer describe the Generic Pack as future work. They distinguish the latest
publicly qualified `v1.0.0-alpha.5` artifacts from post-tag `main` source and do not invent an
Alpha.6 or Alpha.7 publication. They also state before migration that Workspace v2→v3 preserves
the Academic Pack, requires an exact reviewed plan digest and verified backup, and does not convert
academic records into Generic Applications.

## User-facing coverage

The reconciled path covers:

- guide routing and Pack selection;
- a copyable Generic CLI lifecycle with reviewed candidate JSON and revision boundaries;
- the Academic source/PDF/URL and workflow lifecycle;
- Agent v3's nine canonical tools and Agent v2's thirteen academic compatibility tools;
- the shared ten-minute, exact-context, single-use approval boundary;
- Pack-neutral privacy and consent classifications;
- exact-Pack backup, restore, repair, migration, upgrade, and rollback behavior;
- Pack-routed desktop creation, Generic lifecycle, Academic workspace, and migration controls;
- explicit source, distribution, Pack installation, Generic intake, OCR, Agent, and platform
  limitations; and
- the permanent no-login, no-upload, no-submission product boundary.

The root README and installation guide now report Alpha.5 as the latest public checkpoint while
identifying later source as unqualified. The README architecture and examples route academic
commands through an explicit Academic Workspace rather than relying on the new Generic default.

## Source enforcement

`xtask docs check` now requires ten core guides, including the desktop guide and the new known
limitations guide. It checks local links and stable journey markers for both Pack IDs, both
initialization choices, migration preview/digest, Agent tool families, approval TTL, backup Pack
identity, source/public qualification, unsupported image-only PDFs, and
`submission_performed: false`.

The documented quick-start smoke now creates two disposable local Workspaces. It runs the Academic
intake/workflow/backup/restore path with `--pack academic-job`, then runs the Generic v3 lifecycle
through validated PDF export with `--pack generic-application`. This corrects the old smoke's
implicit Academic commands after the CLI default changed to Generic.

## Focused verification

- `cargo run -p xtask --locked -- docs check` — 10 guides and 7 active release runbooks
- `cargo test -p xtask --locked` — 76 tests
- `cargo clippy -p xtask --all-targets --locked -- -D warnings`
- `cargo run -p xtask --locked -- release check`
- `bash -n scripts/smoke_documented_quickstart.sh`
- `cargo build -p canisend-cli --release --locked`
- `scripts/smoke_documented_quickstart.sh target/release/canisend <temporary-path>` — both Pack
  paths, Academic backup/restore, Generic PDF export, no submission
- `cargo fmt --all -- --check`
- `git diff --check`

All smoke inputs are synthetic local fixtures. The run performs no provider call, portal access,
upload, credential operation, or submission.

## Remaining boundary

This completes the GF5-DOC-001 source implementation. It does not qualify or publish a new Alpha.
The workflow-pack authoring/validation guide remains GF5-SDK-001 P1. Exact native candidate
qualification, linked work items, independent committed-evidence inspection, real Codex/Claude
sessions, and target-user validation remain required before GF5, Alpha.6, or Alpha.7 can be marked
Verified.
