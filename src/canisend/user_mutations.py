from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
import json
from pathlib import Path
import re
import stat
from typing import Annotated, Any, Literal, TypeAlias
from uuid import uuid4

from pydantic import (
    BaseModel,
    ConfigDict,
    Field,
    StrictBool,
    StrictInt,
    TypeAdapter,
    ValidationError,
    model_validator,
)

from canisend.decision_models import (
    ApplicationDecisionV1,
    ConfirmedCorrectionsV1,
    CriteriaCatalogV1,
    CriteriaExtractionConfirmationV1,
    CriterionCorrectionV1,
    CriterionMatchesV1,
    DecisionBasisV1,
    JSON_SCHEMA_DIALECT,
    MAX_USER_REVISION,
    SCHEMA_BASE_ID,
    UserControlTimestamp,
    UserRevision,
)
from canisend.stage_runtime import StageRuntimeError, inspect_stage_status
from canisend.stage_store import (
    StageStoreError,
    UnsafeStagePathError,
    resolve_job_relative_path,
)
from canisend.stages.confirm_stage import (
    criteria_extraction_basis_sha256,
    criterion_source_sha256,
)
from canisend.user_file_store import (
    InvalidUserFileError,
    SafeFileSnapshot,
    UnsafeUserFileError,
    UserFileConflictError,
    UserFileStoreError,
    create_safe_file,
    dump_yaml_mapping,
    has_interrupted_safe_publication,
    load_strict_json,
    load_strict_yaml,
    read_optional_safe_bytes,
    read_safe_bytes,
    repair_interrupted_safe_publication,
    replace_safe_file,
    write_safe_immutable_file,
)


USER_MUTATION_SCHEMA_VERSION = "1.0.0"
CONFIRMED_CORRECTIONS_PATH = "confirmed_corrections.yaml"
APPLICATION_DECISION_PATH = "application_decision.yaml"

UserArtifactKind = Literal["corrections", "decision"]
DecisionBasisStatus = Literal["current", "review_required", "unavailable"]
MutationStatus = Literal["committed", "committed_receipt_pending", "reused"]
RecoveryStatus = Literal[
    "committed",
    "receipt_pending",
    "promotion_pending",
    "conflict",
]
CurrentMutationAuditStatus = Literal[
    "untracked",
    "committed",
    "promotion_pending",
    "receipt_pending",
    "conflict",
]

_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
_MUTATION_ID_RE = re.compile(r"^mutation_[0-9a-f]{32}$")
_CRITERION_ID_RE = re.compile(r"^criterion_[0-9a-f]{32}$")

USER_MUTATION_ERROR_CODES = frozenset(
    {
        "job.not_found",
        "user_input.not_initialized",
        "user_input.invalid",
        "user_input.unsafe_path",
        "user_input.consent_required",
        "user_input.conflict",
        "user_input.dependency_not_current",
        "user_input.store_failed",
        "user_input.recovery_required",
    }
)


class UserMutationError(RuntimeError):
    """A stable non-body-bearing failure at the user-owned mutation boundary."""

    def __init__(
        self,
        code: str,
        message: str,
        *,
        mutation_id: str | None = None,
    ) -> None:
        if code not in USER_MUTATION_ERROR_CODES:
            raise ValueError("unsupported user mutation error code")
        if mutation_id is not None and _MUTATION_ID_RE.fullmatch(mutation_id) is None:
            raise ValueError("invalid recovery mutation id")
        super().__init__(message)
        self.code = code
        self.mutation_id = mutation_id


class _MutationModel(BaseModel):
    model_config = ConfigDict(
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
    )


class ConfirmCriterionPatch(_MutationModel):
    operation: Literal["confirm_criterion"] = "confirm_criterion"
    criterion_id: str = Field(pattern=r"^criterion_[0-9a-f]{32}$")
    source_occurrence: Annotated[StrictInt, Field(ge=1)] | None = None


class CorrectCriterionPatch(_MutationModel):
    operation: Literal["correct_criterion"] = "correct_criterion"
    criterion_id: str = Field(pattern=r"^criterion_[0-9a-f]{32}$")
    corrected_text: str = Field(min_length=1, max_length=200_000)
    source_occurrence: Annotated[StrictInt, Field(ge=1)] | None = None


class WithdrawCriterionPatch(_MutationModel):
    operation: Literal["withdraw_criterion"] = "withdraw_criterion"
    criterion_id: str = Field(pattern=r"^criterion_[0-9a-f]{32}$")


class ConfirmEmptyCriteriaPatch(_MutationModel):
    operation: Literal["confirm_empty"] = "confirm_empty"


class SetDecisionPatch(_MutationModel):
    operation: Literal["set_decision"] = "set_decision"
    decision: Literal["apply", "hold", "skip"]
    rationale_mode: Literal["keep", "set", "clear"] = "keep"
    rationale: str | None = Field(default=None, max_length=200_000)

    @model_validator(mode="after")
    def _consistent_rationale(self) -> SetDecisionPatch:
        if self.rationale_mode == "set" and not self.rationale:
            raise ValueError("rationale_mode set requires rationale")
        if self.rationale_mode != "set" and self.rationale is not None:
            raise ValueError("rationale is accepted only with rationale_mode set")
        return self


class ResetDecisionPatch(_MutationModel):
    operation: Literal["reset_decision"] = "reset_decision"


CorrectionsPatch: TypeAlias = Annotated[
    ConfirmCriterionPatch
    | CorrectCriterionPatch
    | WithdrawCriterionPatch
    | ConfirmEmptyCriteriaPatch,
    Field(discriminator="operation"),
]
DecisionPatch: TypeAlias = Annotated[
    SetDecisionPatch | ResetDecisionPatch,
    Field(discriminator="operation"),
]
UserPatch: TypeAlias = CorrectionsPatch | DecisionPatch

_CORRECTIONS_PATCH_ADAPTER = TypeAdapter(CorrectionsPatch)
_DECISION_PATCH_ADAPTER = TypeAdapter(DecisionPatch)


class UserMutationClaimV1(_MutationModel):
    schema_version: Literal["1.0.0"] = USER_MUTATION_SCHEMA_VERSION
    mutation_id: str = Field(pattern=r"^mutation_[0-9a-f]{32}$")
    job_id: str = Field(pattern=r"^[a-z0-9][a-z0-9_.-]*$")
    artifact: UserArtifactKind
    target_path: Literal[CONFIRMED_CORRECTIONS_PATH, APPLICATION_DECISION_PATH]
    expected_revision: UserRevision | None = None
    expected_sha256: str | None = Field(default=None, pattern=r"^[0-9a-f]{64}$")
    result_revision: UserRevision
    result_sha256: str = Field(pattern=r"^[0-9a-f]{64}$")
    candidate_path: str
    candidate_sha256: str = Field(pattern=r"^[0-9a-f]{64}$")
    claimed_at: UserControlTimestamp
    consent_confirmed: StrictBool

    @model_validator(mode="after")
    def _consistent_transition(self) -> UserMutationClaimV1:
        if self.consent_confirmed is not True:
            raise ValueError("a mutation claim requires explicit true consent")
        expected_path = _target_path(self.artifact)
        expected_candidate = _candidate_relative(self.mutation_id)
        if self.target_path != expected_path or self.candidate_path != expected_candidate:
            raise ValueError("mutation claim paths do not match artifact identity")
        absent = self.expected_revision is None and self.expected_sha256 is None
        if absent:
            if self.result_revision != 0:
                raise ValueError("an absent baseline must create revision zero")
        elif self.expected_revision is None or self.expected_sha256 is None:
            raise ValueError("expected revision and hash must appear together")
        elif self.result_revision != self.expected_revision + 1:
            raise ValueError("a mutation must advance exactly one revision")
        if self.candidate_sha256 != self.result_sha256:
            raise ValueError("candidate and result hashes must match")
        return self


