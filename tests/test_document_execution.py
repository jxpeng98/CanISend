from __future__ import annotations

from copy import deepcopy
from hashlib import sha256
import json
from pathlib import Path
from types import SimpleNamespace

from jsonschema import Draft202012Validator
from pydantic import ValidationError
import pytest
from typer.testing import CliRunner

from canisend.agent_protocol import SUPPORTED_AGENT_OPERATIONS
from canisend.cli import app
from canisend.decision_models import (
    DocumentRequirementV1,
    DocumentTaskV1,
    RequiredDocumentPlanV1,
    SourceSpanV1,
)
from canisend.document_execution import (
    DocumentExecutionInspection,
    DocumentExecutionPlanV1,
    derive_document_execution_plan,
    document_execution_status_agent_response,
    document_executor_capabilities,
)


SHA_A = "a" * 64
SHA_B = "b" * 64
PRIVATE_LABEL = "PRIVATE-EXECUTION-LABEL-7721"
PRIVATE_SOURCE = "PRIVATE-EXECUTION-SOURCE-9913"


def _document_id(index: int) -> str:
    return f"document_{index:032x}"


def _source_plan(
    entries: tuple[tuple[str, str, str, tuple[str, ...]], ...],
    *,
    source_blockers: tuple[str, ...] = (),
) -> RequiredDocumentPlanV1:
    requirements = []
    tasks = []
    for index, (kind, requirement, action, task_blockers) in enumerate(entries, start=1):
        document_id = _document_id(index)
        requirements.append(
            DocumentRequirementV1(
                document_id=document_id,
                label=f"{PRIVATE_LABEL}-{index}",
                normalized_kind=kind,
                requirement=requirement,
                source_text=f"{PRIVATE_SOURCE}-{index}",
                source_state="known",
                source_span=SourceSpanV1(
                    path="job_advert.md",
                    start_line=index,
                    end_line=index,
                    text_sha256=SHA_A,
                    anchor_sha256=SHA_B,
                    occurrence=1,
                    occurrence_count=1,
                ),
                confirmation_state="confirmed",
            )
        )
        tasks.append(
            DocumentTaskV1(
                document_id=document_id,
                action=action,
                confirmation_state=(
                    "unconfirmed" if action == "needs_confirmation" else "confirmed"
                ),
                blockers=task_blockers,
            )
        )

    unresolved = tuple(
        task.document_id for task in tasks if task.action == "needs_confirmation"
    )
    blocking = tuple(task.document_id for task in tasks if task.blockers)
    blockers = tuple(
        sorted(
            set(source_blockers)
            | {reason for task in tasks for reason in task.blockers}
        )
    )
    return RequiredDocumentPlanV1(
        job_id="example-role",
        input_fingerprint=SHA_A,
        requirements_state="confirmed",
        requirements_basis_sha256=SHA_B,
        requirements=tuple(requirements),
        tasks=tuple(tasks),
        unresolved_document_ids=unresolved,
        blocking_document_ids=blocking,
        blockers=blockers,
    )


def _mixed_source_plan() -> RequiredDocumentPlanV1:
    return _source_plan(
        (
            ("cover_letter", "required", "prepare", ()),
            ("research_statement", "required", "prepare", ()),
            ("teaching_statement", "optional", "omit", ()),
            ("portfolio_sample", "optional", "prepare", ()),
        )
    )


def test_capability_registry_exposes_two_guarded_document_executors() -> None:
    capabilities = document_executor_capabilities()
    by_kind = {item.normalized_kind: item for item in capabilities}

    assert tuple(by_kind) == tuple(sorted(by_kind))
    assert {item.normalized_kind for item in capabilities if item.availability == "available"} == {
        "cover_letter",
        "research_statement",
    }
    assert by_kind["cover_letter"].authoritative_target == "cover_letter_draft.json"
    assert by_kind["cover_letter"].execution_modes == (
        "configured_provider",
        "host_agent",
    )
    assert by_kind["research_statement"].authoritative_target == (
        "research_statement_draft.json"
    )
    assert by_kind["research_statement"].output_schema == (
        "canisend.research-statement-draft/v1"
    )
    assert by_kind["research_statement"].execution_modes == ("host_agent",)
    for kind in (
        "teaching_statement",
        "supporting_statement",
        "diversity_statement",
        "publication_list",
    ):
        assert by_kind[kind].availability == "planned"
        assert by_kind[kind].scope == "submission_document"
    assert by_kind["application_email"].scope == "workflow_support"
    assert by_kind["interview_preparation"].scope == "workflow_support"


