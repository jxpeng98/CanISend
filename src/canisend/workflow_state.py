from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
from typing import Literal

import yaml

from canisend.agent_protocol import (
    AgentError,
    AgentResponse,
    ArtifactReference,
    ConsentRequirement,
    GateOutcome,
    JobReference,
    NextAction,
    WorkflowSnapshotReference,
    artifact_reference_from_path,
    success_response,
)
from canisend.jobs import JobMetadataError, job_advert_is_stub, load_job_metadata, next_job_action
from canisend.ready_check import APPLICATION_GATE_REPORT
from canisend.workspace import load_workspace_config


EvidenceState = Literal["current", "stale", "not_generated", "missing", "error"]


@dataclass(frozen=True)
class DerivedWorkflowSnapshot:
    workflow: WorkflowSnapshotReference
    job: JobReference | None = None
    artifacts: tuple[ArtifactReference, ...] = ()
    missing_fields: tuple[str, ...] = ()
    required_consents: tuple[ConsentRequirement, ...] = ()
    warnings: tuple[str, ...] = ()
    blockers: tuple[str, ...] = ()
    next_actions: tuple[NextAction, ...] = ()
    gate: GateOutcome | None = None
    error_code: str | None = None
    error_message: str | None = None


@dataclass(frozen=True)
class ProfileEvidenceInspection:
    state: EvidenceState
    artifacts: tuple[ArtifactReference, ...]
    warnings: tuple[str, ...] = ()


def workflow_snapshot_agent_response(
    snapshot: DerivedWorkflowSnapshot,
    *,
    operation: str = "agent.context",
) -> AgentResponse:
    if snapshot.error_code is not None:
        return AgentResponse(
            operation=operation,
            ok=False,
            workflow=snapshot.workflow,
            artifacts=list(snapshot.artifacts),
            missing_fields=list(snapshot.missing_fields),
            required_consents=list(snapshot.required_consents),
            warnings=list(snapshot.warnings),
            blockers=list(snapshot.blockers),
            next_actions=list(snapshot.next_actions),
            gate=snapshot.gate,
            error=AgentError(
                code=snapshot.error_code,
                message=snapshot.error_message or "The requested context could not be derived.",
            ),
        )
    return success_response(
        operation=operation,
        job=snapshot.job,
        workflow=snapshot.workflow,
        artifacts=list(snapshot.artifacts),
        missing_fields=list(snapshot.missing_fields),
        required_consents=list(snapshot.required_consents),
        warnings=list(snapshot.warnings),
        blockers=list(snapshot.blockers),
        next_actions=list(snapshot.next_actions),
        gate=snapshot.gate,
    )


