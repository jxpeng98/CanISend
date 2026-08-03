use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, ApprovalBinding, ApprovalDisposition, ApprovalKind, ApprovalScope,
    ApprovalSourceVersion, IntakeReviewReadModel, JobIntakePreviewReadModel, NetworkFetchConsent,
    PreparedJobSource, PrivateReadConsent, SourceImportReadModel,
    approval_disposition_for_application_error, job_intake_review,
};
use serde::{Deserialize, Serialize};

use crate::{
    approval::{DesktopApprovalStore, DesktopPendingApproval, lease_fields},
    commands::{ApplicationWorkerError, DesktopCommandError, run_application_worker, run_worker},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JobIntakePreviewTokenReadModel {
    preview_token: String,
    expires_at_unix_ms: u64,
    remaining_ttl_seconds: u64,
    preview: ActionReceipt<JobIntakePreviewReadModel>,
    intake: IntakeReviewReadModel,
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
    workspace: PathBuf,
    preview_token: String,
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

#[tauri::command]
pub(crate) async fn preview_local_job_source(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: LocalJobIntakePreviewRequest,
) -> Result<JobIntakePreviewTokenReadModel, DesktopCommandError> {
    let prepared = run_worker(move || prepare_local_job_source_impl(request)).await?;
    let preview = prepared.preview().clone();
    let intake = job_intake_review(&preview.data);
    let scope = ApprovalScope::for_workspace(&preview.data.workspace)
        .map_err(DesktopCommandError::application)?;
    let binding = ApprovalBinding::new(
        ApprovalKind::JobIntake,
        scope,
        Some(preview.data.job.id.to_string()),
        ApprovalSourceVersion::RevisionAndSnapshot {
            revision: preview.data.expected_job_revision,
            snapshot_sha256: preview.data.provenance.original_sha256.clone(),
        },
    );
    let (preview_token, expires_at_unix_ms, remaining_ttl_seconds) = lease_fields(state.insert(
        binding,
        DesktopPendingApproval::JobIntake(Box::new(prepared)),
    )?);
    Ok(JobIntakePreviewTokenReadModel {
        preview_token,
        expires_at_unix_ms,
        remaining_ttl_seconds,
        preview,
        intake,
    })
}

#[tauri::command]
pub(crate) async fn preview_url_job_source(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: UrlJobIntakePreviewRequest,
) -> Result<JobIntakePreviewTokenReadModel, DesktopCommandError> {
    let prepared = run_worker(move || prepare_url_job_source_impl(request)).await?;
    let preview = prepared.preview().clone();
    let intake = job_intake_review(&preview.data);
    let scope = ApprovalScope::for_workspace(&preview.data.workspace)
        .map_err(DesktopCommandError::application)?;
    let binding = ApprovalBinding::new(
        ApprovalKind::JobIntake,
        scope,
        Some(preview.data.job.id.to_string()),
        ApprovalSourceVersion::RevisionAndSnapshot {
            revision: preview.data.expected_job_revision,
            snapshot_sha256: preview.data.provenance.original_sha256.clone(),
        },
    );
    let (preview_token, expires_at_unix_ms, remaining_ttl_seconds) = lease_fields(state.insert(
        binding,
        DesktopPendingApproval::JobIntake(Box::new(prepared)),
    )?);
    Ok(JobIntakePreviewTokenReadModel {
        preview_token,
        expires_at_unix_ms,
        remaining_ttl_seconds,
        preview,
        intake,
    })
}

#[tauri::command]
pub(crate) async fn commit_job_source_preview(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: JobIntakePreviewTokenRequest,
) -> Result<ActionReceipt<SourceImportReadModel>, DesktopCommandError> {
    let scope = ApprovalScope::for_workspace(&request.workspace)
        .map_err(DesktopCommandError::application)?;
    let grant = state.take(&request.preview_token, ApprovalKind::JobIntake, &scope)?;
    let DesktopPendingApproval::JobIntake(prepared) = grant.payload().clone() else {
        state.resolve(grant, ApprovalDisposition::Consume)?;
        return Err(DesktopCommandError::state(
            "Approval payload does not match job intake.",
        ));
    };
    match run_application_worker(move || Application::commit_prepared_job_source(*prepared)).await {
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
pub(crate) fn discard_job_source_preview(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: JobIntakePreviewTokenRequest,
) -> Result<(), DesktopCommandError> {
    let scope = ApprovalScope::for_workspace(&request.workspace)
        .map_err(DesktopCommandError::application)?;
    state.discard(&request.preview_token, ApprovalKind::JobIntake, &scope)
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
    fn job_intake_family_uses_the_shared_single_use_broker() {
        let store = DesktopApprovalStore::default();
        let (workspace, source, prepared) = prepared_fixture("shared-broker");
        let preview = prepared.preview();
        let scope = ApprovalScope::for_workspace(&workspace).expect("approval scope");
        let binding = ApprovalBinding::new(
            ApprovalKind::JobIntake,
            scope.clone(),
            Some(preview.data.job.id.to_string()),
            ApprovalSourceVersion::RevisionAndSnapshot {
                revision: preview.data.expected_job_revision,
                snapshot_sha256: preview.data.provenance.original_sha256.clone(),
            },
        );
        let lease = store
            .insert(
                binding,
                DesktopPendingApproval::JobIntake(Box::new(prepared)),
            )
            .expect("insert shared approval");
        assert_eq!(lease.remaining_ttl_seconds, 600);
        let grant = store
            .take(&lease.token, ApprovalKind::JobIntake, &scope)
            .expect("take shared approval");
        assert!(matches!(
            grant.payload(),
            DesktopPendingApproval::JobIntake(_)
        ));
        store
            .resolve(grant, ApprovalDisposition::Consume)
            .expect("consume approval");
        assert!(
            store
                .take(&lease.token, ApprovalKind::JobIntake, &scope)
                .is_err()
        );

        fs::remove_dir_all(workspace).expect("remove workspace");
        fs::remove_file(source).expect("remove source");
    }

    #[test]
    fn job_preview_projects_the_shared_exact_bytes_intake_contract() {
        let (workspace, source, prepared) = prepared_fixture("shared-review");
        let review = job_intake_review(&prepared.preview().data);

        assert_eq!(
            review.source.kind,
            canisend_app::IntakeSourceKind::LocalFile
        );
        assert_eq!(
            review.commit_boundary,
            canisend_app::IntakeCommitBoundary::ExactPreparedBytes
        );
        assert_eq!(
            review.target.id,
            Some(prepared.preview().data.job.id.clone())
        );

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
