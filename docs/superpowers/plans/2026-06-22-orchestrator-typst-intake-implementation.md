# Orchestrator, Typst Direct Editing, and Job Intake Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement explicit URL/PDF job intake, strict university HR material review, direct-editable Typst outputs, and a local multi-agent orchestrator CLI.

**Architecture:** Split the work into focused modules: `job_import.py` handles URL/PDF/text intake, `material_review.py` owns strict HR checklist logic, `typst_mapping.py` renders direct `.typ` files, and `orchestrator.py` owns plan validation, scheduling, worker execution, and run artifacts. Existing CLI commands remain the entrypoints and call these focused helpers.

**Tech Stack:** Python 3.11, Typer, PyYAML, stdlib `urllib`, `html.parser`, `subprocess`, `concurrent.futures`, and `pypdf` for local PDF text extraction.

---

## Scope And File Map

- Create `src/canisend/job_import.py`: local advert file import, PDF extraction, URL validation, HTML fetch, and readable text extraction.
- Modify `src/canisend/jobs.py`: call `job_import` helpers before writing a job folder; write a source-URL stub when no advert is imported.
- Modify `src/canisend/cli.py`: add `new-job --fetch-url`; add `orchestrate`.
- Modify `pyproject.toml` and `uv.lock`: add `pypdf`.
- Modify `src/canisend/material_review.py`: add strict university HR review section.
- Modify `skills/canisend-material-review/SKILL.md` and `agent-skills/canisend/references/*`: document strict HR review and direct `.typ` editing.
- Modify `src/canisend/typst_mapping.py`: render direct Typst content with stable `// CANISEND:` markers.
- Modify `src/canisend/pipeline.py`: keep optional JSON debug outputs but make `.typ` files independent of JSON.
- Create `src/canisend/orchestrator.py`: parse YAML plans, validate privacy/write scopes, schedule tasks concurrently, run commands, and write run artifacts.
- Add tests in `tests/test_job_import.py`, `tests/test_material_review.py`, `tests/test_orchestrator.py`.
- Modify existing tests in `tests/test_jobs.py`, `tests/test_pipeline.py`, `tests/test_typst_mapping.py`, `tests/test_repository_contract.py`, and `tests/test_skill_distribution.py`.
- Update `README.md`, `skills/canisend/references/*.md`, and matching `agent-skills/canisend/references/*.md`.

---

## Task 1: Add Job Import Helpers

**Files:**
- Create: `src/canisend/job_import.py`
- Test: `tests/test_job_import.py`
- Modify: `pyproject.toml`
- Modify: `uv.lock`

- [x] **Step 1: Add `pypdf` dependency**

Run:

```bash
uv add "pypdf>=5.0"
```

Expected: `pyproject.toml` includes `pypdf>=5.0` and `uv.lock` is updated.

- [x] **Step 2: Write failing tests for text, PDF, URL validation, and HTML extraction**

Create `tests/test_job_import.py`:

```python
from __future__ import annotations

from io import BytesIO
from pathlib import Path

import pytest

from canisend.job_import import (
    JobImportError,
    extract_html_text,
    extract_pdf_text,
    fetch_advert_from_url,
    import_advert_file,
    validate_fetch_url,
)


class FakePage:
    def __init__(self, text: str) -> None:
        self.extract_text_value = text

    def extract_text(self) -> str:
        return self.extract_text_value


class FakeReader:
    def __init__(self, path: Path) -> None:
        self.path = path
        self.pages = [FakePage("Lecturer in Economics\nEssential criteria\nPhD")]


class EmptyReader:
    def __init__(self, path: Path) -> None:
        self.pages = [FakePage("")]


class FakeResponse:
    headers = {"Content-Type": "text/html; charset=utf-8"}

    def __init__(self, body: str) -> None:
        self.body = body.encode("utf-8")

    def __enter__(self) -> "FakeResponse":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        return None

    def read(self) -> bytes:
        return self.body


def test_import_advert_file_preserves_markdown_and_text(tmp_path):
    advert = tmp_path / "advert.md"
    advert.write_text("# Lecturer\n\nEssential criteria:\n- PhD\n", encoding="utf-8")

    imported = import_advert_file(advert)

    assert imported.text == "# Lecturer\n\nEssential criteria:\n- PhD\n"
    assert imported.status == "advert_imported"
    assert imported.notes == ""


def test_extract_pdf_text_uses_reader_factory(tmp_path):
    pdf = tmp_path / "jd.pdf"
    pdf.write_bytes(b"%PDF fake")

    text = extract_pdf_text(pdf, reader_factory=FakeReader)

    assert "Lecturer in Economics" in text
    assert "Essential criteria" in text


def test_extract_pdf_text_rejects_empty_text(tmp_path):
    pdf = tmp_path / "jd.pdf"
    pdf.write_bytes(b"%PDF fake")

    with pytest.raises(JobImportError, match="No text could be extracted"):
        extract_pdf_text(pdf, reader_factory=EmptyReader)


def test_validate_fetch_url_rejects_missing_or_non_http_url():
    with pytest.raises(JobImportError, match="--fetch-url requires --source-url"):
        validate_fetch_url("")
    with pytest.raises(JobImportError, match="Only http:// and https:// URLs"):
        validate_fetch_url("file:///tmp/job.html")


def test_extract_html_text_removes_scripts_styles_and_tags():
    html = """
    <html><head><style>.x{}</style><script>alert(1)</script></head>
    <body><h1>Lecturer in Economics</h1><p>Essential criteria: PhD.</p></body></html>
    """

    text = extract_html_text(html)

    assert "Lecturer in Economics" in text
    assert "Essential criteria: PhD." in text
    assert "alert" not in text
    assert ".x" not in text


def test_fetch_advert_from_url_reads_html_with_injected_opener():
    def opener(request, timeout):
        assert request.full_url == "https://example.edu/jobs/123"
        return FakeResponse("<html><body><h1>Lecturer</h1><p>PhD required.</p></body></html>")

    imported = fetch_advert_from_url("https://example.edu/jobs/123", opener=opener)

    assert imported.status == "advert_imported"
    assert "Lecturer" in imported.text
    assert "PhD required." in imported.text
    assert "Fetched from https://example.edu/jobs/123" in imported.notes
```

