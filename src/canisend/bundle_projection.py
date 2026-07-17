from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
import stat
from typing import Callable

from pydantic import ValidationError

from canisend.bundle_models import (
    ArtifactBundleV1,
    BundleEntryV1,
    ProjectionEntryV1,
    ProjectionJournalV1,
)
from canisend.job_coordination import JobCoordinationError, coordinate_job
from canisend.stage_store import (
    StageStoreError,
    atomic_write_bytes,
    atomic_write_json,
    read_json_object,
    resolve_job_relative_path,
    sha256_bytes,
)


ProjectionFailureInjector = Callable[[str], None]
MAX_PROJECTION_FILE_BYTES = 100_000_000
PROTECTED_TYPST_PATHS = frozenset(
    {
        "typst/cover_letter.typ",
        "typst/application_package.typ",
        "typst/research_statement.typ",
    }
)


class BundleProjectionError(RuntimeError):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class ProjectionInspection:
    current: bool
    missing: tuple[str, ...] = ()
    drifted: tuple[str, ...] = ()


def canonical_bundle_bytes(bundle: ArtifactBundleV1) -> bytes:
    return (
        json.dumps(
            bundle.model_dump(mode="json"),
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
        )
        + "\n"
    ).encode("utf-8")


def load_artifact_bundle(path: Path) -> ArtifactBundleV1:
    try:
        return ArtifactBundleV1.model_validate(read_json_object(path))
    except (StageStoreError, ValidationError, ValueError) as exc:
        raise BundleProjectionError(
            "projection.invalid_bundle",
            "The authoritative artifact bundle is invalid or unsafe.",
        ) from exc


def project_artifact_bundle(
    job_dir: Path,
    bundle: ArtifactBundleV1,
    *,
    failure_injector: ProjectionFailureInjector | None = None,
) -> ProjectionJournalV1:
    job = Path(job_dir).expanduser().resolve()
    if not job.is_dir() or bundle.job_id != job.name:
        raise BundleProjectionError(
            "projection.job_mismatch",
            "The artifact bundle does not belong to the selected job.",
        )
    try:
        with coordinate_job(job):
            return _project_locked(job, bundle, failure_injector=failure_injector)
    except JobCoordinationError as exc:
        raise BundleProjectionError(exc.code, str(exc)) from exc


def inspect_artifact_projection(
    job_dir: Path,
    bundle: ArtifactBundleV1,
) -> ProjectionInspection:
    job = Path(job_dir).expanduser().resolve()
    missing: list[str] = []
    drifted: list[str] = []
    for entry in bundle.entries:
        target = _entry_target(job, entry.path)
        actual = _safe_hash_or_none(target)
        if actual == entry.sha256:
            continue
        if entry.path in PROTECTED_TYPST_PATHS:
            candidate = _generated_candidate(target)
            if _safe_hash_or_none(candidate) == entry.sha256:
                continue
        if actual is None:
            missing.append(entry.path)
        else:
            drifted.append(entry.path)
    return ProjectionInspection(
        current=not missing and not drifted,
        missing=tuple(missing),
        drifted=tuple(drifted),
    )


def _project_locked(
    job: Path,
    bundle: ArtifactBundleV1,
    *,
    failure_injector: ProjectionFailureInjector | None,
) -> ProjectionJournalV1:
    previous = _load_projection_journal(job, bundle.stage)
    previous_by_source = (
        {entry.source_path: entry for entry in previous.entries}
        if previous is not None
        else {}
    )
    projected: list[ProjectionEntryV1] = []
    for entry in bundle.entries:
        receipt = (
            _project_protected_typst(
                job,
                entry,
                previous_by_source.get(entry.path),
            )
            if entry.path in PROTECTED_TYPST_PATHS
            else _project_generated_entry(job, entry)
        )
        projected.append(receipt)
        _inject(failure_injector, f"after_projection:{entry.path}")

    bundle_sha256 = sha256_bytes(canonical_bundle_bytes(bundle))
    journal = ProjectionJournalV1(
        job_id=job.name,
        stage=bundle.stage,
        bundle_sha256=bundle_sha256,
        entries=tuple(projected),
    )
    _inject(failure_injector, "before_projection_journal")
    journal_path = resolve_job_relative_path(
        job,
        f"workflow/projections/{bundle.stage}.json",
    )
    try:
        atomic_write_json(journal_path, journal.model_dump(mode="json"))
    except StageStoreError as exc:
        raise BundleProjectionError(
            "projection.store_failed",
            "The projection journal could not be stored safely.",
        ) from exc
    _inject(failure_injector, "after_projection_journal")
    return journal


