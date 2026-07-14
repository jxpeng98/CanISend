from __future__ import annotations

from datetime import UTC, datetime, timedelta
import json
from pathlib import Path

from jsonschema import (
    Draft202012Validator,
    ValidationError as JsonSchemaValidationError,
)
from pydantic import ValidationError
import pytest

from canisend.stage_models import (
    RUN_MANIFEST_SCHEMA_VERSION,
    TASK_RESULT_SCHEMA_VERSION,
    TASK_SPEC_SCHEMA_VERSION,
    WORKFLOW_STATE_SCHEMA_VERSION,
    ArtifactFingerprint,
    RunManifestV1,
    StageRecord,
    TaskResultV1,
    TaskSpecV1,
    ValidationReportV1,
    WorkflowStateV1,
)


NOW = datetime(2026, 7, 11, 9, 0, tzinfo=UTC)
LATER = NOW + timedelta(seconds=5)
SHA_A = "a" * 64
SHA_B = "b" * 64
RUN_ID = "run_0123456789abcdef0123456789abcdef"
TASK_ID = "task_0123456789abcdef0123456789abcdef"
JOB_ID = "2026-08-01_example-university_lecturer"
DOCUMENT_A = "document_0123456789abcdef0123456789abcdef"
DOCUMENT_B = "document_fedcba9876543210fedcba9876543210"


def fingerprint(
    path: str = "job_advert.md",
    digest: str = SHA_A,
) -> ArtifactFingerprint:
    return ArtifactFingerprint(path=path, sha256=digest, size_bytes=128)


def successful_stage() -> StageRecord:
    return StageRecord(
        stage="parse",
        status="succeeded",
        attempt_count=1,
        run_id=RUN_ID,
        input_fingerprint=SHA_A,
        inputs=(fingerprint(),),
        outputs=(fingerprint("parsed_job.json", SHA_B),),
        started_at=NOW,
        completed_at=LATER,
    )


def task_spec() -> TaskSpecV1:
    candidate_output = f"workflow/runs/{RUN_ID}/candidates/parsed_job.json"
    result_output = f"workflow/runs/{RUN_ID}/tasks/{TASK_ID}/result.json"
    return TaskSpecV1(
        task_id=TASK_ID,
        run_id=RUN_ID,
        job_id=JOB_ID,
        stage="parse",
        operation="stage.parse",
        execution_mode="host_agent",
        created_at=NOW,
        input_fingerprint=SHA_A,
        inputs=(fingerprint(),),
        allowed_reads=("job_advert.md",),
        allowed_writes=(candidate_output, result_output),
        candidate_output=candidate_output,
        result_output=result_output,
        authoritative_target="parsed_job.json",
        expected_output_sha256=None,
        output_schema="canisend.parsed-job/v1",
        privacy_tier=2,
        required_consents=("read-full-job-advert",),
    )


def task_result() -> TaskResultV1:
    return TaskResultV1(
        task_id=TASK_ID,
        run_id=RUN_ID,
        job_id=JOB_ID,
        stage="parse",
        status="succeeded",
        input_fingerprint=SHA_A,
        started_at=NOW,
        completed_at=LATER,
        outputs=(
            fingerprint(f"workflow/runs/{RUN_ID}/candidates/parsed_job.json", SHA_B),
        ),
    )


def validation_report() -> ValidationReportV1:
    return ValidationReportV1(
        task_id=TASK_ID,
        run_id=RUN_ID,
        job_id=JOB_ID,
        stage="parse",
        status="passed",
        checked_at=LATER,
        input_hashes_match=True,
        schema_valid=True,
        scope_valid=True,
        citations_valid=None,
    )


def run_manifest() -> RunManifestV1:
    return RunManifestV1(
        run_id=RUN_ID,
        task_id=TASK_ID,
        job_id=JOB_ID,
        stage="parse",
        attempt=1,
        execution_mode="host_agent",
        status="succeeded",
        created_at=NOW,
        started_at=NOW,
        completed_at=LATER,
        inputs=(fingerprint(),),
        input_fingerprint=SHA_A,
        task_spec_sha256=SHA_B,
        candidate_outputs=(
            fingerprint(f"workflow/runs/{RUN_ID}/candidates/parsed_job.json", SHA_B),
        ),
        promoted_outputs=(fingerprint("parsed_job.json", SHA_B),),
        validation_report_path=f"workflow/runs/{RUN_ID}/validation/report.json",
    )


