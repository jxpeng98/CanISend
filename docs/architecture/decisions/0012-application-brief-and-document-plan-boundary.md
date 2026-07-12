# ADR-012: Separate The User-Owned Application Brief From The Core-Owned Document Plan

**Status:** Accepted as the Task 6 implementation boundary

**Date:** 2026-07-12

## Context

Stage 2 has durable Criteria, Evidence, Match, corrections, and an apply/hold/skip Decision. The next slice must turn a
current decision to apply into a reviewable application brief and an executable required-document plan without
entering Draft or claiming that an application package is ready.

The legacy pipeline keeps language and writing-style values in `job.yaml`, asks broader preparation questions in
Markdown, and generates a fixed material bundle. That representation cannot distinguish an unanswered requirement
set from an explicit confirmation that no documents are required. It also cannot preserve document-specific user
choices when the advert changes or explain why an omitted required document must block later work.

The Brief may contain private motivation, exclusions, emphasis choices, and application strategy. The derived plan
may contain exact advert source text and reveal which documents the user intends to prepare or omit. Treating either
artifact as ordinary control metadata would expose private bodies through AgentResponse, logs, or workflow receipts.
Letting a model provider own either artifact would also violate the user-owned decision boundary established by
ADR-010.

## Decision

`application_brief.yaml` is a user-owned Tier 2 input. `required_document_plan.json` is a core-owned, regenerable
Tier 2 projection. They are separate authoritative artifacts with separate ownership and write policies.

The Task 6 flow is:

```text
current confirmed Decision=apply
  + current advert-document extraction and source receipts
  + user-owned application_brief.yaml
  -> deterministic required-document planning
  -> required_document_plan.json
  -> body-free blocker/status projection for later Draft and Verify
```

### Brief ownership and mutation

The core service may create `application_brief.yaml` only when it is absent. At that one creation boundary, concrete
legacy `job.yaml` values may bootstrap language and writing style. A valid `uk` or `us` language and a non-placeholder
style become the corresponding confirmed initial values; missing, `unknown`, or `needs_confirmation` values remain
unconfirmed. There is no later two-way synchronization with `job.yaml`.

Every programmatic Brief mutation must reuse the guarded user-owned mutation contract from ADR-010:

1. strict, bounded, versioned YAML and a fail-closed artifact-kind mapping;
2. a discriminated scoped patch rather than a whole-file replacement;
3. the expected current raw-byte SHA-256 and revision;
4. explicit consent for initialization, one update, or recovery;
5. one private immutable candidate, a single-winner claim, a final safe reread, and atomic replacement;
6. an immutable body-free receipt and explicit recovery after an interrupted accepted write.

Direct manual YAML edits remain supported. Status validates the bytes that exist and reports only a safe baseline; it
does not normalize or rewrite the file. A semantic reset or replacement does not erase prior private candidates.
Brief candidates, patches, and retained history are Tier 2 and remain inside the ignored private job until the user
makes a separate retention decision.

Read-only Brief status remains available in every Decision state. Creating or semantically updating a Brief, and
generating or refreshing its document plan, require a current confirmed `application_decision.yaml` whose value is
`apply`. `undecided`, `hold`, `skip`, an unavailable Decision basis, or a basis that no longer matches current Criteria
and Match is an executable blocker. An existing Brief is preserved byte for byte when this precondition stops being
true; it is reported for review rather than cleared or silently regenerated.

### Required-document requirement basis

The document requirement set has an explicit basis state with exactly these meanings:

- `unconfirmed`: the complete set is not known or has not been accepted. This includes a missing or ambiguous source,
  an extraction that cannot be reconciled, and an empty `parsed_job.required_documents` list without an explicit
  human confirmation.
- `confirmed`: a non-empty normalized requirement set has been explicitly accepted and remains bound to current
  advert/source receipts.
- `confirmed_empty`: the user explicitly confirmed that the current advert requires no application documents. This
  state is never inferred from an empty parser result, an omitted field, or a failed extraction.

The user confirmation that changes the requirement-set basis is retained in the user-owned Brief. The core-owned plan
projects that confirmation together with current advert-derived requirements and source receipts. Normalized
document IDs are stable semantic identifiers rather than list positions. Duplicate labels and kinds are handled
deterministically, while ambiguous or missing source evidence remains unconfirmed.

An upstream advert, Parse, Criteria, Match, or Decision change does not rewrite the Brief. It invalidates the derived
plan or makes its basis review-required. Regeneration fingerprints the current validated Decision raw hash and basis,
the requirement-set basis and source receipts, the exact raw Brief hash, and the relevant contract versions. A
directly and validly edited Decision therefore remains usable even when it has no CanISend mutation receipt.

