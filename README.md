<p align="center">
  <img src="assets/canisend-logo.svg" alt="这也能投 logo" width="132">
</p>

<p align="center">
  <a href="https://github.com/jxpeng98/CanISend/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/jxpeng98/CanISend/ci.yml?branch=main&label=ci" alt="CI status"></a>
  <a href="https://test.pypi.org/project/canisend/"><img src="https://img.shields.io/badge/TestPyPI-0.2.0-blue" alt="TestPyPI"></a>
  <img src="https://img.shields.io/badge/python-3.11%2B-blue" alt="Python 3.11+">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="MIT license">
</p>

# 这也能投/CanISend

Evidence-backed application prep for academic and professional jobs. CLI/package name: `canisend`.

这也能投是一款 local-first CLI：从职位广告和本地私人履历证据出发，生成可检查的申请材料包，包括 parsed criteria、fit report、cover letter draft、CV tailoring notes、material checklist 和 Typst-ready source。强主张应当能追溯到本地证据。

It prepares materials only. It does not submit applications, create accounts, fill portals, crawl job sites, upload packages, or answer sensitive declarations. A user may explicitly import one supplied advert URL; CanISend does not run background page discovery or broad scraping.

## What It Does

- Creates a private workspace for profile evidence, job folders, prompts, Typst templates, schemas, and agent instructions.
- Imports and filters jobs.ac.uk RSS plus generic RSS/Atom leads without crawling full job pages.
- Creates one local folder per application, with the full advert kept as a manual input.
- Extracts normalized evidence from Typst-first profile sources into `profile/generated/`.
- Generates `parsed_job.json`, preparation questions, fit reports, cover letter drafts, CV tailoring notes, criteria checklists, material review checklists, and structured Typst content.
- Runs deterministic local generation by default, with explicit opt-in for LLM-backed evidence augmentation, parsing, or drafting.
- Ships bridge files plus a self-contained workspace skill pack for Codex, Claude Code, and IDE agents.

## Quick Start

Choose one installation method. CanISend requires Python 3.11 or newer.

### Install as an isolated CLI with `uv`

Use this after the production release is available on PyPI:

```bash
uv tool install canisend
```

### Install as an isolated CLI with `pipx`

If you prefer the `pip` ecosystem but still want an isolated command-line tool:

```bash
pipx install canisend
```

### Install with `pip` in a virtual environment

Use this when you want CanISend available inside a project-specific Python environment:

```bash
python -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install canisend
```

### Install the current TestPyPI build

For the current TestPyPI build, install `canisend==0.2.0` while still resolving dependencies from PyPI:

```bash
uv tool install \
  --index-url https://test.pypi.org/simple/ \
  --extra-index-url https://pypi.org/simple/ \
  canisend==0.2.0
```

Or with `pip` inside an active virtual environment:

```bash
python -m pip install \
  --index-url https://test.pypi.org/simple/ \
  --extra-index-url https://pypi.org/simple/ \
  canisend==0.2.0
```

Run the packaged fake-data workflow before using private profile or job data:

```bash
canisend run-example --workspace /tmp/canisend-example --overwrite
```

Inspect the generated dossier:

```text
/tmp/canisend-example/jobs/2026-06-15_example-university_lecturer-in-applied-economics/
  parsed_job.json
  00_preparation_questions.md
  02_fit_report.md
  03_cover_letter_draft.md
  05_criteria_checklist.md
  07_material_review_checklist.md
  typst/
```

From a development checkout, prefix CLI commands with `uv run`:

```bash
uv run canisend --help
uv run canisend run-example --workspace /tmp/canisend-example --overwrite
```

## Core Workflow

```text
private profile -> generated evidence -> job folder -> parsed criteria
      |                                      |
      v                                      v
item-level receipts                 draft materials + review checklist
      |                                      |
      +------------ manual review -----------+
```

### 1. Initialize a private workspace

Normal users should install the CLI and keep private application data in a separate workspace. They do not need to fork this repository.

```bash
canisend init-workspace --workspace ~/CanISendWorkspace
canisend doctor --workspace ~/CanISendWorkspace
```

The workspace contains private profile data, job leads, job folders, editable prompt copies, Typst templates, schemas, examples, and agent-readable skills:

```text
~/CanISendWorkspace/
  canisend.yaml
  .env.example
  .gitignore
  AGENTS.md
  CLAUDE.md
  profile/
  jobs/
  job_leads/
  prompts/
  templates/
  schemas/
  agent-skills/
    canisend/
    canisend-job-intake/
    canisend-application-package/
    canisend-submission-readiness/
```