class UserMutationReceiptV1(_MutationModel):
    model_config = ConfigDict(
        title="CanISendUserMutationReceiptV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
        json_schema_extra={
            "$schema": JSON_SCHEMA_DIALECT,
            "$id": f"{SCHEMA_BASE_ID}/user-mutation-receipt.schema.json",
            "allOf": [
                {
                    "if": {
                        "properties": {"artifact": {"const": "corrections"}},
                        "required": ["artifact"],
                    },
                    "then": {
                        "properties": {
                            "target_path": {"const": CONFIRMED_CORRECTIONS_PATH}
                        }
                    },
                    "else": {
                        "properties": {
                            "target_path": {"const": APPLICATION_DECISION_PATH}
                        }
                    },
                },
                {
                    "if": {
                        "properties": {
                            "expected_revision": {"not": {"type": "null"}}
                        },
                        "required": ["expected_revision"],
                    },
                    "then": {
                        "properties": {
                            "expected_sha256": {"not": {"type": "null"}}
                        },
                        "required": ["expected_sha256"],
                    },
                    "else": {
                        "properties": {
                            "expected_sha256": {"type": "null"},
                            "result_revision": {"const": 0},
                        }
                    },
                },
                {
                    "if": {
                        "properties": {
                            "expected_sha256": {"not": {"type": "null"}}
                        },
                        "required": ["expected_sha256"],
                    },
                    "then": {
                        "properties": {
                            "expected_revision": {"not": {"type": "null"}}
                        },
                        "required": ["expected_revision"],
                    },
                    "else": {
                        "properties": {"expected_revision": {"type": "null"}}
                    },
                },
            ],
        },
    )

    schema_version: Literal["1.0.0"] = USER_MUTATION_SCHEMA_VERSION
    mutation_id: str = Field(pattern=r"^mutation_[0-9a-f]{32}$")
    job_id: str = Field(pattern=r"^[a-z0-9][a-z0-9_.-]*$")
    artifact: UserArtifactKind
    target_path: Literal[CONFIRMED_CORRECTIONS_PATH, APPLICATION_DECISION_PATH]
    expected_revision: UserRevision | None = None
    expected_sha256: str | None = Field(default=None, pattern=r"^[0-9a-f]{64}$")
    result_revision: UserRevision
    result_sha256: str = Field(pattern=r"^[0-9a-f]{64}$")
    committed_at: UserControlTimestamp
    status: Literal["committed"] = "committed"

    @model_validator(mode="after")
    def _consistent_receipt(self) -> UserMutationReceiptV1:
        if self.target_path != _target_path(self.artifact):
            raise ValueError("mutation receipt target does not match artifact identity")
        absent = self.expected_revision is None and self.expected_sha256 is None
        if absent:
            if self.result_revision != 0:
                raise ValueError("an absent baseline must create revision zero")
        elif self.expected_revision is None or self.expected_sha256 is None:
            raise ValueError("expected revision and hash must appear together")
        elif self.result_revision != self.expected_revision + 1:
            raise ValueError("a mutation receipt must advance exactly one revision")
        return self


UserArtifactModel: TypeAlias = ConfirmedCorrectionsV1 | ApplicationDecisionV1


@dataclass(frozen=True)
class UserArtifactSnapshot:
    artifact: UserArtifactKind
    path: Path
    relative_path: str
    model: UserArtifactModel
    sha256: str
    revision: int
    raw_bytes: bytes
    interrupted_publication: bool = False


@dataclass(frozen=True)
class MutationOutcome:
    status: MutationStatus
    snapshot: UserArtifactSnapshot
    mutation_id: str | None
    claim_path: Path | None
    receipt_path: Path | None
    changed: bool


@dataclass(frozen=True)
class MutationInspection:
    status: RecoveryStatus
    claim: UserMutationClaimV1
    claim_path: Path
    receipt: UserMutationReceiptV1 | None = None
    receipt_path: Path | None = None


@dataclass(frozen=True)
class ApplicationDecisionInspection:
    snapshot: UserArtifactSnapshot | None
    basis_status: DecisionBasisStatus
    reason: str | None = None


@dataclass(frozen=True)
class CurrentArtifactMutationAudit:
    """Privacy-safe state of durable mutation controls for one current artifact."""

    status: CurrentMutationAuditStatus
    mutation_id: str | None = None


def parse_corrections_patch(value: object) -> CorrectionsPatch:
    try:
        return _CORRECTIONS_PATCH_ADAPTER.validate_python(value)
    except ValidationError as exc:
        raise UserMutationError(
            "user_input.invalid",
            "The corrections patch is not a supported scoped operation.",
        ) from exc


def parse_decision_patch(value: object) -> DecisionPatch:
    try:
        return _DECISION_PATCH_ADAPTER.validate_python(value)
    except ValidationError as exc:
        raise UserMutationError(
            "user_input.invalid",
            "The application decision patch is not a supported scoped operation.",
        ) from exc


def inspect_user_artifact(
    workspace: Path,
    job_dir: Path,
    artifact: UserArtifactKind,
) -> UserArtifactSnapshot | None:
    _, job = _mutation_paths(workspace, job_dir)
    return _load_artifact_snapshot(job, artifact, allow_missing=True)


