from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import yaml

from academic_prep.parse import parse_job_advert


def run_pipeline(job_dir: Path) -> list[Path]:
    metadata_path = job_dir / "job.yaml"
    advert_path = job_dir / "job_advert.md"
    metadata = yaml.safe_load(metadata_path.read_text(encoding="utf-8"))
    advert_text = advert_path.read_text(encoding="utf-8")

    parsed_job = parse_job_advert(advert_text, metadata)
    written = [
        _write_json(job_dir / "parsed_job.json", parsed_job),
        _write_text(job_dir / "01_job_summary.md", _job_summary(parsed_job)),
        _write_text(job_dir / "02_fit_report.md", _fit_report(parsed_job)),
        _write_text(job_dir / "03_cover_letter_draft.md", _cover_letter(parsed_job)),
        _write_text(job_dir / "04_cv_tailoring_notes.md", _cv_notes(parsed_job)),
        _write_text(job_dir / "05_criteria_checklist.md", _criteria_checklist(parsed_job)),
        _write_text(job_dir / "06_final_application_package.md", _final_package(parsed_job)),
    ]

    typst_dir = job_dir / "typst"
    written.append(_write_text(typst_dir / "cover_letter.typ", _typst_document(_cover_letter(parsed_job))))
    written.append(_write_text(typst_dir / "application_package.typ", _typst_document(_final_package(parsed_job))))

    metadata["status"] = "packaged"
    metadata_path.write_text(yaml.safe_dump(metadata, sort_keys=False), encoding="utf-8")
    return written


def _write_json(path: Path, data: dict[str, Any]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return path


def _write_text(path: Path, text: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


def _job_summary(parsed_job: dict[str, Any]) -> str:
    return f"""# Job Summary

- Title: {parsed_job["title"]}
- Institution: {parsed_job["institution"]}
- Department: {parsed_job["department"]}
- Location: {parsed_job["location"]}
- Deadline: {parsed_job["deadline"]}
- Contract: {parsed_job["contract_type"]}
- Salary: {parsed_job["salary"]}
- Required documents: {", ".join(parsed_job["required_documents"]) or "unknown"}
"""


def _fit_report(parsed_job: dict[str, Any]) -> str:
    essential = "\n".join(f"- {item['criterion']}" for item in parsed_job["essential_criteria"]) or "- No essential criteria extracted."
    return f"""# Fit Report

## Extracted Essential Criteria

{essential}

## Evidence Review

Manual profile evidence review is required. Strong-fit claims must cite profile files and sections before use in final materials.

## Application Risks

- Confirm that every essential criterion is explicitly covered in the CV or cover letter.
- Add evidence references before treating this report as final.
"""


def _cover_letter(parsed_job: dict[str, Any]) -> str:
    return f"""# Cover Letter Draft

Dear Selection Committee,

I am writing to apply for the position of {parsed_job["title"]} at {parsed_job["institution"]}.

## Research Fit

[Add evidence-based research fit using profile file and section references.]

## Teaching Fit

[Add evidence-based teaching fit using profile file and section references.]

## Departmental Contribution

[Add specific departmental fit after reviewing the advert and department context.]

## Service and Leadership

[Add only supported service or leadership evidence.]

Yours sincerely,

[Applicant name]
"""


def _cv_notes(parsed_job: dict[str, Any]) -> str:
    teaching_fields = ", ".join(parsed_job["teaching_fields"]) or "the advertised teaching areas"
    research_fields = ", ".join(parsed_job["research_fields"]) or "the advertised research areas"
    return f"""# CV Tailoring Notes

1. Move evidence related to {teaching_fields} higher if this is a teaching-heavy role.
2. Foreground research projects related to {research_fields}.
3. Make essential criteria visible in the CV before submission.
4. Add profile evidence references before using these notes as final guidance.
"""


def _criteria_checklist(parsed_job: dict[str, Any]) -> str:
    rows = [
        "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |",
        "|---|---|---|---|---|",
    ]
    for item in parsed_job["essential_criteria"]:
        rows.append(f"| {item['criterion']} | missing | Not yet linked | High | Add explicit evidence from profile files. |")
    for item in parsed_job["desirable_criteria"]:
        rows.append(f"| {item['criterion']} | missing | Not yet linked | Medium | Add supporting evidence if available. |")
    if len(rows) == 2:
        rows.append("| No criteria extracted | missing | Not available | High | Review the advert manually. |")
    return "# Criteria Coverage Checklist\n\n" + "\n".join(rows) + "\n"


def _final_package(parsed_job: dict[str, Any]) -> str:
    return f"""# Final Application Package

## Job Information

- Title: {parsed_job["title"]}
- Institution: {parsed_job["institution"]}
- Department: {parsed_job["department"]}
- Deadline: {parsed_job["deadline"]}
- Application URL: {parsed_job["application_url"]}

## Application Strategy

Use the extracted criteria to decide the main application angle after profile evidence has been linked.

## Fit Report Summary

See `02_fit_report.md`.

## Cover Letter Draft

See `03_cover_letter_draft.md`.

## CV Tailoring Notes

See `04_cv_tailoring_notes.md`.

## Criteria Coverage Checklist

See `05_criteria_checklist.md`.

## Required Documents Checklist

{_required_documents_list(parsed_job)}

## Manual Submission Notes

The system has prepared materials only. The user must manually review and submit the application.

## Remaining Actions Before Submission

- Link every major claim to profile evidence.
- Confirm required documents on the university portal.
- Review sensitive declarations manually.
"""


def _required_documents_list(parsed_job: dict[str, Any]) -> str:
    documents = parsed_job["required_documents"]
    if not documents:
        return "- Required documents were not extracted; check the advert manually."
    return "\n".join(f"- [ ] {document}" for document in documents)


def _typst_document(markdown_text: str) -> str:
    escaped = markdown_text.replace("\\", "\\\\").replace('"', '\\"')
    return f"""// Generated by AAP Copilot. Edit source Markdown before final submission.
#set document(author: "AAP Copilot")
#block[
  {escaped}
]
"""
