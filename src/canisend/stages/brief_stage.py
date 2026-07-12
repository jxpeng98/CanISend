from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass
from hashlib import sha256
import json
from pathlib import Path
import re
from typing import Any, Iterable
import unicodedata

from jsonschema import Draft202012Validator
from pydantic import ValidationError

from canisend.decision_models import (
    ApplicationBriefV1,
    ApplicationDecisionV1,
    CriteriaCatalogV1,
    CriterionMatchesV1,
    DocumentRequirementV1,
    DocumentTaskV1,
    RequiredDocumentPlanV1,
    SourceSpanV1,
)
from canisend.resource_files import read_resource_text
from canisend.stage_store import (
    StageStoreError,
    read_json_object,
    resolve_job_relative_path,
    sha256_file,
)
from canisend.stages.parse_stage import ParseStageValidationError, validate_parse_candidate
from canisend.user_file_store import (
    InvalidUserFileError,
    UnsafeUserFileError,
    load_strict_yaml,
    read_optional_safe_bytes,
)


BRIEF_CONTRACT_VERSION = "1.0.0"
DOCUMENT_NORMALIZATION_VERSION = "1.0.0"

APPLICATION_BRIEF_INPUT_PATH = "application_brief.yaml"
APPLICATION_DECISION_INPUT_PATH = "application_decision.yaml"
CRITERIA_INPUT_PATH = "criteria.json"
CRITERION_MATCHES_INPUT_PATH = "criterion_matches.json"
JOB_ADVERT_INPUT_PATH = "job_advert.md"
PARSED_JOB_INPUT_PATH = "parsed_job.json"
REQUIRED_DOCUMENT_PLAN_OUTPUT_PATH = "required_document_plan.json"

_REQUIREMENT_PREFIX = re.compile(
    r"^(?P<marker>required|mandatory|optional|if\s+applicable)\s*[:\-]\s*",
    flags=re.IGNORECASE,
)
_REQUIREMENT_PAREN_SUFFIX = re.compile(
    r"\s*\((?P<marker>required|mandatory|optional|if\s+applicable)\)\s*$",
    flags=re.IGNORECASE,
)
_REQUIREMENT_DELIMITED_SUFFIX = re.compile(
    r"\s*[:\-]\s*(?P<marker>required|mandatory|optional|if\s+applicable)\s*$",
    flags=re.IGNORECASE,
)
_DOCUMENT_NEGATED_OR_CONDITIONAL_CONTEXT = re.compile(
    r"\b(?:do|does|did|should|must|need|needs|is|are|was|were|will|would|can|could)\s+not\b"
    r"|\b(?:do|does|did|need|is|are|was|were|will|would|can|could)n['’]?t\b"
    r"|\b(?:can|won)['’]?t\b"
    r"|\b(?:cannot|can|could|would)\b"
    r"|\bnot\s+(?:required|mandatory|needed)\b"
    r"|\b(?:no|never)\b"
    r"|\bno\s+(?:need|requirement)\b"
    r"|\bmay\b"
    r"|\bif\b"
    r"|\b(?:where|when)\s+applicable\b"
    r"|\bas\s+(?:required|appropriate|applicable|needed)\b"
    r"|\band\s*/\s*or\b"
    r"|\bor\b"
    r"|\b(?:unless|except|exempt|waiv(?:e|ed|er))\b"
    r"|\bupon\s+request\b"
    r"|\b(?:encouraged|recommended|permitted|allowed)\b"
    r"|\b(?:only\s+)?(?:shortlisted|selected|invited)\b",
    flags=re.IGNORECASE,
)
_TRUSTED_DOCUMENT_SECTION_HEADING = re.compile(
    r"^(?:(?:required|mandatory|optional|application|supporting)\s+"
    r"(?:documents?|materials?)|how\s+to\s+apply|what\s+to\s+submit)$",
    flags=re.IGNORECASE,
)
_LIST_ITEM = re.compile(r"^\s*(?:[-*+]\s+|\d+[.)]\s+)")

_KIND_ALIASES = {
    "application form": "application_form",
    "cover letter": "cover_letter",
    "covering letter": "cover_letter",
    "curriculum vitae": "cv",
    "cv": "cv",
    "diversity statement": "diversity_statement",
    "edi statement": "diversity_statement",
    "equality diversity and inclusion statement": "diversity_statement",
    "personal statement": "personal_statement",
    "publication list": "publication_list",
    "publications list": "publication_list",
    "reference details": "references",
    "references": "references",
    "referee details": "references",
    "research proposal": "research_proposal",
    "research statement": "research_statement",
    "resume": "cv",
    "supporting statement": "supporting_statement",
    "teaching philosophy": "teaching_philosophy",
    "teaching statement": "teaching_statement",
    "writing sample": "writing_sample",
}


class BriefStageError(ValueError):
    """Raised when Brief inputs cannot produce a safe deterministic plan."""


class BriefStageValidationError(BriefStageError):
    """Raised when a Required Document Plan candidate cannot be accepted."""


@dataclass(frozen=True)
class _DocumentSourceOccurrence:
    line_number: int
    source_text: str
    text_sha256: str
    anchor_sha256: str
    context_requirement: str | None