def inspect_current_artifact_mutation(
    workspace: Path,
    job_dir: Path,
    artifact: UserArtifactKind,
) -> CurrentArtifactMutationAudit:
    """Audit current durable mutation controls without changing any file.

    Completed receipts are history once a later valid revision has superseded
    their result.  A receiptless claim, however, is never silently discarded:
    it is either recoverable from the expected/result state or reported as a
    conflict.  This makes a fresh process able to discover a lost response.
    """

    _, job = _mutation_paths(workspace, job_dir)
    target_invalid = False
    try:
        target = _load_artifact_snapshot(job, artifact, allow_missing=True)
    except UserMutationError:
        target = None
        target_invalid = True

    claim_dir_relative = f"workflow/user-mutations/claims/{artifact}"
    try:
        claim_paths = _safe_control_json_paths(job, claim_dir_relative)
    except UserMutationError:
        return CurrentArtifactMutationAudit(status="conflict")

    records: list[
        tuple[UserMutationClaimV1, UserMutationReceiptV1 | None, bool]
    ] = []
    invalid_controls = False
    for path in claim_paths:
        try:
            relative = path.relative_to(job).as_posix()
            claim = _load_claim(job, relative)
            if claim.artifact != artifact:
                raise _recovery_conflict(claim.mutation_id)
            receipt, _ = _load_receipt(
                job,
                claim.mutation_id,
                allow_missing=True,
            )
            if receipt is not None and not _receipt_matches_claim(receipt, claim):
                raise _recovery_conflict(claim.mutation_id)
            interrupted_control = any(
                (
                    has_interrupted_safe_publication(job, relative),
                    has_interrupted_safe_publication(
                        job,
                        _receipt_relative(claim.mutation_id),
                    ),
                )
            )
        except (UserMutationError, UnsafeUserFileError, ValueError):
            invalid_controls = True
            continue
        try:
            interrupted_control = interrupted_control or has_interrupted_safe_publication(
                job,
                claim.candidate_path,
            )
        except UnsafeUserFileError:
            # A receiptless candidate is validated below so the accepted claim
            # ID remains available in the conflict response. A completed
            # receipt does not require retained candidate bytes for validity.
            pass
        records.append((claim, receipt, interrupted_control))

    receiptless = [record for record in records if record[1] is None]
    if invalid_controls or len(receiptless) > 1:
        return CurrentArtifactMutationAudit(status="conflict")

    if receiptless:
        claim, _receipt, _interrupted_control = receiptless[0]
        if not target_invalid and _snapshot_matches_expected(target, claim):
            if not _claim_candidate_is_valid(job, claim):
                return CurrentArtifactMutationAudit(
                    status="conflict",
                    mutation_id=claim.mutation_id,
                )
            return CurrentArtifactMutationAudit(
                status="promotion_pending",
                mutation_id=claim.mutation_id,
            )
        if not target_invalid and _snapshot_matches_result(target, claim):
            return CurrentArtifactMutationAudit(
                status="receipt_pending",
                mutation_id=claim.mutation_id,
            )
        return CurrentArtifactMutationAudit(
            status="conflict",
            mutation_id=claim.mutation_id,
        )

    current_commits: list[tuple[UserMutationClaimV1, bool]] = []
    nonhistorical_conflicts: list[UserMutationClaimV1] = []
    for claim, receipt, interrupted_control in records:
        assert receipt is not None
        if not target_invalid and _snapshot_matches_result(target, claim):
            current_commits.append(
                (
                    claim,
                    interrupted_control
                    or bool(target is not None and target.interrupted_publication),
                )
            )
            continue
        # A valid, sequentially later user artifact makes this a completed
        # historical receipt.  Do not surface it as pending in fresh sessions.
        if (
            not target_invalid
            and target is not None
            and target.revision > claim.result_revision
        ):
            continue
        nonhistorical_conflicts.append(claim)

    if len(current_commits) == 1 and not nonhistorical_conflicts:
        current_claim, interrupted_publication = current_commits[0]
        return CurrentArtifactMutationAudit(
            status="receipt_pending" if interrupted_publication else "committed",
            mutation_id=current_claim.mutation_id,
        )
    if len(current_commits) > 1 or nonhistorical_conflicts or target_invalid:
        unique = (
            nonhistorical_conflicts[0].mutation_id
            if len(nonhistorical_conflicts) == 1 and not current_commits
            else None
        )
        return CurrentArtifactMutationAudit(status="conflict", mutation_id=unique)
    if target is not None and target.interrupted_publication:
        return CurrentArtifactMutationAudit(status="conflict")
    return CurrentArtifactMutationAudit(status="untracked")


def initialize_user_artifact(
    workspace: Path,
    job_dir: Path,
    artifact: UserArtifactKind,
    *,
    consent_confirmed: bool,
    mutation_id: str | None = None,
) -> MutationOutcome:
    _require_consent(consent_confirmed)
    _, job = _mutation_paths(workspace, job_dir)
    existing = _load_artifact_snapshot(job, artifact, allow_missing=True)
    if existing is not None:
        accepted = _load_baseline_claim(
            job,
            artifact,
            expected_revision=None,
            expected_sha256=None,
        )
        if accepted is not None and _snapshot_matches_result(existing, accepted[0]):
            claim, claim_path = accepted
            receipt, _receipt_path = _load_receipt(
                job,
                claim.mutation_id,
                allow_missing=True,
            )
            if receipt is None:
                return _finish_claim(job, claim, claim_path=claim_path)
        return MutationOutcome(
            status="reused",
            snapshot=existing,
            mutation_id=None,
            claim_path=None,
            receipt_path=None,
            changed=False,
        )
    accepted = _load_baseline_claim(
        job,
        artifact,
        expected_revision=None,
        expected_sha256=None,
    )
    if accepted is not None:
        raise _accepted_claim_requires_recovery(accepted[0].mutation_id)
    now = _utc_now()
    model: UserArtifactModel
    if artifact == "corrections":
        model = ConfirmedCorrectionsV1(
            job_id=job.name,
            revision=0,
            updated_at=now,
        )
    else:
        model = ApplicationDecisionV1(
            job_id=job.name,
            revision=0,
            updated_at=now,
        )
    return _commit_candidate(
        job,
        artifact=artifact,
        model=model,
        expected=None,
        mutation_id=_normalized_mutation_id(mutation_id),
    )


def apply_user_patch(
    workspace: Path,
    job_dir: Path,
    patch: UserPatch | dict[str, Any],
    *,
    expected_sha256: str,
    expected_revision: int,
    mutation_id: str | None = None,
    consent_confirmed: bool,
) -> MutationOutcome:
    _require_consent(consent_confirmed)
    _require_expected_baseline(expected_sha256, expected_revision)
    root, job = _mutation_paths(workspace, job_dir)
    artifact = _patch_artifact(patch)
    audit = inspect_current_artifact_mutation(root, job, artifact)
    if audit.status in {"promotion_pending", "receipt_pending", "conflict"}:
        raise UserMutationError(
            "user_input.recovery_required",
            "A prior accepted mutation must be recovered before another update.",
            mutation_id=audit.mutation_id,
        )
    current = _load_artifact_snapshot(job, artifact, allow_missing=False)
    assert current is not None
    accepted = _load_baseline_claim(
        job,
        artifact,
        expected_revision=expected_revision,
        expected_sha256=expected_sha256,
    )
    try:
        _compare_baseline(current, expected_sha256, expected_revision)
    except UserMutationError:
        if accepted is not None and _snapshot_matches_result(current, accepted[0]):
            raise _accepted_claim_requires_recovery(accepted[0].mutation_id)
        raise
    if accepted is not None:
        raise _accepted_claim_requires_recovery(accepted[0].mutation_id)
    normalized_id = _normalized_mutation_id(mutation_id)
    existing_candidate = _load_existing_candidate(job, artifact, normalized_id)
    if existing_candidate is not None:
        result_model, _candidate = existing_candidate
        if result_model.revision != expected_revision + 1:
            raise _conflict()
    elif artifact == "corrections":
        normalized_patch = (
            patch if isinstance(patch, _corrections_patch_classes()) else parse_corrections_patch(patch)
        )
        assert isinstance(current.model, ConfirmedCorrectionsV1)
        result_model = _apply_corrections_patch(
            root,
            job,
            current.model,
            normalized_patch,
        )
    else:
        normalized_patch = (
            patch if isinstance(patch, (SetDecisionPatch, ResetDecisionPatch)) else parse_decision_patch(patch)
        )
        assert isinstance(current.model, ApplicationDecisionV1)
        result_model = _apply_decision_patch(root, job, current.model, normalized_patch)
    return _commit_candidate(
        job,
        artifact=artifact,
        model=result_model,
        expected=current,
        mutation_id=normalized_id,
    )


def inspect_application_decision(
    workspace: Path,
    job_dir: Path,
) -> ApplicationDecisionInspection:
    try:
        root, job = _mutation_paths(workspace, job_dir)
        snapshot = _load_artifact_snapshot(job, "decision", allow_missing=True)
    except UserMutationError as exc:
        return ApplicationDecisionInspection(
            snapshot=None,
            basis_status="unavailable",
            reason=exc.code,
        )
    if snapshot is None:
        return ApplicationDecisionInspection(
            snapshot=None,
            basis_status="unavailable",
            reason="user_input.not_initialized",
        )
    assert isinstance(snapshot.model, ApplicationDecisionV1)
    decision = snapshot.model
    if decision.decision == "undecided" or decision.basis is None:
        return ApplicationDecisionInspection(
            snapshot=snapshot,
            basis_status="unavailable",
            reason="decision.undecided",
        )
    try:
        current = _current_decision_basis(root, job)
    except UserMutationError as exc:
        return ApplicationDecisionInspection(
            snapshot=snapshot,
            basis_status="review_required",
            reason=exc.code,
        )
    effective = (
        "current"
        if decision.basis.status == "current"
        and decision.basis.criteria_sha256 == current.criteria_sha256
        and decision.basis.matches_sha256 == current.matches_sha256
        else "review_required"
    )
    return ApplicationDecisionInspection(
        snapshot=snapshot,
        basis_status=effective,
        reason=None if effective == "current" else "decision.basis_changed",
    )


