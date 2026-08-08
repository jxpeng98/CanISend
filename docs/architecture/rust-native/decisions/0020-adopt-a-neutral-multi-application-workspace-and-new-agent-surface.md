# ADR-RN-0020: Adopt a neutral multi-application Workspace and new Agent surface

**Status:** Accepted

**Date:** 2026-08-05

**Decision owner:** CanISend maintainer

## Context

Alpha.6 proved that the academic-job and generic-application workflow packs can share one
evidence-constrained kernel. It also exposed four product-model problems that cannot be solved by
adding more compatibility adapters:

- the desktop still presents academic and generic work as separate modes;
- a Workspace is treated as if it had one Pack identity instead of containing independent
  Applications;
- generic intake begins with disconnected manual fields instead of a common source and Evidence
  graph; and
- the Agent surface carries Alpha-era Skills, aliases, and protocol negotiation that constrain a
  clearer host workflow.

The desired product is not a compatibility shell around the academic Alpha. It is a new generic
framework in which one user-owned Workspace can contain different evidence-bound Applications,
each selecting the workflow that fits that Application. The App, CLI, MCP, Codex, Claude Code, and
future MCP hosts must all operate on the same authority without requiring the desktop to remain
open.

## Decision

Alpha.7 introduces a controlled breaking architecture boundary:

1. `canisend.workspace/v4` is a domain-neutral container. A Workspace may contain any number of
   Applications using different Packs at the same time.
2. Every Application, rather than the Workspace, binds one exact workflow-pack ID, version, and
   digest. Pack selection is explicit at Application creation and never inferred from another
   Application in the Workspace.
3. Sources, the basic user Profile, and confirmed Evidence may be stored once at Workspace scope.
   Their use by an Application requires an explicit typed association, applicable consent, and
   revision binding. A Pack cannot silently make another Application's private data available.
4. Academic and generic Applications use one Application collection and one domain-neutral App,
   CLI, and MCP surface. Pack-defined labels and stages may differ, but a Pack does not select a
   different business engine or Workspace mode.
5. The desktop initialization journey creates or opens a v4 Workspace, captures only the minimum
   basic information, validates the two embedded Packs, and can install the new host resources and
   MCP configuration with an explicit preview and user approval.
6. `canisend.agent/v4` is a new task-oriented protocol over the same application facade. Its
   workflow is discover context, select or create an Application, inspect confirmed Evidence,
   propose a bounded change, preview, approve, commit, and verify. Every mutating operation remains
   revision-bound and auditable.
7. CanISend will create new first-party Skills for Codex and Claude Code from the v4 product model.
   They are generated from one canonical resource model, identify the exact protocol and Pack
   context they require, prefer MCP for structured operations, and use the native CLI only as a
   transport-equivalent headless surface. Other MCP hosts use the same operation schemas without
   host-specific business rules.
8. The App calls the Rust application facade in process. The CLI and MCP host remain independently
   usable when the App is closed. No surface writes `.canisend` directly or shells out to another
   surface to implement product behavior.

## Compatibility boundary

Alpha.7 does not promise compatibility with Alpha.6 or earlier Skills, Agent v2/v3 requests,
host-resource layouts, job-specific CLI aliases, or Workspace v2/v3 files. These are not Alpha.7
release requirements and must not shape the v4 public contract.

The project will not silently reinterpret or partially mutate an unsupported legacy Workspace or
request. Version detection must fail closed before mutation and identify the supported clean-v4
initialization path. A separately authorized, one-way import tool may be considered after Alpha.7,
but it is not part of the 1.0 critical path and must not become an implicit compatibility promise.

Published Alpha.6 tags, artifacts, documentation, and evidence remain immutable historical facts.
ADR-RN-0018 remains the authority for how Alpha.6 reached the generic kernel; this ADR supersedes
its post-Alpha.6 Workspace, Agent, alias, and compatibility direction.

## New Skills and Agent workflow boundary

The first v4 host-resource set must provide small, composable tasks rather than a copy of desktop
screens. At minimum it must cover:

- Workspace orientation and health checks;
- basic Profile and Evidence management;
- source intake and explicit Application association;
- Application creation with exact Pack selection;
- Requirement extraction and confirmation;
- fit analysis and planning;
- Deliverable drafting, revision, and evidence audit;
- review, render, package, and export; and
- recovery from stale revisions, denied consent, missing runtime, malformed host output, and host
  restart.

The canonical resource generator owns terminology, operation IDs, safety boundaries, examples,
and version declarations. Host adapters may change packaging and concise invocation guidance only.
Tests must detect semantic drift between Codex, Claude Code, MCP documentation, and CLI help.

## Consequences

- Alpha.7 is a breaking framework checkpoint, not an incremental compatibility release.
- The v4 data model and application facade precede desktop or host-specific implementation.
- A user can create academic and generic Applications in one Workspace and work on either without
  switching Workspace mode.
- Generic intake becomes a first-class source-to-Requirement/Evidence flow instead of disconnected
  manual entry.
- App initialization becomes the easiest setup path, while headless CLI and MCP operation remain
  first-class and use the same authority.
- Beta freezes only the v4 Workspace, Agent, Skills manifest, Pack, operation, and approval
  contracts proven on exact Alpha.7 bytes.
- Legacy import, if ever accepted, is isolated from the v4 kernel and cannot weaken clean-v4
  behavior or release evidence.

## Rejected alternatives

- **Keep one Pack per Workspace:** rejected because it makes the storage container a product mode
  and prevents users from combining academic and generic work naturally.
- **Add more v2/v3 compatibility adapters:** rejected because aliases and negotiation would
  continue shaping the new public ontology and multiply test paths.
- **Translate old Skills into the new hosts:** rejected because the old task structure reflects
  the academic and compatibility-era product rather than the desired Application workflow.
- **Let the App manage a separate database from CLI or MCP:** rejected because it breaks
  headless operation, revision consistency, recovery, and audit authority.
- **Share all Workspace information with every Application automatically:** rejected because
  convenience cannot bypass explicit association, consent, and evidence traceability.
- **Silently upgrade legacy Workspaces in place:** rejected because the new ownership and scoping
  model cannot be inferred safely and a failed conversion could leave mixed authority.
