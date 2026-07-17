from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Callable, Literal
from uuid import uuid4

from pydantic import ValidationError

from canisend.job_coordination import JobCoordinationError, coordinate_job
from canisend.jobs import JobMetadataError, load_job_metadata
from canisend.recovery_models import (
    STAGE5_MIGRATION_ID,
    MigrationChangeV1,
    MigrationPlanV1,
    MigrationReceiptV1,
    MigrationRollbackReceiptV1,
    MigrationSourceShape,
    RollbackEntryV1,
)
from canisend.stage_models import ArtifactFingerprint, WorkflowStateV1
from canisend.stage_runtime import (
    StageRuntimeError,
    load_workflow_state_view,
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
    write_immutable_bytes,
    write_immutable_json,
)


MIGRATION_ROOT = f"workflow/migrations/{STAGE5_MIGRATION_ID}"
MIGRATION_PLAN_PATH = f"{MIGRATION_ROOT}/plan.json"
MIGRATION_RECEIPT_PATH = f"{MIGRATION_ROOT}/receipt.json"
MIGRATION_STATE_BACKUP_PATH = f"{MIGRATION_ROOT}/backups/workflow-state.json"
MIGRATION_ROLLBACK_ROOT = f"{MIGRATION_ROOT}/rollbacks"
MAX_MIGRATION_METADATA_FILES = 10_000
MAX_MIGRATION_METADATA_BYTES = 100_000_000

MigrationFailureInjector = Callable[[str], None]
MigrationStatus = Literal["needed", "applied", "rolled_back", "blocked"]


class MigrationError(RuntimeError):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class MigrationInspection:
    job_id: str
    status: MigrationStatus
    source_shape: MigrationSourceShape | None
    observed_metadata: tuple[ArtifactFingerprint, ...] = ()
    planned_changes: tuple[MigrationChangeV1, ...] = ()
    legacy_markers: tuple[str, ...] = ()
    reason_codes: tuple[str, ...] = ()
    receipt_path: Path | None = None


@dataclass(frozen=True)
class MigrationApplyOutcome:
    inspection: MigrationInspection
    receipt: MigrationReceiptV1
    receipt_path: Path
    cache_hit: bool


@dataclass(frozen=True)
class MigrationRollbackOutcome:
    receipt: MigrationRollbackReceiptV1
    receipt_path: Path

    @property
    def conflicts(self) -> tuple[str, ...]:
        return tuple(
            entry.path for entry in self.receipt.entries if entry.outcome == "conflict"
        )


def inspect_migration(workspace: Path, job_dir: Path) -> MigrationInspection:
    """Inspect legacy runtime metadata without creating workflow files or locks."""

    _, job = _migration_paths(workspace, job_dir)
    receipt_path = resolve_job_relative_path(job, MIGRATION_RECEIPT_PATH)
    if receipt_path.is_file() and not receipt_path.is_symlink():
        receipt = _load_receipt(job)
        rolled_back = _has_complete_rollback(job, receipt)
        return MigrationInspection(
            job_id=job.name,
            status="rolled_back" if rolled_back else "applied",
            source_shape=receipt.source_shape,
            observed_metadata=receipt.observed_metadata,
            planned_changes=receipt.changes,
            legacy_markers=_legacy_markers(job),
            reason_codes=("migration.rolled_back",) if rolled_back else (),
            receipt_path=receipt_path,
        )

    try:
        observed = _observed_runtime_metadata(job)
        source_shape = _source_shape(job, observed)
        changes = _planned_state_changes(job)
    except MigrationError as exc:
        return MigrationInspection(
            job_id=job.name,
            status="blocked",
            source_shape=None,
            legacy_markers=_legacy_markers(job),
            reason_codes=(exc.code,),
        )
    return MigrationInspection(
        job_id=job.name,
        status="needed",
        source_shape=source_shape,
        observed_metadata=observed,
        planned_changes=changes,
        legacy_markers=_legacy_markers(job),
    )


