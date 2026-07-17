use std::collections::{BTreeMap, BTreeSet};

use canisend_contracts::{
    DiscoveryBatch, DiscoveryImportDiagnostic, DiscoveryImportReport, DiscoveryLeadCandidate,
    DiscoveryMetadataValue, DiscoverySourceKind, UtcTimestamp,
};
use csv::{ReaderBuilder, StringRecord, Trim};
use time::{Date, format_description};
use url::Url;

use crate::IoAdapterError;

pub const MAX_DISCOVERY_BATCH_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_DISCOVERY_LEADS: usize = 1_000;
const MAX_LABEL_BYTES: usize = 300;
const MAX_URL_BYTES: usize = 4_096;
const MAX_SUMMARY_BYTES: usize = 8 * 1024;
const MAX_METADATA_ENTRIES: usize = 32;
const MAX_METADATA_KEY_BYTES: usize = 64;
const MAX_METADATA_VALUE_BYTES: usize = 1_024;
const MAX_METADATA_BYTES: usize = 16 * 1024;

const REQUIRED_HEADERS: [&str; 3] = ["title", "organization", "url"];
const OPTIONAL_HEADERS: [&str; 4] = ["external_id", "location", "deadline", "summary"];

pub fn parse_csv_batch(
    bytes: &[u8],
    source_name: &str,
    source_url: Option<&str>,
    observed_at: UtcTimestamp,
) -> Result<DiscoveryImportReport, IoAdapterError> {
    check_batch_size(bytes)?;
    validate_source_label(source_name)?;
    if let Some(source_url) = source_url {
        validate_url(source_url)?;
    }
    let mut reader = ReaderBuilder::new()
        .trim(Trim::All)
        .flexible(false)
        .from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|error| IoAdapterError::DiscoveryInput(format!("CSV header: {error}")))?
        .clone();
    let header_map = validate_headers(&headers)?;
    let mut accepted = Vec::new();
    let mut diagnostics = Vec::new();
    for (index, result) in reader.records().take(MAX_DISCOVERY_LEADS + 1).enumerate() {
        let row = u64::try_from(index + 2).expect("bounded CSV row fits u64");
        if index == MAX_DISCOVERY_LEADS {
            return Err(IoAdapterError::DiscoveryInput(format!(
                "batch exceeds {MAX_DISCOVERY_LEADS} leads"
            )));
        }
        match result {
            Ok(record) => {
                let candidate = csv_candidate(&headers, &header_map, &record);
                match normalize_candidate(candidate) {
                    Ok(candidate) => accepted.push(candidate),
                    Err((code, message)) => diagnostics.push(DiscoveryImportDiagnostic {
                        row,
                        code: code.to_owned(),
                        message,
                    }),
                }
            }
            Err(error) => diagnostics.push(DiscoveryImportDiagnostic {
                row,
                code: "csv.row_invalid".to_owned(),
                message: error.to_string(),
            }),
        }
    }
    Ok(report(
        DiscoverySourceKind::Csv,
        source_name,
        source_url,
        observed_at,
        accepted,
        diagnostics,
    ))
}

pub fn parse_json_batch(bytes: &[u8]) -> Result<DiscoveryImportReport, IoAdapterError> {
    parse_json_batch_as(bytes, None)
}

pub fn parse_host_agent_batch(bytes: &[u8]) -> Result<DiscoveryImportReport, IoAdapterError> {
    parse_json_batch_as(bytes, Some(DiscoverySourceKind::HostAgent))
}

