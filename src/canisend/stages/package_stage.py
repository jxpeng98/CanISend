from __future__ import annotations

import json
from pathlib import Path
from typing import Literal

from pydantic import ValidationError

from canisend.bundle_models import (
    PACKAGE_PROJECTION_PATHS,
    ArtifactBundleV1,
    BundleEntryV1,
)
from canisend.stage_models import ArtifactFingerprint
from canisend.stage_store import (
    StageStoreError,
    read_json_object,
    resolve_job_relative_path,
    sha256_bytes,
    sha256_file,
)
from canisend.user_file_store import (
    InvalidUserFileError,
    UnsafeUserFileError,
    load_strict_yaml,
    read_safe_bytes,
)


PACKAGE_BUNDLE_OUTPUT_PATH = "package_bundle.json"
PACKAGE_BUNDLE_SCHEMA = "canisend.artifact-bundle/v1"
PACKAGE_STAGE_INPUT_PATHS = (
    "application_brief.yaml",
    "application_decision.yaml",
    "cover_letter_draft.json",
    "criteria.json",
    "criterion_matches.json",
    "evidence_catalog.json",
    "job.yaml",
    "package_review_dispositions.yaml",
    "package_review_findings.json",
    "parsed_job.json",
    "required_document_plan.json",
    "review_dispositions.yaml",
    "review_findings.json",
)
PACKAGE_OPTIONAL_INPUT_PATHS = (
    "research_statement_draft.json",
    "research_statement_review_dispositions.yaml",
    "research_statement_review_findings.json",
)
PACKAGE_REQUIRED_ENTRY_PATHS = frozenset(
    {
        "00_preparation_questions.md",
        "01_job_summary.md",
        "02_fit_report.md",
        "03_cover_letter_draft.md",
        "04_cv_tailoring_notes.md",
        "05_criteria_checklist.md",
        "06_final_application_package.md",
        "07_material_review_checklist.md",
        "typst/application_package.typ",
        "typst/application_package_content.json",
        "typst/cover_letter.typ",
        "typst/cover_letter_content.json",
    }
)
PACKAGE_RESEARCH_ENTRY_PATHS = frozenset(
    {
        "08_research_statement.md",
        "typst/research_statement.typ",
        "typst/research_statement_content.json",
    }
)


class PackageStageError(ValueError):
    """A body-free Package stage validation failure."""


def package_input_artifacts(job_dir: Path) -> tuple[ArtifactFingerprint, ...]:
    artifacts: list[ArtifactFingerprint] = []
    for relative_path in (*PACKAGE_STAGE_INPUT_PATHS, *PACKAGE_OPTIONAL_INPUT_PATHS):
        path = resolve_job_relative_path(job_dir, relative_path)
        if relative_path in PACKAGE_OPTIONAL_INPUT_PATHS and not path.exists():
            continue
        if relative_path.endswith((".yaml", ".yml")):
            try:
                snapshot = read_safe_bytes(job_dir, relative_path, max_bytes=20_000_000)
            except UnsafeUserFileError as exc:
                raise PackageStageError("A user-owned Package input is missing or unsafe.") from exc
            artifacts.append(
                ArtifactFingerprint(
                    path=relative_path,
                    sha256=snapshot.sha256,
                    size_bytes=len(snapshot.data),
                )
            )
            continue
        try:
            metadata = path.lstat()
            if path.is_symlink() or not path.is_file() or metadata.st_nlink != 1:
                raise PackageStageError("A structured Package input is missing or unsafe.")
            artifacts.append(
                ArtifactFingerprint(
                    path=relative_path,
                    sha256=sha256_file(path),
                    size_bytes=metadata.st_size,
                )
            )
        except (OSError, StageStoreError) as exc:
            raise PackageStageError("A structured Package input is missing or unsafe.") from exc
    return tuple(sorted(artifacts, key=lambda item: item.path))


def package_input_fingerprint(job_dir: Path) -> str:
    artifacts = package_input_artifacts(job_dir)
    payload = [item.model_dump(mode="json") for item in artifacts]
    return sha256_bytes(
        json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
    )


def package_precondition_reasons(workspace: Path, job_dir: Path) -> tuple[str, ...]:
    from canisend.user_mutations import inspect_package_review_dispositions

    inspection = inspect_package_review_dispositions(workspace, job_dir)
    if (
        inspection.basis_status == "current"
        and inspection.readiness is not None
        and inspection.readiness.state == "reviewed"
    ):
        return ()
    reason = inspection.reason or "package.readiness_not_reviewed"
    return (f"input_not_ready:{reason.replace('.', '_')}",)


