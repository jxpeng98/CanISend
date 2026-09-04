# CanISend Product

Status: Approved product direction; R0 complete; R1 not started
Date: 2026-09-04

## Purpose

CanISend is a local-first application for preparing evidence-bound applications and submissions.
It helps a user move from an opportunity to a reviewed, consented, reproducible deliverable without
letting an AI provider become the system of record.

The product is domain-neutral. Academic jobs are the first built-in workflow pack, not the kernel's
ontology.

## Primary user

A person preparing a consequential application who needs to:

- reuse verified profile and evidence material;
- understand what the agent proposes and why;
- review every material mutation before it is committed;
- preview the exact output that will be exported; and
- recover and audit the work locally.

## Product direction

The desktop App is the primary working surface. A normal user should not need to leave the App to
operate Codex or another supported agent. The provider remains the reasoning engine underneath the
App; CanISend remains the local system of record and policy boundary.

For the MVP, Codex App Server carries agent sessions, streamed messages, cancellation, and host
permission requests over its official stdio JSON-RPC interface. MCP exposes CanISend's
evidence-bound product operations to Codex. These are separate boundaries.

After the Codex MVP is proven, the App may add one generic ACP v1 stdio channel for qualified
ACP-compatible agents. Claude Agent is the first planned qualification target. Products that expose
MCP tools but no supported embedded-session protocol remain external integrations rather than being
presented as native Agent providers.

The product owner confirmed the conversation-led Workbench on 2026-09-04: conversation remains in
the center, while a contextual Inspector on the right owns evidence, changes, consent, and preview.

## Core experience

1. Open or create a Workspace and select an Application.
2. Ask the embedded agent to research, draft, revise, or prepare a deliverable.
3. Watch the response and tool progress stream in the App.
4. Inspect sources, proposed changes, validation results, and consent requirements beside the
   conversation.
5. Approve or reject material changes.
6. Compare the exact files produced by the current and a retained earlier build.
7. Inspect the local revision history and provenance of committed changes when needed.
8. Preview the exact export bytes and export only after final review.

## Information architecture

The desktop shell has three primary destinations:

- **Work** — the active Application, conversation, progress, review, history, and preview;
- **Library** — opportunities, reusable profile material, evidence, and completed Applications; and
- **Settings** — Workspaces, agent connection, privacy, appearance, and diagnostics.

The agent is part of Work, not a separate product area. Workflow and delivery are states of the
active Application, not permanent top-level destinations.

## Product principles

- Local product records, consent, audit, recovery, and export truth stay in the Rust core.
- Git tracks source code, ADRs, Trellis plans, tests, and release provenance; it is not the authority
  for private Workspace content.
- An agent may propose work; it may not silently commit or submit it.
- The UI progressively reveals evidence and controls instead of presenting every subsystem at once.
- Exact-output preview is more important than editing inside a preview surface.
- Codex-specific event shapes stop at the Rust session boundary; add a provider abstraction only
  when a second provider is implemented.
- Every committed Agent action remains traceable to its session/turn, CanISend operation, bound
  preview, approval outcome, revision, digest, and audit receipt without persisting private
  conversation bodies or raw single-use approval tokens.
- Framework and provider upgrades are versioned, tested, and reversible; product records never
  depend on a provider-specific transcript format.
- User content remains traceable through local revisions, digests, receipts, dependencies, and audit
  events without requiring Git or a remote repository.
- File-level history covers only files written and registered by CanISend's managed
  projection/export pipeline. It preserves an immutable path/digest/size manifest and the exact
  historical bytes; comparisons must not scan arbitrary Workspace files or regenerate an old
  version with newer rendering code and call it historical.
- Accessibility, English/Chinese support, light/dark themes, and compact/comfortable density remain
  baseline capabilities.
- Existing shadcn-svelte primitives and semantic tokens are reused; this is a structural redesign,
  not a visual rebrand.

## MVP

The first releasable slice supports one Codex App Server session per desktop window, streamed
conversation, cancellation and resume, CanISend MCP tools, explicit permission and mutation review,
and exact PDF preview inside a simplified Workbench.

The MVP fixes clean Workspace v4 preparation so it never attempts legacy Agent, Job, Task, or
Workflow compatibility paths.

The MVP also includes a Git-like line comparison of retained CanISend-managed file snapshots. It
uses the existing content-addressed BlobStore and a bounded in-process diff engine; it does not
initialize a repository or require Git to be installed.

## Explicitly deferred

- migrating the desktop shell from Tauri to Electron;
- the generic ACP channel, Claude Agent qualification, and other provider parity;
- automatic installation from the ACP Registry or unqualified third-party Agent adapters;
- multiple simultaneous agent sessions;
- arbitrary repository editing or a general-purpose IDE;
- App-owned provider credentials or billing;
- persisted transcript bodies;
- a Git-backed Workspace, automatic Git commits/pushes, or remote repository sync;
- scanning, watching, or versioning arbitrary user-managed files elsewhere in a Workspace;
- rich DOCX/HTML editing or annotations inside the preview; and
- automatic submission to third-party systems.

These capabilities are added only after a measured user need or a failed MVP acceptance criterion
justifies them.
