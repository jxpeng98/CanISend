# Agent Orchestration

Use this reference when Codex, Claude Code, or another local agent coordinates the preparation workflow.

## Common Starting Point

Before touching private data:

```bash
canisend doctor --workspace <private-workspace>
```

From a development checkout, prefix CLI commands with `uv run`.

Start by naming the mode:

- Direct CLI deterministic mode: local CLI work without model-backed flags.
- Agent-assisted mode: the agent may process any file, PDF, webpage, or generated material it reads.
- LLM-backed CLI mode: the CLI sends selected context to the configured provider or command.

If the agent is maintaining the repository rather than preparing a private application, inspect `examples/end_to_end/README.md` and run:

```bash
uv run pytest tests/test_examples.py -v
```

## Coordination Rules

1. Read `workflow.md` for the end-to-end sequence.
2. Read `job-lifecycle.md` to decide the next action from current job state.
3. Read `privacy.md` before reading full private sources, summarizing private files, staging, or committing.
4. Read `provider-config.md` before enabling `extract-profile-evidence --llm-augment`, `--llm-parser`, `--llm-drafts`, or command providers.
5. Read `quality-gates.md` before presenting materials as ready for review.
6. Prefer generated evidence and structured job artifacts before raw private sources.

Agents should coordinate through CLI commands and local files, not through hidden state.

## Consent Tiers

- Tier 0: workspace structure, `doctor`, public templates, prompts, schemas, and generated metadata. Agents may inspect these by default.
- Tier 1: generated evidence, `job.yaml`, `parsed_job.json`, and current job review artifacts. Agents may inspect these when needed for the current task.
- Tier 2: full CVs, statements, references, full job adverts, PDFs, source URLs, generated application packages, and institution-specific strategy. Ask first and state that agent-read content may enter the agent model context.
- Tier 3: LLM-backed CLI flags and command-provider runs. Ask first and state that selected private context may be sent to the configured provider or command.

## Suggested Agent Roles

Use separate agents only when the user explicitly asks for multi-agent work.

- Lead coordinator: runs `doctor`, declares the mode, identifies workspace/job state, chooses next command, and checks privacy boundaries.
- Lead scout: fetches jobs.ac.uk or generic RSS/Atom leads and summarizes candidate roles without crawling job pages.
- Evidence reviewer: checks `profile/generated/` coverage and reports gaps without editing private Typst sources.
- Source reviewer: after explicit approval, reads bounded private sources to repair or verify evidence gaps.
- Draft reviewer: checks fit report, cover letter, CV notes, and criteria checklist against quality gates.
- Typst reviewer: checks `typst/cover_letter.typ`, `typst/application_package.typ`, section markers, and optional PDF rendering.

When multiple agents are used, give each agent a bounded task and disjoint write scope. Do not have two agents edit the same job output file at the same time.

## Local Orchestrator Plans

Use `canisend orchestrate` when the user explicitly wants several local CLI workers to coordinate on one job:

```bash
canisend orchestrate \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --plan orchestration.yaml \
  --dry-run
```

Worker entries declare:

- `kind`: optional preset for `codex`, `claude`, `antigravity`, `agy`, or `custom`.
- `command`: optional local CLI command. This overrides the preset command when `kind` is set.
- `prompt_mode`: how the task prompt is delivered: `stdin`, `arg`, or `none`.
- `max_parallel_tasks`: maximum concurrent tasks for that worker.
- `supports_native_subagents`: whether that CLI can run several native subagents within one task.
- `privacy_tier_limit`: highest privacy tier the worker may receive.

Task entries declare `role`, `inputs`, `outputs`, `writes`, `depends_on`, `privacy_tier`, optional `agent_count`, and optional `edits_profile_input`. Use `agent_count` only when the worker supports native subagents and the task can safely split work internally. Keep `writes` disjoint unless an explicit dependency serializes the tasks.

## Profile Input Edit Tasks

Do not edit original profile sources as part of ordinary draft review. If repeated review shows a stable improvement to the source CV or statements, first produce job-folder suggestions. A source edit task must:

- set `edits_profile_input: true`
- write only the intended `profile/...` source path outside `profile/generated/`
- use `privacy_tier: 2` or higher
- depend on at least one prior review task
- be launched with `--allow-private-sources --allow-profile-input-edits --confirm-profile-input-edit --confirm-profile-input-edit-again`

```yaml
tasks:
  - id: profile-source-review
    worker: codex-reviewer
    role: profile_improvement_reviewer
    inputs: ["03_cover_letter_draft.md", "04_cv_tailoring_notes.md"]
    outputs: ["orchestration/reviews/profile-source-suggestions.md"]
    writes: ["orchestration/reviews/profile-source-suggestions.md"]
  - id: profile-source-edit
    worker: codex-reviewer
    role: profile_source_editor
    privacy_tier: 2
    inputs: ["orchestration/reviews/profile-source-suggestions.md", "profile/generated/cv.evidence.md"]
    outputs: ["profile/typst/cv.typ"]
    writes: ["profile/typst/cv.typ"]
    depends_on: ["profile-source-review"]
    edits_profile_input: true
```

## Handoff Format

Use this compact handoff when passing work between agents or tools:

```text
Workspace: <private-workspace>
Job: jobs/<job-slug>
Mode: direct-cli-deterministic | agent-assisted | llm-backed-cli
Current status: <job.yaml status or missing>
Last command run: <command>
Relevant files changed: <paths>
Private sources read directly: <paths or "none">
LLM-backed flags/providers used: <flags/provider or "none">
Next recommended action: <action>
Privacy notes: <any private files touched, or "none staged">
```

## Provider Coordination

The local command provider can point at Codex, Claude Code, or another CLI, but it is separate from
`canisend orchestrate`. Command-provider commands must read stdin and write stdout. Orchestrator
workers can instead use presets and `prompt_mode`, including `prompt_mode: arg` for CLIs that expect
the prompt as a command argument. Do not assume one provider exists; check config and ask the user
before using model-backed steps.

For command-provider tasks, prefer prompts that require JSON or evidence-cited Markdown output. Reject output that omits required citations when evidence exists.

Agent-assisted work and command-provider work are different boundaries. In agent-assisted work, the agent model can see whatever the agent reads. In command-provider work, the CLI sends a prompt to the configured provider or local command. Both require clear scope, but they are not the same execution path.

## Boundaries

Agents must not:

- commit `profile/`, `jobs/`, `job_leads/`, `.env`, PDFs, or real application material
- crawl job sites or scrape search-result pages
- submit applications or interact with portals
- answer sensitive declarations
- fabricate applicant experience, publications, teaching, service, grants, awards, or references

The Typst layer is structured. Agents may update `03_cover_letter_draft.md`, then directly edit bounded sections in `jobs/<job-slug>/typst/cover_letter.typ` or `jobs/<job-slug>/typst/application_package.typ`. Do not rewrite unrelated Typst sections.
