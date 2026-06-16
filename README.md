<p align="center">
  <img src="assets/canisend-logo.svg" alt="这也能投 logo" width="132">
</p>

<p align="center">
  <a href="https://github.com/jxpeng98/CanISend/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/jxpeng98/CanISend/ci.yml?branch=main&label=ci" alt="CI status"></a>
  <a href="https://test.pypi.org/project/canisend/"><img src="https://img.shields.io/badge/TestPyPI-0.2.0b5-blue" alt="TestPyPI"></a>
  <img src="https://img.shields.io/badge/python-3.11%2B-blue" alt="Python 3.11+">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="MIT license">
</p>

# 这也能投/CanISend

Evidence-backed application prep for academic and professional jobs. CLI/package name: `canisend`.

这也能投是一款 local-first CLI：从职位广告和本地私人履历证据出发，生成可检查的申请材料包，包括 parsed criteria、fit report、cover letter draft、CV tailoring notes、material checklist 和 Typst-ready source。强主张应当能追溯到本地证据。

It prepares materials only. It does not submit applications, create accounts, fill portals, scrape full job pages, upload packages, or answer sensitive declarations.

## What It Does

- Creates a private workspace for profile evidence, job folders, prompts, Typst templates, schemas, and agent instructions.
- Imports and filters jobs.ac.uk RSS leads without scraping full job pages.
- Creates one local folder per application, with the full advert kept as a manual input.
- Extracts normalized evidence from Typst-first profile sources into `profile/generated/`.
- Generates `parsed_job.json`, fit reports, cover letter drafts, CV tailoring notes, criteria checklists, material review checklists, and structured Typst content.
- Runs deterministic local generation by default, with explicit opt-in for an LLM-backed parser or LLM-backed drafts.
- Ships bridge files for Codex, Claude Code, Gemini, and IDE agents through `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, and `agent-skills/canisend/SKILL.md`.

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

### Install the current TestPyPI beta

For the current beta, install `canisend==0.2.0b5` from TestPyPI while still resolving dependencies from PyPI:

```bash
uv tool install \
  --index-url https://test.pypi.org/simple/ \
  --extra-index-url https://pypi.org/simple/ \
  canisend==0.2.0b5
```

Or with `pip` inside an active virtual environment:

```bash
python -m pip install \
  --index-url https://test.pypi.org/simple/ \
  --extra-index-url https://pypi.org/simple/ \
  canisend==0.2.0b5
```

Run the packaged fake-data workflow before using private profile or job data:

```bash
canisend run-example --workspace /tmp/canisend-example --overwrite
```

Inspect the generated dossier:

```text
/tmp/canisend-example/jobs/2026-06-15_example-university_lecturer-in-applied-economics/
  parsed_job.json
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
  GEMINI.md
  profile/
  jobs/
  job_leads/
  prompts/
  templates/
  schemas/
  agent-skills/
```

After upgrades:

```bash
uv tool upgrade canisend
canisend update-workspace --workspace ~/CanISendWorkspace
canisend doctor --workspace ~/CanISendWorkspace
```

`update-workspace` preserves local prompt, template, and skill edits by default. Use `--overwrite` only when packaged defaults should replace local copies.

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

Create a job folder from a selected zero-based lead index:

```bash
canisend new-job-from-lead \
  --workspace ~/CanISendWorkspace \
  --lead-index 0 \
  --institution "University X" \
  --deadline "2026-06-15"
```

You can also create a job manually:

```bash
canisend new-job \
  --workspace ~/CanISendWorkspace \
  --title "Lecturer in Economics" \
  --institution "University X" \
  --deadline "2026-06-15" \
  --source-url "https://www.jobs.ac.uk/job/example"
```

Paste the full advert into `jobs/<job-slug>/job_advert.md` before relying on parsed criteria or generated drafts. V1 does not scrape full job pages.

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
  01_job_summary.md
  02_fit_report.md
  03_cover_letter_draft.md
  04_cv_tailoring_notes.md
  05_criteria_checklist.md
  06_final_application_package.md
  07_material_review_checklist.md
  typst/
    cover_letter_content.json
    cover_letter.typ
    application_package_content.json
    application_package.typ
```

LLM-backed parser and draft generation are explicit opt-in modes. Configure a provider before using them:

