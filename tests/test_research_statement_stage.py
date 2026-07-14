from __future__ import annotations

import json
from pathlib import Path

import pytest

from canisend.draft_models import stable_claim_id
from canisend.stage_agent import stage_apply_agent_response, stage_run_agent_response
from canisend.stage_runtime import (
    StageRuntimeError,
    apply_stage_result,
    inspect_stage_status,
    prepare_stage,
    run_configured_provider_stage,
    run_deterministic_stage,
    submit_stage_candidate,
)
from canisend.stage_store import read_json_object
from canisend.stages.draft_stage import (
    HOST_AGENT_RESEARCH_STATEMENT_GENERATOR_STRATEGY,
    research_statement_draft_input_fingerprint,
    research_statement_draft_input_projection,
    validate_research_statement_draft_candidate,
)
from canisend.user_mutations import (
    initialize_review_dispositions,
    inspect_review_dispositions,
)
from tests.test_draft_stage import _candidate, _workspace


PRIVATE_RESEARCH_CLAIM = "PRIVATE-RESEARCH-STATEMENT-CLAIM-7349"


def _research_candidate(workspace: Path, job: Path) -> dict[str, object]:
    projection = research_statement_draft_input_projection(workspace, job)
    document_id = str(projection["research_statement_document_id"])
    criteria = read_json_object(job / "criteria.json")
    evidence = read_json_object(job / "evidence_catalog.json")
    criterion_id = criteria["criteria"][0]["criterion_id"]
    evidence_id = evidence["items"][0]["evidence_id"]

    def claim(
        text: str,
        *,
        kind: str = "factual",
        evidence_refs: list[str] | None = None,
    ) -> dict[str, object]:
        factual = kind == "factual"
        return {
            "claim_id": stable_claim_id(
                job_id=job.name,
                document_id=document_id,
                kind=kind,  # type: ignore[arg-type]
                text=text,
            ),
            "text": text,
            "kind": kind,
            "support_strength": "strong" if factual else "not_applicable",
            "criterion_ids": [criterion_id],
            "evidence_ref_ids": (
                evidence_refs if factual and evidence_refs is not None else []
            ),
            "brief_field_refs": [],
            "job_field_refs": [],
            "blockers": [],
            "review_state": "proposed",
        }

    return {
        "schema_version": "1.0.0",
        "job_id": job.name,
        "document_id": document_id,
        "input_fingerprint": research_statement_draft_input_fingerprint(
            workspace,
            job,
        ),
        "basis": {
            "parsed_job_sha256": projection["parsed_job_sha256"],
            "criteria_sha256": projection["criteria_sha256"],
            "evidence_catalog_sha256": projection["evidence_catalog_sha256"],
            "criterion_matches_sha256": projection["criterion_matches_sha256"],
            "application_decision_sha256": projection[
                "application_decision_sha256"
            ],
            "application_brief_sha256": projection["application_brief_sha256"],
            "required_document_plan_sha256": projection[
                "required_document_plan_sha256"
            ],
        },
        "generation_mode": "host_agent",
        "generator_strategy": HOST_AGENT_RESEARCH_STATEMENT_GENERATOR_STRATEGY,
        "generator_version": "1.0.0",
        "review_state": "proposed",
        "sections": [
            {
                "section_id": "research_overview",
                "heading": None,
                "claims": [
                    claim(PRIVATE_RESEARCH_CLAIM, evidence_refs=[evidence_id])
                ],
            },
            {
                "section_id": "research_contributions",
                "heading": None,
                "claims": [
                    claim(
                        "My current work develops applied policy-evaluation methods.",
                        evidence_refs=[evidence_id],
                    )
                ],
            },
            {
                "section_id": "future_agenda",
                "heading": None,
                "claims": [
                    claim(
                        "I will extend this agenda to new policy settings.",
                        kind="future_intent",
                    )
                ],
            },
        ],
        "blockers": [],
    }


