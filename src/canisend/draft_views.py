from __future__ import annotations

from dataclasses import dataclass
import hashlib
import html
from pathlib import Path
import re
from typing import Any

from pydantic import ValidationError

from canisend.draft_models import CoverLetterDraftV1, ReviewFindingsV1
from canisend.review_readiness import (
    DocumentReadinessV1,
    derive_document_readiness,
)
from canisend.stage_runtime import (
    StageRuntimeError,
    StageStatusInspection,
    inspect_stage_status,
)
from canisend.stage_store import StageStoreError, read_json_object, sha256_file
from canisend.stages.draft_stage import COVER_LETTER_DRAFT_OUTPUT_PATH
from canisend.stages.review_stage import (
    REVIEW_FINDINGS_OUTPUT_PATH,
    validate_review_candidate,
)
from canisend.user_mutations import (
    REVIEW_DISPOSITIONS_PATH,
    UserMutationError,
    inspect_current_artifact_mutation,
    inspect_review_dispositions,
)
from canisend.workspace import load_workspace_config


PARSED_JOB_PATH = "parsed_job.json"
STRUCTURED_DRAFT_PROJECTION_SOURCE = COVER_LETTER_DRAFT_OUTPUT_PATH
STRUCTURED_DRAFT_TYPST_MARKER = "// CANISEND: structured-draft projection"


@dataclass(frozen=True)
class StructuredDraftViews:
    draft: CoverLetterDraftV1
    review: ReviewFindingsV1
    markdown: str
    draft_sha256: str
    review_sha256: str
    document_readiness: DocumentReadinessV1


def load_current_structured_draft_views(
    workspace: Path,
    job_dir: Path,
    *,
    parsed_job: dict[str, Any],
) -> StructuredDraftViews | None:
    """Return a current Draft with blocker-free Review or use the safe legacy path."""

    draft_path = job_dir / COVER_LETTER_DRAFT_OUTPUT_PATH
    review_path = job_dir / REVIEW_FINDINGS_OUTPUT_PATH
    try:
        inspection = inspect_stage_status(workspace, job_dir, stage="review")
        if not _inspection_is_current(inspection):
            return None
        if read_json_object(job_dir / PARSED_JOB_PATH) != parsed_job:
            return None

        draft_hash = sha256_file(draft_path)
        review_hash = sha256_file(review_path)
        draft = CoverLetterDraftV1.model_validate(read_json_object(draft_path))
        review_payload = read_json_object(review_path)
        review = ReviewFindingsV1.model_validate(review_payload)
        schema_dir = load_workspace_config(workspace).path("schema_dir")
        review = validate_review_candidate(
            review_payload,
            workspace=workspace,
            job_dir=job_dir,
            input_fingerprint=review.input_fingerprint,
            review_findings_schema_path=schema_dir / "review-findings.schema.json",
            cover_letter_schema_path=schema_dir / "cover-letter-draft.schema.json",
            parsed_job_schema_path=schema_dir / "parsed_job.schema.json",
            required_document_plan_schema_path=(
                schema_dir / "required-document-plan.schema.json"
            ),
        )
        if review.blocker_finding_ids:
            return None
        disposition_inspection = inspect_review_dispositions(workspace, job_dir)
        mutation_audit = inspect_current_artifact_mutation(
            workspace,
            job_dir,
            "review_dispositions",
        )
        if (
            disposition_inspection.readiness is not None
            and mutation_audit.status in {"untracked", "committed"}
        ):
            document_readiness = disposition_inspection.readiness
        else:
            document_readiness = derive_document_readiness(
                review,
                draft_sha256=draft_hash,
                review_findings_sha256=review_hash,
                dispositions=None,
                review_dispositions_sha256=None,
            )
        markdown = render_structured_draft_markdown(draft)

        final_inspection = inspect_stage_status(workspace, job_dir, stage="review")
        if (
            not _inspection_is_current(final_inspection)
            or sha256_file(draft_path) != draft_hash
            or sha256_file(review_path) != review_hash
            or read_json_object(job_dir / PARSED_JOB_PATH) != parsed_job
            or (
                document_readiness.review_dispositions_sha256 is not None
                and sha256_file(job_dir / REVIEW_DISPOSITIONS_PATH)
                != document_readiness.review_dispositions_sha256
            )
        ):
            return None
        return StructuredDraftViews(
            draft=draft,
            review=review,
            markdown=markdown,
            draft_sha256=draft_hash,
            review_sha256=review_hash,
            document_readiness=document_readiness,
        )
    except (
        AttributeError,
        OSError,
        StageRuntimeError,
        StageStoreError,
        TypeError,
        UnicodeError,
        ValidationError,
        UserMutationError,
        ValueError,
    ):
        return None


