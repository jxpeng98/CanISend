from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal

from pydantic import ValidationError

from canisend.decision_models import CriteriaCatalogV1, EvidenceCatalogV1
from canisend.stage_models import ArtifactFingerprint
from canisend.stage_store import (
    StageStoreError,
    read_json_object,
    resolve_job_relative_path,
    sha256_file,
    write_immutable_json,
)
from canisend.stages.confirm_stage import (
    CONFIRMED_CORRECTIONS_PATH,
    CRITERIA_OUTPUT_PATH,
    build_deterministic_confirm_candidate,
    confirm_input_fingerprint,
    validate_confirm_candidate,
)
from canisend.stages.evidence_stage import (
    EVIDENCE_OUTPUT_PATH,
    EVIDENCE_SNAPSHOT_NAME,
    EvidenceStageError,
    build_deterministic_evidence_candidate,
    evidence_input_fingerprint,
    validate_evidence_candidate,
)
from canisend.stages.match_stage import (
    CRITERIA_INPUT_PATH,
    CRITERION_MATCHES_OUTPUT_PATH,
    EVIDENCE_CATALOG_INPUT_PATH,
    build_deterministic_match_candidate,
    match_input_fingerprint,
    validate_match_candidate,
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

    def prepare_input_artifacts(
        self,
        workspace: Path,
        job_dir: Path,
        *,
        run_root: str,
        input_fingerprint: str,
    ) -> tuple[ArtifactFingerprint, ...]:
        """Materialize any run-scoped inputs before the immutable TaskSpec is written."""

        return self.input_artifacts(workspace, job_dir)

    def prepared_inputs_are_current(
        self,
        workspace: Path,
        job_dir: Path,
        *,
        inputs: tuple[ArtifactFingerprint, ...],
        input_fingerprint: str,
    ) -> bool:
        """Return whether immutable prepared inputs still represent the current source state."""

        return self.input_artifacts(workspace, job_dir) == inputs

    def expected_prepared_input_paths(self, run_id: str) -> tuple[str, ...] | None:
        """Return an exact run-bound input scope when a stage materializes inputs."""

        return None

    def build_deterministic_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        *,
        input_fingerprint: str,
        inputs: tuple[ArtifactFingerprint, ...],
    ) -> dict[str, Any]:
        raise NotImplementedError

    def validate_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        candidate: object,
        *,
        input_fingerprint: str,
        inputs: tuple[ArtifactFingerprint, ...],
    ) -> dict[str, Any]:
        raise NotImplementedError

    def required_consents(self, execution_mode: AdapterExecutionMode) -> tuple[str, ...]:
        return ()

    def precondition_reasons(
        self,
        workspace: Path,
        job_dir: Path,
    ) -> tuple[str, ...]:
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
        inputs: tuple[ArtifactFingerprint, ...],
    ) -> dict[str, Any]:
        return build_deterministic_parse_candidate(job_dir)

    def validate_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        candidate: object,
        *,
        input_fingerprint: str,
        inputs: tuple[ArtifactFingerprint, ...],
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
        inputs: tuple[ArtifactFingerprint, ...],
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
        inputs: tuple[ArtifactFingerprint, ...],
    ) -> dict[str, Any]:
        validated = validate_confirm_candidate(
            candidate,
            job_dir=job_dir,
            input_fingerprint=input_fingerprint,
            parsed_job_schema_path=_schema_path(workspace, "parsed_job.schema.json"),
            criteria_schema_path=_schema_path(workspace, "criteria.schema.json"),
        )
        return validated.model_dump(mode="json")


