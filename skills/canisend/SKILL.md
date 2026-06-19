---
name: canisend
description: Use when preparing evidence-backed academic or professional job application materials in a CanISend workspace, coordinating Codex, Claude Code, Gemini, or another local agent, handling jobs.ac.uk RSS leads, matching criteria to private profile evidence, reviewing citations, or checking modernpro Typst outputs.
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

## Agent Contract

Allowed by default:

- Inspect workspace structure, run `doctor`, list job state, and read generated evidence needed for the current task.
- Run deterministic commands such as `extract-profile-evidence`, `fetch-jobs-ac-uk`, `new-job`, `new-job-from-lead`, `run`, and `render-typst` when inputs are local and clear.
- Edit generated drafts, prompt overrides, templates, examples, docs, tests, and skill files within the user's stated scope.

Requires explicit user approval:

- Reading full private CVs, statements, references, full job adverts, or source URLs when a narrow generated-evidence summary is enough.
- Enabling `--llm-parser`, `--llm-drafts`, or a command provider because that can transmit private advert and evidence context.
- Rendering PDFs, overwriting local defaults, or changing workspace-local prompts/templates that may contain private preferences.

Always forbidden:

- Do not submit applications, create accounts, fill portals, answer sensitive declarations, upload packages, or scrape full job pages.
- Do not fabricate applicant evidence; mark missing evidence as a gap.
- Do not stage private files: `profile/`, `jobs/`, `job_leads/`, `.env`, PDFs, real source URLs, or generated application packages.
- Do not quote private materials in chat beyond narrow summaries unless the user explicitly asks.
- Do not claim materials are ready, final, complete, or submission-ready until `references/quality-gates.md` has been checked.

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
