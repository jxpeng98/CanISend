use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, AssociationChangeV4, EvidenceAssociationCommitReadModelV4,
    EvidenceAssociationCommitRequestV4, EvidenceAssociationListReadModelV4,
    EvidenceAssociationPreviewReadModelV4, EvidenceAssociationPreviewRequestV4, PrivateReadConsent,
    ProfileAssociationCommitReadModelV4, ProfileAssociationCommitRequestV4,
    ProfileAssociationListReadModelV4, ProfileAssociationPreviewReadModelV4,
    ProfileAssociationPreviewRequestV4,
};
use canisend_contracts::{ApplicationId, ContentRevisionReferenceV3, Sha256Digest};
use serde::Deserialize;

use crate::commands::{DesktopCommandError, run_worker};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssociationListRequestV4 {
    workspace: PathBuf,
    application_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileAssociationPreviewDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    profile_source: ContentRevisionReferenceV3,
    change: AssociationChangeV4,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvidenceAssociationPreviewDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    evidence: ContentRevisionReferenceV3,
    change: AssociationChangeV4,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileAssociationCommitDesktopRequestV4 {
    workspace: PathBuf,
    preview: ProfileAssociationPreviewRequestV4,
    expected_preview_sha256: Sha256Digest,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvidenceAssociationCommitDesktopRequestV4 {
    workspace: PathBuf,
    preview: EvidenceAssociationPreviewRequestV4,
    expected_preview_sha256: Sha256Digest,
    confirmed_private_read: bool,
}

fn profile_association_list_impl(
    request: AssociationListRequestV4,
) -> Result<ActionReceipt<ProfileAssociationListReadModelV4>, DesktopCommandError> {
    Application::list_profile_associations_v4(&request.workspace, &request.application_id)
        .map_err(DesktopCommandError::application)
}

fn evidence_association_list_impl(
    request: AssociationListRequestV4,
) -> Result<ActionReceipt<EvidenceAssociationListReadModelV4>, DesktopCommandError> {
    Application::list_evidence_associations_v4(&request.workspace, &request.application_id)
        .map_err(DesktopCommandError::application)
}

fn profile_association_preview_impl(
    request: ProfileAssociationPreviewDesktopRequestV4,
) -> Result<ActionReceipt<ProfileAssociationPreviewReadModelV4>, DesktopCommandError> {
    Application::preview_profile_association_v4(
        &request.workspace,
        ProfileAssociationPreviewRequestV4 {
            application_id: request.application_id,
            profile_source: request.profile_source,
            change: request.change,
        },
    )
    .map_err(DesktopCommandError::application)
}

fn evidence_association_preview_impl(
    request: EvidenceAssociationPreviewDesktopRequestV4,
) -> Result<ActionReceipt<EvidenceAssociationPreviewReadModelV4>, DesktopCommandError> {
    Application::preview_evidence_association_v4(
        &request.workspace,
        EvidenceAssociationPreviewRequestV4 {
            application_id: request.application_id,
            evidence: request.evidence,
            change: request.change,
        },
    )
    .map_err(DesktopCommandError::application)
}

fn profile_association_commit_impl(
    request: ProfileAssociationCommitDesktopRequestV4,
) -> Result<ActionReceipt<ProfileAssociationCommitReadModelV4>, DesktopCommandError> {
    Application::commit_profile_association_v4(
        &request.workspace,
        ProfileAssociationCommitRequestV4 {
            preview: request.preview,
            expected_preview_sha256: request.expected_preview_sha256,
        },
        request
            .confirmed_private_read
            .then(PrivateReadConsent::granted_by_user),
    )
    .map_err(DesktopCommandError::application)
}

fn evidence_association_commit_impl(
    request: EvidenceAssociationCommitDesktopRequestV4,
) -> Result<ActionReceipt<EvidenceAssociationCommitReadModelV4>, DesktopCommandError> {
    Application::commit_evidence_association_v4(
        &request.workspace,
        EvidenceAssociationCommitRequestV4 {
            preview: request.preview,
            expected_preview_sha256: request.expected_preview_sha256,
        },
        request
            .confirmed_private_read
            .then(PrivateReadConsent::granted_by_user),
    )
    .map_err(DesktopCommandError::application)
}

#[tauri::command]
pub(crate) async fn profile_association_list(
    request: AssociationListRequestV4,
) -> Result<ActionReceipt<ProfileAssociationListReadModelV4>, DesktopCommandError> {
    run_worker(move || profile_association_list_impl(request)).await
}

#[tauri::command]
pub(crate) async fn evidence_association_list(
    request: AssociationListRequestV4,
) -> Result<ActionReceipt<EvidenceAssociationListReadModelV4>, DesktopCommandError> {
    run_worker(move || evidence_association_list_impl(request)).await
}

#[tauri::command]
pub(crate) async fn profile_association_preview(
    request: ProfileAssociationPreviewDesktopRequestV4,
) -> Result<ActionReceipt<ProfileAssociationPreviewReadModelV4>, DesktopCommandError> {
    run_worker(move || profile_association_preview_impl(request)).await
}

#[tauri::command]
pub(crate) async fn evidence_association_preview(
    request: EvidenceAssociationPreviewDesktopRequestV4,
) -> Result<ActionReceipt<EvidenceAssociationPreviewReadModelV4>, DesktopCommandError> {
    run_worker(move || evidence_association_preview_impl(request)).await
}

#[tauri::command]
pub(crate) async fn profile_association_commit(
    request: ProfileAssociationCommitDesktopRequestV4,
) -> Result<ActionReceipt<ProfileAssociationCommitReadModelV4>, DesktopCommandError> {
    run_worker(move || profile_association_commit_impl(request)).await
}

#[tauri::command]
pub(crate) async fn evidence_association_commit(
    request: EvidenceAssociationCommitDesktopRequestV4,
) -> Result<ActionReceipt<EvidenceAssociationCommitReadModelV4>, DesktopCommandError> {
    run_worker(move || evidence_association_commit_impl(request)).await
}
