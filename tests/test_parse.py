from canisend.parse import parse_job_advert


def test_parse_job_advert_accepts_markdown_headings_and_bullet_variants():
    advert = """# Lecturer in Economics

+ Department: Department of Economics
* Location: United Kingdom
- Salary: Grade 7
* Contract: Permanent
- Role type: Lecturer
* Research fields: Economics, Econometrics
- Teaching fields: Statistics, Econometrics
* Required documents: CV, Cover letter

## Essential Criteria
1. PhD or near completion in Economics or related field
2) Strong research record in econometrics
+ Ability to teach quantitative methods

### Desirable Criteria
- Experience supervising dissertations
1) Experience with curriculum design
"""
    metadata = {
        "institution": "University X",
        "deadline": "2026-06-15",
        "source_url": "https://example.edu/jobs/123",
    }

    parsed = parse_job_advert(advert, metadata)

    assert parsed["title"] == "Lecturer in Economics"
    assert parsed["institution"] == "University X"
    assert parsed["department"] == "Department of Economics"
    assert parsed["location"] == "United Kingdom"
    assert parsed["salary"] == "Grade 7"
    assert parsed["contract_type"] == "Permanent"
    assert parsed["role_type"] == "Lecturer"
    assert parsed["research_fields"] == ["Economics", "Econometrics"]
    assert parsed["teaching_fields"] == ["Statistics", "Econometrics"]
    assert parsed["required_documents"] == ["CV", "Cover letter"]
    assert [item["criterion"] for item in parsed["essential_criteria"]] == [
        "PhD or near completion in Economics or related field",
        "Strong research record in econometrics",
        "Ability to teach quantitative methods",
    ]
    assert [item["criterion"] for item in parsed["desirable_criteria"]] == [
        "Experience supervising dissertations",
        "Experience with curriculum design",
    ]
