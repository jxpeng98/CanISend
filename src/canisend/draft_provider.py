from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Annotated, Mapping

from pydantic import ConfigDict, Field, ValidationError, field_validator

from canisend.decision_models import (
    BriefFieldName,
    CriterionIdentifier,
    DecisionContractModel,
    DottedIdentifier,
    EvidenceIdentifier,
    SlugIdentifier,
)
from canisend.draft_models import (
    ClaimKind,
    ClaimSupportStrength,
    JobFieldName,
    stable_claim_id,
)
from canisend.llm import LLMProvider
from canisend.resource_files import read_resource_text
from canisend.stages.draft_stage import (
    CONFIGURED_PROVIDER_DRAFT_GENERATOR_STRATEGY,
    DRAFT_GENERATOR_VERSION,
    DraftStageValidationError,
    draft_input_fingerprint,
    draft_input_projection,
    validate_draft_candidate,
)
from canisend.workspace import load_workspace_config


MAX_PROVIDER_DRAFT_RESPONSE_BYTES = 2_000_000

_BoundedProposalBody = Annotated[
    str,
    Field(min_length=1, max_length=20_000, pattern=r"[\s\S]*\S[\s\S]*"),
]


class DraftProviderError(ValueError):
    """A stable provider-Draft failure that must not expose private content."""


class DraftProviderExecutionError(DraftProviderError):
    """The configured provider could not complete the request."""


class DraftProviderResponseError(DraftProviderError):
    """The provider response could not form a valid guarded Draft candidate."""


class DraftProviderInputChangedError(DraftProviderError):
    """The declared Draft inputs changed during provider execution."""


class ProviderClaimProposalV1(DecisionContractModel):
    text: _BoundedProposalBody
    kind: ClaimKind
    support_strength: ClaimSupportStrength
    criterion_ids: tuple[CriterionIdentifier, ...] = Field(default=(), max_length=4_096)
    evidence_ref_ids: tuple[EvidenceIdentifier, ...] = Field(default=(), max_length=4_096)
    brief_field_refs: tuple[BriefFieldName, ...] = Field(default=(), max_length=6)
    job_field_refs: tuple[JobFieldName, ...] = Field(default=(), max_length=6)
    blockers: tuple[DottedIdentifier, ...] = Field(default=(), max_length=64)

    @field_validator(
        "criterion_ids",
        "evidence_ref_ids",
        "brief_field_refs",
        "job_field_refs",
        "blockers",
    )
    @classmethod
    def _unique_refs(cls, values: tuple[str, ...], info: object) -> tuple[str, ...]:
        if len(values) != len(set(values)):
            raise ValueError(f"{getattr(info, 'field_name', 'references')} must be unique")
        return values


class ProviderSectionProposalV1(DecisionContractModel):
    section_id: SlugIdentifier
    claims: tuple[ProviderClaimProposalV1, ...] = Field(min_length=1, max_length=4_096)


class ProviderDraftProposalV1(DecisionContractModel):
    model_config = ConfigDict(
        title="CanISendProviderDraftProposalV1",
        extra="forbid",
        frozen=True,
        str_strip_whitespace=True,
        validate_default=True,
    )

    sections: tuple[ProviderSectionProposalV1, ...] = Field(min_length=1, max_length=256)


