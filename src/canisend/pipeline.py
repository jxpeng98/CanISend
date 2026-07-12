from __future__ import annotations

from dataclasses import replace
import hashlib
import json
from datetime import UTC, datetime
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
from canisend.match_views import load_current_structured_match_views
from canisend.parse import parse_job_advert, parse_job_advert_with_provider
from canisend.resource_files import read_resource_text
from canisend.stages.parse_stage import build_deterministic_parse_candidate
from canisend.typst_mapping import (
    build_application_package_content,
    build_cover_letter_content,
    render_modernpro_application_package_source,
    render_modernpro_cover_letter_source,
)
from canisend.workspace import load_workspace_config


TYPST_GENERATION_MANIFEST = ".canisend-generated.json"
TYPST_GENERATION_MANIFEST_VERSION = 1
APPLICATION_GATE_REPORT = "application_gate_report.json"


def run_pipeline(
    job_dir: Path,
    profile_dir: Path = Path("profile"),
    use_llm_parser: bool = False,
    use_llm_drafts: bool = False,
    prompt_dir: Path = Path("prompts"),
    workspace: Path | None = None,
) -> list[Path]:
    metadata_path = job_dir / "job.yaml"
    advert_path = job_dir / "job_advert.md"
    metadata = yaml.safe_load(metadata_path.read_text(encoding="utf-8"))
    advert_text = advert_path.read_text(encoding="utf-8")

    parsed_job = (
        _parse_job(advert_text, metadata, use_llm_parser=True, prompt_dir=prompt_dir)
        if use_llm_parser
        else build_deterministic_parse_candidate(job_dir)
    )
    evidence = load_generated_evidence(profile_dir)
    style_context = _style_context(metadata)
    materials = _materials(
        parsed_job,
        evidence,
        use_llm_drafts=use_llm_drafts,
        prompt_dir=prompt_dir,
        style_context=style_context,
    )
    structured_criteria = None
    if (
        workspace is not None
        and not use_llm_drafts
        and _uses_workspace_profile(workspace, profile_dir)
    ):
        structured_views = load_current_structured_match_views(
            workspace,
            job_dir,
            parsed_job=parsed_job,
        )
        if structured_views is not None:
            materials = replace(
                materials,
                fit_report=structured_views.fit_report,
                criteria_checklist=structured_views.criteria_checklist,
            )
            structured_criteria = structured_views.criteria_review
    if use_llm_drafts:
        provider = provider_from_config(load_llm_config())
        final_package = generate_final_package_with_provider(
            parsed_job=parsed_job,
            materials=materials,
            evidence=evidence,
            provider=provider,
            prompt_dir=prompt_dir,
            style_context=style_context,
        )
    else:
        final_package = _final_package(parsed_job, materials)
    material_review = build_material_review_checklist(
        parsed_job,
        materials,
        structured_criteria=structured_criteria,
    )
    _invalidate_application_gate_report(job_dir)
    written = [
        _write_json_preserving_equivalent(job_dir / "parsed_job.json", parsed_job),
        _write_text(job_dir / "00_preparation_questions.md", _preparation_questions(parsed_job, metadata)),
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
    written.append(_write_json(typst_dir / "application_package_content.json", application_package_content))
    written.extend(
        _write_protected_typst_sources(
            typst_dir,
            {
                "cover_letter.typ": render_modernpro_cover_letter_source(cover_letter_content),
                "application_package.typ": render_modernpro_application_package_source(
                    application_package_content
                ),
            },
        )
    )

    metadata["status"] = "packaged"
    metadata["updated_at"] = _utc_now()
    metadata_path.write_text(yaml.safe_dump(metadata, sort_keys=False), encoding="utf-8")
    return written


def _uses_workspace_profile(workspace: Path, profile_dir: Path) -> bool:
    """Require one profile provenance before mixing Match views into a package."""

    try:
        configured = load_workspace_config(workspace).path("profile_dir")
        return profile_dir.expanduser().resolve() == configured.expanduser().resolve()
    except (OSError, ValueError, yaml.YAMLError):
        return False


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
    style_context: str,
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
        style_context=style_context,
    )


def _write_json(path: Path, data: dict[str, Any]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return path


def _write_json_preserving_equivalent(path: Path, data: dict[str, Any]) -> Path:
    if path.is_file() and not path.is_symlink():
        try:
            existing = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError):
            existing = None
        if existing == data:
            return path
    return _write_json(path, data)


