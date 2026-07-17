from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
import json
from pathlib import Path
from typing import Any

from pydantic import ValidationError

from canisend.decision_models import ApplicationBriefV1, RequiredDocumentPlanV1
from canisend.document_execution import (
    DocumentExecutionPlanV1,
    DocumentWorkItemV1,
    derive_document_execution_plan,
)
from canisend.draft_models import (
    ClaimV1,
    CoverLetterDraftV1,
    ResearchStatementDraftV1,
    ReviewFindingsV1,
)
from canisend.package_review_models import (
    PackageCorrectionProposalV1,
    PackageDocumentReviewV1,
    PackageReviewFindingV1,
    PackageReviewFindingsV1,
    canonical_sha256,
    normalize_claim_text,
    stable_package_finding_id,
    stable_package_proposal_id,
)
from canisend.resource_files import read_resource_text
from canisend.schema_validation import (
    SchemaCompilationError,
    compiled_schema_validator,
)
from canisend.review_readiness import ReviewDispositionsV1, derive_document_readiness
from canisend.stage_store import (
    StageStoreError,
    read_json_object,
    resolve_job_relative_path,
    sha256_file,
)
from canisend.stages.draft_stage import (
    COVER_LETTER_DRAFT_OUTPUT_PATH,
    RESEARCH_STATEMENT_DRAFT_OUTPUT_PATH,
    DraftStageError,
    StructuredDraftV1,
    validate_draft_candidate,
    validate_research_statement_draft_candidate,
)
from canisend.stages.review_stage import (
    REVIEW_FINDINGS_OUTPUT_PATH,
    RESEARCH_STATEMENT_REVIEW_FINDINGS_OUTPUT_PATH,
    ReviewStageError,
    validate_research_statement_review_candidate,
    validate_review_candidate,
)
from canisend.user_file_store import (
    InvalidUserFileError,
    UnsafeUserFileError,
    load_strict_yaml,
    read_optional_safe_bytes,
)


PACKAGE_REVIEW_CONTRACT_VERSION = "1.0.0"
PACKAGE_REVIEWER_STRATEGY = "deterministic.package_consistency_review"
PACKAGE_REVIEWER_VERSION = "1.0.0"
PACKAGE_REVIEW_FINDINGS_OUTPUT_PATH = "package_review_findings.json"

PARSED_JOB_INPUT_PATH = "parsed_job.json"
APPLICATION_BRIEF_INPUT_PATH = "application_brief.yaml"
REQUIRED_DOCUMENT_PLAN_INPUT_PATH = "required_document_plan.json"
_COMMON_OPTIONAL_INPUT_PATHS = (
    "criteria.json",
    "evidence_catalog.json",
    "criterion_matches.json",
    "application_decision.yaml",
)
_DOCUMENT_PATHS = {
    "cover_letter": (
        COVER_LETTER_DRAFT_OUTPUT_PATH,
        REVIEW_FINDINGS_OUTPUT_PATH,
        "review_dispositions.yaml",
    ),
    "research_statement": (
        RESEARCH_STATEMENT_DRAFT_OUTPUT_PATH,
        RESEARCH_STATEMENT_REVIEW_FINDINGS_OUTPUT_PATH,
        "research_statement_review_dispositions.yaml",
    ),
}


class PackageReviewStageError(ValueError):
    """Raised when aggregate Review inputs cannot be inspected safely."""


class PackageReviewStageValidationError(PackageReviewStageError):
    """Raised when a package Review candidate cannot be accepted."""


@dataclass(frozen=True)
class _DocumentInputs:
    record: PackageDocumentReviewV1
    draft: StructuredDraftV1 | None = None
    review: ReviewFindingsV1 | None = None


@dataclass(frozen=True)
class _PackageReviewInputs:
    plan: RequiredDocumentPlanV1
    execution_plan: DocumentExecutionPlanV1
    parsed_job_sha256: str
    application_brief_sha256: str
    required_document_plan_sha256: str
    document_execution_plan_sha256: str
    documents: tuple[_DocumentInputs, ...]


