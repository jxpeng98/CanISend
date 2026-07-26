use std::path::{Path, PathBuf};

use canisend_contracts::{
    ActorKind, DiscoveryAdapterCapabilities, DiscoveryImportReport, DiscoveryLeadRecord,
    DiscoveryLeadSuggestion, DiscoverySourceKind, DiscoverySourceRecord, JobRecord, NextAction,
};
use canisend_io::{
    DiscoveryAdapter, DiscoveryFileKind, GreenhouseAdapter, HttpFetcher, JobsAcUkAdapter,
    LeverAdapter, RssAtomAdapter, discovery_adapter_capabilities, parse_csv_batch,
    parse_host_agent_batch, parse_json_batch, read_discovery_file,
};
use canisend_store::{DiscoveryService, current_utc_timestamp};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError, NetworkFetchConsent, PrivateReadConsent,
    application::{open_workspace, parse_entity_id},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiscoveryNetworkAdapter {
    RssAtom,
    JobsAcUk,
    Greenhouse,
    Lever,
}

impl DiscoveryNetworkAdapter {
    #[must_use]
    pub const fn source_kind(self) -> DiscoverySourceKind {
        match self {
            Self::RssAtom => DiscoverySourceKind::RssAtom,
            Self::JobsAcUk => DiscoverySourceKind::JobsAcUk,
            Self::Greenhouse => DiscoverySourceKind::Greenhouse,
            Self::Lever => DiscoverySourceKind::Lever,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryImportRequest {
    pub path: PathBuf,
    pub source_name: Option<String>,
    pub source_url: Option<String>,
    pub host_agent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryRefreshRequest {
    pub adapter: DiscoveryNetworkAdapter,
    pub endpoint: String,
    pub source_name: String,
    pub organization: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryAdapterCatalogReadModel {
    pub adapters: Vec<DiscoveryAdapterCapabilities>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoverySourceListReadModel {
    pub workspace: PathBuf,
    pub sources: Vec<DiscoverySourceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryLeadListReadModel {
    pub workspace: PathBuf,
    pub include_history: bool,
    pub leads: Vec<DiscoveryLeadRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoverySuggestionReadModel {
    pub suggestions: Vec<DiscoveryLeadSuggestion>,
    pub automatic_merge: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryPromotionReadModel {
    pub job: JobRecord,
    pub lead_id: canisend_contracts::EntityId,
}

impl Application {
    pub fn discovery_adapters() -> ActionReceipt<DiscoveryAdapterCatalogReadModel> {
        let adapters = discovery_adapter_capabilities();
        ActionReceipt::new(
            "discovery.adapters",
            "available",
            format!("Loaded {} compiled discovery adapter(s)", adapters.len()),
            DiscoveryAdapterCatalogReadModel { adapters },
        )
    }

    pub fn preview_discovery_import(
        request: &DiscoveryImportRequest,
        _consent: PrivateReadConsent,
    ) -> Result<ActionReceipt<DiscoveryImportReport>, ApplicationError> {
        let document = read_discovery_file(&request.path)?;
        let report = match document.kind {
            DiscoveryFileKind::Csv => {
                if request.host_agent {
                    return Err(ApplicationError::InvalidInput(
                        "--host-agent requires a JSON batch".to_owned(),
                    ));
                }
                let source_name = request.source_name.clone().unwrap_or_else(|| {
                    document
                        .path
                        .file_stem()
                        .map(|value| value.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "CSV import".to_owned())
                });
                parse_csv_batch(
                    &document.bytes,
                    &source_name,
                    request.source_url.as_deref(),
                    current_utc_timestamp()?,
                )?
            }
            DiscoveryFileKind::Json => {
                if request.source_name.is_some() || request.source_url.is_some() {
                    return Err(ApplicationError::InvalidInput(
                        "JSON batches declare source_name and source_url inside the versioned contract"
                            .to_owned(),
                    ));
                }
                if request.host_agent {
                    parse_host_agent_batch(&document.bytes)?
                } else {
                    parse_json_batch(&document.bytes)?
                }
            }
        };
        Ok(discovery_report_receipt(
            "discovery.import",
            "validated",
            "Validated discovery batch",
            report,
        ))
    }

    pub fn commit_discovery_import(
        root: &Path,
        report: DiscoveryImportReport,
    ) -> Result<ActionReceipt<DiscoveryImportReport>, ApplicationError> {
        let source_kind = report_source_kind(&report)?;
        if !matches!(
            source_kind,
            DiscoverySourceKind::Csv | DiscoverySourceKind::Json | DiscoverySourceKind::HostAgent
        ) {
            return Err(ApplicationError::InvalidInput(
                "local discovery import report has an incompatible source kind".to_owned(),
            ));
        }
        let actor = if source_kind == DiscoverySourceKind::HostAgent {
            ActorKind::HostAgent
        } else {
            ActorKind::User
        };
        let mut workspace = open_workspace(root)?;
        let report = DiscoveryService::new(&mut workspace.database).import_report(report, actor)?;
        Ok(discovery_report_receipt(
            "discovery.import",
            "imported",
            "Imported discovery batch",
            report,
        ))
    }

    pub fn preview_discovery_refresh(
        request: &DiscoveryRefreshRequest,
        _consent: NetworkFetchConsent,
    ) -> Result<ActionReceipt<DiscoveryImportReport>, ApplicationError> {
        let adapter = discovery_adapter(request);
        let report = adapter.refresh(
            &HttpFetcher::new(),
            &request.endpoint,
            current_utc_timestamp()?,
        )?;
        Ok(discovery_report_receipt(
            "discovery.refresh",
            "validated",
            "Validated discovery refresh",
            report,
        ))
    }

    pub fn commit_discovery_refresh(
        root: &Path,
        report: DiscoveryImportReport,
    ) -> Result<ActionReceipt<DiscoveryImportReport>, ApplicationError> {
        let source_kind = report_source_kind(&report)?;
        if !matches!(
            source_kind,
            DiscoverySourceKind::RssAtom
                | DiscoverySourceKind::JobsAcUk
                | DiscoverySourceKind::Greenhouse
                | DiscoverySourceKind::Lever
        ) {
            return Err(ApplicationError::InvalidInput(
                "network discovery refresh report has an incompatible source kind".to_owned(),
            ));
        }
        let mut workspace = open_workspace(root)?;
        let report = DiscoveryService::new(&mut workspace.database)
            .import_report(report, ActorKind::User)?;
        Ok(discovery_report_receipt(
            "discovery.refresh",
            "refreshed",
            "Committed discovery refresh",
            report,
        ))
    }

    pub fn list_discovery_sources(
        root: &Path,
    ) -> Result<ActionReceipt<DiscoverySourceListReadModel>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let sources = DiscoveryService::new(&mut workspace.database).list_sources()?;
        Ok(ActionReceipt::new(
            "discovery.sources",
            "available",
            format!("Loaded {} discovery source(s)", sources.len()),
            DiscoverySourceListReadModel {
                workspace: workspace.paths.root,
                sources,
            },
        ))
    }

    pub fn list_discovery_leads(
        root: &Path,
        include_history: bool,
    ) -> Result<ActionReceipt<DiscoveryLeadListReadModel>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let leads = DiscoveryService::new(&mut workspace.database).list_leads(include_history)?;
        Ok(ActionReceipt::new(
            "discovery.list",
            "available",
            format!("Loaded {} discovery lead(s)", leads.len()),
            DiscoveryLeadListReadModel {
                workspace: workspace.paths.root,
                include_history,
                leads,
            },
        ))
    }

    pub fn discovery_lead(
        root: &Path,
        lead_id: &str,
    ) -> Result<ActionReceipt<DiscoveryLeadRecord>, ApplicationError> {
        let lead_id = parse_entity_id(lead_id)?;
        let mut workspace = open_workspace(root)?;
        let lead = DiscoveryService::new(&mut workspace.database).get_lead(&lead_id)?;
        Ok(ActionReceipt::new(
            "discovery.show",
            "available",
            format!("Loaded discovery lead {}", lead.id),
            lead,
        ))
    }

    pub fn discovery_suggestions(
        root: &Path,
        lead_id: &str,
        limit: usize,
    ) -> Result<ActionReceipt<DiscoverySuggestionReadModel>, ApplicationError> {
        let lead_id = parse_entity_id(lead_id)?;
        let mut workspace = open_workspace(root)?;
        let suggestions =
            DiscoveryService::new(&mut workspace.database).suggestions(&lead_id, limit)?;
        Ok(ActionReceipt::new(
            "discovery.suggest",
            "available",
            format!("Loaded {} possible duplicate(s)", suggestions.len()),
            DiscoverySuggestionReadModel {
                suggestions,
                automatic_merge: false,
            },
        ))
    }

    pub fn promote_discovery_lead(
        root: &Path,
        lead_id: &str,
    ) -> Result<ActionReceipt<DiscoveryPromotionReadModel>, ApplicationError> {
        let lead_id = parse_entity_id(lead_id)?;
        let mut workspace = open_workspace(root)?;
        let (lead, job) = {
            let mut service = DiscoveryService::new(&mut workspace.database);
            let lead = service.get_lead(&lead_id)?;
            let job = service.promote(&lead_id, ActorKind::User)?;
            (lead, job)
        };
        let import_action = format!("canisend job import {} --url {}", job.id, lead.url);
        Ok(ActionReceipt::new(
            "discovery.promote",
            "promoted",
            format!("Promoted discovery lead into job {}", job.id),
            DiscoveryPromotionReadModel {
                job,
                lead_id: lead_id.clone(),
            },
        )
        .with_next_actions([NextAction {
            action: import_action,
            description: "Import the selected advert through the safe direct-intake URL boundary"
                .to_owned(),
        }]))
    }
}

fn discovery_adapter(request: &DiscoveryRefreshRequest) -> Box<dyn DiscoveryAdapter> {
    match request.adapter {
        DiscoveryNetworkAdapter::RssAtom => Box::new(RssAtomAdapter::new(
            request.source_name.clone(),
            request.organization.clone(),
        )),
        DiscoveryNetworkAdapter::JobsAcUk => {
            Box::new(JobsAcUkAdapter::new(request.organization.clone()))
        }
        DiscoveryNetworkAdapter::Greenhouse => {
            Box::new(GreenhouseAdapter::new(request.source_name.clone()))
        }
        DiscoveryNetworkAdapter::Lever => Box::new(LeverAdapter::new(request.source_name.clone())),
    }
}

fn report_source_kind(
    report: &DiscoveryImportReport,
) -> Result<DiscoverySourceKind, ApplicationError> {
    report
        .batch
        .as_ref()
        .map(|batch| batch.source_kind)
        .ok_or_else(|| {
            ApplicationError::InvalidInput(
                "discovery report has no normalized batch to commit".to_owned(),
            )
        })
}

fn discovery_report_receipt(
    operation: &'static str,
    status: &'static str,
    summary: &'static str,
    report: DiscoveryImportReport,
) -> ActionReceipt<DiscoveryImportReport> {
    ActionReceipt::new(
        operation,
        status,
        format!(
            "{summary}: {} accepted, {} rejected",
            report.accepted, report.rejected
        ),
        report,
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{
        DiscoveryBatch, DiscoveryImportReport, DiscoverySourceKind, ErrorCode, UtcTimestamp,
    };

    use super::{DiscoveryImportRequest, DiscoveryNetworkAdapter, DiscoveryRefreshRequest};
    use crate::{Application, ApplicationError, NetworkFetchConsent, PrivateReadConsent};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-discovery-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn preview_commit_and_reopen_preserve_the_exact_reviewed_batch() {
        let root = temporary_root("workspace");
        let source = temporary_root("leads").with_extension("csv");
        fs::write(
            &source,
            "title,organization,url,location\nLecturer,University X,https://example.edu/a,London\nLecturer,University X,https://example.edu/b,London\n",
        )
        .expect("write CSV");
        Application::initialize_workspace(&root).expect("initialize workspace");

        let preview = Application::preview_discovery_import(
            &DiscoveryImportRequest {
                path: source.clone(),
                source_name: Some("Reviewed leads".to_owned()),
                source_url: Some("https://example.edu/jobs".to_owned()),
                host_agent: false,
            },
            PrivateReadConsent::granted_by_user(),
        )
        .expect("preview import");
        assert_eq!(preview.operation, "discovery.import");
        assert_eq!(preview.status, "validated");
        assert_eq!(preview.data.accepted, 2);

        fs::write(
            &source,
            "title,organization,url\nChanged,Other,https://example.edu/changed\n",
        )
        .expect("replace source after preview");
        let committed = Application::commit_discovery_import(&root, preview.data)
            .expect("commit reviewed report");
        assert_eq!(committed.status, "imported");
        assert_eq!(committed.data.accepted, 2);

        let sources = Application::list_discovery_sources(&root)
            .expect("list sources")
            .data;
        assert_eq!(sources.sources.len(), 1);
        assert_eq!(sources.sources[0].name, "Reviewed leads");
        let leads = Application::list_discovery_leads(&root, false)
            .expect("list leads")
            .data;
        assert_eq!(leads.leads.len(), 2);
        assert!(leads.leads.iter().all(|lead| lead.title == "Lecturer"));
        assert!(leads.leads.iter().all(|lead| lead.title != "Changed"));

        let lead_id = leads.leads[0].id.to_string();
        let shown = Application::discovery_lead(&root, &lead_id)
            .expect("show lead")
            .data;
        assert_eq!(shown.id.as_str(), lead_id);
        let suggestions = Application::discovery_suggestions(&root, &lead_id, 5)
            .expect("suggest duplicate")
            .data;
        assert!(!suggestions.automatic_merge);
        assert_eq!(suggestions.suggestions.len(), 1);

        let promoted = Application::promote_discovery_lead(&root, &lead_id).expect("promote lead");
        let repeated =
            Application::promote_discovery_lead(&root, &lead_id).expect("repeat promotion");
        assert_eq!(promoted.data.job.id, repeated.data.job.id);
        assert_eq!(promoted.next_actions.len(), 1);

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove CSV");
    }

    #[test]
    fn invalid_lead_ids_fail_before_workspace_access() {
        let missing = temporary_root("missing");
        for error in [
            Application::discovery_lead(&missing, "not-a-uuid").expect_err("invalid show ID"),
            Application::discovery_suggestions(&missing, "not-a-uuid", 5)
                .expect_err("invalid suggestion ID"),
            Application::promote_discovery_lead(&missing, "not-a-uuid")
                .expect_err("invalid promotion ID"),
        ] {
            assert!(matches!(error, ApplicationError::InvalidEntityId(_)));
        }
    }

    #[test]
    fn import_modes_and_commit_kinds_are_kept_distinct() {
        let csv = temporary_root("agent").with_extension("csv");
        fs::write(
            &csv,
            "title,organization,url\nRole,Org,https://example.edu/job\n",
        )
        .expect("write CSV");
        let error = Application::preview_discovery_import(
            &DiscoveryImportRequest {
                path: csv.clone(),
                source_name: None,
                source_url: None,
                host_agent: true,
            },
            PrivateReadConsent::granted_by_user(),
        )
        .expect_err("host-agent CSV rejected");
        assert_eq!(error.classify().code, ErrorCode::InputInvalid);

        let report = DiscoveryImportReport {
            dry_run: true,
            accepted: 0,
            rejected: 0,
            diagnostics: Vec::new(),
            batch: Some(DiscoveryBatch {
                source_kind: DiscoverySourceKind::RssAtom,
                source_name: "Feed".to_owned(),
                source_url: Some("https://example.edu/feed".to_owned()),
                cursor: None,
                observed_at: UtcTimestamp::try_new("2026-07-26T00:00:00Z").expect("timestamp"),
                leads: Vec::new(),
            }),
            receipt: None,
        };
        let root = temporary_root("kind");
        let error = Application::commit_discovery_import(&root, report)
            .expect_err("network report rejected by local import");
        assert_eq!(error.classify().code, ErrorCode::InputInvalid);
        assert!(!root.exists());

        fs::remove_file(csv).expect("remove CSV");
    }

    #[test]
    fn adapter_catalog_is_bounded_and_unsafe_refresh_fails_without_workspace() {
        let adapters = Application::discovery_adapters().data.adapters;
        assert_eq!(adapters.len(), 4);
        assert!(adapters.iter().all(|adapter| adapter.network));

        let missing = temporary_root("network");
        let error = Application::preview_discovery_refresh(
            &DiscoveryRefreshRequest {
                adapter: DiscoveryNetworkAdapter::RssAtom,
                endpoint: "http://127.0.0.1:9/feed".to_owned(),
                source_name: "Local feed".to_owned(),
                organization: None,
            },
            NetworkFetchConsent::granted_by_user(),
        )
        .expect_err("loopback refresh rejected");
        assert_eq!(error.classify().code, ErrorCode::InputInvalid);
        assert!(!missing.exists());
    }
}
