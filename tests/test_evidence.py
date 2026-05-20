from typer.testing import CliRunner

from canisend.cli import app
from canisend.evidence import (
    extract_profile_evidence,
    extract_typst_evidence,
    load_generated_evidence,
)


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


def test_extract_typst_evidence_from_modernpro_multiline_blocks(tmp_path):
    typst_file = tmp_path / "cv.typ"
    typst_file.write_text(
        '#section("Teaching")\n'
        "#dated-entry(\n"
        "  title: [Teaching Fellow in Econometrics],\n"
        "  location: [Example University],\n"
        "  date: [2024--2026],\n"
        "  description: [Led quantitative methods seminars and supervised dissertations.],\n"
        ")\n"
        "#entry(\n"
        "  title: [Curriculum Design],\n"
        "  description: [Designed applied econometrics assessment.],\n"
        ")\n"
        '#section("Service")\n'
        "#event(title: [Admissions Interview Panel], role: [Panel member], date: [2025])\n"
    )

    evidence = extract_typst_evidence(typst_file)

    assert [item.kind for item in evidence] == ["dated-entry", "entry", "event"]
    assert evidence[0].section == "Teaching"
    assert "Teaching Fellow in Econometrics" in evidence[0].text
    assert "Led quantitative methods seminars" in evidence[0].text
    assert "Curriculum Design" in evidence[1].text
    assert evidence[2].section == "Service"
    assert "Admissions Interview Panel" in evidence[2].text


def test_extract_typst_evidence_captures_statement_paragraphs_as_claims(tmp_path):
    typst_file = tmp_path / "research_statement.typ"
    typst_file.write_text(
        "#import \"@preview/modernpro-cv:1.0.0\": *\n"
        "= Research Agenda\n"
        "My research develops applied econometrics for labour markets.\n"
        "\n"
        "- Working paper on wage inequality and job mobility.\n"
    )

    evidence = extract_typst_evidence(typst_file)

    assert len(evidence) == 1
    assert evidence[0].section == "Research Agenda"
    assert evidence[0].kind == "statement"
    assert "applied econometrics" in evidence[0].text
    assert "Working paper on wage inequality" in evidence[0].text


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
    assert "- [cv-001] `education`: institution: University X" in content


def test_extract_profile_evidence_writes_stable_item_ids_and_loads_item_level_references(tmp_path):
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
        '#section("Teaching")\n'
        '#job(position: "Teaching Assistant", institution: [University X], date: "2023")\n'
        '#entry(title: [Curriculum Design], description: [Designed applied econometrics assessment.])\n'
    )

    extract_profile_evidence(profile_dir)

    content = (profile_dir / "generated" / "cv.evidence.md").read_text()
    assert "- [cv-001] `job`: position: Teaching Assistant" in content
    assert "- [cv-002] `entry`: title: Curriculum Design" in content

    references = load_generated_evidence(profile_dir)
    assert references[0].source_file == "profile/generated/cv.evidence.md"
    assert references[0].section == "Teaching"
    assert references[0].item_id == "cv-001"
    assert references[1].item_id == "cv-002"


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