fn parse_json_batch_as(
    bytes: &[u8],
    expected_kind: Option<DiscoverySourceKind>,
) -> Result<DiscoveryImportReport, IoAdapterError> {
    check_batch_size(bytes)?;
    let mut batch: DiscoveryBatch = serde_json::from_slice(bytes)
        .map_err(|error| IoAdapterError::DiscoveryInput(format!("JSON batch: {error}")))?;
    if let Some(expected) = expected_kind
        && batch.source_kind != expected
    {
        return Err(IoAdapterError::DiscoveryInput(format!(
            "expected source_kind {}, found {}",
            enum_name(expected),
            enum_name(batch.source_kind)
        )));
    }
    validate_source_label(&batch.source_name)?;
    if let Some(source_url) = &batch.source_url {
        validate_url(source_url)?;
    }
    if batch.leads.len() > MAX_DISCOVERY_LEADS {
        return Err(IoAdapterError::DiscoveryInput(format!(
            "batch exceeds {MAX_DISCOVERY_LEADS} leads"
        )));
    }
    let mut accepted = Vec::with_capacity(batch.leads.len());
    let mut diagnostics = Vec::new();
    for (index, candidate) in batch.leads.drain(..).enumerate() {
        match normalize_candidate(candidate) {
            Ok(candidate) => accepted.push(candidate),
            Err((code, message)) => diagnostics.push(DiscoveryImportDiagnostic {
                row: u64::try_from(index + 1).expect("bounded JSON row fits u64"),
                code: code.to_owned(),
                message,
            }),
        }
    }
    batch.leads = accepted;
    let accepted = u64::try_from(batch.leads.len()).expect("bounded lead count fits u64");
    let rejected = u64::try_from(diagnostics.len()).expect("bounded diagnostics fit u64");
    Ok(DiscoveryImportReport {
        dry_run: true,
        accepted,
        rejected,
        diagnostics,
        batch: Some(batch),
        receipt: None,
    })
}

fn report(
    source_kind: DiscoverySourceKind,
    source_name: &str,
    source_url: Option<&str>,
    observed_at: UtcTimestamp,
    leads: Vec<DiscoveryLeadCandidate>,
    diagnostics: Vec<DiscoveryImportDiagnostic>,
) -> DiscoveryImportReport {
    let accepted = u64::try_from(leads.len()).expect("bounded lead count fits u64");
    let rejected = u64::try_from(diagnostics.len()).expect("bounded diagnostics fit u64");
    DiscoveryImportReport {
        dry_run: true,
        accepted,
        rejected,
        diagnostics,
        batch: Some(DiscoveryBatch {
            source_kind,
            source_name: source_name.trim().to_owned(),
            source_url: source_url.map(str::to_owned),
            cursor: None,
            observed_at,
            leads,
        }),
        receipt: None,
    }
}

fn validate_headers(headers: &StringRecord) -> Result<BTreeMap<String, usize>, IoAdapterError> {
    let mut positions = BTreeMap::new();
    for (index, header) in headers.iter().enumerate() {
        let name = header.trim().to_ascii_lowercase();
        let known = REQUIRED_HEADERS.contains(&name.as_str())
            || OPTIONAL_HEADERS.contains(&name.as_str())
            || valid_metadata_header(&name);
        if !known {
            return Err(IoAdapterError::DiscoveryInput(format!(
                "unknown CSV header: {header}"
            )));
        }
        if positions.insert(name.clone(), index).is_some() {
            return Err(IoAdapterError::DiscoveryInput(format!(
                "duplicate CSV header: {name}"
            )));
        }
    }
    for required in REQUIRED_HEADERS {
        if !positions.contains_key(required) {
            return Err(IoAdapterError::DiscoveryInput(format!(
                "missing required CSV header: {required}"
            )));
        }
    }
    Ok(positions)
}

fn valid_metadata_header(value: &str) -> bool {
    value.strip_prefix("meta.").is_some_and(valid_metadata_key)
}

fn csv_candidate(
    headers: &StringRecord,
    positions: &BTreeMap<String, usize>,
    record: &StringRecord,
) -> DiscoveryLeadCandidate {
    let field = |name: &str| {
        positions
            .get(name)
            .and_then(|index| record.get(*index))
            .unwrap_or_default()
            .trim()
    };
    let optional = |name: &str| {
        let value = field(name);
        (!value.is_empty()).then(|| value.to_owned())
    };
    let metadata = headers
        .iter()
        .enumerate()
        .filter_map(|(index, header)| {
            let key = header.trim().to_ascii_lowercase();
            let key = key.strip_prefix("meta.")?;
            let value = record.get(index).unwrap_or_default().trim();
            (!value.is_empty()).then(|| {
                (
                    key.to_owned(),
                    DiscoveryMetadataValue::Text(value.to_owned()),
                )
            })
        })
        .collect();
    DiscoveryLeadCandidate {
        external_id: optional("external_id"),
        title: field("title").to_owned(),
        organization: field("organization").to_owned(),
        location: optional("location"),
        deadline: optional("deadline"),
        url: field("url").to_owned(),
        summary: optional("summary"),
        metadata,
    }
}

