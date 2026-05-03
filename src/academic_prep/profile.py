from pathlib import Path


PROFILE_TEMPLATES: dict[str, str] = {
    "cv.md": """# CV

## Education

## Academic Employment

## Research Experience

## Teaching Experience

## Service and Leadership
""",
    "publications.md": """# Publications

## Published Articles

## Working Papers

## Research Pipeline
""",
    "teaching_experience.md": """# Teaching Experience

## Modules

## Quantitative Methods Teaching

## Student Feedback

## Supervision
""",
    "research_statement.md": """# Research Statement

## Research Agenda

## Current Projects

## Future Plans
""",
    "teaching_statement.md": """# Teaching Statement

## Teaching Philosophy

## Evidence of Teaching Effectiveness

## Inclusive Teaching
""",
    "service_leadership.md": """# Service and Leadership

## Departmental Service

## Professional Service

## Leadership Evidence
""",
    "grants_awards.md": """# Grants and Awards

## Grants

## Awards

## Scholarships
""",
    "references.md": """# References

## Academic References
""",
    "personal_profile.yaml": """name: ""
email: ""
current_position: ""
primary_fields: []
secondary_fields: []
location_preferences: []
""",
}


TYPST_PROFILE_TEMPLATES: dict[str, str] = {
    "profile.yaml": """profile_mode: hybrid
canonical_evidence_dir: generated
sources:
  cv: typst/cv.typ
  cover_letter_base: typst/cover_letter_base.typ
  research_statement: typst/research_statement.typ
  teaching_statement: typst/teaching_statement.typ
generated:
  cv_evidence: generated/cv.evidence.md
  research_statement_evidence: generated/research_statement.evidence.md
  teaching_statement_evidence: generated/teaching_statement.evidence.md
privacy:
  commit_real_profile: false
""",
    "typst/cv.typ": """#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-cv:1.3.0": *

#show: cv-single.with(
  font-type: "PT Serif",
  name: [Applicant Name],
  address: [],
  contacts: (),
)

#section("Education")

#section("Research")

#section("Teaching")
""",
    "typst/cover_letter_base.typ": """#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-coverletter:0.0.8": *

#show: coverletter.with(
  font-type: "PT Serif",
  name: [Applicant Name],
  address: [],
  salutation: [Yours sincerely,],
  contacts: (),
  recipient: (
    start-title: [Dear Selection Committee,],
    cl-title: [Academic Job Application],
    date: [],
    department: [],
    institution: [],
    address: [],
    postcode: [],
  ),
)
""",
    "typst/research_statement.typ": """#import "@preview/modernpro-coverletter:0.0.8": *

#show: statement.with(
  font-type: "PT Serif",
  name: [Applicant Name],
  address: [],
  contacts: (),
)

= Research Statement
""",
    "typst/teaching_statement.typ": """#import "@preview/modernpro-coverletter:0.0.8": *

#show: statement.with(
  font-type: "PT Serif",
  name: [Applicant Name],
  address: [],
  contacts: (),
)

= Teaching Statement
""",
    "generated/.gitkeep": "",
}


def init_profile(profile_dir: Path, mode: str = "hybrid") -> list[Path]:
    if mode not in {"markdown", "typst", "hybrid"}:
        raise ValueError("profile mode must be markdown, typst, or hybrid")

    profile_dir.mkdir(parents=True, exist_ok=True)
    created: list[Path] = []

    if mode in {"markdown", "hybrid"}:
        created.extend(_write_templates(profile_dir, PROFILE_TEMPLATES))

    if mode in {"typst", "hybrid"}:
        created.extend(_write_templates(profile_dir, TYPST_PROFILE_TEMPLATES))

    return created


def _write_templates(profile_dir: Path, templates: dict[str, str]) -> list[Path]:
    created: list[Path] = []
    for filename, content in templates.items():
        path = profile_dir / filename
        if path.exists():
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        created.append(path)
    return created
