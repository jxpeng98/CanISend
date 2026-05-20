---
name: canisend
description: Use when preparing evidence-backed academic or professional job application materials, coordinating Codex, Claude Code, Gemini, or another local agent around a CanISend workspace, fetching or filtering jobs.ac.uk RSS leads, parsing job adverts, matching criteria to private profile evidence, reviewing evidence citations, generating or checking modernpro Typst cover letter/application package outputs, or maintaining this project.
---

# CanISend

Chinese nickname: 这也能投.

Core principle: 别编了 / No claims without receipts.

## Operating Mode

Treat this as a local-first preparation workflow. The tool helps prepare materials; it must not submit applications, create accounts, fill portals, scrape job pages, or answer sensitive declarations.

Start by identifying the private workspace and, when relevant, the job folder:

```bash
canisend doctor --workspace <private-workspace>
```

From a development checkout, prefix CLI commands with `uv run`.

## Hard Boundaries

- Treat `profile/`, `jobs/`, `job_leads/`, generated PDFs, and `.env` as private local data.
- Do not commit real CVs, statements, references, job adverts, generated packages, PDFs, API keys, or source URLs that reveal application strategy.
- Do not fabricate applicant evidence. Mark missing evidence as a gap.
- Do not convert Markdown to Typst line by line. Use structured content JSON and modernpro Typst templates.
- Do not run LLM-backed parser or draft steps unless provider config is available and the user has opted in.

## References

Read only the reference files needed for the current task:

- `references/workflow.md`: end-to-end CLI flow from workspace init to final manual submission.
- `references/job-lifecycle.md`: job folder state machine and next action by file/status.
- `references/file-contracts.md`: exact workspace, profile, job, prompt, schema, and Typst file contracts.
- `references/typst-profile.md`: Typst-first profile handling with `modernpro-cv` and `modernpro-coverletter`.
- `references/provider-config.md`: OpenAI-compatible and local command provider configuration.
- `references/quality-gates.md`: evidence, parser, draft, package, Typst, and privacy review gates.
- `references/platforms.md`: how to expose this skill in Codex, Claude Code, Gemini CLI, and IDE agents.
- `references/agent-orchestration.md`: Codex, Claude Code, Gemini coordination patterns.
- `references/privacy.md`: privacy and git-safety rules.

## Default Sequence

1. Run or request `canisend doctor --workspace <private-workspace>`.
2. Determine current job state from `job.yaml` and generated files. Read `references/job-lifecycle.md` when uncertain.
3. Keep profile evidence current with `canisend extract-profile-evidence --workspace <private-workspace>`.
4. Use `canisend run --workspace <private-workspace> --job jobs/<job-slug>` for deterministic generation.
5. Add `--llm-parser` and/or `--llm-drafts` only after checking `references/provider-config.md`.
6. Review outputs against `references/quality-gates.md` before rendering or presenting final package materials.
