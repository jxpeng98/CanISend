# ADR-RN-0018: Adopt a generic evidence-application framework

- Status: Accepted
- Date: 2026-08-02
- Decision owner: CanISend maintainer

## Context

CanISend's durable strengths are not specific to academic recruitment. The Rust-native product
already provides local-first storage, immutable content-addressed artifacts, confirmed Evidence,
revision-bound matching, explicit Agent approvals, deterministic review, rendering, export,
backup, restore, repair, and release-integrity controls. Those capabilities apply to many
evidence-bound submissions.

The current public model nevertheless embeds one domain in the core contract. `Job`, `job_id`,
the fixed ten-stage workflow, four fixed academic document kinds, academic profile fields,
jobs.ac.uk discovery, and `jobs/JOB_ID` projections appear across SQLite, Rust types, Agent v2,
the CLI, MCP, and the desktop UI. Relabeling the interface would not make the product generic.

The project needs a product boundary that can support professional jobs, grants, fellowships,
admissions, tenders or proposals, and internal dossiers without weakening the controls that make
CanISend trustworthy. It also needs a controlled migration because the current Alpha contracts
and workspaces already exist.

## Decision

CanISend's final product target is a **local-first, evidence-constrained application-workflow
framework**. A user combines a versioned workflow pack with user-controlled sources to create an
Application, extract and confirm Requirements, confirm Evidence, assess fit, plan Deliverables,
draft and review content, render or package outputs, export them, and recover or audit the local
Workspace.

CanISend will use three architectural layers:

1. **Domain-neutral kernel.** The kernel owns Workspace, Application, Opportunity, Source,
   Requirement, Evidence, Match, Plan, Deliverable, Review, Package, Render, Audit, approval,
   backup, recovery, and invalidation semantics.
2. **Declarative workflow packs.** A pack supplies domain vocabulary, intake metadata, workflow
   stages, Requirement and Evidence taxonomies, Deliverable definitions, templates, prompts,
   validators, localization, and readiness rules. It may reference only capabilities registered
   by the kernel.
3. **Surface adapters.** CLI, MCP, Agent hosts, and the desktop UI expose the same application
   operations and render pack-provided labels without reimplementing business rules.

The 1.0 release must include both:

- `org.canisend.academic-job`, the reference pack that preserves the current supported academic
  application journey; and
- `org.canisend.generic-application`, a configurable starter pack that completes the same core
  journey without academic-only fields or fixed document types.

Additional first-party packs are separate product increments. A third-party pack is not a plugin:
it is validated data and resources, not arbitrary native, shell, JavaScript, Typst-package, or
Agent-executable code.

## Workflow-pack contract

The first pack contract, `canisend.workflow-pack/v1`, must bind at least:

- stable pack ID, semantic version, schema version, publisher, locales, and content digest;
- compatible kernel, Agent protocol, and Workspace format ranges;
- localized nouns, labels, descriptions, and user-facing stage names;
- Application and Opportunity metadata fields;
- a validated acyclic stage graph using stable pack-owned stage IDs;
- Requirement and Evidence categories plus field and confirmation rules;
- stable Deliverable kinds, cardinality, ordering, template/renderer references, and validators;
- readiness and export rules that may strengthen but cannot weaken kernel invariants;
- bundled prompts, examples, templates, and host resources with individual digests; and
- declarative version-to-version migration metadata where the kernel supports it.

Every Application stores the exact pack ID, version, and digest used to create it. Opening a newer
pack version must not silently reinterpret existing data. Migration requires a preview, explicit
user approval, a recoverable backup boundary, and an audit event.

Built-in packs are embedded and digest-verified. Installing an external pack requires an explicit
user action, schema validation, resource-size limits, safe paths, digest calculation, and a clear
trust statement. Signature verification may be added later, but a signature must not grant a pack
extra capabilities.

## Kernel invariants

A workflow pack cannot disable or bypass:

- user confirmation and source traceability for factual claims;
- revision binding, dependency invalidation, and stale-state checks;
- private-read, network, file-write, export, and Agent-mutation consent boundaries;
- URL destination, path, file-size, parsing, renderer, and resource limits;
- preview, approval, expiry, replay, and no-mutation-on-failure rules;
- workspace integrity, backup, restore, projection-repair, and audit behavior;
- review of unsupported claims before readiness or export; or
- the boundary that CanISend prepares and exports but never signs in, uploads, or submits.

Packs may add stricter domain rules. They may not replace the SQLite and immutable-blob authority,
write `.canisend` directly, or introduce a second application engine.

## Compatibility and version boundary

This scope change is intentionally made before Beta. The generic contract cannot truthfully fit
inside the current job-specific Agent v2 and Workspace v2 surfaces.

- Alpha.6 will introduce the workflow-pack kernel and the migration boundary.
- The canonical generic surfaces will use `canisend.agent/v3` and `canisend.workspace/v3` before
  Beta; their schemas use neutral identifiers and pack-defined stages and Deliverables.
- An existing Workspace v2 is deterministically assigned
  `org.canisend.academic-job` and migrated to Workspace v3 through a dry-run-first, backup-backed,
  failure-atomic migration.
- Current fixed document kinds and workflow stages map to stable IDs in the academic pack.
- `job` CLI commands and Agent v2 remain bounded compatibility adapters for migrated academic
  Applications during the 1.0 line. They cannot address non-academic packs and must fail closed
  rather than guess a mapping.
- New generic projections use `applications/APPLICATION_ID/`. Existing managed
  `jobs/JOB_ID/` projections remain recognizable and are never silently overwritten or deleted.
- Storage implementation names may survive temporarily behind the neutral facade, but no new
  v3 public contract may expose an academic-only core noun.

Beta is blocked until migration, downgrade refusal, backup/restore, pack pinning, and both built-in
packs pass the same semantic adapter-parity and end-to-end qualification suites.

## Product boundary

Generic means domain-extensible within evidence-bound applications and submissions. It does not
mean that CanISend becomes:

- a general-purpose workflow automation system;
- a recruitment marketplace or applicant-tracking system;
- a general-purpose AI client, prompt runner, or plugin host;
- a hosted collaboration, form-filling, portal-automation, or submission service; or
- a runtime for untrusted executable extensions.

## Consequences

The academic journey becomes the first reference implementation instead of the kernel's ontology.
The scope is larger than the previous Alpha.6-to-Beta path, so the roadmap adds a second framework
Alpha and requires dual-pack validation before Beta.

The project accepts one pre-Beta protocol and Workspace major transition. This is preferable to
freezing academic names into a supposedly generic 1.0 contract. The migration and compatibility
adapters increase short-term work, but preserve existing user data and provide a finite removal
boundary for job-specific public surfaces.

Future domain support should normally be delivered as a workflow pack. A new core concept is
justified only when at least two materially different packs require the same invariant and the
concept cannot be expressed safely in the pack contract.

## Rejected alternatives

- **Only change product wording:** rejected because fixed public types, stages, document kinds,
  storage paths, and operations would remain academic-specific.
- **Keep the academic model and add optional fields:** rejected because each new domain would add
  branches to the kernel and make semantic parity unbounded.
- **Allow arbitrary executable plugins in 1.0:** rejected because they would undermine local
  trust, reproducibility, consent, and bounded parsing.
- **Remove academic behavior during the refactor:** rejected because the current supported journey
  is the migration oracle and the first reference pack.
- **Defer genericity until after a job-specific 1.0:** rejected because Beta would freeze the wrong
  public ontology and make the later migration more disruptive.