def inspect_user_mutation(
    workspace: Path,
    job_dir: Path,
    mutation_id: str,
) -> MutationInspection:
    _, job = _mutation_paths(workspace, job_dir)
    normalized_id = _normalized_mutation_id(mutation_id)
    claim, claim_path = _find_claim(job, normalized_id)
    receipt, receipt_path = _load_receipt(job, normalized_id, allow_missing=True)
    target = _load_artifact_snapshot(job, claim.artifact, allow_missing=True)
    try:
        interrupted_publication = bool(
            target is not None and target.interrupted_publication
        ) or any(
            (
                has_interrupted_safe_publication(job, _claim_relative(claim)),
                has_interrupted_safe_publication(job, claim.candidate_path),
                has_interrupted_safe_publication(job, _receipt_relative(normalized_id)),
            )
        )
    except UnsafeUserFileError as exc:
        raise _recovery_conflict(normalized_id) from exc
    if receipt is not None:
        if not _receipt_matches_claim(receipt, claim):
            raise UserMutationError(
                "user_input.recovery_required",
                "The mutation receipt does not match its immutable claim.",
            )
        target_matches = (
            target is not None
            and target.sha256 == claim.result_sha256
            and target.revision == claim.result_revision
        )
        if not target_matches:
            status: RecoveryStatus = "conflict"
        elif interrupted_publication:
            status = "receipt_pending"
        else:
            status = "committed"
    elif target is not None and target.sha256 == claim.result_sha256 and target.revision == claim.result_revision:
        status = "receipt_pending"
    elif _snapshot_matches_expected(target, claim):
        status = (
            "promotion_pending"
            if _claim_candidate_is_valid(job, claim)
            else "conflict"
        )
    else:
        status = "conflict"
    return MutationInspection(
        status=status,
        claim=claim,
        claim_path=claim_path,
        receipt=receipt,
        receipt_path=receipt_path,
    )


def recover_user_mutation(
    workspace: Path,
    job_dir: Path,
    mutation_id: str,
    *,
    consent_confirmed: bool,
) -> MutationOutcome:
    _require_consent(consent_confirmed)
    _, job = _mutation_paths(workspace, job_dir)
    normalized_id = _normalized_mutation_id(mutation_id)
    claim, claim_path = _find_claim(job, normalized_id)
    try:
        for relative in dict.fromkeys(
            (
                _claim_relative(claim),
                claim.candidate_path,
                claim.target_path,
                _receipt_relative(normalized_id),
            )
        ):
            repair_interrupted_safe_publication(job, relative)
    except (UnsafeUserFileError, UserFileStoreError) as exc:
        raise _recovery_conflict(normalized_id) from exc
    # Reload all durable evidence after the consented repair so no descriptor or
    # model from the interrupted two-link state is reused for promotion.
    claim, claim_path = _find_claim(job, normalized_id)
    receipt, receipt_path = _load_receipt(job, normalized_id, allow_missing=True)
    if receipt is not None:
        if not _receipt_matches_claim(receipt, claim):
            raise _recovery_conflict(normalized_id)
        target = _load_artifact_snapshot(job, claim.artifact, allow_missing=False)
        assert target is not None
        if target.sha256 != claim.result_sha256 or target.revision != claim.result_revision:
            raise _recovery_conflict(normalized_id)
        return MutationOutcome(
            status="committed",
            snapshot=target,
            mutation_id=normalized_id,
            claim_path=claim_path,
            receipt_path=receipt_path,
            changed=False,
        )
    return _finish_claim(job, claim, claim_path=claim_path)


def initialize_confirmed_corrections(
    workspace: Path,
    job_dir: Path,
    *,
    consent_confirmed: bool,
    mutation_id: str | None = None,
) -> MutationOutcome:
    return initialize_user_artifact(
        workspace,
        job_dir,
        "corrections",
        consent_confirmed=consent_confirmed,
        mutation_id=mutation_id,
    )


def initialize_application_decision(
    workspace: Path,
    job_dir: Path,
    *,
    consent_confirmed: bool,
    mutation_id: str | None = None,
) -> MutationOutcome:
    return initialize_user_artifact(
        workspace,
        job_dir,
        "decision",
        consent_confirmed=consent_confirmed,
        mutation_id=mutation_id,
    )


def _apply_corrections_patch(
    workspace: Path,
    job: Path,
    current: ConfirmedCorrectionsV1,
    patch: CorrectionsPatch,
) -> ConfirmedCorrectionsV1:
    _require_criteria_projection_usable(workspace, job)
    catalog, catalog_file = _load_criteria_catalog_with_snapshot(job)
    now = _next_timestamp(current.updated_at)
    if isinstance(patch, ConfirmEmptyCriteriaPatch):
        basis = _current_empty_extraction_basis(
            workspace,
            job,
            criteria=catalog,
        )
        new_id = _new_correction_id(current)
        history = tuple(
            item.model_copy(
                update={"record_state": "superseded", "superseded_by": new_id}
            )
            if item.record_state == "active"
            else item
            for item in current.criteria_extraction_confirmations
        )
        confirmation = CriteriaExtractionConfirmationV1(
            correction_id=new_id,
            target_extraction_sha256=basis,
            confirmation="confirmed_empty",
            confirmed_at=now,
        )
        _require_current_criteria_snapshot(
            workspace,
            job,
            expected_sha256=catalog_file.sha256,
        )
        return current.model_copy(
            update={
                "revision": current.revision + 1,
                "updated_at": now,
                "criteria_extraction_confirmations": (*history, confirmation),
            }
        )

    criterion = next(
        (item for item in catalog.criteria if item.criterion_id == patch.criterion_id),
        None,
    )
    if criterion is None:
        raise UserMutationError(
            "user_input.invalid",
            "The corrections patch does not target a current criterion.",
        )
    active = next(
        (
            item
            for item in current.criteria
            if item.criterion_id == criterion.criterion_id and item.record_state == "active"
        ),
        None,
    )
    if isinstance(patch, WithdrawCriterionPatch):
        if active is None:
            raise UserMutationError(
                "user_input.invalid",
                "The corrections patch has no active record to withdraw.",
            )
        records = tuple(
            item.model_copy(update={"record_state": "withdrawn", "superseded_by": None})
            if item.correction_id == active.correction_id
            else item
            for item in current.criteria
        )
        _require_current_criteria_snapshot(
            workspace,
            job,
            expected_sha256=catalog_file.sha256,
        )
        return current.model_copy(
            update={
                "revision": current.revision + 1,
                "updated_at": now,
                "criteria": records,
            }
        )

    occurrence = patch.source_occurrence
    anchor = _source_anchor_for_occurrence(criterion, occurrence)
    new_id = _new_correction_id(current)
    records = tuple(
        item.model_copy(update={"record_state": "superseded", "superseded_by": new_id})
        if active is not None and item.correction_id == active.correction_id
        else item
        for item in current.criteria
    )
    corrected_text = patch.corrected_text if isinstance(patch, CorrectCriterionPatch) else None
    correction = CriterionCorrectionV1(
        correction_id=new_id,
        criterion_id=criterion.criterion_id,
        target_source_sha256=criterion_source_sha256(criterion.source_text),
        target_criterion_sha256=criterion.parsed_text_sha256,
        confirmation="corrected" if corrected_text is not None else "confirmed",
        corrected_text=corrected_text,
        source_occurrence=occurrence,
        source_anchor_sha256=anchor,
        confirmed_at=now,
    )
    _require_current_criteria_snapshot(
        workspace,
        job,
        expected_sha256=catalog_file.sha256,
    )
    return current.model_copy(
        update={
            "revision": current.revision + 1,
            "updated_at": now,
            "criteria": (*records, correction),
        }
    )


