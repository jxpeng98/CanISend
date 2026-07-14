from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path

import pytest

from canisend.draft_models import stable_claim_id
from canisend.stage_runtime import (
    apply_stage_result,
    inspect_stage_status,
    prepare_stage,
    run_deterministic_stage,
    submit_stage_candidate,
)
from canisend.stage_agent import stage_run_agent_response
from canisend.stage_store import read_json_object
from canisend.stages.review_stage import (
    ReviewStageValidationError,
    build_deterministic_review_candidate,
    review_input_fingerprint,
    review_input_projection,
    review_precondition_reasons,
    validate_review_candidate,
)
from tests.test_draft_stage import _candidate, _workspace


def _complete_sections(payload: dict[str, object]) -> dict[str, object]:
    updated = deepcopy(payload)
    job_id = str(updated["job_id"])
    document_id = str(updated["document_id"])
    opening_text = "Dear Selection Committee,"
    closing_text = "Yours sincerely,"
    opening = {
        "claim_id": stable_claim_id(
            job_id=job_id,
            document_id=document_id,
            kind="administrative",
            text=opening_text,
        ),
        "text": opening_text,
        "kind": "administrative",
        "support_strength": "not_applicable",
        "criterion_ids": [],
        "evidence_ref_ids": [],
        "brief_field_refs": [],
        "job_field_refs": [],
        "blockers": [],
        "review_state": "proposed",
    }
    closing = {
        **opening,
        "claim_id": stable_claim_id(
            job_id=job_id,
            document_id=document_id,
            kind="administrative",
            text=closing_text,
        ),
        "text": closing_text,
    }
    body = updated["sections"][0]
    updated["sections"] = [
        {"section_id": "opening", "heading": None, "claims": [opening]},
        body,
        {"section_id": "closing", "heading": None, "claims": [closing]},
    ]
    return updated


def _promote_draft(
    workspace: Path,
    job: Path,
    payload: dict[str, object],
) -> None:
    prepared = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
        document_id=str(payload["document_id"]),
    )
    submitted = submit_stage_candidate(
        workspace,
        job,
        task_spec_path=prepared.task_spec_path,
        candidate_bytes=(json.dumps(payload, ensure_ascii=False) + "\n").encode(),
    )
    apply_stage_result(
        workspace,
        job,
        task_spec_path=prepared.task_spec_path,
        task_result_path=submitted.result_path,
    )


