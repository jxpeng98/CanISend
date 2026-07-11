from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re

from pydantic import ValidationError

from canisend.agent_protocol import (
    AgentResponse,
    ArtifactReference,
    ConsentRequirement,
    JsonScalar,
    KNOWN_AGENT_ERROR_CODES,
    NextAction,
    Readiness,
    WorkflowSnapshotReference,
    error_response,
    success_response,
)
from canisend.decision_models import ApplicationDecisionV1
from canisend.stage_runtime import StageRuntimeError, inspect_stage_status
from canisend.user_file_store import (
    InvalidUserFileError,
    UnsafeUserFileError,
    load_strict_yaml,
    load_strict_json,
    read_safe_bytes,
)
from canisend.user_mutations import (
    APPLICATION_DECISION_PATH,
    CONFIRMED_CORRECTIONS_PATH,
    ApplicationDecisionInspection,
    CorrectionsPatch,
    CurrentArtifactMutationAudit,
    DecisionPatch,
    MutationOutcome,
    UserArtifactKind,
    UserArtifactSnapshot,
    UserMutationError,
    UserMutationReceiptV1,
    inspect_application_decision,
    inspect_current_artifact_mutation,
    inspect_user_mutation,
    parse_corrections_patch,
    parse_decision_patch,
)


PATCH_FILE_MAX_BYTES = 256 * 1024
_MUTATION_ID_RE = re.compile(r"^mutation_[0-9a-f]{32}$")


@dataclass(frozen=True)
class UserMutationAgentProjection:
    readiness: Readiness
    artifacts: tuple[ArtifactReference, ...] = ()
    missing_fields: tuple[str, ...] = ()
    required_consents: tuple[ConsentRequirement, ...] = ()
    warnings: tuple[str, ...] = ()
    blockers: tuple[str, ...] = ()
    next_actions: tuple[NextAction, ...] = ()
    extensions: tuple[tuple[str, JsonScalar], ...] = ()

    def extension_dict(self) -> dict[str, JsonScalar]:
        return dict(self.extensions)


def load_corrections_patch_file(path: Path) -> CorrectionsPatch:
    return parse_corrections_patch(_load_bounded_patch_mapping(path))


def load_decision_patch_file(path: Path) -> DecisionPatch:
    return parse_decision_patch(_load_bounded_patch_mapping(path))


def corrections_status_agent_response(
    workspace: Path,
    job_dir: Path,
    snapshot: UserArtifactSnapshot | None,
) -> AgentResponse:
    projection = corrections_agent_projection(workspace, job_dir, snapshot)
    return _projection_response("criteria.corrections_status", projection)


def application_decision_status_agent_response(
    workspace: Path,
    job_dir: Path,
    inspection: ApplicationDecisionInspection,
) -> AgentResponse:
    if inspection.snapshot is None and inspection.reason != "user_input.not_initialized":
        audit = inspect_current_artifact_mutation(workspace, job_dir, "decision")
        if audit.mutation_id is None:
            return user_mutation_error_response(
                "decision.status",
                UserMutationError(
                    inspection.reason or "user_input.invalid",
                    "The user-owned decision cannot be inspected safely.",
                ),
            )
    return _projection_response(
        "decision.status",
        decision_agent_projection(workspace, job_dir, inspection),
    )


