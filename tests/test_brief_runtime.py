from __future__ import annotations

import json
from pathlib import Path
from types import SimpleNamespace

import yaml

from canisend.decision_models import (
    ApplicationBriefV1,
    ApplicationDecisionV1,
    ConfirmedIdSelectionV1,
    ConfirmedStringListV1,
    ConfirmedTextV1,
    DecisionBasisV1,
    DocumentRequirementsConfirmationV1,
    LanguagePreferenceV1,
)
from canisend.document_execution import (
    document_execution_status_agent_response,
    inspect_document_execution,
)
from canisend.stage_agent import stage_run_agent_response, stage_status_agent_response
from canisend.stage_runtime import (
    inspect_stage_status,
    prepare_stage,
    run_deterministic_stage,
)
from canisend.stage_store import read_json_object, sha256_file
from canisend.stages.brief_stage import (
    APPLICATION_BRIEF_INPUT_PATH,
    REQUIRED_DOCUMENT_PLAN_OUTPUT_PATH,
    brief_precondition_reasons,
    document_requirements_basis_sha256,
)


NOW = "2026-07-12T10:00:00Z"
PRIVATE_BRIEF = "PRIVATE-BRIEF-RUNTIME-SENTINEL-8119"


def _write_workspace(tmp_path: Path) -> tuple[Path, Path]:
    workspace = tmp_path / "workspace"
    job = workspace / "jobs" / "example-role"
    job.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\n",
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
    (job / "job_advert.md").write_text(
        """# Lecturer in Economics

Required documents: CV, Cover letter

Essential criteria:
- PhD in Economics
""",
        encoding="utf-8",
    )
    return workspace, job


def _run_to_match(workspace: Path, job: Path) -> None:
    run_deterministic_stage(workspace, job, stage="evidence")
    run_deterministic_stage(workspace, job, stage="parse")
    run_deterministic_stage(workspace, job, stage="confirm")
    run_deterministic_stage(workspace, job, stage="match")


def _write_apply_decision(job: Path, *, decision: str = "apply") -> Path:
    model = ApplicationDecisionV1(
        job_id=job.name,
        revision=1,
        updated_at=NOW,
        decision=decision,
        confirmation_state="confirmed",
        confirmed_at=NOW,
        basis=DecisionBasisV1(
            criteria_sha256=sha256_file(job / "criteria.json"),
            matches_sha256=sha256_file(job / "criterion_matches.json"),
            status="current",
        ),
    )
    path = job / "application_decision.yaml"
    _write_yaml(path, model)
    return path


def _write_confirmed_brief(job: Path, *, private_marker: str = PRIVATE_BRIEF) -> Path:
    parsed = read_json_object(job / "parsed_job.json")
    advert = (job / "job_advert.md").read_text(encoding="utf-8")
    model = ApplicationBriefV1(
        job_id=job.name,
        revision=1,
        updated_at=NOW,
        decision_sha256=sha256_file(job / "application_decision.yaml"),
        language=LanguagePreferenceV1(value="uk", confirmation_state="confirmed"),
        writing_style=ConfirmedTextV1(value="direct", confirmation_state="confirmed"),
        motivation=ConfirmedTextV1(value=private_marker, confirmation_state="confirmed"),
        emphasis=ConfirmedIdSelectionV1(confirmation_state="confirmed"),
        exclusions=ConfirmedStringListV1(
            items=(private_marker,),
            confirmation_state="confirmed",
        ),
        document_requirements_confirmation=DocumentRequirementsConfirmationV1(
            state="confirmed",
            basis_sha256=document_requirements_basis_sha256(parsed, advert),
            confirmed_at=NOW,
        ),
    )
    path = job / APPLICATION_BRIEF_INPUT_PATH
    _write_yaml(path, model)
    return path


def _write_yaml(path: Path, model: object) -> None:
    path.write_text(
        yaml.safe_dump(model.model_dump(mode="json"), sort_keys=False),
        encoding="utf-8",
    )


