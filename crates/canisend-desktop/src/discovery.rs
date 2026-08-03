use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, ApprovalBinding, ApprovalDisposition, ApprovalKind, ApprovalScope,
    ApprovalSourceVersion, DiscoveryAdapterCatalogReadModel, DiscoveryImportRequest,
    DiscoveryLeadListReadModel, DiscoveryNetworkAdapter, DiscoveryPromotionReadModel,
    DiscoveryRefreshRequest, DiscoverySourceListReadModel, DiscoverySuggestionReadModel,
    IntakeReviewReadModel, NetworkFetchConsent, PrivateReadConsent,
    approval_disposition_for_application_error, discovery_intake_review,
};
use canisend_contracts::{ConsentScope, DiscoveryImportReport, DiscoveryLeadRecord, Sha256Digest};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    approval::{DesktopApprovalStore, DesktopDiscoveryKind, DesktopPendingApproval, lease_fields},
    commands::{ApplicationWorkerError, DesktopCommandError, run_application_worker, run_worker},
};

const MAX_SUGGESTIONS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DiscoveryPreviewKind {
    Import,
    Refresh,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoveryPreviewReadModel {
    preview_token: String,
    expires_at_unix_ms: u64,
    remaining_ttl_seconds: u64,
    kind: DiscoveryPreviewKind,
    preview: ActionReceipt<DiscoveryImportReport>,
    intake: IntakeReviewReadModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoveryFilePreviewRequest {
    workspace: PathBuf,
    path: PathBuf,
    source_name: Option<String>,
    source_url: Option<String>,
    host_agent: bool,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoveryNetworkPreviewRequest {
    workspace: PathBuf,
    adapter: DiscoveryNetworkAdapter,
    endpoint: String,
    source_name: String,
    organization: Option<String>,
    confirmed_network_fetch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoveryCommitRequest {
    workspace: PathBuf,
    preview_token: String,
    kind: DiscoveryPreviewKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoveryListRequest {
    workspace: PathBuf,
    include_history: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoveryWorkspaceRequest {
    workspace: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoveryLeadRequest {
    workspace: PathBuf,
    lead_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoverySuggestionRequest {
    workspace: PathBuf,
    lead_id: String,
    limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoveryDiscardRequest {
    workspace: PathBuf,
    preview_token: String,
    kind: DiscoveryPreviewKind,
}

fn preview_discovery_file_impl(
    request: DiscoveryFilePreviewRequest,
) -> Result<(ActionReceipt<DiscoveryImportReport>, IntakeReviewReadModel), DesktopCommandError> {
    if !request.confirmed_private_read {
        return Err(DesktopCommandError::consent(
            "Confirm access to the selected private discovery file before previewing it.",
        ));
    }
    let locator = request.path.to_string_lossy().into_owned();
    let preview = Application::preview_discovery_import(
        &DiscoveryImportRequest {
            path: request.path,
            source_name: request.source_name,
            source_url: request.source_url,
            host_agent: request.host_agent,
        },
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(DesktopCommandError::application)?;
    let intake = discovery_intake_review(&preview.data, locator, ConsentScope::ReadPrivateInputs)
        .map_err(DesktopCommandError::application)?;
    Ok((preview, intake))
}

fn preview_discovery_network_impl(
    request: DiscoveryNetworkPreviewRequest,
) -> Result<(ActionReceipt<DiscoveryImportReport>, IntakeReviewReadModel), DesktopCommandError> {
    if !request.confirmed_network_fetch {
        return Err(DesktopCommandError::consent(
            "Confirm the network request before previewing this discovery source.",
        ));
    }
    let locator = request.endpoint.clone();
    let preview = Application::preview_discovery_refresh(
        &DiscoveryRefreshRequest {
            adapter: request.adapter,
            endpoint: request.endpoint,
            source_name: request.source_name,
            organization: request.organization,
        },
        NetworkFetchConsent::granted_by_user(),
    )
    .map_err(DesktopCommandError::application)?;
    let intake =
        discovery_intake_review(&preview.data, locator, ConsentScope::FetchUserSuppliedUrl)
            .map_err(DesktopCommandError::application)?;
    Ok((preview, intake))
}

fn report_digest(report: &DiscoveryImportReport) -> Result<Sha256Digest, DesktopCommandError> {
    let encoded = serde_json::to_vec(report).map_err(|error| {
        DesktopCommandError::state(format!("Discovery preview could not be bound: {error}"))
    })?;
    Sha256Digest::try_new(hex::encode(Sha256::digest(encoded)))
        .map_err(|error| DesktopCommandError::state(error.to_string()))
}

#[tauri::command]
pub(crate) fn discovery_adapters()
-> Result<ActionReceipt<DiscoveryAdapterCatalogReadModel>, DesktopCommandError> {
    Application::discovery_adapters().map_err(DesktopCommandError::application)
}

#[tauri::command]
pub(crate) async fn preview_discovery_file(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: DiscoveryFilePreviewRequest,
) -> Result<DiscoveryPreviewReadModel, DesktopCommandError> {
    let workspace = request.workspace.clone();
    let (preview, intake) = run_worker(move || preview_discovery_file_impl(request)).await?;
    let scope =
        ApprovalScope::for_workspace(&workspace).map_err(DesktopCommandError::application)?;
    let binding = ApprovalBinding::new(
        ApprovalKind::DiscoveryImport,
        scope,
        None,
        ApprovalSourceVersion::Snapshot(report_digest(&preview.data)?),
    );
    let (preview_token, expires_at_unix_ms, remaining_ttl_seconds) = lease_fields(state.insert(
        binding,
        DesktopPendingApproval::Discovery {
            workspace,
            kind: DesktopDiscoveryKind::Import,
            report: Box::new(preview.data.clone()),
        },
    )?);
    Ok(DiscoveryPreviewReadModel {
        preview_token,
        expires_at_unix_ms,
        remaining_ttl_seconds,
        kind: DiscoveryPreviewKind::Import,
        preview,
        intake,
    })
}

#[tauri::command]
pub(crate) async fn preview_discovery_network(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: DiscoveryNetworkPreviewRequest,
) -> Result<DiscoveryPreviewReadModel, DesktopCommandError> {
    let workspace = request.workspace.clone();
    let (preview, intake) = run_worker(move || preview_discovery_network_impl(request)).await?;
    let scope =
        ApprovalScope::for_workspace(&workspace).map_err(DesktopCommandError::application)?;
    let binding = ApprovalBinding::new(
        ApprovalKind::DiscoveryRefresh,
        scope,
        None,
        ApprovalSourceVersion::Snapshot(report_digest(&preview.data)?),
    );
    let (preview_token, expires_at_unix_ms, remaining_ttl_seconds) = lease_fields(state.insert(
        binding,
        DesktopPendingApproval::Discovery {
            workspace,
            kind: DesktopDiscoveryKind::Refresh,
            report: Box::new(preview.data.clone()),
        },
    )?);
    Ok(DiscoveryPreviewReadModel {
        preview_token,
        expires_at_unix_ms,
        remaining_ttl_seconds,
        kind: DiscoveryPreviewKind::Refresh,
        preview,
        intake,
    })
}

#[tauri::command]
pub(crate) async fn commit_discovery_preview(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: DiscoveryCommitRequest,
) -> Result<ActionReceipt<DiscoveryImportReport>, DesktopCommandError> {
    let scope = ApprovalScope::for_workspace(&request.workspace)
        .map_err(DesktopCommandError::application)?;
    let expected_kind = match request.kind {
        DiscoveryPreviewKind::Import => ApprovalKind::DiscoveryImport,
        DiscoveryPreviewKind::Refresh => ApprovalKind::DiscoveryRefresh,
    };
    let grant = state.take(&request.preview_token, expected_kind, &scope)?;
    let DesktopPendingApproval::Discovery {
        workspace,
        kind,
        report,
    } = grant.payload().clone()
    else {
        state.resolve(grant, ApprovalDisposition::Consume)?;
        return Err(DesktopCommandError::state(
            "Approval payload does not match discovery.",
        ));
    };
    match run_application_worker(move || match kind {
        DesktopDiscoveryKind::Import => Application::commit_discovery_import(&workspace, *report),
        DesktopDiscoveryKind::Refresh => Application::commit_discovery_refresh(&workspace, *report),
    })
    .await
    {
        Ok(receipt) => {
            state.resolve(grant, ApprovalDisposition::Consume)?;
            Ok(receipt)
        }
        Err(ApplicationWorkerError::Application(error)) => {
            let disposition = approval_disposition_for_application_error(&error);
            state.resolve(grant, disposition)?;
            Err(DesktopCommandError::application(error))
        }
        Err(ApplicationWorkerError::Worker(message)) => {
            state.resolve(grant, ApprovalDisposition::Consume)?;
            Err(DesktopCommandError::worker(message))
        }
    }
}

#[tauri::command]
pub(crate) fn discard_discovery_preview(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: DiscoveryDiscardRequest,
) -> Result<(), DesktopCommandError> {
    let scope = ApprovalScope::for_workspace(&request.workspace)
        .map_err(DesktopCommandError::application)?;
    let kind = match request.kind {
        DiscoveryPreviewKind::Import => ApprovalKind::DiscoveryImport,
        DiscoveryPreviewKind::Refresh => ApprovalKind::DiscoveryRefresh,
    };
    state.discard(&request.preview_token, kind, &scope)
}

#[tauri::command]
pub(crate) async fn list_discovery_sources(
    request: DiscoveryWorkspaceRequest,
) -> Result<ActionReceipt<DiscoverySourceListReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::list_discovery_sources(&request.workspace)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn list_discovery_leads(
    request: DiscoveryListRequest,
) -> Result<ActionReceipt<DiscoveryLeadListReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::list_discovery_leads(&request.workspace, request.include_history)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn show_discovery_lead(
    request: DiscoveryLeadRequest,
) -> Result<ActionReceipt<DiscoveryLeadRecord>, DesktopCommandError> {
    run_worker(move || {
        Application::discovery_lead(&request.workspace, &request.lead_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn suggest_discovery_duplicates(
    request: DiscoverySuggestionRequest,
) -> Result<ActionReceipt<DiscoverySuggestionReadModel>, DesktopCommandError> {
    let limit = request.limit.clamp(1, MAX_SUGGESTIONS);
    run_worker(move || {
        Application::discovery_suggestions(&request.workspace, &request.lead_id, limit)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[tauri::command]
pub(crate) async fn promote_discovery_lead(
    request: DiscoveryLeadRequest,
) -> Result<ActionReceipt<DiscoveryPromotionReadModel>, DesktopCommandError> {
    run_worker(move || {
        Application::promote_discovery_lead(&request.workspace, &request.lead_id)
            .map_err(DesktopCommandError::application)
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_app::Application;

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-desktop-discovery-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn discovery_family_uses_the_shared_context_bound_broker() {
        let workspace = temporary_root("shared-broker");
        Application::initialize_workspace(&workspace).expect("initialize workspace");
        let report = DiscoveryImportReport {
            dry_run: true,
            accepted: 0,
            rejected: 0,
            diagnostics: Vec::new(),
            batch: None,
            receipt: None,
        };
        let scope = ApprovalScope::for_workspace(&workspace).expect("approval scope");
        let binding = ApprovalBinding::new(
            ApprovalKind::DiscoveryImport,
            scope.clone(),
            None,
            ApprovalSourceVersion::Snapshot(report_digest(&report).expect("report digest")),
        );
        let store = DesktopApprovalStore::default();
        let lease = store
            .insert(
                binding,
                DesktopPendingApproval::Discovery {
                    workspace: workspace.clone(),
                    kind: DesktopDiscoveryKind::Import,
                    report: Box::new(report),
                },
            )
            .expect("insert discovery approval");
        let grant = store
            .take(&lease.token, ApprovalKind::DiscoveryImport, &scope)
            .expect("take discovery approval");
        assert!(matches!(
            grant.payload(),
            DesktopPendingApproval::Discovery {
                kind: DesktopDiscoveryKind::Import,
                ..
            }
        ));
        store
            .resolve(grant, ApprovalDisposition::Consume)
            .expect("consume discovery approval");
        fs::remove_dir_all(workspace).expect("remove workspace");
    }

    #[test]
    fn local_discovery_preview_and_commit_preserve_the_reviewed_report() {
        let workspace = temporary_root("workspace");
        let source = temporary_root("leads").with_extension("csv");
        fs::write(
            &source,
            "title,organization,url\nLecturer,University X,https://example.edu/job\n",
        )
        .expect("write CSV");
        Application::initialize_workspace(&workspace).expect("initialize workspace");

        let (preview, intake) = preview_discovery_file_impl(DiscoveryFilePreviewRequest {
            workspace: workspace.clone(),
            path: source.clone(),
            source_name: Some("Reviewed leads".to_owned()),
            source_url: None,
            host_agent: false,
            confirmed_private_read: true,
        })
        .expect("preview CSV");
        assert_eq!(preview.data.accepted, 1);
        assert_eq!(intake.source.kind, canisend_app::IntakeSourceKind::Csv);
        assert_eq!(
            intake.commit_boundary,
            canisend_app::IntakeCommitBoundary::ExactNormalizedReport
        );
        fs::write(
            &source,
            "title,organization,url\nChanged,Other,https://example.edu/changed\n",
        )
        .expect("change source after preview");
        let committed = Application::commit_discovery_import(&workspace, preview.data)
            .expect("commit reviewed report");
        assert_eq!(committed.data.accepted, 1);
        let leads = Application::list_discovery_leads(&workspace, false)
            .expect("list leads")
            .data
            .leads;
        assert_eq!(leads[0].title, "Lecturer");

        fs::remove_dir_all(workspace).expect("remove workspace");
        fs::remove_file(source).expect("remove source");
    }

    #[test]
    fn discovery_previews_require_explicit_consent_before_io() {
        let local = preview_discovery_file_impl(DiscoveryFilePreviewRequest {
            workspace: PathBuf::from("/missing/workspace"),
            path: PathBuf::from("/missing/leads.csv"),
            source_name: None,
            source_url: None,
            host_agent: false,
            confirmed_private_read: false,
        })
        .expect_err("local preview needs consent");
        assert_eq!(local.code, "consent-required");

        let network = preview_discovery_network_impl(DiscoveryNetworkPreviewRequest {
            workspace: PathBuf::from("/missing/workspace"),
            adapter: DiscoveryNetworkAdapter::RssAtom,
            endpoint: "https://example.invalid/feed".to_owned(),
            source_name: "Example".to_owned(),
            organization: None,
            confirmed_network_fetch: false,
        })
        .expect_err("network preview needs consent");
        assert_eq!(network.code, "consent-required");
    }
}