def _apply_decision_patch(
    workspace: Path,
    job: Path,
    current: ApplicationDecisionV1,
    patch: DecisionPatch,
) -> ApplicationDecisionV1:
    now = _next_timestamp(current.updated_at)
    if isinstance(patch, ResetDecisionPatch):
        return current.model_copy(
            update={
                "revision": current.revision + 1,
                "updated_at": now,
                "decision": "undecided",
                "confirmation_state": "unconfirmed",
                "confirmed_at": None,
                "rationale": None,
                "basis": None,
            }
        )
    basis = _current_decision_basis(workspace, job)
    rationale = current.rationale
    if patch.rationale_mode == "set":
        rationale = patch.rationale
    elif patch.rationale_mode == "clear":
        rationale = None
    return current.model_copy(
        update={
            "revision": current.revision + 1,
            "updated_at": now,
            "decision": patch.decision,
            "confirmation_state": "confirmed",
            "confirmed_at": now,
            "rationale": rationale,
            "basis": basis,
        }
    )


def _commit_candidate(
    job: Path,
    *,
    artifact: UserArtifactKind,
    model: UserArtifactModel,
    expected: UserArtifactSnapshot | None,
    mutation_id: str,
) -> MutationOutcome:
    candidate_relative = _candidate_relative(mutation_id)
    candidate = _load_existing_candidate(job, artifact, mutation_id)
    if candidate is None:
        candidate_bytes = _serialize_artifact(model)
        try:
            candidate_path = write_safe_immutable_file(job, candidate_relative, candidate_bytes)
        except UserFileConflictError as exc:
            raise _conflict() from exc
        except UserFileStoreError as exc:
            raise UserMutationError(
                "user_input.store_failed",
                "The private mutation candidate could not be stored safely.",
            ) from exc
    else:
        stored_model, stored_file = candidate
        if stored_model.revision != model.revision:
            raise _conflict()
        try:
            existing_claim, existing_claim_path = _find_claim(job, mutation_id)
        except UserMutationError as exc:
            raise UserMutationError(
                "user_input.conflict",
                "The mutation identifier already names an orphaned private candidate.",
            ) from exc
        if (
            existing_claim.artifact != artifact
            or existing_claim.expected_revision
            != (expected.revision if expected is not None else None)
            or existing_claim.expected_sha256
            != (expected.sha256 if expected is not None else None)
            or existing_claim.candidate_sha256 != stored_file.sha256
        ):
            raise _conflict()
        return _finish_claim(job, existing_claim, claim_path=existing_claim_path)

    result_sha256 = _sha256(candidate_bytes)
    claim = UserMutationClaimV1(
        mutation_id=mutation_id,
        job_id=job.name,
        artifact=artifact,
        target_path=_target_path(artifact),
        expected_revision=expected.revision if expected is not None else None,
        expected_sha256=expected.sha256 if expected is not None else None,
        result_revision=model.revision,
        result_sha256=result_sha256,
        candidate_path=candidate_relative,
        candidate_sha256=result_sha256,
        claimed_at=_utc_now(),
        consent_confirmed=True,
    )
    claim_relative = _claim_relative(claim)
    claim_path = resolve_job_relative_path(job, claim_relative)
    if claim_path.exists() or claim_path.is_symlink():
        existing_claim = _load_claim(job, claim_relative)
        if not _claim_same_transition(existing_claim, claim):
            raise _accepted_claim_requires_recovery(existing_claim.mutation_id)
        claim = existing_claim
    else:
        try:
            claim_path = _store_claim(job, claim)
        except UserFileConflictError:
            existing_claim = _load_claim(job, claim_relative)
            if not _claim_same_transition(existing_claim, claim):
                raise _accepted_claim_requires_recovery(existing_claim.mutation_id)
            claim = existing_claim
        except UserFileStoreError as exc:
            try:
                accepted_claim = _load_claim(job, claim_relative)
            except UserMutationError:
                accepted_claim = None
            if accepted_claim is not None:
                raise _accepted_claim_requires_recovery(
                    accepted_claim.mutation_id
                ) from exc
            raise UserMutationError(
                "user_input.store_failed",
                "The mutation claim could not be stored safely.",
            ) from exc
    return _finish_claim(job, claim, claim_path=claim_path)


def _finish_claim(
    job: Path,
    claim: UserMutationClaimV1,
    *,
    claim_path: Path,
) -> MutationOutcome:
    receipt, receipt_path = _load_receipt(job, claim.mutation_id, allow_missing=True)
    current = _load_artifact_snapshot(job, claim.artifact, allow_missing=True)
    if receipt is not None:
        if not _receipt_matches_claim(receipt, claim):
            raise _recovery_conflict(claim.mutation_id)
        if (
            current is None
            or current.sha256 != claim.result_sha256
            or current.revision != claim.result_revision
        ):
            raise _recovery_conflict(claim.mutation_id)
        return MutationOutcome(
            status="committed",
            snapshot=current,
            mutation_id=claim.mutation_id,
            claim_path=claim_path,
            receipt_path=receipt_path,
            changed=False,
        )

    changed = False
    if current is not None and current.sha256 == claim.result_sha256 and current.revision == claim.result_revision:
        committed = current
    elif _snapshot_matches_expected(current, claim):
        candidate = _load_existing_candidate(job, claim.artifact, claim.mutation_id)
        if candidate is None:
            raise UserMutationError(
                "user_input.recovery_required",
                "The accepted mutation candidate is missing or invalid.",
                mutation_id=claim.mutation_id,
            )
        candidate_model, candidate_file = candidate
        if (
            candidate_file.sha256 != claim.candidate_sha256
            or candidate_model.revision != claim.result_revision
        ):
            raise _recovery_conflict(claim.mutation_id)
        # The baseline is read again immediately before the single-file replacement.
        latest = _load_artifact_snapshot(job, claim.artifact, allow_missing=True)
        if not _snapshot_matches_expected(latest, claim):
            raise _recovery_conflict(claim.mutation_id)
        commit_error: Exception | None = None
        try:
            if latest is None:
                create_safe_file(job, claim.target_path, candidate_file.data)
            else:
                replace_safe_file(job, claim.target_path, candidate_file.data)
        except UserFileConflictError as exc:
            raise _recovery_conflict(claim.mutation_id) from exc
        except (UnsafeUserFileError, UserFileStoreError) as exc:
            commit_error = exc
        committed = _load_artifact_snapshot(job, claim.artifact, allow_missing=True)
        if committed is None or (
            committed.sha256 != claim.result_sha256
            or committed.revision != claim.result_revision
        ):
            if commit_error is not None and _snapshot_matches_expected(committed, claim):
                raise UserMutationError(
                    "user_input.recovery_required",
                    "The accepted mutation requires recovery before it can be committed.",
                    mutation_id=claim.mutation_id,
                ) from commit_error
            raise _recovery_conflict(claim.mutation_id) from commit_error
        if committed.sha256 != claim.result_sha256 or committed.revision != claim.result_revision:
            raise _recovery_conflict(claim.mutation_id)
        changed = True
    else:
        raise _recovery_conflict(claim.mutation_id)

    receipt = UserMutationReceiptV1(
        mutation_id=claim.mutation_id,
        job_id=claim.job_id,
        artifact=claim.artifact,
        target_path=claim.target_path,
        expected_revision=claim.expected_revision,
        expected_sha256=claim.expected_sha256,
        result_revision=claim.result_revision,
        result_sha256=claim.result_sha256,
        committed_at=_utc_now(),
    )
    try:
        receipt_path = _store_receipt(job, receipt)
    except UserFileConflictError:
        existing, receipt_path = _load_receipt(
            job,
            claim.mutation_id,
            allow_missing=False,
        )
        if existing is None or not _receipt_matches_claim(existing, claim):
            raise _recovery_conflict(claim.mutation_id)
    except UserFileStoreError:
        return MutationOutcome(
            status="committed_receipt_pending",
            snapshot=committed,
            mutation_id=claim.mutation_id,
            claim_path=claim_path,
            receipt_path=None,
            changed=changed,
        )
    return MutationOutcome(
        status="committed",
        snapshot=committed,
        mutation_id=claim.mutation_id,
        claim_path=claim_path,
        receipt_path=receipt_path,
        changed=changed,
    )