def mutation_outcome_agent_response(
    workspace: Path,
    job_dir: Path,
    outcome: MutationOutcome,
    *,
    operation: str,
) -> AgentResponse:
    if outcome.snapshot.artifact == "decision":
        projection = decision_agent_projection(
            workspace,
            job_dir,
            inspect_application_decision(workspace, job_dir),
        )
    else:
        projection = corrections_agent_projection(
            workspace,
            job_dir,
            outcome.snapshot,
            expose_update_action=operation.endswith("initialize"),
        )
        if operation.endswith("update") or (
            operation == "user_mutation.recover"
            and (outcome.changed or _confirm_requires_rerun(workspace, job_dir))
        ):
            projection = _replace_actions(
                projection,
                (
                    NextAction(
                        id="stage.run_confirm",
                        label="Rerun Confirm against the accepted corrections",
                    ),
                ),
                readiness="action_required",
            )

    try:
        receipt = _receipt_reference(workspace, job_dir, outcome)
    except UserMutationError as exc:
        return user_mutation_error_response(
            operation,
            exc,
            mutation_id=outcome.mutation_id,
        )
    artifacts = projection.artifacts
    if receipt is not None and not any(
        existing.kind == receipt.kind
        and existing.path == receipt.path
        and existing.opaque_id == receipt.opaque_id
        for existing in artifacts
    ):
        artifacts = (*artifacts, receipt)
    warnings = list(projection.warnings)
    actions = projection.next_actions
    consents = projection.required_consents
    readiness = projection.readiness
    if outcome.status == "committed_receipt_pending":
        warnings.append("The user-owned value was committed, but its privacy-safe receipt still requires recovery.")
        consent = _recovery_consent()
        consents = (consent,)
        readiness = "action_required"
        actions = (
            NextAction(
                id="user_mutation.recover",
                label="Complete the accepted user-owned mutation receipt",
                requires_consent=True,
                consent_ids=[consent.id],
            ),
        )

    extensions = {
        **projection.extension_dict(),
        "canisend.mutation_status": outcome.status,
        "canisend.mutation_changed": outcome.changed,
    }
    if outcome.mutation_id is not None:
        extensions["canisend.mutation_id"] = outcome.mutation_id
    return success_response(
        operation=operation,
        workflow=WorkflowSnapshotReference(phase="unknown", readiness=readiness),
        artifacts=list(artifacts),
        missing_fields=list(projection.missing_fields),
        required_consents=list(consents),
        warnings=warnings,
        blockers=list(projection.blockers),
        next_actions=list(actions),
        extensions=extensions,
    )


def corrections_agent_projection(
    workspace: Path,
    job_dir: Path,
    snapshot: UserArtifactSnapshot | None,
    *,
    expose_update_action: bool = True,
) -> UserMutationAgentProjection:
    audit = inspect_current_artifact_mutation(workspace, job_dir, "corrections")
    base = _corrections_agent_projection_without_audit(
        workspace,
        job_dir,
        snapshot,
        expose_update_action=expose_update_action,
    )
    blocked = _mutation_audit_projection(
        workspace,
        job_dir,
        "corrections",
        snapshot,
        audit,
        base,
    )
    if blocked is not None:
        return blocked
    return _with_mutation_audit(
        base,
        audit,
    )


def _corrections_agent_projection_without_audit(
    workspace: Path,
    job_dir: Path,
    snapshot: UserArtifactSnapshot | None,
    *,
    expose_update_action: bool = True,
) -> UserMutationAgentProjection:
    artifact = _user_artifact_reference(
        workspace,
        job_dir,
        "corrections",
        snapshot,
    )
    if snapshot is None:
        consent = _write_consent("corrections")
        return UserMutationAgentProjection(
            readiness="action_required",
            artifacts=(artifact,),
            missing_fields=(CONFIRMED_CORRECTIONS_PATH,),
            required_consents=(consent,),
            next_actions=(
                NextAction(
                    id="criteria.corrections_initialize",
                    label="Initialize the user-owned corrections record",
                    requires_consent=True,
                    consent_ids=[consent.id],
                ),
            ),
            extensions=(
                ("canisend.user_artifact", "corrections"),
                ("canisend.user_artifact_state", "missing"),
                ("canisend.user_artifact_revision", None),
            ),
        )

    consents: tuple[ConsentRequirement, ...] = ()
    actions: tuple[NextAction, ...] = ()
    if expose_update_action:
        consent = _write_consent("corrections")
        consents = (consent,)
        actions = (
            NextAction(
                id="criteria.corrections_update",
                label="Apply one scoped correction with the current revision and hash",
                requires_consent=True,
                consent_ids=[consent.id],
            ),
        )
    return UserMutationAgentProjection(
        readiness="ready_for_next_stage",
        artifacts=(artifact,),
        required_consents=consents,
        next_actions=actions,
        extensions=(
            ("canisend.user_artifact", "corrections"),
            ("canisend.user_artifact_state", "current"),
            ("canisend.user_artifact_revision", snapshot.revision),
        ),
    )


