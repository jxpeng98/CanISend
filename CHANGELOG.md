# Changelog

## Unreleased

- Added generic RSS 2.0, RSS 1.0, and Atom job-feed ingestion while preserving the jobs.ac.uk command.
- Added bounded feed transport, provenance redaction, source collision protection, and safer input validation.
- Kept direct user intake as a first-class path for local PDF/text adverts and single HTML or PDF job URLs.
- Added strict APP-Q package gates with metadata, evidence freshness, Typst structure, blocker, and input-hash checks.
- Added Typst regeneration protection so user edits produce reviewable candidates instead of being overwritten.
- Added focused job-intake, application-package, and submission-readiness skills.
- Added the multi-source discovery and stage-hardening V2 design roadmap.

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
