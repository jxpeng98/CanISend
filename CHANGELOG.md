# Changelog

## Unreleased

- Made command-provider transport, evidence citations, Typst source hashes, and public smoke diagnostics portable
  across Windows while preserving body-free failure reporting.
- Locally accepted the complete Stage 3 dual-document and aggregate-readiness path across Python 3.11–3.14. Source
  and clean-wheel fixtures reach package `reviewed` through 11 guarded stages and 20 immutable user-mutation receipts
  without claiming rendering or submission, and Cover Letter, Research Statement, and package Typst compile with
  Typst 0.15.0.
- Added a fail-closed Stage 3 workspace migration/recovery guide covering additive resource refresh, legacy control
  records, stale decision reset, interrupted mutation recovery, edited Typst preservation, and executable rollback.
- Added strict user-owned `package_review_dispositions.yaml` with explicit-consent revision/hash CAS, immutable
  mutation claims/receipts, interrupted-publication recovery, stale-basis preservation/reset, and non-waivable
  aggregate blockers. `package-review status|init|update` exposes only body-free states, counts, consents, and actions.
- Added strict derived `ApplicationPackageReadinessV1` states over every required-document receipt plus the exact
  package Review and decisions. Optional standalone documents remain outside requiredness; package `reviewed` is not
  rendering approval, submission readiness, or proof of submission.
- Extended `check-package` with fail-closed APP-Q5 revalidation and a final receipt-change check. Legacy packages
  remain readable but cannot pass from Cover Letter readiness alone or from missing, invalid, stale, incomplete, or
  concurrently changed aggregate receipts.
- Added an independent deterministic `package_review` stage and strict packaged
  `package-review-findings.schema.json`. The aggregate output binds the exact Parsed Job, Brief, Required Document
  Plan, derived execution plan, and every observed Draft/Review/disposition/readiness receipt.
- Added fail-closed required-document findings, deterministic duplicate-assertion Evidence-receipt conflict
  detection, Claim-scoped guarded-Draft correction proposals, and explicit semantic alignment deferral. Aggregate
  control responses remain body-free and do not claim package or submission readiness.
- Published and independently verified the `0.3.0.dev2` TestPyPI development checkpoint after retaining
  `0.3.0.dev1` as an immutable pre-upload CI failure.
- Added a standalone compatibility projection for an exact current `reviewed` Research Statement. The pipeline emits
  conditional Markdown and injection-safe Typst views with Draft/Review/disposition/readiness hash provenance,
  replaces stale generated views with a body-free unavailable state, and preserves edited Typst through a reviewable
  candidate.
- Kept Research Statement projection outside application-package content, required package files, APP-Q outcomes, and
  package input hashes. Optional Research Typst can be rendered and explicitly staged, while its pending candidate
  blocks rendering without changing the existing package gate.
- Added document-scoped Review dispositions and derived readiness for both Cover Letter and Research Statement.
  `review-dispositions status|init|update` accepts the stable Required Document Plan ID, auto-resolves a sole target,
  and fails closed when omitted selection is ambiguous.
- Added independent `research_statement_review_dispositions.yaml` CAS, immutable claim/receipt, interrupted-write
  recovery, stale-basis reset, non-waivable blocker, body-free AgentResponse, and schema coverage. Existing Cover
  Letter YAML remains readable through a defaulted `document_kind`.
- Kept disposition histories and mutation namespaces isolated by document; per-document `reviewed` is not
  application-package readiness.
- Added the second guarded document executor for Research Statement with a strict evidence-bound schema,
  `research_statement_draft.json`, host-agent candidate validation/promotion, independent deterministic
  `research_statement_review_findings.json`, and document-specific completeness checks.
- Made Draft/Review adapter selection document-kind-aware through the current stable Required Document Plan ID.
  Cover Letter and Research Statement runs coexist without output/cache/recovery collisions; ambiguous omitted
  selection fails closed, and configured-provider generation remains Cover-Letter-only.
- Promoted `documents.research_statement` / `draft.research_statement` from planned to available while keeping
  configured-provider generation, cross-document Review, package readiness, and submission explicitly out of scope.

- Added backward-readable document-scoped control contracts and `(stage, document_id)` ownership across Draft and
  Review TaskSpec, result, submission, validation, manifest, terminal claim, promotion, WorkflowState, CLI, and
  AgentResponse. Cache, retry, reconstruction, failure, and descendant invalidation no longer collapse future
  same-stage documents, while non-document runs retain the frozen 1.0 wire shape.
- Added optional `--document-id` selection for stage status, prepare, run, and cancel with current Cover Letter
  auto-resolution, strict plan/kind matching, legacy 1.0 Cover Letter association, pending-task resume without
  immutable-record rewrites, and fail-closed ID validation.
- Added a versioned, deterministic required-document execution fan-out with one body-free work item per Brief task,
  an explicit available/planned/unregistered capability registry, exact source-plan hash binding, and fail-closed
  cardinality and executor-unavailable states.
