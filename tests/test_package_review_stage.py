from __future__ import annotations

import json
from pathlib import Path

import pytest

from canisend.draft_models import stable_claim_id
from canisend.stage_agent import stage_run_agent_response
from canisend.stage_runtime import (
    StageRuntimeError,
    inspect_stage_status,
    run_deterministic_stage,
)
from canisend.stage_store import read_json_object
from canisend.user_mutations import (
    SetFindingDispositionPatch,
    apply_user_patch,
    initialize_review_dispositions,
    inspect_review_dispositions,
)
from tests.test_draft_stage import _candidate, _workspace
from tests.test_research_statement_stage import (
    PRIVATE_RESEARCH_CLAIM,
    _promote,
    _research_candidate,
)
from tests.test_review_stage import _complete_sections
from tests.workflow_fixtures import clone_prebuilt_workspace


def _accept_document_findings(
    workspace: Path,
    job: Path,
    *,
    document_id: str,
    review_path: str,
) -> None:
    initialized = initialize_review_dispositions(
        workspace,
        job,
        document_id=document_id,
        consent_confirmed=True,
    )
    snapshot = initialized.snapshot
    review = read_json_object(job / review_path)
    for finding in review["findings"]:
        updated = apply_user_patch(
            workspace,
            job,
            SetFindingDispositionPatch(
                finding_id=finding["finding_id"],
                disposition="accepted",
            ),
            expected_sha256=snapshot.sha256,
            expected_revision=snapshot.revision,
            document_id=document_id,
            consent_confirmed=True,
        )
        snapshot = updated.snapshot
    inspection = inspect_review_dispositions(
        workspace,
        job,
        document_id=document_id,
    )
    assert inspection.readiness is not None
    assert inspection.readiness.state == "reviewed"


def _build_reviewed_dual_documents(
    tmp_path: Path,
    *,
    conflicting_receipts: bool = False,
    include_cv: bool = True,
) -> tuple[Path, Path, dict[str, object], dict[str, object]]:
    workspace, job = _workspace(
        tmp_path,
        include_cv=include_cv,
        include_research_statement=True,
    )
    cover = _complete_sections(_candidate(workspace, job, factual=True))
    research = _research_candidate(workspace, job)
    if conflicting_receipts:
        cover_claim = cover["sections"][1]["claims"][0]
        research_claim = research["sections"][0]["claims"][0]
        evidence = read_json_object(job / "evidence_catalog.json")
        research_claim["text"] = cover_claim["text"]
        research_claim["claim_id"] = stable_claim_id(
            job_id=job.name,
            document_id=str(research["document_id"]),
            kind="factual",
            text=str(research_claim["text"]),
        )
        research_claim["evidence_ref_ids"] = [evidence["items"][1]["evidence_id"]]

    _promote(workspace, job, cover)
    _promote(workspace, job, research)
    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=str(cover["document_id"]),
    )
    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=str(research["document_id"]),
    )
    _accept_document_findings(
        workspace,
        job,
        document_id=str(cover["document_id"]),
        review_path="review_findings.json",
    )
    _accept_document_findings(
        workspace,
        job,
        document_id=str(research["document_id"]),
        review_path="research_statement_review_findings.json",
    )
    return workspace, job, cover, research


def _reviewed_dual_documents(
    tmp_path: Path,
    *,
    conflicting_receipts: bool = False,
    include_cv: bool = True,
) -> tuple[Path, Path, dict[str, object], dict[str, object]]:
    key = (
        "reviewed-dual-documents"
        f"-conflict-{int(conflicting_receipts)}"
        f"-cv-{int(include_cv)}"
    )

    def build(root: Path) -> tuple[Path, Path]:
        workspace, job, _cover, _research = _build_reviewed_dual_documents(
            root,
            conflicting_receipts=conflicting_receipts,
            include_cv=include_cv,
        )
        return workspace, job

    workspace, job = clone_prebuilt_workspace(
        tmp_path,
        key=key,
        builder=build,
    )
    return (
        workspace,
        job,
        read_json_object(job / "cover_letter_draft.json"),
        read_json_object(job / "research_statement_draft.json"),
    )