def decision_agent_projection(
    workspace: Path,
    job_dir: Path,
    inspection: ApplicationDecisionInspection,
    *,
    expose_update_action: bool = True,
) -> UserMutationAgentProjection:
    audit = inspect_current_artifact_mutation(workspace, job_dir, "decision")
    base = _decision_agent_projection_without_audit(
        workspace,
        job_dir,
        inspection,
        expose_update_action=expose_update_action,
    )
    blocked = _mutation_audit_projection(
        workspace,
        job_dir,
        "decision",
        inspection.snapshot,
        audit,
        base,
    )
    if blocked is not None:
        return blocked
    return _with_mutation_audit(
        base,
        audit,
    )


def _decision_agent_projection_without_audit(
    workspace: Path,
    job_dir: Path,
    inspection: ApplicationDecisionInspection,
    *,
    expose_update_action: bool = True,
) -> UserMutationAgentProjection:
    snapshot = inspection.snapshot
    artifact = _user_artifact_reference(workspace, job_dir, "decision", snapshot)
    if snapshot is None:
        if inspection.reason != "user_input.not_initialized":
            return UserMutationAgentProjection(
                readiness="blocked",
                blockers=("The user-owned decision cannot be inspected safely.",),
                next_actions=(
                    NextAction(
                        id="decision.review_file",
                        label="Review the existing application decision file",
                    ),
                ),
                extensions=(
                    ("canisend.user_artifact", "decision"),
                    ("canisend.user_artifact_state", "unavailable"),
                    ("canisend.decision_basis_status", "unavailable"),
                    ("canisend.decision_reason", _public_reason(inspection.reason)),
                ),
            )
        consent = _write_consent("decision")
        return UserMutationAgentProjection(
            readiness="action_required",
            artifacts=(artifact,),
            missing_fields=(APPLICATION_DECISION_PATH,),
            required_consents=(consent,),
            blockers=("An explicit user-owned application decision is required.",),
            next_actions=(
                NextAction(
                    id="decision.initialize",
                    label="Initialize the user-owned application decision",
                    requires_consent=True,
                    consent_ids=[consent.id],
                ),
            ),
            extensions=(
                ("canisend.user_artifact", "decision"),
                ("canisend.user_artifact_state", "missing"),
                ("canisend.user_artifact_revision", None),
                ("canisend.decision_value", None),
                ("canisend.decision_basis_status", "unavailable"),
                ("canisend.decision_reason", "user_input.not_initialized"),
            ),
        )

    assert isinstance(snapshot.model, ApplicationDecisionV1)
    decision = snapshot.model
    extensions: tuple[tuple[str, JsonScalar], ...] = (
        ("canisend.user_artifact", "decision"),
        ("canisend.user_artifact_revision", snapshot.revision),
        ("canisend.decision_value", decision.decision),
        ("canisend.decision_basis_status", inspection.basis_status),
        ("canisend.decision_reason", _public_reason(inspection.reason)),
    )
    if decision.decision == "undecided":
        return _decision_review_projection(
            artifact,
            extensions=(*extensions, ("canisend.user_artifact_state", "undecided")),
            blocker="The application decision remains explicitly undecided.",
            label="Record an explicit apply, hold, or skip decision",
            expose_update_action=expose_update_action,
        )
    if inspection.basis_status != "current":
        return _decision_review_projection(
            artifact,
            extensions=(*extensions, ("canisend.user_artifact_state", "review_required")),
            blocker="The accepted decision basis changed and requires explicit reconfirmation.",
            label="Reconfirm the preserved decision against the current criteria and matches",
            expose_update_action=expose_update_action,
            warning="The accepted application decision was preserved, but its basis is no longer current.",
        )
    if decision.decision == "apply":
        return UserMutationAgentProjection(
            readiness="ready_for_next_stage",
            artifacts=(artifact,),
            extensions=(*extensions, ("canisend.user_artifact_state", "current")),
        )
    # A current hold or skip is an intentional user-owned stop. Do not invent a
    # downstream package, brief, or submission action.
    return UserMutationAgentProjection(
        readiness="review_required",
        artifacts=(artifact,),
        extensions=(*extensions, ("canisend.user_artifact_state", "current")),
    )


