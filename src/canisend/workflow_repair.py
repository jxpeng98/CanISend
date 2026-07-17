from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Literal
from uuid import uuid4

from pydantic import ValidationError

from canisend.bundle_models import ArtifactBundleV1
from canisend.bundle_projection import (
    BundleProjectionError,
    ProjectionInspection,
    inspect_artifact_projection,
    load_artifact_bundle,
    repair_artifact_projection,
)
from canisend.job_coordination import JobCoordinationError, coordinate_job
from canisend.recovery_models import RepairEntryV1, RepairReceiptV1
from canisend.stage_models import WorkflowStateV1
from canisend.stage_runtime import (
    StageRuntimeError,
    reconstruct_workflow_state,
    workflow_state_payload,
)
from canisend.stage_store import (
    StageStoreError,
    atomic_write_bytes,
    canonical_json_bytes,
    read_json_object,
    resolve_job_relative_path,
    sha256_bytes,
    sha256_file,
    write_immutable_json,
)


REPAIR_ROOT = "workflow/repairs"
MAX_REPAIR_METADATA_BYTES = 100_000_000
RepairStatus = Literal["current", "repairable", "blocked"]


class WorkflowRepairError(RuntimeError):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class RepairInspection:
    job_id: str
    kind: Literal["projection", "state"]
    status: RepairStatus
    stage: Literal["package", "render"] | None = None
    missing: tuple[str, ...] = ()
    drifted: tuple[str, ...] = ()
    reason_codes: tuple[str, ...] = ()


@dataclass(frozen=True)
class WorkflowRepairOutcome:
    inspection: RepairInspection
    receipt: RepairReceiptV1
    receipt_path: Path
    cache_hit: bool


def inspect_projection_repair(
    workspace: Path,
    job_dir: Path,
    *,
    stage: Literal["package", "render"],
) -> RepairInspection:
    """Inspect one bundle projection without writing or acquiring a mutation lock."""

    _, job = _repair_paths(workspace, job_dir)
    try:
        bundle = _load_stage_bundle(job, stage)
        inspection = inspect_artifact_projection(job, bundle)
    except BundleProjectionError as exc:
        return RepairInspection(
            job_id=job.name,
            kind="projection",
            stage=stage,
            status="blocked",
            reason_codes=(exc.code,),
        )
    return _projection_inspection(job, stage, inspection)


def repair_projection(
    workspace: Path,
    job_dir: Path,
    *,
    stage: Literal["package", "render"],
) -> WorkflowRepairOutcome:
    """Explicitly replay a validated bundle into its bounded derived projection."""

    _, job = _repair_paths(workspace, job_dir)
    try:
        with coordinate_job(job):
            bundle_path = resolve_job_relative_path(job, f"{stage}_bundle.json")
            bundle = _load_stage_bundle(job, stage)
            before_hashes = _projection_target_hashes(job, bundle)
            result = repair_artifact_projection(job, bundle)
            after = inspect_artifact_projection(job, bundle)
            if not after.current:
                raise WorkflowRepairError(
                    "repair.projection_incomplete",
                    "The explicit projection repair did not converge to current outputs.",
                )
            entries = tuple(
                sorted(
                    (
                        RepairEntryV1(
                            path=entry.target_path,
                            outcome="unchanged" if result.cache_hit else entry.outcome,
                            before_sha256=before_hashes.get(entry.target_path),
                            after_sha256=entry.projected_sha256,
                        )
                        for entry in result.journal.entries
                    ),
                    key=lambda entry: entry.path,
                )
            )
            receipt = RepairReceiptV1(
                repair_id=f"repair_{uuid4().hex}",
                job_id=job.name,
                kind="projection",
                stage=stage,
                completed_at=_utc_now(),
                source_sha256=sha256_file(bundle_path),
                entries=entries,
            )
            receipt_path = _write_repair_receipt(job, receipt)
            return WorkflowRepairOutcome(
                inspection=_projection_inspection(job, stage, result.before),
                receipt=receipt,
                receipt_path=receipt_path,
                cache_hit=result.cache_hit,
            )
    except WorkflowRepairError:
        raise
    except JobCoordinationError as exc:
        raise WorkflowRepairError(exc.code, str(exc)) from exc
    except BundleProjectionError as exc:
        raise WorkflowRepairError(exc.code, str(exc)) from exc
    except (OSError, StageStoreError, ValidationError, ValueError) as exc:
        raise WorkflowRepairError(
            "repair.store_failed",
            "The explicit projection repair could not be stored safely.",
        ) from exc


