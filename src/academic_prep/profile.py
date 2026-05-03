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


def init_profile(profile_dir: Path) -> list[Path]:
    profile_dir.mkdir(parents=True, exist_ok=True)
    created: list[Path] = []

    for filename, content in PROFILE_TEMPLATES.items():
        path = profile_dir / filename
        if path.exists():
            continue
        path.write_text(content, encoding="utf-8")
        created.append(path)

    return created