def apply_migration(
    workspace: Path,
    job_dir: Path,
    *,
    failure_injector: MigrationFailureInjector | None = None,
) -> MigrationApplyOutcome:
    """Apply or resume the explicit Stage 5 runtime-metadata migration."""

    root, job = _migration_paths(workspace, job_dir)
    try:
        with coordinate_job(job):
            receipt_path = resolve_job_relative_path(job, MIGRATION_RECEIPT_PATH)
            if receipt_path.is_file() and not receipt_path.is_symlink():
                receipt = _load_receipt(job)
                if _has_complete_rollback(job, receipt):
                    raise MigrationError(
                        "migration.already_rolled_back",
                        "The Stage 5 migration was already rolled back; its evidence was preserved.",
                    )
                return MigrationApplyOutcome(
                    inspection=inspect_migration(root, job),
                    receipt=receipt,
                    receipt_path=receipt_path,
                    cache_hit=True,
                )

            plan_path = resolve_job_relative_path(job, MIGRATION_PLAN_PATH)
            if plan_path.is_file() and not plan_path.is_symlink():
                plan = _load_plan(job)
            else:
                inspection = inspect_migration(root, job)
                if inspection.status == "blocked" or inspection.source_shape is None:
                    raise MigrationError(
                        inspection.reason_codes[0]
                        if inspection.reason_codes
                        else "migration.blocked",
                        "The legacy runtime metadata cannot be migrated safely.",
                    )
                _write_backups(job, inspection.planned_changes)
                plan = MigrationPlanV1(
                    job_id=job.name,
                    source_shape=inspection.source_shape,
                    planned_at=_utc_now(),
                    observed_metadata=inspection.observed_metadata,
                    changes=inspection.planned_changes,
                )
                write_immutable_json(plan_path, plan.model_dump(mode="json"))
            _inject(failure_injector, "after_plan")

            for change in plan.changes:
                _apply_state_change(job, change)
                _inject(failure_injector, f"after_change:{change.path}")

            receipt = MigrationReceiptV1(
                job_id=job.name,
                source_shape=plan.source_shape,
                applied_at=_utc_now(),
                plan_sha256=sha256_file(plan_path),
                observed_metadata=plan.observed_metadata,
                changes=plan.changes,
            )
            write_immutable_json(receipt_path, receipt.model_dump(mode="json"))
            _inject(failure_injector, "after_receipt")
            return MigrationApplyOutcome(
                inspection=MigrationInspection(
                    job_id=job.name,
                    status="applied",
                    source_shape=receipt.source_shape,
                    observed_metadata=receipt.observed_metadata,
                    planned_changes=receipt.changes,
                    legacy_markers=_legacy_markers(job),
                    receipt_path=receipt_path,
                ),
                receipt=receipt,
                receipt_path=receipt_path,
                cache_hit=False,
            )
    except MigrationError:
        raise
    except JobCoordinationError as exc:
        raise MigrationError(exc.code, str(exc)) from exc
    except (OSError, StageStoreError, ValidationError, ValueError) as exc:
        raise MigrationError(
            "migration.store_failed",
            "The Stage 5 migration could not be stored safely.",
        ) from exc


def rollback_migration(
    workspace: Path,
    job_dir: Path,
    *,
    failure_injector: MigrationFailureInjector | None = None,
) -> MigrationRollbackOutcome:
    """Restore/remove only migration-owned metadata whose hashes still match."""

    _, job = _migration_paths(workspace, job_dir)
    try:
        with coordinate_job(job):
            migration = _load_receipt(job)
            entries: list[RollbackEntryV1] = []
            for change in migration.changes:
                entry = _rollback_change(job, change)
                entries.append(entry)
                _inject(failure_injector, f"after_rollback:{change.path}")
            entries.sort(key=lambda item: item.path)
            receipt = MigrationRollbackReceiptV1(
                rollback_id=f"rollback_{uuid4().hex}",
                job_id=job.name,
                completed_at=_utc_now(),
                status=(
                    "conflict"
                    if any(entry.outcome == "conflict" for entry in entries)
                    else "complete"
                ),
                migration_receipt_sha256=sha256_file(
                    resolve_job_relative_path(job, MIGRATION_RECEIPT_PATH)
                ),
                entries=tuple(entries),
            )
            receipt_path = resolve_job_relative_path(
                job,
                f"{MIGRATION_ROLLBACK_ROOT}/{receipt.rollback_id}.json",
            )
            write_immutable_json(receipt_path, receipt.model_dump(mode="json"))
            _inject(failure_injector, "after_rollback_receipt")
            return MigrationRollbackOutcome(receipt=receipt, receipt_path=receipt_path)
    except MigrationError:
        raise
    except JobCoordinationError as exc:
        raise MigrationError(exc.code, str(exc)) from exc
    except (OSError, StageStoreError, ValidationError, ValueError) as exc:
        raise MigrationError(
            "migration.rollback_failed",
            "The Stage 5 migration rollback could not be completed safely.",
        ) from exc


def _planned_state_changes(job: Path) -> tuple[MigrationChangeV1, ...]:
    state_path = resolve_job_relative_path(job, "workflow/state.json")
    existing = _safe_bytes_or_none(state_path)
    before_hash = sha256_bytes(existing) if existing is not None else None
    target = _target_state_bytes(job, existing)
    after_hash = sha256_bytes(target)
    if existing == target:
        return ()
    if existing is None:
        return (
            MigrationChangeV1(
                path="workflow/state.json",
                action="created",
                after_sha256=after_hash,
            ),
        )
    return (
        MigrationChangeV1(
            path="workflow/state.json",
            action="replaced",
            before_sha256=before_hash,
            after_sha256=after_hash,
            backup_path=MIGRATION_STATE_BACKUP_PATH,
            backup_sha256=before_hash,
        ),
    )


def _target_state_bytes(job: Path, existing: bytes | None) -> bytes:
    if existing is None:
        reconstructed, has_evidence = reconstruct_workflow_state(job)
        state = reconstructed if has_evidence else _baseline_state(job)
        return canonical_json_bytes(workflow_state_payload(state))
    try:
        WorkflowStateV1.model_validate_json(existing)
        state = load_workflow_state_view(job)
    except (StageRuntimeError, ValidationError, ValueError):
        state, has_evidence = reconstruct_workflow_state(job)
        if not has_evidence:
            raise MigrationError(
                "migration.invalid_state_without_evidence",
                "Invalid workflow state has no immutable run evidence for safe reconstruction.",
            )
    return canonical_json_bytes(workflow_state_payload(state))


def _baseline_state(job: Path) -> WorkflowStateV1:
    try:
        metadata = load_job_metadata(job)
        created_at = metadata["created_at"]
    except (JobMetadataError, KeyError, TypeError) as exc:
        raise MigrationError(
            "migration.invalid_job_metadata",
            "The job metadata cannot anchor a migration baseline safely.",
        ) from exc
    try:
        return WorkflowStateV1(
            schema_version="1.1.0",
            job_id=job.name,
            revision=0,
            created_at=created_at,
            updated_at=created_at,
            stages=(),
        )
    except ValidationError as exc:
        raise MigrationError(
            "migration.invalid_job_metadata",
            "The job metadata cannot anchor a migration baseline safely.",
        ) from exc


def _write_backups(job: Path, changes: tuple[MigrationChangeV1, ...]) -> None:
    for change in changes:
        if change.action != "replaced" or change.backup_path is None:
            continue
        current = _safe_bytes_or_none(resolve_job_relative_path(job, change.path))
        if current is None or sha256_bytes(current) != change.before_sha256:
            raise MigrationError(
                "migration.input_changed",
                "Runtime metadata changed before its migration backup was captured.",
            )
        write_immutable_bytes(
            resolve_job_relative_path(job, change.backup_path),
            current,
        )


def _apply_state_change(job: Path, change: MigrationChangeV1) -> None:
    if change.path != "workflow/state.json":
        raise MigrationError(
            "migration.unsafe_scope",
            "The migration plan exceeds the Stage 5 runtime metadata scope.",
        )
    target = resolve_job_relative_path(job, change.path)
    existing = _safe_bytes_or_none(target)
    existing_hash = sha256_bytes(existing) if existing is not None else None
    if existing_hash == change.after_sha256:
        return
    expected_before = change.before_sha256 if change.action == "replaced" else None
    if existing_hash != expected_before:
        raise MigrationError(
            "migration.input_changed",
            "Runtime metadata changed after the migration plan was recorded.",
        )
    desired = _target_state_bytes(job, existing)
    if sha256_bytes(desired) != change.after_sha256:
        raise MigrationError(
            "migration.plan_drift",
            "The migration target no longer matches its immutable plan.",
        )
    atomic_write_bytes(target, desired)


def _rollback_change(job: Path, change: MigrationChangeV1) -> RollbackEntryV1:
    target = resolve_job_relative_path(job, change.path)
    current = _safe_bytes_or_none(target)
    current_hash = sha256_bytes(current) if current is not None else None
    if change.action == "created":
        if current_hash == change.after_sha256:
            try:
                target.unlink()
            except OSError as exc:
                raise MigrationError(
                    "migration.rollback_failed",
                    "Migration-owned metadata could not be removed safely.",
                ) from exc
            return RollbackEntryV1(
                path=change.path,
                outcome="removed",
                expected_after_sha256=change.after_sha256,
                observed_sha256=current_hash,
            )
        if current is None:
            return RollbackEntryV1(
                path=change.path,
                outcome="already_rolled_back",
                expected_after_sha256=change.after_sha256,
            )
        return RollbackEntryV1(
            path=change.path,
            outcome="conflict",
            expected_after_sha256=change.after_sha256,
            observed_sha256=current_hash,
        )

    if change.backup_path is None or change.before_sha256 is None:
        raise MigrationError(
            "migration.invalid_receipt",
            "The migration replacement receipt has no exact backup.",
        )
    if current_hash == change.before_sha256:
        return RollbackEntryV1(
            path=change.path,
            outcome="already_rolled_back",
            expected_after_sha256=change.after_sha256,
            observed_sha256=current_hash,
            restored_sha256=change.before_sha256,
        )
    if current_hash != change.after_sha256:
        return RollbackEntryV1(
            path=change.path,
            outcome="conflict",
            expected_after_sha256=change.after_sha256,
            observed_sha256=current_hash,
        )
    backup = _safe_bytes_or_none(resolve_job_relative_path(job, change.backup_path))
    if backup is None or sha256_bytes(backup) != change.backup_sha256:
        raise MigrationError(
            "migration.invalid_backup",
            "The migration backup is missing or does not match its receipt.",
        )
    atomic_write_bytes(target, backup)
    return RollbackEntryV1(
        path=change.path,
        outcome="restored",
        expected_after_sha256=change.after_sha256,
        observed_sha256=current_hash,
        restored_sha256=change.before_sha256,
    )


def _observed_runtime_metadata(job: Path) -> tuple[ArtifactFingerprint, ...]:
    workflow = job / "workflow"
    if not workflow.exists():
        return ()
    if workflow.is_symlink() or not workflow.is_dir():
        raise MigrationError(
            "migration.unsafe_workflow",
            "The legacy workflow path is not one safe directory.",
        )
    observed: list[ArtifactFingerprint] = []
    try:
        candidates = sorted(path for path in workflow.rglob("*.json"))
    except OSError as exc:
        raise MigrationError(
            "migration.metadata_unreadable",
            "Legacy runtime metadata could not be inventoried safely.",
        ) from exc
    for path in candidates:
        relative = path.relative_to(job).as_posix()
        if relative.startswith(f"{MIGRATION_ROOT}/") or relative.startswith(
            "workflow/repairs/"
        ):
            continue
        if "/candidates/" in relative or "/inputs/" in relative:
            # Candidate and prepared-input JSON may contain private bodies.
            # Migration inventories only body-free runtime control metadata.
            continue
        data = _safe_bytes_or_none(path)
        if data is None:
            raise MigrationError(
                "migration.metadata_unreadable",
                "Legacy runtime metadata contains an unsafe file.",
            )
        observed.append(
            ArtifactFingerprint(
                path=relative,
                sha256=sha256_bytes(data),
                size_bytes=len(data),
            )
        )
        if len(observed) > MAX_MIGRATION_METADATA_FILES or sum(
            item.size_bytes or 0 for item in observed
        ) > MAX_MIGRATION_METADATA_BYTES:
            raise MigrationError(
                "migration.metadata_too_large",
                "Legacy runtime metadata exceeds the migration inspection limit.",
            )
    return tuple(observed)


def _source_shape(
    job: Path,
    observed: tuple[ArtifactFingerprint, ...],
) -> MigrationSourceShape:
    if not observed and not (job / "workflow" / "runs").exists():
        return "pre_workflow"
    for artifact in observed:
        try:
            payload = read_json_object(resolve_job_relative_path(job, artifact.path))
        except StageStoreError:
            continue
        if payload.get("schema_version") == "1.0.0":
            return "prior_schema"
    return "current_unmigrated"


def _load_plan(job: Path) -> MigrationPlanV1:
    try:
        plan = MigrationPlanV1.model_validate(
            read_json_object(resolve_job_relative_path(job, MIGRATION_PLAN_PATH))
        )
    except (StageStoreError, ValidationError) as exc:
        raise MigrationError(
            "migration.invalid_plan",
            "The existing migration plan is invalid or unsafe.",
        ) from exc
    if plan.job_id != job.name:
        raise MigrationError(
            "migration.invalid_plan",
            "The existing migration plan belongs to another job.",
        )
    return plan


def _load_receipt(job: Path) -> MigrationReceiptV1:
    try:
        receipt = MigrationReceiptV1.model_validate(
            read_json_object(resolve_job_relative_path(job, MIGRATION_RECEIPT_PATH))
        )
    except (StageStoreError, ValidationError) as exc:
        raise MigrationError(
            "migration.invalid_receipt",
            "The existing migration receipt is invalid or unsafe.",
        ) from exc
    if receipt.job_id != job.name:
        raise MigrationError(
            "migration.invalid_receipt",
            "The existing migration receipt belongs to another job.",
        )
    plan_path = resolve_job_relative_path(job, MIGRATION_PLAN_PATH)
    try:
        plan_hash = sha256_file(plan_path)
        plan = _load_plan(job)
    except (MigrationError, StageStoreError) as exc:
        raise MigrationError(
            "migration.invalid_receipt",
            "The migration receipt has no matching immutable plan.",
        ) from exc
    if (
        receipt.plan_sha256 != plan_hash
        or receipt.source_shape != plan.source_shape
        or receipt.observed_metadata != plan.observed_metadata
        or receipt.changes != plan.changes
    ):
        raise MigrationError(
            "migration.invalid_receipt",
            "The migration receipt does not match its immutable plan.",
        )
    return receipt


def _has_complete_rollback(job: Path, migration: MigrationReceiptV1) -> bool:
    root = job / MIGRATION_ROLLBACK_ROOT
    if not root.is_dir() or root.is_symlink():
        return False
    try:
        paths = sorted(root.glob("rollback_*.json"))
    except OSError:
        return False
    migration_hash = sha256_file(resolve_job_relative_path(job, MIGRATION_RECEIPT_PATH))
    for path in reversed(paths):
        try:
            receipt = MigrationRollbackReceiptV1.model_validate(read_json_object(path))
        except (StageStoreError, ValidationError):
            continue
        if (
            receipt.job_id == migration.job_id
            and receipt.migration_receipt_sha256 == migration_hash
            and receipt.status == "complete"
            and _rollback_matches_migration(receipt, migration)
        ):
            return True
    return False


def _rollback_matches_migration(
    rollback: MigrationRollbackReceiptV1,
    migration: MigrationReceiptV1,
) -> bool:
    expected = {change.path: change for change in migration.changes}
    if {entry.path for entry in rollback.entries} != set(expected):
        return False
    return all(
        entry.expected_after_sha256 == expected[entry.path].after_sha256
        for entry in rollback.entries
    )


def _legacy_markers(job: Path) -> tuple[str, ...]:
    markers = []
    if (job / "typst" / ".canisend-generated.json").is_file():
        markers.append("typst/.canisend-generated.json")
    return tuple(markers)


def _safe_bytes_or_none(path: Path) -> bytes | None:
    if not path.exists() and not path.is_symlink():
        return None
    try:
        metadata = path.lstat()
        if path.is_symlink() or not path.is_file() or metadata.st_nlink != 1:
            raise MigrationError(
                "migration.unsafe_metadata",
                "Runtime metadata is not one unaliased regular file.",
            )
        if metadata.st_size > MAX_MIGRATION_METADATA_BYTES:
            raise MigrationError(
                "migration.metadata_too_large",
                "Runtime metadata exceeds the migration size limit.",
            )
        return path.read_bytes()
    except MigrationError:
        raise
    except OSError as exc:
        raise MigrationError(
            "migration.metadata_unreadable",
            "Runtime metadata could not be read safely.",
        ) from exc


def _migration_paths(workspace: Path, job_dir: Path) -> tuple[Path, Path]:
    root = Path(workspace).expanduser().resolve()
    raw_job = Path(job_dir).expanduser()
    candidate = raw_job if raw_job.is_absolute() else root / raw_job
    if candidate.is_symlink():
        raise MigrationError(
            "migration.unsafe_job",
            "Migration requires an unaliased job directory.",
        )
    job = candidate.resolve()
    try:
        job.relative_to(root)
    except ValueError as exc:
        raise MigrationError(
            "migration.job_outside_workspace",
            "Migration requires a job inside the selected workspace.",
        ) from exc
    if not job.is_dir() or job.is_symlink():
        raise MigrationError("job.not_found", "The requested job directory does not exist.")
    return root, job


def _utc_now() -> datetime:
    return datetime.now(UTC).replace(microsecond=0)


def _inject(injector: MigrationFailureInjector | None, point: str) -> None:
    if injector is not None:
        injector(point)