def derive_workflow_snapshot(workspace: Path, job: Path) -> DerivedWorkflowSnapshot:
    root = workspace.expanduser().resolve()
    config = load_workspace_config(root)
    job_dir = config.job_dir(job).expanduser().resolve()

    try:
        relative_job_dir = job_dir.relative_to(root).as_posix()
    except ValueError:
        reference = artifact_reference_from_path(
            workspace=root,
            path=job_dir,
            kind="job_directory",
            privacy_tier=2,
            trust_level="trusted_local",
            media_type="inode/directory",
        )
        return _error_snapshot(
            code="input.invalid",
            message="Agent context requires a job directory inside the workspace.",
            artifacts=(reference,),
        )

    if not job_dir.is_dir():
        return _error_snapshot(
            code="job.not_found",
            message="The requested job directory does not exist.",
        )

    artifacts = _job_artifacts(root, job_dir)
    try:
        metadata = load_job_metadata(job_dir)
    except JobMetadataError:
        return _error_snapshot(
            code="job.invalid_metadata",
            message="The requested job has missing or invalid job.yaml metadata.",
            artifacts=tuple(artifacts),
        )

    job_reference = JobReference(
        id=str(metadata["id"]),
        path=relative_job_dir,
        title=str(metadata["title"]),
        institution=str(metadata["institution"]),
        deadline=str(metadata["deadline"]),
        status=str(metadata["status"]),
    )
    status = str(metadata["status"])
    missing_fields = _preference_missing_fields(metadata)
    warnings: list[str] = []
    blockers: list[str] = []
    consents: list[ConsentRequirement] = []
    actions: list[NextAction] = []

    advert_path = job_dir / "job_advert.md"
    advert_is_stub = True
    if advert_path.is_file():
        try:
            advert_is_stub = job_advert_is_stub(advert_path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError):
            warnings.append("The job advert could not be inspected safely.")
    if advert_is_stub:
        missing_fields.insert(0, "job_advert.md")
        blockers.append("A reviewed full job advert is required before evidence matching or drafting.")
        legacy_label = next_job_action(job_dir, status)
        label = (
            legacy_label.capitalize()
            if legacy_label in {"paste full advert", "add advert"}
            else "Import full advert"
        )
        actions.append(NextAction(id="job.import_advert", label=label))
        return _result(
            phase="intake",
            readiness="blocked",
            job=job_reference,
            artifacts=artifacts,
            missing_fields=missing_fields,
            warnings=warnings,
            blockers=blockers,
            actions=actions,
        )

    parsed_path = job_dir / "parsed_job.json"
    parsed_is_valid = _is_json_object(parsed_path)
    if not parsed_is_valid:
        consents.append(_read_advert_consent())

    profile = _inspect_profile_evidence(root, config.path("profile_dir"))
    artifacts.extend(profile.artifacts)
    warnings.extend(profile.warnings)
    if any(artifact.opaque_id is not None for artifact in profile.artifacts):
        warnings.append("External profile artifacts are represented by opaque identifiers.")

    if profile.state != "current":
        missing_fields.append("profile.evidence")
        if profile.state in {"not_generated", "stale"}:
            consents.append(_read_profile_sources_consent())
            actions.append(
                NextAction(
                    id="profile.extract_evidence",
                    label="Refresh profile evidence",
                    requires_consent=True,
                    consent_ids=["read-profile-sources"],
                )
            )
        elif profile.state == "missing":
            actions.append(NextAction(id="profile.initialize", label="Initialize the applicant profile"))
        else:
            actions.append(NextAction(id="profile.review_manifest", label="Review the profile manifest"))
        warnings.append("Profile evidence is not current.")
        return _result(
            phase="evidence",
            readiness="action_required",
            job=job_reference,
            artifacts=artifacts,
            missing_fields=missing_fields,
            consents=consents,
            warnings=warnings,
            blockers=blockers,
            actions=actions,
        )

    if not parsed_is_valid:
        if parsed_path.exists():
            warnings.append("parsed_job.json is present but is not a valid JSON object.")
        missing_fields.append("parsed_job.json")
        actions.append(
            NextAction(
                id="job.parse",
                label="Parse and validate the reviewed job advert",
                requires_consent=True,
                consent_ids=["read-full-job-advert"],
            )
        )
        return _result(
            phase="parse",
            readiness="ready_for_next_stage" if not parsed_path.exists() else "action_required",
            job=job_reference,
            artifacts=artifacts,
            missing_fields=missing_fields,
            consents=consents,
            warnings=warnings,
            blockers=blockers,
            actions=actions,
        )

    if missing_fields:
        actions.append(NextAction(id="job.confirm_preferences", label="Confirm deadline and writing preferences"))
        blockers.append("Job deadline and writing preferences must be confirmed before package generation.")
        return _result(
            phase="package",
            readiness="action_required",
            job=job_reference,
            artifacts=artifacts,
            missing_fields=missing_fields,
            warnings=warnings,
            blockers=blockers,
            actions=actions,
        )

    candidates = sorted((job_dir / "typst").glob("*.generated.typ"))
    if candidates:
        for candidate in candidates:
            artifacts.append(
                artifact_reference_from_path(
                    workspace=root,
                    path=candidate,
                    kind="typst_candidate",
                    privacy_tier=2,
                    trust_level="generated_candidate",
                    media_type="text/plain",
                    include_hash=True,
                )
            )
        blockers.append("Generated Typst candidates require human reconciliation with editable sources.")
        actions.append(NextAction(id="typst.review_candidate", label="Review generated Typst candidates"))
        return _result(
            phase="render",
            readiness="review_required",
            job=job_reference,
            artifacts=artifacts,
            warnings=warnings,
            blockers=blockers,
            actions=actions,
        )

    review_path = job_dir / "07_material_review_checklist.md"
    if not review_path.is_file():
        legacy_label = next_job_action(job_dir, status)
        label = legacy_label.capitalize() if status == "packaged" else "Generate the application package"
        consents.append(_provider_execution_consent())
        actions.append(
            NextAction(
                id="package.generate",
                label=label,
                requires_consent=True,
                consent_ids=["invoke-model-provider"],
            )
        )
        return _result(
            phase="package",
            readiness="action_required" if status == "packaged" else "ready_for_next_stage",
            job=job_reference,
            artifacts=artifacts,
            missing_fields=["07_material_review_checklist.md"] if status == "packaged" else [],
            consents=consents,
            warnings=warnings,
            blockers=blockers,
            actions=actions,
        )

    gate, gate_warning = _read_gate_outcome(root, job_dir)
    if gate_warning:
        warnings.append(gate_warning)
    if gate.status == "FAIL":
        blockers.append("The latest application gate report contains unresolved blockers.")
        actions.append(NextAction(id="package.resolve_blockers", label="Resolve application gate blockers"))
        return _result(
            phase="verify",
            readiness="blocked",
            job=job_reference,
            artifacts=artifacts,
            warnings=warnings,
            blockers=blockers,
            actions=actions,
            gate=gate,
        )
    if gate.status == "STALE":
        actions.append(NextAction(id="package.check", label="Rerun the application package gate"))
        return _result(
            phase="verify",
            readiness="action_required",
            job=job_reference,
            artifacts=artifacts,
            warnings=warnings,
            blockers=blockers,
            actions=actions,
            gate=gate,
        )
    if gate.status == "PASS":
        actions.append(NextAction(id="package.review", label="Perform final human review before manual submission"))
        return _result(
            phase="render",
            readiness="review_required",
            job=job_reference,
            artifacts=artifacts,
            warnings=warnings,
            blockers=blockers,
            actions=actions,
            gate=gate,
        )

    actions.append(NextAction(id="package.check", label="Run the application package gate"))
    return _result(
        phase="verify",
        readiness="ready_for_next_stage" if gate_warning is None else "review_required",
        job=job_reference,
        artifacts=artifacts,
        warnings=warnings,
        blockers=blockers,
        actions=actions,
        gate=gate,
    )


