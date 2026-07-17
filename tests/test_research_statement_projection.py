from __future__ import annotations

import hashlib
import json
from pathlib import Path

from typer.testing import CliRunner

from canisend.bundle_models import ProjectionJournalV1
from canisend.bundle_projection import load_artifact_bundle, project_artifact_bundle
from canisend.cli import app
from canisend.draft_models import stable_claim_id
from canisend.draft_views import (
    RESEARCH_STATEMENT_PROJECTION_SOURCE,
    STRUCTURED_RESEARCH_STATEMENT_TYPST_MARKER,
    build_structured_research_statement_content,
    load_current_reviewed_research_statement_views,
)
from canisend.ready_check import check_application_package
from canisend.stage_runtime import run_deterministic_stage
from canisend.stage_store import read_json_object
from canisend.typst_mapping import render_modernpro_research_statement_source
from canisend.user_mutations import (
    SetFindingDispositionPatch,
    SetPackageFindingDispositionPatch,
    apply_user_patch,
    initialize_package_review_dispositions,
    initialize_review_dispositions,
)
from canisend.workflow_sequence import SequenceOptions, run_sequence
from tests.test_draft_stage import _candidate, _workspace
from tests.test_research_statement_stage import _promote, _research_candidate
from tests.test_review_stage import _complete_sections, _promote_draft
from tests.workflow_fixtures import clone_prebuilt_workspace


RESEARCH_PROJECTION_SENTINEL = "PRIVATE-RESEARCH-PROJECTION-SENTINEL-5291"


def _build_reviewed_research_statement(
    tmp_path: Path,
    *,
    text: str = RESEARCH_PROJECTION_SENTINEL,
) -> tuple[Path, Path, dict[str, object]]:
    workspace, job = _workspace(
        tmp_path,
        include_cv=False,
        include_cover_letter=True,
        include_research_statement=True,
    )
    cover_payload = _complete_sections(_candidate(workspace, job, factual=True))
    _promote_draft(workspace, job, cover_payload)
    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=str(cover_payload["document_id"]),
    )
    cover_outcome = initialize_review_dispositions(
        workspace,
        job,
        document_id=str(cover_payload["document_id"]),
        consent_confirmed=True,
    )
    cover_review = read_json_object(job / "review_findings.json")
    for finding in cover_review["findings"]:
        cover_outcome = apply_user_patch(
            workspace,
            job,
            SetFindingDispositionPatch(
                finding_id=finding["finding_id"],
                disposition="accepted",
            ),
            document_id=str(cover_payload["document_id"]),
            expected_sha256=cover_outcome.snapshot.sha256,
            expected_revision=cover_outcome.snapshot.revision,
            consent_confirmed=True,
        )

    payload = _research_candidate(workspace, job)
    first_claim = payload["sections"][0]["claims"][0]  # type: ignore[index]
    first_claim["text"] = text
    first_claim["claim_id"] = stable_claim_id(
        job_id=job.name,
        document_id=str(payload["document_id"]),
        kind="factual",
        text=text,
    )
    _promote(workspace, job, payload)
    document_id = str(payload["document_id"])
    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=document_id,
    )
    outcome = initialize_review_dispositions(
        workspace,
        job,
        document_id=document_id,
        consent_confirmed=True,
    )
    review = read_json_object(job / "research_statement_review_findings.json")
    for finding in review["findings"]:
        outcome = apply_user_patch(
            workspace,
            job,
            SetFindingDispositionPatch(
                finding_id=finding["finding_id"],
                disposition="accepted",
            ),
            document_id=document_id,
            expected_sha256=outcome.snapshot.sha256,
            expected_revision=outcome.snapshot.revision,
            consent_confirmed=True,
        )
    return workspace, job, payload


def _reviewed_research_statement(
    tmp_path: Path,
    *,
    text: str = RESEARCH_PROJECTION_SENTINEL,
) -> tuple[Path, Path, dict[str, object]]:
    text_key = hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]

    def build(root: Path) -> tuple[Path, Path]:
        workspace, job, _payload = _build_reviewed_research_statement(
            root,
            text=text,
        )
        return workspace, job

    workspace, job = clone_prebuilt_workspace(
        tmp_path,
        key=f"reviewed-research-statement-{text_key}",
        builder=build,
    )
    return (
        workspace,
        job,
        read_json_object(job / "research_statement_draft.json"),
    )