@dataclass(frozen=True)
class _NormalizedDocumentRequirement:
    identity_key: str
    label: str
    normalized_kind: str
    requirement: str
    alternatives: tuple[str, ...]
    occurrences: tuple[_DocumentSourceOccurrence, ...]

    @property
    def source_state(self) -> str:
        return "known" if len(self.occurrences) == 1 else "unknown"

    @property
    def unknown_reason(self) -> str | None:
        if len(self.occurrences) == 1:
            return None
        if self.occurrences:
            return "documents.source_ambiguous"
        return "documents.source_not_found"


def canonical_document_kind(label: str) -> str:
    """Return a stable lowercase kind without relying on list position."""

    cleaned, _requirement = _clean_document_label(label)
    semantic = _normalized_text(cleaned)
    aliased = _KIND_ALIASES.get(_kind_alias_key(cleaned))
    if aliased is not None:
        return aliased
    ascii_text = unicodedata.normalize("NFKD", semantic).encode("ascii", "ignore").decode()
    slug = re.sub(r"[^a-z0-9]+", "_", ascii_text).strip("_")
    if not slug:
        slug = "other_document"
    if slug[0].isdigit():
        slug = f"document_{slug}"
    return slug


def _document_identity_key(label: str, *, normalized_kind: str) -> str:
    semantic = _normalized_text(label)
    # Only explicit strict aliases collapse. Unknown labels retain their full
    # normalized semantics so two labels with the same display slug cannot
    # silently lose a required task.
    return (
        normalized_kind
        if _kind_alias_key(label) in _KIND_ALIASES
        else f"label:{semantic}"
    )


def _kind_alias_key(label: str) -> str:
    words = _normalized_search_text(label)
    if words in _KIND_ALIASES:
        return words
    compact = words.replace(" ", "")
    if compact in _KIND_ALIASES and all(len(part) == 1 for part in words.split()):
        return compact
    return _normalized_text(label)


