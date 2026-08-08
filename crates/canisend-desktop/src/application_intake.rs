use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, ApplicationFlowReadModelV3, ApprovalBinding, ApprovalDisposition,
    ApprovalKind, ApprovalScope, ApprovalSourceVersion, LocalFileIntakeCommitRequestV4,
    LocalFileIntakePreviewReadModelV4, LocalFileIntakePreviewRequestV4, NetworkFetchConsent,
    PastedTextIntakeCommitRequestV4, PastedTextIntakePreviewReadModelV4,
    PastedTextIntakePreviewRequestV4, PrivateReadConsent, UrlDocumentKindV4,
    UrlIntakeCommitRequestV4, UrlIntakePreviewReadModelV4, UrlIntakePreviewRequestV4,
    approval_disposition_for_application_error,
};
use canisend_contracts::{ConsentScope, Sha256Digest, WorkflowPackId, WorkspaceSourceKindV4};
use serde::{Deserialize, Serialize};

use crate::{
    approval::{DesktopApprovalStore, DesktopPendingApproval, lease_fields},
    commands::{ApplicationWorkerError, DesktopCommandError, run_application_worker, run_worker},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ApplicationIntakeSourceKindV4 {
    PastedText,
    LocalFile,
    TextPdf,
    UrlHtml,
    UrlPlainText,
    UrlPdf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplicationIntakePreviewReadModelV4 {
    pack_id: WorkflowPackId,
    title: String,
    source_kind: ApplicationIntakeSourceKindV4,
    requested_locator: Option<String>,
    final_locator: Option<String>,
    redirect_chain: Vec<String>,
    content_type: String,
    preview_sha256: Sha256Digest,
    original_sha256: Option<Sha256Digest>,
    normalized_sha256: Sha256Digest,
    original_bytes: Option<u64>,
    normalized_text_bytes: u64,
    normalized_lines: u64,
    pdf_page_count: Option<u64>,
    requirement_count: u64,
    duplicate_count: u64,
    required_consent: Option<ConsentScope>,
    submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplicationIntakePreviewTokenReadModelV4 {
    preview_token: String,
    expires_at_unix_ms: u64,
    remaining_ttl_seconds: u64,
    preview: ActionReceipt<ApplicationIntakePreviewReadModelV4>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PastedApplicationIntakePreviewRequestV4 {
    workspace: PathBuf,
    preview: PastedTextIntakePreviewRequestV4,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocalApplicationIntakePreviewRequestV4 {
    workspace: PathBuf,
    preview: LocalFileIntakePreviewRequestV4,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UrlApplicationIntakePreviewRequestV4 {
    workspace: PathBuf,
    preview: UrlIntakePreviewRequestV4,
    confirmed_network_fetch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplicationIntakePreviewTokenRequestV4 {
    workspace: PathBuf,
    pack_id: WorkflowPackId,
    preview_token: String,
}

#[derive(Debug, Clone)]
pub(crate) enum PreparedApplicationIntakeV4 {
    Pasted {
        workspace: PathBuf,
        request: PastedTextIntakeCommitRequestV4,
    },
    LocalFile {
        workspace: PathBuf,
        request: LocalFileIntakeCommitRequestV4,
    },
    Url {
        workspace: PathBuf,
        request: UrlIntakeCommitRequestV4,
    },
}

struct PreparedDesktopApplicationIntakeV4 {
    binding: ApprovalBinding,
    pending: PreparedApplicationIntakeV4,
    preview: ActionReceipt<ApplicationIntakePreviewReadModelV4>,
}

fn preview_pasted_application_intake_impl(
    request: PastedApplicationIntakePreviewRequestV4,
) -> Result<PreparedDesktopApplicationIntakeV4, DesktopCommandError> {
    let pack_id = request.preview.pack_id.clone();
    let preview =
        Application::preview_pasted_text_intake_v4(&request.workspace, request.preview.clone())
            .map_err(DesktopCommandError::application)?;
    let digest = preview.data.preview_sha256.clone();
    let summary = pasted_summary(&pack_id, &request.preview, &preview.data)?;
    Ok(PreparedDesktopApplicationIntakeV4 {
        binding: intake_binding(&request.workspace, &pack_id, digest.clone())?,
        pending: PreparedApplicationIntakeV4::Pasted {
            workspace: request.workspace,
            request: PastedTextIntakeCommitRequestV4 {
                preview: request.preview,
                expected_preview_sha256: digest,
            },
        },
        preview: map_receipt(preview, summary),
    })
}

fn preview_local_application_intake_impl(
    request: LocalApplicationIntakePreviewRequestV4,
) -> Result<PreparedDesktopApplicationIntakeV4, DesktopCommandError> {
    if !request.confirmed_private_read {
        return Err(DesktopCommandError::consent(
            "Confirm access to the selected private Source before previewing it.",
        ));
    }
    let pack_id = request.preview.pack_id.clone();
    let preview = Application::preview_local_file_intake_v4(
        &request.workspace,
        request.preview.clone(),
        Some(PrivateReadConsent::granted_by_user()),
    )
    .map_err(DesktopCommandError::application)?;
    let digest = preview.data.preview_sha256.clone();
    let summary = local_summary(&pack_id, &request.preview, &preview.data)?;
    Ok(PreparedDesktopApplicationIntakeV4 {
        binding: intake_binding(&request.workspace, &pack_id, digest.clone())?,
        pending: PreparedApplicationIntakeV4::LocalFile {
            workspace: request.workspace,
            request: LocalFileIntakeCommitRequestV4 {
                preview: request.preview,
                expected_preview_sha256: digest,
            },
        },
        preview: map_receipt(preview, summary),
    })
}

fn preview_url_application_intake_impl(
    request: UrlApplicationIntakePreviewRequestV4,
) -> Result<PreparedDesktopApplicationIntakeV4, DesktopCommandError> {
    if !request.confirmed_network_fetch {
        return Err(DesktopCommandError::consent(
            "Confirm the bounded network fetch before previewing this Source URL.",
        ));
    }
    let pack_id = request.preview.pack_id.clone();
    let preview = Application::preview_url_intake_v4(
        &request.workspace,
        request.preview.clone(),
        Some(NetworkFetchConsent::granted_by_user()),
    )
    .map_err(DesktopCommandError::application)?;
    let digest = preview.data.preview_sha256.clone();
    let summary = url_summary(&pack_id, &request.preview, &preview.data)?;
    Ok(PreparedDesktopApplicationIntakeV4 {
        binding: intake_binding(&request.workspace, &pack_id, digest.clone())?,
        pending: PreparedApplicationIntakeV4::Url {
            workspace: request.workspace,
            request: UrlIntakeCommitRequestV4 {
                preview: request.preview,
                expected_preview_sha256: digest,
            },
        },
        preview: map_receipt(preview, summary),
    })
}

fn intake_binding(
    workspace: &std::path::Path,
    pack_id: &WorkflowPackId,
    digest: Sha256Digest,
) -> Result<ApprovalBinding, DesktopCommandError> {
    let scope = ApprovalScope::for_workspace_pack(workspace, pack_id)
        .map_err(DesktopCommandError::application)?;
    Ok(ApprovalBinding::new(
        ApprovalKind::ApplicationIntake,
        scope,
        None,
        ApprovalSourceVersion::Snapshot(digest),
    ))
}

fn pasted_summary(
    pack_id: &WorkflowPackId,
    request: &PastedTextIntakePreviewRequestV4,
    preview: &PastedTextIntakePreviewReadModelV4,
) -> Result<ApplicationIntakePreviewReadModelV4, DesktopCommandError> {
    Ok(ApplicationIntakePreviewReadModelV4 {
        pack_id: pack_id.clone(),
        title: request.title.clone(),
        source_kind: ApplicationIntakeSourceKindV4::PastedText,
        requested_locator: None,
        final_locator: None,
        redirect_chain: Vec::new(),
        content_type: "text/plain; charset=utf-8".to_owned(),
        preview_sha256: preview.preview_sha256.clone(),
        original_sha256: None,
        normalized_sha256: preview.source_sha256.clone(),
        original_bytes: None,
        normalized_text_bytes: preview.normalized_text_bytes,
        normalized_lines: preview.normalized_lines,
        pdf_page_count: None,
        requirement_count: preview.requirement_count,
        duplicate_count: 0,
        required_consent: None,
        submission_performed: preview.submission_performed,
    })
}

fn local_summary(
    pack_id: &WorkflowPackId,
    request: &LocalFileIntakePreviewRequestV4,
    preview: &LocalFileIntakePreviewReadModelV4,
) -> Result<ApplicationIntakePreviewReadModelV4, DesktopCommandError> {
    let source_kind = match preview.source_kind {
        WorkspaceSourceKindV4::LocalFile => ApplicationIntakeSourceKindV4::LocalFile,
        WorkspaceSourceKindV4::TextPdf => ApplicationIntakeSourceKindV4::TextPdf,
        _ => {
            return Err(DesktopCommandError::state(
                "Local Application intake returned an unexpected Source kind.",
            ));
        }
    };
    Ok(ApplicationIntakePreviewReadModelV4 {
        pack_id: pack_id.clone(),
        title: request.title.clone(),
        source_kind,
        requested_locator: Some(request.path.to_string_lossy().into_owned()),
        final_locator: None,
        redirect_chain: Vec::new(),
        content_type: preview.content_type.clone(),
        preview_sha256: preview.preview_sha256.clone(),
        original_sha256: Some(preview.original_sha256.clone()),
        normalized_sha256: preview.normalized_sha256.clone(),
        original_bytes: Some(preview.original_bytes),
        normalized_text_bytes: preview.normalized_text_bytes,
        normalized_lines: preview.normalized_lines,
        pdf_page_count: preview.pdf_page_count,
        requirement_count: count_requirements(&preview.application.requirements)?,
        duplicate_count: count_duplicates(&preview.duplicates)?,
        required_consent: Some(ConsentScope::ReadPrivateInputs),
        submission_performed: preview.submission_performed,
    })
}

fn url_summary(
    pack_id: &WorkflowPackId,
    request: &UrlIntakePreviewRequestV4,
    preview: &UrlIntakePreviewReadModelV4,
) -> Result<ApplicationIntakePreviewReadModelV4, DesktopCommandError> {
    let source_kind = match preview.document_kind {
        UrlDocumentKindV4::Html => ApplicationIntakeSourceKindV4::UrlHtml,
        UrlDocumentKindV4::PlainText => ApplicationIntakeSourceKindV4::UrlPlainText,
        UrlDocumentKindV4::Pdf => ApplicationIntakeSourceKindV4::UrlPdf,
    };
    Ok(ApplicationIntakePreviewReadModelV4 {
        pack_id: pack_id.clone(),
        title: request.title.clone(),
        source_kind,
        requested_locator: Some(preview.source_url.clone()),
        final_locator: Some(preview.final_url.clone()),
        redirect_chain: preview.redirect_chain.clone(),
        content_type: preview.content_type.clone(),
        preview_sha256: preview.preview_sha256.clone(),
        original_sha256: Some(preview.original_sha256.clone()),
        normalized_sha256: preview.normalized_sha256.clone(),
        original_bytes: Some(preview.original_bytes),
        normalized_text_bytes: preview.normalized_text_bytes,
        normalized_lines: preview.normalized_lines,
        pdf_page_count: preview.pdf_page_count,
        requirement_count: count_requirements(&preview.application.requirements)?,
        duplicate_count: count_duplicates(&preview.duplicates)?,
        required_consent: Some(ConsentScope::FetchUserSuppliedUrl),
        submission_performed: preview.submission_performed,
    })
}

fn count_requirements<T>(items: &[T]) -> Result<u64, DesktopCommandError> {
    u64::try_from(items.len())
        .map_err(|_| DesktopCommandError::state("Application Requirement count overflow."))
}

fn count_duplicates<T>(items: &[T]) -> Result<u64, DesktopCommandError> {
    u64::try_from(items.len())
        .map_err(|_| DesktopCommandError::state("Application duplicate count overflow."))
}

fn map_receipt<T>(
    receipt: ActionReceipt<T>,
    data: ApplicationIntakePreviewReadModelV4,
) -> ActionReceipt<ApplicationIntakePreviewReadModelV4> {
    ActionReceipt {
        operation: receipt.operation,
        status: receipt.status,
        summary: receipt.summary,
        data,
        artifacts: receipt.artifacts,
        required_consents: receipt.required_consents,
        warnings: receipt.warnings,
        next_actions: receipt.next_actions,
        compatibility: receipt.compatibility,
    }
}

fn commit_application_intake_impl(
    pending: PreparedApplicationIntakeV4,
) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, canisend_app::ApplicationError> {
    match pending {
        PreparedApplicationIntakeV4::Pasted { workspace, request } => {
            Application::commit_pasted_text_intake_v4(&workspace, request)
        }
        PreparedApplicationIntakeV4::LocalFile { workspace, request } => {
            Application::commit_local_file_intake_v4(
                &workspace,
                request,
                Some(PrivateReadConsent::granted_by_user()),
            )
        }
        PreparedApplicationIntakeV4::Url { workspace, request } => {
            Application::commit_url_intake_v4(
                &workspace,
                request,
                Some(NetworkFetchConsent::granted_by_user()),
            )
        }
    }
}

fn insert_preview(
    state: tauri::State<'_, DesktopApprovalStore>,
    prepared: PreparedDesktopApplicationIntakeV4,
) -> Result<ApplicationIntakePreviewTokenReadModelV4, DesktopCommandError> {
    let (preview_token, expires_at_unix_ms, remaining_ttl_seconds) = lease_fields(state.insert(
        prepared.binding,
        DesktopPendingApproval::ApplicationIntake(Box::new(prepared.pending)),
    )?);
    Ok(ApplicationIntakePreviewTokenReadModelV4 {
        preview_token,
        expires_at_unix_ms,
        remaining_ttl_seconds,
        preview: prepared.preview,
    })
}

#[tauri::command]
pub(crate) async fn preview_pasted_application_intake(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: PastedApplicationIntakePreviewRequestV4,
) -> Result<ApplicationIntakePreviewTokenReadModelV4, DesktopCommandError> {
    let prepared = run_worker(move || preview_pasted_application_intake_impl(request)).await?;
    insert_preview(state, prepared)
}

#[tauri::command]
pub(crate) async fn preview_local_application_intake(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: LocalApplicationIntakePreviewRequestV4,
) -> Result<ApplicationIntakePreviewTokenReadModelV4, DesktopCommandError> {
    let prepared = run_worker(move || preview_local_application_intake_impl(request)).await?;
    insert_preview(state, prepared)
}

#[tauri::command]
pub(crate) async fn preview_url_application_intake(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: UrlApplicationIntakePreviewRequestV4,
) -> Result<ApplicationIntakePreviewTokenReadModelV4, DesktopCommandError> {
    let prepared = run_worker(move || preview_url_application_intake_impl(request)).await?;
    insert_preview(state, prepared)
}

#[tauri::command]
pub(crate) async fn commit_application_intake_preview(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: ApplicationIntakePreviewTokenRequestV4,
) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, DesktopCommandError> {
    let scope = ApprovalScope::for_workspace_pack(&request.workspace, &request.pack_id)
        .map_err(DesktopCommandError::application)?;
    let grant = state.take(
        &request.preview_token,
        ApprovalKind::ApplicationIntake,
        &scope,
    )?;
    let DesktopPendingApproval::ApplicationIntake(pending) = grant.payload().clone() else {
        state.resolve(grant, ApprovalDisposition::Consume)?;
        return Err(DesktopCommandError::state(
            "Approval payload does not match Application intake.",
        ));
    };
    match run_application_worker(move || commit_application_intake_impl(*pending)).await {
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
pub(crate) fn discard_application_intake_preview(
    state: tauri::State<'_, DesktopApprovalStore>,
    request: ApplicationIntakePreviewTokenRequestV4,
) -> Result<(), DesktopCommandError> {
    let scope = ApprovalScope::for_workspace_pack(&request.workspace, &request.pack_id)
        .map_err(DesktopCommandError::application)?;
    state.discard(
        &request.preview_token,
        ApprovalKind::ApplicationIntake,
        &scope,
    )
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use canisend_app::{ACADEMIC_JOB_WORKFLOW_PACK_ID, GENERIC_APPLICATION_WORKFLOW_PACK_ID};
    use canisend_contracts::{ApplicationFieldValueV3, RequirementPriorityV3, WorkflowPackItemId};

    use super::*;

    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-desktop-application-intake-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("Pack item ID")
    }

    fn pasted(pack_id: &str, private_body: &str) -> PastedTextIntakePreviewRequestV4 {
        PastedTextIntakePreviewRequestV4 {
            pack_id: WorkflowPackId::try_new(pack_id).expect("Pack ID"),
            title: "Desktop connected intake".to_owned(),
            opportunity_metadata: if pack_id == ACADEMIC_JOB_WORKFLOW_PACK_ID {
                BTreeMap::from([(
                    item("institution"),
                    ApplicationFieldValueV3::ShortText("Example University".to_owned()),
                )])
            } else {
                BTreeMap::from([(
                    item("organization"),
                    ApplicationFieldValueV3::ShortText("Example Foundation".to_owned()),
                )])
            },
            application_metadata: BTreeMap::new(),
            source_text: private_body.to_owned(),
            requirement_category: item(if pack_id == ACADEMIC_JOB_WORKFLOW_PACK_ID {
                "qualification"
            } else {
                "format"
            }),
            requirement_priority: RequirementPriorityV3::Mandatory,
        }
    }

    #[test]
    fn pasted_preview_is_body_free_and_commits_either_pack_in_one_workspace() {
        let workspace = root("mixed");
        Application::initialize_workspace_v4(&workspace).expect("Workspace v4");
        for (pack_id, body) in [
            (GENERIC_APPLICATION_WORKFLOW_PACK_ID, "PRIVATE-GENERIC-BODY"),
            (ACADEMIC_JOB_WORKFLOW_PACK_ID, "PRIVATE-ACADEMIC-BODY"),
        ] {
            let prepared =
                preview_pasted_application_intake_impl(PastedApplicationIntakePreviewRequestV4 {
                    workspace: workspace.clone(),
                    preview: pasted(pack_id, body),
                })
                .expect("preview");
            let serialized = serde_json::to_string(&prepared.preview).expect("preview JSON");
            assert!(!serialized.contains(body));
            assert!(!prepared.preview.data.submission_performed);
            commit_application_intake_impl(prepared.pending).expect("commit");
        }
        let applications = Application::list_application_models_v3(&workspace)
            .expect("Applications")
            .data;
        assert_eq!(applications.len(), 2);
        assert!(applications.iter().any(|application| {
            application.snapshot.pack.id.as_str() == GENERIC_APPLICATION_WORKFLOW_PACK_ID
        }));
        assert!(applications.iter().any(|application| {
            application.snapshot.pack.id.as_str() == ACADEMIC_JOB_WORKFLOW_PACK_ID
        }));
        fs::remove_dir_all(workspace).expect("remove Workspace");
    }

    #[test]
    fn local_preview_requires_consent_and_reports_pdf_or_text_without_body() {
        let workspace = root("local");
        let source = root("source").with_extension("txt");
        let body = "PRIVATE-LOCAL-SOURCE";
        fs::write(&source, body).expect("write Source");
        Application::initialize_workspace_v4(&workspace).expect("Workspace v4");
        let request = LocalApplicationIntakePreviewRequestV4 {
            workspace: workspace.clone(),
            preview: LocalFileIntakePreviewRequestV4 {
                pack_id: WorkflowPackId::try_new(GENERIC_APPLICATION_WORKFLOW_PACK_ID)
                    .expect("Pack ID"),
                title: "Local intake".to_owned(),
                opportunity_metadata: BTreeMap::from([(
                    item("organization"),
                    ApplicationFieldValueV3::ShortText("Example Foundation".to_owned()),
                )]),
                application_metadata: BTreeMap::new(),
                path: source.clone(),
                requirement_category: item("format"),
                requirement_priority: RequirementPriorityV3::Mandatory,
            },
            confirmed_private_read: false,
        };
        assert!(preview_local_application_intake_impl(request.clone()).is_err());
        let prepared =
            preview_local_application_intake_impl(LocalApplicationIntakePreviewRequestV4 {
                confirmed_private_read: true,
                ..request
            })
            .expect("preview");
        assert_eq!(
            prepared.preview.data.source_kind,
            ApplicationIntakeSourceKindV4::LocalFile
        );
        assert!(
            !serde_json::to_string(&prepared.preview)
                .expect("preview JSON")
                .contains(body)
        );
        commit_application_intake_impl(prepared.pending).expect("commit");
        fs::remove_dir_all(workspace).expect("remove Workspace");
        fs::remove_file(source).expect("remove Source");
    }
}