def _mutation_audit_projection(
    workspace: Path,
    job_dir: Path,
    artifact: UserArtifactKind,
    snapshot: UserArtifactSnapshot | None,
    audit: CurrentArtifactMutationAudit,
    base: UserMutationAgentProjection,
) -> UserMutationAgentProjection | None:
    if audit.status not in {"promotion_pending", "receipt_pending", "conflict"}:
        return None

    reference = _user_artifact_reference(workspace, job_dir, artifact, snapshot)
    extensions: list[tuple[str, JsonScalar]] = [
        *base.extensions,
        ("canisend.user_artifact_state", audit.status),
        (
            "canisend.user_artifact_revision",
            snapshot.revision if snapshot is not None else None,
        ),
        ("canisend.mutation_audit_status", audit.status),
    ]
    actions: tuple[NextAction, ...]
    consents: tuple[ConsentRequirement, ...] = ()
    references = [reference]
    pending = audit.status in {"promotion_pending", "receipt_pending"}
    if pending and audit.mutation_id is not None:
        recovery = _recovery_consent()
        consents = (recovery,)
        actions = (
            NextAction(
                id="user_mutation.recover",
                label="Recover the accepted user-owned mutation",
                requires_consent=True,
                consent_ids=[recovery.id],
            ),
        )
        extensions.append(("canisend.mutation_id", audit.mutation_id))
        references.append(
            ArtifactReference(
                kind="user_mutation_receipt",
                path=_workspace_artifact_path(
                    workspace,
                    job_dir,
                    f"workflow/user-mutations/events/{audit.mutation_id}/receipt.json",
                ),
                exists=False,
                privacy_tier=1,
                trust_level="validated",
                media_type="application/json",
            )
        )
    else:
        actions = (
            NextAction(
                id="user_mutation.review_controls",
                label="Review and coordinate the conflicting private mutation controls manually",
            ),
        )
        if audit.mutation_id is not None:
            extensions.append(("canisend.mutation_id", audit.mutation_id))
    return UserMutationAgentProjection(
        readiness="action_required" if pending else "blocked",
        artifacts=tuple(references),
        required_consents=consents,
        warnings=(
            ("A previously accepted user-owned mutation requires recovery.",)
            if pending
            else (
                (f"Conflicting mutation ID: {audit.mutation_id}.",)
                if audit.mutation_id is not None
                else ()
            )
        ),
        blockers=(
            ()
            if pending
            else ("The durable user-owned mutation controls conflict with the current artifact.",)
        ),
        next_actions=actions,
        extensions=tuple(extensions),
    )


def _with_mutation_audit(
    projection: UserMutationAgentProjection,
    audit: CurrentArtifactMutationAudit,
) -> UserMutationAgentProjection:
    return UserMutationAgentProjection(
        readiness=projection.readiness,
        artifacts=projection.artifacts,
        missing_fields=projection.missing_fields,
        required_consents=projection.required_consents,
        warnings=projection.warnings,
        blockers=projection.blockers,
        next_actions=projection.next_actions,
        extensions=(
            *projection.extensions,
            ("canisend.mutation_audit_status", audit.status),
        ),
    )


def user_mutation_error_response(
    operation: str,
    error: UserMutationError,
    *,
    mutation_id: str | None = None,
) -> AgentResponse:
    candidate_id = mutation_id or error.mutation_id
    mutation_id = (
        candidate_id
        if candidate_id is not None and _MUTATION_ID_RE.fullmatch(candidate_id)
        else None
    )
    code = error.code if error.code in KNOWN_AGENT_ERROR_CODES else "operation.failed"
    consent = _operation_consent(operation) if code == "user_input.consent_required" else None
    actions: list[NextAction] = []
    if code == "user_input.conflict":
        actions.append(
            NextAction(
                id=(
                    "criteria.corrections_status"
                    if operation.startswith("criteria.corrections")
                    else "decision.status"
                ),
                label="Read the current user-owned revision and hash before retrying",
            )
        )
    elif code == "user_input.recovery_required" and mutation_id is not None:
        recovery = _recovery_consent()
        consent = recovery
        actions.append(
            NextAction(
                id="user_mutation.recover",
                label="Recover the accepted user-owned mutation",
                requires_consent=True,
                consent_ids=[recovery.id],
            )
        )
    elif consent is not None:
        actions.append(
            NextAction(
                id=operation,
                label="Retry only after explicit confirmation of the user-owned write",
                requires_consent=True,
                consent_ids=[consent.id],
            )
        )
    return error_response(
        operation=operation,
        code=code,
        message=_public_error_message(code),
        hint=(
            f"Recover with mutation ID {mutation_id}."
            if code == "user_input.recovery_required" and mutation_id is not None
            else None
        ),
        retryable=code
        in {
            "user_input.conflict",
            "user_input.store_failed",
            "user_input.recovery_required",
        },
        workflow=WorkflowSnapshotReference(phase="unknown", readiness="blocked"),
        required_consents=[consent] if consent is not None else [],
        next_actions=actions,
        extensions={"canisend.mutation_id": mutation_id}
        if mutation_id is not None
        else {},
    )


