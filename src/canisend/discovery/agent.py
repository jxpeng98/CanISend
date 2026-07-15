from __future__ import annotations

from pathlib import Path

from canisend.agent_protocol import (
    AgentResponse,
    NextAction,
    artifact_reference_from_path,
    success_response,
)
from canisend.discovery.catalog_models import LeadCatalogV1


def discovery_catalog_agent_response(
    workspace: Path,
    catalog_path: Path,
    catalog: LeadCatalogV1,
    *,
    operation: str = "discovery.merge",
) -> AgentResponse:
    actions = [
        NextAction(
            id="job.intake_from_lead",
            label="Select a retained lead by stable lead ID",
        )
        if catalog.leads
        else NextAction(
            id="discovery.adjust_filters",
            label="Adjust discovery filters or source inputs",
        )
    ]
    warnings = ["discovery.exclusions_present"] if catalog.excluded else []
    return success_response(
        operation=operation,
        artifacts=[
            artifact_reference_from_path(
                workspace=workspace,
                path=catalog_path,
                kind="discovery-catalog",
                privacy_tier=1,
                trust_level="untrusted_import",
                media_type="application/json",
                include_hash=True,
            )
        ],
        warnings=warnings,
        next_actions=actions,
        extensions={
            "canisend.discovery.catalog_id": catalog.catalog_id,
            "canisend.discovery.input_records": catalog.stats.input_records,
            "canisend.discovery.merged_records": catalog.stats.merged_records,
            "canisend.discovery.retained_records": catalog.stats.retained_records,
            "canisend.discovery.excluded_records": catalog.stats.excluded_records,
            "canisend.discovery.source_count": catalog.stats.source_count,
        },
    )
