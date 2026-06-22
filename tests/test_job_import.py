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
        self,
        body: str,
        content_type: str = "text/html; charset=utf-8",
        content_length: str | None = None,
    ) -> None:
        self.body = body.encode("utf-8")
        self.headers = {"Content-Type": content_type}
        self.read_sizes: list[int] = []
        if content_length is not None:
            self.headers["Content-Length"] = content_length

    def __enter__(self) -> "FakeResponse":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        return None

    def read(self, size: int = -1) -> bytes:
        self.read_sizes.append(size)
        if size < 0:
            return self.body
        return self.body[:size]


def _minimal_pdf_bytes(text: str) -> bytes:
    stream = f"BT /F1 24 Tf 72 720 Td ({text}) Tj ET\n".encode("ascii")
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            b"/Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
        ),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Length " + str(len(stream)).encode("ascii") + b" >>\nstream\n"
        b"" + stream + b"endstream",
    ]
    pdf = b"%PDF-1.4\n"
    offsets = [0]
    for number, body in enumerate(objects, start=1):
        offsets.append(len(pdf))
        pdf += f"{number} 0 obj\n".encode("ascii") + body + b"\nendobj\n"
    startxref = len(pdf)
    xref_rows = [b"0000000000 65535 f "]
    xref_rows.extend(
        f"{offset:010d} 00000 n ".encode("ascii") for offset in offsets[1:]
    )
    pdf += b"xref\n0 6\n" + b"\n".join(xref_rows) + b"\n"
    pdf += b"trailer\n<< /Size 6 /Root 1 0 R >>\n"
    pdf += f"startxref\n{startxref}\n%%EOF\n".encode("ascii")
    return pdf


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


def test_import_advert_file_rejects_missing_text_file(tmp_path: Path) -> None:
    missing = tmp_path / "missing.md"

    with pytest.raises(JobImportError, match="Could not read job advert file"):
        import_advert_file(missing)


def test_import_advert_file_wraps_invalid_utf8_text_file(tmp_path: Path) -> None:
    advert = tmp_path / "advert.txt"
    advert.write_bytes(b"\xff")

    with pytest.raises(JobImportError, match="Could not read job advert file"):
        import_advert_file(advert)


def test_extract_pdf_text_uses_reader_factory(tmp_path: Path) -> None:
    pdf = tmp_path / "jd.pdf"
    pdf.write_bytes(b"%PDF fake")

    text = extract_pdf_text(pdf, reader_factory=FakeReader)

    assert "Lecturer in Economics" in text
    assert "Essential criteria" in text


def test_extract_pdf_text_reads_small_real_pdf(tmp_path: Path) -> None:
    pdf = tmp_path / "real.pdf"
    pdf.write_bytes(_minimal_pdf_bytes("Lecturer PDF smoke"))

    text = extract_pdf_text(pdf)

    assert "Lecturer PDF smoke" in text


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


def test_validate_fetch_url_rejects_credentials() -> None:
    with pytest.raises(JobImportError, match="must not include credentials"):
        validate_fetch_url("https://user@example.edu/jobs/123")
    with pytest.raises(JobImportError, match="must not include credentials"):
        validate_fetch_url("https://user:secret@example.edu/jobs/123")


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


def test_extract_html_text_preserves_list_table_and_quote_boundaries() -> None:
    html = """
    <main>
      <h1>Lecturer</h1>
      <ul><li>Teach</li><li>Research</li></ul>
      <table><tr><th>Criteria</th><td>PhD</td></tr></table>
      <blockquote>Apply online</blockquote>
    </main>
    """

    assert extract_html_text(html).splitlines() == [
        "Lecturer",
        "Teach",
        "Research",
        "Criteria",
        "PhD",
        "Apply online",
    ]


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


def test_fetch_advert_from_url_redacts_query_and_strips_fragment() -> None:
    def opener(request, timeout):
        assert request.full_url == (
            "https://example.edu/jobs/123?token=secret&session=abc"
        )
        return FakeResponse("<html><body><h1>Lecturer</h1></body></html>")

    imported = fetch_advert_from_url(
        "https://example.edu/jobs/123?token=secret&session=abc#details",
        opener=opener,
    )

    assert "https://example.edu/jobs/123?redacted" in imported.text
    assert "https://example.edu/jobs/123?redacted" in imported.notes
    for sensitive_part in ("token=secret", "session=abc", "#details"):
        assert sensitive_part not in imported.text
        assert sensitive_part not in imported.notes
    assert "<!--" not in imported.text


def test_fetch_advert_from_url_uses_safe_non_comment_provenance() -> None:
    def opener(request, timeout):
        return FakeResponse("<html><body><h1>Lecturer</h1></body></html>")

    imported = fetch_advert_from_url(
        "https://example.edu/jobs/-->?token=secret",
        opener=opener,
    )

    assert "<!--" not in imported.text
    assert "-->" not in imported.text
    assert "%3E" in imported.text
    assert "token=secret" not in imported.notes


def test_fetch_advert_from_url_rejects_non_html_content_type() -> None:
    def opener(request, timeout):
        assert request.full_url == "https://example.edu/jobs/123"
        return FakeResponse(
            '{"title":"Lecturer"}',
            content_type="application/json",
        )

    with pytest.raises(JobImportError, match="Fetched URL did not return HTML"):
        fetch_advert_from_url("https://example.edu/jobs/123", opener=opener)


def test_fetch_advert_from_url_rejects_content_length_above_limit() -> None:
    def opener(request, timeout):
        return FakeResponse(
            "<html><body>too large</body></html>",
            content_length="1024",
        )

    with pytest.raises(JobImportError, match="larger than the configured limit"):
        fetch_advert_from_url(
            "https://example.edu/jobs/123",
            opener=opener,
            max_bytes=10,
        )


def test_fetch_advert_from_url_rejects_body_above_limit_without_length() -> None:
    response = FakeResponse("<html><body>too large</body></html>")

    def opener(request, timeout):
        return response

    with pytest.raises(JobImportError, match="larger than the configured limit"):
        fetch_advert_from_url(
            "https://example.edu/jobs/123",
            opener=opener,
            max_bytes=10,
        )
    assert response.read_sizes == [11]