- Added read-only `canisend documents status` and AgentResponse capability routing. It can dispatch guarded
  document paths while keeping unimplemented teaching, supporting, diversity, publication, CV,
  email, interview, and unknown routes explicit without claiming application-package readiness.
- Added Tier 3 configured-provider execution for the structured Cover Letter Draft through the same immutable
  TaskSpec, guarded candidate submission, current-basis validator, atomic promotion, cache, and recovery path as
  host-agent Draft. The provider proposes only sections and Claim semantics; core derives all trusted identity,
  hashes, stable IDs, generation metadata, and review state.
- Added `stage run --stage draft --mode configured-provider --allow-provider-backed`, a bounded packaged prompt,
  body-free failure/status receipts, no-call cache behavior, submitted-candidate resume, input-drift rejection, and
  source/installed-wheel smoke coverage. Raw provider output is never persisted and legacy `--llm-drafts` remains
  a separate compatibility path.
- Added user-owned `review_dispositions.yaml` with strict schemas, explicit-consent one-finding revision/hash CAS,
  immutable receipts, recovery, body-free Agent/CLI status, and exact Draft/Review basis binding. Blocker findings
  are non-waivable; stale dispositions are preserved for explicit reset rather than silently carried forward.
- Added deterministic Cover Letter document readiness (`blocked`, `review_required`, `revision_required`, or
  `reviewed`) without rewriting the core-owned Draft or Review. Structured projections bind the disposition receipt,
  and `check-package` independently re-derives the gate while keeping whole-package readiness separate.
- Locally accepted the first Stage 3 Cover Letter vertical slice across Python 3.11–3.14 with full regression,
  distribution, clean-wheel, guarded Draft/Review, compatibility projection, and fail-closed package-gate evidence.
- Added the Stage 3 structured Draft foundation with strict claim-level support contracts, content-derived IDs,
  current-basis hashes, packaged JSON Schemas, and a guarded host-agent Cover Letter candidate/promotion path.
- Added independent deterministic Review findings for unsupported facts, partial/semantic support, non-factual Claim
  classification, missing required sections, duplicate/long wording, and confirmed Brief-exclusion conflicts, with
  body-free Agent status and blocker counts.
- Added guarded compatibility projection from a current validated structured Draft and blocker-free deterministic
  Review into Cover Letter Markdown, content JSON, and Typst. Every Claim is rendered once with exact Draft/Review
  hash provenance; stale, tampered, blocked, mixed-profile, direct-library, and `--llm-drafts` paths fail closed to
  compatible legacy/provider behavior while edited Typst remains protected.
- Kept Draft and Review artifacts private and proposed: agents write only fresh scratch candidates through guarded
  submit/apply, and only current complete user dispositions can derive reviewed Cover Letter status.
- Locally accepted the complete Stage 2 Decision Spine after migrating deterministic fit/checklist/HR-review and
  Typst package views onto current validated Match projections. Stale, drifted/tampered, graph-invalid,
  mixed-profile, differently parsed, and `--llm-drafts` runs preserve compatible fallback behavior; every Match
  classification remains a proposal rather than a Decision or readiness result.
- Added fail-closed structured HR semantics for unresolved and unknown essential criteria, stable-ID joins,
  confirmed-empty handling, Markdown table escaping, profile-provenance guards, final currentness rechecks, and
  byte-preserving compatibility tests across the full user-owned Decision Spine.
- Added a repository-native canonical skill mirror check to CI and release preparation, and extended the clean-wheel
  smoke through confirmed Brief, guarded host-agent Draft, deterministic Review, structured Markdown/Typst parity,
  Decision Spine byte preservation, and fail-closed package-gate checks.
- Locally accepted the Stage 2 Task 6 slice with ADR-012, a strict user-owned Tier 2
  `application_brief.yaml`, deterministic core-owned Tier 2 `required_document_plan.json`, field-level Brief
  confirmations, source-bound document-requirement states, and body-free Agent status.
- Extended guarded scoped-patch/CAS/recovery semantics to Brief preparation and defined executable plan blockers for
  unconfirmed requirement sets, unresolved choices, `required + omit`, missing preparation actions, and orphaned
  choices. Empty Parsed Job document output is not `confirmed_empty` without explicit current-basis confirmation;
  non-empty confirmation requires complete positive source members bound to current advert anchors.
- Accepted the Stage 2 Task 5 slice with strict user-owned `confirmed_corrections.yaml` and
  `application_decision.yaml`, explicit create-if-absent initialization, scoped correction/Decision patches,
  revision/hash compare-and-swap, single-winner claims, immutable private candidates, and immutable receipts.
- Added host-neutral Agent/CLI status, init, update, and recovery operations. Semantic corrections require current
  Parse and Confirm and a Confirm rerun between patches; unknown remains distinct from `confirmed_empty`, and
  undecided remains distinct from apply, hold, or skip.
- Preserved user YAML bytes and accepted Decision values when their derived basis changes, reporting review-required
  status without normalizing or rewriting manual edits. Private correction/rationale bodies stay in Tier 2
  YAML/candidates/corrected Criteria and never enter Tier 1 receipts or Agent/control output.