def test_required_documents_derive_one_honest_fan_out_item_each() -> None:
    plan = derive_document_execution_plan(
        _mixed_source_plan(),
        source_plan_sha256="c" * 64,
    )

    assert plan.state == "partially_dispatchable"
    assert len(plan.items) == 4
    assert plan.ready_document_ids == (_document_id(1), _document_id(2))
    assert plan.executor_unavailable_document_ids == (_document_id(4),)
    assert plan.omitted_document_ids == (_document_id(3),)
    assert plan.blocked_document_ids == ()
    assert plan.blocking_document_ids == (_document_id(4),)
    assert plan.blockers == ("documents.executor_unregistered",)
    assert [item.state for item in plan.items] == [
        "ready_to_prepare",
        "ready_to_prepare",
        "omitted",
        "executor_unavailable",
    ]
    assert plan.items[0].executor_id == "draft.cover_letter"
    assert plan.items[1].route_id == "documents.research_statement"
    assert plan.items[1].executor_id == "draft.research_statement"
    assert plan.items[3].route_id is None
    serialized = plan.model_dump_json()
    assert PRIVATE_LABEL not in serialized
    assert PRIVATE_SOURCE not in serialized


def test_source_plan_blocker_prevents_prepare_but_preserves_confirmed_omit() -> None:
    source = _source_plan(
        (
            ("cover_letter", "required", "prepare", ()),
            ("teaching_statement", "optional", "omit", ()),
        ),
        source_blockers=("brief.motivation_unconfirmed",),
    )

    plan = derive_document_execution_plan(source, source_plan_sha256="c" * 64)

    assert plan.state == "blocked"
    assert plan.source_blockers == ("brief.motivation_unconfirmed",)
    assert plan.blocked_document_ids == (_document_id(1),)
    assert plan.omitted_document_ids == (_document_id(2),)
    assert plan.items[0].reason_codes == ("documents.plan_blocked",)


def test_multiple_cover_letter_requirements_fail_closed_on_executor_cardinality() -> None:
    source = _source_plan(
        (
            ("cover_letter", "required", "prepare", ()),
            ("cover_letter", "optional", "omit", ()),
        )
    )

    plan = derive_document_execution_plan(source, source_plan_sha256="c" * 64)

    assert plan.state == "blocked"
    assert plan.blocked_document_ids == (_document_id(1),)
    assert plan.items[0].reason_codes == (
        "documents.executor_cardinality_unsupported",
    )
    assert plan.omitted_document_ids == (_document_id(2),)


def test_confirmed_empty_plan_derives_no_work_without_readiness_claim() -> None:
    source = RequiredDocumentPlanV1(
        job_id="example-role",
        input_fingerprint=SHA_A,
        requirements_state="confirmed_empty",
        requirements_basis_sha256=SHA_B,
        requirements=(),
        tasks=(),
    )

    plan = derive_document_execution_plan(source, source_plan_sha256="c" * 64)

    assert plan.state == "no_work"
    assert plan.items == ()
    assert plan.blockers == ()


def test_workflow_support_route_is_not_inferred_as_a_submission_executor() -> None:
    source = _source_plan(
        (("application_email", "required", "prepare", ()),)
    )

    plan = derive_document_execution_plan(source, source_plan_sha256="c" * 64)

    assert plan.items[0].executor_scope == "submission_document"
    assert plan.items[0].executor_availability == "unregistered"
    assert plan.items[0].route_id is None
    assert plan.items[0].reason_codes == ("documents.executor_unregistered",)


