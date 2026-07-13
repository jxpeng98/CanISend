from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path

import pytest
import yaml

from canisend.decision_models import (
    ApplicationBriefV1,
    ApplicationDecisionV1,
    ConfirmedIdSelectionV1,
    ConfirmedStringListV1,
    ConfirmedTextV1,
    DecisionBasisV1,
    DocumentChoiceV1,
    DocumentRequirementsConfirmationV1,
    LanguagePreferenceV1,
)
from canisend.draft_models import stable_claim_id
from canisend.stage_agent import (
    stage_apply_agent_response,
    stage_prepare_agent_response,
    stage_submit_agent_response,
)
from canisend.stage_runtime import (
    StageRuntimeError,
    apply_stage_result,
    inspect_stage_status,
    prepare_stage,
    run_deterministic_stage,
    submit_stage_candidate,
)
from canisend.stage_store import read_json_object, sha256_file
from canisend.stages.brief_stage import (
    document_requirements_basis_sha256,
    stable_document_id,
)
from canisend.stages.draft_stage import (
    DRAFT_GENERATOR_STRATEGY,
    DRAFT_GENERATOR_VERSION,
    DraftStageValidationError,
    draft_input_fingerprint,
    draft_input_projection,
    draft_precondition_reasons,
    validate_draft_candidate,
)


NOW = "2026-07-13T10:00:00Z"
PRIVATE_MOTIVATION = "PRIVATE-DRAFT-MOTIVATION-SENTINEL-8924"


def _workspace(
    tmp_path: Path,
    *,
    include_cover_letter: bool = True,
    omit_cover_letter: bool = False,
    exclusions: tuple[str, ...] = (),
) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job = workspace / "jobs" / "example-role"
    profile = workspace / "profile"
    generated = profile / "generated"
    job.mkdir(parents=True)
    generated.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\n",
        encoding="utf-8",
    )
    (profile / "profile.yaml").write_text(
        "generated:\n  cv_evidence: generated/cv.evidence.md\n",
        encoding="utf-8",
    )
    (generated / "cv.evidence.md").write_text(
        "# Evidence: CV\n\n"
        "## Education\n\n"
        "- [education-001] `education`: Completed a PhD in Economics.\n\n"
        "## Teaching\n\n"
        "- [teaching-001] `teaching`: Designed and taught applied econometrics modules.\n",
        encoding="utf-8",
    )
    (job / "job.yaml").write_text(
        yaml.safe_dump(
            {
                "id": job.name,
                "title": "Lecturer in Economics",
                "institution": "Example University",
                "department": "Economics",
                "location": "London",
                "deadline": "2026-08-01",
                "source_url": "https://example.edu/job",
                "status": "advert_imported",
                "english_variant": "uk",
                "writing_style": "direct",
                "created_at": NOW,
                "updated_at": NOW,
                "notes": "",
            },
            sort_keys=False,
        ),
        encoding="utf-8",
    )
    documents = "CV, Cover letter" if include_cover_letter else "CV"
    (job / "job_advert.md").write_text(
        f"""# Lecturer in Economics

Required documents: {documents}

Essential criteria:
- PhD in Economics
- Experience teaching applied econometrics
""",
        encoding="utf-8",
    )

    for stage in ("evidence", "parse", "confirm", "match"):
        run_deterministic_stage(workspace, job, stage=stage)  # type: ignore[arg-type]

    decision = ApplicationDecisionV1(
        job_id=job.name,
        revision=1,
        updated_at=NOW,
        decision="apply",
        confirmation_state="confirmed",
        confirmed_at=NOW,
        basis=DecisionBasisV1(
            criteria_sha256=sha256_file(job / "criteria.json"),
            matches_sha256=sha256_file(job / "criterion_matches.json"),
            status="current",
        ),
    )
    _write_yaml(job / "application_decision.yaml", decision)

    parsed = read_json_object(job / "parsed_job.json")
    advert = (job / "job_advert.md").read_text(encoding="utf-8")
    document_choices: tuple[DocumentChoiceV1, ...] = ()
    if include_cover_letter and omit_cover_letter:
        document_choices = (
            DocumentChoiceV1(
                document_id=stable_document_id(
                    job_id=job.name,
                    normalized_kind="cover_letter",
                ),
                action="omit",
                confirmation_state="confirmed",
            ),
        )
    brief = ApplicationBriefV1(
        job_id=job.name,
        revision=1,
        updated_at=NOW,
        decision_sha256=sha256_file(job / "application_decision.yaml"),
        language=LanguagePreferenceV1(value="uk", confirmation_state="confirmed"),
        writing_style=ConfirmedTextV1(
            value="direct and evidence-led",
            confirmation_state="confirmed",
        ),
        motivation=ConfirmedTextV1(
            value=PRIVATE_MOTIVATION,
            confirmation_state="confirmed",
        ),
        emphasis=ConfirmedIdSelectionV1(confirmation_state="confirmed"),
        exclusions=ConfirmedStringListV1(
            items=exclusions,
            confirmation_state="confirmed",
        ),
        document_requirements_confirmation=DocumentRequirementsConfirmationV1(
            state="confirmed",
            basis_sha256=document_requirements_basis_sha256(parsed, advert),
            confirmed_at=NOW,
        ),
        document_choices=document_choices,
    )
    _write_yaml(job / "application_brief.yaml", brief)
    run_deterministic_stage(workspace, job, stage="brief")
    return workspace, job


