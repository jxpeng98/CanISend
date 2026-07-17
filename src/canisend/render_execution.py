from __future__ import annotations

import json
from pathlib import Path

from canisend.job_coordination import JobCoordinationError, coordinate_job
from canisend.stage_runtime import (
    StageRunOutcome,
    StageRuntimeError,
    apply_stage_result,
    inspect_stage_status,
    prepare_stage,
    submit_stage_candidate,
)
from canisend.stages.render_stage import (
    RenderStageError,
    build_render_bundle_candidate,
)


def run_render_stage_with_compiler(
    workspace: Path,
    job_dir: Path,
    *,
    typst_bin: str = "typst",
) -> StageRunOutcome:
    """Run Render through the guarded runtime while honoring an explicit compiler."""

    root = workspace.expanduser().resolve()
    job = job_dir.expanduser().resolve()
    try:
        with coordinate_job(job):
            status = inspect_stage_status(root, job, stage="render")
            target = job / "render_bundle.json"
            if status.input_fingerprint is None:
                raise StageRuntimeError(
                    "stage.dependency_not_current",
                    "Render requires a current passing Verify stage.",
                )
            if status.output_drift:
                raise StageRuntimeError(
                    "stage.output_conflict",
                    "The authoritative Render bundle changed since promotion.",
                )
            if status.stage.status == "succeeded" and not status.reasons:
                return StageRunOutcome(
                    stage="render",
                    document_id=None,
                    cache_hit=True,
                    state=status.state,
                    authoritative_path=target,
                )

            prepared = prepare_stage(
                root,
                job,
                stage="render",
                execution_mode="deterministic",
            )
            try:
                candidate = build_render_bundle_candidate(
                    job,
                    input_fingerprint=prepared.task_spec.input_fingerprint,
                    typst_bin=typst_bin,
                )
            except (OSError, UnicodeError, ValueError, RenderStageError) as exc:
                raise StageRuntimeError(
                    "stage.invalid_input",
                    "Render inputs or compiler output could not produce a valid bundle.",
                ) from exc
            candidate_bytes = (
                json.dumps(
                    candidate.model_dump(mode="json"),
                    ensure_ascii=False,
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            ).encode("utf-8")
            submitted = submit_stage_candidate(
                root,
                job,
                task_spec_path=prepared.task_spec_path,
                candidate_bytes=candidate_bytes,
            )
            applied = apply_stage_result(
                root,
                job,
                task_spec_path=prepared.task_spec_path,
                task_result_path=submitted.result_path,
            )
            return StageRunOutcome(
                stage="render",
                document_id=None,
                cache_hit=False,
                state=applied.state,
                authoritative_path=applied.authoritative_path,
                manifest=applied.manifest,
                manifest_path=applied.manifest_path,
            )
    except JobCoordinationError as exc:
        raise StageRuntimeError(exc.code, str(exc)) from exc
