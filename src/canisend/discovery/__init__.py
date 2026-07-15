"""Versioned, source-neutral job discovery contracts and normalization."""

from canisend.discovery.identity import (
    canonicalize_job_url,
    normalize_job_lead,
    stable_lead_id,
)
from canisend.discovery.catalog import (
    build_catalog_from_files,
    load_lead_document,
    merge_lead_catalog,
    write_lead_catalog,
)
from canisend.discovery.catalog_models import (
    LEAD_CATALOG_PROTOCOL,
    LEAD_CATALOG_SCHEMA_VERSION,
    LeadCatalogV1,
    RankingPolicyV1,
    normalized_ranking_policy,
)
from canisend.discovery.models import (
    JOB_LEAD_SCHEMA_VERSION,
    JobLeadV2,
    LeadMatchReasonV1,
    LeadProvenanceV1,
)
from canisend.discovery.refresh_models import (
    DISCOVERY_CACHE_PROTOCOL,
    DISCOVERY_REFRESH_REPORT_PROTOCOL,
    DISCOVERY_SOURCES_PROTOCOL,
    LEAD_BATCH_PROTOCOL,
    DiscoveryCacheV1,
    DiscoveryRefreshReportV1,
    DiscoverySourceV1,
    DiscoverySourcesV1,
    LeadBatchV1,
    SourceRefreshResultV1,
)

__all__ = [
    "JOB_LEAD_SCHEMA_VERSION",
    "DISCOVERY_CACHE_PROTOCOL",
    "DISCOVERY_REFRESH_REPORT_PROTOCOL",
    "DISCOVERY_SOURCES_PROTOCOL",
    "LEAD_BATCH_PROTOCOL",
    "LEAD_CATALOG_PROTOCOL",
    "LEAD_CATALOG_SCHEMA_VERSION",
    "JobLeadV2",
    "LeadCatalogV1",
    "LeadBatchV1",
    "LeadMatchReasonV1",
    "LeadProvenanceV1",
    "DiscoveryCacheV1",
    "DiscoveryRefreshReportV1",
    "DiscoverySourceV1",
    "DiscoverySourcesV1",
    "RankingPolicyV1",
    "SourceRefreshResultV1",
    "build_catalog_from_files",
    "canonicalize_job_url",
    "load_lead_document",
    "merge_lead_catalog",
    "normalize_job_lead",
    "normalized_ranking_policy",
    "stable_lead_id",
    "write_lead_catalog",
]