@pytest.mark.parametrize(
    "unsafe_path",
    [
        "",
        ".",
        "../job_advert.md",
        "workflow/../job_advert.md",
        "/tmp/job_advert.md",
        "C:/Users/example/job_advert.md",
        r"C:\Users\example\job_advert.md",
        "workflow//state.json",
        "workflow/./state.json",
        "workflow/state.json/",
    ],
)
def test_artifact_fingerprint_rejects_unsafe_job_relative_paths(unsafe_path: str) -> None:
    with pytest.raises(ValidationError):
        fingerprint(unsafe_path)


def test_artifact_fingerprint_requires_lowercase_sha256_and_is_frozen() -> None:
    with pytest.raises(ValidationError):
        fingerprint(digest="A" * 64)

    artifact = fingerprint()
    with pytest.raises(ValidationError):
        artifact.path = "other.txt"


def test_models_reject_unknown_fields() -> None:
    with pytest.raises(ValidationError):
        ArtifactFingerprint(
            path="job_advert.md",
            sha256=SHA_A,
            private_body="must not be accepted",
        )


@pytest.mark.parametrize(
    ("status", "changes"),
    [
        ("running", {"run_id": None}),
        ("running", {"completed_at": LATER}),
        ("succeeded", {"outputs": ()}),
        ("failed", {"error_code": None}),
        ("pending", {"run_id": RUN_ID}),
        ("stale", {"outputs": ()}),
    ],
)
def test_stage_record_rejects_inconsistent_status(
    status: str,
    changes: dict[str, object],
) -> None:
    base: dict[str, object] = {
        "stage": "parse",
        "status": status,
        "attempt_count": 1 if status in {"running", "succeeded", "failed", "stale"} else 0,
        "run_id": RUN_ID if status in {"running", "succeeded", "failed", "stale"} else None,
        "input_fingerprint": SHA_A if status in {"running", "succeeded", "failed", "stale"} else None,
        "inputs": (fingerprint(),),
        "outputs": (fingerprint("parsed_job.json", SHA_B),)
        if status in {"succeeded", "stale"}
        else (),
        "started_at": NOW if status in {"running", "succeeded", "failed", "stale"} else None,
        "completed_at": LATER if status in {"succeeded", "failed", "stale"} else None,
        "error_code": "stage.parse_failed" if status == "failed" else None,
    }
    base.update(changes)

    with pytest.raises(ValidationError):
        StageRecord.model_validate(base)


def test_stage_record_rejects_naive_or_reversed_times() -> None:
    payload = successful_stage().model_dump()

    with pytest.raises(ValidationError):
        StageRecord.model_validate({**payload, "started_at": NOW.replace(tzinfo=None)})

    with pytest.raises(ValidationError):
        StageRecord.model_validate(
            {**payload, "started_at": LATER, "completed_at": NOW}
        )


def test_workflow_state_requires_unique_ordered_stages_and_matching_active_run() -> None:
    state = WorkflowStateV1(
        job_id=JOB_ID,
        revision=2,
        created_at=NOW,
        updated_at=LATER,
        stages=(
            StageRecord(stage="intake", status="ready"),
            successful_stage(),
        ),
    )

    assert state.schema_version == WORKFLOW_STATE_SCHEMA_VERSION

    with pytest.raises(ValidationError):
        WorkflowStateV1(
            job_id=JOB_ID,
            revision=0,
            created_at=NOW,
            updated_at=LATER,
            stages=(successful_stage(), successful_stage()),
        )

    with pytest.raises(ValidationError):
        WorkflowStateV1(
            job_id=JOB_ID,
            revision=0,
            created_at=NOW,
            updated_at=LATER,
            stages=(successful_stage(), StageRecord(stage="intake", status="ready")),
        )

    with pytest.raises(ValidationError):
        WorkflowStateV1(
            job_id=JOB_ID,
            revision=0,
            created_at=NOW,
            updated_at=LATER,
            active_run_id=RUN_ID,
            stages=(successful_stage(),),
        )


def test_workflow_state_owns_document_stage_instances_by_composite_identity() -> None:
    first = successful_stage().model_copy(
        update={
            "stage": "draft",
            "document_id": DOCUMENT_A,
            "outputs": (fingerprint("documents/first/draft.json", SHA_A),),
        }
    )
    second = successful_stage().model_copy(
        update={
            "stage": "draft",
            "document_id": DOCUMENT_B,
            "run_id": "run_fedcba9876543210fedcba9876543210",
            "outputs": (fingerprint("documents/second/draft.json", SHA_B),),
        }
    )

    state = WorkflowStateV1(
        job_id=JOB_ID,
        revision=2,
        created_at=NOW,
        updated_at=LATER,
        stages=(first, second),
    )

    assert tuple((item.stage, item.document_id) for item in state.stages) == (
        ("draft", DOCUMENT_A),
        ("draft", DOCUMENT_B),
    )
    with pytest.raises(ValidationError):
        WorkflowStateV1(
            job_id=JOB_ID,
            revision=2,
            created_at=NOW,
            updated_at=LATER,
            stages=(first, first),
        )
    with pytest.raises(ValidationError):
        WorkflowStateV1(
            job_id=JOB_ID,
            revision=2,
            created_at=NOW,
            updated_at=LATER,
            stages=(second, first),
        )