def build_configured_provider_draft_candidate(
    *,
    workspace: Path,
    job_dir: Path,
    input_fingerprint: str,
    input_documents: Mapping[str, object],
    provider: LLMProvider,
) -> dict[str, object]:
    """Generate and prevalidate one full structured Draft candidate.

    The provider proposes only sections and Claim semantics. Core-owned identity,
    basis receipts, stable IDs, execution mode, and review state are derived here.
    """

    try:
        schema_paths = _draft_schema_paths(workspace)
        projection = draft_input_projection(
            workspace,
            job_dir,
            cover_letter_schema_path=schema_paths[0],
            parsed_job_schema_path=schema_paths[1],
            required_document_plan_schema_path=schema_paths[2],
        )
    except (OSError, UnicodeError, ValueError) as exc:
        raise DraftProviderResponseError(
            "The current Draft inputs cannot form a provider request."
        ) from exc
    try:
        current_fingerprint = draft_input_fingerprint(
            workspace,
            job_dir,
            cover_letter_schema_path=schema_paths[0],
            parsed_job_schema_path=schema_paths[1],
            required_document_plan_schema_path=schema_paths[2],
        )
    except (OSError, UnicodeError, ValueError) as exc:
        raise DraftProviderInputChangedError(
            "The configured-provider Draft inputs are no longer readable."
        ) from exc
    if projection.get("stage") != "draft" or input_fingerprint != current_fingerprint:
        raise DraftProviderInputChangedError(
            "The configured-provider Draft task no longer matches current inputs."
        )

    try:
        prompt = _render_provider_prompt(
            input_fingerprint=input_fingerprint,
            job_id=job_dir.name,
            projection=projection,
            input_documents=input_documents,
        )
    except (OSError, UnicodeError, ValueError) as exc:
        raise DraftProviderExecutionError(
            "The configured-provider Draft prompt is unavailable."
        ) from exc
    try:
        response = provider.complete(prompt)
    except Exception as exc:
        raise DraftProviderExecutionError(
            "The configured provider could not complete the Draft request."
        ) from exc

    try:
        final_fingerprint = draft_input_fingerprint(
            workspace,
            job_dir,
            cover_letter_schema_path=schema_paths[0],
            parsed_job_schema_path=schema_paths[1],
            required_document_plan_schema_path=schema_paths[2],
        )
    except (OSError, UnicodeError, ValueError) as exc:
        raise DraftProviderInputChangedError(
            "The configured-provider Draft inputs changed during generation."
        ) from exc
    if final_fingerprint != input_fingerprint:
        raise DraftProviderInputChangedError(
            "The configured-provider Draft inputs changed during generation."
        )

    proposal = _parse_provider_proposal(response.content)
    candidate = _assemble_candidate(
        job_dir=job_dir,
        input_fingerprint=input_fingerprint,
        projection=projection,
        proposal=proposal,
    )
    try:
        validated = validate_draft_candidate(
            candidate,
            workspace=workspace,
            job_dir=job_dir,
            input_fingerprint=input_fingerprint,
            cover_letter_schema_path=schema_paths[0],
            parsed_job_schema_path=schema_paths[1],
            required_document_plan_schema_path=schema_paths[2],
            expected_generation_mode="configured_provider",
        )
    except DraftStageValidationError as exc:
        raise DraftProviderResponseError(
            "The configured provider returned an invalid Draft proposal."
        ) from exc
    return validated.model_dump(mode="json")


def _render_provider_prompt(
    *,
    input_fingerprint: str,
    job_id: str,
    projection: Mapping[str, object],
    input_documents: Mapping[str, object],
) -> str:
    template = read_resource_text("prompts/structured_cover_letter_draft.md")
    control = {
        "job_id": job_id,
        "document_id": projection.get("cover_letter_document_id"),
        "input_fingerprint": input_fingerprint,
    }
    private_inputs = [
        {"path": path, "content": content}
        for path, content in sorted(input_documents.items())
    ]
    return template.replace(
        "{draft_control}",
        json.dumps(control, ensure_ascii=False, indent=2, sort_keys=True),
    ).replace(
        "{declared_private_inputs}",
        json.dumps(private_inputs, ensure_ascii=False, indent=2, sort_keys=True),
    )


def _draft_schema_paths(workspace: Path) -> tuple[Path, Path, Path]:
    schema_dir = load_workspace_config(workspace).path("schema_dir")
    return (
        schema_dir / "cover-letter-draft.schema.json",
        schema_dir / "parsed_job.schema.json",
        schema_dir / "required-document-plan.schema.json",
    )


