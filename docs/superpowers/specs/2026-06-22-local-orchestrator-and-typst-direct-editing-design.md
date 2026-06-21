# Local Orchestrator and Typst Direct Editing Design

## Goal

Add a local orchestrator design for coordinating multiple agent CLIs while changing generated application content so
Typst files become the editable source of truth.

The design has two connected goals:

- coordinate Codex CLI, Claude Code, Antigravity, and other stdin/stdout-compatible agent CLIs through a local
  `canisend orchestrate` command;
- generate application-facing content directly in `.typ` files instead of making users edit JSON files that are later
  read by Typst shells.

## Context

CanISend already has agent-facing instructions, platform bridge files, a command-provider interface, and an
`agent-orchestration.md` reference. These are currently guidance documents, not a runnable coordinator.

The current Typst generation path writes:

- `typst/cover_letter_content.json`
- `typst/cover_letter.typ`
- `typst/application_package_content.json`
- `typst/application_package.typ`

The `.typ` files mostly read JSON values. That keeps rendering structured, but it makes manual editing awkward because
the user has to edit JSON-shaped content instead of the final Typst document.

## Non-Goals

This design does not add:

- hosted queues, remote workers, dashboards, or a long-running daemon;
- vendor-specific assumptions about Codex, Claude Code, Antigravity, or any other CLI;
- browser automation, portal submission, account creation, uploads, or sensitive declaration handling;
- automatic sharing of private CVs, statements, full adverts, PDFs, source URLs, or generated packages without the
  existing consent boundary;
- perfect semantic merge of conflicting agent edits.

## Recommended Approach

Use a plan-driven local orchestrator plus a Typst-first generation migration.

The orchestrator should be a normal CLI command:

```bash
canisend orchestrate \
  --workspace <private-workspace> \
  --job jobs/<job-slug> \
  --plan orchestration.yaml
```

It reads a local YAML plan, validates privacy and write scopes, runs eligible tasks concurrently, records artifacts under
the job folder, and leaves final decisions to the user.

The Typst migration should make generated `.typ` files the editable main outputs. JSON content files may remain as a
temporary compatibility artifact, but agents and users should edit `typst/*.typ` directly.

## Orchestrator Model

### Worker Adapters

Workers are configured as command templates, not hardcoded integrations:

```yaml
workers:
  codex:
    command: "codex exec"
    max_parallel_tasks: 2
    supports_native_subagents: true
    privacy_tier_limit: 1
  claude:
    command: "claude -p"
    max_parallel_tasks: 2
    supports_native_subagents: true
    privacy_tier_limit: 1
  antigravity:
    command: "antigravity run"
    max_parallel_tasks: 2
    supports_native_subagents: true
    privacy_tier_limit: 1
```

Each command must support one of these contracts:

- read a prompt from stdin and write the result to stdout;
- read a prompt from stdin and write to a task-specified output file;
- return non-zero on failure.

The project should not require a specific CLI to exist. Missing worker commands are reported as unavailable workers.

### Two-Level Parallelism

The orchestrator must support two levels of parallel execution:

1. **Across workers:** independent tasks can run at the same time on different CLIs, such as Codex, Claude Code, and
   Antigravity.
2. **Within a worker adapter:** each worker can have `max_parallel_tasks > 1`, allowing multiple independent task
   instances for the same CLI. This covers CLIs that can run multiple prompts in separate processes or expose their own
   multi-agent behavior.

The orchestrator does not need to know how a CLI internally implements multiple agents. It only needs a bounded worker
pool per adapter. If a CLI has a native multi-agent flag, the user can put that in the worker command template.

Tasks may also request a local subagent count:

```yaml
agent_count: 3
```

When `supports_native_subagents` is true, the adapter may pass this count through a command-template variable or include
it in the worker prompt. When it is false, the orchestrator treats `agent_count` as a scheduling hint and may split the
work into multiple normal task instances only when the task is declared splittable. The default is `agent_count: 1`.

### Tasks

Tasks declare role, inputs, outputs, dependencies, privacy tier, and write scope:

```yaml
tasks:
  - id: evidence-gap-review
    worker: codex
    role: evidence_reviewer
    privacy_tier: 1
    inputs:
      - parsed_job.json
      - profile/generated/*.evidence.md
    outputs:
      - orchestration/reviews/evidence-gap-review.md
    writes:
      - orchestration/reviews/evidence-gap-review.md

  - id: strict-hr-review
    worker: claude
    role: strict_university_hr_reviewer
    privacy_tier: 1
    depends_on:
      - evidence-gap-review
    inputs:
      - parsed_job.json
      - 05_criteria_checklist.md
      - 07_material_review_checklist.md
    outputs:
      - orchestration/reviews/strict-hr-review.md
    writes:
      - orchestration/reviews/strict-hr-review.md
```

Recommended built-in roles:

- `lead_coordinator`
- `job_parser_reviewer`
- `evidence_reviewer`
- `strict_university_hr_reviewer`
- `cover_letter_reviewer`
- `typst_editor`
- `typst_render_checker`

### Scheduling

The scheduler should:

- build a dependency graph from `depends_on`;
- reject cycles before running anything;
- run dependency-ready tasks concurrently;
- enforce each worker's `max_parallel_tasks`;
- prevent two tasks from writing the same path at the same time;
- treat missing required inputs as task failures;
- stop downstream dependent tasks when an upstream task fails;
- support a dry-run mode that prints the execution graph without launching workers.

### Run Artifacts

Each orchestration run writes to:

```text
jobs/<job-slug>/orchestration/runs/<run-id>/
  plan.yaml
  status.json
  tasks/
    <task-id>/
      prompt.md
      stdout.txt
      stderr.txt
      result.md
      status.json
```

The current review outputs can also be copied or symlinked to stable paths such as:

```text
jobs/<job-slug>/orchestration/reviews/
```

The run record should include:

- command invoked, with secrets redacted;
- started and finished timestamps;
- exit code;
- files read;
- files written;
- privacy tier;
- downstream tasks skipped due to failure.

## Privacy and Safety

Running `canisend orchestrate` is an agent-assisted multi-CLI mode. The selected task prompts and input excerpts may be
processed by the configured worker CLIs and their model providers. The command invocation is therefore the user's opt-in
to run the declared Tier 0/1 worker tasks in the plan. Higher-risk Tier 2 and Tier 3 tasks still require explicit flags.

The orchestrator must reuse CanISend's existing consent tiers:

- Tier 0: workspace structure, public prompts, templates, schemas, and generated metadata.
- Tier 1: generated evidence, `job.yaml`, `parsed_job.json`, and current review artifacts.
- Tier 2: full CVs, statements, references, full adverts, PDFs, source URLs, generated packages, and
  institution-specific strategy.
- Tier 3: LLM-backed CLI flags and command-provider runs that transmit selected context to a configured provider or
  command.

Plan validation should fail unless each task's declared privacy tier is within the worker's configured limit. To run a
Tier 2 or Tier 3 task, the user must explicitly opt in at the command level, for example:

```bash
canisend orchestrate ... --allow-private-sources
canisend orchestrate ... --allow-provider-backed
```

The orchestrator must not stage or commit private files. It should report private files present in git status if it
detects them, but it should not modify them unless a task explicitly owns that output path.

## Typst Direct Editing

### New Source of Truth

Generated Typst files become the editable main artifacts:

```text
jobs/<job-slug>/typst/cover_letter.typ
jobs/<job-slug>/typst/application_package.typ
```

The generated `.typ` files should contain the actual application text directly. They should not depend on
`json("cover_letter_content.json")` or `json("application_package_content.json")` for normal editing.

### Section Markers

Generated `.typ` files should include stable markers that agents can target:

```typst
// CANISEND: section opening
I am writing to apply for ...

// CANISEND: section research_fit
...

// CANISEND: section teaching_fit
...

// CANISEND: section departmental_contribution
...

// CANISEND: section service_leadership
...
```

For the application package:

```typst
// CANISEND: section job_information
// CANISEND: section fit_report
// CANISEND: section cover_letter
// CANISEND: section cv_tailoring_notes
// CANISEND: section criteria_checklist
// CANISEND: section remaining_actions
```

Agents should edit bounded marker sections instead of rewriting the whole Typst file.

### Typst Escaping and Markdown Conversion

The generator should convert Markdown-derived content into Typst-safe content blocks:

- headings become Typst headings;
- bullet lists become Typst lists;
- evidence citations remain visible as literal text or comments;
- unsupported Markdown is preserved as plain text rather than silently dropped;
- Typst-special characters in generated prose are escaped when needed.

The goal is editable and renderable Typst, not a perfect Markdown-to-Typst converter.

### Compatibility

During migration, the pipeline may keep writing content JSON files as compatibility/debug artifacts. They should be
documented as secondary outputs, not the editing surface.

Existing references and tests that call JSON the contract should be updated to describe `.typ` as the primary contract.

## Data Flow

```text
parsed_job.json + generated materials + profile/generated evidence
        |
        v
canisend run
        |
        v
editable typst/*.typ files + review artifacts
        |
        v
canisend orchestrate
        |
        v
parallel CLI reviews and bounded Typst edits
        |
        v
render/check-package/manual review
```

## Error Handling

The orchestrator should fail before launching workers when:

- the plan is invalid YAML;
- task IDs are duplicated;
- dependencies reference missing tasks;
- dependency cycles exist;
- two concurrently eligible tasks write the same file without an explicit dependency;
- a task requests a privacy tier above its worker limit;
- a required input is missing;
- a worker command is unavailable.

During execution:

- worker timeout marks the task failed;
- non-zero exit marks the task failed and records stderr;
- dependent tasks are skipped;
- unrelated dependency-ready tasks may continue unless the user passes a fail-fast flag;
- partial outputs stay in the run artifact folder and are not promoted to stable review paths unless the task succeeds.

Typst direct generation should fail clearly when generated text cannot be converted into a renderable `.typ` block.
Rendering errors should point to the `.typ` file and line when the Typst compiler reports one.

## Testing

Use test-first implementation for behavior changes.

Required orchestrator tests:

- invalid plans fail before launching worker commands;
- dependency graph runs tasks in the right order;
- independent tasks run concurrently up to worker limits;
- one worker adapter supports multiple parallel task instances;
- write-scope conflicts are rejected or serialized by explicit dependencies;
- failed upstream tasks skip downstream tasks;
- run artifacts include prompts, stdout, stderr, status, privacy tier, and written files;
- Tier 2/Tier 3 tasks require explicit opt-in flags;
- missing worker commands fail clearly.

Required Typst migration tests:

- generated cover letter `.typ` contains direct editable content and no `json("cover_letter_content.json")`;
- generated application package `.typ` contains direct editable content and no `json("application_package_content.json")`;
- section markers are stable and present;
- Markdown bullets and headings convert to renderable Typst;
- evidence citations remain visible;
- existing render command targets the direct `.typ` files;
- docs and skill references describe `.typ` as the editing surface.

Run at least:

```bash
uv run pytest tests/test_typst_mapping.py tests/test_pipeline.py tests/test_typst.py -q
uv run pytest tests/test_skill_distribution.py tests/test_repository_contract.py -q
uv run pytest -q
```

## Documentation Updates

Update README and shared references to state:

- `canisend orchestrate` can coordinate multiple local agent CLIs;
- worker commands are user-configured and not vendor-specific;
- orchestration can run tasks in parallel across workers and within one worker adapter;
- privacy tiers still apply to every task;
- `.typ` files are the primary editable application artifacts;
- JSON content files, if still emitted, are compatibility/debug outputs only.

## Release Boundaries

The change must preserve:

- local-first deterministic generation when orchestration is not used;
- explicit opt-in for private-source and provider-backed tasks;
- current no-submission boundary;
- existing generated Markdown review artifacts;
- Typst rendering through local `typst` only;
- no assumption that Codex, Claude Code, Antigravity, or any other CLI is installed.