def _write_yaml(path: Path, model: object) -> None:
    path.write_text(
        yaml.safe_dump(model.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )


def _candidate(workspace: Path, job: Path, *, factual: bool = False) -> dict[str, object]:
    fingerprint = draft_input_fingerprint(workspace, job)
    projection = draft_input_projection(workspace, job)
    document_id = str(projection["cover_letter_document_id"])
    if factual:
        criteria = read_json_object(job / "criteria.json")
        evidence = read_json_object(job / "evidence_catalog.json")
        criterion_id = criteria["criteria"][0]["criterion_id"]
        evidence_id = evidence["items"][0]["evidence_id"]
        text = "I completed a PhD in Economics."
        claim = {
            "claim_id": stable_claim_id(
                job_id=job.name,
                document_id=document_id,
                kind="factual",
                text=text,
            ),
            "text": text,
            "kind": "factual",
            "support_strength": "strong",
            "criterion_ids": [criterion_id],
            "evidence_ref_ids": [evidence_id],
            "brief_field_refs": [],
            "job_field_refs": [],
            "blockers": [],
            "review_state": "proposed",
        }
    else:
        text = "The opportunity to contribute to the department motivates my application."
        claim = {
            "claim_id": stable_claim_id(
                job_id=job.name,
                document_id=document_id,
                kind="motivation",
                text=text,
            ),
            "text": text,
            "kind": "motivation",
            "support_strength": "not_applicable",
            "criterion_ids": [],
            "evidence_ref_ids": [],
            "brief_field_refs": ["motivation"],
            "job_field_refs": [],
            "blockers": [],
            "review_state": "proposed",
        }
    return {
        "schema_version": "1.0.0",
        "job_id": job.name,
        "document_id": document_id,
        "input_fingerprint": fingerprint,
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
        "generator_strategy": DRAFT_GENERATOR_STRATEGY,
        "generator_version": DRAFT_GENERATOR_VERSION,
        "review_state": "proposed",
        "sections": [{"section_id": "body", "heading": None, "claims": [claim]}],
        "blockers": [],
    }


def test_draft_projection_binds_all_seven_current_inputs(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)

    assert draft_precondition_reasons(workspace, job) == ()
    projection = draft_input_projection(workspace, job)

    assert projection["parsed_job_sha256"] == sha256_file(job / "parsed_job.json")
    assert projection["criteria_sha256"] == sha256_file(job / "criteria.json")
    assert projection["evidence_catalog_sha256"] == sha256_file(
        job / "evidence_catalog.json"
    )
    assert projection["criterion_matches_sha256"] == sha256_file(
        job / "criterion_matches.json"
    )
    assert projection["application_decision_sha256"] == sha256_file(
        job / "application_decision.yaml"
    )
    assert projection["application_brief_sha256"] == sha256_file(
        job / "application_brief.yaml"
    )
    assert projection["required_document_plan_sha256"] == sha256_file(
        job / "required_document_plan.json"
    )
    assert PRIVATE_MOTIVATION not in json.dumps(projection)


@pytest.mark.parametrize("factual", [False, True], ids=["motivation", "factual"])
def test_validator_accepts_structured_current_claims(tmp_path: Path, factual: bool) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job, factual=factual)

    validated = validate_draft_candidate(
        payload,
        workspace=workspace,
        job_dir=job,
        input_fingerprint=str(payload["input_fingerprint"]),
    )

    assert validated.job_id == job.name
    assert validated.review_state == "proposed"
    assert validated.sections[0].claims[0].kind == (
        "factual" if factual else "motivation"
    )


