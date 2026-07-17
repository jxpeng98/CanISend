from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Literal

from canisend.agent_protocol import (
    AgentResponse,
    NextAction,
    WorkflowSnapshotReference,
    artifact_reference_from_path,
    success_response,
)
from canisend.bundle_projection import (
    BundleProjectionError,
    inspect_artifact_projection,
    load_artifact_bundle,
    project_artifact_bundle,
)
from canisend.document_execution import inspect_document_execution
from canisend.job_coordination import JobCoordinationError, coordinate_job
from canisend.legacy_compatibility import (
    LegacyCompatibilityError,
    LegacyCompatibilityOutcome,
    run_legacy_package_compatibility,
)
from canisend.llm import LLMProvider
from canisend.render_execution import run_render_stage_with_compiler
from canisend.stage_registry import DEFAULT_STAGE_REGISTRY
from canisend.stage_runtime import (
    StageRunOutcome,
    StageRuntimeError,
    inspect_stage_status,
    run_configured_provider_stage,
    run_deterministic_stage,
)


SequenceDecision = Literal["current", "execute", "blocked", "repair"]
SequenceExecutionMode = Literal["deterministic", "configured_provider"]

LEGACY_OUTPUT_INVENTORY = (
    "parsed_job.json",
    "00_preparation_questions.md",
    "01_job_summary.md",
    "02_fit_report.md",
    "03_cover_letter_draft.md",
    "04_cv_tailoring_notes.md",
    "05_criteria_checklist.md",
    "06_final_application_package.md",
    "07_material_review_checklist.md",
    "typst/cover_letter_content.json",
    "typst/cover_letter.typ",
    "typst/application_package_content.json",
    "typst/application_package.typ",
)


class WorkflowSequenceError(RuntimeError):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class SequenceOptions:
    use_llm_parser: bool = False
    use_llm_drafts: bool = False
    allow_provider_backed: bool = False
    typst_bin: str = "typst"
    provider: LLMProvider | None = None
    legacy_compatibility: bool = False


@dataclass(frozen=True)
class SequenceItem:
    stage: str
    document_id: str | None
    decision: SequenceDecision
    status: str
    reason_codes: tuple[str, ...] = ()
    execution_mode: SequenceExecutionMode | None = None

    @property
    def key(self) -> tuple[str, str | None]:
        return self.stage, self.document_id


@dataclass(frozen=True)
class SequencePlan:
    job_id: str
    items: tuple[SequenceItem, ...]

    @property
    def complete(self) -> bool:
        return bool(self.items) and all(item.decision == "current" for item in self.items)

    @property
    def first_stop(self) -> SequenceItem | None:
        if any(item.decision == "execute" for item in self.items):
            return None
        return next(
            (item for item in self.items if item.decision in {"blocked", "repair"}),
            None,
        )


@dataclass(frozen=True)
class SequenceRunResult:
    plan: SequencePlan
    executed: tuple[StageRunOutcome, ...]
    projected_paths: tuple[str, ...]
    legacy_compatibility: LegacyCompatibilityOutcome | None = None

    @property
    def complete(self) -> bool:
        return self.plan.complete

    @property
    def stop_item(self) -> SequenceItem | None:
        return self.plan.first_stop