def _run_pipeline(workspace: Path) -> None:
    result = CliRunner().invoke(
        app,
        [
            "run",
            "--workspace",
            str(workspace),
            "--job",
            "jobs/example-role",
            "--no-git-add-materials",
        ],
    )
    assert result.exit_code == 0, result.output


def _project_guarded_package(workspace: Path, job: Path) -> ProjectionJournalV1:
    run_deterministic_stage(workspace, job, stage="package_review")
    package_review = read_json_object(job / "package_review_findings.json")
    assert package_review["blocker_finding_ids"] == []
    outcome = initialize_package_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    for finding in package_review["findings"]:
        outcome = apply_user_patch(
            workspace,
            job,
            SetPackageFindingDispositionPatch(
                finding_id=finding["finding_id"],
                disposition="accepted",
            ),
            expected_sha256=outcome.snapshot.sha256,
            expected_revision=outcome.snapshot.revision,
            consent_confirmed=True,
        )

    run_deterministic_stage(workspace, job, stage="package")
    bundle = load_artifact_bundle(job / "package_bundle.json")
    assert bundle.mode == "guarded"
    return project_artifact_bundle(job, bundle)


def test_reviewed_research_statement_view_is_traceable_and_structure_safe(
    tmp_path: Path,
) -> None:
    hostile = (
        "# Claimed heading\n"
        '#evil("x")\n'
        "[external](https://example.edu) <script>alert(1)</script>"
    )
    workspace, job, payload = _reviewed_research_statement(
        tmp_path,
        text=hostile,
    )

    views = load_current_reviewed_research_statement_views(
        workspace,
        job,
        parsed_job=read_json_object(job / "parsed_job.json"),
    )

    assert views is not None
    assert views.document_readiness.state == "reviewed"
    assert views.markdown.startswith("# Research Statement\n\n")
    assert "\\# Claimed heading" in views.markdown
    assert "\\[external\\](https://example.edu)" in views.markdown
    assert "&lt;script&gt;" in views.markdown
    assert "<script>" not in views.markdown

    content = build_structured_research_statement_content(
        read_json_object(job / "parsed_job.json"),
        views,
    )
    projection = content["projection"]
    assert projection["status"] == "current"
    assert projection["integration_scope"] == "standalone_document"
    assert projection["source"] == RESEARCH_STATEMENT_PROJECTION_SOURCE
    assert projection["document_id"] == payload["document_id"]
    assert projection["document_readiness_state"] == "reviewed"
    assert projection["requires_human_review"] is False
    assert projection["markdown_sha256"] == hashlib.sha256(
        views.markdown.encode("utf-8")
    ).hexdigest()
    assert projection["review_dispositions_sha256"] == projection[
        "document_readiness"
    ]["review_dispositions_sha256"]
    source = render_modernpro_research_statement_source(content)
    assert '#text("# Claimed heading\\n#evil(\\"x\\")' in source
    assert "\n#evil(" not in source
    assert "\n# Claimed heading" not in source