After upgrades:

```bash
uv tool upgrade canisend
canisend update-workspace --workspace ~/CanISendWorkspace
canisend doctor --workspace ~/CanISendWorkspace
```

`update-workspace` preserves local prompt, template, and skill edits by default. Use `--overwrite` only when packaged defaults should replace local copies.
If `doctor` reports deprecated packaged files from an older release, remove only those retired defaults with:

```bash
canisend update-workspace --workspace ~/CanISendWorkspace --prune-deprecated
```

`doctor` also reports local default resources that differ from the packaged version. Use `update-workspace --overwrite` only after deciding those local prompt, template, schema, or bridge edits should be replaced.

### 2. Prepare profile evidence

Put your real modernpro CV and statements under `~/CanISendWorkspace/profile/typst/`. These files stay local, and `profile/` is ignored by git except for `.gitkeep`.

Create starter profile files if needed:

```bash
canisend init-profile --workspace ~/CanISendWorkspace --mode typst
```

Generate normalized evidence:

```bash
canisend extract-profile-evidence --workspace ~/CanISendWorkspace
```

When local Typst extraction misses evidence, you can explicitly opt into provider-backed augmentation:

```bash
canisend extract-profile-evidence \
  --workspace ~/CanISendWorkspace \
  --llm-augment
```

The profile manifest lives at `profile/profile.yaml`. Generated evidence is written to `profile/generated/` and cited with item-level references such as:

```text
profile/generated/cv.evidence.md#Teaching/cv-001
```

Review item-level evidence citations before trusting any generated claim.

### 3. Import leads and create one job folder

Fetch jobs.ac.uk RSS leads locally:

```bash
canisend fetch-jobs-ac-uk \
  --workspace ~/CanISendWorkspace \
  --feed-url "<jobs.ac.uk RSS url>" \
  --include economics \
  --exclude phd
```

Any stable RSS or Atom source can use the generic entrypoint:

```bash
canisend fetch-job-feed \
  --workspace ~/CanISendWorkspace \
  --source-name "Example University" \
  --feed-url "https://example.edu/jobs.atom" \
  --include economics \
  --exclude phd
```

Generic feeds default to `job_leads/<source-name>.json`. They use the same normalized lead fields and local filters as
jobs.ac.uk. Feed records are discovery leads, not full adverts.

Create a job folder from a selected zero-based lead index:

```bash
canisend new-job-from-lead \
  --workspace ~/CanISendWorkspace \
  --lead-index 0 \
  --institution "University X" \
  --deadline "2026-06-15"
```

For a generic feed output, pass its lead file explicitly:

```bash
canisend new-job-from-lead \
  --workspace ~/CanISendWorkspace \
  --leads-file job_leads/example-university.json \
  --lead-index 0 \
  --institution "Example University"
```

You can also create a job manually:

```bash
canisend new-job \
  --workspace ~/CanISendWorkspace \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --deadline "2026-06-15" \
  --english-variant "UK English" \
  --writing-style "direct, warm, evidence-led" \
  --source-url "https://www.jobs.ac.uk/job/example"
```

If these preferences are unknown, leave them unset. CanISend will mark them as `needs_confirmation` and include language/style questions in `00_preparation_questions.md`.

Paste the full advert, or import it from a supported local file, into `jobs/<job-slug>/job_advert.md` before relying on parsed criteria or generated
drafts. A user-provided PDF is a first-class intake path:

```bash
canisend new-job \
  --workspace ~/CanISendWorkspace \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --advert-file ~/Downloads/lecturer-job.pdf
```

`--source-url` records metadata only. When the user explicitly wants one supplied URL imported, add `--fetch-url`;
the response may be HTML or PDF. This bounded one-URL import is different from crawling or searching a site:

```bash
canisend new-job \
  --workspace ~/CanISendWorkspace \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --source-url "https://example.edu/jobs/123" \
  --fetch-url
```

### 4. Generate draft materials

Run the deterministic local pipeline:

```bash
canisend run \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug>
```

Generated output includes:

```text
jobs/<job-slug>/
  parsed_job.json
  00_preparation_questions.md
  01_job_summary.md
  02_fit_report.md
  03_cover_letter_draft.md
  04_cv_tailoring_notes.md
  05_criteria_checklist.md
  06_final_application_package.md
  07_material_review_checklist.md
  typst/
    cover_letter.typ
    application_package.typ
```

Generated Typst files are the editable source of truth for final formatting. Content JSON files may still be emitted as compatibility/debug artifacts, but normal edits should happen in the `.typ` files.

