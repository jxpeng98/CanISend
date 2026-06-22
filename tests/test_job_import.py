from __future__ import annotations

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
        self.path = path
        self.pages = [FakePage("")]


class FakeResponse:
    def __init__(
        self, body: str, content_type: str = "text/html; charset=utf-8"
    ) -> None:
        self.body = body.encode("utf-8")
        self.headers = {"Content-Type": content_type}

    def __enter__(self) -> "FakeResponse":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        return None

    def read(self) -> bytes:
        return self.body


@pytest.mark.parametrize(
    ("suffix", "body"),
    [
        (".md", "# Lecturer\n\nEssential criteria:\n- PhD\n"),
        (".txt", "Lecturer\n\nEssential criteria:\nPhD\n"),
    ],
)
def test_import_advert_file_preserves_markdown_and_text(
    tmp_path: Path, suffix: str, body: str
) -> None:
    advert = tmp_path / f"advert{suffix}"
    advert.write_text(body, encoding="utf-8")

    imported = import_advert_file(advert)

    assert imported.text == body
    assert imported.status == "advert_imported"
    assert imported.notes == ""


def test_extract_pdf_text_uses_reader_factory(tmp_path: Path) -> None:
    pdf = tmp_path / "jd.pdf"
    pdf.write_bytes(b"%PDF fake")

    text = extract_pdf_text(pdf, reader_factory=FakeReader)

    assert "Lecturer in Economics" in text
    assert "Essential criteria" in text


def test_import_advert_file_adds_pdf_provenance_header_and_notes(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    pdf = tmp_path / "job-description.pdf"
    pdf.write_bytes(b"%PDF fake")

    monkeypatch.setattr(
        "canisend.job_import.extract_pdf_text",
        lambda path: "Lecturer in Economics\nEssential criteria\nPhD",
    )

    imported = import_advert_file(pdf)

    assert imported.status == "advert_imported"
    assert imported.text.startswith(
        "<!-- Imported from local PDF: job-description.pdf. "
        "Review extracted text before relying on parsed criteria. -->\n\n"
    )
    assert "Lecturer in Economics" in imported.text
    assert imported.text.endswith("PhD\n")
    assert imported.notes == (
        "Imported from local PDF job-description.pdf; "
        "review extracted text before relying on parsed criteria."
    )


def test_extract_pdf_text_rejects_empty_text(tmp_path: Path) -> None:
    pdf = tmp_path / "jd.pdf"
    pdf.write_bytes(b"%PDF fake")

    with pytest.raises(JobImportError, match="No text could be extracted"):
        extract_pdf_text(pdf, reader_factory=EmptyReader)


def test_validate_fetch_url_rejects_missing_or_non_http_url() -> None:
    with pytest.raises(JobImportError, match="--fetch-url requires --source-url"):
        validate_fetch_url("")
    with pytest.raises(JobImportError, match="Only http:// and https:// URLs"):
        validate_fetch_url("file:///tmp/job.html")
    with pytest.raises(JobImportError, match="Fetch URL must include a host"):
        validate_fetch_url("https:///jobs/123")

    assert (
        validate_fetch_url("https://example.edu/jobs/123")
        == "https://example.edu/jobs/123"
    )


def test_extract_html_text_removes_scripts_styles_and_tags() -> None:
    html = """
    <html><head><style>.x{}</style><script>alert(1)</script></head>
    <body><h1>Lecturer in Economics</h1><noscript>Enable JavaScript</noscript>
    <p>Essential criteria: PhD.</p></body></html>
    """

    text = extract_html_text(html)

    assert "Lecturer in Economics" in text
    assert "Essential criteria: PhD." in text
    assert "alert" not in text
    assert ".x" not in text
    assert "Enable JavaScript" not in text
    assert "<h1>" not in text


def test_fetch_advert_from_url_reads_html_with_injected_opener() -> None:
    def opener(request, timeout):
        assert request.full_url == "https://example.edu/jobs/123"
        assert timeout == 30
        return FakeResponse(
            "<html><body><h1>Lecturer</h1><p>PhD required.</p></body></html>"
        )

    imported = fetch_advert_from_url("https://example.edu/jobs/123", opener=opener)

    assert imported.status == "advert_imported"
    assert "Lecturer" in imported.text
    assert "PhD required." in imported.text
    assert "Fetched from https://example.edu/jobs/123" in imported.notes


def test_fetch_advert_from_url_rejects_non_html_content_type() -> None:
    def opener(request, timeout):
        assert request.full_url == "https://example.edu/jobs/123"
        return FakeResponse(
            '{"title":"Lecturer"}',
            content_type="application/json",
        )

    with pytest.raises(JobImportError, match="Fetched URL did not return HTML"):
        fetch_advert_from_url("https://example.edu/jobs/123", opener=opener)