def _projection_response(operation: str, projection: UserMutationAgentProjection) -> AgentResponse:
    return success_response(
        operation=operation,
        workflow=WorkflowSnapshotReference(phase="unknown", readiness=projection.readiness),
        artifacts=list(projection.artifacts),
        missing_fields=list(projection.missing_fields),
        required_consents=list(projection.required_consents),
        warnings=list(projection.warnings),
        blockers=list(projection.blockers),
        next_actions=list(projection.next_actions),
        extensions=projection.extension_dict(),
    )


def _decision_review_projection(
    artifact: ArtifactReference,
    *,
    extensions: tuple[tuple[str, JsonScalar], ...],
    blocker: str,
    label: str,
    expose_update_action: bool,
    warning: str | None = None,
) -> UserMutationAgentProjection:
    if not expose_update_action:
        return UserMutationAgentProjection(
            readiness="review_required",
            artifacts=(artifact,),
            warnings=((warning,) if warning else ()),
            blockers=(blocker,),
            extensions=extensions,
        )
    consent = _write_consent("decision")
    return UserMutationAgentProjection(
        readiness="review_required",
        artifacts=(artifact,),
        required_consents=(consent,),
        warnings=((warning,) if warning else ()),
        blockers=(blocker,),
        next_actions=(
            NextAction(
                id="decision.update",
                label=label,
                requires_consent=True,
                consent_ids=[consent.id],
            ),
        ),
        extensions=extensions,
    )


def _replace_actions(
    projection: UserMutationAgentProjection,
    actions: tuple[NextAction, ...],
    *,
    readiness: Readiness | None = None,
) -> UserMutationAgentProjection:
    return UserMutationAgentProjection(
        readiness=readiness or projection.readiness,
        artifacts=projection.artifacts,
        missing_fields=projection.missing_fields,
        warnings=projection.warnings,
        blockers=projection.blockers,
        next_actions=actions,
        extensions=projection.extensions,
    )


def _user_artifact_reference(
    workspace: Path,
    job_dir: Path,
    artifact: UserArtifactKind,
    snapshot: UserArtifactSnapshot | None,
) -> ArtifactReference:
    relative = _workspace_artifact_path(
        workspace,
        job_dir,
        snapshot.relative_path if snapshot is not None else _artifact_path(artifact),
    )
    return ArtifactReference(
        kind="confirmed_corrections" if artifact == "corrections" else "application_decision",
        path=relative,
        exists=snapshot is not None,
        sha256=snapshot.sha256 if snapshot is not None else None,
        privacy_tier=2,
        trust_level="trusted_local",
        media_type="application/yaml",
    )


def _receipt_reference(
    workspace: Path,
    job_dir: Path,
    outcome: MutationOutcome,
) -> ArtifactReference | None:
    if outcome.mutation_id is None:
        return None
    relative = (
        f"workflow/user-mutations/events/{outcome.mutation_id}/receipt.json"
    )
    if outcome.receipt_path is None:
        return ArtifactReference(
            kind="user_mutation_receipt",
            path=_workspace_artifact_path(workspace, job_dir, relative),
            exists=False,
            privacy_tier=1,
            trust_level="validated",
            media_type="application/json",
        )
    try:
        inspection = inspect_user_mutation(
            workspace,
            job_dir,
            outcome.mutation_id,
        )
        receipt = read_safe_bytes(job_dir, relative)
        receipt_model = UserMutationReceiptV1.model_validate(
            load_strict_json(receipt.data)
        )
    except (InvalidUserFileError, UnsafeUserFileError, ValidationError) as exc:
        raise UserMutationError(
            "user_input.recovery_required",
            "The committed mutation receipt could not be reread safely.",
        ) from exc
    if (
        inspection.status != "committed"
        or inspection.receipt is None
        or receipt_model != inspection.receipt
    ):
        raise UserMutationError(
            "user_input.recovery_required",
            "The committed mutation receipt no longer matches its durable claim.",
        )
    return ArtifactReference(
        kind="user_mutation_receipt",
        path=_workspace_artifact_path(workspace, job_dir, relative),
        exists=True,
        sha256=receipt.sha256,
        privacy_tier=1,
        trust_level="validated",
        media_type="application/json",
    )