def _load_artifact_snapshot(
    job: Path,
    artifact: UserArtifactKind,
    *,
    allow_missing: bool,
) -> UserArtifactSnapshot | None:
    relative = _target_path(artifact)
    try:
        file_snapshot = read_optional_safe_bytes(
            job,
            relative,
            allow_interrupted_publication=True,
        )
    except UnsafeUserFileError as exc:
        raise UserMutationError(
            "user_input.unsafe_path",
            "The user-owned file is not one safe job-local regular file.",
        ) from exc
    if file_snapshot is None:
        if allow_missing:
            return None
        raise UserMutationError(
            "user_input.not_initialized",
            "The user-owned file has not been initialized.",
        )
    try:
        payload = load_strict_yaml(file_snapshot.data)
        model: UserArtifactModel = (
            ConfirmedCorrectionsV1.model_validate(payload)
            if artifact == "corrections"
            else ApplicationDecisionV1.model_validate(payload)
        )
    except (InvalidUserFileError, ValidationError) as exc:
        raise UserMutationError(
            "user_input.invalid",
            "The user-owned file is not valid strict versioned YAML.",
        ) from exc
    if model.job_id != job.name:
        raise UserMutationError(
            "user_input.invalid",
            "The user-owned file belongs to a different job.",
        )
    return UserArtifactSnapshot(
        artifact=artifact,
        path=file_snapshot.path,
        relative_path=relative,
        model=model,
        sha256=file_snapshot.sha256,
        revision=model.revision,
        raw_bytes=file_snapshot.data,
        interrupted_publication=file_snapshot.interrupted_publication,
    )


def _load_existing_candidate(
    job: Path,
    artifact: UserArtifactKind,
    mutation_id: str,
) -> tuple[UserArtifactModel, SafeFileSnapshot] | None:
    relative = _candidate_relative(mutation_id)
    try:
        snapshot = read_optional_safe_bytes(
            job,
            relative,
            allow_interrupted_publication=True,
        )
    except UnsafeUserFileError as exc:
        raise UserMutationError(
            "user_input.recovery_required",
            "The private mutation candidate is unsafe.",
            mutation_id=mutation_id,
        ) from exc
    if snapshot is None:
        return None
    try:
        payload = load_strict_yaml(snapshot.data)
        model: UserArtifactModel = (
            ConfirmedCorrectionsV1.model_validate(payload)
            if artifact == "corrections"
            else ApplicationDecisionV1.model_validate(payload)
        )
    except (InvalidUserFileError, ValidationError) as exc:
        raise UserMutationError(
            "user_input.recovery_required",
            "The private mutation candidate is invalid.",
            mutation_id=mutation_id,
        ) from exc
    if model.job_id != job.name:
        raise _recovery_conflict(mutation_id)
    return model, snapshot


def _serialize_artifact(model: UserArtifactModel) -> bytes:
    try:
        data = dump_yaml_mapping(model.model_dump(mode="json"))
        payload = load_strict_yaml(data)
        validated = type(model).model_validate(payload)
    except (InvalidUserFileError, ValidationError) as exc:
        raise UserMutationError(
            "user_input.invalid",
            "The user-owned mutation result could not be validated.",
        ) from exc
    if validated != model:
        raise UserMutationError(
            "user_input.invalid",
            "The user-owned mutation result is not round-trip stable.",
        )
    return data


def _load_criteria_catalog_with_snapshot(
    job: Path,
) -> tuple[CriteriaCatalogV1, SafeFileSnapshot]:
    model, snapshot = _load_structured_artifact(job, "criteria.json", CriteriaCatalogV1)
    assert isinstance(model, CriteriaCatalogV1)
    return model, snapshot


def _load_criteria_catalog(job: Path) -> CriteriaCatalogV1:
    return _load_criteria_catalog_with_snapshot(job)[0]


def _current_empty_extraction_basis(
    workspace: Path,
    job: Path,
    *,
    criteria: CriteriaCatalogV1 | None = None,
) -> str:
    _require_stage_current(workspace, job, "parse")
    criteria = criteria or _load_criteria_catalog(job)
    if criteria.criteria or criteria.extraction_state != "unknown":
        raise UserMutationError(
            "user_input.invalid",
            "Empty extraction can be confirmed only for a current unknown empty catalog.",
        )
    parsed, _parsed_file = _load_structured_artifact(job, "parsed_job.json", None)
    advert = _read_safe(job, "job_advert.md")
    try:
        advert_text = advert.data.decode("utf-8")
        return criteria_extraction_basis_sha256(parsed, advert_text)
    except (UnicodeError, ValueError) as exc:
        raise UserMutationError(
            "user_input.dependency_not_current",
            "The current extraction basis could not be validated.",
        ) from exc


def _current_decision_basis(workspace: Path, job: Path) -> DecisionBasisV1:
    _require_stage_current(workspace, job, "confirm")
    _require_stage_current(workspace, job, "match")
    criteria, criteria_file = _load_structured_artifact(
        job,
        "criteria.json",
        CriteriaCatalogV1,
    )
    matches, matches_file = _load_structured_artifact(
        job,
        "criterion_matches.json",
        CriterionMatchesV1,
    )
    assert isinstance(criteria, CriteriaCatalogV1)
    assert isinstance(matches, CriterionMatchesV1)
    if (
        criteria.job_id != job.name
        or matches.job_id != job.name
        or matches.criteria_catalog_sha256 != criteria_file.sha256
    ):
        raise UserMutationError(
            "user_input.dependency_not_current",
            "The decision basis artifacts do not share current receipts.",
        )
    # Close the ordinary drift window around the validated pair. The final
    # byte comparison is intentionally after the runtime checks so a file that
    # changed while status was inspected cannot be accepted as the basis.
    _require_stage_current(workspace, job, "confirm")
    _require_stage_current(workspace, job, "match")
    final_criteria = _read_safe(job, "criteria.json")
    final_matches = _read_safe(job, "criterion_matches.json")
    if (
        final_criteria.sha256 != criteria_file.sha256
        or final_matches.sha256 != matches_file.sha256
    ):
        raise UserMutationError(
            "user_input.dependency_not_current",
            "The decision basis changed while it was inspected.",
        )
    return DecisionBasisV1(
        criteria_sha256=criteria_file.sha256,
        matches_sha256=matches_file.sha256,
        status="current",
    )


