from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest
from typer.testing import CliRunner

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
from canisend.typst import render_typst_files
from canisend.typst_mapping import render_modernpro_research_statement_source
from canisend.user_mutations import (
    SetFindingDispositionPatch,
    apply_user_patch,
    initialize_review_dispositions,
)
from tests.test_draft_stage import _workspace
from tests.test_research_statement_stage import _promote, _research_candidate


RESEARCH_PROJECTION_SENTINEL = "PRIVATE-RESEARCH-PROJECTION-SENTINEL-5291"


def _reviewed_research_statement(
    tmp_path: Path,
    *,
    text: str = RESEARCH_PROJECTION_SENTINEL,
) -> tuple[Path, Path, dict[str, object]]:
    workspace, job = _workspace(
        tmp_path,
        include_cover_letter=False,
        include_research_statement=True,
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


def test_pipeline_projects_reviewed_research_statement_without_package_embedding(
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

    _run_pipeline(workspace)

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
    package_manifest = json.loads(
        (job / "typst" / ".canisend-generated.json").read_text(
            encoding="utf-8"
        )
    )
    research_manifest = json.loads(
        (job / "typst" / ".canisend-research-generated.json").read_text(
            encoding="utf-8"
        )
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
    assert set(package_manifest["files"]) == {
        "cover_letter.typ",
        "application_package.typ",
    }
    assert set(research_manifest["files"]) == {"research_statement.typ"}

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


def test_stale_research_projection_is_replaced_without_retaining_claim_body(
    tmp_path: Path,
) -> None:
    workspace, job, _payload = _reviewed_research_statement(tmp_path)
    _run_pipeline(workspace)
    dispositions = job / "research_statement_review_dispositions.yaml"
    dispositions.write_bytes(dispositions.read_bytes() + b" ")

    _run_pipeline(workspace)

    markdown = (job / "08_research_statement.md").read_text(encoding="utf-8")
    content = json.loads(
        (job / "typst" / "research_statement_content.json").read_text(
            encoding="utf-8"
        )
    )
    source = (job / "typst" / "research_statement.typ").read_text(
        encoding="utf-8"
    )
    assert RESEARCH_PROJECTION_SENTINEL not in markdown
    assert RESEARCH_PROJECTION_SENTINEL not in json.dumps(content)
    assert RESEARCH_PROJECTION_SENTINEL not in source
    assert content == {
        "projection": {
            "status": "unavailable",
            "reason": "current_reviewed_projection_unavailable",
            "integration_scope": "standalone_document",
            "document_kind": "research_statement",
        },
        "structured_sections": [],
    }
    assert "research-statement projection unavailable" in source


def test_stale_projection_preserves_edited_typst_and_isolated_candidate(
    tmp_path: Path,
) -> None:
    workspace, job, _payload = _reviewed_research_statement(tmp_path)
    _run_pipeline(workspace)
    primary = job / "typst" / "research_statement.typ"
    edited = primary.read_text(encoding="utf-8") + "\n// user edit\n"
    primary.write_text(edited, encoding="utf-8")
    dispositions = job / "research_statement_review_dispositions.yaml"
    dispositions.write_bytes(dispositions.read_bytes() + b" ")

    _run_pipeline(workspace)

    candidate = job / "typst" / "research_statement.generated.typ"
    assert primary.read_text(encoding="utf-8") == edited
    assert candidate.is_file()
    assert RESEARCH_PROJECTION_SENTINEL not in candidate.read_text(encoding="utf-8")
    package_check = check_application_package(job, workspace / "profile")
    assert not any(issue.path == "typst/research_statement.generated.typ" for issue in package_check.issues)
    assert "job/typst/research_statement.generated.typ" not in package_check.input_hashes
    with pytest.raises(RuntimeError, match="research_statement.generated.typ"):
        render_typst_files(job, typst_bin="missing-typst")
