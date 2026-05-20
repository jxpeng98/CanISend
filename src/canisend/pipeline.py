from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import yaml

from canisend.evidence import EvidenceReference, load_generated_evidence
from canisend.llm import load_llm_config, provider_from_config
from canisend.match import (
    EvidenceIndex,
    format_cover_letter_draft,
    format_criteria_checklist,
    format_cv_notes,
    format_fit_report,
)
from canisend.materials import (
    ApplicationMaterials,
    generate_final_package_with_provider,
    generate_materials_with_provider,
)
from canisend.material_review import build_material_review_checklist
from canisend.parse import parse_job_advert, parse_job_advert_with_provider
from canisend.resource_files import read_resource_text
from canisend.typst_mapping import (
    build_application_package_content,
    build_cover_letter_content,
    render_modernpro_application_package_source,
    render_modernpro_cover_letter_source,
)


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
    if use_llm_drafts:
        provider = provider_from_config(load_llm_config())
        final_package = generate_final_package_with_provider(
            parsed_job=parsed_job,
            materials=materials,
            provider=provider,
            prompt_dir=prompt_dir,
        )
    else:
        final_package = _final_package(parsed_job, materials)
    material_review = build_material_review_checklist(parsed_job, materials)
    written = [
        _write_json(job_dir / "parsed_job.json", parsed_job),
        _write_text(job_dir / "01_job_summary.md", _job_summary(parsed_job)),
        _write_text(job_dir / "02_fit_report.md", materials.fit_report),
        _write_text(job_dir / "03_cover_letter_draft.md", materials.cover_letter_draft),
        _write_text(job_dir / "04_cv_tailoring_notes.md", materials.cv_tailoring_notes),
        _write_text(job_dir / "05_criteria_checklist.md", materials.criteria_checklist),
        _write_text(job_dir / "06_final_application_package.md", final_package),
        _write_text(job_dir / "07_material_review_checklist.md", material_review),
    ]

    typst_dir = job_dir / "typst"
    cover_letter_content = build_cover_letter_content(parsed_job, materials)
    application_package_content = build_application_package_content(parsed_job, materials, final_package)
    written.append(_write_json(typst_dir / "cover_letter_content.json", cover_letter_content))
    written.append(_write_text(typst_dir / "cover_letter.typ", render_modernpro_cover_letter_source(cover_letter_content)))
    written.append(_write_json(typst_dir / "application_package_content.json", application_package_content))
    written.append(_write_text(typst_dir / "application_package.typ", render_modernpro_application_package_source()))

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

    prompt_text = read_resource_text("prompts/job_parser.md", local_path=prompt_dir / "job_parser.md")
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
        index = EvidenceIndex(evidence)
        essential_matches = [
            index.match_criterion(item["criterion"])
            for item in parsed_job["essential_criteria"]
        ]
        desirable_matches = [
            index.match_criterion(item["criterion"])
            for item in parsed_job["desirable_criteria"]
        ]
        all_matches = essential_matches + desirable_matches
        return ApplicationMaterials(
            fit_report=format_fit_report(essential_matches, desirable_matches, evidence),
            cover_letter_draft=format_cover_letter_draft(parsed_job, all_matches),
            cv_tailoring_notes=format_cv_notes(parsed_job, all_matches),
            criteria_checklist=format_criteria_checklist(essential_matches, desirable_matches),
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