def test_review_projection_is_body_free_and_binds_current_draft(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _complete_sections(_candidate(workspace, job, factual=True))
    private_claim = "PRIVATE-REVIEW-CLAIM-SENTINEL-4831"
    body_claim = payload["sections"][1]["claims"][0]
    body_claim["text"] = private_claim
    body_claim["claim_id"] = stable_claim_id(
        job_id=job.name,
        document_id=str(payload["document_id"]),
        kind="factual",
        text=private_claim,
    )
    _promote_draft(workspace, job, payload)

    assert review_precondition_reasons(workspace, job) == ()
    projection = review_input_projection(workspace, job)

    assert projection["draft_sha256"]
    assert projection["draft_input_fingerprint"] == payload["input_fingerprint"]
    assert private_claim not in json.dumps(projection)


def test_review_requires_semantic_review_for_structurally_supported_fact(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _complete_sections(_candidate(workspace, job, factual=True))
    _promote_draft(workspace, job, payload)
    fingerprint = review_input_fingerprint(workspace, job)

    review = build_deterministic_review_candidate(
        workspace,
        job,
        input_fingerprint=fingerprint,
    )

    assert review.blocker_finding_ids == ()
    assert len(review.findings) == 3
    assert all(item.severity == "review" for item in review.findings)
    support_review = next(
        item
        for item in review.findings
        if item.code == "claim.semantic_support_review"
    )
    assert support_review.claim_ids == (
        payload["sections"][1]["claims"][0]["claim_id"],
    )


def test_review_requires_kind_review_for_non_factual_claims(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _complete_sections(_candidate(workspace, job))
    disguised_fact = "I won a national teaching award."
    body_claim = {
        "claim_id": stable_claim_id(
            job_id=job.name,
            document_id=str(payload["document_id"]),
            kind="administrative",
            text=disguised_fact,
        ),
        "text": disguised_fact,
        "kind": "administrative",
        "support_strength": "not_applicable",
        "criterion_ids": [],
        "evidence_ref_ids": [],
        "brief_field_refs": [],
        "job_field_refs": [],
        "blockers": [],
        "review_state": "proposed",
    }
    payload["sections"][1]["claims"].append(body_claim)
    _promote_draft(workspace, job, payload)

    review = build_deterministic_review_candidate(
        workspace,
        job,
        input_fingerprint=review_input_fingerprint(workspace, job),
    )

    kind_review = next(
        item
        for item in review.findings
        if body_claim["claim_id"] in item.claim_ids
    )
    assert kind_review.code == "claim.semantic_kind_review"
    assert kind_review.severity == "review"


def test_review_turns_unsupported_claim_and_missing_sections_into_blockers(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job, factual=True)
    claim = payload["sections"][0]["claims"][0]
    claim["support_strength"] = "unsupported"
    claim["evidence_ref_ids"] = []
    claim["blockers"] = ["claim.unsupported"]
    payload["blockers"] = ["claim.unsupported"]
    _promote_draft(workspace, job, payload)

    review = build_deterministic_review_candidate(
        workspace,
        job,
        input_fingerprint=review_input_fingerprint(workspace, job),
    )
    codes = [item.code for item in review.findings]

    assert codes.count("document.section_missing") == 2
    assert "claim.unsupported" in codes
    assert len(review.blocker_finding_ids) == 3
    assert all(
        item.severity == "blocker"
        for item in review.findings
        if item.finding_id in review.blocker_finding_ids
    )


def test_review_detects_confirmed_brief_exclusion_conflict_as_blocker(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path, exclusions=("rankings",))
    payload = _complete_sections(_candidate(workspace, job))
    claim = payload["sections"][1]["claims"][0]
    text = "Department rankings are central to my motivation for applying."
    claim["text"] = text
    claim["claim_id"] = stable_claim_id(
        job_id=job.name,
        document_id=str(payload["document_id"]),
        kind="motivation",
        text=text,
    )
    _promote_draft(workspace, job, payload)

    review = build_deterministic_review_candidate(
        workspace,
        job,
        input_fingerprint=review_input_fingerprint(workspace, job),
    )
    conflict = next(
        item for item in review.findings if item.code == "brief.exclusion_conflict"
    )

    assert conflict.severity == "blocker"
    assert conflict.category == "contradiction"
    assert conflict.finding_id in review.blocker_finding_ids
    assert "rankings" not in conflict.message.casefold()


def test_review_candidate_must_equal_deterministic_projection(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _complete_sections(_candidate(workspace, job, factual=True))
    _promote_draft(workspace, job, payload)
    fingerprint = review_input_fingerprint(workspace, job)
    review = build_deterministic_review_candidate(
        workspace,
        job,
        input_fingerprint=fingerprint,
    )
    tampered = review.model_dump(mode="json")
    tampered["findings"][0]["next_action"] = "Trust the generated claim without review."

    with pytest.raises(ReviewStageValidationError, match="deterministic projection"):
        validate_review_candidate(
            tampered,
            workspace=workspace,
            job_dir=job,
            input_fingerprint=fingerprint,
        )


def test_review_runs_through_shared_deterministic_runtime(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _complete_sections(_candidate(workspace, job, factual=True))
    private_claim = "PRIVATE-RUNTIME-REVIEW-SENTINEL-7129"
    body_claim = payload["sections"][1]["claims"][0]
    body_claim["text"] = private_claim
    body_claim["claim_id"] = stable_claim_id(
        job_id=job.name,
        document_id=str(payload["document_id"]),
        kind="factual",
        text=private_claim,
    )
    _promote_draft(workspace, job, payload)

    prepared = prepare_stage(
        workspace,
        job,
        stage="review",
        execution_mode="deterministic",
    )
    assert [item.path for item in prepared.task_spec.inputs] == [
        "parsed_job.json",
        "criteria.json",
        "evidence_catalog.json",
        "criterion_matches.json",
        "application_decision.yaml",
        "application_brief.yaml",
        "required_document_plan.json",
        "cover_letter_draft.json",
    ]
    assert prepared.task_spec.authoritative_target == "review_findings.json"
    assert prepared.task_spec.document_id == payload["document_id"]
    assert prepared.task_spec.output_schema == "canisend.review-findings/v1"
    assert prepared.task_spec.required_consents == ()
    assert private_claim not in prepared.task_spec.model_dump_json()

    outcome = run_deterministic_stage(workspace, job, stage="review")
    response = stage_run_agent_response(workspace, outcome)
    findings = read_json_object(job / "review_findings.json")

    assert outcome.stage == "review"
    assert outcome.document_id == payload["document_id"]
    assert findings["review_state"] == "proposed"
    assert any(
        item["code"] == "claim.semantic_support_review"
        for item in findings["findings"]
    )
    assert response.workflow is not None
    assert response.workflow.readiness == "review_required"
    assert response.extensions["canisend.review_finding_count"] == 3
    assert response.extensions["canisend.review_blocker_count"] == 0
    assert response.extensions["canisend.document_id"] == payload["document_id"]
    assert private_claim not in response.model_dump_json()

    status = inspect_stage_status(workspace, job, stage="review")
    assert status.stage.status == "succeeded"
    assert status.stage.document_id == payload["document_id"]
    assert not status.reasons

    cached = run_deterministic_stage(workspace, job, stage="review")
    assert cached.cache_hit is True
    assert cached.document_id == payload["document_id"]
