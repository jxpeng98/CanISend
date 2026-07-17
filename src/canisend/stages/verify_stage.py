from __future__ import annotations

from datetime import UTC, datetime
import json
from pathlib import Path, PurePosixPath
import re
from typing import Literal

from pydantic import AwareDatetime, BaseModel, ConfigDict, Field, field_validator, model_validator

from canisend.bundle_projection import (
    BundleProjectionError,
    canonical_bundle_bytes,
    inspect_artifact_projection,
    load_artifact_bundle,
)
from canisend.stage_models import ArtifactFingerprint
from canisend.stage_store import (
    StageStoreError,
    read_json_object,
    resolve_job_relative_path,
    sha256_bytes,
    sha256_file,
    write_immutable_json,
)


VERIFY_OUTPUT_PATH = "application_gate_report.json"
VERIFY_BASIS_NAME = "package-check-basis.json"
VERIFY_OUTPUT_SCHEMA = "canisend.application-gate-report/v1"
_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


class VerifyStageError(ValueError):
    """A body-free Verify stage validation failure."""


class VerifyContractModel(BaseModel):
    model_config = ConfigDict(
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
    )


class VerifyIssueV1(VerifyContractModel):
    gate: str = Field(pattern=r"^APP-Q[1-9][0-9]*$")
    path: str = Field(min_length=1, max_length=1000)
    message: str = Field(min_length=1, max_length=10_000)


class VerifyBasisV1(VerifyContractModel):
    schema_version: Literal["1.0.0"] = "1.0.0"
    job_id: str = Field(pattern=r"^[a-z0-9][a-z0-9_.-]*$")
    package_bundle_sha256: str
    projection_journal_sha256: str
    status: Literal["PASS", "FAIL"]
    input_hashes: dict[str, str]
    issues: tuple[VerifyIssueV1, ...]

    @field_validator("package_bundle_sha256", "projection_journal_sha256")
    @classmethod
    def _hash(cls, value: str) -> str:
        if _SHA256_RE.fullmatch(value) is None:
            raise ValueError("Verify receipt hashes must be lowercase SHA-256 values")
        return value

    @field_validator("input_hashes")
    @classmethod
    def _input_hashes(cls, values: dict[str, str]) -> dict[str, str]:
        if tuple(values) != tuple(sorted(values)):
            raise ValueError("Verify input hashes must use deterministic label ordering")
        for label, value in values.items():
            path = PurePosixPath(label)
            if (
                path.is_absolute()
                or not label
                or "\\" in label
                or any(part in {"", ".", ".."} for part in path.parts)
                or _SHA256_RE.fullmatch(value) is None
            ):
                raise ValueError("Verify input hash labels or values are invalid")
        return values

    @model_validator(mode="after")
    def _status_matches_issues(self) -> VerifyBasisV1:
        if (self.status == "PASS") != (not self.issues):
            raise ValueError("Verify status must match the issue set")
        return self


class ApplicationGateReportV1(VerifyBasisV1):
    generated_at: AwareDatetime
    input_fingerprint: str

    @field_validator("input_fingerprint")
    @classmethod
    def _input_fingerprint(cls, value: str) -> str:
        if _SHA256_RE.fullmatch(value) is None:
            raise ValueError("Verify input fingerprint must be a lowercase SHA-256 value")
        return value


def build_verify_basis(workspace: Path, job_dir: Path) -> VerifyBasisV1:
    from canisend.ready_check import check_application_package
    from canisend.workspace import load_workspace_config

    bundle_path = job_dir / "package_bundle.json"
    projection_path = job_dir / "workflow" / "projections" / "package.json"
    try:
        bundle = load_artifact_bundle(bundle_path)
        if bundle.stage != "package" or bundle.mode != "guarded":
            raise VerifyStageError("Verify requires one guarded Package bundle.")
        projection = inspect_artifact_projection(job_dir, bundle)
        if not projection.current:
            raise VerifyStageError("Package projections are missing or locally drifted.")
        projection_document = read_json_object(projection_path)
        if projection_document.get("bundle_sha256") != sha256_bytes(
            canonical_bundle_bytes(bundle)
        ):
            raise VerifyStageError("The Package projection journal is stale.")
        config = load_workspace_config(workspace)
        result = check_application_package(
            job_dir,
            config.path("profile_dir"),
            workspace=config.root,
        )
        return VerifyBasisV1(
            job_id=job_dir.name,
            package_bundle_sha256=sha256_file(bundle_path),
            projection_journal_sha256=sha256_file(projection_path),
            status=result.status,
            input_hashes=dict(sorted(result.input_hashes.items())),
            issues=tuple(
                VerifyIssueV1(
                    gate=issue.gate,
                    path=issue.path,
                    message=issue.message,
                )
                for issue in result.issues
            ),
        )
    except (BundleProjectionError, OSError, StageStoreError) as exc:
        raise VerifyStageError("Verify inputs are missing, stale, or unsafe.") from exc


