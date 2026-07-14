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
from canisend.decision_models import ApplicationBriefV1, ApplicationDecisionV1
from canisend.package_readiness import PackageReviewDispositionsV1
from canisend.review_readiness import ReviewDispositionsV1
from canisend.stage_agent import stage_status_agent_response
from canisend.stage_runtime import StageRuntimeError, inspect_stage_status
from canisend.user_file_store import (
    InvalidUserFileError,
    UnsafeUserFileError,
    load_strict_yaml,
    load_strict_json,
    read_safe_bytes,
)
from canisend.user_mutations import (
    APPLICATION_BRIEF_PATH,
    APPLICATION_DECISION_PATH,
    CONFIRMED_CORRECTIONS_PATH,
    PACKAGE_REVIEW_DISPOSITIONS_PATH,
    REVIEW_DISPOSITIONS_PATH,
    RESEARCH_STATEMENT_REVIEW_DISPOSITIONS_PATH,
    ApplicationBriefInspection,
    ApplicationDecisionInspection,
    BriefPatch,
    CorrectionsPatch,
    CurrentArtifactMutationAudit,
    DecisionPatch,
    MutationOutcome,
    PackageReviewDispositionPatch,
    PackageReviewDispositionsInspection,
    ReviewDispositionPatch,
    ReviewDispositionsInspection,
    UserArtifactKind,
    UserArtifactSnapshot,
    UserMutationError,
    UserMutationReceiptV1,
    inspect_application_decision,
    inspect_application_brief,
    inspect_current_artifact_mutation,
    inspect_package_review_dispositions,
    inspect_review_dispositions,
    inspect_user_mutation,
    parse_brief_patch,
    parse_corrections_patch,
    parse_decision_patch,
    parse_package_review_disposition_patch,
    parse_review_disposition_patch,
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


def load_brief_patch_file(path: Path) -> BriefPatch:
    return parse_brief_patch(_load_bounded_patch_mapping(path))


def load_review_disposition_patch_file(path: Path) -> ReviewDispositionPatch:
    return parse_review_disposition_patch(_load_bounded_patch_mapping(path))


def load_package_review_disposition_patch_file(
    path: Path,
) -> PackageReviewDispositionPatch:
    return parse_package_review_disposition_patch(_load_bounded_patch_mapping(path))


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


def application_brief_status_agent_response(
    workspace: Path,
    job_dir: Path,
    inspection: ApplicationBriefInspection,
) -> AgentResponse:
    projection = brief_agent_projection(workspace, job_dir, inspection)
    if (
        inspection.snapshot is not None
        and inspection.basis_status == "current"
        and projection.extension_dict().get("canisend.mutation_audit_status")
        in {"untracked", "committed"}
    ):
        if _brief_plan_requires_refresh(workspace, job_dir):
            projection = _replace_actions(
                projection,
                (
                    NextAction(
                        id="stage.run_brief",
                        label="Generate or refresh the required-document plan",
                    ),
                ),
                readiness="action_required",
            )
        else:
            projection = _merge_current_brief_plan_status(
                workspace,
                job_dir,
                projection,
            )
    return _projection_response("brief.status", projection)


def review_dispositions_status_agent_response(
    workspace: Path,
    job_dir: Path,
    inspection: ReviewDispositionsInspection,
) -> AgentResponse:
    return _projection_response(
        "review.dispositions_status",
        review_dispositions_agent_projection(workspace, job_dir, inspection),
    )


def package_review_dispositions_status_agent_response(
    workspace: Path,
    job_dir: Path,
    inspection: PackageReviewDispositionsInspection,
) -> AgentResponse:
    return _projection_response(
        "package_review.dispositions_status",
        package_review_dispositions_agent_projection(
            workspace,
            job_dir,
            inspection,
        ),
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
        if projection.readiness == "ready_for_next_stage":
            projection = _chain_projection(
                projection,
                brief_agent_projection(
                    workspace,
                    job_dir,
                    inspect_application_brief(workspace, job_dir),
                ),
            )
    elif outcome.snapshot.artifact == "corrections":
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
    elif outcome.snapshot.artifact == "brief":
        projection = brief_agent_projection(
            workspace,
            job_dir,
            inspect_application_brief(workspace, job_dir),
        )
        projection = _replace_actions(
            projection,
            (
                NextAction(
                    id="stage.run_brief",
                    label="Refresh the required-document plan from the accepted Brief",
                ),
            ),
            readiness="action_required",
        )
    elif outcome.snapshot.artifact in {
        "review_dispositions",
        "research_statement_review_dispositions",
    }:
        assert isinstance(outcome.snapshot.model, ReviewDispositionsV1)
        projection = review_dispositions_agent_projection(
            workspace,
            job_dir,
            inspect_review_dispositions(
                workspace,
                job_dir,
                document_id=outcome.snapshot.model.document_id,
            ),
        )
    elif outcome.snapshot.artifact == "package_review_dispositions":
        assert isinstance(outcome.snapshot.model, PackageReviewDispositionsV1)
        projection = package_review_dispositions_agent_projection(
            workspace,
            job_dir,
            inspect_package_review_dispositions(workspace, job_dir),
        )
    else:  # pragma: no cover - guarded by UserArtifactKind
        raise ValueError("unsupported user-owned artifact kind")

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


def brief_agent_projection(
    workspace: Path,
    job_dir: Path,
    inspection: ApplicationBriefInspection,
    *,
    expose_update_action: bool = True,
) -> UserMutationAgentProjection:
    audit = inspect_current_artifact_mutation(workspace, job_dir, "brief")
    base = _brief_agent_projection_without_audit(
        workspace,
        job_dir,
        inspection,
        expose_update_action=expose_update_action,
    )
    blocked = _mutation_audit_projection(
        workspace,
        job_dir,
        "brief",
        inspection.snapshot,
        audit,
        base,
    )
    if blocked is not None:
        return blocked
    return _with_mutation_audit(base, audit)


def _brief_agent_projection_without_audit(
    workspace: Path,
    job_dir: Path,
    inspection: ApplicationBriefInspection,
    *,
    expose_update_action: bool,
) -> UserMutationAgentProjection:
    snapshot = inspection.snapshot
    artifact = _user_artifact_reference(workspace, job_dir, "brief", snapshot)
    if snapshot is None:
        if inspection.reason != "user_input.not_initialized":
            if inspection.reason is not None and (
                inspection.reason.startswith("decision.")
                or inspection.reason == "user_input.dependency_not_current"
            ):
                return UserMutationAgentProjection(
                    readiness="review_required",
                    artifacts=(artifact,),
                    blockers=(
                        "A current confirmed apply decision is required before initializing the application brief.",
                    ),
                    next_actions=(
                        NextAction(
                            id="decision.status",
                            label="Review the current user-owned application decision",
                        ),
                    ),
                    extensions=(
                        ("canisend.user_artifact", "brief"),
                        ("canisend.user_artifact_state", "missing"),
                        ("canisend.user_artifact_revision", None),
                        ("canisend.brief_basis_status", inspection.basis_status),
                        ("canisend.brief_reason", _public_reason(inspection.reason)),
                        ("canisend.brief_unresolved_field_count", 0),
                    ),
                )
            return UserMutationAgentProjection(
                readiness="blocked",
                artifacts=(artifact,),
                blockers=("The user-owned application brief cannot be inspected safely.",),
                next_actions=(
                    NextAction(
                        id="brief.review_file",
                        label="Review the existing application brief file",
                    ),
                ),
                extensions=(
                    ("canisend.user_artifact", "brief"),
                    ("canisend.user_artifact_state", "unavailable"),
                    ("canisend.brief_basis_status", inspection.basis_status),
                    ("canisend.brief_reason", _public_reason(inspection.reason)),
                    ("canisend.brief_unresolved_field_count", 0),
                ),
            )
        consent = _write_consent("brief")
        return UserMutationAgentProjection(
            readiness="action_required",
            artifacts=(artifact,),
            missing_fields=(APPLICATION_BRIEF_PATH,),
            required_consents=(consent,),
            blockers=("A user-owned application brief is required before document planning.",),
            next_actions=(
                NextAction(
                    id="brief.initialize",
                    label="Initialize the user-owned application brief",
                    requires_consent=True,
                    consent_ids=[consent.id],
                ),
            ),
            extensions=(
                ("canisend.user_artifact", "brief"),
                ("canisend.user_artifact_state", "missing"),
                ("canisend.user_artifact_revision", None),
                ("canisend.brief_basis_status", inspection.basis_status),
                ("canisend.brief_reason", _public_reason(inspection.reason)),
                ("canisend.brief_unresolved_field_count", 0),
            ),
        )

    assert isinstance(snapshot.model, ApplicationBriefV1)
    extensions: tuple[tuple[str, JsonScalar], ...] = (
        ("canisend.user_artifact", "brief"),
        ("canisend.user_artifact_revision", snapshot.revision),
        ("canisend.brief_basis_status", inspection.basis_status),
        ("canisend.brief_reason", _public_reason(inspection.reason)),
        ("canisend.brief_unresolved_field_count", len(inspection.unresolved_fields)),
    )
    if inspection.basis_status != "current":
        if inspection.reason == "brief.decision_changed" and expose_update_action:
            return _brief_review_projection(
                artifact,
                extensions=(*extensions, ("canisend.user_artifact_state", "review_required")),
                missing_fields=(),
                blocker="The preserved application brief must be reconfirmed against the current apply decision.",
                label="Reconfirm the preserved application brief",
            )
        return UserMutationAgentProjection(
            readiness="review_required",
            artifacts=(artifact,),
            warnings=("The application brief was preserved while its apply-decision basis became unavailable.",),
            blockers=("A current confirmed apply decision is required before changing the application brief.",),
            next_actions=(
                NextAction(
                    id="decision.status",
                    label="Review the current user-owned application decision",
                ),
            ),
            extensions=(*extensions, ("canisend.user_artifact_state", "review_required")),
        )
    if inspection.unresolved_fields:
        if not expose_update_action:
            return UserMutationAgentProjection(
                readiness="review_required",
                artifacts=(artifact,),
                missing_fields=tuple(
                    f"application_brief.{field}" for field in inspection.unresolved_fields
                ),
                blockers=("The application brief still contains unconfirmed fields.",),
                extensions=(*extensions, ("canisend.user_artifact_state", "unresolved")),
            )
        return _brief_review_projection(
            artifact,
            extensions=(*extensions, ("canisend.user_artifact_state", "unresolved")),
            missing_fields=tuple(
                f"application_brief.{field}" for field in inspection.unresolved_fields
            ),
            blocker="The application brief still contains unconfirmed fields.",
            label="Confirm one scoped application brief field",
        )
    if not expose_update_action:
        return UserMutationAgentProjection(
            readiness="ready_for_next_stage",
            artifacts=(artifact,),
            extensions=(*extensions, ("canisend.user_artifact_state", "current")),
        )
    consent = _write_consent("brief")
    return UserMutationAgentProjection(
        readiness="ready_for_next_stage",
        artifacts=(artifact,),
        required_consents=(consent,),
        next_actions=(
            NextAction(
                id="brief.update",
                label="Apply one scoped application brief or document-choice update",
                requires_consent=True,
                consent_ids=[consent.id],
            ),
        ),
        extensions=(*extensions, ("canisend.user_artifact_state", "current")),
    )


def review_dispositions_agent_projection(
    workspace: Path,
    job_dir: Path,
    inspection: ReviewDispositionsInspection,
) -> UserMutationAgentProjection:
    artifact = inspection.artifact
    audit = inspect_current_artifact_mutation(
        workspace,
        job_dir,
        artifact,
    )
    base = _review_dispositions_agent_projection_without_audit(
        workspace,
        job_dir,
        inspection,
    )
    blocked = _mutation_audit_projection(
        workspace,
        job_dir,
        artifact,
        inspection.snapshot,
        audit,
        base,
    )
    if blocked is not None:
        return blocked
    return _with_mutation_audit(base, audit)


def _review_dispositions_agent_projection_without_audit(
    workspace: Path,
    job_dir: Path,
    inspection: ReviewDispositionsInspection,
) -> UserMutationAgentProjection:
    snapshot = inspection.snapshot
    artifact_id = inspection.artifact
    artifact = _user_artifact_reference(
        workspace,
        job_dir,
        artifact_id,
        snapshot,
    )
    readiness = inspection.readiness
    extensions: tuple[tuple[str, JsonScalar], ...] = (
        ("canisend.user_artifact", artifact_id),
        ("canisend.user_artifact_revision", snapshot.revision if snapshot else None),
        ("canisend.document_id", inspection.document_id),
        ("canisend.document_kind", inspection.document_kind),
        ("canisend.review_disposition_basis_status", inspection.basis_status),
        ("canisend.review_disposition_reason", _public_reason(inspection.reason)),
        (
            "canisend.document_readiness",
            readiness.state if readiness is not None else "unavailable",
        ),
        (
            "canisend.review_accepted_finding_count",
            len(readiness.accepted_finding_ids) if readiness is not None else 0,
        ),
        (
            "canisend.review_revision_required_count",
            len(readiness.revision_required_finding_ids)
            if readiness is not None
            else 0,
        ),
        (
            "canisend.review_unresolved_finding_count",
            len(readiness.unresolved_finding_ids) if readiness is not None else 0,
        ),
        (
            "canisend.review_blocker_count",
            len(readiness.blocker_finding_ids) if readiness is not None else 0,
        ),
    )

    if readiness is None:
        return UserMutationAgentProjection(
            readiness="review_required",
            artifacts=(artifact,),
            blockers=("A current deterministic Review is required before finding disposition.",),
            next_actions=(
                NextAction(
                    id="workflow.stage_status",
                    label="Inspect the current Draft and Review stages",
                ),
            ),
            extensions=(*extensions, ("canisend.user_artifact_state", "unavailable")),
        )

    if readiness.state == "blocked":
        return UserMutationAgentProjection(
            readiness="blocked",
            artifacts=(artifact,),
            blockers=("Executable Review blockers require a new Draft and cannot be waived.",),
            next_actions=(
                NextAction(
                    id="review.resolve_blockers",
                    label="Resolve blocker findings and regenerate the structured Draft",
                ),
            ),
            extensions=(
                *extensions,
                (
                    "canisend.user_artifact_state",
                    "current" if snapshot is not None else "missing",
                ),
            ),
        )

    if snapshot is None:
        consent = _write_consent(artifact_id)
        return UserMutationAgentProjection(
            readiness="action_required",
            artifacts=(artifact,),
            missing_fields=(_artifact_path(artifact_id),),
            required_consents=(consent,),
            blockers=("Explicit user-owned Review dispositions are required.",),
            next_actions=(
                NextAction(
                    id="review.dispositions_initialize",
                    label="Initialize dispositions for the current Draft and Review",
                    requires_consent=True,
                    consent_ids=[consent.id],
                ),
            ),
            extensions=(*extensions, ("canisend.user_artifact_state", "missing")),
        )

    if inspection.basis_status != "current":
        consent = _write_consent(artifact_id)
        return UserMutationAgentProjection(
            readiness="review_required",
            artifacts=(artifact,),
            required_consents=(consent,),
            warnings=("The preserved dispositions belong to an older Draft or Review.",),
            blockers=("Review dispositions must be reset against the current Review.",),
            next_actions=(
                NextAction(
                    id="review.dispositions_update",
                    label="Reset dispositions for the current Review",
                    requires_consent=True,
                    consent_ids=[consent.id],
                ),
            ),
            extensions=(*extensions, ("canisend.user_artifact_state", "review_required")),
        )

    if readiness.state == "revision_required":
        return UserMutationAgentProjection(
            readiness="review_required",
            artifacts=(artifact,),
            blockers=("At least one current finding explicitly requires Draft revision.",),
            next_actions=(
                NextAction(
                    id="review.resolve_findings",
                    label="Revise the structured Draft for the selected findings",
                ),
            ),
            extensions=(*extensions, ("canisend.user_artifact_state", "current")),
        )

    if readiness.state == "review_required":
        read_consent = _read_review_findings_consent(inspection.document_kind)
        write_consent = _write_consent(artifact_id)
        return UserMutationAgentProjection(
            readiness="review_required",
            artifacts=(artifact,),
            required_consents=(read_consent, write_consent),
            blockers=("Every current non-blocker finding requires an explicit disposition.",),
            next_actions=(
                NextAction(
                    id="review.dispositions_update",
                    label="Inspect and disposition one current Review finding",
                    requires_consent=True,
                    consent_ids=[read_consent.id, write_consent.id],
                ),
            ),
            extensions=(*extensions, ("canisend.user_artifact_state", "current")),
        )

    return UserMutationAgentProjection(
        readiness="ready_for_next_stage",
        artifacts=(artifact,),
        extensions=(*extensions, ("canisend.user_artifact_state", "current")),
    )


def package_review_dispositions_agent_projection(
    workspace: Path,
    job_dir: Path,
    inspection: PackageReviewDispositionsInspection,
) -> UserMutationAgentProjection:
    artifact_id: UserArtifactKind = "package_review_dispositions"
    audit = inspect_current_artifact_mutation(workspace, job_dir, artifact_id)
    base = _package_review_dispositions_projection_without_audit(
        workspace,
        job_dir,
        inspection,
    )
    blocked = _mutation_audit_projection(
        workspace,
        job_dir,
        artifact_id,
        inspection.snapshot,
        audit,
        base,
    )
    if blocked is not None:
        return blocked
    return _with_mutation_audit(base, audit)


def _package_review_dispositions_projection_without_audit(
    workspace: Path,
    job_dir: Path,
    inspection: PackageReviewDispositionsInspection,
) -> UserMutationAgentProjection:
    artifact_id: UserArtifactKind = "package_review_dispositions"
    snapshot = inspection.snapshot
    artifact = _user_artifact_reference(
        workspace,
        job_dir,
        artifact_id,
        snapshot,
    )
    readiness = inspection.readiness
    extensions: tuple[tuple[str, JsonScalar], ...] = (
        ("canisend.user_artifact", artifact_id),
        ("canisend.user_artifact_revision", snapshot.revision if snapshot else None),
        ("canisend.package_review_disposition_basis_status", inspection.basis_status),
        ("canisend.package_review_disposition_reason", _public_reason(inspection.reason)),
        (
            "canisend.application_package_readiness",
            readiness.state if readiness is not None else "unavailable",
        ),
        (
            "canisend.package_required_document_count",
            len(readiness.required_document_ids) if readiness is not None else 0,
        ),
        (
            "canisend.package_reviewed_required_document_count",
            len(readiness.reviewed_required_document_ids)
            if readiness is not None
            else 0,
        ),
        (
            "canisend.package_optional_document_count",
            len(readiness.optional_document_ids) if readiness is not None else 0,
        ),
        (
            "canisend.package_accepted_finding_count",
            len(readiness.accepted_finding_ids) if readiness is not None else 0,
        ),
        (
            "canisend.package_revision_required_count",
            len(readiness.revision_required_finding_ids)
            if readiness is not None
            else 0,
        ),
        (
            "canisend.package_unresolved_finding_count",
            len(readiness.unresolved_finding_ids) if readiness is not None else 0,
        ),
        (
            "canisend.package_blocker_count",
            len(readiness.blocker_finding_ids) if readiness is not None else 0,
        ),
    )

    if readiness is None:
        return UserMutationAgentProjection(
            readiness="review_required",
            artifacts=(artifact,),
            blockers=(
                "A current deterministic aggregate Review is required before package disposition.",
            ),
            next_actions=(
                NextAction(
                    id="workflow.stage_status",
                    label="Inspect and run the aggregate package Review stage",
                ),
            ),
            extensions=(*extensions, ("canisend.user_artifact_state", "unavailable")),
        )

    if readiness.state == "blocked":
        return UserMutationAgentProjection(
            readiness="blocked",
            artifacts=(artifact,),
            blockers=(
                "Required-document or aggregate Review blockers must be resolved and cannot be waived.",
            ),
            next_actions=(
                NextAction(
                    id="package_review.resolve_blockers",
                    label="Resolve package blockers and rerun affected guarded stages",
                ),
            ),
            extensions=(
                *extensions,
                (
                    "canisend.user_artifact_state",
                    "current" if snapshot is not None else "missing",
                ),
            ),
        )

    if snapshot is None:
        consent = _write_consent(artifact_id)
        return UserMutationAgentProjection(
            readiness="action_required",
            artifacts=(artifact,),
            missing_fields=(PACKAGE_REVIEW_DISPOSITIONS_PATH,),
            required_consents=(consent,),
            blockers=("Explicit user-owned package finding decisions are required.",),
            next_actions=(
                NextAction(
                    id="package_review.dispositions_initialize",
                    label="Initialize decisions for the current aggregate Review",
                    requires_consent=True,
                    consent_ids=[consent.id],
                ),
            ),
            extensions=(*extensions, ("canisend.user_artifact_state", "missing")),
        )

    if inspection.basis_status != "current":
        consent = _write_consent(artifact_id)
        return UserMutationAgentProjection(
            readiness="review_required",
            artifacts=(artifact,),
            required_consents=(consent,),
            warnings=(
                "The preserved package decisions belong to an older aggregate Review.",
            ),
            blockers=(
                "Package dispositions must be reset against the current aggregate Review.",
            ),
            next_actions=(
                NextAction(
                    id="package_review.dispositions_update",
                    label="Reset decisions for the current aggregate Review",
                    requires_consent=True,
                    consent_ids=[consent.id],
                ),
            ),
            extensions=(
                *extensions,
                ("canisend.user_artifact_state", "review_required"),
            ),
        )

    if readiness.state == "revision_required":
        return UserMutationAgentProjection(
            readiness="review_required",
            artifacts=(artifact,),
            blockers=(
                "At least one package finding explicitly requires guarded document revision.",
            ),
            next_actions=(
                NextAction(
                    id="package_review.resolve_findings",
                    label="Revise targeted documents through guarded Draft candidates",
                ),
            ),
            extensions=(*extensions, ("canisend.user_artifact_state", "current")),
        )

    if readiness.state == "review_required":
        read_consent = _read_package_review_findings_consent()
        write_consent = _write_consent(artifact_id)
        return UserMutationAgentProjection(
            readiness="review_required",
            artifacts=(artifact,),
            required_consents=(read_consent, write_consent),
            blockers=(
                "Every current non-blocker package finding requires an explicit decision.",
            ),
            next_actions=(
                NextAction(
                    id="package_review.dispositions_update",
                    label="Inspect and disposition one current package finding",
                    requires_consent=True,
                    consent_ids=[read_consent.id, write_consent.id],
                ),
            ),
            extensions=(*extensions, ("canisend.user_artifact_state", "current")),
        )

    return UserMutationAgentProjection(
        readiness="ready_for_next_stage",
        artifacts=(artifact,),
        warnings=(
            "Application-package review is not rendering approval or proof of submission.",
        ),
        next_actions=(
            NextAction(
                id="package.check",
                label="Run the independent application-package quality gate",
            ),
        ),
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
    artifact: UserArtifactKind | None = None,
    mutation_id: str | None = None,
) -> AgentResponse:
    candidate_id = mutation_id or error.mutation_id
    mutation_id = (
        candidate_id
        if candidate_id is not None and _MUTATION_ID_RE.fullmatch(candidate_id)
        else None
    )
    code = error.code if error.code in KNOWN_AGENT_ERROR_CODES else "operation.failed"
    consent = (
        _operation_consent(operation, artifact=artifact)
        if code == "user_input.consent_required"
        else None
    )
    actions: list[NextAction] = []
    if code == "user_input.conflict":
        if operation == "user_mutation.recover":
            actions.append(
                NextAction(
                    id="user_mutation.review_controls",
                    label="Review and coordinate the conflicting private mutation controls manually",
                )
            )
        else:
            status_operation = {
                "corrections": "criteria.corrections_status",
                "decision": "decision.status",
                "brief": "brief.status",
                "review_dispositions": "review.dispositions_status",
                "package_review_dispositions": (
                    "package_review.dispositions_status"
                ),
            }[
                "corrections"
                if operation.startswith("criteria.corrections")
                else "package_review_dispositions"
                if operation.startswith("package_review.dispositions")
                else "review_dispositions"
                if operation.startswith("review.dispositions")
                else "brief"
                if operation.startswith("brief.")
                else "decision"
            ]
            actions.append(
                NextAction(
                    id=status_operation,
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
    elif code in {
        "user_input.document_ambiguous",
        "user_input.document_not_found",
    }:
        actions.append(
            NextAction(
                id="documents.status",
                label="Inspect document targets and select one stable document ID",
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


def _brief_review_projection(
    artifact: ArtifactReference,
    *,
    extensions: tuple[tuple[str, JsonScalar], ...],
    missing_fields: tuple[str, ...],
    blocker: str,
    label: str,
) -> UserMutationAgentProjection:
    consent = _write_consent("brief")
    return UserMutationAgentProjection(
        readiness="review_required",
        artifacts=(artifact,),
        missing_fields=missing_fields,
        required_consents=(consent,),
        blockers=(blocker,),
        next_actions=(
            NextAction(
                id="brief.update",
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


def _chain_projection(
    current: UserMutationAgentProjection,
    downstream: UserMutationAgentProjection,
) -> UserMutationAgentProjection:
    artifacts = list(current.artifacts)
    for candidate in downstream.artifacts:
        if not any(
            item.kind == candidate.kind
            and item.path == candidate.path
            and item.opaque_id == candidate.opaque_id
            for item in artifacts
        ):
            artifacts.append(candidate)
    return UserMutationAgentProjection(
        readiness=downstream.readiness,
        artifacts=tuple(artifacts),
        missing_fields=downstream.missing_fields,
        required_consents=downstream.required_consents,
        warnings=current.warnings + downstream.warnings,
        blockers=downstream.blockers,
        next_actions=downstream.next_actions,
        extensions=tuple(
            {
                **current.extension_dict(),
                **downstream.extension_dict(),
            }.items()
        ),
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
    kind = {
        "corrections": "confirmed_corrections",
        "decision": "application_decision",
        "brief": "application_brief",
        "review_dispositions": "review_dispositions",
        "research_statement_review_dispositions": (
            "research_statement_review_dispositions"
        ),
        "package_review_dispositions": "package_review_dispositions",
    }[artifact]
    return ArtifactReference(
        kind=kind,
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
    kind = {
        "corrections": "confirmed_corrections",
        "decision": "application_decision",
        "brief": "application_brief",
        "review_dispositions": "review_dispositions",
        "research_statement_review_dispositions": (
            "research_statement_review_dispositions"
        ),
        "package_review_dispositions": "package_review_dispositions",
    }[artifact]
    return ConsentRequirement(
        id=f"write-user-owned-{artifact}",
        purpose=f"Allow one explicit guarded update to the user-owned {artifact} file.",
        privacy_tier=2,
        artifact_kinds=[kind],
    )


def _read_review_findings_consent(document_kind: str) -> ConsentRequirement:
    artifact_kind = (
        "review_findings"
        if document_kind == "cover_letter"
        else "research_statement_review_findings"
    )
    return ConsentRequirement(
        id="read-private-review-findings",
        purpose="Allow inspection of private Review finding bodies before disposition.",
        privacy_tier=2,
        artifact_kinds=[artifact_kind],
    )


def _read_package_review_findings_consent() -> ConsentRequirement:
    return ConsentRequirement(
        id="read-private-package-review-findings",
        purpose="Allow inspection of private aggregate Review finding bodies before disposition.",
        privacy_tier=2,
        artifact_kinds=["package_review_findings"],
    )


def _recovery_consent() -> ConsentRequirement:
    return ConsentRequirement(
        id="recover-user-owned-mutation",
        purpose="Allow completion of one previously accepted user-owned mutation claim.",
        privacy_tier=2,
        artifact_kinds=[
            "confirmed_corrections",
            "application_decision",
            "application_brief",
            "review_dispositions",
            "research_statement_review_dispositions",
            "package_review_dispositions",
        ],
    )


def _operation_consent(
    operation: str,
    *,
    artifact: UserArtifactKind | None = None,
) -> ConsentRequirement | None:
    if operation.startswith("criteria.corrections"):
        return _write_consent("corrections")
    if operation.startswith("decision."):
        return _write_consent("decision")
    if operation.startswith("brief."):
        return _write_consent("brief")
    if operation.startswith("review.dispositions"):
        return _write_consent(artifact or "review_dispositions")
    if operation.startswith("package_review.dispositions"):
        return _write_consent("package_review_dispositions")
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


def _brief_plan_requires_refresh(workspace: Path, job_dir: Path) -> bool:
    try:
        inspection = inspect_stage_status(workspace, job_dir, stage="brief")
    except StageRuntimeError:
        return True
    return (
        inspection.stage.status != "succeeded"
        or bool(inspection.reasons)
        or inspection.output_drift
    )


def _merge_current_brief_plan_status(
    workspace: Path,
    job_dir: Path,
    projection: UserMutationAgentProjection,
) -> UserMutationAgentProjection:
    try:
        inspection = inspect_stage_status(workspace, job_dir, stage="brief")
        response = stage_status_agent_response(workspace, job_dir, inspection)
    except StageRuntimeError:
        return projection
    if response.workflow is None or response.workflow.readiness != "blocked":
        return projection
    safe_plan_extensions = {
        key: value
        for key, value in response.extensions.items()
        if key
        in {
            "canisend.required_document_count",
            "canisend.unresolved_document_count",
            "canisend.blocking_document_count",
            "canisend.orphaned_document_choice_count",
            "canisend.unresolved_brief_field_count",
            "canisend.document_plan_blocker_count",
            "canisend.document_plan_primary_blocker",
            "canisend.document_requirements_state",
            "canisend.document_requirements_basis_sha256",
        }
    }
    return UserMutationAgentProjection(
        readiness="blocked",
        artifacts=projection.artifacts,
        missing_fields=projection.missing_fields,
        required_consents=projection.required_consents,
        warnings=projection.warnings,
        blockers=(
            *projection.blockers,
            "The current required-document plan contains unresolved blockers.",
        ),
        next_actions=projection.next_actions,
        extensions=tuple(
            {
                **projection.extension_dict(),
                **safe_plan_extensions,
                "canisend.document_plan_readiness": "blocked",
            }.items()
        ),
    )


def _artifact_path(artifact: UserArtifactKind) -> str:
    return {
        "corrections": CONFIRMED_CORRECTIONS_PATH,
        "decision": APPLICATION_DECISION_PATH,
        "brief": APPLICATION_BRIEF_PATH,
        "review_dispositions": REVIEW_DISPOSITIONS_PATH,
        "research_statement_review_dispositions": (
            RESEARCH_STATEMENT_REVIEW_DISPOSITIONS_PATH
        ),
        "package_review_dispositions": PACKAGE_REVIEW_DISPOSITIONS_PATH,
    }[artifact]


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
        "user_input.document_ambiguous": "More than one document target is available; select one stable document ID.",
        "user_input.document_not_found": "The selected document has no available Review disposition target.",
        "user_input.store_failed": "The user-owned mutation could not be stored safely.",
        "user_input.recovery_required": "The accepted user-owned mutation requires explicit recovery.",
    }.get(code, "The user-owned mutation could not be completed.")
