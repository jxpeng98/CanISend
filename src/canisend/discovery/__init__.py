"""Versioned, source-neutral job discovery contracts and normalization."""

from canisend.discovery.identity import (
    canonicalize_job_url,
    normalize_job_lead,
    stable_lead_id,
)
from canisend.discovery.models import (
    JOB_LEAD_SCHEMA_VERSION,
    JobLeadV2,
    LeadMatchReasonV1,
    LeadProvenanceV1,
)

__all__ = [
    "JOB_LEAD_SCHEMA_VERSION",
    "JobLeadV2",
    "LeadMatchReasonV1",
    "LeadProvenanceV1",
    "canonicalize_job_url",
    "normalize_job_lead",
    "stable_lead_id",
]
