from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import yaml

from academic_prep.evidence import EvidenceReference, load_generated_evidence
from academic_prep.llm import load_llm_config, provider_from_config
from academic_prep.materials import ApplicationMaterials, generate_materials_with_provider
from academic_prep.parse import parse_job_advert, parse_job_advert_with_provider


def run_pipeline(
    job_dir: Path,
    profile_dir: Path = Path("profile"),
    use_llm_parser: bool = False,
    use_llm_drafts: bool = False,
    prompt_dir: Path = Path("prompts"),
) -> list[Path]:
    metadata_path = job_dir / "job.yaml"
    advert_path = job_dir / "job_advert.md"
    metadata = yaml.safe_load(metadata_path.read_text(encoding="utf-8"))
    advert_text = advert_path.read_text(encoding="utf-8")

    parsed_job = _parse_job(advert_text, metadata, use_llm_parser=use_llm_parser, prompt_dir=prompt_dir)
    evidence = load_generated_evidence(profile_dir)
    materials = _materials(
        parsed_job,
        evidence,
        use_llm_drafts=use_llm_drafts,
        prompt_dir=prompt_dir,
    )
    final_package = _final_package(parsed_job, materials)
    written = [
        _write_json(job_dir / "parsed_job.json", parsed_job),
        _write_text(job_dir / "01_job_summary.md", _job_summary(parsed_job)),
        _write_text(job_dir / "02_fit_report.md", materials.fit_report),
        _write_text(job_dir / "03_cover_letter_draft.md", materials.cover_letter_draft),
        _write_text(job_dir / "04_cv_tailoring_notes.md", materials.cv_tailoring_notes),
        _write_text(job_dir / "05_criteria_checklist.md", materials.criteria_checklist),
        _write_text(job_dir / "06_final_application_package.md", final_package),
    ]

    typst_dir = job_dir / "typst"
    written.append(_write_text(typst_dir / "cover_letter.typ", _modernpro_coverletter_source(materials.cover_letter_draft)))
    written.append(_write_text(typst_dir / "application_package.typ", _modernpro_package_source(final_package)))

    metadata["status"] = "packaged"
    metadata_path.write_text(yaml.safe_dump(metadata, sort_keys=False), encoding="utf-8")
    return written


def _parse_job(
    advert_text: str,
    metadata: dict[str, Any],
    *,
    use_llm_parser: bool,
    prompt_dir: Path,
) -> dict[str, Any]:
    if not use_llm_parser:
        return parse_job_advert(advert_text, metadata)

    prompt_text = (prompt_dir / "job_parser.md").read_text(encoding="utf-8")
    provider = provider_from_config(load_llm_config())
    return parse_job_advert_with_provider(
        advert_text=advert_text,
        metadata=metadata,
        provider=provider,
        prompt_text=prompt_text,
    )


def _materials(
    parsed_job: dict[str, Any],
    evidence: list[EvidenceReference],
    *,
    use_llm_drafts: bool,
    prompt_dir: Path,
) -> ApplicationMaterials:
    if not use_llm_drafts:
        return ApplicationMaterials(
            fit_report=_fit_report(parsed_job, evidence),
            cover_letter_draft=_cover_letter(parsed_job),
            cv_tailoring_notes=_cv_notes(parsed_job),
            criteria_checklist=_criteria_checklist(parsed_job, evidence),
        )

    provider = provider_from_config(load_llm_config())
    return generate_materials_with_provider(
        parsed_job=parsed_job,
        evidence=evidence,
        provider=provider,
        prompt_dir=prompt_dir,
    )


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


def _fit_report(parsed_job: dict[str, Any], evidence: list[EvidenceReference]) -> str:
    essential = "\n".join(f"- {item['criterion']}" for item in parsed_job["essential_criteria"]) or "- No essential criteria extracted."
    evidence_summary = _evidence_summary(evidence)
    return f"""# Fit Report

## Extracted Essential Criteria

{essential}

## Evidence Review

{evidence_summary}

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


def _criteria_checklist(parsed_job: dict[str, Any], evidence: list[EvidenceReference]) -> str:
    evidence_source = _first_evidence_source(evidence)
    evidence_text = _first_evidence_text(evidence)
    rows = [
        "| Criterion | Coverage | Evidence Source | Risk | Suggested Improvement |",
        "|---|---|---|---|---|",
    ]
    for item in parsed_job["essential_criteria"]:
        coverage = "partial" if evidence else "missing"
        rows.append(f"| {item['criterion']} | {coverage} | {evidence_source} | High | Review evidence: {evidence_text} |")
    for item in parsed_job["desirable_criteria"]:
        coverage = "partial" if evidence else "missing"
        rows.append(f"| {item['criterion']} | {coverage} | {evidence_source} | Medium | Review evidence: {evidence_text} |")
    if len(rows) == 2:
        rows.append("| No criteria extracted | missing | Not available | High | Review the advert manually. |")
    return "# Criteria Coverage Checklist\n\n" + "\n".join(rows) + "\n"


def _evidence_summary(evidence: list[EvidenceReference]) -> str:
    if not evidence:
        return "Manual profile evidence review is required. Strong-fit claims must cite profile files and sections before use in final materials."
    lines = ["Generated profile evidence is available:"]
    for item in evidence:
        lines.append(f"- `{item.source_file}#{item.section}`: {item.text}")
    return "\n".join(lines)


def _first_evidence_source(evidence: list[EvidenceReference]) -> str:
    if not evidence:
        return "Not yet linked"
    item = evidence[0]
    return f"`{item.source_file}#{item.section}`"


def _first_evidence_text(evidence: list[EvidenceReference]) -> str:
    if not evidence:
        return "Add explicit evidence from profile files."
    return evidence[0].text


def _final_package(parsed_job: dict[str, Any], materials: ApplicationMaterials) -> str:
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

{materials.fit_report.strip()}

## Cover Letter Draft

{materials.cover_letter_draft.strip()}

## CV Tailoring Notes

{materials.cv_tailoring_notes.strip()}

## Criteria Coverage Checklist

{materials.criteria_checklist.strip()}

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


def _modernpro_coverletter_source(markdown_text: str) -> str:
    body = _markdown_to_typst(markdown_text)
    return f"""// Generated by AAP Copilot. Edit source Markdown before final submission.
#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-coverletter:0.0.8": *

#show: coverletter.with(
  font-type: "PT Serif",
  margin: (left: 2cm, right: 2cm, top: 3cm, bottom: 2cm),
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

{body}
"""


def _modernpro_package_source(markdown_text: str) -> str:
    body = _markdown_to_typst(markdown_text)
    return f"""// Generated by AAP Copilot. Edit source Markdown before final submission.
#import "@preview/fontawesome:0.6.0": *
#import "@preview/modernpro-coverletter:0.0.8": *

#show: statement.with(
  font-type: "PT Serif",
  margin: (left: 2cm, right: 2cm, top: 3cm, bottom: 2cm),
  name: [Applicant Name],
  address: [],
  contacts: (),
)

{body}
"""


def _markdown_to_typst(markdown_text: str) -> str:
    lines: list[str] = []
    for line in markdown_text.splitlines():
        if line.startswith("# "):
            lines.append("= " + line[2:])
        elif line.startswith("## "):
            lines.append("== " + line[3:])
        else:
            lines.append(line)
    return "\n".join(lines).strip() + "\n"