def _project_generated_entry(job: Path, entry: BundleEntryV1) -> ProjectionEntryV1:
    target = _entry_target(job, entry.path)
    before = _safe_hash_or_none(target)
    if before == entry.sha256:
        outcome = "unchanged"
    else:
        _atomic_projection_write(target, entry.decoded_bytes())
        outcome = "created" if before is None else "replaced"
    return ProjectionEntryV1(
        source_path=entry.path,
        target_path=entry.path,
        source_sha256=entry.sha256,
        projected_sha256=entry.sha256,
        outcome=outcome,
    )


def _project_protected_typst(
    job: Path,
    entry: BundleEntryV1,
    previous: ProjectionEntryV1 | None,
) -> ProjectionEntryV1:
    primary = _entry_target(job, entry.path)
    primary_hash = _safe_hash_or_none(primary)
    if primary_hash == entry.sha256:
        return _projection_receipt(entry, entry.path, "unchanged")
    previous_owned_primary = (
        previous is not None
        and previous.target_path == entry.path
        and primary_hash == previous.projected_sha256
    )
    if primary_hash is None or previous_owned_primary:
        _atomic_projection_write(primary, entry.decoded_bytes())
        return _projection_receipt(
            entry,
            entry.path,
            "created" if primary_hash is None else "replaced",
        )

    candidate = _generated_candidate(primary)
    candidate_relative = candidate.relative_to(job).as_posix()
    candidate_hash = _safe_hash_or_none(candidate)
    if candidate_hash == entry.sha256:
        return _projection_receipt(entry, candidate_relative, "unchanged")
    previous_owned_candidate = (
        previous is not None
        and previous.target_path == candidate_relative
        and candidate_hash == previous.projected_sha256
    )
    if candidate_hash is not None and not previous_owned_candidate:
        raise BundleProjectionError(
            "projection.output_conflict",
            "A generated Typst candidate contains unrecognized local edits.",
        )
    _atomic_projection_write(candidate, entry.decoded_bytes())
    return _projection_receipt(
        entry,
        candidate_relative,
        "candidate_created" if candidate_hash is None else "candidate_replaced",
    )


def _projection_receipt(
    entry: BundleEntryV1,
    target_path: str,
    outcome: str,
) -> ProjectionEntryV1:
    return ProjectionEntryV1(
        source_path=entry.path,
        target_path=target_path,
        source_sha256=entry.sha256,
        projected_sha256=entry.sha256,
        outcome=outcome,  # type: ignore[arg-type]
    )


def _load_projection_journal(
    job: Path,
    stage: str,
) -> ProjectionJournalV1 | None:
    path = resolve_job_relative_path(job, f"workflow/projections/{stage}.json")
    if not path.exists():
        return None
    try:
        journal = ProjectionJournalV1.model_validate(read_json_object(path))
    except (StageStoreError, ValidationError) as exc:
        raise BundleProjectionError(
            "projection.invalid_journal",
            "The existing projection journal is invalid or unsafe.",
        ) from exc
    if journal.job_id != job.name or journal.stage != stage:
        raise BundleProjectionError(
            "projection.invalid_journal",
            "The existing projection journal does not belong to this job stage.",
        )
    return journal


def _entry_target(job: Path, relative_path: str) -> Path:
    try:
        return resolve_job_relative_path(job, relative_path)
    except StageStoreError as exc:
        raise BundleProjectionError(
            "projection.unsafe_path",
            "A bundle projection path is outside the selected job.",
        ) from exc


def _generated_candidate(primary: Path) -> Path:
    return primary.with_name(f"{primary.stem}.generated{primary.suffix}")


def _safe_hash_or_none(path: Path) -> str | None:
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return None
    except OSError as exc:
        raise BundleProjectionError(
            "projection.output_unreadable",
            "A projection target could not be inspected safely.",
        ) from exc
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_nlink != 1
        or metadata.st_size > MAX_PROJECTION_FILE_BYTES
    ):
        raise BundleProjectionError(
            "projection.output_unreadable",
            "A projection target is not one bounded unaliased regular file.",
        )
    try:
        data = path.read_bytes()
    except OSError as exc:
        raise BundleProjectionError(
            "projection.output_unreadable",
            "A projection target could not be read safely.",
        ) from exc
    return sha256_bytes(data)


def _atomic_projection_write(path: Path, data: bytes) -> None:
    try:
        atomic_write_bytes(path, data)
    except StageStoreError as exc:
        raise BundleProjectionError(
            "projection.store_failed",
            "A bundle projection could not be written safely.",
        ) from exc


def _inject(injector: ProjectionFailureInjector | None, point: str) -> None:
    if injector is not None:
        injector(point)
