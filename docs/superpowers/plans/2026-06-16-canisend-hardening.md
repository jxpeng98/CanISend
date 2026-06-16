# CanISend Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the current release blocker and harden CanISend's evidence, parsing, diagnostics, and release-consistency guarantees without expanding product scope.

**Architecture:** Keep the existing small-module CLI architecture. Add narrow validation helpers where the current behavior already has a natural boundary, prefer conservative deterministic output, and keep all changes covered by focused pytest tests.

**Tech Stack:** Python 3.11+, Typer, pytest, PyYAML, existing CanISend modules under `src/canisend`.

---

## Scope

Implement these findings from the project review:

- Fix the failing `run-example --overwrite` safety-error test.
- Ensure deterministic draft outputs preserve evidence citations.
- Validate LLM final-package citations before writing `06_final_application_package.md`.
- Harden deterministic advert parsing for common Markdown heading and bullet variants.
- Make coverage labels more conservative and easier to explain.
- Align schema/docs/plugin versions and improve `doctor` diagnostics.

Do not add web scraping, portal submission, account automation, or a large parser framework.
Do not create git commits unless the user explicitly asks for them.

## File Map

- `src/canisend/cli.py`: user-facing CLI error handling for example workspace safety failures.
- `src/canisend/examples.py`: safety guard remains the source of truth for example overwrite checks.
- `src/canisend/match.py`: deterministic evidence citation rendering and conservative coverage scoring.
- `src/canisend/materials.py`: reusable citation validation helper and final-package validation.
- `src/canisend/pipeline.py`: pass evidence into final-package validation and preserve metadata timestamps.
- `src/canisend/parse.py`: deterministic parser normalization for Markdown headings and bullet forms.
- `src/canisend/workspace.py`: doctor details for config warnings and generated evidence paths from manifest.
- `schemas/*.schema.json`: align schemas with generated artifacts or explicitly scope them.
- `README.md`, `.codex-plugin/plugin.json`: release/version consistency.
- `tests/test_examples.py`: existing red test for safety-message failure.
- `tests/test_match.py`: deterministic citation and conservative coverage tests.
- `tests/test_materials_llm.py`: final-package validation tests.
- `tests/test_parser_llm.py` or new `tests/test_parse.py`: deterministic parser hardening tests.
- `tests/test_workspace_productization.py`: doctor diagnostics and staleness tests.
- `tests/test_release_productization.py` or `tests/test_skill_distribution.py`: version consistency tests.

---

### Task 1: Fix Example Workspace Safety Error Display

**Files:**
- Modify: `src/canisend/cli.py:150-168`
- Use existing failing test: `tests/test_examples.py::test_run_example_overwrite_refuses_unmarked_workspace`

- [ ] **Step 1: Verify the existing failing test is red**

Run:

```bash
uv run pytest tests/test_examples.py::test_run_example_overwrite_refuses_unmarked_workspace -q
```

Expected: FAIL because `result.output` does not contain `CanISend example workspace`.

- [ ] **Step 2: Make the CLI surface the safety error as a runtime failure**

Replace the `ValueError` handling in `run_example()` with a normal CLI error message and exit code:

```python
    try:
        result = run_packaged_example(workspace, overwrite=overwrite)
    except ValueError as exc:
        typer.echo(str(exc), err=True)
        raise typer.Exit(code=1) from exc
```

Keep `run_packaged_example()` and `_remove_workspace_for_example()` unchanged.

- [ ] **Step 3: Verify the targeted test is green**

Run:

```bash
uv run pytest tests/test_examples.py::test_run_example_overwrite_refuses_unmarked_workspace -q
```

Expected: PASS.

- [ ] **Step 4: Verify example workflow tests still pass**

Run:

```bash
uv run pytest tests/test_examples.py -q
```

Expected: all tests in `tests/test_examples.py` pass.

---

### Task 2: Preserve Evidence Citations in Deterministic Drafts and Validate Final Package

