# Orchestrator Worker Presets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add preset-based worker configuration for Codex, Claude, and Antigravity/agy while preserving existing custom commands.

**Architecture:** Extend `WorkerConfig` with `kind` and `prompt_mode`, resolve preset defaults during plan parsing, and route prompt delivery in `_run_task`. Keep scheduling and artifact behavior unchanged.

**Tech Stack:** Python dataclasses, PyYAML plan parsing, `subprocess.run`, pytest.

---

### Task 1: Worker Preset Parsing

**Files:**
- Modify: `src/canisend/orchestrator.py`
- Test: `tests/test_orchestrator.py`

- [ ] **Step 1: Write failing tests**

Add tests for `kind: claude` without command, `kind: agy` and `kind: antigravity`, command override,
and unknown kind rejection.

- [ ] **Step 2: Verify tests fail**

Run: `uv run pytest tests/test_orchestrator.py::test_load_orchestration_plan_supports_claude_worker_preset_without_command tests/test_orchestrator.py::test_load_orchestration_plan_supports_antigravity_aliases tests/test_orchestrator.py::test_load_orchestration_plan_prefers_explicit_command_over_preset tests/test_orchestrator.py::test_load_orchestration_plan_rejects_unknown_worker_kind -q`

Expected: FAIL because `kind` and preset behavior are not implemented.

- [ ] **Step 3: Implement parser defaults**

Add a preset table, validate worker kind, populate default command and prompt mode, and keep explicit
command values authoritative.

- [ ] **Step 4: Verify parsing tests pass**

Run: `uv run pytest tests/test_orchestrator.py -q`

Expected: PASS.

### Task 2: Prompt Mode Execution

**Files:**
- Modify: `src/canisend/orchestrator.py`
- Test: `tests/test_orchestrator.py`

- [ ] **Step 1: Write failing tests**

Add tests showing `prompt_mode: arg` appends prompt text as an argument and `prompt_mode: none` only
writes `prompt.md`.

- [ ] **Step 2: Verify tests fail**

Run: `uv run pytest tests/test_orchestrator.py::test_run_orchestration_prompt_mode_arg_passes_prompt_as_argument tests/test_orchestrator.py::test_run_orchestration_prompt_mode_none_writes_prompt_without_passing_it -q`

Expected: FAIL because all workers currently receive prompt text through stdin.

- [ ] **Step 3: Implement prompt delivery**

Route subprocess invocation through a small helper that returns argv and stdin input for `stdin`,
`arg`, and `none`.

- [ ] **Step 4: Verify execution tests pass**

Run: `uv run pytest tests/test_orchestrator.py -q`

Expected: PASS.

### Task 3: Documentation

**Files:**
- Modify: `README.md`
- Modify: `skills/canisend/references/provider-config.md`
- Modify: `agent-skills/canisend/references/provider-config.md`

- [ ] **Step 1: Update examples**

Document `kind` presets, `prompt_mode`, and `command` overrides. Replace examples that imply Claude
must use one fixed command-line shape.

- [ ] **Step 2: Verify focused tests**

Run: `uv run pytest tests/test_orchestrator.py tests/test_cli.py -q`

Expected: PASS.

- [ ] **Step 3: Review diff**

Run: `git diff -- src/canisend/orchestrator.py tests/test_orchestrator.py README.md skills/canisend/references/provider-config.md agent-skills/canisend/references/provider-config.md`

Expected: Diff only contains worker preset implementation, tests, and related docs.
