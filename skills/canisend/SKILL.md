---
name: canisend
description: Use when preparing evidence-backed academic or professional job application materials in a CanISend workspace, coordinating Codex, Claude Code, and IDE agents, handling jobs.ac.uk RSS leads, matching criteria to private profile evidence, reviewing citations, or checking modernpro Typst outputs.
---

# CanISend

Chinese nickname: 这也能投.

Core principle: 别编了 / No claims without receipts.

## Operating Modes

Treat CanISend as local-first only in direct CLI deterministic mode. If an agent such as Codex or Claude Code reads files, PDFs, webpages, or generated materials, that content may be processed by the agent model provider. If the CLI is run with LLM-backed flags or a command provider, selected advert, profile, evidence, and draft context may be transmitted to the configured provider.

The tool helps prepare materials; it must not submit applications, create accounts, fill portals, scrape job pages, upload packages, or answer sensitive declarations.

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

- Reading full private CVs, statements, references, full job adverts, PDFs, source URLs, or generated application packages when a narrow generated-evidence summary is enough. In agent-assisted mode, tell the user that content read by the agent may enter the agent model context.
- Enabling `extract-profile-evidence --llm-augment`, `--llm-parser`, `--llm-drafts`, or a command provider because that can transmit private advert, profile, evidence, and draft context.
- Rendering PDFs, overwriting local defaults, or changing workspace-local prompts/templates that may contain private preferences.
- Modifying original profile inputs under `profile/` outside `profile/generated/`. Prefer job-folder suggestions; write source inputs only through an orchestrator task with `edits_profile_input: true`, a prior review dependency, privacy tier 2+, and two explicit profile-edit confirmations.

Always forbidden:

- Do not submit applications, create accounts, fill portals, answer sensitive declarations, upload packages, or scrape full job pages.
- Do not fabricate applicant evidence; mark missing evidence as a gap.
- Do not edit original `profile/` sources during ordinary draft review.
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
- `references/platforms.md`: how to expose this skill in Codex, Claude Code, and IDE agents.
- `references/agent-orchestration.md`: Codex, Claude Code, and IDE agent coordination patterns.
- `references/privacy.md`: privacy and git-safety rules.

## Default Sequence

1. Run or request `canisend doctor --workspace <private-workspace>`.
2. Determine current job state from `job.yaml` and generated files. Read `references/job-lifecycle.md` when uncertain.
3. Keep profile evidence current with `canisend extract-profile-evidence --workspace <private-workspace>`.
4. Use `canisend run --workspace <private-workspace> --job jobs/<job-slug>` for deterministic generation.
5. Add `extract-profile-evidence --llm-augment`, `--llm-parser`, or `--llm-drafts` only after checking `references/provider-config.md` and getting explicit user approval.
6. Review outputs against `references/quality-gates.md` before rendering or presenting final package materials.