def build_package_bundle_candidate(
    workspace: Path,
    job_dir: Path,
    *,
    input_fingerprint: str,
    mode: Literal["guarded", "legacy_compatibility"] = "guarded",
) -> ArtifactBundleV1:
    if package_input_fingerprint(job_dir) != input_fingerprint:
        raise PackageStageError("Package inputs changed before bundle construction.")
    if mode == "guarded" and package_precondition_reasons(workspace, job_dir):
        raise PackageStageError("The application package is not fully reviewed.")

    from dataclasses import replace

    from canisend.decision_models import EvidenceCatalogV1
    from canisend.draft_views import (
        RESEARCH_STATEMENT_CONTENT_OUTPUT_PATH,
        RESEARCH_STATEMENT_MARKDOWN_OUTPUT_PATH,
        ReviewedResearchStatementViews,
        StructuredDraftViews,
        build_structured_cover_letter_content,
        build_structured_research_statement_content,
        load_current_reviewed_research_statement_views,
        load_current_structured_draft_views,
    )
    from canisend.evidence import EvidenceReference
    from canisend.material_review import build_material_review_checklist
    from canisend.match_views import load_current_structured_match_views
    from canisend.pipeline import (
        _final_package,
        _job_summary,
        _materials,
        _preparation_questions,
        _style_context,
    )
    from canisend.typst_mapping import (
        build_application_package_content,
        build_cover_letter_content,
        render_modernpro_application_package_source,
        render_modernpro_cover_letter_source,
        render_modernpro_research_statement_source,
    )

    try:
        parsed_job = read_json_object(job_dir / "parsed_job.json")
        metadata_snapshot = read_safe_bytes(job_dir, "job.yaml", max_bytes=20_000_000)
        metadata = load_strict_yaml(metadata_snapshot.data, max_bytes=20_000_000)
        catalog = EvidenceCatalogV1.model_validate(
            read_json_object(job_dir / "evidence_catalog.json")
        )
    except (
        InvalidUserFileError,
        UnsafeUserFileError,
        StageStoreError,
        ValidationError,
    ) as exc:
        raise PackageStageError("Package source artifacts are invalid or unsafe.") from exc
    evidence = [
        EvidenceReference(
            source_file=item.path,
            section=item.section,
            text=item.text,
            item_id=item.item_locator or "",
        )
        for item in catalog.items
    ]
    materials = _materials(
        parsed_job,
        evidence,
        use_llm_drafts=False,
        prompt_dir=workspace / "prompts",
        style_context=_style_context(metadata),
    )
    structured_criteria = None
    structured_draft_views: StructuredDraftViews | None = None
    research_statement_views: ReviewedResearchStatementViews | None = None
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
    structured_draft_views = load_current_structured_draft_views(
        workspace,
        job_dir,
        parsed_job=parsed_job,
    )
    if structured_draft_views is not None:
        materials = replace(
            materials,
            cover_letter_draft=structured_draft_views.markdown,
        )
    research_statement_views = load_current_reviewed_research_statement_views(
        workspace,
        job_dir,
        parsed_job=parsed_job,
    )

    final_package = _final_package(parsed_job, materials)
    material_review = build_material_review_checklist(
        parsed_job,
        materials,
        structured_criteria=structured_criteria,
    )
    cover_letter_content = (
        build_structured_cover_letter_content(parsed_job, structured_draft_views)
        if structured_draft_views is not None
        else build_cover_letter_content(parsed_job, materials)
    )
    application_package_content = build_application_package_content(
        parsed_job,
        materials,
        final_package,
        structured_cover_letter_content=(
            cover_letter_content if structured_draft_views is not None else None
        ),
    )
    text_entries = {
        "00_preparation_questions.md": _preparation_questions(parsed_job, metadata),
        "01_job_summary.md": _job_summary(parsed_job),
        "02_fit_report.md": materials.fit_report,
        "03_cover_letter_draft.md": materials.cover_letter_draft,
        "04_cv_tailoring_notes.md": materials.cv_tailoring_notes,
        "05_criteria_checklist.md": materials.criteria_checklist,
        "06_final_application_package.md": final_package,
        "07_material_review_checklist.md": material_review,
        "typst/application_package.typ": render_modernpro_application_package_source(
            application_package_content
        ),
        "typst/cover_letter.typ": render_modernpro_cover_letter_source(
            cover_letter_content
        ),
    }
    json_entries = {
        "typst/application_package_content.json": application_package_content,
        "typst/cover_letter_content.json": cover_letter_content,
    }
    if research_statement_views is not None:
        research_content = build_structured_research_statement_content(
            parsed_job,
            research_statement_views,
        )
        text_entries[RESEARCH_STATEMENT_MARKDOWN_OUTPUT_PATH] = (
            research_statement_views.markdown
        )
        text_entries["typst/research_statement.typ"] = (
            render_modernpro_research_statement_source(research_content)
        )
        json_entries[RESEARCH_STATEMENT_CONTENT_OUTPUT_PATH] = research_content

    entries = [
        BundleEntryV1.from_bytes(
            path=path,
            media_type="text/markdown" if path.endswith(".md") else "text/plain",
            data=text.encode("utf-8"),
        )
        for path, text in text_entries.items()
    ]
    entries.extend(
        BundleEntryV1.from_bytes(
            path=path,
            media_type="application/json",
            data=(json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8"),
        )
        for path, value in json_entries.items()
    )
    bundle = ArtifactBundleV1(
        job_id=job_dir.name,
        stage="package",
        mode=mode,
        input_fingerprint=input_fingerprint,
        entries=tuple(sorted(entries, key=lambda item: item.path)),
    )
    return validate_package_bundle_candidate(
        bundle.model_dump(mode="json"),
        job_dir=job_dir,
        input_fingerprint=input_fingerprint,
        mode=mode,
    )


def validate_package_bundle_candidate(
    candidate: object,
    *,
    job_dir: Path,
    input_fingerprint: str,
    mode: Literal["guarded", "legacy_compatibility"] = "guarded",
) -> ArtifactBundleV1:
    try:
        bundle = ArtifactBundleV1.model_validate(candidate)
    except ValidationError as exc:
        raise PackageStageError("Package bundle schema validation failed.") from exc
    paths = {entry.path for entry in bundle.entries}
    research_paths = paths & PACKAGE_RESEARCH_ENTRY_PATHS
    if (
        bundle.job_id != job_dir.name
        or bundle.stage != "package"
        or bundle.mode != mode
        or bundle.input_fingerprint != input_fingerprint
        or package_input_fingerprint(job_dir) != input_fingerprint
        or not PACKAGE_REQUIRED_ENTRY_PATHS.issubset(paths)
        or not paths.issubset(PACKAGE_PROJECTION_PATHS)
        or research_paths not in {frozenset(), PACKAGE_RESEARCH_ENTRY_PATHS}
    ):
        raise PackageStageError("Package bundle identity or required entries are invalid.")
    return bundle