**Files:**
- Modify: `src/canisend/match.py:169-270`
- Modify: `src/canisend/materials.py:71-115`
- Modify: `src/canisend/pipeline.py:52-62`
- Modify: `tests/test_match.py`
- Modify: `tests/test_materials_llm.py`
- Modify: `tests/test_pipeline.py` only if existing expectations need citation updates.

- [ ] **Step 1: Add failing tests for deterministic citation rendering**

Add or update tests in `tests/test_match.py` so deterministic cover letter and CV notes include item-level citations next to inserted evidence:

```python
def test_format_cover_letter_draft_preserves_evidence_citations():
    evidence = EvidenceReference(
        source_file="profile/generated/cv.evidence.md",
        section="Teaching",
        item_id="cv-001",
        text="Led econometrics seminars.",
    )
    match = CriterionMatch(
        criterion="Teaching experience in econometrics",
        coverage="partial",
        matched_items=[evidence],
        suggestion="Some evidence found.",
    )
    parsed_job = {"title": "Lecturer", "institution": "University X"}

    draft = format_cover_letter_draft(parsed_job, [match])

    assert "Led econometrics seminars." in draft
    assert "`profile/generated/cv.evidence.md#Teaching/cv-001`" in draft
```

```python
def test_format_cv_notes_preserves_evidence_citations():
    evidence = EvidenceReference(
        source_file="profile/generated/cv.evidence.md",
        section="Research",
        item_id="cv-002",
        text="Published on labour markets.",
    )
    match = CriterionMatch(
        criterion="Strong research publication record",
        coverage="partial",
        matched_items=[evidence],
        suggestion="Some evidence found.",
    )
    parsed_job = {"title": "Lecturer", "institution": "University X"}

    notes = format_cv_notes(parsed_job, [match])

    assert "Published on labour markets." in notes
    assert "`profile/generated/cv.evidence.md#Research/cv-002`" in notes
```

Run:

```bash
uv run pytest tests/test_match.py::test_format_cover_letter_draft_preserves_evidence_citations tests/test_match.py::test_format_cv_notes_preserves_evidence_citations -q
```

Expected: FAIL because deterministic drafts currently omit citations.

- [ ] **Step 2: Add failing tests for final-package citation validation**

In `tests/test_materials_llm.py`, add a provider test that final-package generation rejects unknown evidence citations:

```python
def test_generate_final_package_with_provider_rejects_unknown_citation():
    provider = FakeProvider(
        "# Final Package\n\nClaim (`profile/generated/cv.evidence.md#Teaching/unknown`)."
    )
    materials = ApplicationMaterials(
        fit_report="# Fit\n\nClaim (`profile/generated/cv.evidence.md#Teaching/cv-001`).",
        cover_letter_draft="# Cover\n\nClaim (`profile/generated/cv.evidence.md#Teaching/cv-001`).",
        cv_tailoring_notes="# Notes\n\nClaim (`profile/generated/cv.evidence.md#Teaching/cv-001`).",
        criteria_checklist="# Criteria\n\nClaim (`profile/generated/cv.evidence.md#Teaching/cv-001`).",
    )

    with pytest.raises(MaterialValidationError, match="final_package contains unknown evidence citation"):
        generate_final_package_with_provider(
            parsed_job=parsed_job(),
            materials=materials,
            evidence=evidence(),
            provider=provider,
        )
```

Run:

```bash
uv run pytest tests/test_materials_llm.py::test_generate_final_package_with_provider_rejects_unknown_citation -q
```

Expected: FAIL because `generate_final_package_with_provider()` does not accept evidence or validate the package.

- [ ] **Step 3: Implement citation rendering in deterministic drafts**

In `src/canisend/match.py`, append citations where deterministic evidence text is inserted:

```python
def _evidence_bullet(item: EvidenceReference) -> str:
    return f"{item.text} (`{item.citation}`)"