fn normalize_candidate(
    mut candidate: DiscoveryLeadCandidate,
) -> Result<DiscoveryLeadCandidate, (&'static str, String)> {
    candidate.title = normalize_label("title", &candidate.title)?;
    candidate.organization = normalize_label("organization", &candidate.organization)?;
    candidate.location = normalize_optional_label("location", candidate.location)?;
    candidate.external_id = normalize_optional_label("external_id", candidate.external_id)?;
    candidate.summary = normalize_optional_summary(candidate.summary)?;
    candidate.url =
        validate_url(&candidate.url).map_err(|error| ("lead.url_invalid", error.to_string()))?;
    if let Some(deadline) = &candidate.deadline {
        let format = format_description::parse_borrowed::<3>("[year]-[month]-[day]")
            .expect("static deadline format is valid");
        Date::parse(deadline, &format).map_err(|_| {
            (
                "lead.deadline_invalid",
                "deadline must be a real calendar date in YYYY-MM-DD form".to_owned(),
            )
        })?;
    }
    validate_metadata(&candidate.metadata)?;
    Ok(candidate)
}

fn normalize_label(name: &'static str, value: &str) -> Result<String, (&'static str, String)> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty()
        || normalized.len() > MAX_LABEL_BYTES
        || normalized.chars().any(forbidden_control)
    {
        return Err((
            "lead.field_invalid",
            format!("{name} must contain between 1 and {MAX_LABEL_BYTES} safe UTF-8 bytes"),
        ));
    }
    Ok(normalized)
}

fn normalize_optional_label(
    name: &'static str,
    value: Option<String>,
) -> Result<Option<String>, (&'static str, String)> {
    value.map(|value| normalize_label(name, &value)).transpose()
}

fn normalize_optional_summary(
    value: Option<String>,
) -> Result<Option<String>, (&'static str, String)> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Ok(None);
    }
    if normalized.len() > MAX_SUMMARY_BYTES || normalized.chars().any(forbidden_control) {
        return Err((
            "lead.summary_invalid",
            format!("summary exceeds {MAX_SUMMARY_BYTES} safe UTF-8 bytes"),
        ));
    }
    Ok(Some(normalized))
}

