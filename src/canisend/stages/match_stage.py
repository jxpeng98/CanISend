from __future__ import annotations

from hashlib import sha256
import json
from pathlib import Path

from pydantic import ValidationError

from canisend.decision_models import (
    CriteriaCatalogV1,
    CriterionMatchV1,
    CriterionMatchesV1,
    CriterionV1,
    EvidenceCatalogItemV1,
    EvidenceCatalogV1,
    EvidenceGapV1,
    EvidenceRefV1,
)
from canisend.evidence import EvidenceReference
from canisend.match import EvidenceIndex, _direct_overlap_score
from canisend.resource_files import read_resource_text
from canisend.schema_validation import (
    SchemaCompilationError,
    compiled_schema_validator,
)
from canisend.stage_store import StageStoreError, read_json_object, sha256_file


MATCH_CONTRACT_VERSION = "1.0.0"
MATCHER_STRATEGY = "deterministic.keyword"
MATCHER_VERSION = "1.0.0"
CRITERIA_INPUT_PATH = "criteria.json"
EVIDENCE_CATALOG_INPUT_PATH = "evidence_catalog.json"
CRITERION_MATCHES_OUTPUT_PATH = "criterion_matches.json"

_CATALOG_UNAVAILABLE_GAP = EvidenceGapV1(
    code="evidence.catalog_unavailable",
    message="The normalized evidence catalog is not available for matching.",
    next_action="Refresh profile evidence and rerun the Evidence stage.",
)
_CATALOG_EMPTY_GAP = EvidenceGapV1(
    code="evidence.catalog_empty",
    message="The current evidence catalog is valid but contains no evidence items.",
    next_action="Add profile evidence or explicitly review the empty catalog.",
)
_NO_RELEVANT_SUPPORT_GAP = EvidenceGapV1(
    code="evidence.no_relevant_support",
    message="No relevant evidence is linked to this criterion.",
    next_action="Add supported profile evidence or record the evidence gap.",
)
_DIRECT_SUPPORT_MISSING_GAP = EvidenceGapV1(
    code="evidence.direct_support_missing",
    message="Related evidence exists, but it does not directly support this criterion.",
    next_action="Add direct evidence or review the proposed weak match.",
)
_MORE_DETAIL_NEEDED_GAP = EvidenceGapV1(
    code="evidence.more_detail_needed",
    message="Direct evidence exists, but more detail is needed for strong support.",
    next_action="Review the linked evidence and add relevant context.",
)


class MatchStageError(ValueError):
    """Raised when Match inputs cannot form a safe deterministic projection."""


class MatchStageValidationError(MatchStageError):
    """Raised when a Match candidate cannot be accepted."""


def match_input_projection(
    job_dir: Path,
    *,
    criterion_matches_schema_path: Path | None = None,
) -> dict[str, str]:
    """Return the semantic and physical inputs that make Match current."""

    _load_match_inputs(job_dir)
    schema_text = _criterion_matches_schema_text(criterion_matches_schema_path)
    return {
        "stage": "match",
        "contract_version": MATCH_CONTRACT_VERSION,
        "matcher_strategy": MATCHER_STRATEGY,
        "matcher_version": MATCHER_VERSION,
        "criteria_catalog_sha256": _input_sha256(job_dir, CRITERIA_INPUT_PATH),
        "evidence_catalog_sha256": _input_sha256(
            job_dir,
            EVIDENCE_CATALOG_INPUT_PATH,
        ),
        "schema_sha256": sha256(schema_text.encode("utf-8")).hexdigest(),
    }


