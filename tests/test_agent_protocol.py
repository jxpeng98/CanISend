from __future__ import annotations

import json
import os
from pathlib import Path

from jsonschema import Draft202012Validator
from pydantic import ValidationError
import pytest

from canisend.agent_protocol import (
    AGENT_PROTOCOL,
    AGENT_SCHEMA_VERSION,
    AgentError,
    AgentResponse,
    ArtifactReference,
    ConsentRequirement,
    GateOutcome,
    JobReference,
    NextAction,
    WorkflowSnapshotReference,
    artifact_reference_from_path,
    dumps_agent_response,
    error_response,
    success_response,
)


def test_agent_response_serializes_protocol_and_operation() -> None:
    response = success_response(
        operation="workspace.inspect",
        warnings=["profile evidence is missing"],
    )

    serialized = dumps_agent_response(response)
    payload = json.loads(serialized)

    assert serialized.endswith("\n")
    assert not serialized.endswith("\n\n")
    assert payload["protocol"] == AGENT_PROTOCOL
    assert payload["schema_version"] == AGENT_SCHEMA_VERSION
    assert payload["operation"] == "workspace.inspect"
    assert payload["ok"] is True
    assert payload["request_id"].startswith("req_")
    assert payload["warnings"] == ["profile evidence is missing"]
    assert payload["error"] is None


def test_agent_response_rejects_unknown_fields() -> None:
    with pytest.raises(ValidationError):
        AgentResponse.model_validate(
            {
                "operation": "workspace.inspect",
                "ok": True,
                "job_advert_body": "private text",
            }
        )


@pytest.mark.parametrize(
    "unsafe_path",
    [
        "../profile/cv.typ",
        "/private/tmp/job.yaml",
        "C:/Users/example/job.yaml",
        r"C:\Users\example\job.yaml",
        "jobs/./role/job.yaml",
    ],
)
def test_artifact_reference_rejects_unsafe_relative_paths(unsafe_path: str) -> None:
    with pytest.raises(ValidationError):
        ArtifactReference(
            kind="job_metadata",
            path=unsafe_path,
            privacy_tier=1,
            trust_level="validated",
            exists=True,
        )


def test_artifact_reference_accepts_normalized_workspace_relative_path() -> None:
    reference = ArtifactReference(
        kind="job_metadata",
        path="jobs/example/job.yaml",
        privacy_tier=1,
        trust_level="validated",
        exists=True,
    )

    assert reference.path == "jobs/example/job.yaml"
    assert reference.opaque_id is None


def test_artifact_reference_requires_exactly_one_locator() -> None:
    with pytest.raises(ValidationError):
        ArtifactReference(
            kind="job_metadata",
            privacy_tier=1,
            trust_level="validated",
            exists=True,
        )

    with pytest.raises(ValidationError):
        ArtifactReference(
            kind="job_metadata",
            path="jobs/example/job.yaml",
            opaque_id="external-0123456789ab",
            privacy_tier=1,
            trust_level="validated",
            exists=True,
        )


def test_external_artifact_uses_opaque_id_without_basename(tmp_path: Path) -> None:
    workspace = tmp_path / "workspace"
    workspace.mkdir()
    external = tmp_path / "Peng_CV.pdf"
    external.write_bytes(b"private pdf")

    reference = artifact_reference_from_path(
        workspace=workspace,
        path=external,
        kind="profile_source",
        privacy_tier=2,
        trust_level="trusted_local",
        media_type="application/pdf",
        include_hash=True,
    )
    serialized = json.dumps(reference.model_dump(mode="json"), sort_keys=True)

    assert reference.path is None
    assert reference.opaque_id is not None
    assert reference.opaque_id.startswith("external-")
    assert reference.sha256 is None
    assert "Peng" not in serialized
    assert "CV.pdf" not in serialized
    assert str(tmp_path) not in serialized


def test_internal_artifact_can_include_sha256(tmp_path: Path) -> None:
    workspace = tmp_path / "workspace"
    artifact = workspace / "jobs" / "example" / "job.yaml"
    artifact.parent.mkdir(parents=True)
    artifact.write_text("title: Lecturer\n", encoding="utf-8")

    reference = artifact_reference_from_path(
        workspace=workspace,
        path=artifact,
        kind="job_metadata",
        privacy_tier=1,
        trust_level="validated",
        include_hash=True,
    )

    assert reference.path == "jobs/example/job.yaml"
    assert reference.opaque_id is None
    assert reference.sha256 is not None
    assert len(reference.sha256) == 64