def _promote(
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


def test_research_statement_candidate_is_bound_to_current_research_document(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(
        tmp_path,
        include_cover_letter=False,
        include_research_statement=True,
    )
    payload = _research_candidate(workspace, job)

    validated = validate_research_statement_draft_candidate(
        payload,
        workspace=workspace,
        job_dir=job,
        document_id=str(payload["document_id"]),
        input_fingerprint=str(payload["input_fingerprint"]),
        research_statement_schema_path=workspace
        / "schemas"
        / "research-statement-draft.schema.json",
    )

    assert validated.generator_strategy == "host_agent.research_statement"
    assert validated.document_id == payload["document_id"]
    assert len(validated.sections) == 3


def test_cover_and_research_statement_use_independent_guarded_runs(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path, include_research_statement=True)
    cover = _candidate(workspace, job, factual=True)
    research = _research_candidate(workspace, job)

    with pytest.raises(StageRuntimeError) as ambiguous:
        prepare_stage(
            workspace,
            job,
            stage="draft",
            execution_mode="host_agent",
        )
    assert ambiguous.value.code == "stage.document_ambiguous"

    with pytest.raises(StageRuntimeError) as unsupported_provider:
        run_configured_provider_stage(
            workspace,
            job,
            stage="draft",
            document_id=str(research["document_id"]),
            allow_provider_backed=True,
        )
    assert unsupported_provider.value.code == "stage.unsupported_mode"

    _promote(workspace, job, cover)
    cover_bytes = (job / "cover_letter_draft.json").read_bytes()

    prepared = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
        document_id=str(research["document_id"]),
    )
    assert prepared.document_kind == "research_statement"
    assert prepared.task_spec.authoritative_target == "research_statement_draft.json"
    assert prepared.task_spec.output_schema == "canisend.research-statement-draft/v1"
    assert prepared.task_spec.required_consents == ("read-private-draft-inputs",)
    assert PRIVATE_RESEARCH_CLAIM not in prepared.task_spec.model_dump_json()

    submitted = submit_stage_candidate(
        workspace,
        job,
        task_spec_path=prepared.task_spec_path,
        candidate_bytes=(json.dumps(research, ensure_ascii=False) + "\n").encode(),
    )
    applied = apply_stage_result(
        workspace,
        job,
        task_spec_path=prepared.task_spec_path,
        task_result_path=submitted.result_path,
    )
    response = stage_apply_agent_response(workspace, applied)

    assert applied.document_kind == "research_statement"
    assert response.artifacts[0].kind == "research_statement_draft"
    assert response.extensions["canisend.document_id"] == research["document_id"]
    assert PRIVATE_RESEARCH_CLAIM not in response.model_dump_json()
    assert (job / "cover_letter_draft.json").read_bytes() == cover_bytes
    assert (job / "research_statement_draft.json").is_file()

    research_review = run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=str(research["document_id"]),
    )
    review_response = stage_run_agent_response(workspace, research_review)
    findings = read_json_object(job / "research_statement_review_findings.json")

    assert research_review.document_kind == "research_statement"
    assert findings["reviewer_strategy"] == "deterministic.research_statement_review"
    assert findings["blocker_finding_ids"] == []
    assert len(findings["findings"]) == 3
    assert review_response.artifacts[0].kind == "research_statement_review_findings"
    assert [item.id for item in review_response.next_actions] == [
        "review.inspect_findings"
    ]
    assert PRIVATE_RESEARCH_CLAIM not in review_response.model_dump_json()
    assert not (job / "review_findings.json").exists()

    cover_review = run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=str(cover["document_id"]),
    )
    assert cover_review.document_kind == "cover_letter"
    assert (job / "review_findings.json").is_file()
    assert (job / "research_statement_review_findings.json").is_file()

    initialized_dispositions = initialize_review_dispositions(
        workspace,
        job,
        consent_confirmed=True,
    )
    dispositions = inspect_review_dispositions(workspace, job)
    assert initialized_dispositions.snapshot.model.document_id == cover["document_id"]
    assert dispositions.basis_status == "current"
    assert dispositions.readiness is not None

    state = inspect_stage_status(
        workspace,
        job,
        stage="review",
        document_id=str(research["document_id"]),
    ).state
    document_records = [
        item for item in state.stages if item.stage in {"draft", "review"}
    ]
    assert {(item.stage, item.document_id) for item in document_records} == {
        ("draft", cover["document_id"]),
        ("draft", research["document_id"]),
        ("review", cover["document_id"]),
        ("review", research["document_id"]),
    }


def test_research_statement_rejects_cover_letter_contract(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path, include_research_statement=True)
    cover = _candidate(workspace, job)
    research = _research_candidate(workspace, job)
    prepared = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
        document_id=str(research["document_id"]),
    )

    with pytest.raises(StageRuntimeError) as failure:
        submit_stage_candidate(
            workspace,
            job,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(json.dumps(cover) + "\n").encode(),
        )

    assert failure.value.code == "stage.invalid_candidate"
    assert not (job / "research_statement_draft.json").exists()
    assert not prepared.candidate_path.exists()


def test_research_statement_review_blocks_missing_required_sections(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(
        tmp_path,
        include_cover_letter=False,
        include_research_statement=True,
    )
    payload = _research_candidate(workspace, job)
    payload["sections"] = [payload["sections"][0]]
    _promote(workspace, job, payload)

    run_deterministic_stage(
        workspace,
        job,
        stage="review",
        document_id=str(payload["document_id"]),
    )
    findings = read_json_object(job / "research_statement_review_findings.json")
    missing = [
        item for item in findings["findings"] if item["code"] == "document.section_missing"
    ]

    assert len(missing) == 2
    assert all(item["severity"] == "blocker" for item in missing)
    assert all("Research Statement" in item["message"] for item in missing)