def render_structured_draft_markdown(draft: CoverLetterDraftV1) -> str:
    """Render Claim text once, in Draft order, without agent-controlled Markdown structure."""

    lines = ["# Cover Letter Draft", ""]
    for section in draft.sections:
        for claim in section.claims:
            lines.extend(_markdown_plain_text(claim.text).splitlines())
            lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def build_structured_cover_letter_content(
    parsed_job: dict[str, Any],
    views: StructuredDraftViews,
) -> dict[str, Any]:
    """Build a Typst-ready compatibility payload without synthesizing new prose."""

    draft = views.draft
    review = views.review
    readiness = views.document_readiness
    return {
        "job": {
            "title": parsed_job["title"],
            "institution": parsed_job["institution"],
            "department": parsed_job["department"],
            "deadline": parsed_job["deadline"],
            "application_url": parsed_job["application_url"],
        },
        "recipient": {
            "start_title": "",
            "cl_title": f"Application for {parsed_job['title']}",
            "date": "",
            "department": _unknown_to_empty(parsed_job["department"]),
            "institution": _unknown_to_empty(parsed_job["institution"]),
            "address": "",
            "postcode": "",
        },
        "salutation": "",
        "projection": {
            "source": STRUCTURED_DRAFT_PROJECTION_SOURCE,
            "review_source": REVIEW_FINDINGS_OUTPUT_PATH,
            "job_id": draft.job_id,
            "document_id": draft.document_id,
            "draft_schema_version": draft.schema_version,
            "draft_review_state": draft.review_state,
            "draft_input_fingerprint": draft.input_fingerprint,
            "draft_sha256": views.draft_sha256,
            "review_schema_version": review.schema_version,
            "review_state": review.review_state,
            "review_input_fingerprint": review.input_fingerprint,
            "review_sha256": views.review_sha256,
            "review_dispositions_source": REVIEW_DISPOSITIONS_PATH,
            "document_readiness": readiness.model_dump(mode="json"),
            "document_readiness_state": readiness.state,
            "markdown_sha256": hashlib.sha256(
                views.markdown.encode("utf-8")
            ).hexdigest(),
            "finding_count": len(review.findings),
            "blocker_count": len(review.blocker_finding_ids),
            "draft_blocker_count": len(draft.blockers),
            "review_blocker_count": len(review.blocker_finding_ids),
            "requires_human_review": readiness.state != "reviewed",
        },
        "structured_sections": [
            {
                "section_id": section.section_id,
                "claims": [
                    claim.model_dump(mode="json") for claim in section.claims
                ],
            }
            for section in draft.sections
        ],
    }


def _inspection_is_current(inspection: StageStatusInspection) -> bool:
    return bool(
        inspection.stage.status == "succeeded"
        and not inspection.reasons
        and not inspection.output_drift
    )


def _markdown_plain_text(value: str) -> str:
    escaped = html.escape(value, quote=False)
    escaped = escaped.replace("\\", "\\\\")
    escaped = escaped.replace("`", "\\`")
    escaped = escaped.replace("[", "\\[").replace("]", "\\]")
    escaped = re.sub(r"(?m)^ {4}", "&#32;   ", escaped)
    escaped = re.sub(r"(?m)^\t", "&#9;", escaped)
    escaped = re.sub(
        r"(?m)^([ \t]{0,3})([#>*+_|=~-])",
        r"\1\\\2",
        escaped,
    )
    escaped = re.sub(r"(?m)^(\s{0,3})(\d+)([.)])(?=\s)", r"\1\2\\\3", escaped)
    return escaped


def _unknown_to_empty(value: str) -> str:
    return "" if value == "unknown" else value