CanISend records hashes of its generated Typst baseline. On a later `run`, an unchanged source updates normally. If a
source has been edited, the user version is preserved and the new generation is written as `*.generated.typ` for
review instead of silently overwriting the edit. While a candidate is pending, `check-package` and `render-typst`
refuse to proceed, and `run --git-add-materials` skips staging so Markdown and Typst cannot be recorded as a mismatched
set.

To track edits to generated application materials in a private git repository, opt in after generation:

```bash
canisend run \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug> \
  --git-add-materials
```

Without the flag, interactive terminals are asked whether to add generated application materials to git; non-interactive runs skip git staging unless the flag is set. CanISend stages only the generated preparation questions, fit report, cover letter draft, CV tailoring notes, criteria checklist, final package, material review checklist, and editable Typst sources. It does not stage raw adverts, source URLs, parsed job JSON, compatibility JSON, PDFs, or profile files, and it never commits automatically.

LLM-backed evidence augmentation, parser, and draft generation are explicit opt-in modes. Configure a provider before using them:

```bash
ACADEMIC_PREP_LLM_PROVIDER=openai-compatible
OPENAI_API_KEY=...
OPENAI_BASE_URL=https://api.openai.com/v1
OPENAI_MODEL=...

canisend extract-profile-evidence \
  --workspace ~/CanISendWorkspace \
  --llm-augment

canisend run \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug> \
  --llm-parser \
  --llm-drafts
```

For local CLI model access, use the command provider:

```bash
ACADEMIC_PREP_LLM_PROVIDER=command
ACADEMIC_PREP_LLM_COMMAND="codex exec --json"
ACADEMIC_PREP_LLM_TIMEOUT_SECONDS=300
```

The LLM-backed evidence augmenter only accepts items tied to a supporting `source_text` found in the local profile source. The LLM-backed parser must return JSON matching the `parsed_job.json` contract. Draft outputs must cite profile evidence; unknown citations fail validation. Missing evidence should be marked as a gap, not replaced with unsupported claims.

### 5. Review, render, and submit manually

Review files in this order:

1. `parsed_job.json`
2. `00_preparation_questions.md`
3. `05_criteria_checklist.md`
4. `02_fit_report.md`
5. `03_cover_letter_draft.md`
6. `04_cv_tailoring_notes.md`
7. `07_material_review_checklist.md`
8. `typst/cover_letter.typ`
9. `typst/application_package.typ`
10. `06_final_application_package.md`

Use `00_preparation_questions.md` to confirm US English vs UK English, writing style, and the grill-me details that make the application specific. Use `07_material_review_checklist.md` to track the cover letter draft, CV tailoring notes, placeholders, item-level evidence citation checks, and next manual actions.

After reviewing the Markdown drafts, directly edit `typst/cover_letter.typ` and `typst/application_package.typ` for final wording and layout. The files include stable `// CANISEND: section ...` markers so agents can make bounded edits without rewriting the whole Typst source.

Run a read-only package check before treating generated materials as ready for user review:

```bash
canisend check-package \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug>
```

The check reports incomplete advert or parsed-job inputs, missing or structurally incomplete Typst sources, explicit review blockers,
unresolved bracketed placeholders, and unknown profile evidence citations. It does not modify files unless an explicit
machine-readable gate report is requested:

```bash
canisend check-package \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug> \
  --write-report
```

The report records safe relative input labels and SHA-256 hashes. A later `canisend run` marks an existing report
`STALE`, so it cannot be mistaken for a check of regenerated materials.

Render Typst only when needed:

```bash
canisend render-typst \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug>
```

Rendering requires a local `typst` binary. Source generation does not. Submit manually through the institution portal outside this tool.

## Agent Usage

Codex, Claude Code, and IDE agents can run the `canisend` workflow by opening the private workspace as the project root. This is an agent-assisted workflow, not a local-only workflow: any file, PDF, webpage, or generated material the agent reads or summarizes may be processed by the agent model provider.

- Codex and AGENTS.md-aware tools should read `AGENTS.md`.
- Claude Code should read `CLAUDE.md`, which imports `agent-skills/canisend/SKILL.md`.
- IDE agents can read `AGENTS.md` and then `agent-skills/canisend/SKILL.md`.

Agents should start with:

```bash
canisend agent context --workspace . --format json
```

Inspect the versioned runtime contract before choosing an operation:

```bash
canisend agent capabilities --format json
```

Phase 1 provides JSON output for `doctor`, `new-job`, `new-job-from-lead`, `list-jobs`, and `check-package`. Each
successful invocation writes one `canisend.agent/v1` response to stdout. Responses use workspace-relative or opaque
artifact references and hashes rather than private document bodies. A failed package gate remains a successful
operation result (`ok: true`) but exits non-zero; operational failures return `ok: false` with a stable error code.