def test_validator_rejects_stale_or_tampered_basis(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job)
    tampered = deepcopy(payload)
    tampered["basis"]["evidence_catalog_sha256"] = "0" * 64

    with pytest.raises(DraftStageValidationError, match="basis"):
        validate_draft_candidate(
            tampered,
            workspace=workspace,
            job_dir=job,
            input_fingerprint=str(payload["input_fingerprint"]),
        )

    with pytest.raises(DraftStageValidationError, match="stale"):
        validate_draft_candidate(
            payload,
            workspace=workspace,
            job_dir=job,
            input_fingerprint="0" * 64,
        )


@pytest.mark.parametrize("reference", ["criterion", "evidence"])
def test_validator_rejects_fabricated_semantic_references(
    tmp_path: Path,
    reference: str,
) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job, factual=True)
    claim = payload["sections"][0]["claims"][0]
    if reference == "criterion":
        claim["criterion_ids"] = ["criterion_" + "0" * 32]
    else:
        claim["evidence_ref_ids"] = ["evidence_" + "0" * 32]

    with pytest.raises(DraftStageValidationError, match="current"):
        validate_draft_candidate(
            payload,
            workspace=workspace,
            job_dir=job,
            input_fingerprint=str(payload["input_fingerprint"]),
        )


def test_validator_rejects_unknown_role_context_field(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job)
    text = "The deadline is clearly stated."
    claim = payload["sections"][0]["claims"][0]
    claim.update(
        {
            "claim_id": stable_claim_id(
                job_id=job.name,
                document_id=str(payload["document_id"]),
                kind="role_context",
                text=text,
            ),
            "text": text,
            "kind": "role_context",
            "support_strength": "not_applicable",
            "brief_field_refs": [],
            "criterion_ids": [],
            "job_field_refs": ["deadline"],
        }
    )
    parsed = read_json_object(job / "parsed_job.json")
    parsed["deadline"] = ""
    parsed["unknown_fields"] = sorted({*parsed["unknown_fields"], "deadline"})
    (job / "parsed_job.json").write_text(
        json.dumps(parsed, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )

    with pytest.raises(DraftStageValidationError):
        validate_draft_candidate(
            payload,
            workspace=workspace,
            job_dir=job,
            input_fingerprint=str(payload["input_fingerprint"]),
        )


def test_validator_rejects_provider_mode_before_provider_boundary_exists(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job)
    payload["generation_mode"] = "configured_provider"

    with pytest.raises(DraftStageValidationError, match="unsupported generator"):
        validate_draft_candidate(
            payload,
            workspace=workspace,
            job_dir=job,
            input_fingerprint=str(payload["input_fingerprint"]),
        )


def test_precondition_blocks_omitted_or_unplanned_cover_letter(tmp_path: Path) -> None:
    omitted_workspace, omitted_job = _workspace(
        tmp_path / "omitted",
        omit_cover_letter=True,
    )
    missing_workspace, missing_job = _workspace(
        tmp_path / "missing",
        include_cover_letter=False,
    )

    assert draft_precondition_reasons(omitted_workspace, omitted_job) == (
        "input_not_ready:document_plan_blocked",
    )
    assert draft_precondition_reasons(missing_workspace, missing_job) == (
        "input_not_ready:cover_letter_not_planned",
    )


def test_draft_runs_through_guarded_host_agent_promotion(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job, factual=True)

    prepared = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
    )
    prepare_response = stage_prepare_agent_response(workspace, job, prepared)

    assert [item.path for item in prepared.task_spec.inputs] == [
        "parsed_job.json",
        "criteria.json",
        "evidence_catalog.json",
        "criterion_matches.json",
        "application_decision.yaml",
        "application_brief.yaml",
        "required_document_plan.json",
    ]
    assert prepared.task_spec.authoritative_target == "cover_letter_draft.json"
    assert prepared.task_spec.output_schema == "canisend.cover-letter-draft/v1"
    assert prepared.task_spec.execution_mode == "host_agent"
    assert prepared.task_spec.required_consents == ("read-private-draft-inputs",)
    assert prepared.task_spec.write_authority == "core_service"
    assert PRIVATE_MOTIVATION not in prepared.task_spec.model_dump_json()
    assert [item.id for item in prepare_response.required_consents] == [
        "read-private-draft-inputs"
    ]
    assert prepare_response.workflow is not None
    assert prepare_response.workflow.readiness == "action_required"

    submitted = submit_stage_candidate(
        workspace,
        job,
        task_spec_path=prepared.task_spec_path,
        candidate_bytes=(json.dumps(payload, ensure_ascii=False) + "\n").encode(),
    )
    submit_response = stage_submit_agent_response(workspace, submitted)
    assert not (job / "cover_letter_draft.json").exists()
    assert submit_response.workflow is not None
    assert submit_response.workflow.readiness == "review_required"

    applied = apply_stage_result(
        workspace,
        job,
        task_spec_path=prepared.task_spec_path,
        task_result_path=submitted.result_path,
    )
    apply_response = stage_apply_agent_response(workspace, applied)
    promoted = read_json_object(job / "cover_letter_draft.json")

    assert promoted["review_state"] == "proposed"
    assert promoted["sections"][0]["claims"][0]["kind"] == "factual"
    assert apply_response.workflow is not None
    assert apply_response.workflow.readiness == "review_required"
    assert apply_response.extensions["canisend.draft_claim_count"] == 1
    assert apply_response.extensions["canisend.draft_factual_claim_count"] == 1
    assert apply_response.extensions["canisend.draft_blocker_count"] == 0
    assert [item.id for item in apply_response.next_actions] == [
        "stage.run_review"
    ]

    status = inspect_stage_status(workspace, job, stage="draft")
    assert status.stage.status == "succeeded"
    assert not status.reasons


