# Changelog

## Unreleased

- Locally accepted the Stage 2 Task 6 slice with ADR-012, a strict user-owned Tier 2
  `application_brief.yaml`, deterministic core-owned Tier 2 `required_document_plan.json`, field-level Brief
  confirmations, source-bound document-requirement states, and body-free Agent status. This does not mark Task 7 or
  Stage 2 complete.
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