- [x] **Step 3: Run tests to verify failure**

Run:

```bash
uv run pytest tests/test_job_import.py -q
```

Expected: FAIL because `canisend.job_import` does not exist.

- [x] **Step 4: Implement `src/canisend/job_import.py`**

Add:

```python
from __future__ import annotations

from dataclasses import dataclass
from html.parser import HTMLParser
from pathlib import Path
import re
from typing import Any, Callable
from urllib.parse import urlparse
from urllib.request import Request, urlopen


class JobImportError(ValueError):
    pass


@dataclass(frozen=True)
class ImportedAdvert:
    text: str
    status: str
    notes: str = ""


class _ReadableHTMLParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self._skip_depth = 0
        self.parts: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag.lower() in {"script", "style", "noscript"}:
            self._skip_depth += 1
        if tag.lower() in {"p", "div", "section", "article", "h1", "h2", "h3", "li", "br"}:
            self.parts.append("\n")

    def handle_endtag(self, tag: str) -> None:
        if tag.lower() in {"script", "style", "noscript"} and self._skip_depth:
            self._skip_depth -= 1
        if tag.lower() in {"p", "div", "section", "article", "h1", "h2", "h3", "li"}:
            self.parts.append("\n")

    def handle_data(self, data: str) -> None:
        if not self._skip_depth:
            self.parts.append(data)


def import_advert_file(path: Path) -> ImportedAdvert:
    suffix = path.suffix.lower()
    if suffix in {".md", ".txt"}:
        return ImportedAdvert(text=path.read_text(encoding="utf-8"), status="advert_imported")
    if suffix == ".pdf":
        text = extract_pdf_text(path)
        header = f"<!-- Imported from local PDF: {path.name}. Review extracted text before relying on parsed criteria. -->\n\n"
        return ImportedAdvert(
            text=header + text.rstrip() + "\n",
            status="advert_imported",
            notes=f"Imported from local PDF {path.name}; review extracted text before relying on parsed criteria.",
        )
    raise JobImportError("CanISend imports local .md, .txt, or .pdf job advert files.")


def extract_pdf_text(path: Path, *, reader_factory: Callable[[Path], Any] | None = None) -> str:
    if reader_factory is None:
        try:
            from pypdf import PdfReader
        except ImportError as exc:
            raise JobImportError("PDF import requires the pypdf package.") from exc
        reader_factory = PdfReader
    try:
        reader = reader_factory(path)
    except Exception as exc:
        raise JobImportError(f"Could not read PDF advert: {exc}") from exc
    pages = [page.extract_text() or "" for page in reader.pages]
    text = "\n\n".join(page.strip() for page in pages if page.strip()).strip()
    if not text:
        raise JobImportError("No text could be extracted from the PDF advert.")
    return text


def validate_fetch_url(source_url: str) -> str:
    if not source_url.strip():
        raise JobImportError("--fetch-url requires --source-url.")
    parsed = urlparse(source_url)
    if parsed.scheme not in {"http", "https"}:
        raise JobImportError("Only http:// and https:// URLs can be fetched.")
    if not parsed.netloc:
        raise JobImportError("Fetch URL must include a host.")
    return source_url


def fetch_advert_from_url(
    source_url: str,
    *,
    opener: Callable[..., Any] = urlopen,
    timeout: int = 30,
) -> ImportedAdvert:
    url = validate_fetch_url(source_url)
    request = Request(url, headers={"User-Agent": "CanISend/0.1 job-advert-import"})
    try:
        with opener(request, timeout=timeout) as response:
            content_type = response.headers.get("Content-Type", "")
            if "html" not in content_type.lower():
                raise JobImportError(f"Fetched URL did not return HTML: {content_type or 'unknown content type'}")
            raw = response.read()
    except JobImportError:
        raise
    except Exception as exc:
        raise JobImportError(f"Could not fetch job URL: {exc}") from exc
    html = raw.decode("utf-8", errors="replace")
    text = extract_html_text(html)
    if not text.strip():
        raise JobImportError("No readable text could be extracted from the fetched job page.")
    header = f"<!-- Fetched from {url}. Review extracted text before relying on parsed criteria. -->\n\n"
    return ImportedAdvert(
        text=header + text.rstrip() + "\n",
        status="advert_imported",
        notes=f"Fetched from {url}; review extracted text before relying on parsed criteria.",
    )


def extract_html_text(html: str) -> str:
    parser = _ReadableHTMLParser()
    parser.feed(html)
    raw = "".join(parser.parts)
    lines = [re.sub(r"[ \t]+", " ", line).strip() for line in raw.splitlines()]
    compact = [line for line in lines if line]
    return "\n".join(compact).strip()
```

- [x] **Step 5: Run tests to verify pass**

Run:

```bash
uv run pytest tests/test_job_import.py -q
```

Expected: PASS.

- [x] **Step 6: Commit**

Run:

```bash
git add pyproject.toml uv.lock src/canisend/job_import.py tests/test_job_import.py
git commit -m "feat(job-intake): add advert import helpers"
```

---

## Task 2: Wire Job Intake Into `new-job`

**Files:**
- Modify: `src/canisend/jobs.py`
- Modify: `src/canisend/cli.py`
- Modify: `tests/test_jobs.py`

- [x] **Step 1: Write failing CLI tests**

Append to `tests/test_jobs.py`:

```python
def test_new_job_with_source_url_writes_metadata_stub(tmp_path):
    jobs_dir = tmp_path / "jobs"
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer in Economics",
            "--institution",
            "University X",
            "--deadline",
            "2026-06-15",
            "--source-url",
            "https://example.edu/jobs/123",
            "--jobs-dir",
            str(jobs_dir),
        ],
    )

    assert result.exit_code == 0
    job_dir = jobs_dir / "2026-06-15_university-x_lecturer-in-economics"
    metadata = yaml.safe_load((job_dir / "job.yaml").read_text())
    assert metadata["status"] == "new"
    advert = (job_dir / "job_advert.md").read_text()
    assert "Source URL saved" in advert
    assert "https://example.edu/jobs/123" in advert
    assert "full advert still needs manual paste, PDF import, or explicit fetch" in advert


def test_new_job_fetch_url_requires_source_url(tmp_path):
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer",
            "--institution",
            "University X",
            "--fetch-url",
            "--jobs-dir",
            str(tmp_path / "jobs"),
        ],
    )

    assert result.exit_code != 0
    assert "--fetch-url requires --source-url" in result.output
    assert not (tmp_path / "jobs").exists()


def test_new_job_fetch_url_imports_fetched_advert(tmp_path, monkeypatch):
    jobs_dir = tmp_path / "jobs"

    def fake_fetch(source_url: str):
        from canisend.job_import import ImportedAdvert

        return ImportedAdvert(
            text="# Lecturer\n\nEssential criteria:\n- PhD\n",
            status="advert_imported",
            notes=f"Fetched from {source_url}; review extracted text.",
        )

    monkeypatch.setattr("canisend.jobs.fetch_advert_from_url", fake_fetch)
    runner = CliRunner()

    result = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer",
            "--institution",
            "University X",
            "--deadline",
            "2026-06-15",
            "--source-url",
            "https://example.edu/jobs/123",
            "--fetch-url",
            "--jobs-dir",
            str(jobs_dir),
        ],
    )

    assert result.exit_code == 0
    job_dir = jobs_dir / "2026-06-15_university-x_lecturer"
    metadata = yaml.safe_load((job_dir / "job.yaml").read_text())
    assert metadata["status"] == "advert_imported"
    assert "Fetched from https://example.edu/jobs/123" in metadata["notes"]
    assert "Essential criteria" in (job_dir / "job_advert.md").read_text()
```

Change the existing `test_new_job_creates_slugged_job_folder_and_metadata` assertion from empty advert text to a stub when `--source-url` is used:

```python
advert = (job_dir / "job_advert.md").read_text()
assert "Source URL saved" in advert
```

- [x] **Step 2: Run tests to verify failure**

Run:

```bash
uv run pytest tests/test_jobs.py -q
```

Expected: FAIL because `--fetch-url` is unknown and source URL still writes an empty advert.

- [x] **Step 3: Update `src/canisend/jobs.py`**

Modify imports:

```python
from canisend.job_import import JobImportError, fetch_advert_from_url, import_advert_file
```

Change `create_job` signature:

```python
def create_job(
    *,
    jobs_dir: Path,
    title: str,
    institution: str,
    deadline: str,
    source_url: str = "",
    advert_file: Path | None = None,
    fetch_url: bool = False,
) -> Path:
```

Replace the import block inside `create_job` with:

```python
    advert_text = ""
    status = "new"
    notes = ""
    if advert_file is not None and fetch_url:
        raise ValueError("Use either --advert-file or --fetch-url, not both.")
    if advert_file is not None:
        try:
            imported = import_advert_file(advert_file)
        except JobImportError as exc:
            raise ValueError(str(exc)) from exc
        advert_text = imported.text
        status = imported.status
        notes = imported.notes
    elif fetch_url:
        try:
            imported = fetch_advert_from_url(source_url)
        except JobImportError as exc:
            raise ValueError(str(exc)) from exc
        advert_text = imported.text
        status = imported.status
        notes = imported.notes
    elif source_url.strip():
        advert_text = _source_url_stub(source_url)
```

Add helper:

```python
def _source_url_stub(source_url: str) -> str:
    return "\n".join(
        [
            "# Job Advert Pending Import",
            "",
            f"Source URL saved: {source_url}",
            "",
            "The full advert still needs manual paste, PDF import, or explicit fetch before final parsing.",
            "",
        ]
    )
```

- [x] **Step 4: Update `src/canisend/cli.py`**

Add option to `new_job`:

```python
    fetch_url: bool = typer.Option(
        False,
        "--fetch-url",
        help="Explicitly fetch --source-url and import readable HTML text into job_advert.md.",
    ),
```

Pass it into `create_job`:

```python
            fetch_url=fetch_url,
```

- [x] **Step 5: Run tests to verify pass**

Run:

```bash
uv run pytest tests/test_jobs.py tests/test_job_import.py -q
```

Expected: PASS.

- [x] **Step 6: Commit**

Run:

```bash
git add src/canisend/jobs.py src/canisend/cli.py tests/test_jobs.py
git commit -m "feat(job-intake): support URL and PDF job imports"
```

---

## Task 3: Add Strict University HR Review Checklist

**Files:**
- Modify: `src/canisend/material_review.py`
- Create: `tests/test_material_review.py`
- Modify: `skills/canisend-material-review/SKILL.md`

- [x] **Step 1: Write failing strict HR tests**

Create `tests/test_material_review.py`:

```python
from canisend.material_review import build_material_review_checklist
from canisend.materials import ApplicationMaterials


def test_material_review_includes_strict_hr_review_and_blocks_missing_essentials():
    parsed_job = {
        "title": "Lecturer in Economics",
        "institution": "University X",
        "deadline": "2026-06-15",
        "required_documents": ["CV", "Cover letter"],
        "essential_criteria": [
            {"criterion": "PhD in Economics", "source_text": "PhD in Economics"},
            {"criterion": "Evidence of teaching excellence", "source_text": "Evidence of teaching excellence"},
        ],
        "desirable_criteria": [],
    }
    materials = ApplicationMaterials(
        fit_report="# Fit\n\nClaim (`profile/generated/cv.evidence.md#Teaching/cv-001`).",
        cover_letter_draft="# Cover\n\nI can teach econometrics.",
        cv_tailoring_notes="# Notes\n\nMove teaching higher.",
        criteria_checklist=(
            "# Criteria Coverage Checklist\n\n"
            "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |\n"
            "|---|---|---|---|---|\n"
            "| PhD in Economics | strong | `profile/generated/cv.evidence.md#Education/cv-001` | low | Keep visible. |\n"
        ),
    )

    checklist = build_material_review_checklist(parsed_job, materials)

    assert "## Strict University HR Review" in checklist
    assert "Review lens: strict university HR / shortlisting panel" in checklist
    assert "Evidence of teaching excellence" in checklist
    assert "BLOCKER" in checklist
    assert "Missing from criteria checklist" in checklist


def test_material_review_flags_weak_or_missing_essential_coverage():
    parsed_job = {
        "title": "Lecturer",
        "institution": "University X",
        "deadline": "2026-06-15",
        "required_documents": [],
        "essential_criteria": [
            {"criterion": "Strong research record", "source_text": "Strong research record"},
            {"criterion": "Teaching excellence", "source_text": "Teaching excellence"},
        ],
        "desirable_criteria": [],
    }
    materials = ApplicationMaterials(
        fit_report="# Fit\n",
        cover_letter_draft="# Cover\n",
        cv_tailoring_notes="# Notes\n",
        criteria_checklist=(
            "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |\n"
            "|---|---|---|---|---|\n"
            "| Strong research record | weak | Not yet linked | High | Add evidence. |\n"
            "| Teaching excellence | missing | Not yet linked | High | Add evidence. |\n"
        ),
    )

    checklist = build_material_review_checklist(parsed_job, materials)

    assert "Strong research record" in checklist
    assert "Teaching excellence" in checklist
    assert checklist.count("BLOCKER") == 2
```

- [x] **Step 2: Run tests to verify failure**

Run:

```bash
uv run pytest tests/test_material_review.py -q
```

Expected: FAIL because strict HR section does not exist.

- [x] **Step 3: Implement strict HR section in `material_review.py`**

Add `_strict_hr_review_section(parsed_job, materials)` and call it before `Management Actions`:

```python
            "",
            _strict_hr_review_section(parsed_job, materials),
            "",
            "## Management Actions",
```

Add helpers:

```python
def _strict_hr_review_section(parsed_job: dict[str, Any], materials: ApplicationMaterials) -> str:
    coverage = _criteria_coverage(materials.criteria_checklist)
    lines = [
        "## Strict University HR Review",
        "",
        "- Review lens: strict university HR / shortlisting panel.",
        "- Standard: every advertised essential criterion must be visible, proportionate, and evidence-backed before submission.",
        "",
        "| Essential Criterion | HR Status | Reason |",
        "|---|---|---|",
    ]
    essentials = parsed_job.get("essential_criteria", [])
    if not essentials:
        lines.append("| No essential criteria extracted | BLOCKER | Review the JD manually before relying on generated materials. |")
        return "\n".join(lines)
    for item in essentials:
        criterion = str(item.get("criterion", "")).strip()
        label = coverage.get(_criterion_key(criterion))
        if label is None:
            lines.append(f"| {criterion} | BLOCKER | Missing from criteria checklist. |")
        elif label in {"weak", "missing"}:
            lines.append(f"| {criterion} | BLOCKER | Coverage is {label}; strengthen evidence and JD wording. |")
        elif label == "partial":
            lines.append(f"| {criterion} | REVIEW | Partial coverage; make fit more explicit for HR screening. |")
        else:
            lines.append(f"| {criterion} | OK | Strong coverage recorded; confirm claim wording stays proportional. |")
    return "\n".join(lines)


def _criteria_coverage(markdown: str) -> dict[str, str]:
    coverage: dict[str, str] = {}
    for line in markdown.splitlines():
        stripped = line.strip()
        if not stripped.startswith("|") or "---" in stripped or "Criterion" in stripped:
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if len(cells) < 2:
            continue
        label = cells[1].strip().lower()
        if label in {"strong", "partial", "weak", "missing"}:
            coverage[_criterion_key(cells[0])] = label
    return coverage


def _criterion_key(value: str) -> str:
    return re.sub(r"\s+", " ", value).strip().lower()
```

- [x] **Step 4: Update material-review skill**

In `skills/canisend-material-review/SKILL.md`, add to Workflow:

```markdown
4. Review like a strict university HR or shortlisting panel member: every essential criterion must be visible, proportionate, and evidence-backed.
5. Treat weak or missing essential criteria as blockers, not polish items.
```

Renumber the remaining steps. Make the same change in any mirrored workspace skill only if tests require it.

- [x] **Step 5: Run tests to verify pass**

Run:

```bash
uv run pytest tests/test_material_review.py tests/test_pipeline.py::test_run_pipeline_generates_parsed_job_and_application_outputs -q
```

Expected: PASS.

- [x] **Step 6: Commit**

Run:

```bash
git add src/canisend/material_review.py tests/test_material_review.py skills/canisend-material-review/SKILL.md
git commit -m "feat(review): add strict university HR checklist"
```

---

## Task 4: Generate Direct Editable Typst Files

**Files:**
- Modify: `src/canisend/typst_mapping.py`
- Modify: `src/canisend/pipeline.py`
- Modify: `tests/test_typst_mapping.py`
- Modify: `tests/test_pipeline.py`

- [x] **Step 1: Rewrite Typst tests for direct content**

In `tests/test_typst_mapping.py`, replace JSON-contract assertions with:

```python
def test_render_modernpro_cover_letter_source_embeds_editable_content():
    content = build_cover_letter_content(parsed_job(), materials())

    source = render_modernpro_cover_letter_source(content)

    assert '@preview/modernpro-coverletter:0.0.8' in source
    assert 'json("cover_letter_content.json")' not in source
    assert "// CANISEND: section opening" in source
    assert "// CANISEND: section research_fit" in source
    assert "I am writing to apply for the Lecturer in Economics role." in source
    assert "My research fits the department's applied economics focus." in source