```

Use `_evidence_bullet(item)` in `format_cover_letter_draft()` and `format_cv_notes()` instead of appending only `item.text`.

- [ ] **Step 4: Implement reusable markdown citation validation**

In `src/canisend/materials.py`, extract the validation body into a helper that can validate either a named markdown string or the `ApplicationMaterials` bundle:

```python
def validate_markdown_citations(
    name: str,
    markdown: str,
    evidence: list[EvidenceReference],
    *,
    require_citation: bool = False,
) -> None:
    allowed = _allowed_citations(evidence)
    citations = _markdown_citations(markdown)
    unknown = sorted(citations - allowed)
    if unknown:
        raise MaterialValidationError(f"{name} contains unknown evidence citation: {unknown[0]}")
    if require_citation and evidence and not citations:
        raise MaterialValidationError(f"{name} must cite at least one profile evidence reference")
```

Update `validate_material_citations()` to call this helper for each material with `require_citation=True`.

- [ ] **Step 5: Validate LLM final package**

Change `generate_final_package_with_provider()` signature to accept `evidence: list[EvidenceReference]`, then validate output:

```python
def generate_final_package_with_provider(
    *,
    parsed_job: dict[str, Any],
    materials: ApplicationMaterials,
    evidence: list[EvidenceReference],
    provider: LLMProvider,
    prompt_dir: Path = Path("prompts"),
) -> str:
    ...
    content = provider.complete(prompt).content.strip() + "\n"
    validate_markdown_citations("final_package", content, evidence, require_citation=bool(evidence))
    return content
```

Update `src/canisend/pipeline.py` to pass `evidence=evidence`.

- [ ] **Step 6: Verify targeted tests**

Run:

```bash
uv run pytest tests/test_match.py tests/test_materials_llm.py tests/test_pipeline.py -q
```

Expected: all selected tests pass.

---

### Task 3: Harden Deterministic Parser and Conservative Coverage Labels

**Files:**
- Modify: `src/canisend/parse.py:124-167`
- Modify: `src/canisend/match.py:65-116`
- Modify: `tests/test_parser_llm.py` or create `tests/test_parse.py`
- Modify: `tests/test_match.py`

- [ ] **Step 1: Add failing parser tests for Markdown variants**

Create `tests/test_parse.py` with:

```python
from canisend.parse import parse_job_advert


def test_parse_job_advert_accepts_markdown_headings_and_bullet_variants():
    advert = """# Lecturer in Economics

## Job Details

- Department: Department of Economics
- Location: London
- Salary: Grade 7
- Contract: Permanent
- Role type: Lecturer
- Research fields: Economics, Econometrics
- Teaching fields: Statistics, Econometrics
- Required documents: CV, Cover letter

## Essential Criteria

* PhD or near completion in Economics
1. Evidence of teaching excellence

## Desirable Criteria

- Experience supervising dissertations
"""

    parsed = parse_job_advert(advert, {"institution": "University X", "deadline": "2026-06-15"})

    assert parsed["department"] == "Department of Economics"
    assert parsed["location"] == "London"
    assert parsed["essential_criteria"] == [
        {"criterion": "PhD or near completion in Economics", "source_text": "PhD or near completion in Economics"},
        {"criterion": "Evidence of teaching excellence", "source_text": "Evidence of teaching excellence"},
    ]
    assert parsed["desirable_criteria"] == [
        {"criterion": "Experience supervising dissertations", "source_text": "Experience supervising dissertations"}
    ]
```

Run:

```bash
uv run pytest tests/test_parse.py -q
```

Expected: FAIL before implementation.

- [ ] **Step 2: Normalize Markdown labels and bullets**

In `src/canisend/parse.py`, add helpers:

```python
def _plain_line(line: str) -> str:
    stripped = line.strip()
    stripped = re.sub(r"^#{1,6}\s*", "", stripped)
    stripped = re.sub(r"^[-*]\s+", "", stripped)
    stripped = re.sub(r"^\d+[.)]\s+", "", stripped)
    return stripped.strip()


def _bullet_text(line: str) -> str:
    stripped = line.strip()
    match = re.match(r"(?:[-*]|\d+[.)])\s+(.+)", stripped)
    return match.group(1).strip() if match else ""
