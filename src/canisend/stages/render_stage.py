from __future__ import annotations

import json
from pathlib import Path
import shutil
import stat
import subprocess
import tempfile

from pydantic import ValidationError

from canisend.bundle_models import ArtifactBundleV1, BundleEntryV1
from canisend.stage_models import ArtifactFingerprint
from canisend.stage_store import (
    StageStoreError,
    read_json_object,
    resolve_job_relative_path,
    sha256_bytes,
    sha256_file,
)
from canisend.stages.verify_stage import ApplicationGateReportV1


RENDER_BUNDLE_OUTPUT_PATH = "render_bundle.json"
RENDER_OUTPUT_SCHEMA = "canisend.artifact-bundle/v1"
RENDER_SOURCE_PATHS = (
    "typst/application_package.typ",
    "typst/cover_letter.typ",
    "typst/research_statement.typ",
)


class RenderStageError(ValueError):
    """A body-free Render stage execution or validation failure."""


def render_input_artifacts(job_dir: Path) -> tuple[ArtifactFingerprint, ...]:
    paths = ["application_gate_report.json"]
    paths.extend(
        path
        for path in RENDER_SOURCE_PATHS
        if resolve_job_relative_path(job_dir, path).is_file()
    )
    artifacts: list[ArtifactFingerprint] = []
    for relative_path in paths:
        path = resolve_job_relative_path(job_dir, relative_path)
        try:
            metadata = path.lstat()
            if (
                not stat.S_ISREG(metadata.st_mode)
                or metadata.st_nlink != 1
                or path.is_symlink()
            ):
                raise RenderStageError("A Render input is missing or unsafe.")
            artifacts.append(
                ArtifactFingerprint(
                    path=relative_path,
                    sha256=sha256_file(path),
                    size_bytes=metadata.st_size,
                )
            )
        except (OSError, StageStoreError) as exc:
            raise RenderStageError("A Render input is missing or unsafe.") from exc
    if len(artifacts) < 2:
        raise RenderStageError("Render requires a gate report and at least one Typst source.")
    return tuple(artifacts)


def render_input_fingerprint(job_dir: Path) -> str:
    payload = [
        item.model_dump(mode="json")
        for item in render_input_artifacts(job_dir)
    ]
    return sha256_bytes(
        json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
    )


def render_precondition_reasons(job_dir: Path) -> tuple[str, ...]:
    if any(
        resolve_job_relative_path(
            job_dir,
            path.replace(".typ", ".generated.typ"),
        ).exists()
        for path in RENDER_SOURCE_PATHS
    ):
        return ("input_not_ready:typst_candidate",)
    try:
        report = ApplicationGateReportV1.model_validate(
            read_json_object(job_dir / "application_gate_report.json")
        )
    except (StageStoreError, ValidationError):
        return ("input_not_ready:verify_report",)
    if report.status != "PASS":
        return ("input_not_ready:verify_failed",)
    try:
        render_input_artifacts(job_dir)
    except RenderStageError:
        return ("input_not_ready:typst_source",)
    return ()


def build_render_bundle_candidate(
    job_dir: Path,
    *,
    input_fingerprint: str,
    typst_bin: str = "typst",
) -> ArtifactBundleV1:
    if render_precondition_reasons(job_dir):
        raise RenderStageError("Render requires a passing current Verify report.")
    if render_input_fingerprint(job_dir) != input_fingerprint:
        raise RenderStageError("Render inputs changed before compilation.")
    executable = shutil.which(typst_bin) if Path(typst_bin).name == typst_bin else typst_bin
    if executable is None or not Path(executable).is_file():
        raise RenderStageError("The configured Typst compiler is unavailable.")

    sources = [
        resolve_job_relative_path(job_dir, path)
        for path in RENDER_SOURCE_PATHS
        if resolve_job_relative_path(job_dir, path).is_file()
    ]
    entries: list[BundleEntryV1] = []
    with tempfile.TemporaryDirectory(prefix="canisend-render-") as temporary:
        temporary_root = Path(temporary)
        for source in sources:
            output = temporary_root / f"{source.stem}.pdf"
            try:
                completed = subprocess.run(
                    [str(executable), "compile", str(source), str(output)],
                    text=True,
                    capture_output=True,
                    check=False,
                    timeout=120,
                )
            except (OSError, subprocess.SubprocessError) as exc:
                raise RenderStageError("Typst execution failed before producing a bundle.") from exc
            if completed.returncode != 0 or not output.is_file():
                raise RenderStageError("Typst rejected one or more package sources.")
            data = output.read_bytes()
            if not data.startswith(b"%PDF-"):
                raise RenderStageError("Typst output is not a valid PDF payload.")
            entries.append(
                BundleEntryV1.from_bytes(
                    path=f"pdf/{source.stem}.pdf",
                    media_type="application/pdf",
                    data=data,
                )
            )
    return ArtifactBundleV1(
        job_id=job_dir.name,
        stage="render",
        mode="guarded",
        input_fingerprint=input_fingerprint,
        entries=tuple(sorted(entries, key=lambda item: item.path)),
    )


def validate_render_bundle_candidate(
    candidate: object,
    *,
    job_dir: Path,
    input_fingerprint: str,
) -> ArtifactBundleV1:
    try:
        bundle = ArtifactBundleV1.model_validate(candidate)
    except ValidationError as exc:
        raise RenderStageError("Render bundle schema validation failed.") from exc
    expected_paths = {
        f"pdf/{Path(path).stem}.pdf"
        for path in RENDER_SOURCE_PATHS
        if resolve_job_relative_path(job_dir, path).is_file()
    }
    if (
        bundle.job_id != job_dir.name
        or bundle.stage != "render"
        or bundle.mode != "guarded"
        or bundle.input_fingerprint != input_fingerprint
        or {entry.path for entry in bundle.entries} != expected_paths
        or render_input_fingerprint(job_dir) != input_fingerprint
        or any(not entry.decoded_bytes().startswith(b"%PDF-") for entry in bundle.entries)
    ):
        raise RenderStageError("Render bundle identity, scope, or inputs are invalid.")
    return bundle