def test_render_modernpro_application_package_source_embeds_editable_content():
    content = {
        "job": parsed_job(),
        "fit_report": "# Fit Report\n\nStrong teaching fit.",
        "cover_letter": "# Cover Letter Draft\n\nLetter text.",
        "cv_tailoring_notes": "# CV Tailoring Notes\n\n- Move teaching higher.",
        "criteria_checklist": "# Criteria Coverage Checklist\n\n| Criterion | Coverage |\n|---|---|\n",
    }

    source = render_modernpro_application_package_source(content)

    assert '@preview/modernpro-coverletter:0.0.8' in source
    assert 'json("application_package_content.json")' not in source
    assert "// CANISEND: section job_information" in source
    assert "// CANISEND: section fit_report" in source
    assert "= Fit Report" in source
    assert "- Move teaching higher." in source
```

Update `tests/test_pipeline.py` assertions so `cover_letter.typ` and `application_package.typ` do not reference JSON:

```python
assert 'json("cover_letter_content.json")' not in cover_source
assert 'json("application_package_content.json")' not in package_source
assert "// CANISEND: section research_fit" in cover_source
assert "// CANISEND: section criteria_checklist" in package_source
```

- [x] **Step 2: Run tests to verify failure**

Run:

```bash
uv run pytest tests/test_typst_mapping.py tests/test_pipeline.py::test_run_pipeline_generates_parsed_job_and_application_outputs -q
```

Expected: FAIL because renderers still depend on JSON.

- [x] **Step 3: Implement direct Typst rendering**

In `src/canisend/typst_mapping.py`, add:

```python
def render_modernpro_application_package_source(content: dict[str, Any]) -> str:
    job = content["job"]
    return f"""// Generated by CanISend as an editable Typst source.
// Edit this .typ file directly after reviewing evidence citations.
#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-coverletter:0.0.8": *

#show: statement.with(
  font-type: "PT Serif",
  margin: (left: 2cm, right: 2cm, top: 3cm, bottom: 2cm),
  name: [Applicant Name],
  address: [],
  contacts: (),
)

= Application Package

// CANISEND: section job_information
== Job Information
- Title: {typst_text(job["title"])}
- Institution: {typst_text(job["institution"])}
- Department: {typst_text(job["department"])}
- Deadline: {typst_text(job["deadline"])}
- Application URL: {typst_text(job["application_url"])}

// CANISEND: section fit_report
{markdown_to_typst(content["fit_report"])}

// CANISEND: section cover_letter
{markdown_to_typst(content["cover_letter"])}

// CANISEND: section cv_tailoring_notes
{markdown_to_typst(content["cv_tailoring_notes"])}

// CANISEND: section criteria_checklist
{markdown_to_typst(content["criteria_checklist"])}
"""
```

Replace `render_modernpro_cover_letter_source` with a direct renderer that embeds `content["opening"]` and each section under markers. Add helpers:

```python
def typst_text(value: str) -> str:
    return str(value).replace("\\", "\\\\").replace("#", "\\#").replace("$", "\\$").replace("[", "\\[").replace("]", "\\]")


def markdown_to_typst(markdown_text: str) -> str:
    lines: list[str] = []
    for raw_line in markdown_text.splitlines():
        line = raw_line.rstrip()
        if line.startswith("### "):
            lines.append("=== " + typst_text(line[4:]))
        elif line.startswith("## "):
            lines.append("== " + typst_text(line[3:]))
        elif line.startswith("# "):
            lines.append("= " + typst_text(line[2:]))
        else:
            lines.append(typst_text(line))
    return "\n".join(lines).strip()
```

- [x] **Step 4: Update pipeline call**

Change:

```python
written.append(_write_text(typst_dir / "application_package.typ", render_modernpro_application_package_source()))
```

to:

```python
written.append(_write_text(typst_dir / "application_package.typ", render_modernpro_application_package_source(application_package_content)))
```

Keep writing `cover_letter_content.json` and `application_package_content.json` as compatibility/debug files for this release.

- [x] **Step 5: Run tests to verify pass**

Run:

```bash
uv run pytest tests/test_typst_mapping.py tests/test_pipeline.py tests/test_typst.py -q
```

Expected: PASS.

- [x] **Step 6: Commit**

Run:

```bash
git add src/canisend/typst_mapping.py src/canisend/pipeline.py tests/test_typst_mapping.py tests/test_pipeline.py
git commit -m "feat(typst): render editable source files directly"
```

---

## Task 5: Implement Local Orchestrator Core

**Files:**
- Create: `src/canisend/orchestrator.py`
- Create: `tests/test_orchestrator.py`

- [ ] **Step 1: Write failing plan validation and dry-run tests**

Create `tests/test_orchestrator.py` with:

```python
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

import pytest
import yaml

from canisend.orchestrator import OrchestrationError, load_orchestration_plan, run_orchestration


def write_plan(path: Path, data: dict) -> Path:
    path.write_text(yaml.safe_dump(data, sort_keys=False), encoding="utf-8")
    return path


def base_job(tmp_path: Path) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "job"
    job_dir.mkdir(parents=True)
    (job_dir / "parsed_job.json").write_text('{"title": "Lecturer"}\n', encoding="utf-8")
    (job_dir / "05_criteria_checklist.md").write_text("# Criteria\n", encoding="utf-8")
    profile = workspace / "profile" / "generated"
    profile.mkdir(parents=True)
    (profile / "cv.evidence.md").write_text("# Evidence\n", encoding="utf-8")
    return workspace, job_dir