def _parse_provider_proposal(content: str) -> ProviderDraftProposalV1:
    if not isinstance(content, str):
        raise DraftProviderResponseError(
            "The configured provider returned invalid text."
        )
    try:
        size_bytes = len(content.encode("utf-8"))
    except UnicodeError as exc:
        raise DraftProviderResponseError(
            "The configured provider returned invalid text."
        ) from exc
    if size_bytes == 0 or size_bytes > MAX_PROVIDER_DRAFT_RESPONSE_BYTES:
        raise DraftProviderResponseError(
            "The configured provider response is empty or exceeds the Draft limit."
        )

    stripped = content.strip()
    fenced = re.findall(
        r"```(?:json)?\s*\n(.*?)\n```",
        stripped,
        flags=re.DOTALL | re.IGNORECASE,
    )
    if fenced:
        if len(fenced) != 1:
            raise DraftProviderResponseError(
                "The configured provider must return exactly one JSON proposal."
            )
        stripped = fenced[0].strip()
    try:
        payload = json.loads(
            stripped,
            parse_constant=_reject_json_constant,
            object_pairs_hook=_unique_json_object,
        )
        return ProviderDraftProposalV1.model_validate(payload)
    except (json.JSONDecodeError, ValidationError, ValueError) as exc:
        raise DraftProviderResponseError(
            "The configured provider must return one valid Draft proposal object."
        ) from exc


def _reject_json_constant(value: str) -> object:
    raise ValueError(f"Non-finite JSON constant is not allowed: {value}")


def _unique_json_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate JSON object key is not allowed.")
        result[key] = value
    return result


def _assemble_candidate(
    *,
    job_dir: Path,
    input_fingerprint: str,
    projection: Mapping[str, object],
    proposal: ProviderDraftProposalV1,
) -> dict[str, object]:
    document_id = projection.get("cover_letter_document_id")
    if not isinstance(document_id, str):
        raise DraftProviderResponseError(
            "The current Draft task does not identify one Cover Letter."
        )

    sections: list[dict[str, object]] = []
    aggregate_blockers: set[str] = set()
    for section in proposal.sections:
        claims: list[dict[str, object]] = []
        for claim in section.claims:
            blockers = tuple(sorted(claim.blockers))
            aggregate_blockers.update(blockers)
            claims.append(
                {
                    "claim_id": stable_claim_id(
                        job_id=job_dir.name,
                        document_id=document_id,
                        kind=claim.kind,
                        text=claim.text,
                    ),
                    "text": claim.text,
                    "kind": claim.kind,
                    "support_strength": claim.support_strength,
                    "criterion_ids": sorted(claim.criterion_ids),
                    "evidence_ref_ids": sorted(claim.evidence_ref_ids),
                    "brief_field_refs": sorted(claim.brief_field_refs),
                    "job_field_refs": sorted(claim.job_field_refs),
                    "blockers": list(blockers),
                    "review_state": "proposed",
                }
            )
        sections.append(
            {
                "section_id": section.section_id,
                "heading": None,
                "claims": claims,
            }
        )

    basis_keys = (
        "parsed_job_sha256",
        "criteria_sha256",
        "evidence_catalog_sha256",
        "criterion_matches_sha256",
        "application_decision_sha256",
        "application_brief_sha256",
        "required_document_plan_sha256",
    )
    return {
        "schema_version": "1.0.0",
        "job_id": job_dir.name,
        "document_id": document_id,
        "input_fingerprint": input_fingerprint,
        "basis": {key: projection.get(key) for key in basis_keys},
        "generation_mode": "configured_provider",
        "generator_strategy": CONFIGURED_PROVIDER_DRAFT_GENERATOR_STRATEGY,
        "generator_version": DRAFT_GENERATOR_VERSION,
        "review_state": "proposed",
        "sections": sections,
        "blockers": sorted(aggregate_blockers),
    }
