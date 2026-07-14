from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
import json
from pathlib import Path
import unicodedata

from jsonschema import Draft202012Validator
from pydantic import ValidationError

from canisend.decision_models import ApplicationBriefV1
from canisend.draft_models import (
    CoverLetterDraftV1,
    ReviewFindingV1,
    ReviewFindingsV1,
    stable_finding_id,
)
from canisend.resource_files import read_resource_text
from canisend.stage_store import (
    StageStoreError,
    read_json_object,
    resolve_job_relative_path,
    sha256_file,
)
from canisend.stages.brief_stage import APPLICATION_BRIEF_INPUT_PATH
from canisend.stages.draft_stage import (
    COVER_LETTER_DRAFT_OUTPUT_PATH,
    DraftStageError,
    validate_draft_candidate,
)
from canisend.user_file_store import (
    InvalidUserFileError,
    UnsafeUserFileError,
    load_strict_yaml,
    read_optional_safe_bytes,
)


REVIEW_CONTRACT_VERSION = "1.0.0"
REVIEWER_STRATEGY = "deterministic.cover_letter_review"
REVIEWER_VERSION = "1.0.0"
REVIEW_FINDINGS_OUTPUT_PATH = "review_findings.json"

_REQUIRED_SECTION_IDS = ("body", "closing", "opening")
_LONG_CLAIM_CHARACTERS = 1_200


class ReviewStageError(ValueError):
    """Raised when a current Draft cannot be reviewed safely."""


class ReviewStageValidationError(ReviewStageError):
    """Raised when a Review Findings candidate cannot be accepted."""


@dataclass(frozen=True)
class _ReviewInputs:
    draft: CoverLetterDraftV1
    brief: ApplicationBriefV1
    draft_sha256: str


def review_precondition_reasons(
    workspace: Path,
    job_dir: Path,
    *,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> tuple[str, ...]:
    try:
        _load_review_inputs(
            workspace,
            job_dir,
            cover_letter_schema_path=cover_letter_schema_path,
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
    review_findings_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> dict[str, object]:
    inputs = _load_review_inputs(
        workspace,
        job_dir,
        cover_letter_schema_path=cover_letter_schema_path,
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
    review_findings_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> str:
    return _projection_sha256(
        review_input_projection(
            workspace,
            job_dir,
            review_findings_schema_path=review_findings_schema_path,
            cover_letter_schema_path=cover_letter_schema_path,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
        )
    )


def build_deterministic_review_candidate(
    workspace: Path,
    job_dir: Path,
    *,
    input_fingerprint: str,
    review_findings_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> ReviewFindingsV1:
    inputs = _load_review_inputs(
        workspace,
        job_dir,
        cover_letter_schema_path=cover_letter_schema_path,
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
            _derive_findings(inputs.draft, inputs.brief),
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
        reviewer_strategy=REVIEWER_STRATEGY,
        reviewer_version=REVIEWER_VERSION,
        findings=findings,
        blocker_finding_ids=blockers,
    )
    final_inputs = _load_review_inputs(
        workspace,
        job_dir,
        cover_letter_schema_path=cover_letter_schema_path,
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
    review_findings_schema_path: Path | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> ReviewFindingsV1:
    if not isinstance(candidate, dict):
        raise ReviewStageValidationError("Review candidate must be a JSON object.")
    try:
        schema = json.loads(_review_findings_schema_text(review_findings_schema_path))
        Draft202012Validator.check_schema(schema)
    except (json.JSONDecodeError, ValueError) as exc:
        raise ReviewStageValidationError(
            "The configured Review Findings schema is invalid."
        ) from exc
    if list(Draft202012Validator(schema).iter_errors(candidate)):
        raise ReviewStageValidationError("Review candidate failed schema validation.")
    try:
        validated = ReviewFindingsV1.model_validate(candidate)
    except ValidationError as exc:
        raise ReviewStageValidationError("Review candidate failed semantic validation.") from exc

    expected = build_deterministic_review_candidate(
        workspace,
        job_dir,
        input_fingerprint=input_fingerprint,
        review_findings_schema_path=review_findings_schema_path,
        cover_letter_schema_path=cover_letter_schema_path,
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
    cover_letter_schema_path: Path | None,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
) -> _ReviewInputs:
    try:
        draft_path = resolve_job_relative_path(job_dir, COVER_LETTER_DRAFT_OUTPUT_PATH)
        raw_draft = read_json_object(draft_path)
        draft = CoverLetterDraftV1.model_validate(raw_draft)
        draft = validate_draft_candidate(
            raw_draft,
            workspace=workspace,
            job_dir=job_dir,
            input_fingerprint=draft.input_fingerprint,
            cover_letter_schema_path=cover_letter_schema_path,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
            expected_generation_mode=draft.generation_mode,
        )
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
    draft: CoverLetterDraftV1,
    brief: ApplicationBriefV1,
) -> tuple[ReviewFindingV1, ...]:
    findings: list[ReviewFindingV1] = []
    claims = tuple(claim for section in draft.sections for claim in section.claims)

    section_ids = {section.section_id for section in draft.sections}
    for section_id in _REQUIRED_SECTION_IDS:
        if section_id not in section_ids:
            findings.append(
                _finding(
                    draft,
                    code="document.section_missing",
                    severity="blocker",
                    category="completeness",
                    message=f"The Cover Letter is missing its {section_id} section.",
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
                    message="A Claim block is unusually long for a Cover Letter.",
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
    draft: CoverLetterDraftV1,
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