def test_document_identity_is_scoped_and_legacy_stage_contracts_remain_readable() -> None:
    with pytest.raises(ValidationError):
        StageRecord(stage="parse", document_id=DOCUMENT_A, status="ready")

    current = task_spec().model_dump()
    current.update(
        {
            "stage": "draft",
            "operation": "stage.draft",
            "document_id": None,
        }
    )
    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate(current)

    legacy = {**current, "schema_version": "1.0.0"}
    assert TaskSpecV1.model_validate(legacy).document_id is None
    legacy_state = WorkflowStateV1(
        schema_version="1.0.0",
        job_id=JOB_ID,
        revision=1,
        created_at=NOW,
        updated_at=LATER,
        stages=(
            successful_stage().model_copy(
                update={"stage": "draft", "document_id": None}
            ),
        ),
    )
    assert legacy_state.stages[0].document_id is None


def test_workflow_state_accepts_exactly_one_matching_running_stage() -> None:
    running = StageRecord(
        stage="parse",
        status="running",
        attempt_count=1,
        run_id=RUN_ID,
        input_fingerprint=SHA_A,
        inputs=(fingerprint(),),
        started_at=NOW,
    )

    state = WorkflowStateV1(
        job_id=JOB_ID,
        revision=1,
        created_at=NOW,
        updated_at=NOW,
        active_run_id=RUN_ID,
        stages=(running,),
    )

    assert state.active_run_id == RUN_ID


def test_task_spec_enforces_identity_operation_and_declared_scope() -> None:
    spec = task_spec()

    assert spec.schema_version == TASK_SPEC_SCHEMA_VERSION
    assert TaskSpecV1.model_validate(
        {**spec.model_dump(), "execution_mode": "deterministic"}
    ).execution_mode == "deterministic"

    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate({**spec.model_dump(), "task_id": "task_bad"})

    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate({**spec.model_dump(), "operation": "stage.match"})

    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate({**spec.model_dump(), "allowed_reads": ()})

    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate(
            {
                **spec.model_dump(),
                "allowed_writes": ("job_advert.md",),
            }
        )

    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate(
            {
                **spec.model_dump(),
                "candidate_output": "workflow/runs/other/candidate.json",
            }
        )

    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate(
            {
                **spec.model_dump(),
                "allowed_writes": (
                    "parsed_job.json",
                    spec.result_output,
                ),
                "candidate_output": "parsed_job.json",
            }
        )

    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate(
            {
                **spec.model_dump(),
                "allowed_writes": (spec.candidate_output,),
            }
        )

    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate(
            {**spec.model_dump(), "expected_output_sha256": "A" * 64}
        )

    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate(
            {
                **spec.model_dump(),
                "allowed_reads": ("job_advert.md", "../private.txt"),
            }
        )


def test_task_spec_rejects_naive_timestamp_and_duplicate_paths() -> None:
    spec = task_spec()

    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate(
            {**spec.model_dump(), "created_at": NOW.replace(tzinfo=None)}
        )

    with pytest.raises(ValidationError):
        TaskSpecV1.model_validate(
            {
                **spec.model_dump(),
                "allowed_reads": ("job_advert.md", "job_advert.md"),
            }
        )


def test_task_result_enforces_terminal_state_and_times() -> None:
    result = task_result()

    assert result.schema_version == TASK_RESULT_SCHEMA_VERSION
    assert result.input_fingerprint == task_spec().input_fingerprint

    with pytest.raises(ValidationError):
        TaskResultV1.model_validate({**result.model_dump(), "outputs": ()})

    with pytest.raises(ValidationError):
        TaskResultV1.model_validate(
            {
                **result.model_dump(),
                "status": "failed",
                "outputs": (),
                "error_code": None,
                "error_message": None,
            }
        )

    with pytest.raises(ValidationError):
        TaskResultV1.model_validate(
            {**result.model_dump(), "started_at": LATER, "completed_at": NOW}
        )