def _load_structured_artifact(
    job: Path,
    relative_path: str,
    model_type: type[BaseModel] | None,
) -> tuple[Any, SafeFileSnapshot]:
    snapshot = _read_safe(job, relative_path)
    try:
        payload = load_strict_json(snapshot.data)
        model = payload if model_type is None else model_type.model_validate(payload)
    except (InvalidUserFileError, ValidationError) as exc:
        raise UserMutationError(
            "user_input.dependency_not_current",
            "A required structured decision input is invalid.",
        ) from exc
    if hasattr(model, "job_id") and model.job_id != job.name:
        raise UserMutationError(
            "user_input.dependency_not_current",
            "A required structured decision input belongs to a different job.",
        )
    return model, snapshot


def _require_stage_current(workspace: Path, job: Path, stage: str) -> None:
    try:
        inspection = inspect_stage_status(
            workspace,
            job,
            stage=stage,  # type: ignore[arg-type]
        )
    except StageRuntimeError as exc:
        raise UserMutationError(
            "user_input.dependency_not_current",
            "A required Decision Spine stage is not current.",
        ) from exc
    if (
        inspection.stage.status != "succeeded"
        or inspection.reasons
        or inspection.output_drift
    ):
        raise UserMutationError(
            "user_input.dependency_not_current",
            "A required Decision Spine stage is not current.",
        )


def _require_criteria_projection_usable(workspace: Path, job: Path) -> None:
    _require_stage_current(workspace, job, "parse")
    # StageStatus v1 exposes one aggregate fingerprint, so it cannot prove
    # that a stale Confirm result differs *only* because of corrections. Require
    # a current catalog before every scoped patch; callers rerun deterministic
    # Confirm between consecutive semantic updates.
    _require_stage_current(workspace, job, "confirm")


def _require_current_criteria_snapshot(
    workspace: Path,
    job: Path,
    *,
    expected_sha256: str,
) -> None:
    _require_stage_current(workspace, job, "confirm")
    current = _read_safe(job, "criteria.json")
    if current.sha256 != expected_sha256:
        raise UserMutationError(
            "user_input.dependency_not_current",
            "The criteria projection changed while it was inspected.",
        )


def _source_anchor_for_occurrence(criterion: Any, occurrence: int | None) -> str | None:
    if occurrence is None:
        return None
    candidates = tuple(criterion.source_candidates)
    if criterion.source_span is not None:
        candidates = (*candidates, criterion.source_span)
    selected = next((item for item in candidates if item.occurrence == occurrence), None)
    if selected is None:
        raise UserMutationError(
            "user_input.invalid",
            "The requested source occurrence is not a current criterion source candidate.",
        )
    return selected.anchor_sha256


def _new_correction_id(current: ConfirmedCorrectionsV1) -> str:
    known = {item.correction_id for item in current.criteria} | {
        item.correction_id for item in current.criteria_extraction_confirmations
    }
    while True:
        candidate = f"correction_{uuid4().hex}"
        if candidate not in known:
            return candidate


def _safe_control_json_paths(job: Path, relative_directory: str) -> tuple[Path, ...]:
    """List immutable control JSON files without following directory symlinks."""

    try:
        directory = resolve_job_relative_path(job, relative_directory)
        relative = directory.relative_to(job.expanduser().resolve())
    except (ValueError, StageStoreError, UnsafeStagePathError) as exc:
        raise _recovery_conflict() from exc

    current = job.expanduser().resolve()
    for part in relative.parts:
        current = current / part
        try:
            metadata = current.lstat()
        except FileNotFoundError:
            return ()
        except OSError as exc:
            raise _recovery_conflict() from exc
        if not stat.S_ISDIR(metadata.st_mode):
            raise _recovery_conflict()
    try:
        entries = tuple(sorted(directory.iterdir(), key=lambda path: path.name))
    except OSError as exc:
        raise _recovery_conflict() from exc
    return tuple(path for path in entries if path.suffix == ".json")


def _load_claim(job: Path, relative: str) -> UserMutationClaimV1:
    try:
        snapshot = read_safe_bytes(
            job,
            relative,
            allow_interrupted_publication=True,
        )
        claim = UserMutationClaimV1.model_validate(load_strict_json(snapshot.data))
    except (InvalidUserFileError, UnsafeUserFileError, UserFileStoreError, ValidationError) as exc:
        raise _recovery_conflict() from exc
    if claim.job_id != job.name or relative != _claim_relative(claim):
        raise _recovery_conflict(claim.mutation_id)
    return claim


def _load_baseline_claim(
    job: Path,
    artifact: UserArtifactKind,
    *,
    expected_revision: int | None,
    expected_sha256: str | None,
) -> tuple[UserMutationClaimV1, Path] | None:
    relative = _baseline_claim_relative(
        artifact,
        expected_revision=expected_revision,
        expected_sha256=expected_sha256,
    )
    try:
        snapshot = read_optional_safe_bytes(
            job,
            relative,
            allow_interrupted_publication=True,
        )
    except (UnsafeUserFileError, UserFileStoreError) as exc:
        raise _recovery_conflict() from exc
    if snapshot is None:
        return None
    claim = _load_claim(job, relative)
    if (
        claim.artifact != artifact
        or claim.expected_revision != expected_revision
        or claim.expected_sha256 != expected_sha256
    ):
        raise _recovery_conflict(claim.mutation_id)
    return claim, snapshot.path


def _store_claim(job: Path, claim: UserMutationClaimV1) -> Path:
    return write_safe_immutable_file(
        job,
        _claim_relative(claim),
        _json_model_bytes(claim),
    )


def _store_receipt(job: Path, receipt: UserMutationReceiptV1) -> Path:
    return write_safe_immutable_file(
        job,
        _receipt_relative(receipt.mutation_id),
        _json_model_bytes(receipt),
    )


def _find_claim(job: Path, mutation_id: str) -> tuple[UserMutationClaimV1, Path]:
    found: list[tuple[UserMutationClaimV1, Path]] = []
    for artifact in ("corrections", "decision"):
        directory = job / "workflow" / "user-mutations" / "claims" / artifact
        try:
            entries = tuple(directory.glob("*.json")) if directory.is_dir() and not directory.is_symlink() else ()
        except OSError:
            entries = ()
        for path in entries:
            try:
                relative = path.relative_to(job).as_posix()
                claim = _load_claim(job, relative)
            except (UserMutationError, ValueError):
                continue
            if claim.mutation_id == mutation_id:
                if relative != _claim_relative(claim):
                    raise _recovery_conflict(mutation_id)
                found.append((claim, path))
    if len(found) != 1:
        raise UserMutationError(
            "user_input.recovery_required",
            "The requested mutation has no unique valid claim.",
        )
    return found[0]


def _load_receipt(
    job: Path,
    mutation_id: str,
    *,
    allow_missing: bool,
) -> tuple[UserMutationReceiptV1 | None, Path | None]:
    relative = _receipt_relative(mutation_id)
    try:
        snapshot = read_optional_safe_bytes(
            job,
            relative,
            allow_interrupted_publication=True,
        )
    except UnsafeUserFileError as exc:
        raise _recovery_conflict(mutation_id) from exc
    if snapshot is None:
        if allow_missing:
            return None, None
        raise UserMutationError(
            "user_input.recovery_required",
            "The immutable mutation receipt is missing.",
            mutation_id=mutation_id,
        )
    try:
        receipt = UserMutationReceiptV1.model_validate(load_strict_json(snapshot.data))
    except (InvalidUserFileError, ValidationError) as exc:
        raise _recovery_conflict(mutation_id) from exc
    return receipt, snapshot.path