Use `canisend doctor --workspace .` when a human-readable environment diagnostic is also useful.

A fresh Codex, Claude Code, or IDE shell session resumes from the same durable workspace state by running the same
`agent context` command; it does not need the previous chat transcript. The fake-data conformance fixture under
`examples/agent_handoff/` demonstrates this host-neutral handoff. Phase 2 adds semantic task/result exchange and local
MCP transport.

They may run local deterministic CLI commands, inspect generated evidence, and review current job artifacts. They must ask first before reading full private CVs, statements, full job adverts, references, PDFs, source URLs, generated packages, or enabling LLM-backed CLI flags/providers. They must not scrape pages, submit applications, upload packages, fabricate evidence, or commit private profile/job data.

Original profile inputs under `profile/` are protected. Agents should normally produce profile-improvement suggestions inside the job folder, not rewrite the source CV or statements. A write back to `profile/typst/*.typ`, `profile/*.md`, or other non-generated profile input is allowed only through an orchestrator task that declares `edits_profile_input: true`, depends on a prior review task, uses privacy tier 2 or higher, and is launched with `--allow-profile-input-edits --confirm-profile-input-edit --confirm-profile-input-edit-again`.

For coordinated multi-CLI review, use `canisend orchestrate` with an explicit local YAML plan:

```bash
canisend orchestrate \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug> \
  --plan orchestration.yaml \
  --dry-run
```

Worker entries declare either `kind` or `command`. Built-in `kind` values are `codex`, `claude`, `antigravity`, `agy`, and `custom`; `antigravity` and `agy` share the same default command. Use `command` to override the preset for the CLI installed on your machine, and use `prompt_mode` to choose how the generated task prompt is delivered: `stdin`, `arg`, or `none`.

```yaml
workers:
  codex-reviewer:
    kind: codex
  claude-reviewer:
    kind: claude
    prompt_mode: arg
  agy-reviewer:
    kind: agy
    command: "agy --print"
```

Worker entries also support `max_parallel_tasks`, `supports_native_subagents`, and `privacy_tier_limit`. Task entries declare `role`, `inputs`, `outputs`, `writes`, `depends_on`, `privacy_tier`, optional `agent_count`, and optional `edits_profile_input` for tightly controlled original profile source edits. The orchestrator runs dependency-ready tasks in parallel, enforces worker concurrency limits, writes run artifacts under `jobs/<job-slug>/orchestration/runs/`, and requires `--allow-private-sources`, `--allow-provider-backed`, or the profile-input edit confirmation flags for higher-risk tasks.

Privacy modes:

- Direct CLI deterministic mode can be local-only when no agent reads private content and no LLM flags/providers are used.
- Agent-assisted mode means agent-read content may enter the agent model context.
- LLM-backed CLI mode means selected context may be sent to the configured provider or local command through flags such as `extract-profile-evidence --llm-augment`, `--llm-parser`, or `--llm-drafts`.

Detailed agent guidance lives in:

```text
agent-skills/canisend/
  SKILL.md
  references/
    workflow.md
    job-lifecycle.md
    file-contracts.md
    typst-profile.md
    provider-config.md
    quality-gates.md
    agent-orchestration.md
    platforms.md
    privacy.md
```

`prompts/` contains LLM prompt files used by the application pipeline. Workspace-local `agent-skills/` contains the
complete main and focused skill pack, so routes from the main skill do not depend on a global installation.

## Skill Distribution

The project also ships a reusable skill pack for cases where you want a narrower agent behavior without opening a
CanISend workspace as the active project. `skills/` is the canonical pack used by exports and workspace updates. The
root Codex plugin manifest at `.codex-plugin/plugin.json` exposes that pack, while `agent-skills/canisend/` remains a
one-release compatibility mirror of the canonical main skill.

Use the main `canisend` skill for full workspace workflows. Use `canisend-job-intake` for source-to-advert intake,
`canisend-application-package` for coordinated package construction, and `canisend-submission-readiness` for the final
manual-submission gate. Material-focused skills cover job fit, research and teaching statements, cover letters, CV
tailoring, humanization, application email, interview preparation, criteria checks, and material review.

The material-focused skill IDs are `canisend-job-fit`, `canisend-research-statement`,
`canisend-teaching-statement`, `canisend-cover-letter`, `canisend-cv-tailoring`, `canisend-humanizer`,
`canisend-application-email`, `canisend-interview-prep`, `canisend-criteria-check`, and `canisend-material-review`.