class EvidenceStageAdapter(StageAdapter):
    def input_fingerprint(self, workspace: Path, job_dir: Path) -> str:
        return evidence_input_fingerprint(
            workspace,
            job_dir,
            evidence_schema_path=_schema_path(workspace, "evidence-catalog.schema.json"),
        )

    def input_artifacts(
        self,
        workspace: Path,
        job_dir: Path,
    ) -> tuple[ArtifactFingerprint, ...]:
        raise EvidenceStageError(
            "Evidence inputs must be materialized into an immutable run-scoped snapshot."
        )

    def prepare_input_artifacts(
        self,
        workspace: Path,
        job_dir: Path,
        *,
        run_root: str,
        input_fingerprint: str,
    ) -> tuple[ArtifactFingerprint, ...]:
        snapshot = build_deterministic_evidence_candidate(
            workspace,
            job_dir,
            input_fingerprint=input_fingerprint,
            evidence_schema_path=_schema_path(workspace, "evidence-catalog.schema.json"),
        )
        relative_path = f"{run_root}/inputs/{EVIDENCE_SNAPSHOT_NAME}"
        snapshot_path = resolve_job_relative_path(job_dir, relative_path)
        write_immutable_json(snapshot_path, snapshot.model_dump(mode="json"))
        return (_artifact(job_dir, relative_path),)

    def prepared_inputs_are_current(
        self,
        workspace: Path,
        job_dir: Path,
        *,
        inputs: tuple[ArtifactFingerprint, ...],
        input_fingerprint: str,
    ) -> bool:
        if len(inputs) != 1:
            return False
        receipt = inputs[0]
        parts = Path(receipt.path).parts
        if (
            len(parts) != 5
            or parts[0:2] != ("workflow", "runs")
            or parts[3:] != ("inputs", EVIDENCE_SNAPSHOT_NAME)
        ):
            return False
        try:
            if _unaliased_artifact(job_dir, receipt.path) != receipt:
                return False
            snapshot = EvidenceCatalogV1.model_validate(
                read_json_object(resolve_job_relative_path(job_dir, receipt.path))
            )
        except (StageStoreError, ValidationError):
            return False
        return (
            snapshot.job_id == job_dir.name
            and snapshot.input_fingerprint == input_fingerprint
        )

    def expected_prepared_input_paths(self, run_id: str) -> tuple[str, ...]:
        return (
            f"workflow/runs/{run_id}/inputs/{EVIDENCE_SNAPSHOT_NAME}",
        )

    def build_deterministic_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        *,
        input_fingerprint: str,
        inputs: tuple[ArtifactFingerprint, ...],
    ) -> dict[str, Any]:
        snapshot = self._validated_snapshot(
            workspace,
            job_dir,
            input_fingerprint=input_fingerprint,
            inputs=inputs,
        )
        return snapshot.model_dump(mode="json")

    def validate_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        candidate: object,
        *,
        input_fingerprint: str,
        inputs: tuple[ArtifactFingerprint, ...],
    ) -> dict[str, Any]:
        validated = validate_evidence_candidate(
            candidate,
            workspace=workspace,
            job_dir=job_dir,
            input_fingerprint=input_fingerprint,
            evidence_schema_path=_schema_path(workspace, "evidence-catalog.schema.json"),
        )
        snapshot = self._validated_snapshot(
            workspace,
            job_dir,
            input_fingerprint=input_fingerprint,
            inputs=inputs,
        )
        if validated != snapshot:
            raise EvidenceStageError(
                "Evidence candidate does not match its immutable prepared snapshot."
            )
        return validated.model_dump(mode="json")

    def _validated_snapshot(
        self,
        workspace: Path,
        job_dir: Path,
        *,
        input_fingerprint: str,
        inputs: tuple[ArtifactFingerprint, ...],
    ) -> EvidenceCatalogV1:
        if not self.prepared_inputs_are_current(
            workspace,
            job_dir,
            inputs=inputs,
            input_fingerprint=input_fingerprint,
        ):
            raise EvidenceStageError("Evidence snapshot is missing, stale, or invalid.")
        try:
            return EvidenceCatalogV1.model_validate(
                read_json_object(resolve_job_relative_path(job_dir, inputs[0].path))
            )
        except (StageStoreError, ValidationError) as exc:
            raise EvidenceStageError("Evidence snapshot is not a valid catalog.") from exc


class MatchStageAdapter(StageAdapter):
    def precondition_reasons(
        self,
        workspace: Path,
        job_dir: Path,
    ) -> tuple[str, ...]:
        try:
            criteria = CriteriaCatalogV1.model_validate(
                read_json_object(job_dir / CRITERIA_INPUT_PATH)
            )
        except (StageStoreError, ValidationError):
            return ()
        return (
            ("input_not_ready:criteria_review",)
            if criteria.extraction_state == "unknown"
            else ()
        )

    def input_fingerprint(self, workspace: Path, job_dir: Path) -> str:
        return match_input_fingerprint(
            job_dir,
            criterion_matches_schema_path=_schema_path(
                workspace,
                "criterion-matches.schema.json",
            ),
        )

    def input_artifacts(
        self,
        workspace: Path,
        job_dir: Path,
    ) -> tuple[ArtifactFingerprint, ...]:
        return (
            _artifact(job_dir, CRITERIA_INPUT_PATH),
            _artifact(job_dir, EVIDENCE_CATALOG_INPUT_PATH),
        )

    def build_deterministic_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        *,
        input_fingerprint: str,
        inputs: tuple[ArtifactFingerprint, ...],
    ) -> dict[str, Any]:
        candidate = build_deterministic_match_candidate(
            job_dir,
            input_fingerprint=input_fingerprint,
            criterion_matches_schema_path=_schema_path(
                workspace,
                "criterion-matches.schema.json",
            ),
        )
        return candidate.model_dump(mode="json")

    def validate_candidate(
        self,
        workspace: Path,
        job_dir: Path,
        candidate: object,
        *,
        input_fingerprint: str,
        inputs: tuple[ArtifactFingerprint, ...],
    ) -> dict[str, Any]:
        validated = validate_match_candidate(
            candidate,
            job_dir=job_dir,
            input_fingerprint=input_fingerprint,
            criterion_matches_schema_path=_schema_path(
                workspace,
                "criterion-matches.schema.json",
            ),
        )
        return validated.model_dump(mode="json")


_ADAPTERS = {
    "evidence": EvidenceStageAdapter(
        stage_id="evidence",
        authoritative_target=EVIDENCE_OUTPUT_PATH,
        candidate_name=EVIDENCE_OUTPUT_PATH,
        output_schema="canisend.evidence-catalog/v1",
        artifact_kind="evidence_catalog",
        media_type="application/json",
        privacy_tier=2,
        task_privacy_tier=2,
        citations_validated=True,
    ),
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
    "match": MatchStageAdapter(
        stage_id="match",
        authoritative_target=CRITERION_MATCHES_OUTPUT_PATH,
        candidate_name=CRITERION_MATCHES_OUTPUT_PATH,
        output_schema="canisend.criterion-matches/v1",
        artifact_kind="criterion_matches",
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


def _unaliased_artifact(job_dir: Path, relative_path: str) -> ArtifactFingerprint:
    path = resolve_job_relative_path(job_dir, relative_path)
    metadata = path.lstat()
    if path.is_symlink() or not path.is_file() or metadata.st_nlink != 1:
        raise StageStoreError("Stage input must be one unaliased regular file.")
    return _artifact(job_dir, relative_path)