def match_input_fingerprint(
    job_dir: Path,
    *,
    criterion_matches_schema_path: Path | None = None,
) -> str:
    projection = match_input_projection(
        job_dir,
        criterion_matches_schema_path=criterion_matches_schema_path,
    )
    canonical = json.dumps(
        projection,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return sha256(canonical).hexdigest()


def build_deterministic_match_candidate(
    job_dir: Path,
    *,
    input_fingerprint: str | None = None,
    criterion_matches_schema_path: Path | None = None,
) -> CriterionMatchesV1:
    """Build one canonical proposed match for every catalog criterion."""

    current_fingerprint = match_input_fingerprint(
        job_dir,
        criterion_matches_schema_path=criterion_matches_schema_path,
    )
    if input_fingerprint is not None and input_fingerprint != current_fingerprint:
        raise MatchStageError("The requested Match input fingerprint is stale.")

    criteria_catalog, evidence_catalog = _load_match_inputs(job_dir)
    evidence_by_id = {
        item.evidence_id: item
        for item in sorted(evidence_catalog.items, key=lambda item: item.evidence_id)
    }
    index, indexed_items = _build_evidence_index(tuple(evidence_by_id.values()))

    matches = tuple(
        _match_criterion(
            criterion,
            evidence_catalog=evidence_catalog,
            index=index,
            indexed_items=indexed_items,
        )
        for criterion in sorted(
            criteria_catalog.criteria,
            key=lambda item: item.criterion_id,
        )
    )
    referenced_ids = sorted(
        {
            evidence_id
            for match in matches
            for evidence_id in match.evidence_ref_ids
        }
    )
    evidence_refs = tuple(
        _match_catalog_reference(evidence_by_id[evidence_id])
        for evidence_id in referenced_ids
    )
    return CriterionMatchesV1(
        job_id=criteria_catalog.job_id,
        input_fingerprint=current_fingerprint,
        criteria_catalog_sha256=_input_sha256(job_dir, CRITERIA_INPUT_PATH),
        evidence_catalog_sha256=_input_sha256(
            job_dir,
            EVIDENCE_CATALOG_INPUT_PATH,
        ),
        matcher_strategy=MATCHER_STRATEGY,
        matcher_version=MATCHER_VERSION,
        evidence_refs=evidence_refs,
        matches=matches,
    )


def validate_match_candidate(
    candidate: object,
    *,
    job_dir: Path,
    input_fingerprint: str,
    criterion_matches_schema_path: Path | None = None,
) -> CriterionMatchesV1:
    """Validate a candidate by rebuilding the complete canonical Match projection."""

    if not isinstance(candidate, dict):
        raise MatchStageValidationError("Match candidate must be a JSON object.")
    try:
        validator = compiled_schema_validator(
            _criterion_matches_schema_text(criterion_matches_schema_path)
        )
    except SchemaCompilationError as exc:
        raise MatchStageValidationError(
            "The configured Criterion Matches schema is invalid."
        ) from exc
    if list(validator.iter_errors(candidate)):
        raise MatchStageValidationError(
            "Match candidate failed schema validation."
        )
    try:
        validated = CriterionMatchesV1.model_validate(candidate)
    except ValidationError as exc:
        raise MatchStageValidationError(
            "Match candidate failed semantic validation."
        ) from exc

    current_fingerprint = match_input_fingerprint(
        job_dir,
        criterion_matches_schema_path=criterion_matches_schema_path,
    )
    if (
        input_fingerprint != current_fingerprint
        or validated.input_fingerprint != input_fingerprint
    ):
        raise MatchStageValidationError(
            "Match candidate input fingerprint is stale."
        )
    expected = build_deterministic_match_candidate(
        job_dir,
        input_fingerprint=input_fingerprint,
        criterion_matches_schema_path=criterion_matches_schema_path,
    )
    if validated.model_dump(mode="json") != expected.model_dump(mode="json"):
        raise MatchStageValidationError(
            "Match candidate does not match the current canonical projection."
        )
    return validated


def _match_criterion(
    criterion: CriterionV1,
    *,
    evidence_catalog: EvidenceCatalogV1,
    index: EvidenceIndex | None,
    indexed_items: dict[str, EvidenceCatalogItemV1],
) -> CriterionMatchV1:
    if evidence_catalog.state in {"empty", "unavailable"}:
        return CriterionMatchV1(
            criterion_id=criterion.criterion_id,
            classification="unknown",
            gaps=(
                _CATALOG_EMPTY_GAP
                if evidence_catalog.state == "empty"
                else _CATALOG_UNAVAILABLE_GAP,
            ),
            review_state="proposed",
        )

    if index is None:
        raise MatchStageError(
            "An available Evidence Catalog requires indexable evidence items."
        )
    proposed_items = index.search(criterion.text)
    evidence_ids = tuple(
        sorted(
            {
                indexed_items[item.citation].evidence_id
                for item in proposed_items
            }
        )
    )
    direct_match_count = sum(
        1
        for item in proposed_items
        if _direct_overlap_score(
            criterion.text.casefold(),
            EvidenceReference(
                source_file=item.source_file,
                section=item.section,
                item_id=item.item_id,
                text=indexed_items[item.citation].text,
            ),
        )
        > 0
    )
    if direct_match_count >= 2:
        classification = "strong"
    elif direct_match_count == 1:
        classification = "partial"
    elif evidence_ids:
        classification = "weak"
    else:
        classification = "missing"

    gaps: tuple[EvidenceGapV1, ...]
    if classification == "missing":
        gaps = (_NO_RELEVANT_SUPPORT_GAP,)
    elif classification == "weak":
        gaps = (_DIRECT_SUPPORT_MISSING_GAP,)
    elif classification == "partial":
        gaps = (_MORE_DETAIL_NEEDED_GAP,)
    else:
        gaps = ()
    return CriterionMatchV1(
        criterion_id=criterion.criterion_id,
        classification=classification,
        evidence_ref_ids=evidence_ids,
        gaps=gaps,
        review_state="proposed",
    )


def _build_evidence_index(
    items: tuple[EvidenceCatalogItemV1, ...],
) -> tuple[EvidenceIndex | None, dict[str, EvidenceCatalogItemV1]]:
    if not items:
        return None, {}
    references: list[EvidenceReference] = []
    indexed: dict[str, EvidenceCatalogItemV1] = {}
    for item in items:
        reference = EvidenceReference(
            source_file=f"semantic/{item.evidence_id}",
            section="item",
            text=f"`{item.kind}`: {item.text}",
        )
        references.append(reference)
        indexed[reference.citation] = item
    return EvidenceIndex(references), indexed


def _match_catalog_reference(item: EvidenceCatalogItemV1) -> EvidenceRefV1:
    """Use an opaque job-local locator so Match never copies profile headings or labels."""

    return EvidenceRefV1(
        evidence_id=item.evidence_id,
        path=EVIDENCE_CATALOG_INPUT_PATH,
        section="items",
        item_locator=item.evidence_id,
        kind="catalog_item",
        content_sha256=item.content_sha256,
    )


def _load_match_inputs(
    job_dir: Path,
) -> tuple[CriteriaCatalogV1, EvidenceCatalogV1]:
    try:
        criteria = CriteriaCatalogV1.model_validate(
            read_json_object(job_dir / CRITERIA_INPUT_PATH)
        )
        evidence = EvidenceCatalogV1.model_validate(
            read_json_object(job_dir / EVIDENCE_CATALOG_INPUT_PATH)
        )
    except (StageStoreError, ValidationError) as exc:
        raise MatchStageError(
            "Match requires current valid Criteria and Evidence Catalog inputs."
        ) from exc
    if criteria.job_id != job_dir.name or evidence.job_id != job_dir.name:
        raise MatchStageError("Match inputs belong to a different job.")
    if criteria.extraction_state == "unknown":
        raise MatchStageError(
            "Match requires criteria extraction to be resolved or explicitly confirmed empty."
        )
    return criteria, evidence


def _input_sha256(job_dir: Path, relative_path: str) -> str:
    try:
        return sha256_file(job_dir / relative_path)
    except StageStoreError as exc:
        raise MatchStageError("A Match input cannot be hashed safely.") from exc


def _criterion_matches_schema_text(schema_path: Path | None) -> str:
    try:
        return read_resource_text(
            "schemas/criterion-matches.schema.json",
            local_path=schema_path,
        )
    except (OSError, UnicodeError) as exc:
        raise MatchStageError(
            "The Criterion Matches schema is not readable."
        ) from exc