@pytest.mark.skipif(os.name == "nt", reason="creating symlinks can require elevated Windows privileges")
def test_artifact_reference_treats_symlink_escape_as_external(tmp_path: Path) -> None:
    workspace = tmp_path / "workspace"
    workspace.mkdir()
    external = tmp_path / "private.txt"
    external.write_text("secret", encoding="utf-8")
    symlink = workspace / "linked.txt"
    symlink.symlink_to(external)

    reference = artifact_reference_from_path(
        workspace=workspace,
        path=symlink,
        kind="external_source",
        privacy_tier=2,
        trust_level="untrusted_import",
    )

    assert reference.path is None
    assert reference.opaque_id is not None


def test_agent_response_validates_ok_and_error_consistency() -> None:
    with pytest.raises(ValidationError):
        AgentResponse(operation="job.intake", ok=False)

    with pytest.raises(ValidationError):
        AgentResponse(
            operation="job.intake",
            ok=True,
            error=AgentError(code="input.invalid", message="bad input"),
        )

    response = error_response(
        operation="job.intake",
        code="input.invalid",
        message="bad input",
    )

    assert response.ok is False
    assert response.error is not None
    assert response.error.code == "input.invalid"


def test_completed_failed_gate_is_not_an_operational_error() -> None:
    response = success_response(
        operation="package.check",
        gate=GateOutcome(status="FAIL", issue_count=2),
        blockers=["two package issues remain"],
    )

    assert response.ok is True
    assert response.error is None
    assert response.gate is not None
    assert response.gate.status == "FAIL"


def test_protocol_models_reject_structured_extension_values() -> None:
    with pytest.raises(ValidationError):
        AgentResponse(
            operation="workspace.inspect",
            ok=True,
            extensions={"canisend.example": {"private": "value"}},
        )

    response = AgentResponse(
        operation="workspace.inspect",
        ok=True,
        extensions={"canisend.example": "enabled"},
    )

    assert response.extensions == {"canisend.example": "enabled"}


def test_protocol_models_cover_job_workflow_actions_and_consent() -> None:
    response = success_response(
        operation="job.inspect",
        job=JobReference(
            id="2026-08-01_example-university_lecturer",
            path="jobs/2026-08-01_example-university_lecturer",
            title="Lecturer",
            institution="Example University",
            deadline="2026-08-01",
            status="advert_imported",
        ),
        workflow=WorkflowSnapshotReference(
            phase="parse",
            readiness="action_required",
            derived=True,
        ),
        required_consents=[
            ConsentRequirement(
                id="read-full-job-advert",
                purpose="Allow the host agent to read the full job advert.",
                privacy_tier=2,
                artifact_kinds=["job_advert"],
            )
        ],
        next_actions=[
            NextAction(
                id="job.parse",
                label="Parse the reviewed job advert",
                requires_consent=True,
                consent_ids=["read-full-job-advert"],
            )
        ],
    )

    assert response.job is not None
    assert response.workflow is not None
    assert response.next_actions[0].consent_ids == ["read-full-job-advert"]


def test_packaged_agent_response_schema_declares_strict_contract() -> None:
    schema = json.loads(Path("schemas/agent-response.schema.json").read_text(encoding="utf-8"))

    assert schema["$schema"] == "https://json-schema.org/draft/2020-12/schema"
    assert schema["title"] == "CanISendAgentResponse"
    assert schema["additionalProperties"] is False
    assert set(schema["required"]) >= {
        "protocol",
        "schema_version",
        "request_id",
        "operation",
        "ok",
        "artifacts",
        "missing_fields",
        "required_consents",
        "warnings",
        "blockers",
        "next_actions",
        "extensions",
    }
    assert schema["properties"]["protocol"]["const"] == AGENT_PROTOCOL
    assert schema["properties"]["schema_version"]["const"] == AGENT_SCHEMA_VERSION


def test_agent_response_model_dump_conforms_to_packaged_schema() -> None:
    schema = json.loads(Path("schemas/agent-response.schema.json").read_text(encoding="utf-8"))
    response = success_response(
        operation="workspace.inspect",
        artifacts=[
            ArtifactReference(
                kind="workspace_config",
                path="config.yaml",
                exists=True,
                privacy_tier=1,
                trust_level="validated",
            )
        ],
    )

    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(response.model_dump(mode="json"))