def test_load_orchestration_plan_rejects_duplicate_task_ids(tmp_path):
    plan_path = write_plan(
        tmp_path / "plan.yaml",
        {
            "workers": {"echo": {"command": sys.executable, "max_parallel_tasks": 1}},
            "tasks": [
                {"id": "review", "worker": "echo", "role": "a", "inputs": [], "outputs": ["a.md"]},
                {"id": "review", "worker": "echo", "role": "b", "inputs": [], "outputs": ["b.md"]},
            ],
        },
    )

    with pytest.raises(OrchestrationError, match="Duplicate task id"):
        load_orchestration_plan(plan_path)


def test_run_orchestration_dry_run_returns_execution_order(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    plan_path = write_plan(
        tmp_path / "plan.yaml",
        {
            "workers": {"echo": {"command": sys.executable, "max_parallel_tasks": 1}},
            "tasks": [
                {
                    "id": "review",
                    "worker": "echo",
                    "role": "job_parser_reviewer",
                    "privacy_tier": 1,
                    "inputs": ["parsed_job.json"],
                    "outputs": ["orchestration/reviews/review.md"],
                    "writes": ["orchestration/reviews/review.md"],
                }
            ],
        },
    )

    result = run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, dry_run=True)

    assert result.ok
    assert result.dry_run
    assert result.task_statuses["review"] == "ready"
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
uv run pytest tests/test_orchestrator.py -q
```

Expected: FAIL because `canisend.orchestrator` does not exist.

- [ ] **Step 3: Implement dataclasses and plan validation**

Create `src/canisend/orchestrator.py` with:

```python
from __future__ import annotations

from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
from dataclasses import dataclass, field
from datetime import UTC, datetime
import json
from pathlib import Path
import shlex
import shutil
import subprocess
from threading import Lock
from typing import Any
from uuid import uuid4

import yaml


class OrchestrationError(ValueError):
    pass


@dataclass(frozen=True)
class WorkerConfig:
    name: str
    command: str
    max_parallel_tasks: int = 1
    supports_native_subagents: bool = False
    privacy_tier_limit: int = 1
    timeout_seconds: int = 300


@dataclass(frozen=True)
class OrchestrationTask:
    id: str
    worker: str
    role: str
    privacy_tier: int = 1
    inputs: tuple[str, ...] = ()
    outputs: tuple[str, ...] = ()
    writes: tuple[str, ...] = ()
    depends_on: tuple[str, ...] = ()
    agent_count: int = 1


@dataclass(frozen=True)
class OrchestrationPlan:
    workers: dict[str, WorkerConfig]
    tasks: dict[str, OrchestrationTask]


@dataclass(frozen=True)
class OrchestrationResult:
    ok: bool
    run_dir: Path | None
    dry_run: bool
    task_statuses: dict[str, str]
```

Implement `load_orchestration_plan(path)` to parse YAML, build dataclasses, reject duplicate IDs, missing workers, missing dependencies, cycles, write conflicts without dependencies, invalid privacy tiers, and worker `max_parallel_tasks < 1`.

- [ ] **Step 4: Implement dry-run path**

Implement:

```python
def run_orchestration(
    *,
    workspace: Path,
    job_dir: Path,
    plan_path: Path,
    dry_run: bool = False,
    allow_private_sources: bool = False,
    allow_provider_backed: bool = False,
    fail_fast: bool = False,
    run_id: str | None = None,
) -> OrchestrationResult:
    plan = load_orchestration_plan(plan_path)
    _validate_privacy_flags(plan, allow_private_sources=allow_private_sources, allow_provider_backed=allow_provider_backed)
    _validate_inputs(plan, workspace=workspace, job_dir=job_dir)
    if dry_run:
        return OrchestrationResult(ok=True, run_dir=None, dry_run=True, task_statuses={task_id: "ready" for task_id in plan.tasks})
    return _execute_plan(
        plan,
        workspace=workspace,
        job_dir=job_dir,
        plan_path=plan_path,
        fail_fast=fail_fast,
        run_id=run_id,
    )
```

- [ ] **Step 5: Run tests to verify pass**

Run:

```bash
uv run pytest tests/test_orchestrator.py::test_load_orchestration_plan_rejects_duplicate_task_ids tests/test_orchestrator.py::test_run_orchestration_dry_run_returns_execution_order -q
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/canisend/orchestrator.py tests/test_orchestrator.py
git commit -m "feat(orchestrator): validate local agent plans"
```

---

## Task 6: Add Orchestrator Concurrent Execution

**Files:**
- Modify: `src/canisend/orchestrator.py`
- Modify: `tests/test_orchestrator.py`

- [ ] **Step 1: Add failing execution tests**

Append to `tests/test_orchestrator.py`:

```python
def test_run_orchestration_executes_worker_and_writes_artifacts(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    worker = tmp_path / "worker.py"
    worker.write_text(
        "import sys\n"
        "prompt = sys.stdin.read()\n"
        "print('RESULT:' + prompt.splitlines()[0])\n",
        encoding="utf-8",
    )
    plan_path = write_plan(
        tmp_path / "plan.yaml",
        {
            "workers": {"python": {"command": f"{sys.executable} {worker}", "max_parallel_tasks": 1}},
            "tasks": [
                {
                    "id": "review",
                    "worker": "python",
                    "role": "job_parser_reviewer",
                    "privacy_tier": 1,
                    "inputs": ["parsed_job.json"],
                    "outputs": ["orchestration/reviews/review.md"],
                    "writes": ["orchestration/reviews/review.md"],
                }
            ],
        },
    )

    result = run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, run_id="run-test")

    assert result.ok
    assert result.task_statuses["review"] == "succeeded"
    assert (job_dir / "orchestration" / "runs" / "run-test" / "tasks" / "review" / "stdout.txt").exists()
    assert "RESULT:Role: job_parser_reviewer" in (job_dir / "orchestration" / "reviews" / "review.md").read_text()


