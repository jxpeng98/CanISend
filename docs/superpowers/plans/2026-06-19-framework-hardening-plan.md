# Framework Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Strengthen CanISend's current CLI, workspace, and agent-skill framework after removing retired Gemini support.

**Architecture:** Implement hardening in small, testable slices that preserve the local-first workspace contract. Keep migration logic in `src/canisend/workspace.py`, CLI flags in `src/canisend/cli.py`, package readiness checks in a focused module, and tests in the existing productization and contract suites.

**Tech Stack:** Python 3.11+, Typer CLI, pytest, packaged resource trees, local Markdown/YAML/JSON workspace files.

---

### Task 1: Deprecated Workspace Bridge Cleanup

**Files:**
- Modify: `src/canisend/workspace.py`
- Modify: `src/canisend/cli.py`
- Modify: `tests/test_workspace_productization.py`
- Modify: `README.md`

- [x] **Step 1: Write failing tests**

Add tests that create an old workspace-local `GEMINI.md`, then assert:

```python
def test_doctor_reports_deprecated_workspace_bridge(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    (workspace / "GEMINI.md").write_text("old bridge\n", encoding="utf-8")

    lines = doctor_lines(workspace)

    assert "- Deprecated files: GEMINI.md (run `canisend update-workspace --prune-deprecated`)" in lines
```

```python
def test_update_workspace_prunes_deprecated_bridge_when_requested(tmp_path):
    workspace = tmp_path / "workspace"
    runner = CliRunner()
    runner.invoke(app, ["init-workspace", "--workspace", str(workspace), "--profile-mode", "typst"])
    (workspace / "GEMINI.md").write_text("old bridge\n", encoding="utf-8")

    result = runner.invoke(app, ["update-workspace", "--workspace", str(workspace), "--prune-deprecated"])

    assert result.exit_code == 0
    assert not (workspace / "GEMINI.md").exists()
    assert "Removed 1 deprecated file." in result.output
```

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
uv run python -m pytest tests/test_workspace_productization.py::test_doctor_reports_deprecated_workspace_bridge tests/test_workspace_productization.py::test_update_workspace_prunes_deprecated_bridge_when_requested -q
```

Expected: fail because `doctor_lines` does not report deprecated files and `update-workspace` has no `--prune-deprecated` option.

- [x] **Step 3: Implement minimal cleanup support**

Add a `DEPRECATED_WORKSPACE_FILES = ("GEMINI.md",)` constant, a `deprecated_workspace_files(workspace)` helper, and a `prune_deprecated_workspace_files(workspace)` helper in `workspace.py`. Add a `--prune-deprecated` flag to `update-workspace` and print the removed count.

- [x] **Step 4: Run targeted tests to verify they pass**

Run:

```bash
uv run python -m pytest tests/test_workspace_productization.py::test_doctor_reports_deprecated_workspace_bridge tests/test_workspace_productization.py::test_update_workspace_prunes_deprecated_bridge_when_requested -q
```

Expected: pass.

- [x] **Step 5: Update README upgrade guidance**

Document:

```bash
canisend update-workspace --workspace ~/CanISendWorkspace --prune-deprecated
```

Explain that deprecated bridge files are only removed when the user opts in.

### Task 2: Package Ready Check Command

**Files:**
- Create: `src/canisend/ready_check.py`
- Modify: `src/canisend/cli.py`
- Test: `tests/test_ready_check.py`
- Modify: `README.md`

- [x] **Step 1: Write failing tests for ready checks**

Tests should create a minimal job folder and assert `canisend check-package --workspace <workspace> --job <job>` reports missing or stale required artifacts before claiming package readiness.

- [x] **Step 2: Implement a read-only checker**

The checker should inspect required files, placeholder tokens, unknown evidence citations, material review checklist presence, and Typst JSON presence without generating or modifying files.

- [x] **Step 3: Document command usage**

Add a short README section near material review showing `canisend check-package`.

### Task 3: Job Lifecycle Status Guidance

**Files:**
- Modify: `src/canisend/jobs.py`
- Modify: `src/canisend/pipeline.py`
- Modify: `src/canisend/workspace.py`
- Test: `tests/test_jobs.py`, `tests/test_pipeline.py`, `tests/test_workspace_productization.py`

- [x] **Step 1: Write failing tests for next-action guidance**

Tests should assert `doctor` or a helper reports the next recommended action from `job.yaml status` and generated files.

- [x] **Step 2: Implement status summarization**

Add conservative status guidance without blocking existing commands.

### Task 4: Provider Output Diagnostics

**Files:**
- Modify: `src/canisend/llm.py`
- Modify: `src/canisend/parse.py`
- Modify: `src/canisend/materials.py`
- Test: `tests/test_llm.py`, `tests/test_parser_llm.py`, `tests/test_materials_llm.py`

- [x] **Step 1: Write failing tests for malformed provider output**

Tests should cover invalid JSON, Markdown wrapped JSON, empty stdout, and non-zero command exit.

- [x] **Step 2: Improve diagnostics and limited repair**

Keep repair conservative: extract one JSON object when it is clearly fenced or surrounded by prose; otherwise fail with actionable diagnostics.

### Task 5: Workspace Resource Versioning

**Files:**
- Create or modify: resource manifest under packaged defaults
- Modify: `src/canisend/workspace.py`
- Test: `tests/test_workspace_productization.py`, `tests/test_release_productization.py`

- [x] **Step 1: Write failing tests for stale packaged defaults**

Tests should verify `doctor` reports stale local defaults when package resource checksums differ.

- [x] **Step 2: Implement manifest and doctor reporting**

Report stale resources without overwriting local edits unless `update-workspace --overwrite` is used.

---

## Execution Notes

Start with Task 1 because it completes the Gemini retirement work and gives existing workspaces a safe migration path. Do not start Task 2 until Task 1 has passing targeted tests and the full suite still passes.
