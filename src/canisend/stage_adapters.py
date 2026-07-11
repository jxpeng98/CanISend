from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal

from canisend.stage_models import ArtifactFingerprint
from canisend.stage_store import resolve_job_relative_path, sha256_file
from canisend.stages.confirm_stage import (
    CONFIRMED_CORRECTIONS_PATH,
    CRITERIA_OUTPUT_PATH,
    build_deterministic_confirm_candidate,
    confirm_input_fingerprint,
    validate_confirm_candidate,
)
from canisend.stages.parse_stage import (
    build_deterministic_parse_candidate,
    parse_input_fingerprint,
    validate_parse_candidate,
)
from canisend.workspace import load_workspace_config


AdapterExecutionMode = Literal["deterministic", "host_agent"]


@dataclass(frozen=True)
class StageAdapter:
    stage_id: str
    authoritative_target: str
    candidate_name: str
    output_schema: str
    artifact_kind: str
    media_type: str
    privacy_tier: int
    task_privacy_tier: int
    citations_validated: bool

    def input_fingerprint(self, workspace: Path, job_dir: Path) -> str:
        raise NotImplementedError

    def input_artifacts(
        self,
        workspace: Path,
        job_dir: Path,
    ) -> tuple[ArtifactFingerprint, ...]:
        raise NotImplementedError

    def build_deterministic_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        *,
        input_fingerprint: str,
    ) -> dict[str, Any]:
        raise NotImplementedError

    def validate_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        candidate: object,
        *,
        input_fingerprint: str,
    ) -> dict[str, Any]:
        raise NotImplementedError

    def required_consents(self, execution_mode: AdapterExecutionMode) -> tuple[str, ...]:
        return ()


class ParseStageAdapter(StageAdapter):
    def input_fingerprint(self, workspace: Path, job_dir: Path) -> str:
        return parse_input_fingerprint(
            job_dir,
            schema_path=_schema_path(workspace, "parsed_job.schema.json"),
        )

    def input_artifacts(
        self,
        workspace: Path,
        job_dir: Path,
    ) -> tuple[ArtifactFingerprint, ...]:
        return (
            _artifact(job_dir, "job.yaml"),
            _artifact(job_dir, "job_advert.md"),
        )

    def build_deterministic_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        *,
        input_fingerprint: str,
    ) -> dict[str, Any]:
        return build_deterministic_parse_candidate(job_dir)

    def validate_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        candidate: object,
        *,
        input_fingerprint: str,
    ) -> dict[str, Any]:
        advert_text = (job_dir / "job_advert.md").read_text(encoding="utf-8")
        return validate_parse_candidate(
            candidate,
            advert_text=advert_text,
            schema_path=_schema_path(workspace, "parsed_job.schema.json"),
        )

    def required_consents(self, execution_mode: AdapterExecutionMode) -> tuple[str, ...]:
        return ("read-full-job-advert",) if execution_mode == "host_agent" else ()


class ConfirmStageAdapter(StageAdapter):
    def input_fingerprint(self, workspace: Path, job_dir: Path) -> str:
        return confirm_input_fingerprint(
            job_dir,
            parsed_job_schema_path=_schema_path(workspace, "parsed_job.schema.json"),
            criteria_schema_path=_schema_path(workspace, "criteria.schema.json"),
        )

    def input_artifacts(
        self,
        workspace: Path,
        job_dir: Path,
    ) -> tuple[ArtifactFingerprint, ...]:
        paths = ["parsed_job.json", "job_advert.md"]
        if (job_dir / CONFIRMED_CORRECTIONS_PATH).exists():
            paths.append(CONFIRMED_CORRECTIONS_PATH)
        return tuple(_artifact(job_dir, path) for path in paths)

    def build_deterministic_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        *,
        input_fingerprint: str,
    ) -> dict[str, Any]:
        candidate = build_deterministic_confirm_candidate(
            job_dir,
            input_fingerprint=input_fingerprint,
            parsed_job_schema_path=_schema_path(workspace, "parsed_job.schema.json"),
            criteria_schema_path=_schema_path(workspace, "criteria.schema.json"),
        )
        return candidate.model_dump(mode="json")

    def validate_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        candidate: object,
        *,
        input_fingerprint: str,
    ) -> dict[str, Any]:
        validated = validate_confirm_candidate(
            candidate,
            job_dir=job_dir,
            input_fingerprint=input_fingerprint,
            parsed_job_schema_path=_schema_path(workspace, "parsed_job.schema.json"),
            criteria_schema_path=_schema_path(workspace, "criteria.schema.json"),
        )
        return validated.model_dump(mode="json")


_ADAPTERS = {
    "parse": ParseStageAdapter(
        stage_id="parse",
        authoritative_target="parsed_job.json",
        candidate_name="parsed_job.json",
        output_schema="canisend.parsed-job/v1",
        artifact_kind="parsed_job",
        media_type="application/json",
        privacy_tier=1,
        task_privacy_tier=2,
        citations_validated=True,
    ),
    "confirm": ConfirmStageAdapter(
        stage_id="confirm",
        authoritative_target=CRITERIA_OUTPUT_PATH,
        candidate_name=CRITERIA_OUTPUT_PATH,
        output_schema="canisend.criteria/v1",
        artifact_kind="criteria_catalog",
        media_type="application/json",
        privacy_tier=2,
        task_privacy_tier=2,
        citations_validated=True,
    ),
}


def get_stage_adapter(stage_id: str) -> StageAdapter:
    try:
        return _ADAPTERS[stage_id]
    except KeyError as exc:
        raise KeyError(f"stage has no executable adapter: {stage_id}") from exc


def _schema_path(workspace: Path, filename: str) -> Path:
    return load_workspace_config(workspace).path("schema_dir") / filename


def _artifact(job_dir: Path, relative_path: str) -> ArtifactFingerprint:
    path = resolve_job_relative_path(job_dir, relative_path)
    return ArtifactFingerprint(
        path=relative_path,
        sha256=sha256_file(path),
        size_bytes=path.stat().st_size,
    )
