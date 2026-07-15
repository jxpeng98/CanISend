from __future__ import annotations

from pathlib import Path

from canisend.agent_protocol import (
    AgentResponse,
    NextAction,
    artifact_reference_from_path,
    error_response,
    success_response,
)
from canisend.discovery.catalog_models import LeadCatalogV1
from canisend.discovery.local_import import DiscoveryLocalImportExecution
from canisend.discovery.refresh import DiscoveryRefreshExecution
from canisend.discovery.search_import import DiscoverySearchImportExecution


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


def discovery_refresh_agent_response(
    workspace: Path,
    execution: DiscoveryRefreshExecution,
    *,
    operation: str = "discovery.refresh",
) -> AgentResponse:
    report = execution.report
    artifacts = [
        artifact_reference_from_path(
            workspace=workspace,
            path=execution.report_path,
            kind="discovery-refresh-report",
            privacy_tier=1,
            trust_level="validated",
            media_type="application/json",
            include_hash=True,
        )
    ]
    if execution.catalog_path is not None:
        artifacts.insert(
            0,
            artifact_reference_from_path(
                workspace=workspace,
                path=execution.catalog_path,
                kind="discovery-catalog",
                privacy_tier=1,
                trust_level="untrusted_import",
                media_type="application/json",
                include_hash=True,
            ),
        )
    warnings: list[str] = []
    if report.status == "partial":
        warnings.append("discovery.partial_refresh")
    if report.stale_sources:
        warnings.append("discovery.stale_sources_reused")
    if report.failed_sources:
        warnings.append("discovery.sources_unavailable")
    extensions = {
        "canisend.discovery.refresh_id": report.refresh_id,
        "canisend.discovery.refresh_status": report.status,
        "canisend.discovery.catalog_id": report.catalog_id,
        "canisend.discovery.source_count": report.source_count,
        "canisend.discovery.successful_sources": report.successful_sources,
        "canisend.discovery.stale_sources": report.stale_sources,
        "canisend.discovery.failed_sources": report.failed_sources,
        "canisend.discovery.input_records": report.input_records,
        "canisend.discovery.retained_records": report.retained_records,
        "canisend.discovery.excluded_records": report.excluded_records,
    }
    if report.status == "failed":
        return error_response(
            operation=operation,
            code="source.import_failed",
            message="No validated discovery catalog could be promoted.",
            retryable=True,
            artifacts=artifacts,
            warnings=["discovery.refresh_failed", *warnings],
            next_actions=[
                NextAction(
                    id="discovery.retry_refresh",
                    label="Inspect the body-free refresh report and retry failed sources",
                )
            ],
            extensions=extensions,
        )
    return success_response(
        operation=operation,
        artifacts=artifacts,
        warnings=warnings,
        next_actions=[
            NextAction(
                id="job.intake_from_lead",
                label="Select a retained lead by stable lead ID",
            )
            if report.retained_records
            else NextAction(
                id="discovery.adjust_filters",
                label="Adjust discovery filters or source inputs",
            )
        ],
        extensions=extensions,
    )


def discovery_local_import_agent_response(
    workspace: Path,
    execution: DiscoveryLocalImportExecution,
    *,
    operation: str = "discovery.import",
) -> AgentResponse:
    report = execution.report
    artifacts = [
        artifact_reference_from_path(
            workspace=workspace,
            path=execution.report_path,
            kind="discovery-import-report",
            privacy_tier=1,
            trust_level="validated",
            media_type="application/json",
            include_hash=True,
        )
    ]
    if execution.batch_path is not None:
        artifacts.insert(
            0,
            artifact_reference_from_path(
                workspace=workspace,
                path=execution.batch_path,
                kind="discovery-lead-batch",
                privacy_tier=1,
                trust_level="untrusted_import",
                media_type="application/json",
                include_hash=True,
            ),
        )
    if execution.catalog_path is not None:
        artifacts.insert(
            0,
            artifact_reference_from_path(
                workspace=workspace,
                path=execution.catalog_path,
                kind="discovery-catalog",
                privacy_tier=1,
                trust_level="untrusted_import",
                media_type="application/json",
                include_hash=True,
            ),
        )
    warnings: list[str] = []
    if report.status == "partial":
        warnings.append("discovery.import_rows_rejected")
    if report.ignored_records:
        warnings.append("discovery.import_records_ignored")
    extensions = {
        "canisend.discovery.import_id": report.import_id,
        "canisend.discovery.import_status": report.status,
        "canisend.discovery.import_format": report.format,
        "canisend.discovery.batch_id": report.batch_id,
        "canisend.discovery.catalog_id": report.catalog_id,
        "canisend.discovery.input_records": report.input_records,
        "canisend.discovery.imported_records": report.imported_records,
        "canisend.discovery.rejected_records": report.rejected_records,
        "canisend.discovery.ignored_records": report.ignored_records,
        "canisend.discovery.retained_records": report.retained_records,
        "canisend.discovery.excluded_records": report.excluded_records,
    }
    if report.status == "failed":
        return error_response(
            operation=operation,
            code="source.import_failed",
            message="No validated local discovery import could be promoted.",
            retryable=False,
            artifacts=artifacts,
            warnings=["discovery.import_failed", *warnings],
            next_actions=[
                NextAction(
                    id="discovery.review_import_report",
                    label="Inspect the body-free import report and correct the export",
                )
            ],
            extensions=extensions,
        )
    return success_response(
        operation=operation,
        artifacts=artifacts,
        warnings=warnings,
        next_actions=[
            NextAction(
                id="job.intake_from_lead",
                label="Select a retained lead by stable lead ID",
            )
            if report.retained_records
            else NextAction(
                id="discovery.adjust_filters",
                label="Adjust discovery filters or local inputs",
            )
        ],
        extensions=extensions,
    )


def discovery_search_import_agent_response(
    workspace: Path,
    execution: DiscoverySearchImportExecution,
    *,
    operation: str = "discovery.search_import",
) -> AgentResponse:
    """Return control metadata only; imported titles and snippets stay in artifacts."""

    catalog = execution.catalog
    return success_response(
        operation=operation,
        artifacts=[
            artifact_reference_from_path(
                workspace=workspace,
                path=execution.catalog_path,
                kind="discovery-catalog",
                privacy_tier=1,
                trust_level="untrusted_import",
                media_type="application/json",
                include_hash=True,
            ),
            artifact_reference_from_path(
                workspace=workspace,
                path=execution.batch_path,
                kind="discovery-lead-batch",
                privacy_tier=1,
                trust_level="untrusted_import",
                media_type="application/json",
                include_hash=True,
            ),
        ],
        next_actions=[
            NextAction(
                id="job.intake_from_lead",
                label="Select a retained host-search lead by stable lead ID",
            )
            if catalog.leads
            else NextAction(
                id="discovery.adjust_filters",
                label="Adjust discovery filters or host search inputs",
            )
        ],
        extensions={
            "canisend.discovery.source_id": execution.envelope.source_id,
            "canisend.discovery.batch_id": execution.batch.batch_id,
            "canisend.discovery.catalog_id": catalog.catalog_id,
            "canisend.discovery.input_records": execution.envelope.result_count,
            "canisend.discovery.imported_records": execution.batch.record_count,
            "canisend.discovery.retained_records": catalog.stats.retained_records,
            "canisend.discovery.excluded_records": catalog.stats.excluded_records,
        },
    )