```

Update `_field()` to inspect `_plain_line(raw_line)`. Update `_criteria()` to detect section headings using `_plain_line(raw_line).rstrip(":").lower()` and to accept `_bullet_text(raw_line)` while active.

- [ ] **Step 3: Add failing conservative coverage test**

In `tests/test_match.py`, add:

```python
def test_match_criterion_does_not_mark_kind_only_matches_as_strong():
    evidence = [
        EvidenceReference("profile/generated/cv.evidence.md", "Research", "Published a paper on medieval history.", "cv-001"),
        EvidenceReference("profile/generated/cv.evidence.md", "Research", "Presented at a history conference.", "cv-002"),
    ]
    index = EvidenceIndex(evidence)

    match = index.match_criterion("Strong research record in econometrics")

    assert match.coverage in {"partial", "weak"}
```

Run:

```bash
uv run pytest tests/test_match.py::test_match_criterion_does_not_mark_kind_only_matches_as_strong -q
```

Expected: FAIL if current broad kind overlap returns `strong`.

- [ ] **Step 4: Make coverage labels require direct lexical support**

In `src/canisend/match.py`, add:

```python
def _direct_overlap_score(query: str, item: EvidenceReference) -> int:
    item_text = item.text.lower()
    return sum(1 for token in _tokenize(query) if len(token) >= 4 and token in item_text)
```

Update `match_criterion()` so:

```python
        direct_matches = [item for item in matches if _direct_overlap_score(criterion_text.lower(), item) > 0]

        if len(direct_matches) >= 2:
            coverage = "strong"
        elif direct_matches:
            coverage = "partial"
        elif matches or related_kinds:
            coverage = "weak"
        else:
            coverage = "missing"
```

Leave `search()` as a broad retrieval function, but make coverage more conservative.

- [ ] **Step 5: Verify parser and match suites**

Run:

```bash
uv run pytest tests/test_parse.py tests/test_match.py tests/test_pipeline.py -q
```

Expected: all selected tests pass. If existing expectations relied on kind-only `strong`, update tests to assert conservative labels.

---

### Task 4: Align Schemas, Doctor Diagnostics, and Version Surfaces

**Files:**
- Modify: `src/canisend/workspace.py:126-160`
- Modify: `schemas/criteria_check.schema.json`
- Modify: `schemas/fit_report.schema.json`
- Modify: `README.md`
- Modify: `.codex-plugin/plugin.json`
- Modify: `tests/test_workspace_productization.py`
- Modify: `tests/test_release_productization.py`
- Modify: `tests/test_skill_distribution.py` only if plugin version expectations are added.

- [ ] **Step 1: Add failing doctor diagnostics tests**

In `tests/test_workspace_productization.py`, add tests that prove doctor reports actual config warnings and honors custom generated evidence mapping:

```python
def test_doctor_reports_config_warning_details(tmp_path):
    workspace = tmp_path
    (workspace / "canisend.yaml").write_text("unknown_key: value\n", encoding="utf-8")

    lines = doctor_lines(workspace)

    assert any("Unknown key in canisend.yaml" in line for line in lines)
```

```python
def test_doctor_uses_profile_generated_manifest_paths(tmp_path):
    profile = tmp_path / "profile"
    source = profile / "typst" / "cv.typ"
    generated = profile / "custom" / "cv-items.md"
    source.parent.mkdir(parents=True)
    generated.parent.mkdir(parents=True)
    (profile / "profile.yaml").write_text(
        "sources:\n  cv: typst/cv.typ\n"
        "generated:\n  cv_evidence: custom/cv-items.md\n",
        encoding="utf-8",
    )
    source.write_text("#section(\"Teaching\")\n", encoding="utf-8")
    generated.write_text("# Evidence: cv\n", encoding="utf-8")

    lines = doctor_lines(tmp_path)

    assert "- Evidence staleness: up to date" in lines