def test_brief_runs_through_shared_runtime_and_keeps_control_plane_body_free(
    tmp_path: Path,
) -> None:
    workspace, job = _write_workspace(tmp_path)
    _run_to_match(workspace, job)
    _write_apply_decision(job)
    _write_confirmed_brief(job)

    prepared = prepare_stage(
        workspace,
        job,
        stage="brief",
        execution_mode="deterministic",
    )
    assert [item.path for item in prepared.task_spec.inputs] == [
        "parsed_job.json",
        "job_advert.md",
        "criteria.json",
        "criterion_matches.json",
        "application_decision.yaml",
        "application_brief.yaml",
    ]
    assert PRIVATE_BRIEF not in prepared.task_spec.model_dump_json()

    outcome = run_deterministic_stage(workspace, job, stage="brief")
    plan = read_json_object(job / REQUIRED_DOCUMENT_PLAN_OUTPUT_PATH)
    response = stage_run_agent_response(workspace, outcome)

    assert outcome.stage == "brief"
    assert plan["requirements_state"] == "confirmed"
    assert all(item["action"] == "prepare" for item in plan["tasks"])
    assert PRIVATE_BRIEF not in json.dumps(plan)
    assert PRIVATE_BRIEF not in response.model_dump_json()
    assert response.extensions["canisend.required_document_count"] == 2
    assert response.extensions["canisend.document_plan_blocker_count"] == 0
    assert response.workflow is not None
    assert response.workflow.readiness == "ready_for_next_stage"
    assert [item.id for item in response.next_actions] == ["documents.status"]

    execution = inspect_document_execution(workspace, job)
    assert execution.source_state == "current"
    assert execution.plan is not None
    assert execution.plan.state == "partially_dispatchable"
    execution_response = document_execution_status_agent_response(
        workspace,
        job,
        execution,
    )
    assert execution_response.workflow is not None
    assert execution_response.workflow.readiness == "action_required"
    assert execution_response.extensions["canisend.document_ready_to_prepare_count"] == 1
    assert execution_response.extensions["canisend.document_executor_unavailable_count"] == 1
    assert PRIVATE_BRIEF not in execution_response.model_dump_json()

    cached = run_deterministic_stage(workspace, job, stage="brief")
    assert cached.cache_hit is True
    status = inspect_stage_status(workspace, job, stage="brief")
    assert status.stage.status == "succeeded"
    assert not status.reasons


def test_brief_status_stales_after_direct_user_owned_brief_edit(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    _run_to_match(workspace, job)
    _write_apply_decision(job)
    brief_path = _write_confirmed_brief(job)
    run_deterministic_stage(workspace, job, stage="brief")

    brief_path.write_bytes(brief_path.read_bytes() + b"# reviewed locally\n")
    status = inspect_stage_status(workspace, job, stage="brief")

    assert status.stage.status == "stale"
    assert "input_changed" in status.reasons
    response = stage_status_agent_response(workspace, job, status)
    assert [item.id for item in response.next_actions] == ["stage.run_brief"]


def test_brief_runtime_blocks_hold_even_when_brief_hash_matches(tmp_path: Path) -> None:
    workspace, job = _write_workspace(tmp_path)
    _run_to_match(workspace, job)
    _write_apply_decision(job, decision="hold")
    _write_confirmed_brief(job)

    status = inspect_stage_status(workspace, job, stage="brief")
    response = stage_status_agent_response(workspace, job, status)

    assert status.stage.status == "blocked"
    assert status.reasons == ("input_not_ready:decision_not_apply",)
    assert [item.id for item in response.next_actions] == ["decision.status"]
    assert response.workflow is not None
    assert response.workflow.readiness == "blocked"


def test_brief_precondition_fails_closed_on_pending_user_mutation(
    tmp_path: Path,
    monkeypatch,
) -> None:
    workspace, job = _write_workspace(tmp_path)

    monkeypatch.setattr(
        "canisend.user_mutations.inspect_current_artifact_mutation",
        lambda *_args, **_kwargs: SimpleNamespace(status="promotion_pending"),
    )

    assert brief_precondition_reasons(workspace, job) == (
        "input_not_ready:decision_mutation",
    )
