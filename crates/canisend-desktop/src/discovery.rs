use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use canisend_app::{
    ActionReceipt, Application, DiscoveryAdapterCatalogReadModel, DiscoveryImportRequest,
    DiscoveryLeadListReadModel, DiscoveryNetworkAdapter, DiscoveryPromotionReadModel,
    DiscoveryRefreshRequest, DiscoverySourceListReadModel, DiscoverySuggestionReadModel,
    IntakeReviewReadModel, NetworkFetchConsent, PrivateReadConsent, discovery_intake_review,
};
use canisend_contracts::{ConsentScope, DiscoveryImportReport, DiscoveryLeadRecord};
use serde::{Deserialize, Serialize};

use crate::commands::{DesktopCommandError, run_worker};

const MAX_PENDING_PREVIEWS: usize = 8;
const MAX_SUGGESTIONS: usize = 20;
static NEXT_PREVIEW: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum DiscoveryPreviewKind {
    Import,
    Refresh,
}

#[derive(Debug, Clone)]
struct PendingDiscoveryPreview {
    kind: DiscoveryPreviewKind,
    report: DiscoveryImportReport,
}

#[derive(Debug, Default)]
struct PendingDiscoveryState {
    previews: BTreeMap<String, PendingDiscoveryPreview>,
    order: VecDeque<String>,
}

#[derive(Debug, Default)]
pub(crate) struct DiscoveryPreviewStore {
    state: Mutex<PendingDiscoveryState>,
}

impl DiscoveryPreviewStore {
    fn insert(
        &self,
        kind: DiscoveryPreviewKind,
        report: DiscoveryImportReport,
    ) -> Result<String, DesktopCommandError> {
        let token = preview_token()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Discovery preview state is unavailable"))?;
        while state.previews.len() >= MAX_PENDING_PREVIEWS {
            let Some(oldest) = state.order.pop_front() else {
                break;
            };
            state.previews.remove(&oldest);
        }
        state.order.push_back(token.clone());
        state
            .previews
            .insert(token.clone(), PendingDiscoveryPreview { kind, report });
        Ok(token)
    }

    fn take(&self, token: &str) -> Result<PendingDiscoveryPreview, DesktopCommandError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Discovery preview state is unavailable"))?;
        let preview = state.previews.remove(token).ok_or_else(|| {
            DesktopCommandError::state(
                "The reviewed discovery preview expired; preview the source again.",
            )
        })?;
        state.order.retain(|existing| existing != token);
        Ok(preview)
    }

    fn restore(
        &self,
        token: String,
        preview: PendingDiscoveryPreview,
    ) -> Result<(), DesktopCommandError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Discovery preview state is unavailable"))?;
        state.order.retain(|existing| existing != &token);
        state.order.push_back(token.clone());
        state.previews.insert(token, preview);
        Ok(())
    }

    fn discard(&self, token: &str) -> Result<(), DesktopCommandError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Discovery preview state is unavailable"))?;
        state.previews.remove(token);
        state.order.retain(|existing| existing != token);
        Ok(())
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.state
            .lock()
            .expect("preview store lock")
            .previews
            .len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoveryPreviewReadModel {
    preview_token: String,
    kind: DiscoveryPreviewKind,
    preview: ActionReceipt<DiscoveryImportReport>,
    intake: IntakeReviewReadModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoveryFilePreviewRequest {
    path: PathBuf,
    source_name: Option<String>,
    source_url: Option<String>,
    host_agent: bool,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoveryNetworkPreviewRequest {
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
    preview_token: String,
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

fn preview_token() -> Result<String, DesktopCommandError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| DesktopCommandError::state("System clock is before the Unix epoch"))?
        .as_millis();
    let sequence = NEXT_PREVIEW.fetch_add(1, Ordering::Relaxed);
    Ok(format!(
        "discovery-preview-{}-{timestamp}-{sequence}",
        std::process::id()
    ))
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) fn discovery_adapters() -> ActionReceipt<DiscoveryAdapterCatalogReadModel> {
    Application::discovery_adapters()
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn preview_discovery_file(
    state: tauri::State<'_, DiscoveryPreviewStore>,
    request: DiscoveryFilePreviewRequest,
) -> Result<DiscoveryPreviewReadModel, DesktopCommandError> {
    let (preview, intake) = run_worker(move || preview_discovery_file_impl(request)).await?;
    let preview_token = state.insert(DiscoveryPreviewKind::Import, preview.data.clone())?;
    Ok(DiscoveryPreviewReadModel {
        preview_token,
        kind: DiscoveryPreviewKind::Import,
        preview,
        intake,
    })
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn preview_discovery_network(
    state: tauri::State<'_, DiscoveryPreviewStore>,
    request: DiscoveryNetworkPreviewRequest,
) -> Result<DiscoveryPreviewReadModel, DesktopCommandError> {
    let (preview, intake) = run_worker(move || preview_discovery_network_impl(request)).await?;
    let preview_token = state.insert(DiscoveryPreviewKind::Refresh, preview.data.clone())?;
    Ok(DiscoveryPreviewReadModel {
        preview_token,
        kind: DiscoveryPreviewKind::Refresh,
        preview,
        intake,
    })
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn commit_discovery_preview(
    state: tauri::State<'_, DiscoveryPreviewStore>,
    request: DiscoveryCommitRequest,
) -> Result<ActionReceipt<DiscoveryImportReport>, DesktopCommandError> {
    let preview = state.take(&request.preview_token)?;
    let retry_preview = preview.clone();
    let workspace = request.workspace;
    let result = run_worker(move || {
        match preview.kind {
            DiscoveryPreviewKind::Import => {
                Application::commit_discovery_import(&workspace, preview.report)
            }
            DiscoveryPreviewKind::Refresh => {
                Application::commit_discovery_refresh(&workspace, preview.report)
            }
        }
        .map_err(DesktopCommandError::application)
    })
    .await;
    if result.is_err() {
        state.restore(request.preview_token, retry_preview)?;
    }
    result
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) fn discard_discovery_preview(
    state: tauri::State<'_, DiscoveryPreviewStore>,
    request: DiscoveryDiscardRequest,
) -> Result<(), DesktopCommandError> {
    state.discard(&request.preview_token)
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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
    fn discovery_preview_store_is_bounded_and_single_use() {
        let store = DiscoveryPreviewStore::default();
        let report = DiscoveryImportReport {
            dry_run: true,
            accepted: 0,
            rejected: 0,
            diagnostics: Vec::new(),
            batch: None,
            receipt: None,
        };
        let mut latest = String::new();
        for _ in 0..(MAX_PENDING_PREVIEWS + 2) {
            latest = store
                .insert(DiscoveryPreviewKind::Import, report.clone())
                .expect("store preview");
        }
        assert_eq!(store.len(), MAX_PENDING_PREVIEWS);
        assert_eq!(
            store.take(&latest).expect("take latest").kind,
            DiscoveryPreviewKind::Import
        );
        assert!(store.take(&latest).is_err());
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
            path: PathBuf::from("/missing/leads.csv"),
            source_name: None,
            source_url: None,
            host_agent: false,
            confirmed_private_read: false,
        })
        .expect_err("local preview needs consent");
        assert_eq!(local.code, "consent-required");

        let network = preview_discovery_network_impl(DiscoveryNetworkPreviewRequest {
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
