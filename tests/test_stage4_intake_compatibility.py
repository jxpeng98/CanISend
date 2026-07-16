from __future__ import annotations

import json

import pytest
from typer.testing import CliRunner

from canisend.cli import app
from canisend.job_import import fetch_advert_from_url


ADVERT = (
    "# Lecturer in Economics\n\n"
    "Required documents: CV, Cover letter\n\n"
    "Essential criteria:\n"
    "- PhD in Economics\n"
)


class _Response:
    def __init__(self, body: bytes, content_type: str, final_url: str) -> None:
        self.body = body
        self.headers = {"Content-Type": content_type}
        self.final_url = final_url

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        return None

    def read(self, size: int = -1) -> bytes:
        return self.body if size < 0 else self.body[:size]

    def geturl(self) -> str:
        return self.final_url


def _assert_application_workflow(job_dir, runner: CliRunner) -> None:
    result = runner.invoke(
        app,
        ["run", "--job", str(job_dir), "--no-git-add-materials"],
    )

    assert result.exit_code == 0, result.output
    parsed = json.loads((job_dir / "parsed_job.json").read_text(encoding="utf-8"))
    assert parsed["title"] == "Lecturer in Economics"
    assert parsed["required_documents"] == ["CV", "Cover letter"]
    assert parsed["essential_criteria"][0]["criterion"] == "PhD in Economics"
    assert (job_dir / "03_cover_letter_draft.md").is_file()
    assert (job_dir / "07_material_review_checklist.md").is_file()


@pytest.mark.parametrize(
    ("suffix", "file_body"),
    [
        (".txt", ADVERT.encode("utf-8")),
        (".pdf", b"%PDF synthetic fixture"),
    ],
)
def test_stage4_keeps_local_text_and_pdf_in_the_application_workflow(
    tmp_path, monkeypatch, suffix: str, file_body: bytes
) -> None:
    advert = tmp_path / f"advert{suffix}"
    advert.write_bytes(file_body)
    if suffix == ".pdf":
        monkeypatch.setattr("canisend.job_import.extract_pdf_text", lambda path: ADVERT)
    jobs_dir = tmp_path / "jobs"
    runner = CliRunner()

    intake = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer in Economics",
            "--institution",
            "Example University",
            "--deadline",
            "2026-08-31",
            "--advert-file",
            str(advert),
            "--jobs-dir",
            str(jobs_dir),
        ],
    )

    assert intake.exit_code == 0, intake.output
    _assert_application_workflow(
        jobs_dir / "2026-08-31_example-university_lecturer-in-economics",
        runner,
    )


@pytest.mark.parametrize(
    ("source_url", "content_type", "response_body"),
    [
        (
            "https://jobs.example.edu/lecturer.html",
            "text/html; charset=utf-8",
            (
                "<html><body><h1>Lecturer in Economics</h1>"
                "<p>Required documents: CV, Cover letter</p>"
                "<h2>Essential criteria:</h2><ul><li>- PhD in Economics</li></ul>"
                "</body></html>"
            ).encode("utf-8"),
        ),
        (
            "https://jobs.example.edu/lecturer.pdf",
            "application/pdf",
            b"%PDF synthetic fixture",
        ),
    ],
)
def test_stage4_keeps_explicit_html_and_pdf_urls_in_the_application_workflow(
    tmp_path,
    monkeypatch,
    source_url: str,
    content_type: str,
    response_body: bytes,
) -> None:
    monkeypatch.setattr(
        "canisend.job_import.extract_pdf_bytes",
        lambda body: ADVERT,
    )

    def bounded_fetch(url: str):
        return fetch_advert_from_url(
            url,
            opener=lambda request, timeout: _Response(
                response_body,
                content_type,
                request.full_url,
            ),
            resolver=lambda hostname: ("93.184.216.34",),
        )

    monkeypatch.setattr("canisend.jobs.fetch_advert_from_url", bounded_fetch)
    jobs_dir = tmp_path / "jobs"
    runner = CliRunner()

    intake = runner.invoke(
        app,
        [
            "new-job",
            "--title",
            "Lecturer in Economics",
            "--institution",
            "Example University",
            "--deadline",
            "2026-08-31",
            "--source-url",
            source_url,
            "--fetch-url",
            "--jobs-dir",
            str(jobs_dir),
        ],
    )

    assert intake.exit_code == 0, intake.output
    _assert_application_workflow(
        jobs_dir / "2026-08-31_example-university_lecturer-in-economics",
        runner,
    )