def package_review_precondition_reasons(
    workspace: Path,
    job_dir: Path,
    *,
    parsed_job_schema_path: Path | None = None,
    package_review_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    research_statement_schema_path: Path | None = None,
    review_findings_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> tuple[str, ...]:
    try:
        _load_package_review_inputs(
            workspace,
            job_dir,
            parsed_job_schema_path=parsed_job_schema_path,
            cover_letter_schema_path=cover_letter_schema_path,
            research_statement_schema_path=research_statement_schema_path,
            review_findings_schema_path=review_findings_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
        )
        _package_review_schema_text(package_review_schema_path)
    except PackageReviewStageError:
        return ("input_not_ready:package_review_basis",)
    return ()


def package_review_input_artifact_paths(job_dir: Path) -> tuple[str, ...]:
    """Return the exact existing files read by aggregate Review in stable order."""

    paths = [
        PARSED_JOB_INPUT_PATH,
        APPLICATION_BRIEF_INPUT_PATH,
        REQUIRED_DOCUMENT_PLAN_INPUT_PATH,
    ]
    try:
        plan = RequiredDocumentPlanV1.model_validate(
            read_json_object(job_dir / REQUIRED_DOCUMENT_PLAN_INPUT_PATH)
        )
    except (StageStoreError, ValidationError, OSError, ValueError) as exc:
        raise PackageReviewStageError(
            "Package Review requires a valid Required Document Plan."
        ) from exc
    for path in _COMMON_OPTIONAL_INPUT_PATHS:
        candidate = job_dir / path
        if candidate.exists() or candidate.is_symlink():
            paths.append(path)
    for requirement in plan.requirements:
        document_paths = _DOCUMENT_PATHS.get(requirement.normalized_kind)
        if document_paths is None:
            continue
        for path in document_paths:
            candidate = job_dir / path
            if candidate.exists() or candidate.is_symlink():
                paths.append(path)
    return tuple(dict.fromkeys(paths))


def package_review_input_projection(
    workspace: Path,
    job_dir: Path,
    *,
    package_review_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    research_statement_schema_path: Path | None = None,
    review_findings_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> dict[str, object]:
    inputs = _load_package_review_inputs(
        workspace,
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
        cover_letter_schema_path=cover_letter_schema_path,
        research_statement_schema_path=research_statement_schema_path,
        review_findings_schema_path=review_findings_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    return _package_review_projection(
        inputs,
        package_review_schema_path=package_review_schema_path,
    )


def package_review_input_fingerprint(
    workspace: Path,
    job_dir: Path,
    *,
    package_review_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    research_statement_schema_path: Path | None = None,
    review_findings_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> str:
    return canonical_sha256(
        package_review_input_projection(
            workspace,
            job_dir,
            package_review_schema_path=package_review_schema_path,
            parsed_job_schema_path=parsed_job_schema_path,
            cover_letter_schema_path=cover_letter_schema_path,
            research_statement_schema_path=research_statement_schema_path,
            review_findings_schema_path=review_findings_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
        )
    )


def build_deterministic_package_review_candidate(
    workspace: Path,
    job_dir: Path,
    *,
    input_fingerprint: str,
    package_review_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    research_statement_schema_path: Path | None = None,
    review_findings_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> PackageReviewFindingsV1:
    inputs = _load_package_review_inputs(
        workspace,
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
        cover_letter_schema_path=cover_letter_schema_path,
        research_statement_schema_path=research_statement_schema_path,
        review_findings_schema_path=review_findings_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    current_fingerprint = canonical_sha256(
        _package_review_projection(
            inputs,
            package_review_schema_path=package_review_schema_path,
        )
    )
    if current_fingerprint != input_fingerprint:
        raise PackageReviewStageError("Package Review input fingerprint is stale.")

    findings, proposals = _derive_package_findings(inputs, job_id=job_dir.name)
    findings = tuple(sorted(findings, key=lambda item: item.finding_id))
    proposals = tuple(sorted(proposals, key=lambda item: item.proposal_id))
    documents = tuple(item.record for item in inputs.documents)
    result = PackageReviewFindingsV1(
        job_id=job_dir.name,
        input_fingerprint=input_fingerprint,
        parsed_job_sha256=inputs.parsed_job_sha256,
        application_brief_sha256=inputs.application_brief_sha256,
        required_document_plan_sha256=inputs.required_document_plan_sha256,
        document_execution_plan_sha256=inputs.document_execution_plan_sha256,
        reviewer_strategy=PACKAGE_REVIEWER_STRATEGY,
        reviewer_version=PACKAGE_REVIEWER_VERSION,
        documents=documents,
        required_document_ids=tuple(
            item.document_id for item in documents if item.requirement == "required"
        ),
        selected_document_ids=tuple(
            item.document_id for item in documents if item.action == "prepare"
        ),
        reviewed_document_ids=tuple(
            item.document_id for item in documents if item.state == "reviewed"
        ),
        findings=findings,
        correction_proposals=proposals,
        blocker_finding_ids=tuple(
            item.finding_id for item in findings if item.severity == "blocker"
        ),
    )

    final_inputs = _load_package_review_inputs(
        workspace,
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
        cover_letter_schema_path=cover_letter_schema_path,
        research_statement_schema_path=research_statement_schema_path,
        review_findings_schema_path=review_findings_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    final_fingerprint = canonical_sha256(
        _package_review_projection(
            final_inputs,
            package_review_schema_path=package_review_schema_path,
        )
    )
    if final_fingerprint != input_fingerprint:
        raise PackageReviewStageError(
            "Package Review inputs changed while findings were derived."
        )
    return result


def validate_package_review_candidate(
    candidate: object,
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
    package_review_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    research_statement_schema_path: Path | None = None,
    review_findings_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> PackageReviewFindingsV1:
    if not isinstance(candidate, dict):
        raise PackageReviewStageValidationError(
            "Package Review candidate must be a JSON object."
        )
    try:
        validator = compiled_schema_validator(
            _package_review_schema_text(package_review_schema_path)
        )
    except SchemaCompilationError as exc:
        raise PackageReviewStageValidationError(
            "The configured Package Review schema is invalid."
        ) from exc
    if list(validator.iter_errors(candidate)):
        raise PackageReviewStageValidationError(
            "Package Review candidate failed schema validation."
        )
    try:
        validated = PackageReviewFindingsV1.model_validate(candidate)
    except ValidationError as exc:
        raise PackageReviewStageValidationError(
            "Package Review candidate failed semantic validation."
        ) from exc
    expected = build_deterministic_package_review_candidate(
        workspace,
        job_dir,
        input_fingerprint=input_fingerprint,
        package_review_schema_path=package_review_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        cover_letter_schema_path=cover_letter_schema_path,
        research_statement_schema_path=research_statement_schema_path,
        review_findings_schema_path=review_findings_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    if validated != expected:
        raise PackageReviewStageValidationError(
            "Package Review candidate does not match the current deterministic projection."
        )
    return validated


def _load_package_review_inputs(
    workspace: Path,
    job_dir: Path,
    *,
    parsed_job_schema_path: Path | None,
    cover_letter_schema_path: Path | None,
    research_statement_schema_path: Path | None,
    review_findings_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
) -> _PackageReviewInputs:
    try:
        parsed_path = resolve_job_relative_path(job_dir, PARSED_JOB_INPUT_PATH)
        parsed_job = read_json_object(parsed_path)
        parsed_validator = compiled_schema_validator(
            read_resource_text(
                "schemas/parsed_job.schema.json", local_path=parsed_job_schema_path
            )
        )
        if list(parsed_validator.iter_errors(parsed_job)):
            raise PackageReviewStageError("Package Review requires a valid Parsed Job.")

        brief_snapshot = read_optional_safe_bytes(job_dir, APPLICATION_BRIEF_INPUT_PATH)
        if brief_snapshot is None:
            raise PackageReviewStageError(
                "Package Review requires the current Application Brief."
            )
        brief = ApplicationBriefV1.model_validate(load_strict_yaml(brief_snapshot.data))
        plan_path = resolve_job_relative_path(job_dir, REQUIRED_DOCUMENT_PLAN_INPUT_PATH)
        plan = RequiredDocumentPlanV1.model_validate(read_json_object(plan_path))
        if brief.job_id != job_dir.name or plan.job_id != job_dir.name:
            raise PackageReviewStageError("Package Review inputs belong to another job.")
        plan_sha256 = sha256_file(plan_path)
        execution_plan = derive_document_execution_plan(
            plan,
            source_plan_sha256=plan_sha256,
        )
        execution_sha256 = canonical_sha256(execution_plan.model_dump(mode="json"))
        work_by_id = {item.document_id: item for item in execution_plan.items}
        documents = tuple(
            _load_document_inputs(
                workspace,
                job_dir,
                requirement=requirement,
                work_item=work_by_id[requirement.document_id],
                cover_letter_schema_path=cover_letter_schema_path,
                research_statement_schema_path=research_statement_schema_path,
                review_findings_schema_path=review_findings_schema_path,
                parsed_job_schema_path=parsed_job_schema_path,
                required_document_plan_schema_path=required_document_plan_schema_path,
            )
            for requirement in plan.requirements
        )
        return _PackageReviewInputs(
            plan=plan,
            execution_plan=execution_plan,
            parsed_job_sha256=sha256_file(parsed_path),
            application_brief_sha256=brief_snapshot.sha256,
            required_document_plan_sha256=plan_sha256,
            document_execution_plan_sha256=execution_sha256,
            documents=documents,
        )
    except PackageReviewStageError:
        raise
    except (
        InvalidUserFileError,
        UnsafeUserFileError,
        StageStoreError,
        ValidationError,
        OSError,
        UnicodeError,
        ValueError,
    ) as exc:
        raise PackageReviewStageError(
            "Package Review requires a safe current aggregate basis."
        ) from exc


def _load_document_inputs(
    workspace: Path,
    job_dir: Path,
    *,
    requirement: Any,
    work_item: DocumentWorkItemV1,
    cover_letter_schema_path: Path | None,
    research_statement_schema_path: Path | None,
    review_findings_schema_path: Path | None,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
) -> _DocumentInputs:
    base = {
        "document_id": requirement.document_id,
        "normalized_kind": requirement.normalized_kind,
        "requirement": requirement.requirement,
        "action": work_item.action,
        "executor_availability": work_item.executor_availability,
    }
    if work_item.action == "omit":
        return _DocumentInputs(
            record=PackageDocumentReviewV1(
                **base,
                state="omitted",
                reason_codes=("package.document_omitted",),
            )
        )
    if work_item.state == "blocked":
        return _DocumentInputs(
            record=PackageDocumentReviewV1(
                **base,
                state="plan_blocked",
                reason_codes=tuple(
                    sorted({"package.document_plan_blocked", *work_item.reason_codes})
                ),
            )
        )
    if work_item.executor_availability != "available":
        return _DocumentInputs(
            record=PackageDocumentReviewV1(
                **base,
                state="executor_unavailable",
                reason_codes=tuple(
                    sorted({"package.document_executor_unavailable", *work_item.reason_codes})
                ),
            )
        )

    document_kind = requirement.normalized_kind
    paths = _DOCUMENT_PATHS.get(document_kind)
    if paths is None:
        return _DocumentInputs(
            record=PackageDocumentReviewV1(
                **base,
                executor_availability="unregistered",
                state="executor_unavailable",
                reason_codes=("package.document_executor_unavailable",),
            )
        )
    draft_path, review_path, disposition_path = paths
    observed_draft_hash = _optional_core_hash(job_dir, draft_path)
    if observed_draft_hash is None:
        return _DocumentInputs(
            record=PackageDocumentReviewV1(
                **base,
                state="draft_missing",
                reason_codes=("package.document_draft_missing",),
            )
        )

    try:
        raw_draft = read_json_object(resolve_job_relative_path(job_dir, draft_path))
        if document_kind == "cover_letter":
            draft_model = CoverLetterDraftV1.model_validate(raw_draft)
            draft: StructuredDraftV1 = validate_draft_candidate(
                raw_draft,
                workspace=workspace,
                job_dir=job_dir,
                document_id=requirement.document_id,
                input_fingerprint=draft_model.input_fingerprint,
                cover_letter_schema_path=cover_letter_schema_path,
                parsed_job_schema_path=parsed_job_schema_path,
                required_document_plan_schema_path=required_document_plan_schema_path,
                expected_generation_mode=draft_model.generation_mode,
            )
        else:
            draft_model = ResearchStatementDraftV1.model_validate(raw_draft)
            draft = validate_research_statement_draft_candidate(
                raw_draft,
                workspace=workspace,
                job_dir=job_dir,
                document_id=requirement.document_id,
                input_fingerprint=draft_model.input_fingerprint,
                research_statement_schema_path=research_statement_schema_path,
                parsed_job_schema_path=parsed_job_schema_path,
                required_document_plan_schema_path=required_document_plan_schema_path,
                expected_generation_mode=draft_model.generation_mode,
            )
    except (DraftStageError, StageStoreError, ValidationError, OSError, ValueError):
        return _DocumentInputs(
            record=PackageDocumentReviewV1(
                **base,
                state="draft_not_current",
                draft_sha256=observed_draft_hash,
                reason_codes=("package.document_draft_not_current",),
            )
        )
    claim_ids = tuple(
        sorted(claim.claim_id for section in draft.sections for claim in section.claims)
    )

    observed_review_hash = _optional_core_hash(job_dir, review_path)
    if observed_review_hash is None:
        return _DocumentInputs(
            record=PackageDocumentReviewV1(
                **base,
                state="review_missing",
                draft_sha256=observed_draft_hash,
                claim_ids=claim_ids,
                reason_codes=("package.document_review_missing",),
            ),
            draft=draft,
        )
    try:
        raw_review = read_json_object(resolve_job_relative_path(job_dir, review_path))
        review_model = ReviewFindingsV1.model_validate(raw_review)
        if document_kind == "cover_letter":
            review = validate_review_candidate(
                raw_review,
                workspace=workspace,
                job_dir=job_dir,
                document_id=requirement.document_id,
                input_fingerprint=review_model.input_fingerprint,
                review_findings_schema_path=review_findings_schema_path,
                cover_letter_schema_path=cover_letter_schema_path,
                parsed_job_schema_path=parsed_job_schema_path,
                required_document_plan_schema_path=required_document_plan_schema_path,
            )
        else:
            review = validate_research_statement_review_candidate(
                raw_review,
                workspace=workspace,
                job_dir=job_dir,
                document_id=requirement.document_id,
                input_fingerprint=review_model.input_fingerprint,
                review_findings_schema_path=review_findings_schema_path,
                research_statement_schema_path=research_statement_schema_path,
                parsed_job_schema_path=parsed_job_schema_path,
                required_document_plan_schema_path=required_document_plan_schema_path,
            )
    except (ReviewStageError, StageStoreError, ValidationError, OSError, ValueError):
        return _DocumentInputs(
            record=PackageDocumentReviewV1(
                **base,
                state="review_not_current",
                draft_sha256=observed_draft_hash,
                review_findings_sha256=observed_review_hash,
                claim_ids=claim_ids,
                reason_codes=("package.document_review_not_current",),
            ),
            draft=draft,
        )

    dispositions = None
    dispositions_hash = None
    disposition_reason = None
    try:
        disposition_snapshot = read_optional_safe_bytes(job_dir, disposition_path)
        if disposition_snapshot is not None:
            dispositions_hash = disposition_snapshot.sha256
            dispositions = ReviewDispositionsV1.model_validate(
                load_strict_yaml(disposition_snapshot.data)
            )
    except UnsafeUserFileError as exc:
        raise PackageReviewStageError(
            "A Package Review disposition input is unsafe."
        ) from exc
    except (InvalidUserFileError, ValidationError, ValueError):
        disposition_reason = "review.dispositions_invalid"
        dispositions = None

    readiness = derive_document_readiness(
        review,
        document_kind=document_kind,
        draft_sha256=observed_draft_hash,
        review_findings_sha256=observed_review_hash,
        dispositions=dispositions,
        review_dispositions_sha256=(
            dispositions_hash if dispositions is not None else None
        ),
    )
    reasons = set(readiness.reason_codes)
    if disposition_reason is not None:
        reasons.add(disposition_reason)
    return _DocumentInputs(
        record=PackageDocumentReviewV1(
            **base,
            state=readiness.state,
            draft_sha256=observed_draft_hash,
            review_findings_sha256=observed_review_hash,
            review_dispositions_sha256=dispositions_hash,
            document_readiness_sha256=canonical_sha256(
                readiness.model_dump(mode="json")
            ),
            readiness_state=readiness.state,
            claim_ids=claim_ids,
            reason_codes=tuple(sorted(reasons)),
        ),
        draft=draft,
        review=review,
    )


def _package_review_projection(
    inputs: _PackageReviewInputs,
    *,
    package_review_schema_path: Path | None,
) -> dict[str, object]:
    return {
        "stage": "package_review",
        "contract_version": PACKAGE_REVIEW_CONTRACT_VERSION,
        "parsed_job_sha256": inputs.parsed_job_sha256,
        "application_brief_sha256": inputs.application_brief_sha256,
        "required_document_plan_sha256": inputs.required_document_plan_sha256,
        "document_execution_plan_sha256": inputs.document_execution_plan_sha256,
        "documents": [
            item.record.model_dump(mode="json") for item in inputs.documents
        ],
        "schema_sha256": sha256(
            _package_review_schema_text(package_review_schema_path).encode("utf-8")
        ).hexdigest(),
    }


def _derive_package_findings(
    inputs: _PackageReviewInputs,
    *,
    job_id: str,
) -> tuple[list[PackageReviewFindingV1], list[PackageCorrectionProposalV1]]:
    findings: list[PackageReviewFindingV1] = []
    proposals: list[PackageCorrectionProposalV1] = []
    if inputs.plan.blockers:
        findings.append(
            _finding(
                job_id=job_id,
                code="package.document_plan_blocked",
                severity="blocker",
                category="completeness",
                message="The current Required Document Plan contains unresolved blockers.",
                next_action="Resolve the Brief and Required Document Plan before package approval.",
            )
        )

    for item in inputs.documents:
        record = item.record
        if record.state == "reviewed":
            continue
        if record.requirement != "required" and record.action == "omit":
            continue
        severity = "blocker" if record.requirement == "required" else "review"
        code, message, action = _document_finding_text(record.state)
        findings.append(
            _finding(
                job_id=job_id,
                code=code,
                severity=severity,
                category="completeness",
                message=message,
                next_action=action,
                document_ids=(record.document_id,),
                claim_ids=record.claim_ids,
            )
        )

    selected = tuple(
        item for item in inputs.documents if item.record.action == "prepare"
    )
    if not selected:
        findings.append(
            _finding(
                job_id=job_id,
                code="package.no_selected_documents",
                severity="review",
                category="completeness",
                message="No application document is currently selected for preparation.",
                next_action="Confirm that the empty or omitted document set is intentional.",
            )
        )

    claim_entries = [
        (item.record.document_id, claim)
        for item in inputs.documents
        if item.draft is not None
        for section in item.draft.sections
        for claim in section.claims
    ]
    by_text: dict[str, list[tuple[str, ClaimV1]]] = {}
    for document_id, claim in claim_entries:
        if claim.kind != "factual":
            continue
        by_text.setdefault(normalize_claim_text(claim.text), []).append(
            (document_id, claim)
        )
    for entries in by_text.values():
        document_ids = tuple(sorted({document_id for document_id, _ in entries}))
        if len(document_ids) < 2:
            continue
        claim_ids = tuple(sorted(claim.claim_id for _, claim in entries))
        evidence_ids = tuple(
            sorted({value for _, claim in entries for value in claim.evidence_ref_ids})
        )
        receipt_sets = {
            (claim.support_strength, tuple(claim.evidence_ref_ids))
            for _, claim in entries
        }
        if len(receipt_sets) > 1:
            group_proposals = []
            for document_id in document_ids:
                scoped_claims = tuple(
                    sorted(
                        claim.claim_id
                        for owner, claim in entries
                        if owner == document_id
                    )
                )
                instruction = (
                    "Verify the intended Evidence receipt for the repeated assertion, then "
                    "submit any revision through a new guarded Draft candidate."
                )
                proposal = PackageCorrectionProposalV1(
                    proposal_id=stable_package_proposal_id(
                        job_id=job_id,
                        document_id=document_id,
                        claim_ids=scoped_claims,
                        reason_code="package.duplicate_claim_evidence_conflict",
                        instruction=instruction,
                    ),
                    document_id=document_id,
                    claim_ids=scoped_claims,
                    reason_code="package.duplicate_claim_evidence_conflict",
                    instruction=instruction,
                )
                proposals.append(proposal)
                group_proposals.append(proposal.proposal_id)
            findings.append(
                _finding(
                    job_id=job_id,
                    code="package.duplicate_claim_evidence_conflict",
                    severity="blocker",
                    category="contradiction",
                    message=(
                        "The same normalized factual assertion has inconsistent support "
                        "classification or Evidence receipts across documents."
                    ),
                    next_action=(
                        "Verify the authoritative support and regenerate each affected Claim "
                        "through its guarded Draft boundary."
                    ),
                    document_ids=document_ids,
                    claim_ids=claim_ids,
                    evidence_ref_ids=evidence_ids,
                    correction_proposal_ids=tuple(sorted(group_proposals)),
                )
            )
        else:
            findings.append(
                _finding(
                    job_id=job_id,
                    code="package.duplicate_claim_reuse_review",
                    severity="review",
                    category="consistency",
                    message="The same factual assertion is repeated across application documents.",
                    next_action="Confirm that the repetition is intentional and proportionate.",
                    document_ids=document_ids,
                    claim_ids=claim_ids,
                    evidence_ref_ids=evidence_ids,
                )
            )

    by_evidence: dict[str, list[tuple[str, ClaimV1]]] = {}
    for document_id, claim in claim_entries:
        if claim.kind != "factual":
            continue
        for evidence_id in claim.evidence_ref_ids:
            by_evidence.setdefault(evidence_id, []).append((document_id, claim))
    for evidence_id, entries in by_evidence.items():
        document_ids = tuple(sorted({document_id for document_id, _ in entries}))
        texts = {normalize_claim_text(claim.text) for _, claim in entries}
        if len(document_ids) < 2 or len(texts) < 2:
            continue
        findings.append(
            _finding(
                job_id=job_id,
                code="package.shared_evidence_semantic_review",
                severity="review",
                category="consistency",
                message="One Evidence receipt supports different factual wording across documents.",
                next_action="Review whether each wording is semantically supported and proportionate.",
                document_ids=document_ids,
                claim_ids=tuple(sorted(claim.claim_id for _, claim in entries)),
                evidence_ref_ids=(evidence_id,),
            )
        )

    current_document_ids = tuple(
        sorted(item.record.document_id for item in inputs.documents if item.draft is not None)
    )
    if len(current_document_ids) > 1:
        findings.append(
            _finding(
                job_id=job_id,
                code="package.semantic_alignment_review",
                severity="review",
                category="consistency",
                message="Cross-document tone, emphasis, and narrative alignment require human review.",
                next_action="Review the selected documents together before package approval.",
                document_ids=current_document_ids,
            )
        )
    return findings, proposals


def _document_finding_text(state: str) -> tuple[str, str, str]:
    values = {
        "omitted": (
            "package.required_document_omitted",
            "A required document is explicitly omitted.",
            "Change the document choice or confirm a valid plan that does not require it.",
        ),
        "plan_blocked": (
            "package.document_plan_blocked",
            "A selected document is blocked by the current document plan.",
            "Resolve the Required Document Plan before preparing the document.",
        ),
        "executor_unavailable": (
            "package.document_executor_unavailable",
            "A selected document has no available guarded executor.",
            "Add a guarded executor or revise the confirmed document plan.",
        ),
        "draft_missing": (
            "package.document_draft_missing",
            "A selected document has no structured Draft.",
            "Prepare and promote the document through its guarded Draft stage.",
        ),
        "draft_not_current": (
            "package.document_draft_not_current",
            "A selected document Draft is stale, invalid, or locally changed.",
            "Resolve Draft drift and promote a current validated candidate.",
        ),
        "review_missing": (
            "package.document_review_missing",
            "A selected document has no deterministic Review.",
            "Run deterministic Review for the current Draft.",
        ),
        "review_not_current": (
            "package.document_review_not_current",
            "A selected document Review is stale, invalid, or locally changed.",
            "Run deterministic Review again against the current Draft.",
        ),
        "blocked": (
            "package.document_review_blocked",
            "A selected document has non-waivable Review blockers.",
            "Resolve the document blockers and regenerate its Draft and Review.",
        ),
        "review_required": (
            "package.document_review_required",
            "A selected document has unresolved Review findings.",
            "Record explicit dispositions for every current non-blocker finding.",
        ),
        "revision_required": (
            "package.document_revision_required",
            "A selected document has an explicit revision request.",
            "Revise the targeted document through a new guarded Draft candidate.",
        ),
    }
    return values[state]


def _finding(
    *,
    job_id: str,
    code: str,
    severity: str,
    category: str,
    message: str,
    next_action: str,
    document_ids: tuple[str, ...] = (),
    claim_ids: tuple[str, ...] = (),
    evidence_ref_ids: tuple[str, ...] = (),
    correction_proposal_ids: tuple[str, ...] = (),
) -> PackageReviewFindingV1:
    ordered_documents = tuple(sorted(document_ids))
    ordered_claims = tuple(sorted(claim_ids))
    ordered_evidence = tuple(sorted(evidence_ref_ids))
    ordered_proposals = tuple(sorted(correction_proposal_ids))
    return PackageReviewFindingV1(
        finding_id=stable_package_finding_id(
            job_id=job_id,
            code=code,
            message=message,
            document_ids=ordered_documents,
            claim_ids=ordered_claims,
            evidence_ref_ids=ordered_evidence,
            correction_proposal_ids=ordered_proposals,
        ),
        code=code,
        severity=severity,
        category=category,
        message=message,
        next_action=next_action,
        document_ids=ordered_documents,
        claim_ids=ordered_claims,
        evidence_ref_ids=ordered_evidence,
        correction_proposal_ids=ordered_proposals,
    )


def _optional_core_hash(job_dir: Path, relative_path: str) -> str | None:
    path = resolve_job_relative_path(job_dir, relative_path)
    if not path.exists() and not path.is_symlink():
        return None
    try:
        return sha256_file(path)
    except (OSError, StageStoreError) as exc:
        raise PackageReviewStageError(
            "A Package Review document input is unsafe or unreadable."
        ) from exc


def _package_review_schema_text(path: Path | None) -> str:
    try:
        return read_resource_text(
            "schemas/package-review-findings.schema.json",
            local_path=path,
        )
    except (OSError, UnicodeError) as exc:
        raise PackageReviewStageError(
            "The Package Review Findings schema is not readable."
        ) from exc