### Plan resolution and blockers

`required_document_plan.json` is written only by the deterministic core through the validated single-output stage
runtime. It resolves one task for every current normalized requirement and reconciles explicit Brief choices by
stable document ID. It does not infer a fixed CV/cover-letter/statement bundle.

The plan must expose executable, body-free blocker codes for at least these states:

- the requirement-set basis is `unconfirmed`;
- a current requirement or document choice still needs confirmation;
- a document marked `required` is explicitly set to `omit`;
- a required document has no executable preparation action;
- a Brief choice refers to a document ID that no longer exists in the current requirement set;
- the current confirmed apply Decision or another declared plan basis is unavailable or stale.

A confirmed omission remains recorded; it is not silently changed to `prepare`. Nevertheless, `required + omit` is a
blocker for later Draft and Verify. An omitted optional document may remain a non-blocking confirmed choice. Orphaned
choices remain in the user-owned Brief and appear as reconciliation blockers; regeneration never deletes them.
`confirmed_empty` produces an explicitly empty plan only when its current basis is valid.

Task 6 records these blockers for downstream enforcement but does not generate application-facing prose, run Draft,
or claim readiness.

### Privacy and execution boundary

The Tier 2 private data plane consists of `application_brief.yaml`, strict Brief patch files, immutable Brief
candidates/history, the required-document-plan candidate, and `required_document_plan.json`. The plan remains Tier 2
even if most fields are normalized because it can contain advert source text and application strategy.

The control plane consists of mutation claims and receipts, workflow state, TaskSpec, TaskResult, preparation and
submission receipts, validation and promotion records, manifests, errors, ordinary CLI output, and AgentResponse.
It contains only relative paths, hashes, opaque semantic IDs, confirmation states, reason or blocker codes, counts,
and other scalar status. It never copies language/style values, motivation, exclusions, source text, document labels,
or another private body. Mutation receipts are Tier 1 integrity records rather than copies of the Brief.

An agent must ask before reading the body of either Tier 2 authoritative artifact. Read-only status and Agent context
do not require body access. Deterministic CanISend services may read the job-local inputs to validate and rebuild the
projection without exposing their bodies to a host model.

Brief mutation and required-document planning are local deterministic operations. They do not invoke a configured
provider, access the network, use MCP transport, require a platform SDK/API, or depend on a Codex-, Claude-, or
IDE-specific session. Every supported shell-capable host uses the same CLI, schemas, receipts, and recovery contract.

## Consequences

- Private motivation and document strategy remain user-owned and cannot be regenerated by Parse, Match, or a model.
- The derived plan can be refreshed after upstream changes without overwriting the Brief.
- An empty parser list can no longer masquerade as confirmation that no documents are required.
- Later Draft and Verify stages receive stable machine blockers for unresolved, omitted-required, and orphaned work.
- AgentResponse v1 remains body-free and host-neutral; Stage 2 phases may continue using scalar extensions where the
  frozen protocol requires them.
- The private job may retain previous Brief bodies and plan candidates after failure, reset, or supersession. No
  automatic secure erasure or deletion from backups and snapshots is claimed.
- Multi-file transactions are not introduced. A Brief mutation commits independently; a plan refresh follows from
  the newly current inputs.

## Rejected Alternatives

- Keep the Brief in `job.yaml`: rejected because workflow metadata writes would share a last-writer-wins file with
  private user decisions and could not preserve field-level confirmation.
- Generate the Brief from Match or a model: rejected because motivation, exclusions, emphasis, and document choices
  belong to the user.
- Treat an empty parsed list as no required documents: rejected because absence, parser failure, ambiguity, and
  confirmed none are different states.
- Let the Brief directly own `required_document_plan.json`: rejected because a derived plan must be reproducible,
  validated, fingerprinted, and safely replaceable without rewriting the user input.
- Downgrade the plan to Tier 1 because it is structured: rejected because exact source text and prepare/omit strategy
  are still private application content.
- Allow `required + omit` to pass after confirmation: rejected because recording a choice is not the same as making a
  complete application package.
- Add a platform API or model-provider implementation for Brief: rejected because the CLI and deterministic core
  already provide the portable execution boundary required by this slice.

## Revisit When

Revisit before remote or multi-user workspaces, multi-file transactions, encrypted private-artifact storage, or
automatic retention cleanup. Also revisit if TaskSpec v2 introduces scope-qualified private artifacts, if a portal
provides a separately verified required-document manifest, or before any workflow attempts automatic upload or
submission.