def _job_artifacts(workspace: Path, job_dir: Path) -> list[ArtifactReference]:
    definitions = [
        ("job_metadata", "job.yaml", 1, "validated", "application/yaml"),
        ("job_advert", "job_advert.md", 2, "untrusted_import", "text/markdown"),
        ("parsed_job", "parsed_job.json", 1, "validated", "application/json"),
        ("material_review", "07_material_review_checklist.md", 2, "generated_candidate", "text/markdown"),
        ("application_gate_report", APPLICATION_GATE_REPORT, 1, "validated", "application/json"),
    ]
    return [
        artifact_reference_from_path(
            workspace=workspace,
            path=job_dir / relative_path,
            kind=kind,
            privacy_tier=privacy_tier,
            trust_level=trust_level,
            media_type=media_type,
            include_hash=True,
        )
        for kind, relative_path, privacy_tier, trust_level, media_type in definitions
    ]


def _inspect_profile_evidence(workspace: Path, profile_dir: Path) -> ProfileEvidenceInspection:
    manifest_path = profile_dir / "profile.yaml"
    artifacts = [
        artifact_reference_from_path(
            workspace=workspace,
            path=manifest_path,
            kind="profile_manifest",
            privacy_tier=2,
            trust_level="trusted_local",
            media_type="application/yaml",
            include_hash=True,
        )
    ]
    if not manifest_path.is_file():
        return ProfileEvidenceInspection(state="missing", artifacts=tuple(artifacts))
    try:
        manifest = yaml.safe_load(manifest_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, yaml.YAMLError):
        return ProfileEvidenceInspection(
            state="error",
            artifacts=tuple(artifacts),
            warnings=("The profile manifest is not valid YAML.",),
        )
    if not isinstance(manifest, dict):
        return ProfileEvidenceInspection(
            state="error",
            artifacts=tuple(artifacts),
            warnings=("The profile manifest must contain a mapping.",),
        )
    sources = manifest.get("sources", {})
    generated = manifest.get("generated", {})
    if not isinstance(sources, dict) or not isinstance(generated, dict):
        return ProfileEvidenceInspection(
            state="error",
            artifacts=tuple(artifacts),
            warnings=("Profile source and generated-path declarations must be mappings.",),
        )
    if not sources:
        return ProfileEvidenceInspection(state="not_generated", artifacts=tuple(artifacts))

    has_missing = False
    has_stale = False
    has_source = False
    for source_key, raw_source in sources.items():
        source_value = _path_value(raw_source)
        if source_value is None:
            has_missing = True
            continue
        source_path = _resolve_configured_path(profile_dir, source_value)
        artifacts.append(
            artifact_reference_from_path(
                workspace=workspace,
                path=source_path,
                kind="profile_source",
                privacy_tier=2,
                trust_level="trusted_local",
                media_type="application/octet-stream",
                include_hash=True,
            )
        )
        if not source_path.is_file():
            has_missing = True
            continue
        has_source = True
        raw_evidence = generated.get(f"{source_key}_evidence", f"generated/{source_key}.evidence.md")
        evidence_value = _path_value(raw_evidence)
        if evidence_value is None:
            has_missing = True
            continue
        evidence_path = _resolve_configured_path(profile_dir, evidence_value)
        artifacts.append(
            artifact_reference_from_path(
                workspace=workspace,
                path=evidence_path,
                kind="profile_evidence",
                privacy_tier=1,
                trust_level="validated",
                media_type="text/markdown",
                include_hash=True,
            )
        )
        if not evidence_path.is_file():
            has_missing = True
        elif source_path.stat().st_mtime_ns > evidence_path.stat().st_mtime_ns:
            has_stale = True

    if has_missing or not has_source:
        state: EvidenceState = "not_generated"
    elif has_stale:
        state = "stale"
    else:
        state = "current"
    return ProfileEvidenceInspection(state=state, artifacts=tuple(artifacts))