def stable_document_id(
    *,
    job_id: str,
    normalized_kind: str,
    identity_key: str | None = None,
) -> str:
    canonical = json.dumps(
        {
            "job_id": job_id,
            "document_identity": identity_key or normalized_kind,
        },
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return f"document_{sha256(canonical).hexdigest()[:32]}"


def document_requirements_basis_sha256(
    parsed_job: dict[str, Any],
    advert_text: str,
) -> str:
    raw_documents = parsed_job.get("required_documents")
    if not isinstance(raw_documents, list):
        raise BriefStageError("Parsed required documents must be a list.")
    normalized_requirements = _normalized_document_requirements(
        raw_documents,
        advert_text,
    )
    projection = {
        "basis_version": DOCUMENT_NORMALIZATION_VERSION,
        "required_documents": tuple(
            {
                "identity_key": item.identity_key,
                "requirement": item.requirement,
                "source_receipt": {
                    "source_state": item.source_state,
                    "unknown_reason": item.unknown_reason,
                    "occurrences": tuple(
                        {
                            "path": JOB_ADVERT_INPUT_PATH,
                            "start_line": occurrence.line_number,
                            "end_line": occurrence.line_number,
                            "text_sha256": occurrence.text_sha256,
                            "anchor_sha256": occurrence.anchor_sha256,
                            "occurrence": index,
                            "occurrence_count": len(item.occurrences),
                        }
                        for index, occurrence in enumerate(item.occurrences, start=1)
                    ),
                },
            }
            for item in normalized_requirements
        ),
    }
    # With no extracted requirement there is no narrower source receipt to bind
    # an explicit confirmed_empty decision to. Bind that decision to the exact
    # current advert so an empty parser result cannot survive an advert change.
    if not normalized_requirements:
        projection["empty_advert_sha256"] = sha256(
            advert_text.encode("utf-8")
        ).hexdigest()
    return _projection_sha256(projection)


def brief_precondition_reasons(workspace: Path, job_dir: Path) -> tuple[str, ...]:
    """Return body-free reasons that must block a Brief stage run."""

    # Local import avoids the user-mutation -> runtime -> adapter import cycle.
    from canisend.user_mutations import inspect_current_artifact_mutation

    for artifact in ("decision", "brief"):
        audit = inspect_current_artifact_mutation(
            workspace,
            job_dir,
            artifact,  # type: ignore[arg-type]
        )
        if audit.status not in {"untracked", "committed"}:
            return (f"input_not_ready:{artifact}_mutation",)
    try:
        decision, decision_sha256 = _load_user_model(
            job_dir,
            APPLICATION_DECISION_INPUT_PATH,
            ApplicationDecisionV1,
        )
    except BriefStageError:
        return ("input_not_ready:decision_unavailable",)
    if decision.decision != "apply" or decision.confirmation_state != "confirmed":
        return ("input_not_ready:decision_not_apply",)
    if decision.basis is None or decision.basis.status != "current":
        return ("input_not_ready:decision_review",)
    try:
        if (
            decision.basis.criteria_sha256
            != _safe_core_hash(job_dir, CRITERIA_INPUT_PATH)
            or decision.basis.matches_sha256
            != _safe_core_hash(job_dir, CRITERION_MATCHES_INPUT_PATH)
        ):
            return ("input_not_ready:decision_review",)
    except (BriefStageError, StageStoreError):
        return ("input_not_ready:decision_review",)

    try:
        brief, _brief_sha256 = _load_user_model(
            job_dir,
            APPLICATION_BRIEF_INPUT_PATH,
            ApplicationBriefV1,
        )
    except BriefStageError:
        return ("input_not_ready:brief_unavailable",)
    if brief.decision_sha256 != decision_sha256:
        return ("input_not_ready:brief_review",)
    if brief.emphasis.confirmation_state == "confirmed" and (
        brief.emphasis.criterion_ids or brief.emphasis.evidence_ref_ids
    ):
        try:
            criteria = CriteriaCatalogV1.model_validate(
                read_json_object(
                    resolve_job_relative_path(job_dir, CRITERIA_INPUT_PATH)
                )
            )
            matches = CriterionMatchesV1.model_validate(
                read_json_object(
                    resolve_job_relative_path(job_dir, CRITERION_MATCHES_INPUT_PATH)
                )
            )
        except (StageStoreError, ValidationError):
            return ("input_not_ready:brief_review",)
        current_criteria = {item.criterion_id for item in criteria.criteria}
        current_evidence = {item.evidence_id for item in matches.evidence_refs}
        if not set(brief.emphasis.criterion_ids).issubset(current_criteria) or not set(
            brief.emphasis.evidence_ref_ids
        ).issubset(current_evidence):
            return ("input_not_ready:brief_review",)
    return ()


def brief_input_projection(
    workspace: Path,
    job_dir: Path,
    *,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> dict[str, object]:
    del workspace  # All current Brief inputs are declared job-local artifacts.
    parsed_job, advert_text = _load_validated_parse_inputs(
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
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
    if decision.job_id != job_dir.name or brief.job_id != job_dir.name:
        raise BriefStageError("Brief inputs belong to a different job.")
    return {
        "stage": "brief",
        "contract_version": BRIEF_CONTRACT_VERSION,
        "normalization_version": DOCUMENT_NORMALIZATION_VERSION,
        "requirements_basis_sha256": document_requirements_basis_sha256(
            parsed_job,
            advert_text,
        ),
        "parsed_job_sha256": _safe_core_hash(job_dir, PARSED_JOB_INPUT_PATH),
        "advert_sha256": _safe_core_hash(job_dir, JOB_ADVERT_INPUT_PATH),
        "criteria_sha256": _safe_core_hash(job_dir, CRITERIA_INPUT_PATH),
        "matches_sha256": _safe_core_hash(job_dir, CRITERION_MATCHES_INPUT_PATH),
        "decision_sha256": decision_sha256,
        "brief_sha256": brief_sha256,
        "schema_sha256": sha256(
            _required_document_plan_schema_text(
                required_document_plan_schema_path
            ).encode("utf-8")
        ).hexdigest(),
    }


def brief_input_fingerprint(
    workspace: Path,
    job_dir: Path,
    *,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> str:
    return _projection_sha256(
        brief_input_projection(
            workspace,
            job_dir,
            parsed_job_schema_path=parsed_job_schema_path,
            required_document_plan_schema_path=required_document_plan_schema_path,
        )
    )


def build_deterministic_brief_candidate(
    workspace: Path,
    job_dir: Path,
    *,
    input_fingerprint: str,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> RequiredDocumentPlanV1:
    if brief_precondition_reasons(workspace, job_dir):
        raise BriefStageError("Brief inputs are not current and ready for planning.")
    projection = brief_input_projection(
        workspace,
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    current_fingerprint = _projection_sha256(projection)
    if current_fingerprint != input_fingerprint:
        raise BriefStageError("Brief input fingerprint is stale.")
    parsed_job, advert_text = _load_validated_parse_inputs(
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
    )
    brief, _brief_sha256 = _load_user_model(
        job_dir,
        APPLICATION_BRIEF_INPUT_PATH,
        ApplicationBriefV1,
    )
    plan = _build_plan(
        job_id=job_dir.name,
        parsed_job=parsed_job,
        advert_text=advert_text,
        brief=brief,
        requirements_basis_sha256=str(projection["requirements_basis_sha256"]),
        input_fingerprint=input_fingerprint,
    )
    if brief_precondition_reasons(workspace, job_dir):
        raise BriefStageError("Brief inputs changed while the plan was built.")
    final_projection = brief_input_projection(
        workspace,
        job_dir,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    if _projection_sha256(final_projection) != input_fingerprint:
        raise BriefStageError("Brief inputs changed while the plan was built.")
    return plan


def validate_brief_candidate(
    candidate: object,
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
    parsed_job_schema_path: Path | None = None,
    required_document_plan_schema_path: Path | None = None,
) -> RequiredDocumentPlanV1:
    if not isinstance(candidate, dict):
        raise BriefStageValidationError("Brief candidate must be a JSON object.")
    try:
        schema = json.loads(
            _required_document_plan_schema_text(required_document_plan_schema_path)
        )
        Draft202012Validator.check_schema(schema)
    except (json.JSONDecodeError, ValueError) as exc:
        raise BriefStageValidationError(
            "The configured Required Document Plan schema is invalid."
        ) from exc
    if list(Draft202012Validator(schema).iter_errors(candidate)):
        raise BriefStageValidationError("Brief candidate failed schema validation.")
    try:
        validated = RequiredDocumentPlanV1.model_validate(candidate)
    except ValidationError as exc:
        raise BriefStageValidationError("Brief candidate failed semantic validation.") from exc
    expected = build_deterministic_brief_candidate(
        workspace,
        job_dir,
        input_fingerprint=input_fingerprint,
        parsed_job_schema_path=parsed_job_schema_path,
        required_document_plan_schema_path=required_document_plan_schema_path,
    )
    if validated != expected:
        raise BriefStageValidationError(
            "Brief candidate does not match the current deterministic projection."
        )
    return validated


def _build_plan(
    *,
    job_id: str,
    parsed_job: dict[str, Any],
    advert_text: str,
    brief: ApplicationBriefV1,
    requirements_basis_sha256: str,
    input_fingerprint: str,
) -> RequiredDocumentPlanV1:
    raw_documents = parsed_job["required_documents"]
    normalized_requirements = _normalized_document_requirements(
        raw_documents,
        advert_text,
    )

    confirmation = brief.document_requirements_confirmation
    confirmation_current = (
        confirmation.basis_sha256 == requirements_basis_sha256
        and confirmation.state in {"confirmed", "confirmed_empty"}
    )
    if not normalized_requirements:
        requirements_state = (
            "confirmed_empty"
            if confirmation_current and confirmation.state == "confirmed_empty"
            else "unconfirmed"
        )
    else:
        requirements_state = (
            "confirmed"
            if (
                confirmation_current
                and confirmation.state == "confirmed"
                and all(
                    item.source_state == "known"
                    for item in normalized_requirements
                )
            )
            else "unconfirmed"
        )

    requirements: list[DocumentRequirementV1] = []
    for item in normalized_requirements:
        source_text: str | None = None
        source_span = None
        if item.source_state == "known":
            occurrence = item.occurrences[0]
            source_text = occurrence.source_text
            source_span = SourceSpanV1(
                path=JOB_ADVERT_INPUT_PATH,
                start_line=occurrence.line_number,
                end_line=occurrence.line_number,
                text_sha256=occurrence.text_sha256,
                anchor_sha256=occurrence.anchor_sha256,
                occurrence=1,
                occurrence_count=1,
            )
        requirements.append(
            DocumentRequirementV1(
                document_id=stable_document_id(
                    job_id=job_id,
                    normalized_kind=item.normalized_kind,
                    identity_key=item.identity_key,
                ),
                label=item.label,
                normalized_kind=item.normalized_kind,
                requirement=item.requirement,
                source_text=source_text,
                source_state=item.source_state,
                source_span=source_span,
                confirmation_state=(
                    "confirmed" if requirements_state == "confirmed" else "unconfirmed"
                ),
                unknown_reason=item.unknown_reason,
            )
        )

    requirements.sort(key=lambda item: item.document_id)
    choice_by_id = {choice.document_id: choice for choice in brief.document_choices}
    requirement_ids = {item.document_id for item in requirements}
    orphaned_choice_ids = tuple(
        sorted(document_id for document_id in choice_by_id if document_id not in requirement_ids)
    )
    tasks: list[DocumentTaskV1] = []
    for requirement in requirements:
        choice = choice_by_id.get(requirement.document_id)
        if requirements_state != "confirmed":
            tasks.append(
                DocumentTaskV1(
                    document_id=requirement.document_id,
                    action="needs_confirmation",
                    confirmation_state="unconfirmed",
                    blockers=("documents.requirements_unconfirmed",),
                )
            )
            continue
        if choice is None:
            tasks.append(
                DocumentTaskV1(
                    document_id=requirement.document_id,
                    action=(
                        "prepare"
                        if requirement.requirement == "required"
                        else "needs_confirmation"
                    ),
                    confirmation_state=(
                        "confirmed"
                        if requirement.requirement == "required"
                        else "unconfirmed"
                    ),
                    blockers=(
                        ()
                        if requirement.requirement == "required"
                        else ("documents.optional_choice_required",)
                    ),
                )
            )
            continue
        if choice.action == "needs_confirmation":
            blockers = ("documents.choice_unconfirmed",)
        elif choice.action == "omit" and requirement.requirement == "required":
            blockers = ("documents.required_omitted",)
        else:
            blockers = ()
        tasks.append(
            DocumentTaskV1(
                document_id=requirement.document_id,
                action=choice.action,
                confirmation_state=choice.confirmation_state,
                blockers=blockers,
            )
        )

    unresolved_brief_fields = tuple(
        field
        for field in ("language", "writing_style", "motivation", "emphasis", "exclusions")
        if getattr(brief, field).confirmation_state != "confirmed"
    )
    tasks.sort(key=lambda item: item.document_id)
    unresolved_document_ids = tuple(
        sorted(task.document_id for task in tasks if task.action == "needs_confirmation")
    )
    blocking_document_ids = tuple(
        sorted(task.document_id for task in tasks if task.blockers)
    )
    blockers: list[str] = [
        f"brief.{field}_unconfirmed" for field in unresolved_brief_fields
    ]
    if requirements_state == "unconfirmed":
        blockers.append("documents.requirements_unconfirmed")
        blockers.extend(
            item.unknown_reason
            for item in requirements
            if item.unknown_reason is not None
        )
        if (
            confirmation.basis_sha256 is not None
            and confirmation.basis_sha256 != requirements_basis_sha256
        ):
            blockers.append("documents.requirements_basis_changed")
    if orphaned_choice_ids:
        blockers.append("documents.choice_orphaned")
    for task in tasks:
        blockers.extend(task.blockers)

    return RequiredDocumentPlanV1(
        job_id=job_id,
        input_fingerprint=input_fingerprint,
        requirements_state=requirements_state,
        requirements_basis_sha256=requirements_basis_sha256,
        requirements=tuple(requirements),
        tasks=tuple(tasks),
        unresolved_document_ids=unresolved_document_ids,
        unresolved_brief_fields=unresolved_brief_fields,
        blocking_document_ids=blocking_document_ids,
        orphaned_document_choice_ids=orphaned_choice_ids,
        blockers=_ordered_unique(blockers),
    )


def _load_validated_parse_inputs(
    job_dir: Path,
    *,
    parsed_job_schema_path: Path | None,
) -> tuple[dict[str, Any], str]:
    try:
        parsed_path = resolve_job_relative_path(job_dir, PARSED_JOB_INPUT_PATH)
        advert_path = resolve_job_relative_path(job_dir, JOB_ADVERT_INPUT_PATH)
        parsed = read_json_object(parsed_path)
        advert_text = advert_path.read_text(encoding="utf-8")
        validated = validate_parse_candidate(
            parsed,
            advert_text=advert_text,
            schema_path=parsed_job_schema_path,
        )
    except (
        OSError,
        UnicodeError,
        StageStoreError,
        ParseStageValidationError,
    ) as exc:
        raise BriefStageError("Brief requires current valid Parse inputs.") from exc
    return validated, advert_text


def _load_user_model(
    job_dir: Path,
    relative_path: str,
    model_type: type[ApplicationDecisionV1] | type[ApplicationBriefV1],
) -> tuple[ApplicationDecisionV1 | ApplicationBriefV1, str]:
    try:
        snapshot = read_optional_safe_bytes(job_dir, relative_path)
    except UnsafeUserFileError as exc:
        raise BriefStageError("A user-owned Brief input is unsafe.") from exc
    if snapshot is None:
        raise BriefStageError("A required user-owned Brief input is missing.")
    try:
        model = model_type.model_validate(load_strict_yaml(snapshot.data))
    except (InvalidUserFileError, ValidationError) as exc:
        raise BriefStageError("A user-owned Brief input is invalid.") from exc
    if model.job_id != job_dir.name:
        raise BriefStageError("A user-owned Brief input belongs to a different job.")
    return model, snapshot.sha256


def _safe_core_hash(job_dir: Path, relative_path: str) -> str:
    try:
        return sha256_file(resolve_job_relative_path(job_dir, relative_path))
    except StageStoreError as exc:
        raise BriefStageError("A Brief input cannot be hashed safely.") from exc


def _clean_document_label(value: object) -> tuple[str, str]:
    if not isinstance(value, str):
        raise BriefStageError("Each parsed required document must be text.")
    raw = unicodedata.normalize("NFKC", value).strip()
    if not raw:
        raise BriefStageError("Parsed required document labels must not be empty.")
    markers: list[str] = []
    cleaned = raw
    prefix = _REQUIREMENT_PREFIX.match(cleaned)
    if prefix is not None:
        markers.append(_normalized_requirement_marker(prefix.group("marker")))
        cleaned = cleaned[prefix.end() :]
    for pattern in (_REQUIREMENT_PAREN_SUFFIX, _REQUIREMENT_DELIMITED_SUFFIX):
        suffix = pattern.search(cleaned)
        if suffix is not None:
            markers.append(_normalized_requirement_marker(suffix.group("marker")))
            cleaned = cleaned[: suffix.start()]
            break
    # Required/mandatory wins a contradictory marker. This is fail-closed:
    # optional prose inside the document label never downgrades a required item.
    requirement = (
        "required"
        if not markers or any(marker == "required" for marker in markers)
        else "optional"
    )
    cleaned = cleaned.strip(" \t:;-–—")
    if not cleaned:
        raise BriefStageError("Parsed required document labels must contain a name.")
    return cleaned, requirement


def _normalized_requirement_marker(value: str) -> str:
    normalized = re.sub(r"\s+", " ", value).strip().casefold()
    return "required" if normalized in {"required", "mandatory"} else "optional"


def _canonical_raw_document(value: object) -> dict[str, str]:
    label, requirement = _clean_document_label(value)
    normalized_kind = canonical_document_kind(label)
    return {
        "label": label,
        "normalized_kind": normalized_kind,
        "identity_key": _document_identity_key(
            label,
            normalized_kind=normalized_kind,
        ),
        "requirement": requirement,
    }


def _normalized_document_requirements(
    raw_documents: list[object],
    advert_text: str,
) -> tuple[_NormalizedDocumentRequirement, ...]:
    grouped: dict[str, list[dict[str, str]]] = defaultdict(list)
    for raw in raw_documents:
        canonical = _canonical_raw_document(raw)
        grouped[canonical["identity_key"]].append(canonical)

    all_source_labels: set[str] = set()
    for identity_key, grouped_items in grouped.items():
        all_source_labels.update(item["label"] for item in grouped_items)
        grouped_kind = grouped_items[0]["normalized_kind"]
        if identity_key == grouped_kind:
            all_source_labels.update(
                alias
                for alias, alias_kind in _KIND_ALIASES.items()
                if alias_kind == grouped_kind
            )
    all_document_needles = tuple(
        sorted(
            {
                needle
                for label in all_source_labels
                for needle in (_normalized_search_text(label),)
                if needle
            }
        )
    )

    normalized: list[_NormalizedDocumentRequirement] = []
    for identity_key in sorted(grouped):
        alternatives = sorted(
            grouped[identity_key],
            key=lambda item: (
                _normalized_text(item["label"]),
                item["requirement"],
                item["normalized_kind"],
                item["label"],
            ),
        )
        label = alternatives[0]["label"]
        kind = alternatives[0]["normalized_kind"]
        requirement = (
            "required"
            if any(item["requirement"] == "required" for item in alternatives)
            else "optional"
        )
        source_labels = tuple(item["label"] for item in alternatives)
        if identity_key == kind:
            source_labels = tuple(
                sorted(
                    {
                        *source_labels,
                        *(
                            alias
                            for alias, alias_kind in _KIND_ALIASES.items()
                            if alias_kind == kind
                        ),
                    },
                    key=lambda value: (_normalized_text(value), value),
                )
            )
        source_occurrences = _document_source_occurrences(
            advert_text,
            source_labels,
            known_document_needles=all_document_needles,
        )
        source_requirement_markers = {
            item[3] for item in source_occurrences if item[3] is not None
        }
        if len(source_occurrences) == 1 and len(source_requirement_markers) == 1:
            requirement = next(iter(source_requirement_markers))
        normalized.append(
            _NormalizedDocumentRequirement(
                identity_key=identity_key,
                label=label,
                normalized_kind=kind,
                requirement=requirement,
                alternatives=tuple(item["label"] for item in alternatives),
                occurrences=tuple(
                    _DocumentSourceOccurrence(
                        line_number=line_number,
                        source_text=source_text,
                        text_sha256=_normalized_sha256(source_text),
                        anchor_sha256=_normalized_sha256(anchor_text),
                        context_requirement=context_requirement,
                    )
                    for (
                        line_number,
                        source_text,
                        anchor_text,
                        context_requirement,
                    ) in source_occurrences
                ),
            )
        )
    return tuple(normalized)


def _document_source_occurrences(
    advert_text: str,
    labels: Iterable[str],
    *,
    known_document_needles: tuple[str, ...] | None = None,
) -> tuple[tuple[int, str, str, str | None], ...]:
    needles = tuple(_normalized_search_text(label) for label in labels)
    known_needles = known_document_needles or tuple(
        sorted(needle for needle in needles if needle)
    )
    found: list[tuple[int, str, str, str | None]] = []
    lines = advert_text.splitlines()
    section_anchor_by_line = _document_section_anchor_by_line(advert_text)
    for line_number, line in enumerate(lines, start=1):
        source_text = line.strip()
        if not source_text:
            continue
        section_anchor = section_anchor_by_line[line_number - 1]
        inline_list = _trusted_inline_document_list(source_text)
        if inline_list is not None:
            if _document_list_item_has_continuation(lines, line_number - 1):
                continue
            header, source_sentence, members = inline_list
            if not _document_members_are_complete(members, known_needles):
                continue
            matching_member = _matching_document_member(members, needles)
            if matching_member is None:
                continue
            anchor_text = source_sentence
            if _DOCUMENT_NEGATED_OR_CONDITIONAL_CONTEXT.search(anchor_text):
                continue
            found.append(
                (
                    line_number,
                    source_sentence,
                    anchor_text,
                    _explicit_requirement_marker(matching_member)
                    or _requirement_from_source_context(header),
                )
            )
            continue
        if section_anchor is not None:
            if _document_list_item_has_continuation(lines, line_number - 1) or (
                _has_dangling_document_list_delimiter(source_text)
            ):
                continue
            members = _structured_document_members(source_text, strip_list_marker=True)
            if not _document_members_are_complete(members, known_needles):
                continue
            matching_member = _matching_document_member(members, needles)
            if matching_member is None:
                continue
            anchor_text = f"{section_anchor}\n{source_text}"
            if _DOCUMENT_NEGATED_OR_CONDITIONAL_CONTEXT.search(anchor_text):
                continue
            found.append(
                (
                    line_number,
                    source_text,
                    anchor_text,
                    _explicit_requirement_marker(matching_member)
                    or _requirement_from_source_context(section_anchor),
                )
            )
            continue
        clauses = _document_source_clauses(source_text)
        if len(clauses) != 1:
            continue
        for clause in clauses:
            haystack = _normalized_search_text(clause)
            if not any(
                _contains_search_phrase(haystack, needle) for needle in needles
            ):
                continue
            if not _inline_document_requirement_context(
                clause,
                needles,
                known_document_needles=known_needles,
            ):
                continue
            anchor_text = clause
            if _DOCUMENT_NEGATED_OR_CONDITIONAL_CONTEXT.search(anchor_text):
                continue
            found.append(
                (
                    line_number,
                    clause,
                    anchor_text,
                    _requirement_from_source_context(anchor_text),
                )
            )
    return tuple(found)


def _document_section_anchor_by_line(advert_text: str) -> tuple[str | None, ...]:
    active_document_section: str | None = None
    anchors: list[str | None] = []
    for line in advert_text.splitlines():
        stripped = line.strip()
        heading_text = re.sub(r"^#{1,6}\s+", "", stripped).rstrip(":").strip()
        is_heading_or_label = bool(
            re.match(r"^#{1,6}\s+", stripped)
            or (stripped.endswith(":") and len(stripped) <= 120)
        )
        contextual_item = bool(
            active_document_section is not None
            and (
                _LIST_ITEM.match(line)
                or _REQUIREMENT_PREFIX.match(stripped)
            )
        )
        anchors.append(active_document_section if contextual_item else None)
        if is_heading_or_label:
            active_document_section = (
                heading_text
                if _TRUSTED_DOCUMENT_SECTION_HEADING.fullmatch(heading_text)
                else None
            )
    return tuple(anchors)


def _document_source_clauses(source_text: str) -> tuple[str, ...]:
    return tuple(
        clause.strip()
        for clause in re.split(
            r"(?<=[.!?;])\s*"
            r"|\s+[–—]\s+"
            r"|,\s*(?=(?:(?:and|but|while)\s+)?"
            r"(?:your|our|their|the|a|an|applicants?|candidates?|we|you|it|they)\s+"
            r"[a-z][a-z'-]*\s+(?:will|would|is|are|was|were|can|could|should|must)\b)",
            source_text,
            flags=re.IGNORECASE,
        )
        if clause.strip()
    )


def _document_list_item_has_continuation(
    lines: list[str],
    index: int,
) -> bool:
    current_indent = len(lines[index]) - len(lines[index].lstrip())
    skipped_blank = False
    for next_line in lines[index + 1 :]:
        stripped = next_line.strip()
        if not stripped:
            skipped_blank = True
            continue
        next_indent = len(next_line) - len(next_line.lstrip())
        if _LIST_ITEM.match(next_line) and next_indent <= current_indent:
            return False
        heading_or_label = bool(
            re.match(r"^#{1,6}\s+", stripped)
            or (stripped.endswith(":") and len(stripped) <= 120)
        )
        if heading_or_label:
            return False
        if next_indent > current_indent:
            return True
        return not skipped_blank
    return False


def _inline_document_requirement_context(
    clause: str,
    needles: tuple[str, ...],
    *,
    known_document_needles: tuple[str, ...],
) -> bool:
    if _REQUIREMENT_PREFIX.match(clause):
        return True
    direct_members = _trusted_direct_document_members(clause)
    if (
        direct_members
        and _document_members_are_complete(direct_members, known_document_needles)
        and _matching_document_member(direct_members, needles) is not None
    ):
        return True
    normalized = _normalized_search_text(clause)
    action = r"(?:submit|upload|attach|provide|send|supply|enclose|include)"
    for needle in needles:
        if not needle:
            continue
        document = rf"(?:(?:a|an|the|your)\s+)?{re.escape(needle)}"
        templates = (
            rf"^{document}\s+(?:(?:is|are)\s+)?(?:required|mandatory|optional)$",
            rf"^{document}\s+submission\s+(?:is|are)\s+"
            rf"(?:required|mandatory|optional)$",
            rf"^{document}\s+(?:must|should)\s+be\s+"
            rf"(?:submitted|uploaded|attached|provided|sent|supplied|enclosed|included)$",
            rf"^(?:please\s+)?{action}\s+{document}$",
            rf"^to\s+apply\s+(?:please\s+)?{action}\s+{document}$",
            rf"^(?:candidates?|applicants?|you)\s+"
            rf"(?:must|should|need(?:s)?(?:\s+to)?|(?:are|is)\s+required\s+to)\s+"
            rf"(?:{action}\s+)?{document}$",
            rf"^(?:your\s+)?applications?\s+"
            rf"(?:must|should|need(?:s)?(?:\s+to)?|(?:is|are)\s+required\s+to|requires?)\s+"
            rf"(?:(?:include|contain|submit|provide)\s+)?{document}$",
        )
        if any(re.fullmatch(template, normalized) for template in templates):
            return True
    return False


def _trusted_direct_document_members(clause: str) -> tuple[str, ...]:
    patterns = (
        r"^\s*(?:please\s+)?(?:submit|upload|attach|provide|send|supply|enclose|include)\s+"
        r"(?P<body>.+)$",
        r"^\s*(?:candidates?|applicants?|you)\s+"
        r"(?:must|should|need(?:s)?(?:\s+to)?|(?:are|is)\s+required\s+to)\s+"
        r"(?:submit|upload|attach|provide|send|supply|enclose|include)\s+(?P<body>.+)$",
        r"^\s*(?:(?:the|your)\s+)?applications?\s+"
        r"(?:must|should|need(?:s)?(?:\s+to)?|(?:is|are)\s+required\s+to)\s+"
        r"(?:include|contain|submit|provide|attach)\s+(?P<body>.+)$",
        r"^\s*(?:(?:the|your)\s+)?applications?\s+"
        r"(?:must|should)\s+be\s+accompanied\s+by\s+(?P<body>.+)$",
    )
    for pattern in patterns:
        match = re.fullmatch(pattern, clause, flags=re.IGNORECASE)
        if match is not None:
            if _has_dangling_document_list_delimiter(match.group("body")):
                return ()
            return _structured_document_members(
                match.group("body"),
                strip_list_marker=False,
            )
    return ()


def _trusted_inline_document_list(
    source_text: str,
) -> tuple[str, str, tuple[str, ...]] | None:
    match = re.match(
        r"^\s*(?P<header>(?:required|mandatory|optional|application|supporting)\s+"
        r"(?:documents?|materials?))\s*"
        r"(?:[:\-–—]|\b(?:include|contain|are)\b)\s*:?[ \t]*"
        r"(?P<body>.+)$",
        source_text,
        flags=re.IGNORECASE,
    )
    if match is None:
        return None
    body = match.group("body").strip()
    if (
        not body
        or re.search(r"[!?]\s*\S|\.\s+(?=\S)", body)
        or _has_dangling_document_list_delimiter(body)
    ):
        return None
    source_sentence = source_text[: match.start("body")] + body
    return (
        match.group("header").strip(),
        source_sentence.strip(),
        _structured_document_members(body, strip_list_marker=False),
    )


def _structured_document_members(
    value: str,
    *,
    strip_list_marker: bool,
) -> tuple[str, ...]:
    cleaned = value.strip()
    if strip_list_marker:
        cleaned = re.sub(r"^\s*(?:[-*+]\s+|\d+[.)]\s+)", "", cleaned)
    return tuple(
        item.strip()
        for item in re.split(r"\s*(?:,|;|\band\b)\s*", cleaned, flags=re.IGNORECASE)
        if item.strip()
    )


def _has_dangling_document_list_delimiter(value: str) -> bool:
    normalized = value.strip().rstrip(".)]").strip()
    return bool(
        re.search(
            r"(?:[,;]|\band\s*/\s*or\b|\band\b|\bor\b)\s*$",
            normalized,
            flags=re.IGNORECASE,
        )
    )


def _matching_document_member(
    members: tuple[str, ...],
    needles: tuple[str, ...],
) -> str | None:
    for member in members:
        try:
            label, _requirement = _clean_document_label(member)
        except BriefStageError:
            continue
        normalized = _normalized_search_text(label)
        normalized = re.sub(r"^(?:a|an|the|your)\s+", "", normalized)
        if normalized in needles:
            return member
    return None


def _document_members_are_complete(
    members: tuple[str, ...],
    known_document_needles: tuple[str, ...],
) -> bool:
    return bool(members) and all(
        _matching_document_member((member,), known_document_needles) is not None
        for member in members
    )


def _explicit_requirement_marker(value: str) -> str | None:
    normalized = unicodedata.normalize("NFKC", value).strip()
    markers: list[str] = []
    prefix = _REQUIREMENT_PREFIX.match(normalized)
    if prefix is not None:
        markers.append(_normalized_requirement_marker(prefix.group("marker")))
        normalized = normalized[prefix.end() :]
    for pattern in (_REQUIREMENT_PAREN_SUFFIX, _REQUIREMENT_DELIMITED_SUFFIX):
        suffix = pattern.search(normalized)
        if suffix is not None:
            markers.append(_normalized_requirement_marker(suffix.group("marker")))
            break
    if not markers:
        return None
    return "required" if "required" in markers else "optional"


def _requirement_from_source_context(value: str) -> str | None:
    normalized = _normalized_text(value)
    has_optional = bool(re.search(r"\boptional\b", normalized))
    has_required = bool(
        re.search(
            r"\b(?:required|mandatory|must|requires?|needs?|submit|upload|attach|provide|send|supply|enclose)\b",
            normalized,
        )
    )
    if has_optional == has_required:
        return None
    return "optional" if has_optional else "required"


def _normalized_text(value: str) -> str:
    normalized = unicodedata.normalize("NFKC", value)
    return re.sub(r"\s+", " ", normalized).strip().casefold()


def _normalized_search_text(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", " ", _normalized_text(value)).strip()


def _contains_search_phrase(haystack: str, needle: str) -> bool:
    if not needle:
        return False
    return re.search(
        rf"(?:^| ){' +'.join(re.escape(part) for part in needle.split())}(?:$| )",
        haystack,
    ) is not None


def _normalized_sha256(value: str) -> str:
    return sha256(_normalized_text(value).encode("utf-8")).hexdigest()


def _projection_sha256(value: object) -> str:
    canonical = json.dumps(
        value,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return sha256(canonical).hexdigest()


def _ordered_unique(values: Iterable[str]) -> tuple[str, ...]:
    return tuple(dict.fromkeys(values))


def _required_document_plan_schema_text(schema_path: Path | None) -> str:
    try:
        return read_resource_text(
            "schemas/required-document-plan.schema.json",
            local_path=schema_path,
        )
    except (OSError, UnicodeError) as exc:
        raise BriefStageError("The Required Document Plan schema is not readable.") from exc