def plan_sequence(
    workspace: Path,
    job_dir: Path,
    *,
    options: SequenceOptions | None = None,
) -> SequencePlan:
    """Derive a read-only deterministic plan for every registered stage instance."""

    root, job = _sequence_paths(workspace, job_dir)
    selected = options or SequenceOptions()
    items: list[SequenceItem] = []
    document_items = None

    for definition in DEFAULT_STAGE_REGISTRY.topological_order():
        stage = definition.id
        if stage in {"draft", "review"}:
            if document_items is None:
                document_items = _document_work_items(root, job)
            if document_items is None:
                items.append(
                    SequenceItem(
                        stage=stage,
                        document_id=None,
                        decision="blocked",
                        status="blocked",
                        reason_codes=("documents.plan_not_current",),
                    )
                )
                continue
            active = tuple(item for item in document_items if item.action == "prepare")
            if not active:
                items.append(
                    SequenceItem(
                        stage=stage,
                        document_id=None,
                        decision="current",
                        status="succeeded",
                        reason_codes=("documents.no_work",),
                    )
                )
                continue
            for document in active:
                if stage == "draft" and document.state != "ready_to_prepare":
                    items.append(
                        SequenceItem(
                            stage=stage,
                            document_id=document.document_id,
                            decision="blocked",
                            status="blocked",
                            reason_codes=(
                                tuple(document.reason_codes)
                                or ("documents.executor_unavailable",)
                            ),
                        )
                    )
                    continue
                mode, mode_reason = _execution_mode(
                    stage,
                    selected,
                    document_modes=tuple(document.execution_modes),
                )
                items.append(
                    _plan_task_instance(
                        root,
                        job,
                        stage=stage,
                        document_id=document.document_id,
                        execution_mode=mode,
                        unavailable_reason=mode_reason,
                    )
                )
            continue

        if definition.execution_kind == "source":
            items.append(_plan_source_instance(root, job, stage=stage))
            continue
        mode, mode_reason = _execution_mode(stage, selected)
        items.append(
            _plan_task_instance(
                root,
                job,
                stage=stage,
                document_id=None,
                execution_mode=mode,
                unavailable_reason=mode_reason,
            )
        )

    return SequencePlan(job_id=job.name, items=tuple(items))


def run_sequence(
    workspace: Path,
    job_dir: Path,
    *,
    options: SequenceOptions | None = None,
) -> SequenceRunResult:
    """Execute all currently eligible work through registered guarded services."""

    root, job = _sequence_paths(workspace, job_dir)
    selected = options or SequenceOptions()
    executed: list[StageRunOutcome] = []
    projected: list[str] = []
    seen_progress: set[tuple[str, str | None, str]] = set()
    try:
        with coordinate_job(job):
            for _ in range(512):
                plan = plan_sequence(root, job, options=selected)
                repair = next(
                    (item for item in plan.items if item.decision == "repair"),
                    None,
                )
                if repair is not None:
                    return SequenceRunResult(
                        plan=plan,
                        executed=tuple(executed),
                        projected_paths=tuple(projected),
                    )
                runnable = next(
                    (item for item in plan.items if item.decision == "execute"),
                    None,
                )
                if runnable is None:
                    compatibility = _run_compatibility_if_eligible(
                        root,
                        job,
                        plan,
                        selected,
                    )
                    if (
                        compatibility is not None
                        and compatibility.journal is not None
                        and not compatibility.cache_hit
                    ):
                        projected.extend(
                            entry.target_path for entry in compatibility.journal.entries
                        )
                    return SequenceRunResult(
                        plan=plan,
                        executed=tuple(executed),
                        projected_paths=tuple(projected),
                        legacy_compatibility=compatibility,
                    )
                progress_key = (*runnable.key, runnable.status)
                if progress_key in seen_progress:
                    raise WorkflowSequenceError(
                        "sequence.no_progress",
                        "The workflow sequence could not make deterministic progress.",
                    )
                seen_progress.add(progress_key)
                outcome = _execute_item(root, job, runnable, selected)
                executed.append(outcome)
                if runnable.stage in {"package", "render"} and not outcome.cache_hit:
                    try:
                        bundle = load_artifact_bundle(outcome.authoritative_path)
                        journal = project_artifact_bundle(job, bundle)
                    except BundleProjectionError as exc:
                        raise WorkflowSequenceError(exc.code, str(exc)) from exc
                    projected.extend(entry.target_path for entry in journal.entries)
            raise WorkflowSequenceError(
                "sequence.limit_exceeded",
                "The workflow sequence exceeded its bounded progress limit.",
            )
    except JobCoordinationError as exc:
        raise WorkflowSequenceError(exc.code, str(exc)) from exc
    except StageRuntimeError as exc:
        raise WorkflowSequenceError(exc.code, str(exc)) from exc
    except LegacyCompatibilityError as exc:
        raise WorkflowSequenceError(exc.code, str(exc)) from exc