def _receipt_matches_claim(
    receipt: UserMutationReceiptV1,
    claim: UserMutationClaimV1,
) -> bool:
    return (
        receipt.mutation_id == claim.mutation_id
        and receipt.job_id == claim.job_id
        and receipt.artifact == claim.artifact
        and receipt.target_path == claim.target_path
        and receipt.expected_revision == claim.expected_revision
        and receipt.expected_sha256 == claim.expected_sha256
        and receipt.result_revision == claim.result_revision
        and receipt.result_sha256 == claim.result_sha256
    )


def _claim_same_transition(first: UserMutationClaimV1, second: UserMutationClaimV1) -> bool:
    return first.model_copy(update={"claimed_at": second.claimed_at}) == second


def _snapshot_matches_expected(
    snapshot: UserArtifactSnapshot | None,
    claim: UserMutationClaimV1,
) -> bool:
    if claim.expected_revision is None and claim.expected_sha256 is None:
        return snapshot is None
    return (
        snapshot is not None
        and snapshot.revision == claim.expected_revision
        and snapshot.sha256 == claim.expected_sha256
    )


def _snapshot_matches_result(
    snapshot: UserArtifactSnapshot | None,
    claim: UserMutationClaimV1,
) -> bool:
    return (
        snapshot is not None
        and snapshot.revision == claim.result_revision
        and snapshot.sha256 == claim.result_sha256
    )


def _claim_candidate_is_valid(job: Path, claim: UserMutationClaimV1) -> bool:
    try:
        candidate = _load_existing_candidate(
            job,
            claim.artifact,
            claim.mutation_id,
        )
    except UserMutationError:
        return False
    if candidate is None:
        return False
    candidate_model, candidate_file = candidate
    return (
        candidate_model.revision == claim.result_revision
        and candidate_file.sha256 == claim.candidate_sha256
        and candidate_file.sha256 == claim.result_sha256
    )


def _compare_baseline(
    snapshot: UserArtifactSnapshot,
    expected_sha256: str,
    expected_revision: int,
) -> None:
    if snapshot.sha256 != expected_sha256 or snapshot.revision != expected_revision:
        raise _conflict()


def _require_expected_baseline(expected_sha256: str, expected_revision: int) -> None:
    if (
        _SHA256_RE.fullmatch(expected_sha256) is None
        or type(expected_revision) is not int
        or expected_revision < 0
        or expected_revision > MAX_USER_REVISION
    ):
        raise UserMutationError(
            "user_input.invalid",
            "Expected hash and revision must form a valid mutation baseline.",
        )


def _require_consent(consent_confirmed: bool) -> None:
    if type(consent_confirmed) is not bool or consent_confirmed is not True:
        raise UserMutationError(
            "user_input.consent_required",
            "An explicit user-owned write confirmation is required.",
        )


def _mutation_paths(workspace: Path, job_dir: Path) -> tuple[Path, Path]:
    root = workspace.expanduser().resolve()
    candidate = job_dir.expanduser()
    job = (root / candidate).resolve() if not candidate.is_absolute() else candidate.resolve()
    try:
        job.relative_to(root)
    except ValueError as exc:
        raise UserMutationError(
            "user_input.unsafe_path",
            "User-owned mutation requires a job inside the selected workspace.",
        ) from exc
    if not job.is_dir():
        raise UserMutationError("job.not_found", "The requested job directory does not exist.")
    return root, job


def _patch_artifact(patch: UserPatch | dict[str, Any]) -> UserArtifactKind:
    operation = patch.get("operation") if isinstance(patch, dict) else patch.operation
    if operation in {
        "confirm_criterion",
        "correct_criterion",
        "withdraw_criterion",
        "confirm_empty",
    }:
        return "corrections"
    if operation in {"set_decision", "reset_decision"}:
        return "decision"
    raise UserMutationError(
        "user_input.invalid",
        "The user-owned patch operation is not supported.",
    )


def _corrections_patch_classes() -> tuple[type[BaseModel], ...]:
    return (
        ConfirmCriterionPatch,
        CorrectCriterionPatch,
        WithdrawCriterionPatch,
        ConfirmEmptyCriteriaPatch,
    )


def _target_path(artifact: UserArtifactKind) -> str:
    return CONFIRMED_CORRECTIONS_PATH if artifact == "corrections" else APPLICATION_DECISION_PATH


def _candidate_relative(mutation_id: str) -> str:
    return f"workflow/user-mutations/events/{mutation_id}/candidate.yaml"


def _receipt_relative(mutation_id: str) -> str:
    return f"workflow/user-mutations/events/{mutation_id}/receipt.json"


def _claim_relative(claim: UserMutationClaimV1) -> str:
    return _baseline_claim_relative(
        claim.artifact,
        expected_revision=claim.expected_revision,
        expected_sha256=claim.expected_sha256,
    )


def _baseline_claim_relative(
    artifact: UserArtifactKind,
    *,
    expected_revision: int | None,
    expected_sha256: str | None,
) -> str:
    baseline = (
        "absent"
        if expected_sha256 is None
        else f"r{expected_revision}-{expected_sha256}"
    )
    return f"workflow/user-mutations/claims/{artifact}/{baseline}.json"


def _normalized_mutation_id(value: str | None) -> str:
    mutation_id = value or f"mutation_{uuid4().hex}"
    if _MUTATION_ID_RE.fullmatch(mutation_id) is None:
        raise UserMutationError(
            "user_input.invalid",
            "The mutation identifier is invalid.",
        )
    return mutation_id


def _read_safe(job: Path, relative: str) -> SafeFileSnapshot:
    try:
        return read_safe_bytes(job, relative)
    except (UnsafeUserFileError, UserFileStoreError) as exc:
        raise UserMutationError(
            "user_input.dependency_not_current",
            "A required decision input could not be read safely.",
        ) from exc


def _next_timestamp(previous: datetime) -> datetime:
    now = _utc_now()
    return max(now, previous + timedelta(microseconds=1))


def _utc_now() -> datetime:
    return datetime.now(UTC)


def _sha256(data: bytes) -> str:
    from hashlib import sha256

    return sha256(data).hexdigest()


def _json_model_bytes(model: BaseModel) -> bytes:
    try:
        rendered = json.dumps(
            model.model_dump(mode="json"),
            allow_nan=False,
            ensure_ascii=False,
            indent=2,
            sort_keys=True,
        )
    except (TypeError, ValueError) as exc:
        raise UserFileStoreError("The mutation control record could not be serialized.") from exc
    return f"{rendered}\n".encode("utf-8")


def _conflict() -> UserMutationError:
    return UserMutationError(
        "user_input.conflict",
        "The user-owned file no longer matches the expected hash and revision.",
    )


def _accepted_claim_requires_recovery(mutation_id: str) -> UserMutationError:
    return UserMutationError(
        "user_input.recovery_required",
        "A durable mutation claim already owns the requested baseline.",
        mutation_id=mutation_id,
    )


def _recovery_conflict(mutation_id: str | None = None) -> UserMutationError:
    return UserMutationError(
        "user_input.recovery_required",
        "The mutation cannot be recovered because its durable evidence conflicts.",
        mutation_id=mutation_id,
    )
