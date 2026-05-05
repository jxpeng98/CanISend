# Agent Orchestration

Use this reference when Codex, Claude Code, Gemini, or another local agent coordinates the preparation workflow.

## Common Starting Point

Before touching private data:

```bash
academic-prep doctor --workspace <private-workspace>
```

From a development checkout, prefix CLI commands with `uv run`.

If the agent is maintaining the repository rather than preparing a private application, inspect `examples/end_to_end/README.md` and run:

```bash
uv run pytest tests/test_examples.py -v
```

## Coordination Rules

1. Read `workflow.md` for the end-to-end sequence.
2. Read `job-lifecycle.md` to decide the next action from current job state.
3. Read `provider-config.md` before enabling `--llm-parser` or `--llm-drafts`.
4. Read `quality-gates.md` before presenting materials as ready for review.
5. Read `privacy.md` before staging, committing, quoting, or summarizing private files.

Agents should coordinate through CLI commands and local files, not through hidden state.

## Suggested Agent Roles

Use separate agents only when the user explicitly asks for multi-agent work.

- Lead coordinator: runs `doctor`, identifies workspace/job state, chooses next command, and checks privacy boundaries.
- Lead scout: fetches jobs.ac.uk RSS leads and summarizes candidate roles without scraping full pages.
- Evidence reviewer: checks `profile/generated/` coverage and reports gaps without editing private Typst sources.
- Draft reviewer: checks fit report, cover letter, CV notes, and criteria checklist against quality gates.
- Typst reviewer: checks `cover_letter_content.json` and optional PDF rendering.

When multiple agents are used, give each agent a bounded task and disjoint write scope. Do not have two agents edit the same job output file at the same time.

## Handoff Format

Use this compact handoff when passing work between agents or tools:

```text
Workspace: <private-workspace>
Job: jobs/<job-slug>
Current status: <job.yaml status or missing>
Last command run: <command>
Relevant files changed: <paths>
Next recommended action: <action>
Privacy notes: <any private files touched, or "none staged">
```

## Provider Coordination

The local command provider can point at Codex, Claude Code, Gemini, or another CLI. The command must read stdin and write stdout. Do not assume one provider exists; check config and ask the user before using model-backed steps.

For command-provider tasks, prefer prompts that require JSON or evidence-cited Markdown output. Reject output that omits required citations when evidence exists.

## Boundaries

Agents must not:

- commit `profile/`, `jobs/`, `job_leads/`, `.env`, PDFs, or real application material
- scrape full job pages in V1
- submit applications or interact with portals
- answer sensitive declarations
- fabricate applicant experience, publications, teaching, service, grants, awards, or references

The Typst layer is structured. Agents may update `03_cover_letter_draft.md` or `jobs/<job-slug>/typst/cover_letter_content.json`; they should not replace this with line-by-line Markdown-to-Typst conversion.
