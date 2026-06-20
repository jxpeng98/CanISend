from __future__ import annotations

import json
import os
import sys


CITATION = "`profile/generated/cv.evidence.md#Teaching`"
FORBIDDEN_SECRET_ENV = ("OPENAI_API_KEY", "ANTHROPIC_API_KEY")


def main() -> None:
    for key in FORBIDDEN_SECRET_ENV:
        if os.environ.get(key):
            raise SystemExit(f"fake provider received secret env var: {key}")

    prompt = sys.stdin.read()
    heading = first_prompt_heading(prompt)
    if heading == "# Job Parser":
        print(json.dumps(parsed_job(), indent=2))
        return
    if heading == "# Profile Matcher":
        print(
            "# Fit Report\n\n"
            f"- Strong teaching fit: the profile includes econometrics teaching evidence ({CITATION}).\n"
            f"- Research fit is plausible for applied economics, but the user should verify current projects ({CITATION}).\n"
            "- Gap: add department-specific evidence after reviewing the full advert.\n"
        )
        return
    if heading == "# Cover Letter Writer":
        print(
            "# Cover Letter Draft\n\n"
            "Dear Selection Committee,\n\n"
            "I am writing to apply for the Lecturer in Applied Economics role.\n\n"
            "## Research Fit\n\n"
            f"My applied economics research agenda aligns with the role and should be checked against current projects ({CITATION}).\n\n"
            "## Teaching Fit\n\n"
            f"I can contribute to econometrics teaching using documented teaching experience ({CITATION}).\n\n"
            "## Departmental Contribution\n\n"
            f"I would support quantitative methods provision and programme development ({CITATION}).\n\n"
            "## Service and Leadership\n\n"
            f"I can contribute to collegial service where it is supported by the final CV ({CITATION}).\n\n"
            "Yours sincerely,\n\n"
            "[Applicant name]\n"
        )
        return
    if heading == "# CV Tailor":
        print(
            "# CV Tailoring Notes\n\n"
            f"- Move econometrics teaching evidence higher in the CV ({CITATION}).\n"
            f"- Foreground applied economics projects only where the profile supports them ({CITATION}).\n"
        )
        return
    if heading == "# Criteria Checker":
        print(
            "# Criteria Coverage Checklist\n\n"
            "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |\n"
            "|---|---|---|---|---|\n"
            f"| Evidence of teaching excellence in econometrics or quantitative methods | strong | {CITATION} | low | Keep this evidence visible in CV and cover letter. |\n"
            f"| Active research agenda in applied economics | partial | {CITATION} | medium | Add a research-project citation from the profile before final use. |\n"
        )
        return
    if heading == "# Package Builder":
        print(
            "# Final Application Package\n\n"
            "## Job Information\n\n"
            "- Title: Lecturer in Applied Economics\n"
            "- Institution: Example University\n\n"
            "## Application Strategy\n\n"
            f"Use the extracted criteria to decide the main application angle ({CITATION}).\n\n"
            "## Required Documents\n\n"
            "- [ ] CV\n- [ ] Cover letter\n- [ ] Research statement\n- [ ] Teaching statement\n\n"
            "## Manual Submission Notes\n\n"
            "The system has prepared materials only.\n"
        )
        return
    raise SystemExit("fake provider received an unknown prompt")


def first_prompt_heading(prompt: str) -> str:
    for raw_line in prompt.splitlines():
        line = raw_line.strip()
        if line.startswith("#"):
            return line
    return ""


def parsed_job() -> dict:
    return {
        "title": "Lecturer in Applied Economics",
        "institution": "Example University",
        "department": "Department of Economics",
        "location": "United Kingdom",
        "deadline": "2026-06-15",
        "salary": "Grade 7",
        "contract_type": "Permanent",
        "role_type": "Lecturer",
        "research_fields": ["Applied Economics", "Econometrics"],
        "teaching_fields": ["Econometrics", "Quantitative Methods"],
        "essential_criteria": [
            {
                "criterion": "PhD or near completion in Economics or a related field",
                "source_text": "PhD or near completion in Economics or a related field",
            },
            {
                "criterion": "Evidence of teaching excellence in econometrics or quantitative methods",
                "source_text": "Evidence of teaching excellence in econometrics or quantitative methods",
            },
            {
                "criterion": "Active research agenda in applied economics",
                "source_text": "Active research agenda in applied economics",
            },
        ],
        "desirable_criteria": [
            {
                "criterion": "Experience supervising dissertations",
                "source_text": "Experience supervising dissertations",
            }
        ],
        "required_documents": ["CV", "Cover letter", "Research statement", "Teaching statement"],
        "application_url": "https://www.jobs.ac.uk/job/EXA001/lecturer-in-applied-economics",
        "unknown_fields": [],
        "notes": "Example fixture output from fake_llm_provider.py",
    }


if __name__ == "__main__":
    main()
