from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
import json
from pathlib import Path
import unicodedata

from pydantic import ValidationError

from canisend.decision_models import ApplicationBriefV1
from canisend.draft_models import (
    CoverLetterDraftV1,
    ResearchStatementDraftV1,
    ReviewFindingV1,
    ReviewFindingsV1,
    stable_finding_id,
)
from canisend.resource_files import read_resource_text
from canisend.schema_validation import (
    SchemaCompilationError,
    compiled_schema_validator,
)
from canisend.stage_store import (
    StageStoreError,
    read_json_object,
    resolve_job_relative_path,
    sha256_file,
)
from canisend.stages.brief_stage import APPLICATION_BRIEF_INPUT_PATH
from canisend.stages.draft_stage import (
    COVER_LETTER_DRAFT_OUTPUT_PATH,
    RESEARCH_STATEMENT_DRAFT_OUTPUT_PATH,
    DraftDocumentKind,
    DraftStageError,
    StructuredDraftV1,
    validate_draft_candidate,
    validate_research_statement_draft_candidate,
)
from canisend.user_file_store import (
    InvalidUserFileError,
    UnsafeUserFileError,
    load_strict_yaml,
    read_optional_safe_bytes,
)


REVIEW_CONTRACT_VERSION = "1.0.0"
REVIEWER_STRATEGY = "deterministic.cover_letter_review"
RESEARCH_STATEMENT_REVIEWER_STRATEGY = "deterministic.research_statement_review"
REVIEWER_VERSION = "1.0.0"
REVIEW_FINDINGS_OUTPUT_PATH = "review_findings.json"
RESEARCH_STATEMENT_REVIEW_FINDINGS_OUTPUT_PATH = (
    "research_statement_review_findings.json"
)

_REQUIRED_SECTION_IDS = {
    "cover_letter": ("body", "closing", "opening"),
    "research_statement": (
        "future_agenda",
        "research_contributions",
        "research_overview",
    ),
}
_LONG_CLAIM_CHARACTERS = 1_200


class ReviewStageError(ValueError):
    """Raised when a current Draft cannot be reviewed safely."""


class ReviewStageValidationError(ReviewStageError):
    """Raised when a Review Findings candidate cannot be accepted."""


@dataclass(frozen=True)
class _ReviewInputs:
    draft: StructuredDraftV1
    brief: ApplicationBriefV1
    draft_sha256: str
    document_kind: DraftDocumentKind