For a Codex marketplace repository, mount this repository as the plugin source:

```bash
git submodule add https://github.com/jxpeng98/CanISend plugins/canisend
```

Then add a marketplace entry that points to `./plugins/canisend`.

To export skills from an installed package instead of a checkout:

```bash
canisend export-skills --target ~/plugins/canisend --kind codex-plugin
canisend export-skills --target ~/.claude/skills --kind skills-only
```

`codex-plugin` writes `.codex-plugin/` plus `skills/`. `skills-only` writes only the skill folders for agents that install skills directly.

## Privacy Boundaries

This repository is intended to be open source. Personal application data should stay local:

- `profile/` is ignored by git except for `.gitkeep`.
- `jobs/` generated job folders are ignored by git unless selected generated materials are explicitly staged with `canisend run --git-add-materials`.
- `job_leads/` feed outputs are ignored by git.
- `.env`, API keys, rendered PDFs, raw job adverts, real source URLs, parsed job JSON, and profile files should not be committed.
- Sensitive declarations such as right-to-work, visa, disability, equality monitoring, health, criminal record, and conflicts remain user-only.

这也能投只是材料准备工具，不是提交凭证。

## Maintainer Release

Release automation lives in GitHub Actions for `jxpeng98/CanISend`.

Local checks:

```bash
uv run python -m pytest -v
uv build
uvx twine check dist/*
uv run python -m canisend.package_check dist/*.whl
```

CI runs the same test/build/resource-check sequence on pushes and pull requests. The release workflow uses PyPI Trusted Publishing with OIDC:

- Pushing `test/v<version>` publishes to TestPyPI only.
- Pushing `v<version>bN` or `v<version>rcN` publishes to TestPyPI, smoke-tests the TestPyPI package, then publishes a PyPI prerelease and creates a GitHub prerelease.
- Pushing `v<version>` publishes to TestPyPI, smoke-tests the TestPyPI package, then publishes a stable PyPI release and creates a GitHub Release.
- TestPyPI and PyPI need a Trusted Publisher for `.github/workflows/release.yml` with environments named `testpypi` and `pypi`.

Preferred tag-driven release orchestration:

```bash
scripts/release.sh test --version 0.3.0.dev1
scripts/release.sh beta --version 0.3.0a1
scripts/release.sh stable --version 0.3.0
```

The script updates `pyproject.toml` and `src/canisend/__init__.py`, runs local checks, commits the version bump, pushes the current branch, then creates and pushes the matching git tag:

```bash
git tag -a test/v0.3.0.dev1 HEAD -m "canisend 0.3.0.dev1 TestPyPI"
git tag -a v0.3.0a1 HEAD -m "canisend 0.3.0a1 prerelease"
git tag -a v0.3.0 HEAD -m "canisend 0.3.0 stable"
```

`release.yml` is the only remote publisher. It always publishes to TestPyPI first, and only promotes `v*` tags to
prerelease or stable PyPI after the TestPyPI publish and smoke test succeed. The release script verifies the candidate
commit on its remote branch before tagging; stable releases must be reachable from `origin/main`. Published tags,
including `v0.2.0`, are never reused.
Use `test/v*` with a disposable version because TestPyPI package versions cannot be overwritten.

Use `RELEASE.md` for the full TestPyPI and PyPI release playbook. Version updates must change both `pyproject.toml` and `src/canisend/__init__.py`.

## Repository Layout

```text
src/canisend/             CLI and application pipeline
prompts/                  LLM prompt templates
templates/typst/          modernpro Typst templates
schemas/                  JSON schema contracts
agent-skills/             one-release compatibility mirror of the main skill
skills/                   canonical reusable and workspace-installed skill pack
.codex-plugin/            Codex plugin manifest for the skill pack
platform-bridges/         AGENTS.md and CLAUDE.md workspace bridges
examples/end_to_end/      fully local fake-data workflow
examples/agent_handoff/   host-neutral agent contract fixtures
tests/                    CLI, pipeline, packaging, release, and contract tests
assets/                   project logo and README media
RELEASE.md                maintainer release playbook
```

See `canisend_v1_proposal.md` for the original V1 engineering proposal,
`docs/superpowers/specs/2026-07-09-discovery-and-workflow-v2-design.md` for detailed multi-source and stage-hardening
constraints, and `docs/superpowers/specs/2026-07-10-agent-native-workflow-roadmap.md` for the current agent-native
delivery roadmap. Phase 1 is specified in
`docs/superpowers/plans/2026-07-10-agent-runtime-contract-foundation.md`.
