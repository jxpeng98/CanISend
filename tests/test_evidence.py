from academic_prep.evidence import extract_typst_evidence


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
