use std::collections::BTreeMap;

use canisend_contracts::{
    DiscoveryAdapterCapabilities, DiscoveryImportDiagnostic, DiscoveryImportReport,
    DiscoveryLeadCandidate, DiscoveryMetadataValue, DiscoverySourceKind, UtcTimestamp,
};
use feed_rs::model::Entry;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

use super::{AdapterReportInput, MAX_DISCOVERY_LEADS, adapter_report, normalize_adapter_candidate};
use crate::{HttpFetcher, IoAdapterError, RemotePayloadKind};

pub trait DiscoveryAdapter {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> DiscoveryAdapterCapabilities;
    fn payload_kind(&self) -> RemotePayloadKind;
    fn parse(
        &self,
        bytes: &[u8],
        source_url: &str,
        observed_at: UtcTimestamp,
    ) -> Result<DiscoveryImportReport, IoAdapterError>;

    fn refresh(
        &self,
        fetcher: &HttpFetcher,
        source_url: &str,
        observed_at: UtcTimestamp,
    ) -> Result<DiscoveryImportReport, IoAdapterError> {
        self.validate_endpoint(source_url)?;
        let payload = fetcher.fetch_discovery(source_url)?;
        if payload.kind != self.payload_kind() {
            return Err(IoAdapterError::UnsupportedContentType(format!(
                "{} expects {:?}, received {:?}",
                self.id(),
                self.payload_kind(),
                payload.kind
            )));
        }
        self.parse(&payload.bytes, &payload.source_url, observed_at)
    }

    fn validate_endpoint(&self, source_url: &str) -> Result<(), IoAdapterError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RssAtomAdapter {
    source_name: String,
    organization_fallback: Option<String>,
}

impl RssAtomAdapter {
    #[must_use]
    pub fn new(source_name: impl Into<String>, organization_fallback: Option<String>) -> Self {
        Self {
            source_name: source_name.into(),
            organization_fallback,
        }
    }
}

impl DiscoveryAdapter for RssAtomAdapter {
    fn id(&self) -> &'static str {
        "rss-atom"
    }

    fn capabilities(&self) -> DiscoveryAdapterCapabilities {
        capabilities(DiscoverySourceKind::RssAtom)
    }

    fn payload_kind(&self) -> RemotePayloadKind {
        RemotePayloadKind::Xml
    }

    fn parse(
        &self,
        bytes: &[u8],
        source_url: &str,
        observed_at: UtcTimestamp,
    ) -> Result<DiscoveryImportReport, IoAdapterError> {
        parse_feed(
            bytes,
            source_url,
            observed_at,
            FeedOptions {
                kind: DiscoverySourceKind::RssAtom,
                source_name: &self.source_name,
                organization_fallback: self.organization_fallback.as_deref(),
                jobs_ac_uk_title: false,
            },
        )
    }