def _load_bounded_patch_mapping(path: Path) -> dict[str, object]:
    expanded = path.expanduser()
    if not expanded.name or expanded.name in {".", ".."}:
        raise UserMutationError(
            "user_input.unsafe_path",
            "The patch file path is unsafe.",
        )
    try:
        snapshot = read_safe_bytes(
            expanded.parent,
            expanded.name,
            max_bytes=PATCH_FILE_MAX_BYTES,
        )
    except UnsafeUserFileError as exc:
        raise UserMutationError(
            "user_input.unsafe_path",
            "The patch file is not one bounded unaliased regular file.",
        ) from exc
    try:
        return load_strict_yaml(snapshot.data)
    except InvalidUserFileError as exc:
        raise UserMutationError(
            "user_input.invalid",
            "The patch file is not valid strict bounded YAML.",
        ) from exc


def _write_consent(artifact: UserArtifactKind) -> ConsentRequirement:
    kind = "confirmed_corrections" if artifact == "corrections" else "application_decision"
    return ConsentRequirement(
        id=f"write-user-owned-{artifact}",
        purpose=f"Allow one explicit guarded update to the user-owned {artifact} file.",
        privacy_tier=2,
        artifact_kinds=[kind],
    )


def _recovery_consent() -> ConsentRequirement:
    return ConsentRequirement(
        id="recover-user-owned-mutation",
        purpose="Allow completion of one previously accepted user-owned mutation claim.",
        privacy_tier=2,
        artifact_kinds=["confirmed_corrections", "application_decision"],
    )


def _operation_consent(operation: str) -> ConsentRequirement | None:
    if operation.startswith("criteria.corrections"):
        return _write_consent("corrections")
    if operation.startswith("decision."):
        return _write_consent("decision")
    if operation == "user_mutation.recover":
        return _recovery_consent()
    return None


def _confirm_requires_rerun(workspace: Path, job_dir: Path) -> bool:
    try:
        inspection = inspect_stage_status(workspace, job_dir, stage="confirm")
    except StageRuntimeError:
        return True
    return (
        inspection.stage.status != "succeeded"
        or bool(inspection.reasons)
        or inspection.output_drift
    )


def _artifact_path(artifact: UserArtifactKind) -> str:
    return CONFIRMED_CORRECTIONS_PATH if artifact == "corrections" else APPLICATION_DECISION_PATH


def _workspace_artifact_path(workspace: Path, job_dir: Path, relative: str) -> str:
    try:
        job_relative = job_dir.expanduser().resolve().relative_to(
            workspace.expanduser().resolve()
        )
    except ValueError as exc:
        raise UserMutationError(
            "user_input.unsafe_path",
            "The user-owned artifact is outside the selected workspace.",
        ) from exc
    return (job_relative / relative).as_posix()


def _public_reason(reason: str | None) -> str | None:
    return reason


def _public_error_message(code: str) -> str:
    return {
        "job.not_found": "The requested job directory does not exist.",
        "user_input.not_initialized": "The user-owned file has not been initialized.",
        "user_input.invalid": "The user-owned input is not a supported strict versioned record or scoped patch.",
        "user_input.unsafe_path": "The user-owned input path is not safe for this operation.",
        "user_input.consent_required": "Explicit confirmation of the user-owned write is required.",
        "user_input.conflict": "The user-owned file or recovery evidence conflicts with the expected baseline.",
        "user_input.dependency_not_current": "The required Decision Spine inputs are not current.",
        "user_input.store_failed": "The user-owned mutation could not be stored safely.",
        "user_input.recovery_required": "The accepted user-owned mutation requires explicit recovery.",
    }.get(code, "The user-owned mutation could not be completed.")
