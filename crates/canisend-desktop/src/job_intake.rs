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
    ActionReceipt, Application, JobIntakePreviewReadModel, NetworkFetchConsent, PreparedJobSource,
    PrivateReadConsent, SourceImportReadModel,
};
use serde::{Deserialize, Serialize};

use crate::commands::{DesktopCommandError, run_worker};

const MAX_PENDING_PREVIEWS: usize = 8;
static NEXT_PREVIEW: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Default)]
struct PendingJobIntakeState {
    previews: BTreeMap<String, PreparedJobSource>,
    order: VecDeque<String>,
}

#[derive(Debug, Default)]
pub(crate) struct JobIntakePreviewStore {
    state: Mutex<PendingJobIntakeState>,
}

impl JobIntakePreviewStore {
    fn insert(&self, prepared: PreparedJobSource) -> Result<String, DesktopCommandError> {
        let token = preview_token()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Job intake preview state is unavailable"))?;
        while state.previews.len() >= MAX_PENDING_PREVIEWS {
            let Some(oldest) = state.order.pop_front() else {
                break;
            };
            state.previews.remove(&oldest);
        }
        state.order.push_back(token.clone());
        state.previews.insert(token.clone(), prepared);
        Ok(token)
    }

    fn take(&self, token: &str) -> Result<PreparedJobSource, DesktopCommandError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Job intake preview state is unavailable"))?;
        let prepared = state.previews.remove(token).ok_or_else(|| {
            DesktopCommandError::state(
                "The reviewed job intake preview expired; preview the source again.",
            )
        })?;
        state.order.retain(|existing| existing != token);
        Ok(prepared)
    }

    fn restore(
        &self,
        token: String,
        prepared: PreparedJobSource,
    ) -> Result<(), DesktopCommandError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Job intake preview state is unavailable"))?;
        state.order.retain(|existing| existing != &token);
        state.order.push_back(token.clone());
        state.previews.insert(token, prepared);
        Ok(())
    }

    fn discard(&self, token: &str) -> Result<(), DesktopCommandError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DesktopCommandError::state("Job intake preview state is unavailable"))?;
        state.previews.remove(token);
        state.order.retain(|existing| existing != token);
        Ok(())
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.state
            .lock()
            .expect("job intake preview lock")
            .previews
            .len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JobIntakePreviewTokenReadModel {
    preview_token: String,
    preview: ActionReceipt<JobIntakePreviewReadModel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocalJobIntakePreviewRequest {
    workspace: PathBuf,
    job_id: String,
    source: PathBuf,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UrlJobIntakePreviewRequest {
    workspace: PathBuf,
    job_id: String,
    url: String,
    confirmed_network_fetch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JobIntakePreviewTokenRequest {
    preview_token: String,
}

fn preview_token() -> Result<String, DesktopCommandError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| DesktopCommandError::state("System clock is before the Unix epoch"))?
        .as_millis();
    let sequence = NEXT_PREVIEW.fetch_add(1, Ordering::Relaxed);
    Ok(format!(
        "job-intake-preview-{}-{timestamp}-{sequence}",
        std::process::id()
    ))
}

fn prepare_local_job_source_impl(
    request: LocalJobIntakePreviewRequest,
) -> Result<PreparedJobSource, DesktopCommandError> {
    if !request.confirmed_private_read {
        return Err(DesktopCommandError::consent(
            "Confirm access to the selected private source before previewing it.",
        ));
    }
    Application::prepare_local_job_source(
        &request.workspace,
        &request.job_id,
        &request.source,
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(DesktopCommandError::application)
}

fn prepare_url_job_source_impl(
    request: UrlJobIntakePreviewRequest,
) -> Result<PreparedJobSource, DesktopCommandError> {
    if !request.confirmed_network_fetch {
        return Err(DesktopCommandError::consent(
            "Confirm the network request before previewing this source URL.",
        ));
    }
    Application::prepare_url_job_source(
        &request.workspace,
        &request.job_id,
        &request.url,
        NetworkFetchConsent::granted_by_user(),
    )
    .map_err(DesktopCommandError::application)
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn preview_local_job_source(
    state: tauri::State<'_, JobIntakePreviewStore>,
    request: LocalJobIntakePreviewRequest,
) -> Result<JobIntakePreviewTokenReadModel, DesktopCommandError> {
    let prepared = run_worker(move || prepare_local_job_source_impl(request)).await?;
    let preview = prepared.preview().clone();
    let preview_token = state.insert(prepared)?;
    Ok(JobIntakePreviewTokenReadModel {
        preview_token,
        preview,
    })
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn preview_url_job_source(
    state: tauri::State<'_, JobIntakePreviewStore>,
    request: UrlJobIntakePreviewRequest,
) -> Result<JobIntakePreviewTokenReadModel, DesktopCommandError> {
    let prepared = run_worker(move || prepare_url_job_source_impl(request)).await?;
    let preview = prepared.preview().clone();
    let preview_token = state.insert(prepared)?;
    Ok(JobIntakePreviewTokenReadModel {
        preview_token,
        preview,
    })
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn commit_job_source_preview(
    state: tauri::State<'_, JobIntakePreviewStore>,
    request: JobIntakePreviewTokenRequest,
) -> Result<ActionReceipt<SourceImportReadModel>, DesktopCommandError> {
    let prepared = state.take(&request.preview_token)?;
    let retry = prepared.clone();
    let result = run_worker(move || {
        Application::commit_prepared_job_source(prepared).map_err(DesktopCommandError::application)
    })
    .await;
    if result.is_err() {
        state.restore(request.preview_token, retry)?;
    }
    result
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) fn discard_job_source_preview(
    state: tauri::State<'_, JobIntakePreviewStore>,
    request: JobIntakePreviewTokenRequest,
) -> Result<(), DesktopCommandError> {
    state.discard(&request.preview_token)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-desktop-job-intake-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn prepared_fixture(label: &str) -> (PathBuf, PathBuf, PreparedJobSource) {
        let workspace = temporary_root(label);
        let source = temporary_root("source").with_extension("txt");
        fs::write(&source, "Lecturer job fixture").expect("write source");
        Application::initialize_workspace(&workspace).expect("initialize workspace");
        let job = Application::create_job(&workspace, "Lecturer", "University")
            .expect("create job")
            .data;
        let prepared = Application::prepare_local_job_source(
            &workspace,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("prepare source");
        (workspace, source, prepared)
    }

    #[test]
    fn preview_store_is_bounded_and_single_use() {
        let store = JobIntakePreviewStore::default();
        let (workspace, source, prepared) = prepared_fixture("bounded");
        let mut latest = String::new();
        for _ in 0..(MAX_PENDING_PREVIEWS + 2) {
            latest = store.insert(prepared.clone()).expect("store preview");
        }
        assert_eq!(store.len(), MAX_PENDING_PREVIEWS);
        assert!(store.take(&latest).is_ok());
        assert!(store.take(&latest).is_err());
        fs::remove_dir_all(workspace).expect("remove workspace");
        fs::remove_file(source).expect("remove source");
    }

    #[test]
    fn preview_requires_consent_before_file_or_network_access() {
        let local = prepare_local_job_source_impl(LocalJobIntakePreviewRequest {
            workspace: PathBuf::from("/missing/workspace"),
            job_id: "not-an-id".to_owned(),
            source: PathBuf::from("/missing/private.pdf"),
            confirmed_private_read: false,
        })
        .expect_err("private read consent");
        assert_eq!(local.code, "consent-required");

        let network = prepare_url_job_source_impl(UrlJobIntakePreviewRequest {
            workspace: PathBuf::from("/missing/workspace"),
            job_id: "not-an-id".to_owned(),
            url: "https://example.invalid/job.pdf".to_owned(),
            confirmed_network_fetch: false,
        })
        .expect_err("network consent");
        assert_eq!(network.code, "consent-required");
    }
}