    fn validate_endpoint(&self, source_url: &str) -> Result<(), IoAdapterError> {
        validate_https_or_http(source_url).map(|_| ())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobsAcUkAdapter {
    organization_fallback: Option<String>,
}

impl JobsAcUkAdapter {
    #[must_use]
    pub fn new(organization_fallback: Option<String>) -> Self {
        Self {
            organization_fallback,
        }
    }
}

impl DiscoveryAdapter for JobsAcUkAdapter {
    fn id(&self) -> &'static str {
        "jobs-ac-uk"
    }

    fn capabilities(&self) -> DiscoveryAdapterCapabilities {
        capabilities(DiscoverySourceKind::JobsAcUk)
    }

    fn payload_kind(&self) -> RemotePayloadKind {
        RemotePayloadKind::Xml
    }

    fn parse(
        &self,
        bytes: &[u8],
        source_url: &str,
        observed_at: UtcTimestamp,
    ) -> Result<DiscoveryImportReport, IoAdapterError> {
        self.validate_endpoint(source_url)?;
        parse_feed(
            bytes,
            source_url,
            observed_at,
            FeedOptions {
                kind: DiscoverySourceKind::JobsAcUk,
                source_name: "jobs.ac.uk",
                organization_fallback: self.organization_fallback.as_deref(),
                jobs_ac_uk_title: true,
            },
        )
    }

    fn validate_endpoint(&self, source_url: &str) -> Result<(), IoAdapterError> {
        validate_provider_endpoint(source_url, &["jobs.ac.uk", "www.jobs.ac.uk"]).map(|_| ())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreenhouseAdapter {
    organization: String,
}

impl GreenhouseAdapter {
    #[must_use]
    pub fn new(organization: impl Into<String>) -> Self {
        Self {
            organization: organization.into(),
        }
    }
}

impl DiscoveryAdapter for GreenhouseAdapter {
    fn id(&self) -> &'static str {
        "greenhouse"
    }

    fn capabilities(&self) -> DiscoveryAdapterCapabilities {
        capabilities(DiscoverySourceKind::Greenhouse)
    }

    fn payload_kind(&self) -> RemotePayloadKind {
        RemotePayloadKind::Json
    }

    fn parse(
        &self,
        bytes: &[u8],
        source_url: &str,
        observed_at: UtcTimestamp,
    ) -> Result<DiscoveryImportReport, IoAdapterError> {
        self.validate_endpoint(source_url)?;
        let response: GreenhouseResponse = serde_json::from_slice(bytes)
            .map_err(|error| IoAdapterError::DiscoveryInput(format!("Greenhouse JSON: {error}")))?;
        if response.jobs.len() > MAX_DISCOVERY_LEADS {
            return Err(too_many_leads());
        }
        let mut leads = Vec::new();
        let mut diagnostics = Vec::new();
        for (index, job) in response.jobs.into_iter().enumerate() {
            let row = row_number(index);
            let candidate = greenhouse_candidate(job, &self.organization);
            push_candidate(candidate, row, &mut leads, &mut diagnostics);
        }
        adapter_report(AdapterReportInput {
            source_kind: DiscoverySourceKind::Greenhouse,
            source_name: self.organization.clone(),
            source_url: source_url.to_owned(),
            cursor: Some(payload_digest(bytes)),
            observed_at,
            leads,
            diagnostics,
        })
    }

    fn validate_endpoint(&self, source_url: &str) -> Result<(), IoAdapterError> {
        let url = validate_provider_endpoint(source_url, &["boards-api.greenhouse.io"])?;
        if !url.path().starts_with("/v1/boards/") || !url.path().ends_with("/jobs") {
            return Err(IoAdapterError::UrlPolicy(
                "Greenhouse endpoint must be /v1/boards/{board_token}/jobs".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeverAdapter {
    organization: String,
}

impl LeverAdapter {
    #[must_use]
    pub fn new(organization: impl Into<String>) -> Self {
        Self {
            organization: organization.into(),
        }
    }
}

impl DiscoveryAdapter for LeverAdapter {
    fn id(&self) -> &'static str {
        "lever"
    }

    fn capabilities(&self) -> DiscoveryAdapterCapabilities {
        capabilities(DiscoverySourceKind::Lever)
    }

    fn payload_kind(&self) -> RemotePayloadKind {
        RemotePayloadKind::Json
    }

    fn parse(
        &self,
        bytes: &[u8],
        source_url: &str,
        observed_at: UtcTimestamp,
    ) -> Result<DiscoveryImportReport, IoAdapterError> {
        self.validate_endpoint(source_url)?;
        let postings: Vec<LeverPosting> = serde_json::from_slice(bytes)
            .map_err(|error| IoAdapterError::DiscoveryInput(format!("Lever JSON: {error}")))?;
        if postings.len() > MAX_DISCOVERY_LEADS {
            return Err(too_many_leads());
        }
        let mut leads = Vec::new();
        let mut diagnostics = Vec::new();
        for (index, posting) in postings.into_iter().enumerate() {
            let row = row_number(index);
            let candidate = lever_candidate(posting, &self.organization);
            push_candidate(candidate, row, &mut leads, &mut diagnostics);
        }
        adapter_report(AdapterReportInput {
            source_kind: DiscoverySourceKind::Lever,
            source_name: self.organization.clone(),
            source_url: source_url.to_owned(),
            cursor: Some(payload_digest(bytes)),
            observed_at,
            leads,
            diagnostics,
        })
    }

    fn validate_endpoint(&self, source_url: &str) -> Result<(), IoAdapterError> {
        let url = validate_provider_endpoint(source_url, &["api.lever.co", "api.eu.lever.co"])?;
        if !url.path().starts_with("/v0/postings/") {
            return Err(IoAdapterError::UrlPolicy(
                "Lever endpoint must be /v0/postings/{site}".to_owned(),
            ));
        }
        let mode_is_json = url
            .query_pairs()
            .any(|(key, value)| key == "mode" && value.eq_ignore_ascii_case("json"));
        if !mode_is_json {
            return Err(IoAdapterError::UrlPolicy(
                "Lever endpoint must include mode=json".to_owned(),
            ));
        }
        Ok(())
    }
}

struct FeedOptions<'a> {
    kind: DiscoverySourceKind,
    source_name: &'a str,
    organization_fallback: Option<&'a str>,
    jobs_ac_uk_title: bool,
}

fn parse_feed(
    bytes: &[u8],
    source_url: &str,
    observed_at: UtcTimestamp,
    options: FeedOptions<'_>,
) -> Result<DiscoveryImportReport, IoAdapterError> {
    let feed = feed_rs::parser::parse(bytes)
        .map_err(|error| IoAdapterError::DiscoveryInput(format!("RSS/Atom: {error}")))?;
    if feed.entries.len() > MAX_DISCOVERY_LEADS {
        return Err(too_many_leads());
    }
    let mut leads = Vec::new();
    let mut diagnostics = Vec::new();
    for (index, entry) in feed.entries.into_iter().enumerate() {
        let row = row_number(index);
        let candidate = feed_candidate(entry, &options);
        push_candidate(candidate, row, &mut leads, &mut diagnostics);
    }
    adapter_report(AdapterReportInput {
        source_kind: options.kind,
        source_name: options.source_name.to_owned(),
        source_url: source_url.to_owned(),
        cursor: Some(payload_digest(bytes)),
        observed_at,
        leads,
        diagnostics,
    })
}

fn feed_candidate(
    entry: Entry,
    options: &FeedOptions<'_>,
) -> Result<DiscoveryLeadCandidate, String> {
    let raw_title = entry
        .title
        .as_ref()
        .map(|title| title.content.trim())
        .filter(|title| !title.is_empty())
        .ok_or_else(|| "feed entry has no title".to_owned())?;
    let (title, title_organization) = if options.jobs_ac_uk_title {
        split_jobs_ac_uk_title(raw_title)
    } else {
        (raw_title.to_owned(), None)
    };
    let organization = entry
        .authors
        .first()
        .map(|author| author.name.trim())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .or(title_organization)
        .or_else(|| options.organization_fallback.map(str::to_owned))
        .ok_or_else(|| {
            "feed entry has no organization; configure an explicit organization fallback".to_owned()
        })?;
    let url = entry
        .links
        .iter()
        .find(|link| link.rel.as_deref().is_none_or(|rel| rel == "alternate"))
        .or_else(|| entry.links.first())
        .map(|link| link.href.clone())
        .ok_or_else(|| "feed entry has no job URL".to_owned())?;
    let mut metadata = BTreeMap::new();
    if let Some(published) = entry.published {
        metadata.insert(
            "feed.published".to_owned(),
            DiscoveryMetadataValue::Text(published.to_rfc3339()),
        );
    }
    if let Some(updated) = entry.updated {
        metadata.insert(
            "feed.updated".to_owned(),
            DiscoveryMetadataValue::Text(updated.to_rfc3339()),
        );
    }
    if !entry.categories.is_empty() {
        metadata.insert(
            "feed.categories".to_owned(),
            DiscoveryMetadataValue::Json(Value::Array(
                entry
                    .categories
                    .iter()
                    .map(|category| Value::String(category.term.clone()))
                    .collect(),
            )),
        );
    }
    let summary = entry
        .summary
        .as_ref()
        .map(|summary| summary.content.trim().to_owned())
        .filter(|summary| !summary.is_empty() && summary.len() <= 8 * 1024);
    Ok(DiscoveryLeadCandidate {
        external_id: (!entry.id.trim().is_empty()).then(|| entry.id.trim().to_owned()),
        title,
        organization,
        location: None,
        deadline: None,
        url,
        summary,
        metadata,
    })
}

fn split_jobs_ac_uk_title(title: &str) -> (String, Option<String>) {
    title
        .rsplit_once(" at ")
        .filter(|(role, organization)| !role.trim().is_empty() && !organization.trim().is_empty())
        .map_or_else(
            || (title.to_owned(), None),
            |(role, organization)| (role.trim().to_owned(), Some(organization.trim().to_owned())),
        )
}

#[derive(Debug, Deserialize)]
struct GreenhouseResponse {
    jobs: Vec<GreenhouseJob>,
}

#[derive(Debug, Deserialize)]
struct GreenhouseJob {
    id: Value,
    title: Option<String>,
    location: Option<GreenhouseLocation>,
    absolute_url: Option<String>,
    updated_at: Option<String>,
    requisition_id: Option<Value>,
    language: Option<String>,
    metadata: Option<Value>,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GreenhouseLocation {
    name: Option<String>,
}

fn greenhouse_candidate(
    job: GreenhouseJob,
    organization: &str,
) -> Result<DiscoveryLeadCandidate, String> {
    let mut metadata = BTreeMap::new();
    insert_json_metadata(
        &mut metadata,
        "greenhouse.requisition_id",
        job.requisition_id,
    );
    insert_text_metadata(&mut metadata, "greenhouse.updated_at", job.updated_at);
    insert_text_metadata(&mut metadata, "greenhouse.language", job.language);
    insert_json_metadata(&mut metadata, "greenhouse.metadata", job.metadata);
    if job.content.is_some() {
        metadata.insert(
            "greenhouse.content_available".to_owned(),
            DiscoveryMetadataValue::Boolean(true),
        );
    }
    Ok(DiscoveryLeadCandidate {
        external_id: Some(scalar_id(&job.id)?),
        title: job
            .title
            .ok_or_else(|| "Greenhouse job has no title".to_owned())?,
        organization: organization.to_owned(),
        location: job.location.and_then(|location| location.name),
        deadline: None,
        url: job
            .absolute_url
            .ok_or_else(|| "Greenhouse job has no absolute_url".to_owned())?,
        summary: None,
        metadata,
    })
}

#[derive(Debug, Deserialize)]
struct LeverPosting {
    id: Option<String>,
    text: Option<String>,
    categories: Option<Value>,
    #[serde(rename = "hostedUrl")]
    hosted_url: Option<String>,
    #[serde(rename = "openingPlain")]
    opening_plain: Option<String>,
    #[serde(rename = "workplaceType")]
    workplace_type: Option<String>,
    country: Option<String>,
}

fn lever_candidate(
    posting: LeverPosting,
    organization: &str,
) -> Result<DiscoveryLeadCandidate, String> {
    let location = posting
        .categories
        .as_ref()
        .and_then(|categories| categories.get("location"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let mut metadata = BTreeMap::new();
    insert_json_metadata(&mut metadata, "lever.categories", posting.categories);
    insert_text_metadata(
        &mut metadata,
        "lever.workplace_type",
        posting.workplace_type,
    );
    insert_text_metadata(&mut metadata, "lever.country", posting.country);
    let summary = posting
        .opening_plain
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty() && value.len() <= 8 * 1024);
    Ok(DiscoveryLeadCandidate {
        external_id: posting.id,
        title: posting
            .text
            .ok_or_else(|| "Lever posting has no text title".to_owned())?,
        organization: organization.to_owned(),
        location,
        deadline: None,
        url: posting
            .hosted_url
            .ok_or_else(|| "Lever posting has no hostedUrl".to_owned())?,
        summary,
        metadata,
    })
}

fn push_candidate(
    candidate: Result<DiscoveryLeadCandidate, String>,
    row: u64,
    leads: &mut Vec<DiscoveryLeadCandidate>,
    diagnostics: &mut Vec<DiscoveryImportDiagnostic>,
) {
    match candidate.and_then(|candidate| {
        normalize_adapter_candidate(candidate).map_err(|(_, message)| message)
    }) {
        Ok(candidate) => leads.push(candidate),
        Err(message) => diagnostics.push(DiscoveryImportDiagnostic {
            row,
            code: "adapter.row_invalid".to_owned(),
            message,
        }),
    }
}

fn insert_text_metadata(
    metadata: &mut BTreeMap<String, DiscoveryMetadataValue>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        metadata.insert(
            key.to_owned(),
            DiscoveryMetadataValue::Text(value.trim().to_owned()),
        );
    }
}

fn insert_json_metadata(
    metadata: &mut BTreeMap<String, DiscoveryMetadataValue>,
    key: &str,
    value: Option<Value>,
) {
    if let Some(value) = value.filter(|value| !value.is_null()) {
        metadata.insert(key.to_owned(), DiscoveryMetadataValue::Json(value));
    }
}

fn scalar_id(value: &Value) -> Result<String, String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        _ => Err("provider job ID must be a string or number".to_owned()),
    }
}

fn capabilities(kind: DiscoverySourceKind) -> DiscoveryAdapterCapabilities {
    DiscoveryAdapterCapabilities {
        kind,
        network: true,
        supports_cursor: true,
        preserves_removed: true,
        max_items_per_refresh: u32::try_from(MAX_DISCOVERY_LEADS)
            .expect("discovery limit fits u32"),
    }
}

fn validate_https_or_http(source_url: &str) -> Result<Url, IoAdapterError> {
    let url =
        Url::parse(source_url).map_err(|error| IoAdapterError::InvalidUrl(error.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(IoAdapterError::UrlPolicy(
            "discovery endpoint must be an absolute HTTP(S) URL".to_owned(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(IoAdapterError::UrlPolicy(
            "discovery endpoint cannot contain credentials".to_owned(),
        ));
    }
    Ok(url)
}

fn validate_provider_endpoint(
    source_url: &str,
    allowed_hosts: &[&str],
) -> Result<Url, IoAdapterError> {
    let url = validate_https_or_http(source_url)?;
    if url.scheme() != "https" {
        return Err(IoAdapterError::UrlPolicy(
            "provider discovery endpoints require HTTPS".to_owned(),
        ));
    }
    let host = url.host_str().unwrap_or_default();
    if !allowed_hosts
        .iter()
        .any(|allowed| host.eq_ignore_ascii_case(allowed))
    {
        return Err(IoAdapterError::UrlPolicy(format!(
            "provider endpoint host is not allowed: {host}"
        )));
    }
    Ok(url)
}

fn payload_digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn row_number(index: usize) -> u64 {
    u64::try_from(index + 1).expect("bounded adapter row fits u64")
}

fn too_many_leads() -> IoAdapterError {
    IoAdapterError::DiscoveryInput(format!(
        "adapter response exceeds {MAX_DISCOVERY_LEADS} leads"
    ))
}

#[cfg(test)]
mod tests {
    use canisend_contracts::{DiscoverySourceKind, UtcTimestamp};

    use super::{
        DiscoveryAdapter, GreenhouseAdapter, JobsAcUkAdapter, LeverAdapter, RssAtomAdapter,
    };

    fn observed_at() -> UtcTimestamp {
        UtcTimestamp::try_new("2026-07-17T10:00:00Z").expect("timestamp")
    }

    #[test]
    fn rss_and_atom_fixtures_normalize_and_diagnose_entries_offline() {
        let rss = include_bytes!("../../../../fixtures/v2-spec/discovery/rss.xml");
        let report = RssAtomAdapter::new("Academic jobs", None)
            .parse(rss, "https://feed.example/jobs.xml", observed_at())
            .expect("RSS report");
        assert_eq!(report.accepted, 1);
        assert_eq!(report.rejected, 1);
        assert_eq!(
            report.batch.expect("batch").source_kind,
            DiscoverySourceKind::RssAtom
        );

        let atom = include_bytes!("../../../../fixtures/v2-spec/discovery/atom.xml");
        let atom_report = RssAtomAdapter::new("Atom jobs", None)
            .parse(atom, "https://feed.example/atom.xml", observed_at())
            .expect("Atom report");
        assert_eq!(atom_report.accepted, 1);
    }

    #[test]
    fn jobs_ac_uk_fixture_extracts_organization_from_title() {
        let rss = include_bytes!("../../../../fixtures/v2-spec/discovery/jobs-ac-uk.xml");
        let report = JobsAcUkAdapter::new(None)
            .parse(
                rss,
                "https://www.jobs.ac.uk/rss/economics.xml",
                observed_at(),
            )
            .expect("jobs.ac.uk report");
        let lead = &report.batch.expect("batch").leads[0];
        assert_eq!(lead.title, "Lecturer in Economics");
        assert_eq!(lead.organization, "University X");
    }

    #[test]
    fn greenhouse_fixture_preserves_bounded_provider_metadata() {
        let json = include_bytes!("../../../../fixtures/v2-spec/discovery/greenhouse.json");
        let report = GreenhouseAdapter::new("University X")
            .parse(
                json,
                "https://boards-api.greenhouse.io/v1/boards/example/jobs",
                observed_at(),
            )
            .expect("Greenhouse report");
        assert_eq!(report.accepted, 1);
        assert_eq!(report.rejected, 1);
        let lead = &report.batch.expect("batch").leads[0];
        assert_eq!(lead.external_id.as_deref(), Some("127817"));
        assert!(lead.metadata.contains_key("greenhouse.metadata"));
    }

    #[test]
    fn lever_fixture_uses_documented_public_posting_fields() {
        let json = include_bytes!("../../../../fixtures/v2-spec/discovery/lever.json");
        let adapter = LeverAdapter::new("University X");
        let report = adapter
            .parse(
                json,
                "https://api.lever.co/v0/postings/example?mode=json",
                observed_at(),
            )
            .expect("Lever report");
        assert_eq!(report.accepted, 1);
        assert_eq!(report.rejected, 1);
        assert_eq!(
            report.batch.expect("batch").leads[0].location.as_deref(),
            Some("London")
        );
        assert!(
            adapter
                .validate_endpoint("https://evil.example/v0/postings/example?mode=json")
                .is_err()
        );
    }
}