def _read_gate_outcome(workspace: Path, job_dir: Path) -> tuple[GateOutcome, str | None]:
    report_path = job_dir / APPLICATION_GATE_REPORT
    if not report_path.is_file():
        return GateOutcome(status="NOT_RUN"), None
    try:
        report = json.loads(report_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return GateOutcome(status="NOT_RUN", report_path=_relative_path(workspace, report_path)), (
            "The application gate report is invalid."
        )
    if not isinstance(report, dict) or report.get("status") not in {"PASS", "FAIL", "STALE"}:
        return GateOutcome(status="NOT_RUN", report_path=_relative_path(workspace, report_path)), (
            "The application gate report has an unsupported status."
        )
    issues = report.get("issues", [])
    issue_count = len(issues) if isinstance(issues, list) else 0
    return (
        GateOutcome(
            status=report["status"],
            issue_count=issue_count,
            report_path=_relative_path(workspace, report_path),
        ),
        None,
    )


def _preference_missing_fields(metadata: dict[str, object]) -> list[str]:
    missing: list[str] = []
    if str(metadata.get("deadline", "")).strip().lower() in {"", "unknown", "needs_confirmation"}:
        missing.append("job.deadline")
    if str(metadata.get("english_variant", "")).strip().lower() not in {"uk", "us"}:
        missing.append("job.english_variant")
    if str(metadata.get("writing_style", "")).strip().lower() in {"", "unknown", "needs_confirmation"}:
        missing.append("job.writing_style")
    return missing


def _read_advert_consent() -> ConsentRequirement:
    return ConsentRequirement(
        id="read-full-job-advert",
        purpose="Allow the host agent to read the full imported job advert for parsing.",
        privacy_tier=2,
        artifact_kinds=["job_advert"],
    )


def _read_profile_sources_consent() -> ConsentRequirement:
    return ConsentRequirement(
        id="read-profile-sources",
        purpose="Allow the host agent to read configured profile sources for evidence extraction.",
        privacy_tier=2,
        artifact_kinds=["profile_source"],
    )


def _provider_execution_consent() -> ConsentRequirement:
    return ConsentRequirement(
        id="invoke-model-provider",
        purpose="Allow an approved model executor to process selected application context.",
        privacy_tier=3,
        artifact_kinds=["job_advert", "profile_evidence"],
    )


def _result(
    *,
    phase: str,
    readiness: str,
    job: JobReference,
    artifacts: list[ArtifactReference],
    missing_fields: list[str] | None = None,
    warnings: list[str],
    blockers: list[str],
    actions: list[NextAction],
    consents: list[ConsentRequirement] | None = None,
    gate: GateOutcome | None = None,
) -> DerivedWorkflowSnapshot:
    return DerivedWorkflowSnapshot(
        workflow=WorkflowSnapshotReference(phase=phase, readiness=readiness),
        job=job,
        artifacts=tuple(artifacts),
        missing_fields=tuple(dict.fromkeys(missing_fields or [])),
        required_consents=_unique_consents(consents or []),
        warnings=tuple(dict.fromkeys(warnings)),
        blockers=tuple(dict.fromkeys(blockers)),
        next_actions=tuple(actions),
        gate=gate,
    )


def _error_snapshot(
    *,
    code: str,
    message: str,
    artifacts: tuple[ArtifactReference, ...] = (),
) -> DerivedWorkflowSnapshot:
    return DerivedWorkflowSnapshot(
        workflow=WorkflowSnapshotReference(phase="unknown", readiness="blocked"),
        artifacts=artifacts,
        blockers=(message,),
        error_code=code,
        error_message=message,
    )


def _unique_consents(consents: list[ConsentRequirement]) -> tuple[ConsentRequirement, ...]:
    unique: dict[str, ConsentRequirement] = {}
    for consent in consents:
        unique.setdefault(consent.id, consent)
    return tuple(unique.values())


def _is_json_object(path: Path) -> bool:
    if not path.is_file():
        return False
    try:
        return isinstance(json.loads(path.read_text(encoding="utf-8")), dict)
    except (OSError, UnicodeError, json.JSONDecodeError):
        return False


def _path_value(value: object) -> str | None:
    if value is None or isinstance(value, (dict, list)):
        return None
    normalized = str(value).strip()
    return normalized or None


def _resolve_configured_path(root: Path, value: str) -> Path:
    path = Path(value).expanduser()
    return path if path.is_absolute() else root / path


def _relative_path(workspace: Path, path: Path) -> str:
    return path.resolve().relative_to(workspace.resolve()).as_posix()
