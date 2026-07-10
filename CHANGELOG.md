# Changelog

## Unreleased

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