def test_package_review_runs_with_missing_documents_and_records_blockers(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path, include_research_statement=True)

    outcome = run_deterministic_stage(workspace, job, stage="package_review")
    review = read_json_object(job / "package_review_findings.json")
    response = stage_run_agent_response(workspace, outcome)

    assert outcome.document_id is None
    by_kind = {item["normalized_kind"]: item for item in review["documents"]}
    assert by_kind["cover_letter"]["state"] == "draft_missing"
    assert by_kind["research_statement"]["state"] == "draft_missing"
    assert any(
        item["state"] == "executor_unavailable" for item in review["documents"]
    )
    assert len(review["blocker_finding_ids"]) == len(review["documents"])
    assert response.workflow is not None
    assert response.workflow.readiness == "blocked"
    assert response.artifacts[0].kind == "package_review_findings"
    assert response.extensions["canisend.package_review_blocker_count"] == len(
        review["documents"]
    )


def test_package_review_detects_exact_duplicate_receipt_conflict_without_leaking_body(
    tmp_path: Path,
) -> None:
    workspace, job, _cover, _research = _reviewed_dual_documents(
        tmp_path,
        conflicting_receipts=True,
    )

    outcome = run_deterministic_stage(workspace, job, stage="package_review")
    review = read_json_object(job / "package_review_findings.json")
    response = stage_run_agent_response(workspace, outcome)
    task = read_json_object(outcome.manifest_path.parent / "task-spec.json")

    conflict = next(
        item
        for item in review["findings"]
        if item["code"] == "package.duplicate_claim_evidence_conflict"
    )
    assert conflict["severity"] == "blocker"
    assert len(conflict["document_ids"]) == 2
    assert len(conflict["correction_proposal_ids"]) == 2
    by_kind = {item["normalized_kind"]: item for item in review["documents"]}
    assert by_kind["cover_letter"]["state"] == "reviewed"
    assert by_kind["research_statement"]["state"] == "reviewed"
    assert any(
        item["state"] == "executor_unavailable" for item in review["documents"]
    )
    assert all(
        item["application_route"] == "guarded_draft_candidate"
        for item in review["correction_proposals"]
    )
    control_text = json.dumps(
        {
            "response": response.model_dump(mode="json"),
            "task": task,
            "manifest": outcome.manifest.model_dump(mode="json"),
        },
        ensure_ascii=False,
    )
    assert PRIVATE_RESEARCH_CLAIM not in control_text
    assert "I completed a PhD in Economics." not in control_text


def test_package_review_cache_and_selective_invalidation_preserve_sibling_review(
    tmp_path: Path,
) -> None:
    workspace, job, _cover, research = _reviewed_dual_documents(tmp_path)
    first = run_deterministic_stage(workspace, job, stage="package_review")
    original = (job / "package_review_findings.json").read_bytes()
    cached = run_deterministic_stage(workspace, job, stage="package_review")

    assert first.cache_hit is False
    assert cached.cache_hit is True
    assert (job / "package_review_findings.json").read_bytes() == original

    cover_path = job / "cover_letter_draft.json"
    cover_payload = read_json_object(cover_path)
    cover_path.write_text(json.dumps(cover_payload, indent=4) + "\n", encoding="utf-8")

    aggregate = inspect_stage_status(workspace, job, stage="package_review")
    sibling = inspect_stage_status(
        workspace,
        job,
        stage="review",
        document_id=str(research["document_id"]),
    )
    assert aggregate.stage.status == "stale"
    assert "input_changed" in aggregate.reasons
    assert sibling.stage.status == "succeeded"
    assert sibling.reasons == ()
    assert (job / "package_review_findings.json").read_bytes() == original


def test_package_review_output_drift_is_preserved_and_blocks_replacement(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path, include_research_statement=True)
    run_deterministic_stage(workspace, job, stage="package_review")
    authoritative = job / "package_review_findings.json"
    authoritative.write_text('{"manual":"preserve"}\n', encoding="utf-8")

    status = inspect_stage_status(workspace, job, stage="package_review")

    assert status.output_drift is True
    assert "output_drift" in status.reasons
    with pytest.raises(StageRuntimeError) as captured:
        run_deterministic_stage(workspace, job, stage="package_review")
    assert captured.value.code == "stage.output_conflict"
    assert authoritative.read_text(encoding="utf-8") == '{"manual":"preserve"}\n'


def test_package_review_state_reconstructs_from_immutable_receipts(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path, include_research_statement=True)
    run_deterministic_stage(workspace, job, stage="package_review")
    state_path = job / "workflow" / "state.json"
    state_path.write_text("not json\n", encoding="utf-8")

    status = inspect_stage_status(workspace, job, stage="package_review")

    assert status.reconstructed is True
    assert status.stage.status == "succeeded"
    assert status.output_drift is False
    assert state_path.read_text(encoding="utf-8") == "not json\n"