```

Run:

```bash
uv run pytest tests/test_workspace_productization.py::test_doctor_reports_config_warning_details tests/test_workspace_productization.py::test_doctor_uses_profile_generated_manifest_paths -q
```

Expected: FAIL before implementation.

- [ ] **Step 2: Fix doctor diagnostics**

In `src/canisend/workspace.py`, update `_evidence_staleness_line()` to use `manifest["generated"][f"{source_key}_evidence"]` when present:

```python
def _generated_evidence_path(profile_dir: Path, source_key: str, generated: dict[str, str]) -> Path:
    return profile_dir / generated.get(f"{source_key}_evidence", f"generated/{source_key}.evidence.md")
```

Use that helper for both stale checks and "any generated evidence exists" checks.

Update `_config_validation_line()` to include warning text:

```python
    return "- Config validation: " + " | ".join(warnings)
```

- [ ] **Step 3: Align schema files with Markdown artifacts**

Because `02_fit_report.md` and `05_criteria_checklist.md` are Markdown outputs, update `schemas/fit_report.schema.json` and `schemas/criteria_check.schema.json` to describe the structured content files that can actually be validated later, or rename titles/descriptions to avoid claiming runtime validation today.

Minimum acceptable change:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "CriteriaCheckContent",
  "description": "Structured criteria-check content contract for future JSON output. Current CLI output is 05_criteria_checklist.md.",
  ...
}
```

Also change `risk` enum values to match generated labels if the schema is meant to represent current criteria rows:

```json
"risk": {"enum": ["Low", "Medium", "High"]}
```

- [ ] **Step 4: Align release-visible versions**

Update README TestPyPI badge and install examples from `0.2.0b2` to `0.2.0b3`.

Update `.codex-plugin/plugin.json` version from `0.2.0` to `0.2.0b3` unless tests indicate plugin version intentionally follows a stable marketplace version. If preserving `0.2.0` is intentional, add a test or README note explaining the divergence.

- [ ] **Step 5: Add release consistency tests**

In `tests/test_release_productization.py`, add:

```python
def test_readme_current_testpypi_version_matches_package_version():
    readme = Path("README.md").read_text(encoding="utf-8")

    assert f"canisend=={__version__}" in readme
    assert f"TestPyPI-{__version__}-blue" in readme
```

In `tests/test_skill_distribution.py`, add:

```python
def test_codex_plugin_version_matches_package_version():
    manifest = json.loads(Path(".codex-plugin/plugin.json").read_text(encoding="utf-8"))

    assert manifest["version"] == __version__
```

Run:

```bash
uv run pytest tests/test_workspace_productization.py tests/test_release_productization.py tests/test_skill_distribution.py -q
```

Expected: all selected tests pass.

---

### Task 5: Integration Verification and Cleanup

**Files:**
- Inspect all changed files.
- No new production files unless required by earlier tasks.

- [ ] **Step 1: Review git diff**

Run:

```bash
git diff -- src tests schemas README.md .codex-plugin docs
```

Expected: only scoped changes from Tasks 1-4.

- [ ] **Step 2: Run full test suite**

Run:

```bash
uv run pytest -q
```

Expected: all tests pass.

- [ ] **Step 3: Run build**

Run:

```bash
uv build
```

Expected: `dist/canisend-0.2.0b3.tar.gz` and `dist/canisend-0.2.0b3-py3-none-any.whl` build successfully.

- [ ] **Step 4: Run packaged resource check**

Run:

```bash
uv run python -m canisend.package_check dist/canisend-0.2.0b3-py3-none-any.whl
```

Expected: `packaged resources ok`.

- [ ] **Step 5: Run focused CLI smoke checks**

Run:

```bash
uv run canisend run-example --workspace /tmp/canisend-example-hardening --overwrite
uv run canisend doctor --workspace /tmp/canisend-example-hardening
```

Expected: example generation succeeds and doctor reports initialized workspace resources as ok.

- [ ] **Step 6: Final status**

Run:

```bash
git status --short
```

Expected: only intended source, test, schema, docs, README, and plugin manifest changes are present. `dist/` remains ignored.
