from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

from canisend.bundle_models import ProjectionJournalV1
from canisend.bundle_projection import (
    BundleProjectionError,
    canonical_bundle_bytes,
    inspect_artifact_projection,
    load_artifact_bundle,
    project_artifact_bundle,
)
from canisend.job_coordination import JobCoordinationError, coordinate_job
from canisend.stage_store import (
    StageStoreError,
    atomic_write_bytes,
    atomic_write_json,
    read_json_object,
    sha256_bytes,
)
from canisend.stages.package_stage import (
    PackageStageError,
    build_legacy_package_bundle_candidate,
    legacy_package_input_fingerprint,
)


class LegacyCompatibilityError(RuntimeError):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class LegacyCompatibilityOutcome:
    active: bool
    cache_hit: bool
    bundle_path: Path | None = None
    journal: ProjectionJournalV1 | None = None
    report_invalidated: bool = False


def run_legacy_package_compatibility(
    workspace: Path,
    job_dir: Path,
) -> LegacyCompatibilityOutcome:
    """Project the old material inventory without claiming Package readiness."""

    root = workspace.expanduser().resolve()
    job = job_dir.expanduser().resolve()
    bundle_path = job / "package_bundle.json"
    try:
        with coordinate_job(job):
            fingerprint = legacy_package_input_fingerprint(job)
            if bundle_path.exists() or bundle_path.is_symlink():
                bundle = load_artifact_bundle(bundle_path)
                if bundle.mode == "guarded":
                    return LegacyCompatibilityOutcome(active=False, cache_hit=True)
                if bundle.stage != "package" or bundle.mode != "legacy_compatibility":
                    raise LegacyCompatibilityError(
                        "legacy.bundle_conflict",
                        "The existing Package bundle cannot be used for legacy compatibility.",
                    )
                if bundle.input_fingerprint == fingerprint:
                    inspection = inspect_artifact_projection(job, bundle)
                    if not inspection.current:
                        raise LegacyCompatibilityError(
                            "legacy.projection_repair_required",
                            "Legacy projections are missing or locally drifted and require explicit repair.",
                        )
                    journal = _load_package_journal(job)
                    invalidated = _invalidate_legacy_gate_report(job)
                    _write_compatibility_receipt(job, bundle, journal)
                    return LegacyCompatibilityOutcome(
                        active=True,
                        cache_hit=True,
                        bundle_path=bundle_path,
                        journal=journal,
                        report_invalidated=invalidated,
                    )

            bundle = build_legacy_package_bundle_candidate(
                root,
                job,
                input_fingerprint=fingerprint,
            )
            atomic_write_bytes(bundle_path, canonical_bundle_bytes(bundle))
            journal = project_artifact_bundle(job, bundle)
            invalidated = _invalidate_legacy_gate_report(job)
            _write_compatibility_receipt(job, bundle, journal)
            return LegacyCompatibilityOutcome(
                active=True,
                cache_hit=False,
                bundle_path=bundle_path,
                journal=journal,
                report_invalidated=invalidated,
            )
    except LegacyCompatibilityError:
        raise
    except JobCoordinationError as exc:
        raise LegacyCompatibilityError(exc.code, str(exc)) from exc
    except BundleProjectionError as exc:
        raise LegacyCompatibilityError(exc.code, str(exc)) from exc
    except (OSError, StageStoreError, PackageStageError, ValueError) as exc:
        raise LegacyCompatibilityError(
            "legacy.compatibility_failed",
            "Legacy compatibility materials could not be produced safely.",
        ) from exc


def _load_package_journal(job: Path) -> ProjectionJournalV1:
    try:
        return ProjectionJournalV1.model_validate(
            read_json_object(job / "workflow" / "projections" / "package.json")
        )
    except Exception as exc:
        raise LegacyCompatibilityError(
            "legacy.projection_repair_required",
            "The legacy Package projection journal is missing or invalid.",
        ) from exc


def _write_compatibility_receipt(
    job: Path,
    bundle: object,
    journal: ProjectionJournalV1,
) -> None:
    from canisend.bundle_models import ArtifactBundleV1

    validated = ArtifactBundleV1.model_validate(bundle)
    path = job / "workflow" / "compatibility" / "package.json"
    payload = {
        "schema_version": "1.0.0",
        "job_id": job.name,
        "mode": "legacy_compatibility",
        "input_fingerprint": validated.input_fingerprint,
        "bundle_sha256": sha256_bytes(canonical_bundle_bytes(validated)),
        "projection_bundle_sha256": journal.bundle_sha256,
        "completed_at": datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    }
    if path.is_file():
        existing = read_json_object(path)
        comparable = dict(existing)
        comparable["completed_at"] = payload["completed_at"]
        if comparable == payload:
            return
    atomic_write_json(path, payload)


def _invalidate_legacy_gate_report(job: Path) -> bool:
    path = job / "application_gate_report.json"
    if not path.exists() and not path.is_symlink():
        return False
    try:
        existing = read_json_object(path)
    except StageStoreError as exc:
        raise LegacyCompatibilityError(
            "legacy.gate_report_conflict",
            "The existing application gate report cannot be invalidated safely.",
        ) from exc
    if (
        existing.get("status") == "STALE"
        and existing.get("invalidation_reason")
        == "legacy compatibility application artifacts were regenerated"
    ):
        return False
    atomic_write_json(
        path,
        {
            "schema_version": "1.0.0",
            "status": "STALE",
            "invalidated_at": datetime.now(UTC)
            .replace(microsecond=0)
            .isoformat()
            .replace("+00:00", "Z"),
            "invalidation_reason": (
                "legacy compatibility application artifacts were regenerated"
            ),
            "input_hashes": {},
            "issues": [],
        },
    )
    return True