fn validate_url(value: &str) -> Result<String, IoAdapterError> {
    if value.len() > MAX_URL_BYTES {
        return Err(IoAdapterError::DiscoveryInput("URL is too long".to_owned()));
    }
    let mut url = Url::parse(value)
        .map_err(|error| IoAdapterError::DiscoveryInput(format!("URL: {error}")))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(IoAdapterError::DiscoveryInput(
            "URL must use http or https".to_owned(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() || url.host_str().is_none() {
        return Err(IoAdapterError::DiscoveryInput(
            "URL must have a host and cannot contain credentials".to_owned(),
        ));
    }
    url.set_fragment(None);
    Ok(url.to_string())
}

fn validate_source_label(value: &str) -> Result<(), IoAdapterError> {
    normalize_label("source_name", value)
        .map(|_| ())
        .map_err(|(_, message)| IoAdapterError::DiscoveryInput(message))
}

fn validate_metadata(
    metadata: &BTreeMap<String, DiscoveryMetadataValue>,
) -> Result<(), (&'static str, String)> {
    if metadata.len() > MAX_METADATA_ENTRIES {
        return Err((
            "lead.metadata_invalid",
            format!("metadata exceeds {MAX_METADATA_ENTRIES} entries"),
        ));
    }
    let mut total = 0_usize;
    let mut normalized_keys = BTreeSet::new();
    for (key, value) in metadata {
        if !valid_metadata_key(key) || !normalized_keys.insert(key.to_ascii_lowercase()) {
            return Err((
                "lead.metadata_invalid",
                format!("metadata key is invalid or duplicated: {key}"),
            ));
        }
        let value_length = match value {
            DiscoveryMetadataValue::Text(value) => {
                if value.chars().any(forbidden_control) {
                    return Err((
                        "lead.metadata_invalid",
                        format!("metadata value contains controls: {key}"),
                    ));
                }
                value.len()
            }
            DiscoveryMetadataValue::Integer(value) => value.to_string().len(),
            DiscoveryMetadataValue::Boolean(value) => value.to_string().len(),
        };
        if value_length > MAX_METADATA_VALUE_BYTES {
            return Err((
                "lead.metadata_invalid",
                format!("metadata value is too long: {key}"),
            ));
        }
        total = total.saturating_add(key.len()).saturating_add(value_length);
    }
    if total > MAX_METADATA_BYTES {
        return Err((
            "lead.metadata_invalid",
            format!("metadata exceeds {MAX_METADATA_BYTES} bytes"),
        ));
    }
    Ok(())
}

fn valid_metadata_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_METADATA_KEY_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn forbidden_control(character: char) -> bool {
    character.is_control() && !matches!(character, '\n' | '\r' | '\t')
}

fn check_batch_size(bytes: &[u8]) -> Result<(), IoAdapterError> {
    if bytes.is_empty() {
        return Err(IoAdapterError::DiscoveryInput("batch is empty".to_owned()));
    }
    if bytes.len() > MAX_DISCOVERY_BATCH_BYTES {
        return Err(IoAdapterError::InputTooLarge {
            limit: u64::try_from(MAX_DISCOVERY_BATCH_BYTES).expect("batch limit fits u64"),
        });
    }
    Ok(())
}

fn enum_name(kind: DiscoverySourceKind) -> &'static str {
    match kind {
        DiscoverySourceKind::Csv => "csv",
        DiscoverySourceKind::Json => "json",
        DiscoverySourceKind::HostAgent => "host-agent",
        DiscoverySourceKind::RssAtom => "rss-atom",
        DiscoverySourceKind::JobsAcUk => "jobs-ac-uk",
        DiscoverySourceKind::Greenhouse => "greenhouse",
        DiscoverySourceKind::Lever => "lever",
    }
}

#[cfg(test)]
mod tests {
    use canisend_contracts::{DiscoverySourceKind, UtcTimestamp};

    use super::{parse_csv_batch, parse_host_agent_batch, parse_json_batch};

    fn timestamp() -> UtcTimestamp {
        UtcTimestamp::try_new("2026-07-17T10:00:00Z").expect("timestamp")
    }

    #[test]
    fn csv_import_has_explicit_headers_normalization_and_row_diagnostics() {
        let csv = b"title,organization,url,deadline,meta.rank\r\n  Lecturer   in Economics , University X ,https://example.edu/jobs/1#apply,2026-08-31,1\r\nBad,University X,ftp://example.edu/2,not-a-date,2\r\n";
        let report = parse_csv_batch(csv, "University feed", None, timestamp()).expect("report");
        assert_eq!(report.accepted, 1);
        assert_eq!(report.rejected, 1);
        assert_eq!(report.diagnostics[0].row, 3);
        let batch = report.batch.expect("batch");
        assert_eq!(batch.leads[0].title, "Lecturer in Economics");
        assert_eq!(batch.leads[0].url, "https://example.edu/jobs/1");
    }

    #[test]
    fn csv_rejects_implicit_and_duplicate_mapping() {
        assert!(
            parse_csv_batch(b"title,organization\nRole,Org\n", "feed", None, timestamp()).is_err()
        );
        assert!(
            parse_csv_batch(
                b"title,organization,url,title\nRole,Org,https://example.test,Again\n",
                "feed",
                None,
                timestamp()
            )
            .is_err()
        );
    }

    #[test]
    fn json_and_host_agent_share_the_normalized_batch_contract() {
        let json = br#"{
          "source_kind":"json",
          "source_name":"Export",
          "source_url":null,
          "cursor":null,
          "observed_at":"2026-07-17T10:00:00Z",
          "leads":[{
            "external_id":"  42  ",
            "title":" Research   Fellow ",
            "organization":"University X",
            "location":null,
            "deadline":"2026-09-01",
            "url":"https://example.edu/42#details",
            "summary":null,
            "metadata":{"remote":true,"rank":2}
          }]
        }"#;
        let report = parse_json_batch(json).expect("JSON report");
        assert_eq!(report.accepted, 1);
        assert_eq!(
            report.batch.expect("batch").leads[0].title,
            "Research Fellow"
        );
        assert!(parse_host_agent_batch(json).is_err());

        let host = String::from_utf8(json.to_vec())
            .expect("UTF-8")
            .replace("\"json\"", "\"host-agent\"");
        let host_report = parse_host_agent_batch(host.as_bytes()).expect("host report");
        assert_eq!(
            host_report.batch.expect("batch").source_kind,
            DiscoverySourceKind::HostAgent
        );
    }
}
