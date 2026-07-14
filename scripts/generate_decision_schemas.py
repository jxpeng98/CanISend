from __future__ import annotations

import json
from pathlib import Path

from canisend.decision_models import (
    ApplicationBriefV1,
    ApplicationDecisionV1,
    ConfirmedCorrectionsV1,
    CriteriaCatalogV1,
    CriterionMatchesV1,
    EvidenceCatalogV1,
    RequiredDocumentPlanV1,
)
from canisend.draft_models import (
    CoverLetterDraftV1,
    ResearchStatementDraftV1,
    ReviewFindingsV1,
)
from canisend.document_execution import DocumentExecutionPlanV1
from canisend.package_review_models import PackageReviewFindingsV1
from canisend.review_readiness import DocumentReadinessV1, ReviewDispositionsV1
from canisend.user_mutations import UserMutationReceiptV1


SCHEMAS = {
    "criteria.schema.json": CriteriaCatalogV1,
    "evidence-catalog.schema.json": EvidenceCatalogV1,
    "criterion-matches.schema.json": CriterionMatchesV1,
    "confirmed-corrections.schema.json": ConfirmedCorrectionsV1,
    "application-decision.schema.json": ApplicationDecisionV1,
    "application-brief.schema.json": ApplicationBriefV1,
    "required-document-plan.schema.json": RequiredDocumentPlanV1,
    "cover-letter-draft.schema.json": CoverLetterDraftV1,
    "research-statement-draft.schema.json": ResearchStatementDraftV1,
    "review-findings.schema.json": ReviewFindingsV1,
    "review-dispositions.schema.json": ReviewDispositionsV1,
    "document-readiness.schema.json": DocumentReadinessV1,
    "document-execution-plan.schema.json": DocumentExecutionPlanV1,
    "package-review-findings.schema.json": PackageReviewFindingsV1,
    "user-mutation-receipt.schema.json": UserMutationReceiptV1,
}


def main() -> None:
    schema_dir = Path(__file__).resolve().parents[1] / "schemas"
    for filename, model in SCHEMAS.items():
        schema = model.model_json_schema(mode="validation")
        (schema_dir / filename).write_text(
            json.dumps(schema, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )


if __name__ == "__main__":
    main()