def _write_text(path: Path, text: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


def _invalidate_application_gate_report(job_dir: Path) -> None:
    report_path = job_dir / APPLICATION_GATE_REPORT
    if not report_path.is_file():
        return
    try:
        report = json.loads(report_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return
    if not isinstance(report, dict):
        return
    report["status"] = "STALE"
    report["invalidated_at"] = _utc_now()
    report["invalidation_reason"] = "application artifacts were regenerated"
    _write_json(report_path, report)


def _write_protected_typst_sources(typst_dir: Path, sources: dict[str, str]) -> list[Path]:
    """Write generated Typst without overwriting a user-edited primary source."""
    manifest_path = typst_dir / TYPST_GENERATION_MANIFEST
    manifest = _load_typst_generation_manifest(manifest_path)
    file_records = manifest["files"]
    written: list[Path] = []

    for filename, source in sources.items():
        primary_path = typst_dir / filename
        candidate_path = primary_path.with_name(f"{primary_path.stem}.generated{primary_path.suffix}")
        record = file_records.get(filename)
        if not isinstance(record, dict):
            record = {}

        primary_hash = _manifest_hash(record.get("primary_hash"))
        candidate_hash = _manifest_hash(record.get("candidate_hash"))

        generated_hash = _hash_text(source)
        if not primary_path.exists():
            written.append(_write_text(primary_path, source))
            primary_hash = generated_hash
        else:
            current_hash = _hash_file(primary_path)
            adopted_candidate = candidate_hash is not None and current_hash == candidate_hash
            safe_primary = current_hash == primary_hash or current_hash == generated_hash
            if safe_primary or adopted_candidate:
                written.append(_write_text(primary_path, source))
                primary_hash = generated_hash
                if adopted_candidate:
                    candidate_is_unchanged = (
                        candidate_path.is_file()
                        and _hash_file(candidate_path) == candidate_hash
                    )
                    if candidate_is_unchanged:
                        candidate_path.unlink()
                    candidate_hash = None
            else:
                written.append(_write_text(candidate_path, source))
                candidate_hash = generated_hash

        file_records[filename] = {
            "primary_hash": primary_hash,
            "candidate_hash": candidate_hash,
        }

    _write_json(manifest_path, manifest)
    return written


def _load_typst_generation_manifest(path: Path) -> dict[str, Any]:
    if path.is_file():
        try:
            loaded = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError):
            loaded = None
        if (
            isinstance(loaded, dict)
            and type(loaded.get("version")) is int
            and loaded["version"] == TYPST_GENERATION_MANIFEST_VERSION
            and isinstance(loaded.get("files"), dict)
        ):
            return {
                "version": TYPST_GENERATION_MANIFEST_VERSION,
                "files": dict(loaded["files"]),
            }
    return {
        "version": TYPST_GENERATION_MANIFEST_VERSION,
        "files": {},
    }


def _manifest_hash(value: Any) -> str | None:
    if not isinstance(value, str):
        return None
    normalized = value.strip().lower()
    if len(normalized) != 64 or any(
        character not in "0123456789abcdef" for character in normalized
    ):
        return None
    return normalized


def _hash_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def _hash_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _utc_now() -> str:
    return datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")


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


def _style_context(metadata: dict[str, Any]) -> str:
    variant = str(metadata.get("english_variant") or "needs_confirmation")
    style = str(metadata.get("writing_style") or "needs_confirmation")
    variant_label = {
        "uk": "UK English",
        "us": "US English",
        "needs_confirmation": "needs confirmation: ask whether to use US English or UK English",
    }.get(variant, variant)
    style_label = style if style != "needs_confirmation" else "needs confirmation: ask for preferred writing style"
    return "\n".join(
        [
            "## Language and Style Preferences",
            "",
            f"- English variant: {variant_label}",
            f"- Writing style: {style_label}",
            "- Preserve evidence citations and do not invent details to satisfy style preferences.",
        ]
    )


def _preparation_questions(parsed_job: dict[str, Any], metadata: dict[str, Any]) -> str:
    return f"""# Preparation Questions

Use this as a short grill me checklist before treating generated materials as final.

## Language And Style

- Confirm whether the materials should use US English or UK English.
- Confirm writing style: direct, warm, formal, concise, evidence-led, or another target voice.
- Confirm whether the role or institution expects local spelling, title conventions, or sector-specific tone.

## Content Details To Confirm

- What is the specific motivation for {parsed_job["institution"]} and this {parsed_job["title"]} role?
- Which 2-3 evidence-backed achievements should be most visible?
- Which criteria are real strengths, and which are stretch or risk areas?
- Which teaching, research, service, leadership, or industry details should not be overclaimed?
- Are there any details the user wants excluded from cover letters, CV notes, or statements?

## Current Metadata

- English variant: {metadata.get("english_variant", "needs_confirmation")}
- Writing style: {metadata.get("writing_style", "needs_confirmation")}
- Status: {metadata.get("status", "unknown")}
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
