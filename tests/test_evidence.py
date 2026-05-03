from typer.testing import CliRunner

from academic_prep.cli import app
from academic_prep.evidence import extract_profile_evidence, extract_typst_evidence


def test_extract_typst_evidence_from_modernpro_cv_blocks(tmp_path):
    typst_file = tmp_path / "cv.typ"
    typst_file.write_text(
        '#section("Education")\n'
        '#education(institution: [University X], major: [PhD Economics], date: "2022-now")\n'
        '#section("Experience")\n'
        '#job(position: "Teaching Assistant", institution: [University X], location: "UK", date: "2023")\n'
        '#section("Publications")\n'
        '+ @smith2025\n'
    )

    evidence = extract_typst_evidence(typst_file)

    assert evidence[0].source_file == str(typst_file)
    assert evidence[0].section == "Education"
    assert evidence[0].kind == "education"
    assert "University X" in evidence[0].text
    assert evidence[1].section == "Experience"
    assert evidence[1].kind == "job"
    assert "Teaching Assistant" in evidence[1].text
    assert evidence[2].section == "Publications"
    assert evidence[2].kind == "publication"
    assert "@smith2025" in evidence[2].text


def test_extract_profile_evidence_writes_generated_markdown_from_manifest(tmp_path):
    profile_dir = tmp_path / "profile"
    typst_dir = profile_dir / "typst"
    typst_dir.mkdir(parents=True)
    (profile_dir / "profile.yaml").write_text(
        "sources:\n"
        "  cv: typst/cv.typ\n"
        "generated:\n"
        "  cv_evidence: generated/cv.evidence.md\n"
    )
    (typst_dir / "cv.typ").write_text(
        '#section("Education")\n'
        '#education(institution: [University X], major: [PhD Economics], date: "2022-now")\n'
    )

    written = extract_profile_evidence(profile_dir)

    output = profile_dir / "generated" / "cv.evidence.md"
    assert written == [output]
    content = output.read_text()
    assert "# Evidence: cv" in content
    assert "## Education" in content
    assert "- `education`: institution: University X" in content


def test_extract_profile_evidence_cli_writes_generated_evidence(tmp_path):
    profile_dir = tmp_path / "profile"
    typst_dir = profile_dir / "typst"
    typst_dir.mkdir(parents=True)
    (profile_dir / "profile.yaml").write_text(
        "sources:\n"
        "  cv: typst/cv.typ\n"
        "generated:\n"
        "  cv_evidence: generated/cv.evidence.md\n"
    )
    (typst_dir / "cv.typ").write_text(
        '#section("Experience")\n'
        '#job(position: "Teaching Assistant", institution: [University X], location: "UK", date: "2023")\n'
    )
    runner = CliRunner()

    result = runner.invoke(app, ["extract-profile-evidence", "--profile-dir", str(profile_dir)])

    assert result.exit_code == 0
    assert "Generated 1 evidence files" in result.output
    assert "Teaching Assistant" in (profile_dir / "generated" / "cv.evidence.md").read_text()
