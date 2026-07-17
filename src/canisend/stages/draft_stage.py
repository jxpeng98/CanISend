from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
import json
from pathlib import Path
from typing import Any, Literal

from pydantic import ValidationError

from canisend.decision_models import (
    ApplicationBriefV1,
    ApplicationDecisionV1,
    CriteriaCatalogV1,
    CriterionMatchesV1,
    EvidenceCatalogV1,
    RequiredDocumentPlanV1,
)
from canisend.draft_models import (
    CoverLetterDraftV1,
    DraftBasisV1,
    DraftGenerationMode,
    ResearchStatementDraftV1,
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
from canisend.stages.brief_stage import (
    APPLICATION_BRIEF_INPUT_PATH,
    APPLICATION_DECISION_INPUT_PATH,
    REQUIRED_DOCUMENT_PLAN_OUTPUT_PATH,
    brief_input_fingerprint,
    brief_precondition_reasons,
    validate_brief_candidate,
)
from canisend.user_file_store import (
    InvalidUserFileError,
    UnsafeUserFileError,
    load_strict_yaml,
    read_optional_safe_bytes,
)


DRAFT_CONTRACT_VERSION = "1.0.0"
HOST_AGENT_DRAFT_GENERATOR_STRATEGY = "host_agent.cover_letter"
CONFIGURED_PROVIDER_DRAFT_GENERATOR_STRATEGY = "configured_provider.cover_letter"
HOST_AGENT_RESEARCH_STATEMENT_GENERATOR_STRATEGY = "host_agent.research_statement"
# Compatibility alias for host-agent callers that predate configured-provider Draft.
DRAFT_GENERATOR_STRATEGY = HOST_AGENT_DRAFT_GENERATOR_STRATEGY
DRAFT_GENERATOR_VERSION = "1.0.0"

PARSED_JOB_INPUT_PATH = "parsed_job.json"
CRITERIA_INPUT_PATH = "criteria.json"
EVIDENCE_CATALOG_INPUT_PATH = "evidence_catalog.json"
CRITERION_MATCHES_INPUT_PATH = "criterion_matches.json"
COVER_LETTER_DRAFT_OUTPUT_PATH = "cover_letter_draft.json"
RESEARCH_STATEMENT_DRAFT_OUTPUT_PATH = "research_statement_draft.json"

DraftDocumentKind = Literal["cover_letter", "research_statement"]
StructuredDraftV1 = CoverLetterDraftV1 | ResearchStatementDraftV1


class DraftStageError(ValueError):
    """Raised when current inputs cannot support a guarded Draft task."""


class DraftStageValidationError(DraftStageError):
    """Raised when a Cover Letter Draft candidate cannot be accepted."""


@dataclass(frozen=True)
class _DraftInputs:
    parsed_job: dict[str, Any]
    criteria: CriteriaCatalogV1
    evidence: EvidenceCatalogV1
    matches: CriterionMatchesV1
    decision: ApplicationDecisionV1
    brief: ApplicationBriefV1
    plan: RequiredDocumentPlanV1
    basis: DraftBasisV1
    document_kind: DraftDocumentKind
    document_id: str | None


def draft_precondition_reasons(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> tuple[str, ...]:
    """Return one body-free reason when Draft inputs are not ready."""

    return _draft_precondition_reasons(
        workspace,
        job_dir,
        document_kind="cover_letter",
        document_id=document_id,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def research_statement_draft_precondition_reasons(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> tuple[str, ...]:
    """Return one body-free reason when Research Statement inputs are not ready."""

    return _draft_precondition_reasons(
        workspace,
        job_dir,
        document_kind="research_statement",
        document_id=document_id,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def _draft_precondition_reasons(
    workspace: Path,
    job_dir: Path,
    *,
    document_kind: DraftDocumentKind,
    document_id: str | None,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
) -> tuple[str, ...]:
    upstream = brief_precondition_reasons(workspace, job_dir)
    if upstream:
        return upstream
    try:
        inputs = _load_draft_inputs(
            workspace,
            job_dir,
            document_kind=document_kind,
            document_id=document_id,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
            validate_plan=True,
        )
    except DraftStageError:
        return ("input_not_ready:draft_inputs",)
    return _draft_input_reason(inputs, document_id=document_id)


def draft_input_projection(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> dict[str, object]:
    return _draft_input_projection(
        workspace,
        job_dir,
        document_kind="cover_letter",
        document_id=document_id,
        draft_schema_path=cover_letter_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def research_statement_draft_input_projection(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    research_statement_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> dict[str, object]:
    return _draft_input_projection(
        workspace,
        job_dir,
        document_kind="research_statement",
        document_id=document_id,
        draft_schema_path=research_statement_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )


def _draft_input_projection(
    workspace: Path,
    job_dir: Path,
    *,
    document_kind: DraftDocumentKind,
    document_id: str | None,
    draft_schema_path: Path | None,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
) -> dict[str, object]:
    inputs = _load_draft_inputs(
        workspace,
        job_dir,
        document_kind=document_kind,
        document_id=document_id,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
        validate_plan=True,
    )
    if _draft_input_reason(inputs, document_id=document_id):
        raise DraftStageError("Draft inputs are not current and ready.")
    return _draft_projection(
        inputs,
        draft_schema_path=draft_schema_path,
    )


def _draft_projection(
    inputs: _DraftInputs,
    *,
    draft_schema_path: Path | None,
) -> dict[str, object]:
    return {
        "stage": "draft",
        "contract_version": DRAFT_CONTRACT_VERSION,
        **inputs.basis.model_dump(mode="json"),
        f"{inputs.document_kind}_document_id": inputs.document_id,
        "schema_sha256": sha256(
            _draft_schema_text(
                inputs.document_kind,
                draft_schema_path,
            ).encode("utf-8")
        ).hexdigest(),
    }


def draft_input_fingerprint(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> str:
    return _projection_sha256(
        draft_input_projection(
            workspace,
            job_dir,
            document_id=document_id,
            cover_letter_schema_path=cover_letter_schema_path,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
        )
    )


def research_statement_draft_input_fingerprint(
    workspace: Path,
    job_dir: Path,
    *,
    document_id: str | None = None,
    research_statement_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> str:
    return _projection_sha256(
        research_statement_draft_input_projection(
            workspace,
            job_dir,
            document_id=document_id,
            research_statement_schema_path=research_statement_schema_path,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
        )
    )


def validate_draft_candidate(
    candidate: object,
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
    document_id: str | None = None,
    cover_letter_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
    expected_generation_mode: DraftGenerationMode = "host_agent",
) -> CoverLetterDraftV1:
    validated = _validate_draft_candidate(
        candidate,
        workspace=workspace,
        job_dir=job_dir,
        input_fingerprint=input_fingerprint,
        document_kind="cover_letter",
        document_id=document_id,
        draft_schema_path=cover_letter_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
        expected_generation_mode=expected_generation_mode,
    )
    if not isinstance(validated, CoverLetterDraftV1):
        raise DraftStageValidationError("Draft candidate uses the wrong document contract.")
    return validated


def validate_research_statement_draft_candidate(
    candidate: object,
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
    document_id: str | None = None,
    research_statement_schema_path: Path | None = None,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
    expected_generation_mode: DraftGenerationMode = "host_agent",
) -> ResearchStatementDraftV1:
    validated = _validate_draft_candidate(
        candidate,
        workspace=workspace,
        job_dir=job_dir,
        input_fingerprint=input_fingerprint,
        document_kind="research_statement",
        document_id=document_id,
        draft_schema_path=research_statement_schema_path,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
        expected_generation_mode=expected_generation_mode,
    )
    if not isinstance(validated, ResearchStatementDraftV1):
        raise DraftStageValidationError("Draft candidate uses the wrong document contract.")
    return validated


def _validate_draft_candidate(
    candidate: object,
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
    document_kind: DraftDocumentKind,
    document_id: str | None,
    draft_schema_path: Path | None,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
    expected_generation_mode: DraftGenerationMode,
) -> StructuredDraftV1:
    if not isinstance(candidate, dict):
        raise DraftStageValidationError("Draft candidate must be a JSON object.")
    if document_kind == "research_statement" and expected_generation_mode != "host_agent":
        raise DraftStageValidationError(
            "Research Statement Draft supports host-agent generation only."
        )
    if brief_precondition_reasons(workspace, job_dir):
        raise DraftStageValidationError("Draft inputs are not current and ready.")
    try:
        inputs = _load_draft_inputs(
            workspace,
            job_dir,
            document_kind=document_kind,
            document_id=document_id,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
            validate_plan=True,
        )
    except DraftStageError as exc:
        raise DraftStageValidationError("Draft inputs are not current and ready.") from exc
    if _draft_input_reason(inputs, document_id=document_id):
        raise DraftStageValidationError("Draft inputs are not current and ready.")

    current_fingerprint = _projection_sha256(
        _draft_projection(
            inputs,
            draft_schema_path=draft_schema_path,
        )
    )
    if input_fingerprint != current_fingerprint:
        raise DraftStageValidationError("Draft input fingerprint is stale.")

    try:
        validator = compiled_schema_validator(
            _draft_schema_text(document_kind, draft_schema_path)
        )
    except SchemaCompilationError as exc:
        raise DraftStageValidationError(
            "The configured structured Draft schema is invalid."
        ) from exc
    if list(validator.iter_errors(candidate)):
        raise DraftStageValidationError("Draft candidate failed schema validation.")
    try:
        model_type = (
            CoverLetterDraftV1
            if document_kind == "cover_letter"
            else ResearchStatementDraftV1
        )
        validated = model_type.model_validate(candidate)
    except ValidationError as exc:
        raise DraftStageValidationError("Draft candidate failed semantic validation.") from exc

    _validate_candidate_identity(
        validated,
        inputs=inputs,
        job_dir=job_dir,
        input_fingerprint=input_fingerprint,
        expected_generation_mode=expected_generation_mode,
    )
    _validate_claim_references(validated, inputs=inputs)

    try:
        final_inputs = _load_draft_inputs(
            workspace,
            job_dir,
            document_kind=document_kind,
            document_id=document_id,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
            validate_plan=False,
        )
    except DraftStageError as exc:
        raise DraftStageValidationError(
            "Draft inputs changed during candidate validation."
        ) from exc
    final_fingerprint = _projection_sha256(
        _draft_projection(
            final_inputs,
            draft_schema_path=draft_schema_path,
        )
    )
    if final_fingerprint != input_fingerprint:
        raise DraftStageValidationError("Draft inputs changed during candidate validation.")
    return validated


def _load_draft_inputs(
    workspace: Path,
    job_dir: Path,
    *,
    document_kind: DraftDocumentKind,
    document_id: str | None,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
    validate_plan: bool,
) -> _DraftInputs:
    try:
        parsed_job = read_json_object(
            resolve_job_relative_path(job_dir, PARSED_JOB_INPUT_PATH)
        )
        parsed_validator = compiled_schema_validator(
            _parsed_job_schema_text(parsed_job_schema_path)
        )
        if list(parsed_validator.iter_errors(parsed_job)):
            raise DraftStageError("Draft Parsed Job input is invalid.")

        criteria = CriteriaCatalogV1.model_validate(
            read_json_object(resolve_job_relative_path(job_dir, CRITERIA_INPUT_PATH))
        )
        evidence = EvidenceCatalogV1.model_validate(
            read_json_object(
                resolve_job_relative_path(job_dir, EVIDENCE_CATALOG_INPUT_PATH)
            )
        )
        matches = CriterionMatchesV1.model_validate(
            read_json_object(
                resolve_job_relative_path(job_dir, CRITERION_MATCHES_INPUT_PATH)
            )
        )
        plan = RequiredDocumentPlanV1.model_validate(
            read_json_object(
                resolve_job_relative_path(job_dir, REQUIRED_DOCUMENT_PLAN_OUTPUT_PATH)
            )
        )
        decision, decision_sha256 = _load_user_model(
            job_dir,
            APPLICATION_DECISION_INPUT_PATH,
            ApplicationDecisionV1,
        )
        brief, brief_sha256 = _load_user_model(
            job_dir,
            APPLICATION_BRIEF_INPUT_PATH,
            ApplicationBriefV1,
        )
        basis = DraftBasisV1(
            parsed_job_sha256=_core_hash(job_dir, PARSED_JOB_INPUT_PATH),
            criteria_sha256=_core_hash(job_dir, CRITERIA_INPUT_PATH),
            evidence_catalog_sha256=_core_hash(job_dir, EVIDENCE_CATALOG_INPUT_PATH),
            criterion_matches_sha256=_core_hash(job_dir, CRITERION_MATCHES_INPUT_PATH),
            application_decision_sha256=decision_sha256,
            application_brief_sha256=brief_sha256,
            required_document_plan_sha256=_core_hash(
                job_dir,
                REQUIRED_DOCUMENT_PLAN_OUTPUT_PATH,
            ),
        )
    except DraftStageError:
        raise
    except (
        InvalidUserFileError,
        UnsafeUserFileError,
        StageStoreError,
        ValidationError,
        json.JSONDecodeError,
        OSError,
        UnicodeError,
        ValueError,
    ) as exc:
        raise DraftStageError("Draft requires valid current structured inputs.") from exc

    try:
        _validate_upstream_links(
            workspace,
            job_dir,
            criteria=criteria,
            evidence=evidence,
            matches=matches,
            decision=decision,
            brief=brief,
            plan=plan,
            basis=basis,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
            validate_plan=validate_plan,
        )
    except DraftStageError:
        raise
    except (OSError, StageStoreError, UnicodeError, ValueError) as exc:
        raise DraftStageError("Draft upstream receipts could not be validated.") from exc
    planned_document_id = _optional_document_id(plan, document_kind=document_kind)
    if document_id is not None and document_id != planned_document_id:
        raise DraftStageError("Draft target is not the planned document kind.")
    return _DraftInputs(
        parsed_job=parsed_job,
        criteria=criteria,
        evidence=evidence,
        matches=matches,
        decision=decision,
        brief=brief,
        plan=plan,
        basis=basis,
        document_kind=document_kind,
        document_id=planned_document_id,
    )


def _validate_upstream_links(
    workspace: Path,
    job_dir: Path,
    *,
    criteria: CriteriaCatalogV1,
    evidence: EvidenceCatalogV1,
    matches: CriterionMatchesV1,
    decision: ApplicationDecisionV1,
    brief: ApplicationBriefV1,
    plan: RequiredDocumentPlanV1,
    basis: DraftBasisV1,
    parsed_job_schema_path: Path | None,
    required_document_plan_schema_path: Path | None,
    validate_plan: bool,
) -> None:
    job_id = job_dir.name
    if any(
        item.job_id != job_id
        for item in (criteria, evidence, matches, decision, brief, plan)
    ):
        raise DraftStageError("A Draft input belongs to a different job.")
    if (
        matches.criteria_catalog_sha256 != basis.criteria_sha256
        or matches.evidence_catalog_sha256 != basis.evidence_catalog_sha256
    ):
        raise DraftStageError("Criterion Matches does not bind the current catalogs.")
    criterion_ids = {item.criterion_id for item in criteria.criteria}
    if {item.criterion_id for item in matches.matches} != criterion_ids:
        raise DraftStageError("Criterion Matches does not cover the current Criteria exactly.")
    evidence_ids = {item.evidence_id for item in evidence.items}
    if not {item.evidence_id for item in matches.evidence_refs}.issubset(evidence_ids):
        raise DraftStageError("Criterion Matches references unavailable current Evidence.")

    if (
        decision.decision != "apply"
        or decision.confirmation_state != "confirmed"
        or decision.basis is None
        or decision.basis.status != "current"
        or decision.basis.criteria_sha256 != basis.criteria_sha256
        or decision.basis.matches_sha256 != basis.criterion_matches_sha256
    ):
        raise DraftStageError("Draft requires a current confirmed apply Decision.")
    if brief.decision_sha256 != basis.application_decision_sha256:
        raise DraftStageError("Application Brief does not bind the current Decision.")

    current_brief_fingerprint = brief_input_fingerprint(
        workspace,
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    if plan.input_fingerprint != current_brief_fingerprint:
        raise DraftStageError("Required Document Plan is stale.")
    if validate_plan:
        try:
            validate_brief_candidate(
                plan.model_dump(mode="json"),
                workspace=workspace,
                job_dir=job_dir,
                input_fingerprint=current_brief_fingerprint,
                parsed_job_schema_path=parsed_job_schema_path,
                required_document_plan_schema_path=required_document_plan_schema_path,
            )
        except ValueError as exc:
            raise DraftStageError("Required Document Plan is not canonical and current.") from exc


def _draft_input_reason(
    inputs: _DraftInputs,
    *,
    document_id: str | None,
) -> tuple[str, ...]:
    plan = inputs.plan
    if plan.requirements_state != "confirmed":
        return ("input_not_ready:document_requirements",)
    if plan.blockers or plan.unresolved_brief_fields or plan.unresolved_document_ids:
        return ("input_not_ready:document_plan_blocked",)
    try:
        planned_document_id = _document_id(
            plan,
            document_kind=inputs.document_kind,
        )
    except DraftStageError:
        return (f"input_not_ready:{inputs.document_kind}_not_planned",)
    if document_id is not None and document_id != planned_document_id:
        return ("input_not_ready:document_not_planned",)
    task = next(
        (item for item in plan.tasks if item.document_id == planned_document_id),
        None,
    )
    if (
        task is None
        or task.action != "prepare"
        or task.confirmation_state != "confirmed"
        or task.blockers
    ):
        return (f"input_not_ready:{inputs.document_kind}_not_prepared",)
    return ()


def _cover_letter_document_id(plan: RequiredDocumentPlanV1) -> str:
    return _document_id(plan, document_kind="cover_letter")


def _document_id(
    plan: RequiredDocumentPlanV1,
    *,
    document_kind: DraftDocumentKind,
) -> str:
    document_id = _optional_document_id(plan, document_kind=document_kind)
    if document_id is None:
        raise DraftStageError(
            f"Draft requires exactly one planned {document_kind} document."
        )
    return document_id


def _optional_cover_letter_document_id(plan: RequiredDocumentPlanV1) -> str | None:
    return _optional_document_id(plan, document_kind="cover_letter")


def _optional_document_id(
    plan: RequiredDocumentPlanV1,
    *,
    document_kind: DraftDocumentKind,
) -> str | None:
    matching = tuple(
        requirement.document_id
        for requirement in plan.requirements
        if requirement.normalized_kind == document_kind
    )
    return matching[0] if len(matching) == 1 else None


def _validate_candidate_identity(
    candidate: StructuredDraftV1,
    *,
    inputs: _DraftInputs,
    job_dir: Path,
    input_fingerprint: str,
    expected_generation_mode: DraftGenerationMode,
) -> None:
    if candidate.job_id != job_dir.name:
        raise DraftStageValidationError("Draft candidate belongs to a different job.")
    if inputs.document_id is None or candidate.document_id != inputs.document_id:
        raise DraftStageValidationError("Draft candidate targets a different document.")
    if candidate.input_fingerprint != input_fingerprint:
        raise DraftStageValidationError("Draft candidate declares a stale input fingerprint.")
    if candidate.basis != inputs.basis:
        raise DraftStageValidationError("Draft candidate basis does not match current inputs.")
    expected_strategy = (
        HOST_AGENT_RESEARCH_STATEMENT_GENERATOR_STRATEGY
        if inputs.document_kind == "research_statement"
        else {
            "host_agent": HOST_AGENT_DRAFT_GENERATOR_STRATEGY,
            "configured_provider": CONFIGURED_PROVIDER_DRAFT_GENERATOR_STRATEGY,
        }[expected_generation_mode]
    )
    if (
        candidate.generation_mode != expected_generation_mode
        or candidate.generator_strategy != expected_strategy
        or candidate.generator_version != DRAFT_GENERATOR_VERSION
    ):
        raise DraftStageValidationError("Draft candidate declares an unsupported generator.")


def _validate_claim_references(
    candidate: StructuredDraftV1,
    *,
    inputs: _DraftInputs,
) -> None:
    current_criteria = {item.criterion_id for item in inputs.criteria.criteria}
    current_evidence = {item.evidence_id for item in inputs.evidence.items}
    unknown_job_fields = {
        value for value in inputs.parsed_job.get("unknown_fields", []) if isinstance(value, str)
    }
    for section in candidate.sections:
        for claim in section.claims:
            if not set(claim.criterion_ids).issubset(current_criteria):
                raise DraftStageValidationError(
                    "A Draft claim references a non-current Criterion."
                )
            if not set(claim.evidence_ref_ids).issubset(current_evidence):
                raise DraftStageValidationError(
                    "A Draft claim references non-current Evidence."
                )
            if "motivation" in claim.brief_field_refs and (
                inputs.brief.motivation.confirmation_state != "confirmed"
                or inputs.brief.motivation.value is None
            ):
                raise DraftStageValidationError(
                    "A Draft motivation claim lacks a confirmed Brief basis."
                )
            if "emphasis" in claim.brief_field_refs and (
                inputs.brief.emphasis.confirmation_state != "confirmed"
                or not (
                    inputs.brief.emphasis.criterion_ids
                    or inputs.brief.emphasis.evidence_ref_ids
                )
            ):
                raise DraftStageValidationError(
                    "A Draft future-intent claim lacks a selected Brief emphasis basis."
                )
            for field in claim.job_field_refs:
                value = inputs.parsed_job.get(field)
                if field in unknown_job_fields or not isinstance(value, str) or not value.strip():
                    raise DraftStageValidationError(
                        "A Draft role-context claim references an unknown job field."
                    )


def _load_user_model(
    job_dir: Path,
    relative_path: str,
    model_type: type[ApplicationDecisionV1] | type[ApplicationBriefV1],
) -> tuple[ApplicationDecisionV1 | ApplicationBriefV1, str]:
    snapshot = read_optional_safe_bytes(job_dir, relative_path)
    if snapshot is None:
        raise DraftStageError("A required user-owned Draft input is missing.")
    model = model_type.model_validate(load_strict_yaml(snapshot.data))
    return model, snapshot.sha256


def _core_hash(job_dir: Path, relative_path: str) -> str:
    return sha256_file(resolve_job_relative_path(job_dir, relative_path))


def _projection_sha256(value: object) -> str:
    encoded = json.dumps(
        value,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return sha256(encoded).hexdigest()


def _cover_letter_schema_text(path: Path | None) -> str:
    return _draft_schema_text("cover_letter", path)


def _draft_schema_text(
    document_kind: DraftDocumentKind,
    path: Path | None,
) -> str:
    filename = {
        "cover_letter": "cover-letter-draft.schema.json",
        "research_statement": "research-statement-draft.schema.json",
    }[document_kind]
    try:
        return read_resource_text(
            f"schemas/{filename}",
            local_path=path,
        )
    except (OSError, UnicodeError) as exc:
        raise DraftStageError("The structured Draft schema is not readable.") from exc


def _parsed_job_schema_text(path: Path | None) -> str:
    try:
        return read_resource_text("schemas/parsed_job.schema.json", local_path=path)
    except (OSError, UnicodeError) as exc:
        raise DraftStageError("The Parsed Job schema is not readable.") from exc