def test_run_orchestration_runs_independent_tasks_in_parallel_for_one_worker(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    worker = tmp_path / "slow_worker.py"
    worker.write_text(
        "import sys, time\n"
        "time.sleep(0.4)\n"
        "print('done')\n",
        encoding="utf-8",
    )
    plan_path = write_plan(
        tmp_path / "plan.yaml",
        {
            "workers": {"python": {"command": f"{sys.executable} {worker}", "max_parallel_tasks": 2}},
            "tasks": [
                {"id": "a", "worker": "python", "role": "r", "inputs": ["parsed_job.json"], "outputs": ["orchestration/reviews/a.md"], "writes": ["orchestration/reviews/a.md"]},
                {"id": "b", "worker": "python", "role": "r", "inputs": ["parsed_job.json"], "outputs": ["orchestration/reviews/b.md"], "writes": ["orchestration/reviews/b.md"]},
            ],
        },
    )

    started = time.monotonic()
    result = run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, run_id="parallel")
    elapsed = time.monotonic() - started

    assert result.ok
    assert result.task_statuses == {"a": "succeeded", "b": "succeeded"}
    assert elapsed < 0.75


def test_run_orchestration_skips_downstream_after_failure(tmp_path):
    workspace, job_dir = base_job(tmp_path)
    worker = tmp_path / "fail_worker.py"
    worker.write_text("import sys\nsys.exit(2)\n", encoding="utf-8")
    plan_path = write_plan(
        tmp_path / "plan.yaml",
        {
            "workers": {"python": {"command": f"{sys.executable} {worker}", "max_parallel_tasks": 1}},
            "tasks": [
                {"id": "a", "worker": "python", "role": "r", "inputs": ["parsed_job.json"], "outputs": ["orchestration/reviews/a.md"], "writes": ["orchestration/reviews/a.md"]},
                {"id": "b", "worker": "python", "role": "r", "depends_on": ["a"], "inputs": ["parsed_job.json"], "outputs": ["orchestration/reviews/b.md"], "writes": ["orchestration/reviews/b.md"]},
            ],
        },
    )

    result = run_orchestration(workspace=workspace, job_dir=job_dir, plan_path=plan_path, run_id="failed")

    assert not result.ok
    assert result.task_statuses["a"] == "failed"
    assert result.task_statuses["b"] == "skipped"
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
uv run pytest tests/test_orchestrator.py -q
```

Expected: FAIL because execution path is not implemented.

- [ ] **Step 3: Implement execution**

In `src/canisend/orchestrator.py`, implement:

- `_execute_plan` using `ThreadPoolExecutor(max_workers=sum(worker.max_parallel_tasks for worker in plan.workers.values()))`;
- per-worker in-flight counts;
- dependency-ready scheduling loop;
- `_run_task` that writes `prompt.md`, runs `subprocess.run(shlex.split(worker.command), input=prompt, text=True, capture_output=True, timeout=worker.timeout_seconds)`;
- stdout/stderr/status artifacts;
- promotion of stdout to the first declared output when the command succeeds;
- skipped downstream statuses when dependencies fail.

Use prompt header:

```python
def _task_prompt(task: OrchestrationTask, *, workspace: Path, job_dir: Path) -> str:
    return "\n".join(
        [
            f"Role: {task.role}",
            f"Task: {task.id}",
            f"Privacy tier: {task.privacy_tier}",
            f"Agent count: {task.agent_count}",
            "",
            "Inputs:",
            *_input_blocks(task, workspace=workspace, job_dir=job_dir),
        ]
    )
```

- [ ] **Step 4: Run tests to verify pass**

Run:

```bash
uv run pytest tests/test_orchestrator.py -q
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/canisend/orchestrator.py tests/test_orchestrator.py
git commit -m "feat(orchestrator): run local agent tasks concurrently"
```

---

## Task 7: Expose `canisend orchestrate`

**Files:**
- Modify: `src/canisend/cli.py`
- Modify: `tests/test_cli.py`

- [ ] **Step 1: Add failing CLI tests**

Append to `tests/test_cli.py`:

```python
def test_orchestrate_dry_run_lists_ready_tasks(tmp_path):
    workspace = tmp_path / "workspace"
    job_dir = workspace / "jobs" / "job"
    job_dir.mkdir(parents=True)
    (job_dir / "parsed_job.json").write_text('{"title": "Lecturer"}\n', encoding="utf-8")
    worker = tmp_path / "worker.py"
    worker.write_text("print('ok')\n", encoding="utf-8")
    plan = tmp_path / "plan.yaml"
    plan.write_text(
        f"""
workers:
  python:
    command: "{sys.executable} {worker}"
    max_parallel_tasks: 1
tasks:
  - id: review
    worker: python
    role: job_parser_reviewer
    privacy_tier: 1
    inputs:
      - parsed_job.json
    outputs:
      - orchestration/reviews/review.md
    writes:
      - orchestration/reviews/review.md
""",
        encoding="utf-8",
    )
    runner = CliRunner()

    result = runner.invoke(
        app,
        ["orchestrate", "--workspace", str(workspace), "--job", "job", "--plan", str(plan), "--dry-run"],
    )

    assert result.exit_code == 0
    assert "review: ready" in result.output
```

Add `import sys` at the top of `tests/test_cli.py` if missing.

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
uv run pytest tests/test_cli.py::test_orchestrate_dry_run_lists_ready_tasks -q
```

Expected: FAIL because command does not exist.

- [ ] **Step 3: Add CLI command**

In `src/canisend/cli.py`, import:

```python
from canisend.orchestrator import OrchestrationError, run_orchestration
```

Add command:

```python
@app.command("orchestrate")
def orchestrate(
    job: Path = typer.Option(..., "--job", help="Job folder path or slug."),
    plan: Path = typer.Option(..., "--plan", help="Local orchestration YAML plan."),
    workspace: Path = typer.Option(Path("."), "--workspace", help="User workspace directory containing canisend.yaml."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Validate and print task readiness without launching workers."),
    allow_private_sources: bool = typer.Option(False, "--allow-private-sources", help="Allow Tier 2 private-source tasks declared in the plan."),
    allow_provider_backed: bool = typer.Option(False, "--allow-provider-backed", help="Allow Tier 3 provider-backed tasks declared in the plan."),
    fail_fast: bool = typer.Option(False, "--fail-fast", help="Stop scheduling unrelated tasks after the first failure."),
) -> None:
    """Coordinate multiple local agent CLI workers for one job."""
    config = load_workspace_config(workspace)
    try:
        result = run_orchestration(
            workspace=config.root,
            job_dir=config.job_dir(job),
            plan_path=plan,
            dry_run=dry_run,
            allow_private_sources=allow_private_sources,
            allow_provider_backed=allow_provider_backed,
            fail_fast=fail_fast,
        )
    except OrchestrationError as exc:
        typer.echo(str(exc), err=True)
        raise typer.Exit(code=1) from exc
    if result.run_dir is not None:
        typer.echo(f"Orchestration run: {result.run_dir}")
    for task_id, status in sorted(result.task_statuses.items()):
        typer.echo(f"{task_id}: {status}")
    if not result.ok:
        raise typer.Exit(code=1)
```

- [ ] **Step 4: Run test to verify pass**

Run:

```bash
uv run pytest tests/test_cli.py::test_orchestrate_dry_run_lists_ready_tasks tests/test_orchestrator.py -q
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/canisend/cli.py tests/test_cli.py
git commit -m "feat(cli): expose local agent orchestration"
```

---

## Task 8: Update Docs, Skills, And Resource Contracts

**Files:**
- Modify: `README.md`
- Modify: `canisend_v1_proposal.md`
- Modify: `skills/canisend/SKILL.md`
- Modify: `agent-skills/canisend/SKILL.md`
- Modify: `skills/canisend/references/agent-orchestration.md`
- Modify: `agent-skills/canisend/references/agent-orchestration.md`
- Modify: `skills/canisend/references/file-contracts.md`
- Modify: `agent-skills/canisend/references/file-contracts.md`
- Modify: `skills/canisend/references/typst-profile.md`
- Modify: `agent-skills/canisend/references/typst-profile.md`
- Modify: `tests/test_repository_contract.py`
- Modify: `tests/test_skill_distribution.py`

- [ ] **Step 1: Add failing documentation contract tests**

Update `tests/test_repository_contract.py`:

```python
assert "canisend orchestrate" in readme
assert "typst/cover_letter.typ" in readme
assert "cover_letter_content.json" not in readme
assert "directly edit `typst/cover_letter.typ`" in readme
```

Update agent orchestration contract assertions:

```python
orchestration = (references / "agent-orchestration.md").read_text()
assert "canisend orchestrate" in orchestration
assert "max_parallel_tasks" in orchestration
assert "agent_count" in orchestration
assert "supports_native_subagents" in orchestration
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
uv run pytest tests/test_repository_contract.py tests/test_skill_distribution.py -q
```

Expected: FAIL because docs still describe JSON as the editing contract.

- [ ] **Step 3: Update README and references**

Make these content changes:

- Replace primary references to `cover_letter_content.json` and `application_package_content.json` with `typst/cover_letter.typ` and `typst/application_package.typ`.
- Add a section under Agent Usage explaining `canisend orchestrate`.
- Document worker fields: `command`, `max_parallel_tasks`, `supports_native_subagents`, `privacy_tier_limit`, and task `agent_count`.
- Explain that JSON content files, if emitted, are compatibility/debug outputs only.
- Mirror relevant reference updates under both `skills/canisend/references/` and `agent-skills/canisend/references/`.

- [ ] **Step 4: Run docs tests to verify pass**

Run:

```bash
uv run pytest tests/test_repository_contract.py tests/test_skill_distribution.py -q
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add README.md canisend_v1_proposal.md skills/canisend agent-skills/canisend tests/test_repository_contract.py tests/test_skill_distribution.py
git commit -m "docs(agent): document orchestration and Typst editing"
```

---

## Task 9: Full Verification And Cleanup

**Files:**
- Inspect all changed files.

- [ ] **Step 1: Run focused test suite**

Run:

```bash
uv run pytest tests/test_job_import.py tests/test_jobs.py tests/test_material_review.py tests/test_typst_mapping.py tests/test_pipeline.py tests/test_typst.py tests/test_orchestrator.py tests/test_cli.py tests/test_repository_contract.py tests/test_skill_distribution.py -q
```

Expected: PASS.

- [ ] **Step 2: Run full test suite**

Run:

```bash
uv run pytest -q
```

Expected: PASS.

- [ ] **Step 3: Check repository diff and private-file safety**

Run:

```bash
git status --short
git diff --check
```

Expected: only intended source, tests, docs, and lockfile changes; no whitespace errors; no private `profile/`, `jobs/`, `job_leads/`, `.env`, or real PDFs staged.

- [ ] **Step 4: Commit final cleanup if needed**

If formatting, docs, or tests required cleanup after prior commits:

```bash
git add README.md canisend_v1_proposal.md pyproject.toml uv.lock src/canisend tests skills agent-skills
git commit -m "chore: finalize orchestrator implementation"
```

- [ ] **Step 5: Summarize verification evidence**

Record in the final response:

- the branch name;
- commits created;
- focused tests run;
- full test result;
- any tests not run and why.