def review_precondition_reasons(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> tuple[str, ...]:
    return _review_precondition_reasons(
        workspace,
        job_dir,
        document_kind="cover_letter",
        document_id=document_id,
        draft_schema_path=cover_letter_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def research_statement_review_precondition_reasons(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    research_statement_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> tuple[str, ...]:
    return _review_precondition_reasons(
        workspace,
        job_dir,
        document_kind="research_statement",
        document_id=document_id,
        draft_schema_path=research_statement_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def _review_precondition_reasons(
    workspace: Path,
    job_dir: Path,
    *,
    document_kind: DraftDocumentKind,
    document_id: str | None,
    draft_schema_path: Path | None,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
) -> tuple[str, ...]:
    try:
        _load_review_inputs(
            workspace,
            job_dir,
            document_kind=document_kind,
            document_id=document_id,
            draft_schema_path=draft_schema_path,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
        )
    except ReviewStageError:
        return ("input_not_ready:draft_review",)
    return ()


def review_input_projection(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    review_findings_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> dict[str, object]:
    return _review_input_projection(
        workspace,
        job_dir,
        document_kind="cover_letter",
        document_id=document_id,
        review_findings_schema_path=review_findings_schema_path,
        draft_schema_path=cover_letter_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def research_statement_review_input_projection(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    review_findings_schema_path: Path | None = None,
    research_statement_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> dict[str, object]:
    return _review_input_projection(
        workspace,
        job_dir,
        document_kind="research_statement",
        document_id=document_id,
        review_findings_schema_path=review_findings_schema_path,
        draft_schema_path=research_statement_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def _review_input_projection(
    workspace: Path,
    job_dir: Path,
    *,
    document_kind: DraftDocumentKind,
    document_id: str | None,
    review_findings_schema_path: Path | None,
    draft_schema_path: Path | None,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
) -> dict[str, object]:
    inputs = _load_review_inputs(
        workspace,
        job_dir,
        document_kind=document_kind,
        document_id=document_id,
        draft_schema_path=draft_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    return _review_projection(
        inputs,
        review_findings_schema_path=review_findings_schema_path,
    )


def _review_projection(
    inputs: _ReviewInputs,
    *,
    review_findings_schema_path: Path | None,
) -> dict[str, object]:
    return {
        "stage": "review",
        "contract_version": REVIEW_CONTRACT_VERSION,
        "draft_sha256": inputs.draft_sha256,
        "draft_input_fingerprint": inputs.draft.input_fingerprint,
        **inputs.draft.basis.model_dump(mode="json"),
        "schema_sha256": sha256(
            _review_findings_schema_text(review_findings_schema_path).encode("utf-8")
        ).hexdigest(),
    }


def review_input_fingerprint(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    review_findings_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> str:
    return _projection_sha256(
        review_input_projection(
            workspace,
            job_dir,
            document_id=document_id,
            review_findings_schema_path=review_findings_schema_path,
            cover_letter_schema_path=cover_letter_schema_path,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
        )
    )


def research_statement_review_input_fingerprint(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    review_findings_schema_path: Path | None = None,
    research_statement_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> str:
    return _projection_sha256(
        research_statement_review_input_projection(
            workspace,
            job_dir,
            document_id=document_id,
            review_findings_schema_path=review_findings_schema_path,
            research_statement_schema_path=research_statement_schema_path,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
        )
    )


def build_deterministic_review_candidate(
    workspace: Path,
    job_dir: Path,
    *,
    input_fingerprint: str,
    document_id: str | None = None,
    review_findings_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> ReviewFindingsV1:
    return _build_deterministic_review_candidate(
        workspace,
        job_dir,
        document_kind="cover_letter",
        input_fingerprint=input_fingerprint,
        document_id=document_id,
        review_findings_schema_path=review_findings_schema_path,
        draft_schema_path=cover_letter_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def build_deterministic_research_statement_review_candidate(
    workspace: Path,
    job_dir: Path,
    *,
    input_fingerprint: str,
    document_id: str | None = None,
    review_findings_schema_path: Path | None = None,
    research_statement_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> ReviewFindingsV1:
    return _build_deterministic_review_candidate(
        workspace,
        job_dir,
        document_kind="research_statement",
        input_fingerprint=input_fingerprint,
        document_id=document_id,
        review_findings_schema_path=review_findings_schema_path,
        draft_schema_path=research_statement_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def _build_deterministic_review_candidate(
    workspace: Path,
    job_dir: Path,
    *,
    document_kind: DraftDocumentKind,
    input_fingerprint: str,
    document_id: str | None,
    review_findings_schema_path: Path | None,
    draft_schema_path: Path | None,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
) -> ReviewFindingsV1:
    inputs = _load_review_inputs(
        workspace,
        job_dir,
        document_kind=document_kind,
        document_id=document_id,
        draft_schema_path=draft_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    current_fingerprint = _projection_sha256(
        _review_projection(
            inputs,
            review_findings_schema_path=review_findings_schema_path,
        )
    )
    if current_fingerprint != input_fingerprint:
        raise ReviewStageError("Review input fingerprint is stale.")
    findings = tuple(
        sorted(
            _derive_findings(
                inputs.draft,
                inputs.brief,
                document_kind=document_kind,
            ),
            key=lambda finding: finding.finding_id,
        )
    )
    blockers = tuple(
        finding.finding_id for finding in findings if finding.severity == "blocker"
    )
    result = ReviewFindingsV1(
        job_id=job_dir.name,
        document_id=inputs.draft.document_id,
        input_fingerprint=input_fingerprint,
        draft_sha256=inputs.draft_sha256,
        reviewer_strategy=(
            REVIEWER_STRATEGY
            if document_kind == "cover_letter"
            else RESEARCH_STATEMENT_REVIEWER_STRATEGY
        ),
        reviewer_version=REVIEWER_VERSION,
        findings=findings,
        blocker_finding_ids=blockers,
    )
    final_inputs = _load_review_inputs(
        workspace,
        job_dir,
        document_kind=document_kind,
        document_id=document_id,
        draft_schema_path=draft_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    final_fingerprint = _projection_sha256(
        _review_projection(
            final_inputs,
            review_findings_schema_path=review_findings_schema_path,
        )
    )
    if final_fingerprint != input_fingerprint:
        raise ReviewStageError("Review inputs changed while findings were derived.")
    return result


def validate_review_candidate(
    candidate: object,
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
    document_id: str | None = None,
    review_findings_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> ReviewFindingsV1:
    return _validate_review_candidate(
        candidate,
        workspace=workspace,
        job_dir=job_dir,
        input_fingerprint=input_fingerprint,
        document_kind="cover_letter",
        document_id=document_id,
        review_findings_schema_path=review_findings_schema_path,
        draft_schema_path=cover_letter_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def validate_research_statement_review_candidate(
    candidate: object,
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
    document_id: str | None = None,
    review_findings_schema_path: Path | None = None,
    research_statement_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> ReviewFindingsV1:
    return _validate_review_candidate(
        candidate,
        workspace=workspace,
        job_dir=job_dir,
        input_fingerprint=input_fingerprint,
        document_kind="research_statement",
        document_id=document_id,
        review_findings_schema_path=review_findings_schema_path,
        draft_schema_path=research_statement_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def _validate_review_candidate(
    candidate: object,
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
    document_kind: DraftDocumentKind,
    document_id: str | None,
    review_findings_schema_path: Path | None,
    draft_schema_path: Path | None,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
) -> ReviewFindingsV1:
    if not isinstance(candidate, dict):
        raise ReviewStageValidationError("Review candidate must be a JSON object.")
    try:
        validator = compiled_schema_validator(
            _review_findings_schema_text(review_findings_schema_path)
        )
    except SchemaCompilationError as exc:
        raise ReviewStageValidationError(
            "The configured Review Findings schema is invalid."
        ) from exc
    if list(validator.iter_errors(candidate)):
        raise ReviewStageValidationError("Review candidate failed schema validation.")
    try:
        validated = ReviewFindingsV1.model_validate(candidate)
    except ValidationError as exc:
        raise ReviewStageValidationError("Review candidate failed semantic validation.") from exc

    expected = _build_deterministic_review_candidate(
        workspace,
        job_dir,
        document_kind=document_kind,
        input_fingerprint=input_fingerprint,
        document_id=document_id,
        review_findings_schema_path=review_findings_schema_path,
        draft_schema_path=draft_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    if validated != expected:
        raise ReviewStageValidationError(
            "Review candidate does not match the current deterministic projection."
        )
    return validated


def _load_review_inputs(
    workspace: Path,
    job_dir: Path,
    *,
    document_kind: DraftDocumentKind,
    document_id: str | None,
    draft_schema_path: Path | None,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
) -> _ReviewInputs:
    try:
        draft_path = resolve_job_relative_path(
            job_dir,
            (
                COVER_LETTER_DRAFT_OUTPUT_PATH
                if document_kind == "cover_letter"
                else RESEARCH_STATEMENT_DRAFT_OUTPUT_PATH
            ),
        )
        raw_draft = read_json_object(draft_path)
        if document_kind == "cover_letter":
            cover_draft = CoverLetterDraftV1.model_validate(raw_draft)
            draft: StructuredDraftV1 = validate_draft_candidate(
                raw_draft,
                workspace=workspace,
                job_dir=job_dir,
                input_fingerprint=cover_draft.input_fingerprint,
                document_id=document_id,
                cover_letter_schema_path=draft_schema_path,
                parsed_job_schema_path=parsed_job_schema_path,
                required_document_plan_schema_path=required_document_plan_schema_path,
                expected_generation_mode=cover_draft.generation_mode,
            )
        else:
            research_draft = ResearchStatementDraftV1.model_validate(raw_draft)
            draft = validate_research_statement_draft_candidate(
                raw_draft,
                workspace=workspace,
                job_dir=job_dir,
                input_fingerprint=research_draft.input_fingerprint,
                document_id=document_id,
                research_statement_schema_path=draft_schema_path,
                parsed_job_schema_path=parsed_job_schema_path,
                required_document_plan_schema_path=required_document_plan_schema_path,
                expected_generation_mode=research_draft.generation_mode,
            )
        if document_id is not None and draft.document_id != document_id:
            raise ReviewStageError("Review target does not match the Draft document.")
        brief_snapshot = read_optional_safe_bytes(job_dir, APPLICATION_BRIEF_INPUT_PATH)
        if brief_snapshot is None:
            raise ReviewStageError("Review requires the current Application Brief.")
        brief = ApplicationBriefV1.model_validate(load_strict_yaml(brief_snapshot.data))
        if brief.job_id != job_dir.name:
            raise ReviewStageError("Review inputs belong to different jobs.")
        return _ReviewInputs(
            draft=draft,
            brief=brief,
            draft_sha256=sha256_file(draft_path),
            document_kind=document_kind,
        )
    except ReviewStageError:
        raise
    except (
        DraftStageError,
        InvalidUserFileError,
        UnsafeUserFileError,
        StageStoreError,
        ValidationError,
        OSError,
        UnicodeError,
        ValueError,
    ) as exc:
        raise ReviewStageError("Review requires a current validated Draft.") from exc


def _derive_findings(
    draft: StructuredDraftV1,
    brief: ApplicationBriefV1,
    *,
    document_kind: DraftDocumentKind,
) -> tuple[ReviewFindingV1, ...]:
    findings: list[ReviewFindingV1] = []
    claims = tuple(claim for section in draft.sections for claim in section.claims)
    document_label = (
        "Cover Letter"
        if document_kind == "cover_letter"
        else "Research Statement"
    )

    section_ids = {section.section_id for section in draft.sections}
    for section_id in _REQUIRED_SECTION_IDS[document_kind]:
        if section_id not in section_ids:
            findings.append(
                _finding(
                    draft,
                    code="document.section_missing",
                    severity="blocker",
                    category="completeness",
                    message=f"The {document_label} is missing its {section_id} section.",
                    next_action=(
                        f"Add a structured {section_id} section with explicit Claim blocks."
                    ),
                )
            )

    for claim in claims:
        if claim.kind == "factual" and claim.support_strength == "unsupported":
            findings.append(
                _finding(
                    draft,
                    code="claim.unsupported",
                    severity="blocker",
                    category="support",
                    message="A factual claim has no current Evidence support.",
                    next_action=(
                        "Remove the claim or attach current Evidence and narrow its wording."
                    ),
                    claim_ids=(claim.claim_id,),
                    criterion_ids=claim.criterion_ids,
                )
            )
        elif claim.kind == "factual" and claim.support_strength == "partial":
            findings.append(
                _finding(
                    draft,
                    code="claim.partial_support",
                    severity="review",
                    category="support",
                    message="A factual claim has only partial structural support.",
                    next_action="Confirm that the wording is proportional to the linked Evidence.",
                    claim_ids=(claim.claim_id,),
                    criterion_ids=claim.criterion_ids,
                    evidence_ref_ids=claim.evidence_ref_ids,
                )
            )
        elif claim.kind == "factual" and claim.support_strength == "strong":
            findings.append(
                _finding(
                    draft,
                    code="claim.semantic_support_review",
                    severity="review",
                    category="support",
                    message=(
                        "A structurally supported factual claim still requires semantic review."
                    ),
                    next_action=(
                        "Confirm that every linked Evidence item supports the claim as worded."
                    ),
                    claim_ids=(claim.claim_id,),
                    criterion_ids=claim.criterion_ids,
                    evidence_ref_ids=claim.evidence_ref_ids,
                )
            )
        else:
            findings.append(
                _finding(
                    draft,
                    code="claim.semantic_kind_review",
                    severity="review",
                    category="compliance",
                    message="A non-factual Claim classification requires semantic review.",
                    next_action=(
                        "Confirm that the wording matches its declared Claim kind and does "
                        "not state an applicant fact without Evidence."
                    ),
                    claim_ids=(claim.claim_id,),
                    criterion_ids=claim.criterion_ids,
                )
            )

        if len(claim.text) > _LONG_CLAIM_CHARACTERS:
            findings.append(
                _finding(
                    draft,
                    code="style.claim_length",
                    severity="warning",
                    category="style",
                    message=f"A Claim block is unusually long for a {document_label}.",
                    next_action="Consider splitting or tightening the Claim block.",
                    claim_ids=(claim.claim_id,),
                )
            )

    normalized_exclusions = tuple(
        item
        for item in (_normalized_text(value) for value in brief.exclusions.items)
        if len(item) >= 4
    )
    for claim in claims:
        normalized_claim = _normalized_text(claim.text)
        if any(exclusion in normalized_claim for exclusion in normalized_exclusions):
            findings.append(
                _finding(
                    draft,
                    code="brief.exclusion_conflict",
                    severity="blocker",
                    category="contradiction",
                    message="A Claim conflicts with a confirmed Application Brief exclusion.",
                    next_action="Remove or rewrite the Claim to respect the confirmed exclusion.",
                    claim_ids=(claim.claim_id,),
                )
            )

    by_text: dict[str, list[str]] = {}
    for claim in claims:
        by_text.setdefault(_normalized_text(claim.text), []).append(claim.claim_id)
    for duplicate_ids in by_text.values():
        if len(duplicate_ids) < 2:
            continue
        findings.append(
            _finding(
                draft,
                code="claim.duplicate_wording",
                severity="review",
                category="consistency",
                message="The same wording appears in more than one semantic Claim block.",
                next_action="Consolidate duplicate wording or make each Claim's purpose explicit.",
                claim_ids=tuple(sorted(duplicate_ids)),
            )
        )
    return tuple(findings)


def _finding(
    draft: StructuredDraftV1,
    *,
    code: str,
    severity: str,
    category: str,
    message: str,
    next_action: str,
    claim_ids: tuple[str, ...] = (),
    criterion_ids: tuple[str, ...] = (),
    evidence_ref_ids: tuple[str, ...] = (),
) -> ReviewFindingV1:
    ordered_claims = tuple(sorted(claim_ids))
    ordered_criteria = tuple(sorted(criterion_ids))
    ordered_evidence = tuple(sorted(evidence_ref_ids))
    return ReviewFindingV1(
        finding_id=stable_finding_id(
            job_id=draft.job_id,
            document_id=draft.document_id,
            code=code,
            message=message,
            claim_ids=ordered_claims,
            criterion_ids=ordered_criteria,
            evidence_ref_ids=ordered_evidence,
        ),
        code=code,
        severity=severity,
        category=category,
        message=message,
        next_action=next_action,
        claim_ids=ordered_claims,
        criterion_ids=ordered_criteria,
        evidence_ref_ids=ordered_evidence,
    )


def _normalized_text(value: str) -> str:
    return " ".join(unicodedata.normalize("NFKC", value).split()).casefold()


def _projection_sha256(value: object) -> str:
    encoded = json.dumps(
        value,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return sha256(encoded).hexdigest()


def _review_findings_schema_text(path: Path | None) -> str:
    try:
        return read_resource_text("schemas/review-findings.schema.json", local_path=path)
    except (OSError, UnicodeError) as exc:
        raise ReviewStageError("The Review Findings schema is not readable.") from exc