def sequence_agent_response(
    workspace: Path,
    job_dir: Path,
    result: SequenceRunResult,
) -> AgentResponse:
    stop = result.stop_item
    phase = _phase_for_stage(stop.stage if stop is not None else "render")
    artifacts = []
    state_path = job_dir / "workflow" / "state.json"
    if state_path.is_file():
        artifacts.append(
            artifact_reference_from_path(
                workspace=workspace,
                path=state_path,
                kind="workflow_state",
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
                include_hash=True,
            )
        )
    blockers = [] if stop is None else [_safe_blocker(stop)]
    actions = [] if stop is None else [_next_action(stop)]
    return success_response(
        operation="workflow.sequence_run",
        workflow=WorkflowSnapshotReference(
            phase=phase,
            readiness="ready_for_next_stage" if result.complete else "blocked",
        ),
        artifacts=artifacts,
        blockers=blockers,
        next_actions=actions,
        extensions={
            "canisend.sequence.complete": result.complete,
            "canisend.sequence.executed_count": len(result.executed),
            "canisend.sequence.projected_count": len(result.projected_paths),
            "canisend.sequence.legacy_compatibility": bool(
                result.legacy_compatibility and result.legacy_compatibility.active
            ),
            "canisend.sequence.stop_stage": stop.stage if stop is not None else None,
            "canisend.sequence.stop_decision": stop.decision if stop is not None else None,
        },
    )


def sequence_plan_agent_response(
    workspace: Path,
    job_dir: Path,
    plan: SequencePlan,
) -> AgentResponse:
    stop = plan.first_stop
    next_work = next(
        (item for item in plan.items if item.decision == "execute"),
        None,
    )
    phase = _phase_for_stage(
        stop.stage if stop is not None else (next_work.stage if next_work is not None else "render")
    )
    state_path = job_dir / "workflow" / "state.json"
    artifacts = (
        [
            artifact_reference_from_path(
                workspace=workspace,
                path=state_path,
                kind="workflow_state",
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
                include_hash=True,
            )
        ]
        if state_path.is_file()
        else []
    )
    return success_response(
        operation="workflow.sequence_plan",
        workflow=WorkflowSnapshotReference(
            phase=phase,
            readiness="ready_for_next_stage" if plan.complete else "action_required",
        ),
        artifacts=artifacts,
        blockers=[] if stop is None else [_safe_blocker(stop)],
        next_actions=(
            [_next_action(stop)]
            if stop is not None
            else (
                [NextAction(id="workflow.sequence_run", label="Run the eligible workflow stages")]
                if next_work is not None
                else []
            )
        ),
        extensions={
            "canisend.sequence.complete": plan.complete,
            "canisend.sequence.current_count": sum(
                item.decision == "current" for item in plan.items
            ),
            "canisend.sequence.execute_count": sum(
                item.decision == "execute" for item in plan.items
            ),
            "canisend.sequence.blocked_count": sum(
                item.decision == "blocked" for item in plan.items
            ),
            "canisend.sequence.repair_count": sum(
                item.decision == "repair" for item in plan.items
            ),
            "canisend.sequence.stop_stage": stop.stage if stop is not None else None,
        },
    )


def sequence_plan_lines(plan: SequencePlan) -> list[str]:
    lines = [f"Workflow sequence for {plan.job_id}:"]
    for item in plan.items:
        scope = f"/{item.document_id}" if item.document_id is not None else ""
        mode = f" ({item.execution_mode})" if item.execution_mode is not None else ""
        reasons = f" — {', '.join(item.reason_codes)}" if item.reason_codes else ""
        lines.append(f"  - {item.stage}{scope}: {item.decision}{mode}{reasons}")
    return lines