- Added fresh-session recovery for a process interruption between publishing a complete immutable/exclusive target
  link and removing CanISend's private temporary link. Status remains read-only, explicit recovery cleans only the
  verified same-directory two-link marker, and ordinary hard links remain rejected.
- Documented that reset/clear/withdraw is not erasure: private-mode candidates (0600 on POSIX) and correction history remain for
  audit/recovery until the user separately removes retained events or the private job; automatic secure deletion from
  backups or filesystem snapshots is not claimed.
- Accepted the Stage 2 Evidence/Match slice with content-derived Evidence IDs, strict `evidence_catalog.json`,
  run-scoped immutable job-local snapshots, bounded race-resistant profile reads, and distinct available, empty,
  unavailable, missing-receipt, stale-receipt, and malformed-input handling.
- Bound Typst-generated evidence to current raw profile sources with source-hash receipts; older receiptless output
  now requires re-extraction, and resumable Evidence rejects workspace-external profile roots and unsafe aliases.
- Added deterministic `criterion_matches.json` with one canonical proposed classification per criterion, explicit
  privacy-safe gaps, matcher provenance, and opaque catalog references. Match remains review input, not a user-owned
  application Decision or package-readiness signal.
- Extended the shared resumable runtime and agent handoff through deterministic Evidence and Match without a
  configured provider or platform API, while keeping evidence bodies only in the private snapshot/candidate/catalog
  data plane and preserving the legacy pipeline.
- Started the Stage 2 Decision Spine with stable criterion IDs and source spans, strict schemas for criteria,
  evidence matches, corrections, decisions, briefs, and document plans, plus a resumable Confirm stage that preserves
  user-owned corrections and reports unresolved review work.
- Added single-active-task enforcement, immutable preparation and candidate-submission receipts, guarded candidate
  writes, receipt-based dependency recovery, and `stage cancel` for safely abandoning stale work.
- Added a CLI-first resumable Parse stage with versioned workflow state, TaskSpec/TaskResult contracts, immutable run
  evidence, precise input fingerprints, candidate validation, output-drift protection, and atomic promotion.
- Added the versioned `canisend.agent/v1` agent protocol, JSON Schema, capabilities, workspace/job context, and structured
  output for initial intake, listing, diagnostics, and package-gate commands.
- Added packaged fake-data capability/context fixtures proving fresh-session handoff from durable workspace state.
- Added conservative workflow-state derivation, privacy-tiered artifact references, explicit consent requirements,
  stable operational error envelopes, and opaque external-path handling.
- Made workspace skill installation self-contained from the canonical `skills/` pack and updated Codex/Claude
  bootstrap guidance to use the machine-readable agent context.
- Made dry-run previews provider-free, delimited imported adverts as untrusted data, and added minimum public-address
  validation for initial and redirected URL/feed hosts.
- Added generic RSS 2.0, RSS 1.0, and Atom job-feed ingestion while preserving the jobs.ac.uk command.
- Added bounded feed transport, provenance redaction, source collision protection, and safer input validation.
- Expanded CI coverage to Python 3.11 through 3.13, added cross-OS CLI smoke checks, and guarded stable releases
  against unpushed or non-main-reachable candidate commits.
- Kept direct user intake as a first-class path for local PDF/text adverts and single HTML or PDF job URLs.
- Added strict APP-Q package gates with metadata, evidence freshness, Typst structure, blocker, and input-hash checks.
- Added Typst regeneration protection so user edits produce reviewable candidates instead of being overwritten.
- Added focused job-intake, application-package, and submission-readiness skills.
- Added the multi-source discovery and stage-hardening V2 design roadmap.

## 0.2.0 - 2026-06-22

- Added direct local text/PDF and explicit single-URL job advert intake with bounded transport and redacted
  provenance.
- Added strict university HR and material-review checks backed by evidence citations.
- Added editable Typst source rendering and protection for user-modified Typst files.
- Added the local multi-worker orchestrator with dependency scheduling, bounded paths, privacy tiers, and explicit
  profile-source edit confirmations.
- Added the reusable multi-skill distribution, hardened workspace readiness checks, LLM-backed evidence
  augmentation, version status reporting, and explicit generated-material git staging.
- Stabilized tag-driven TestPyPI/PyPI release automation through the 0.2.0 beta series.

## 0.1.0 - Alpha

Initial alpha release of the local-first academic application preparation CLI.

- Added installable `canisend` CLI and private workspace workflow.
- Added jobs.ac.uk RSS lead import with local keyword filtering.
- Added Typst-first private profile scaffolding and normalized evidence extraction.
- Added OpenAI-compatible and local command LLM provider support.
- Added job advert parsing, evidence matching, cover letter draft, CV tailoring notes, criteria checklist, final package, and material review checklist outputs.
- Added modernpro Typst source generation with optional local PDF rendering.
- Added cross-platform agent skill resources for Codex, Claude Code, and IDE agents.

This release prepares application materials only. It does not submit applications, fill web portals, create accounts, scrape full job pages, or answer sensitive declarations.