```bash
ACADEMIC_PREP_LLM_PROVIDER=openai-compatible
OPENAI_API_KEY=...
OPENAI_BASE_URL=https://api.openai.com/v1
OPENAI_MODEL=...

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

The LLM-backed parser must return JSON matching the `parsed_job.json` contract. Draft outputs must cite profile evidence; unknown citations fail validation. Missing evidence should be marked as a gap, not replaced with unsupported claims.

### 5. Review, render, and submit manually

Review files in this order:

1. `parsed_job.json`
2. `05_criteria_checklist.md`
3. `02_fit_report.md`
4. `03_cover_letter_draft.md`
5. `04_cv_tailoring_notes.md`
6. `07_material_review_checklist.md`
7. `typst/cover_letter_content.json`
8. `06_final_application_package.md`

Use `07_material_review_checklist.md` to track the cover letter draft, CV tailoring notes, placeholders, item-level evidence citation checks, and next manual actions.

Render Typst only when needed:

```bash
canisend render-typst \
  --workspace ~/CanISendWorkspace \
  --job jobs/<job-slug>
```

Rendering requires a local `typst` binary. Source generation does not. Submit manually through the institution portal outside this tool.

## Agent Usage

Codex, Claude Code, Gemini, and other local agents can run the `canisend` workflow by opening the private workspace as the project root.

- Codex and AGENTS.md-aware tools should read `AGENTS.md`.
- Claude Code should read `CLAUDE.md`, which imports `agent-skills/canisend/SKILL.md`.
- Gemini CLI should read `GEMINI.md`.
- IDE agents can read any bridge file and then `agent-skills/canisend/SKILL.md`.

Agents should start with:

```bash
canisend doctor --workspace .
```

They may run local CLI commands, inspect generated evidence, and review current job artifacts. They must ask first before reading full private CVs, full job adverts, references, source URLs, or enabling LLM-backed flags. They must not scrape pages, submit applications, upload packages, fabricate evidence, or commit private profile/job data.

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

`prompts/` contains LLM prompt files used by the application pipeline. `agent-skills/` contains agent-readable workflow and quality guidance.

## Skill Distribution

The project also ships a reusable skill pack for cases where you want a narrower agent behavior without opening a CanISend workspace as the active project. The root Codex plugin manifest at `.codex-plugin/plugin.json` exposes `skills/` as a Codex plugin manifest, while the original `agent-skills/canisend/` workspace skill remains unchanged.

Use the main `canisend` skill for full workspace and job-package workflows. Use material-specific skills for focused tasks such as `canisend-research-statement`, `canisend-cover-letter`, `canisend-cv-tailoring`, `canisend-criteria-check`, and `canisend-material-review`.

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
- `jobs/` generated job folders are ignored by git.
- `job_leads/` RSS outputs are ignored by git.
- `.env`, API keys, rendered PDFs, real source URLs, and generated application packages should not be committed.
- Sensitive declarations such as right-to-work, visa, disability, equality monitoring, health, criminal record, and conflicts remain user-only.

这也能投只是材料准备工具，不是提交凭证。

## Maintainer Release

Release automation lives in GitHub Actions for `jxpeng98/CanISend`.

Local checks:

```bash
uv run pytest -v
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
scripts/release.sh test --version 0.2.0.dev1
scripts/release.sh beta --version 0.2.0b1
scripts/release.sh stable --version 0.2.0
```

The script updates `pyproject.toml` and `src/canisend/__init__.py`, runs local checks, commits the version bump, pushes the current branch, then creates and pushes the matching git tag:

```bash
git tag -a test/v0.2.0.dev1 HEAD -m "canisend 0.2.0.dev1 TestPyPI"
git tag -a v0.2.0b1 HEAD -m "canisend 0.2.0b1 beta"
git tag -a v0.2.0 HEAD -m "canisend 0.2.0 stable"
```

`release.yml` is the only remote publisher. It always publishes to TestPyPI first, and only promotes `v*` tags to beta or stable PyPI after the TestPyPI publish and smoke test succeed.
Use `test/v*` with a disposable version because TestPyPI package versions cannot be overwritten.

Use `RELEASE.md` for the full TestPyPI and PyPI release playbook. Version updates must change both `pyproject.toml` and `src/canisend/__init__.py`.

## Repository Layout

```text
src/canisend/             CLI and application pipeline
prompts/                  LLM prompt templates
templates/typst/          modernpro Typst templates
schemas/                  JSON schema contracts
agent-skills/             canisend skill and agent references
skills/                   reusable Codex and Claude skill pack
.codex-plugin/            Codex plugin manifest for the skill pack
platform-bridges/         AGENTS.md, CLAUDE.md, GEMINI.md workspace bridges
examples/end_to_end/      fully local fake-data workflow
tests/                    CLI, pipeline, packaging, release, and contract tests
assets/                   project logo and README media
RELEASE.md                maintainer release playbook
```

See `canisend_v1_proposal.md` for the original V1 engineering proposal.
