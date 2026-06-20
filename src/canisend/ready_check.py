from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
import re

from canisend.evidence import load_generated_evidence
from canisend.materials import MaterialValidationError, validate_markdown_citations


REQUIRED_MARKDOWN_FILES = [
    "01_job_summary.md",
    "02_fit_report.md",
    "03_cover_letter_draft.md",
    "04_cv_tailoring_notes.md",
    "05_criteria_checklist.md",
    "06_final_application_package.md",
    "07_material_review_checklist.md",
]
REQUIRED_JSON_FILES = [
    "parsed_job.json",
    "typst/cover_letter_content.json",
    "typst/application_package_content.json",
]
REQUIRED_SOURCE_FILES = [
    "job.yaml",
    "job_advert.md",
]
PLACEHOLDER_RE = re.compile(r"\[[A-Za-z][^\]\n]{2,}\]")


@dataclass(frozen=True)
class PackageCheckIssue:
    path: str
    message: str


@dataclass(frozen=True)
class PackageCheckResult:
    job_dir: Path
    issues: list[PackageCheckIssue]

    @property
    def ok(self) -> bool:
        return not self.issues

    def output_lines(self) -> list[str]:
        if self.ok:
            return [f"Package check passed: {self.job_dir}"]
        lines = [f"Package check failed: {self.job_dir}"]
        lines.extend(f"- {issue.path}: {issue.message}" for issue in self.issues)
        return lines


def check_application_package(job_dir: Path, profile_dir: Path) -> PackageCheckResult:
    job_dir = job_dir.expanduser().resolve()
    profile_dir = profile_dir.expanduser().resolve()
    issues: list[PackageCheckIssue] = []

    if not job_dir.exists():
        return PackageCheckResult(
            job_dir=job_dir,
            issues=[PackageCheckIssue(str(job_dir), "job directory does not exist")],
        )

    for relative_path in REQUIRED_SOURCE_FILES:
        _require_file(job_dir, relative_path, issues)
    for relative_path in REQUIRED_MARKDOWN_FILES:
        path = _require_file(job_dir, relative_path, issues)
        if path is not None:
            _check_placeholders(path, relative_path, issues)
    for relative_path in REQUIRED_JSON_FILES:
        path = _require_file(job_dir, relative_path, issues)
        if path is not None:
            _check_json(path, relative_path, issues)

    evidence = load_generated_evidence(profile_dir)
    for relative_path in REQUIRED_MARKDOWN_FILES:
        path = job_dir / relative_path
        if not path.exists():
            continue
        try:
            validate_markdown_citations(relative_path, path.read_text(encoding="utf-8"), evidence)
        except MaterialValidationError as exc:
            issues.append(PackageCheckIssue(relative_path, str(exc)))

    return PackageCheckResult(job_dir=job_dir, issues=issues)


def _require_file(job_dir: Path, relative_path: str, issues: list[PackageCheckIssue]) -> Path | None:
    path = job_dir / relative_path
    if not path.exists():
        issues.append(PackageCheckIssue(relative_path, f"missing {relative_path}"))
        return None
    if not path.is_file():
        issues.append(PackageCheckIssue(relative_path, "expected a file"))
        return None
    return path


def _check_json(path: Path, relative_path: str, issues: list[PackageCheckIssue]) -> None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        issues.append(PackageCheckIssue(relative_path, f"invalid JSON: {exc.msg}"))
        return
    if not isinstance(value, dict):
        issues.append(PackageCheckIssue(relative_path, "expected a JSON object"))


def _check_placeholders(path: Path, relative_path: str, issues: list[PackageCheckIssue]) -> None:
    text = path.read_text(encoding="utf-8")
    for placeholder in _visible_placeholders(text):
        issues.append(PackageCheckIssue(relative_path, f"placeholder {placeholder} must be resolved"))


def _visible_placeholders(text: str) -> list[str]:
    placeholders: list[str] = []
    for match in PLACEHOLDER_RE.finditer(text):
        if match.end() < len(text) and text[match.end()] == "(":
            continue
        placeholders.append(match.group(0))
    return placeholders