def inspect_state_repair(workspace: Path, job_dir: Path) -> RepairInspection:
    """Compare mutable state.json with immutable run evidence without writing it."""

    _, job = _repair_paths(workspace, job_dir)
    try:
        reconstructed, has_evidence = reconstruct_workflow_state(job)
        current = _safe_bytes_or_none(job / "workflow" / "state.json")
        if not has_evidence:
            if current is None:
                return RepairInspection(
                    job_id=job.name,
                    kind="state",
                    status="blocked",
                    reason_codes=("repair.migration_required",),
                )
            try:
                WorkflowStateV1.model_validate_json(current)
            except (ValidationError, ValueError):
                return RepairInspection(
                    job_id=job.name,
                    kind="state",
                    status="blocked",
                    reason_codes=("repair.no_immutable_evidence",),
                )
            return RepairInspection(job_id=job.name, kind="state", status="current")
        desired = canonical_json_bytes(workflow_state_payload(reconstructed))
    except WorkflowRepairError as exc:
        return RepairInspection(
            job_id=job.name,
            kind="state",
            status="blocked",
            reason_codes=(exc.code,),
        )
    if current == desired:
        return RepairInspection(job_id=job.name, kind="state", status="current")
    return RepairInspection(
        job_id=job.name,
        kind="state",
        status="repairable",
        missing=("workflow/state.json",) if current is None else (),
        drifted=("workflow/state.json",) if current is not None else (),
        reason_codes=("state.missing",) if current is None else ("state.drifted",),
    )


def repair_state(workspace: Path, job_dir: Path) -> WorkflowRepairOutcome:
    """Explicitly rebuild only state.json from immutable run/task evidence."""

    root, job = _repair_paths(workspace, job_dir)
    try:
        with coordinate_job(job):
            inspection = inspect_state_repair(root, job)
            if inspection.status == "blocked":
                raise WorkflowRepairError(
                    inspection.reason_codes[0]
                    if inspection.reason_codes
                    else "repair.state_blocked",
                    "Workflow state cannot be repaired without immutable run evidence.",
                )
            path = resolve_job_relative_path(job, "workflow/state.json")
            before = _safe_bytes_or_none(path)
            if inspection.status == "current":
                if before is None:  # pragma: no cover - guarded by inspection
                    raise WorkflowRepairError(
                        "repair.state_unavailable",
                        "Current workflow state is unexpectedly unavailable.",
                    )
                after = before
                outcome: Literal["created", "replaced", "unchanged"] = "unchanged"
                cache_hit = True
            else:
                reconstructed, has_evidence = reconstruct_workflow_state(job)
                if not has_evidence:
                    raise WorkflowRepairError(
                        "repair.no_immutable_evidence",
                        "Workflow state cannot be repaired without immutable run evidence.",
                    )
                after = canonical_json_bytes(workflow_state_payload(reconstructed))
                atomic_write_bytes(path, after)
                outcome = "created" if before is None else "replaced"
                cache_hit = False
            after_hash = sha256_bytes(after)
            entry = RepairEntryV1(
                path="workflow/state.json",
                outcome=outcome,
                before_sha256=sha256_bytes(before) if before is not None else None,
                after_sha256=after_hash,
            )
            receipt = RepairReceiptV1(
                repair_id=f"repair_{uuid4().hex}",
                job_id=job.name,
                kind="state",
                completed_at=_utc_now(),
                source_sha256=after_hash,
                entries=(entry,),
            )
            receipt_path = _write_repair_receipt(job, receipt)
            return WorkflowRepairOutcome(
                inspection=inspection,
                receipt=receipt,
                receipt_path=receipt_path,
                cache_hit=cache_hit,
            )
    except WorkflowRepairError:
        raise
    except JobCoordinationError as exc:
        raise WorkflowRepairError(exc.code, str(exc)) from exc
    except (OSError, StageRuntimeError, StageStoreError, ValidationError, ValueError) as exc:
        raise WorkflowRepairError(
            "repair.store_failed",
            "Workflow state repair could not be completed safely.",
        ) from exc