def test_package_projects_reviewed_research_statement_without_package_embedding(
    tmp_path: Path,
) -> None:
    workspace, job, payload = _reviewed_research_statement(tmp_path)
    authoritative_names = (
        "parsed_job.json",
        "criteria.json",
        "evidence_catalog.json",
        "criterion_matches.json",
        "application_decision.yaml",
        "application_brief.yaml",
        "required_document_plan.json",
        "research_statement_draft.json",
        "research_statement_review_findings.json",
        "research_statement_review_dispositions.yaml",
    )
    before = {name: (job / name).read_bytes() for name in authoritative_names}

    journal = _project_guarded_package(workspace, job)

    assert {name: (job / name).read_bytes() for name in authoritative_names} == before
    markdown = (job / "08_research_statement.md").read_text(encoding="utf-8")
    content = json.loads(
        (job / "typst" / "research_statement_content.json").read_text(
            encoding="utf-8"
        )
    )
    source = (job / "typst" / "research_statement.typ").read_text(
        encoding="utf-8"
    )
    package_content = json.loads(
        (job / "typst" / "application_package_content.json").read_text(
            encoding="utf-8"
        )
    )
    package_source = (job / "typst" / "application_package.typ").read_text(
        encoding="utf-8"
    )
    claim_id = payload["sections"][0]["claims"][0]["claim_id"]  # type: ignore[index]

    assert markdown.count(RESEARCH_PROJECTION_SENTINEL) == 1
    assert content["projection"]["document_readiness_state"] == "reviewed"
    assert STRUCTURED_RESEARCH_STATEMENT_TYPST_MARKER in source
    assert source.count(RESEARCH_PROJECTION_SENTINEL) == 1
    assert f"// CANISEND: claim {claim_id}" in source
    assert "research_statement_projection" not in package_content
    assert "structured_research_statement_sections" not in package_content
    assert RESEARCH_PROJECTION_SENTINEL not in package_source
    projected_targets = {entry.target_path for entry in journal.entries}
    assert {
        "typst/cover_letter.typ",
        "typst/application_package.typ",
        "typst/research_statement.typ",
    } <= projected_targets

    package_check = check_application_package(job, workspace / "profile")
    assert not any(
        issue.path.startswith("08_research_statement")
        or issue.path.startswith("typst/research_statement")
        for issue in package_check.issues
    )
    assert not any(
        key.startswith("job/") and "research_statement" in key
        for key in package_check.input_hashes
    )


def test_pipeline_does_not_project_research_statement_before_reviewed(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(
        tmp_path,
        include_cover_letter=False,
        include_research_statement=True,
    )
    payload = _research_candidate(workspace, job)
    _promote(workspace, job, payload)
    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=str(payload["document_id"]),
    )

    _run_pipeline(workspace)

    assert not (job / "08_research_statement.md").exists()
    assert not (job / "typst" / "research_statement_content.json").exists()
    assert not (job / "typst" / "research_statement.typ").exists()


def test_sequence_fails_closed_and_preserves_research_projection_on_review_drift(
    tmp_path: Path,
) -> None:
    workspace, job, _payload = _reviewed_research_statement(tmp_path)
    _project_guarded_package(workspace, job)
    protected = {
        path: path.read_bytes()
        for path in (
            job / "08_research_statement.md",
            job / "typst" / "research_statement_content.json",
            job / "typst" / "research_statement.typ",
            job / "package_bundle.json",
            job / "workflow" / "projections" / "package.json",
        )
    }
    dispositions = job / "research_statement_review_dispositions.yaml"
    dispositions.write_bytes(dispositions.read_bytes() + b" ")

    sequence = run_sequence(
        workspace,
        job,
        options=SequenceOptions(legacy_compatibility=True),
    )

    assert sequence.legacy_compatibility is None
    assert sequence.plan.first_stop is not None
    assert sequence.plan.first_stop.decision in {"blocked", "repair"}
    assert not any(item.decision == "execute" for item in sequence.plan.items)
    assert {path: path.read_bytes() for path in protected} == protected


def test_review_drift_preserves_edited_typst_without_silent_candidate(
    tmp_path: Path,
) -> None:
    workspace, job, _payload = _reviewed_research_statement(tmp_path)
    _project_guarded_package(workspace, job)
    primary = job / "typst" / "research_statement.typ"
    edited = primary.read_text(encoding="utf-8") + "\n// user edit\n"
    primary.write_text(edited, encoding="utf-8")
    dispositions = job / "research_statement_review_dispositions.yaml"
    dispositions.write_bytes(dispositions.read_bytes() + b" ")

    sequence = run_sequence(
        workspace,
        job,
        options=SequenceOptions(legacy_compatibility=True),
    )

    candidate = job / "typst" / "research_statement.generated.typ"
    assert sequence.legacy_compatibility is None
    assert sequence.plan.first_stop is not None
    assert sequence.plan.first_stop.decision in {"blocked", "repair"}
    assert not any(item.decision == "execute" for item in sequence.plan.items)
    assert primary.read_text(encoding="utf-8") == edited
    assert not candidate.exists()
    package_check = check_application_package(job, workspace / "profile")
    assert not any(issue.path == "typst/research_statement.generated.typ" for issue in package_check.issues)
    assert "job/typst/research_statement.generated.typ" not in package_check.input_hashes