def test_validation_report_status_matches_checks_and_errors() -> None:
    report = validation_report()

    assert report.status == "passed"

    with pytest.raises(ValidationError):
        ValidationReportV1.model_validate(
            {**report.model_dump(), "schema_valid": False}
        )

    with pytest.raises(ValidationError):
        ValidationReportV1.model_validate(
            {**report.model_dump(), "status": "failed"}
        )

    failed = ValidationReportV1.model_validate(
        {
            **report.model_dump(),
            "status": "failed",
            "scope_valid": False,
            "errors": ("candidate path is outside the declared scope",),
        }
    )

    assert failed.errors


def test_run_manifest_enforces_lifecycle_and_host_task_identity() -> None:
    manifest = run_manifest()

    assert manifest.schema_version == RUN_MANIFEST_SCHEMA_VERSION

    with pytest.raises(ValidationError):
        RunManifestV1.model_validate({**manifest.model_dump(), "task_id": None})

    with pytest.raises(ValidationError):
        RunManifestV1.model_validate(
            {
                **manifest.model_dump(),
                "execution_mode": "configured_provider",
                "task_id": None,
            }
        )

    with pytest.raises(ValidationError):
        RunManifestV1.model_validate(
            {**manifest.model_dump(), "status": "running", "completed_at": LATER}
        )

    with pytest.raises(ValidationError):
        RunManifestV1.model_validate(
            {**manifest.model_dump(), "promoted_outputs": ()}
        )

    with pytest.raises(ValidationError):
        RunManifestV1.model_validate(
            {
                **manifest.model_dump(),
                "status": "failed",
                "promoted_outputs": (),
                "error_code": None,
                "error_message": None,
            }
        )


@pytest.mark.parametrize(
    ("schema_name", "model"),
    [
        ("workflow-state.schema.json", lambda: WorkflowStateV1(
            job_id=JOB_ID,
            revision=1,
            created_at=NOW,
            updated_at=LATER,
            stages=(successful_stage(),),
        )),
        ("task-spec.schema.json", task_spec),
        ("task-result.schema.json", task_result),
        ("run-manifest.schema.json", run_manifest),
    ],
)
def test_static_schema_is_strict_and_accepts_model_dump(
    schema_name: str,
    model: object,
) -> None:
    schema = json.loads((Path("schemas") / schema_name).read_text(encoding="utf-8"))

    Draft202012Validator.check_schema(schema)
    assert schema["$schema"] == "https://json-schema.org/draft/2020-12/schema"
    assert schema["additionalProperties"] is False
    Draft202012Validator(schema).validate(model().model_dump(mode="json"))


@pytest.mark.parametrize(
    ("schema_name", "payload"),
    [
        (
            "task-spec.schema.json",
            {
                **task_spec().model_dump(mode="json"),
                "stage": "draft",
                "operation": "stage.draft",
                "document_id": None,
            },
        ),
        (
            "task-result.schema.json",
            {
                **task_result().model_dump(mode="json"),
                "stage": "review",
                "document_id": None,
            },
        ),
        (
            "run-manifest.schema.json",
            {
                **run_manifest().model_dump(mode="json"),
                "stage": "draft",
                "document_id": None,
            },
        ),
    ],
)
def test_static_schema_requires_document_identity_for_current_document_stages(
    schema_name: str,
    payload: dict[str, object],
) -> None:
    schema = json.loads((Path("schemas") / schema_name).read_text(encoding="utf-8"))

    with pytest.raises(JsonSchemaValidationError):
        Draft202012Validator(schema).validate(payload)

    payload["schema_version"] = "1.0.0"
    payload.pop("document_id")
    Draft202012Validator(schema).validate(payload)


def test_workflow_schema_accepts_legacy_document_record_but_requires_current_identity() -> None:
    schema = json.loads(
        (Path("schemas") / "workflow-state.schema.json").read_text(encoding="utf-8")
    )
    legacy = WorkflowStateV1(
        schema_version="1.0.0",
        job_id=JOB_ID,
        revision=1,
        created_at=NOW,
        updated_at=LATER,
        stages=(
            successful_stage().model_copy(
                update={"stage": "draft", "document_id": None}
            ),
        ),
    ).model_dump(mode="json")
    legacy["stages"][0].pop("document_id")

    Draft202012Validator(schema).validate(legacy)
    cross_scoped = {
        **legacy,
        "stages": [
            {
                **legacy["stages"][0],
                "stage": "parse",
                "document_id": DOCUMENT_A,
            }
        ],
    }
    with pytest.raises(JsonSchemaValidationError):
        Draft202012Validator(schema).validate(cross_scoped)
    current = {**legacy, "schema_version": "1.1.0"}
    with pytest.raises(JsonSchemaValidationError):
        Draft202012Validator(schema).validate(current)