def _plan_source_instance(workspace: Path, job: Path, *, stage: str) -> SequenceItem:
    try:
        inspection = inspect_stage_status(workspace, job, stage=stage)  # type: ignore[arg-type]
    except StageRuntimeError as exc:
        return SequenceItem(stage, None, "blocked", "blocked", (exc.code,))
    if stage == "decide" and inspection.source_value in {"hold", "skip"}:
        return SequenceItem(
            stage,
            None,
            "blocked",
            inspection.stage.status,
            (f"decision.{inspection.source_value}",),
        )
    if inspection.stage.status == "succeeded" and not inspection.reasons:
        return SequenceItem(stage, None, "current", inspection.stage.status)
    return SequenceItem(
        stage,
        None,
        "blocked",
        inspection.stage.status,
        tuple(inspection.reasons) or ("source.not_current",),
    )


def _plan_task_instance(
    workspace: Path,
    job: Path,
    *,
    stage: str,
    document_id: str | None,
    execution_mode: SequenceExecutionMode | None,
    unavailable_reason: str | None,
) -> SequenceItem:
    try:
        inspection = inspect_stage_status(
            workspace,
            job,
            stage=stage,  # type: ignore[arg-type]
            document_id=document_id,
        )
    except StageRuntimeError as exc:
        return SequenceItem(stage, document_id, "blocked", "blocked", (exc.code,))
    receipt_reasons = set(inspection.reasons)
    if inspection.output_drift or receipt_reasons & {
        "output_drift",
        "output_missing",
        "output_unreadable",
    }:
        return SequenceItem(
            stage,
            document_id,
            "repair",
            inspection.stage.status,
            tuple(inspection.reasons) or ("output_drift",),
        )
    if inspection.stage.status == "succeeded" and not inspection.reasons:
        projection = _projection_decision(job, stage)
        if projection is not None:
            return SequenceItem(
                stage,
                document_id,
                projection[0],
                inspection.stage.status,
                projection[1],
            )
        return SequenceItem(stage, document_id, "current", inspection.stage.status)
    if inspection.input_fingerprint is None:
        return SequenceItem(
            stage,
            document_id,
            "blocked",
            inspection.stage.status,
            tuple(inspection.reasons) or ("dependency.not_current",),
        )
    if execution_mode is None:
        return SequenceItem(
            stage,
            document_id,
            "blocked",
            inspection.stage.status,
            (unavailable_reason or "executor.unavailable",),
        )
    return SequenceItem(
        stage,
        document_id,
        "execute",
        inspection.stage.status,
        tuple(inspection.reasons),
        execution_mode,
    )


def _projection_decision(
    job: Path,
    stage: str,
) -> tuple[SequenceDecision, tuple[str, ...]] | None:
    if stage not in {"package", "render"}:
        return None
    try:
        bundle = load_artifact_bundle(job / f"{stage}_bundle.json")
        inspection = inspect_artifact_projection(job, bundle)
    except BundleProjectionError:
        return "repair", ("projection.invalid",)
    if inspection.current:
        return "current", ()
    reasons = tuple(
        [f"projection.missing:{path}" for path in inspection.missing]
        + [f"projection.drifted:{path}" for path in inspection.drifted]
    )
    return "repair", reasons or ("projection.not_current",)


def _execution_mode(
    stage: str,
    options: SequenceOptions,
    *,
    document_modes: tuple[str, ...] = (),
) -> tuple[SequenceExecutionMode | None, str | None]:
    if stage == "parse" and options.use_llm_parser:
        if not options.allow_provider_backed:
            return None, "provider.consent_required"
        return "configured_provider", None
    if stage == "draft":
        if options.use_llm_drafts and "configured_provider" in document_modes:
            if not options.allow_provider_backed:
                return None, "provider.consent_required"
            return "configured_provider", None
        if "host_agent" in document_modes:
            return None, "executor.host_agent_required"
        return None, "executor.unavailable"
    return "deterministic", None


def _document_work_items(workspace: Path, job: Path):
    inspection = inspect_document_execution(workspace, job)
    return inspection.plan.items if inspection.source_state == "current" and inspection.plan else None