def verify_input_fingerprint(workspace: Path, job_dir: Path) -> str:
    return _basis_fingerprint(build_verify_basis(workspace, job_dir))


def verify_precondition_reasons(workspace: Path, job_dir: Path) -> tuple[str, ...]:
    try:
        build_verify_basis(workspace, job_dir)
    except (VerifyStageError, ValueError):
        return ("input_not_ready:package_projection",)
    return ()


def prepare_verify_basis(
    workspace: Path,
    job_dir: Path,
    *,
    run_root: str,
    input_fingerprint: str,
) -> tuple[ArtifactFingerprint, ...]:
    basis = build_verify_basis(workspace, job_dir)
    if _basis_fingerprint(basis) != input_fingerprint:
        raise VerifyStageError("Verify inputs changed during preparation.")
    relative_path = f"{run_root}/inputs/{VERIFY_BASIS_NAME}"
    path = resolve_job_relative_path(job_dir, relative_path)
    try:
        write_immutable_json(path, basis.model_dump(mode="json"))
        return (
            ArtifactFingerprint(
                path=relative_path,
                sha256=sha256_file(path),
                size_bytes=path.stat().st_size,
            ),
        )
    except (OSError, StageStoreError) as exc:
        raise VerifyStageError("The Verify basis snapshot could not be stored safely.") from exc


def verify_prepared_inputs_are_current(
    workspace: Path,
    job_dir: Path,
    *,
    inputs: tuple[ArtifactFingerprint, ...],
    input_fingerprint: str,
) -> bool:
    if len(inputs) != 1 or not inputs[0].path.endswith(f"/inputs/{VERIFY_BASIS_NAME}"):
        return False
    try:
        path = resolve_job_relative_path(job_dir, inputs[0].path)
        stored = VerifyBasisV1.model_validate(read_json_object(path))
        return (
            sha256_file(path) == inputs[0].sha256
            and path.stat().st_size == inputs[0].size_bytes
            and _basis_fingerprint(stored) == input_fingerprint
            and verify_input_fingerprint(workspace, job_dir) == input_fingerprint
        )
    except (OSError, StageStoreError, ValidationError, VerifyStageError, ValueError):
        return False


def build_verify_candidate(
    job_dir: Path,
    *,
    input_fingerprint: str,
    inputs: tuple[ArtifactFingerprint, ...],
) -> ApplicationGateReportV1:
    if len(inputs) != 1:
        raise VerifyStageError("Verify requires one immutable basis snapshot.")
    try:
        basis = VerifyBasisV1.model_validate(
            read_json_object(resolve_job_relative_path(job_dir, inputs[0].path))
        )
    except (StageStoreError, ValidationError) as exc:
        raise VerifyStageError("The Verify basis snapshot is invalid.") from exc
    if _basis_fingerprint(basis) != input_fingerprint:
        raise VerifyStageError("The Verify basis does not match its task fingerprint.")
    return ApplicationGateReportV1(
        **basis.model_dump(mode="python"),
        generated_at=datetime.now(UTC).replace(microsecond=0),
        input_fingerprint=input_fingerprint,
    )


def validate_verify_candidate(
    candidate: object,
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
) -> ApplicationGateReportV1:
    try:
        report = ApplicationGateReportV1.model_validate(candidate)
        current = build_verify_basis(workspace, job_dir)
    except (ValidationError, VerifyStageError) as exc:
        raise VerifyStageError("The Verify candidate is invalid or stale.") from exc
    candidate_basis = VerifyBasisV1.model_validate(
        report.model_dump(
            mode="python",
            exclude={"generated_at", "input_fingerprint"},
        )
    )
    if (
        report.job_id != job_dir.name
        or report.input_fingerprint != input_fingerprint
        or candidate_basis != current
        or _basis_fingerprint(current) != input_fingerprint
    ):
        raise VerifyStageError("The Verify candidate does not match current package checks.")
    return report


def _basis_fingerprint(basis: VerifyBasisV1) -> str:
    return sha256_bytes(
        json.dumps(
            basis.model_dump(mode="json"),
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
    )