def test_invalid_draft_submission_cannot_modify_authoritative_or_owned_inputs(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job)
    payload["ready"] = True
    legacy = job / "03_cover_letter_draft.md"
    legacy.write_text("USER-EDITED-LEGACY-DRAFT\n", encoding="utf-8")
    protected = {
        path: path.read_bytes()
        for path in (
            job / "application_decision.yaml",
            job / "application_brief.yaml",
            job / "required_document_plan.json",
            job / "03_cover_letter_draft.md",
            workspace / "profile" / "profile.yaml",
        )
    }
    prepared = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
    )

    with pytest.raises(StageRuntimeError) as failure:
        submit_stage_candidate(
            workspace,
            job,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(json.dumps(payload) + "\n").encode(),
        )

    assert failure.value.code == "stage.invalid_candidate"
    assert not (job / "cover_letter_draft.json").exists()
    assert not prepared.candidate_path.exists()
    assert all(path.read_bytes() == before for path, before in protected.items())


def test_prepared_draft_becomes_stale_before_submission_without_promotion(
    tmp_path: Path,
) -> None:
    workspace, job = _workspace(tmp_path)
    payload = _candidate(workspace, job)
    prepared = prepare_stage(
        workspace,
        job,
        stage="draft",
        execution_mode="host_agent",
    )
    brief = job / "application_brief.yaml"
    brief.write_bytes(brief.read_bytes() + b"# edited after task preparation\n")

    with pytest.raises(StageRuntimeError) as failure:
        submit_stage_candidate(
            workspace,
            job,
            task_spec_path=prepared.task_spec_path,
            candidate_bytes=(json.dumps(payload) + "\n").encode(),
        )

    assert failure.value.code in {"stage.dependency_not_current", "stage.stale_input"}
    assert not (job / "cover_letter_draft.json").exists()
    assert not prepared.candidate_path.exists()


def test_draft_refuses_deterministic_execution_mode(tmp_path: Path) -> None:
    workspace, job = _workspace(tmp_path)

    with pytest.raises(StageRuntimeError) as failure:
        prepare_stage(
            workspace,
            job,
            stage="draft",
            execution_mode="deterministic",
        )

    assert failure.value.code == "stage.unsupported_mode"