def _execute_item(
    workspace: Path,
    job: Path,
    item: SequenceItem,
    options: SequenceOptions,
) -> StageRunOutcome:
    if item.stage == "render":
        return run_render_stage_with_compiler(
            workspace,
            job,
            typst_bin=options.typst_bin,
        )
    if item.execution_mode == "configured_provider":
        return run_configured_provider_stage(
            workspace,
            job,
            stage=item.stage,  # type: ignore[arg-type]
            document_id=item.document_id,
            allow_provider_backed=options.allow_provider_backed,
            provider=options.provider,
        )
    return run_deterministic_stage(
        workspace,
        job,
        stage=item.stage,  # type: ignore[arg-type]
        document_id=item.document_id,
    )


def _run_compatibility_if_eligible(
    workspace: Path,
    job: Path,
    plan: SequencePlan,
    options: SequenceOptions,
) -> LegacyCompatibilityOutcome | None:
    if not options.legacy_compatibility:
        return None
    stop = plan.first_stop
    if (
        stop is None
        or stop.stage != "decide"
        or stop.decision != "blocked"
        or not set(stop.reason_codes)
        & {
            "input_not_ready:decision_not_initialized",
            "input_not_ready:decision_undecided",
            "input_not_ready:decision_unavailable",
        }
    ):
        return None
    prerequisites = {
        item.stage: item.decision
        for item in plan.items
        if item.document_id is None
    }
    if any(
        prerequisites.get(stage) != "current"
        for stage in ("intake", "evidence", "parse", "confirm", "match")
    ):
        return None
    return run_legacy_package_compatibility(workspace, job)


def _sequence_paths(workspace: Path, job_dir: Path) -> tuple[Path, Path]:
    root = workspace.expanduser().resolve()
    job = job_dir.expanduser()
    job = (root / job).resolve() if not job.is_absolute() else job.resolve()
    try:
        job.relative_to(root)
    except ValueError as exc:
        raise WorkflowSequenceError(
            "sequence.job_outside_workspace",
            "The workflow sequence requires a job inside the selected workspace.",
        ) from exc
    if not job.is_dir():
        raise WorkflowSequenceError("job.not_found", "The requested job directory does not exist.")
    return root, job


def _phase_for_stage(stage: str) -> Literal[
    "intake", "evidence", "parse", "package", "verify", "render", "unknown"
]:
    if stage in {"intake", "evidence", "parse", "package", "verify", "render"}:
        return stage  # type: ignore[return-value]
    if stage in {"confirm", "match", "decide", "brief", "draft", "review", "package_review"}:
        return "package"
    return "unknown"


def _safe_blocker(item: SequenceItem) -> str:
    scope = f" document {item.document_id}" if item.document_id is not None else ""
    reason = item.reason_codes[0] if item.reason_codes else "workflow.blocked"
    return f"Stage {item.stage}{scope} is {item.decision}: {reason}."


def _next_action(item: SequenceItem) -> NextAction:
    reasons = set(item.reason_codes)
    if item.decision == "repair":
        if item.stage in {"package", "render"} and any(
            reason.startswith("projection.") for reason in reasons
        ):
            return NextAction(
                id="workflow.repair_projection",
                label=f"Explicitly repair the {item.stage} bundle projection",
            )
        return NextAction(
            id="workflow.stage_status",
            label=f"Inspect and reconcile authoritative {item.stage} output drift",
        )
    if item.stage == "intake":
        return NextAction(id="job.import_advert", label="Provide and review the full job advert")
    if item.stage == "decide":
        return NextAction(id="decision.status", label="Review or update the application decision")
    if item.stage == "brief":
        return NextAction(id="brief.status", label="Review or update the application brief")
    if item.stage == "draft" and "executor.host_agent_required" in reasons:
        return NextAction(id="workflow.stage_prepare", label="Prepare the Draft task for a host agent")
    if item.stage == "package" or any("package" in reason for reason in reasons):
        return NextAction(
            id="package_review.dispositions_status",
            label="Review current package findings and dispositions",
        )
    if item.stage == "render" and "input_not_ready:verify_failed" in reasons:
        return NextAction(id="package.resolve_blockers", label="Resolve application gate blockers")
    return NextAction(id="workflow.stage_status", label=f"Inspect the blocked {item.stage} stage")
