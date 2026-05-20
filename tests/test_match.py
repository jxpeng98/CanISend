from canisend.evidence import EvidenceReference
from canisend.match import (
    CriterionMatch,
    EvidenceIndex,
    coverage_label,
    format_criteria_checklist,
    format_cv_notes,
    format_fit_report,
    format_cover_letter_draft,
)


def test_evidence_index_groups_by_kind():
    items = [
        EvidenceReference(
            source_file="profile/generated/cv.evidence.md",
            section="Teaching",
            item_id="cv-001",
            text="`job`: Teaching Assistant for Econometrics",
        ),
        EvidenceReference(
            source_file="profile/generated/cv.evidence.md",
            section="Research",
            item_id="cv-002",
            text="`publication`: Applied Economics Paper in Journal X",
        ),
    ]
    index = EvidenceIndex(items)
    assert "teaching" in index._by_kind
    assert "research" in index._by_kind
    assert any("Teaching Assistant" in item.text for item in index._by_kind["teaching"])


def test_evidence_index_search_finds_teaching():
    items = [
        EvidenceReference(
            source_file="profile/generated/cv.evidence.md",
            section="Teaching",
            item_id="cv-001",
            text="`job`: Teaching Assistant for Econometrics",
        ),
    ]
    index = EvidenceIndex(items)
    matches = index.search("Evidence of teaching excellence")
    assert len(matches) >= 1
    assert any("Teaching Assistant" in m.text for m in matches)


def test_evidence_index_search_finds_research():
    items = [
        EvidenceReference(
            source_file="profile/generated/cv.evidence.md",
            section="Research",
            item_id="cv-002",
            text="`publication`: Applied Economics Paper in Journal X",
        ),
    ]
    index = EvidenceIndex(items)
    matches = index.search("Active research agenda in applied economics")
    assert len(matches) >= 1


def test_match_criterion_strong_coverage():
    items = [
        EvidenceReference(
            source_file="profile/generated/cv.evidence.md",
            section="Teaching",
            item_id="cv-001",
            text="`job`: Teaching Assistant for Econometrics",
        ),
        EvidenceReference(
            source_file="profile/generated/cv.evidence.md",
            section="Teaching",
            item_id="cv-002",
            text="`teaching`: Module leader for Quantitative Methods",
        ),
    ]
    index = EvidenceIndex(items)
    result = index.match_criterion("Evidence of teaching excellence in econometrics")
    assert result.coverage == "strong"
    assert len(result.matched_items) >= 2


def test_match_criterion_partial_coverage():
    items = [
        EvidenceReference(
            source_file="profile/generated/cv.evidence.md",
            section="Teaching",
            item_id="cv-001",
            text="`job`: Teaching Assistant for Econometrics",
        ),
    ]
    index = EvidenceIndex(items)
    result = index.match_criterion("Evidence of teaching excellence")
    assert result.coverage == "partial"
    assert len(result.matched_items) == 1


def test_match_criterion_missing_coverage():
    index = EvidenceIndex([])
    result = index.match_criterion("Experience in university administration")
    assert result.coverage == "missing"
    assert len(result.matched_items) == 0


def test_coverage_label():
    items = [
        EvidenceReference(
            source_file="profile/generated/cv.evidence.md",
            section="Teaching",
            item_id="cv-001",
            text="`job`: Teaching Assistant",
        ),
        EvidenceReference(
            source_file="profile/generated/cv.evidence.md",
            section="Teaching",
            item_id="cv-002",
            text="`teaching`: Module leader",
        ),
    ]
    assert coverage_label(items[:2], "teaching excellence") == "strong"
    assert coverage_label(items[:1], "teaching excellence") == "partial"
    assert coverage_label([], "teaching excellence") == "weak"
    assert coverage_label([], "xyzzy unknown criteria") == "missing"


def test_format_fit_report():
    matches = [
        CriterionMatch(
            criterion="PhD in Economics",
            coverage="strong",
            matched_items=[
                EvidenceReference(
                    source_file="profile/generated/cv.evidence.md",
                    section="Education",
                    item_id="cv-001",
                    text="`education`: PhD in Economics",
                )
            ],
            suggestion="Evidence found.",
        ),
        CriterionMatch(
            criterion="Grant writing experience",
            coverage="missing",
            matched_items=[],
            suggestion="No evidence available.",
        ),
    ]
    evidence: list[EvidenceReference] = []
    report = format_fit_report(matches, [], evidence)
    assert "STRONG" in report
    assert "MISSING" in report
    assert "PhD in Economics" in report
    assert "Grant writing experience" in report


def test_format_criteria_checklist():
    matches = [
        CriterionMatch(
            criterion="PhD in Economics",
            coverage="strong",
            matched_items=[
                EvidenceReference(
                    source_file="profile/generated/cv.evidence.md",
                    section="Education",
                    item_id="cv-001",
                    text="`education`: PhD in Economics",
                )
            ],
            suggestion="Evidence found.",
        ),
    ]
    checklist = format_criteria_checklist(matches, [])
    assert "PhD in Economics" in checklist
    assert "strong" in checklist
    assert "profile/generated/cv.evidence.md#Education/cv-001" in checklist


def test_format_cv_notes():
    matches = [
        CriterionMatch(
            criterion="Evidence of teaching excellence",
            coverage="strong",
            matched_items=[
                EvidenceReference(
                    source_file="profile/generated/cv.evidence.md",
                    section="Teaching",
                    item_id="cv-001",
                    text="`job`: Teaching Assistant for Econometrics",
                )
            ],
            suggestion="Evidence found.",
        ),
    ]
    parsed_job = {
        "title": "Lecturer",
        "institution": "University X",
        "teaching_fields": ["Econometrics"],
        "research_fields": ["Economics"],
    }
    notes = format_cv_notes(parsed_job, matches)
    assert "teaching-heavy" in notes.lower()
    assert "Teaching Assistant" in notes


def test_format_cover_letter_draft():
    matches = [
        CriterionMatch(
            criterion="Evidence of teaching excellence",
            coverage="strong",
            matched_items=[
                EvidenceReference(
                    source_file="profile/generated/cv.evidence.md",
                    section="Teaching",
                    item_id="cv-001",
                    text="`job`: Teaching Assistant for Econometrics",
                )
            ],
            suggestion="Evidence found.",
        ),
    ]
    parsed_job = {
        "title": "Lecturer",
        "institution": "University X",
    }
    draft = format_cover_letter_draft(parsed_job, matches)
    assert "Teaching Assistant" in draft
    assert "Dear Selection Committee" in draft
    assert "Yours sincerely" in draft
