from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import subprocess


APPLICATION_MATERIAL_RELATIVE_PATHS = [
    "00_preparation_questions.md",
    "02_fit_report.md",
    "03_cover_letter_draft.md",
    "04_cv_tailoring_notes.md",
    "05_criteria_checklist.md",
    "06_final_application_package.md",
    "07_material_review_checklist.md",
    "08_research_statement.md",
    "typst/cover_letter.typ",
    "typst/application_package.typ",
    "typst/research_statement.typ",
]


class GitTrackingError(RuntimeError):
    """Raised when generated application materials cannot be added to git."""


@dataclass(frozen=True)
class GitAddResult:
    files: list[Path]


def application_material_paths(job_dir: Path) -> list[Path]:
    """Return existing generated application materials that are safe to stage."""
    return [
        job_dir / relative_path
        for relative_path in APPLICATION_MATERIAL_RELATIVE_PATHS
        if (job_dir / relative_path).exists()
    ]


def git_add_application_materials(job_dir: Path, *, repo_dir: Path) -> GitAddResult:
    files = application_material_paths(job_dir)
    if not files:
        return GitAddResult(files=[])

    result = subprocess.run(
        ["git", "add", "-f", "--", *(str(path) for path in files)],
        cwd=repo_dir,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        message = result.stderr.strip() or result.stdout.strip() or "git add failed"
        raise GitTrackingError(message)
    return GitAddResult(files=files)
