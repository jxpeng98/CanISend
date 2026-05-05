---
name: academic-application-prep
description: Use when preparing academic job application materials, parsing academic job adverts, matching criteria to a local profile, working with Typst-first academic profiles, or maintaining this project.
---

# Academic Application Prep

## Use This Project Safely

This project prepares academic job application materials only. It does not submit applications, fill portals, create accounts, or answer sensitive declarations.

## References

- For the user workflow, read `references/workflow.md`.
- For repository file contracts, read `references/file-contracts.md`.
- For Typst-first profile handling, read `references/typst-profile.md`.
- For Codex, Claude Code, Gemini, or other agent orchestration, read `references/agent-orchestration.md`.
- For privacy rules, read `references/privacy.md`.

## Core Rules

- Treat `profile/`, `jobs/`, and `job_leads/` as private local data.
- Use `prompts/` for application LLM prompts; do not call them Codex skills.
- Use `agent-skills/` for Codex-readable skills.
- Require profile evidence citations before using strong application claims.
- Keep generated materials human-reviewable and conservative.
- Coordinate the workflow through local files and CLI commands; do not scrape pages or submit applications.