def _load_stage_bundle(
    job: Path,
    stage: Literal["package", "render"],
) -> ArtifactBundleV1:
    bundle = load_artifact_bundle(job / f"{stage}_bundle.json")
    if bundle.stage != stage:
        raise BundleProjectionError(
            "projection.invalid_bundle",
            "The selected bundle does not match the requested projection stage.",
        )
    return bundle


def _projection_target_hashes(
    job: Path,
    bundle: ArtifactBundleV1,
) -> dict[str, str | None]:
    targets = {entry.path for entry in bundle.entries}
    journal_path = job / "workflow" / "projections" / f"{bundle.stage}.json"
    try:
        payload = read_json_object(journal_path)
        raw_entries = payload.get("entries", [])
        if isinstance(raw_entries, list):
            targets.update(
                entry["target_path"]
                for entry in raw_entries
                if isinstance(entry, dict) and isinstance(entry.get("target_path"), str)
            )
    except StageStoreError:
        pass
    for entry in bundle.entries:
        if entry.path.endswith(".typ"):
            primary = Path(entry.path)
            targets.add(
                primary.with_name(primary.stem + ".generated.typ").as_posix()
            )
    return {
        target: _safe_hash_or_none(resolve_job_relative_path(job, target))
        for target in sorted(targets)
    }


def _projection_inspection(
    job: Path,
    stage: Literal["package", "render"],
    inspection: ProjectionInspection,
) -> RepairInspection:
    return RepairInspection(
        job_id=job.name,
        kind="projection",
        stage=stage,
        status="current" if inspection.current else "repairable",
        missing=inspection.missing,
        drifted=inspection.drifted,
        reason_codes=()
        if inspection.current
        else tuple(
            [f"projection.missing:{path}" for path in inspection.missing]
            + [f"projection.drifted:{path}" for path in inspection.drifted]
        ),
    )


def _write_repair_receipt(job: Path, receipt: RepairReceiptV1) -> Path:
    path = resolve_job_relative_path(job, f"{REPAIR_ROOT}/{receipt.repair_id}.json")
    write_immutable_json(path, receipt.model_dump(mode="json"))
    return path


def _safe_hash_or_none(path: Path) -> str | None:
    data = _safe_bytes_or_none(path)
    return sha256_bytes(data) if data is not None else None


def _safe_bytes_or_none(path: Path) -> bytes | None:
    if not path.exists() and not path.is_symlink():
        return None
    try:
        metadata = path.lstat()
        if path.is_symlink() or not path.is_file() or metadata.st_nlink != 1:
            raise WorkflowRepairError(
                "repair.unsafe_metadata",
                "Repair metadata is not one unaliased regular file.",
            )
        if metadata.st_size > MAX_REPAIR_METADATA_BYTES:
            raise WorkflowRepairError(
                "repair.metadata_too_large",
                "Repair metadata exceeds the bounded size limit.",
            )
        return path.read_bytes()
    except WorkflowRepairError:
        raise
    except OSError as exc:
        raise WorkflowRepairError(
            "repair.metadata_unreadable",
            "Repair metadata could not be read safely.",
        ) from exc


def _repair_paths(workspace: Path, job_dir: Path) -> tuple[Path, Path]:
    root = Path(workspace).expanduser().resolve()
    raw_job = Path(job_dir).expanduser()
    candidate = raw_job if raw_job.is_absolute() else root / raw_job
    if candidate.is_symlink():
        raise WorkflowRepairError(
            "repair.unsafe_job",
            "Repair requires an unaliased job directory.",
        )
    job = candidate.resolve()
    try:
        job.relative_to(root)
    except ValueError as exc:
        raise WorkflowRepairError(
            "repair.job_outside_workspace",
            "Repair requires a job inside the selected workspace.",
        ) from exc
    if not job.is_dir() or job.is_symlink():
        raise WorkflowRepairError("job.not_found", "The requested job directory does not exist.")
    return root, job


def _utc_now() -> datetime:
    return datetime.now(UTC).replace(microsecond=0)