def test_execution_plan_schema_matches_model_and_rejects_false_ready_state() -> None:
    stored = json.loads(
        Path("schemas/document-execution-plan.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(stored)
    assert stored == DocumentExecutionPlanV1.model_json_schema(mode="validation")
    plan = derive_document_execution_plan(
        _mixed_source_plan(),
        source_plan_sha256="c" * 64,
    )
    payload = plan.model_dump(mode="json")
    Draft202012Validator(stored).validate(payload)

    false_ready = deepcopy(payload)
    false_ready["state"] = "ready"
    with pytest.raises(ValidationError):
        DocumentExecutionPlanV1.model_validate(false_ready)
    assert list(Draft202012Validator(stored).iter_errors(false_ready))

    false_planned_executor = deepcopy(payload)
    false_planned_executor["items"][3]["executor_id"] = "draft.research_statement"
    with pytest.raises(ValidationError):
        DocumentExecutionPlanV1.model_validate(false_planned_executor)
    assert list(Draft202012Validator(stored).iter_errors(false_planned_executor))


def test_body_free_status_exposes_counts_and_dispatchable_next_actions(tmp_path: Path) -> None:
    workspace = tmp_path / "workspace"
    job = workspace / "jobs" / "example-role"
    job.mkdir(parents=True)
    source_path = job / "required_document_plan.json"
    source = _mixed_source_plan()
    source_path.write_text(source.model_dump_json(indent=2) + "\n", encoding="utf-8")
    plan = derive_document_execution_plan(
        source,
        source_plan_sha256=sha256(source_path.read_bytes()).hexdigest(),
    )

    response = document_execution_status_agent_response(
        workspace,
        job,
        DocumentExecutionInspection(
            source_path=source_path,
            source_state="current",
            plan=plan,
        ),
    )

    assert response.ok is True
    assert response.workflow is not None
    assert response.workflow.readiness == "action_required"
    assert [item.id for item in response.next_actions] == [
        "stage.run_draft",
        "documents.review_capabilities",
    ]
    assert response.extensions["canisend.document_execution_item_count"] == 4
    assert response.extensions["canisend.document_ready_to_prepare_count"] == 2
    assert response.extensions["canisend.document_executor_unavailable_count"] == 1
    serialized = response.model_dump_json()
    assert PRIVATE_LABEL not in serialized
    assert PRIVATE_SOURCE not in serialized
    assert "research_statement" not in serialized
    assert _document_id(1) not in serialized


def test_documents_status_cli_is_read_only_and_uses_agent_response_v1(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workspace = tmp_path / "workspace"
    job = workspace / "jobs" / "example-role"
    job.mkdir(parents=True)
    (workspace / "canisend.yaml").write_text(
        "profile_dir: profile\njobs_dir: jobs\nschema_dir: schemas\n",
        encoding="utf-8",
    )
    source = _mixed_source_plan()
    (job / "required_document_plan.json").write_text(
        source.model_dump_json(indent=2) + "\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(
        "canisend.document_execution.inspect_stage_status",
        lambda *_args, **_kwargs: SimpleNamespace(
            stage=SimpleNamespace(status="succeeded"),
            reasons=(),
            output_drift=False,
        ),
    )
    before = _tree_hashes(workspace)

    result = CliRunner().invoke(
        app,
        [
            "documents",
            "status",
            "--workspace",
            str(workspace),
            "--job",
            "jobs/example-role",
            "--format",
            "json",
        ],
    )

    assert result.exit_code == 0, result.stdout
    payload = json.loads(result.stdout)
    assert payload["protocol"] == "canisend.agent/v1"
    assert payload["operation"] == "documents.status"
    assert payload["extensions"]["canisend.document_execution_state"] == (
        "partially_dispatchable"
    )
    assert payload["extensions"]["canisend.document_ready_to_prepare_count"] == 2
    assert payload["extensions"]["canisend.document_executor_unavailable_count"] == 1
    assert PRIVATE_LABEL not in result.stdout
    assert PRIVATE_SOURCE not in result.stdout
    assert _tree_hashes(workspace) == before
    assert "documents.status" in SUPPORTED_AGENT_OPERATIONS


def _tree_hashes(root: Path) -> dict[str, str]:
    return {
        path.relative_to(root).as_posix(): sha256(path.read_bytes()).hexdigest()
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }
