use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, AssociationApprovalBrokerV4, AssociationApprovalErrorV4,
    AssociationApprovalPreviewReadModelV4, AssociationChangeV4,
    EvidenceAssociationCommitReadModelV4, EvidenceAssociationListReadModelV4,
    EvidenceAssociationPreviewReadModelV4, EvidenceAssociationPreviewRequestV4, PrivateReadConsent,
    ProfileAssociationCommitReadModelV4, ProfileAssociationListReadModelV4,
    ProfileAssociationPreviewReadModelV4, ProfileAssociationPreviewRequestV4,
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
    application_id: ApplicationId,
    preview_token: String,
    preview_sha256: Sha256Digest,
    approved: bool,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvidenceAssociationCommitDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    preview_token: String,
    preview_sha256: Sha256Digest,
    approved: bool,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssociationDiscardDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    preview_token: String,
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
    broker: AssociationApprovalBrokerV4,
    request: ProfileAssociationPreviewDesktopRequestV4,
) -> Result<
    AssociationApprovalPreviewReadModelV4<ProfileAssociationPreviewReadModelV4>,
    DesktopCommandError,
> {
    broker
        .preview_profile(
            &request.workspace,
            ProfileAssociationPreviewRequestV4 {
                application_id: request.application_id,
                profile_source: request.profile_source,
                change: request.change,
            },
        )
        .map_err(association_error)
}

fn evidence_association_preview_impl(
    broker: AssociationApprovalBrokerV4,
    request: EvidenceAssociationPreviewDesktopRequestV4,
) -> Result<
    AssociationApprovalPreviewReadModelV4<EvidenceAssociationPreviewReadModelV4>,
    DesktopCommandError,
> {
    broker
        .preview_evidence(
            &request.workspace,
            EvidenceAssociationPreviewRequestV4 {
                application_id: request.application_id,
                evidence: request.evidence,
                change: request.change,
            },
        )
        .map_err(association_error)
}

fn profile_association_commit_impl(
    broker: AssociationApprovalBrokerV4,
    request: ProfileAssociationCommitDesktopRequestV4,
) -> Result<ActionReceipt<ProfileAssociationCommitReadModelV4>, DesktopCommandError> {
    broker
        .commit_profile(
            &request.workspace,
            &request.application_id,
            &request.preview_token,
            &request.preview_sha256,
            request.approved,
            request
                .confirmed_private_read
                .then(PrivateReadConsent::granted_by_user),
        )
        .map_err(association_error)
}

fn evidence_association_commit_impl(
    broker: AssociationApprovalBrokerV4,
    request: EvidenceAssociationCommitDesktopRequestV4,
) -> Result<ActionReceipt<EvidenceAssociationCommitReadModelV4>, DesktopCommandError> {
    broker
        .commit_evidence(
            &request.workspace,
            &request.application_id,
            &request.preview_token,
            &request.preview_sha256,
            request.approved,
            request
                .confirmed_private_read
                .then(PrivateReadConsent::granted_by_user),
        )
        .map_err(association_error)
}

fn association_error(error: AssociationApprovalErrorV4) -> DesktopCommandError {
    match error {
        AssociationApprovalErrorV4::Application(error) => DesktopCommandError::application(error),
        AssociationApprovalErrorV4::Approval(error) => DesktopCommandError::approval(error),
        AssociationApprovalErrorV4::Denied => {
            DesktopCommandError::state("Association approval was denied.")
        }
        AssociationApprovalErrorV4::BindingMismatch => DesktopCommandError::state(
            "Association approval does not match the reviewed Application or preview.",
        ),
    }
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
    state: tauri::State<'_, AssociationApprovalBrokerV4>,
    request: ProfileAssociationPreviewDesktopRequestV4,
) -> Result<
    AssociationApprovalPreviewReadModelV4<ProfileAssociationPreviewReadModelV4>,
    DesktopCommandError,
> {
    let broker = state.inner().clone();
    run_worker(move || profile_association_preview_impl(broker, request)).await
}

#[tauri::command]
pub(crate) async fn evidence_association_preview(
    state: tauri::State<'_, AssociationApprovalBrokerV4>,
    request: EvidenceAssociationPreviewDesktopRequestV4,
) -> Result<
    AssociationApprovalPreviewReadModelV4<EvidenceAssociationPreviewReadModelV4>,
    DesktopCommandError,
> {
    let broker = state.inner().clone();
    run_worker(move || evidence_association_preview_impl(broker, request)).await
}

#[tauri::command]
pub(crate) async fn profile_association_commit(
    state: tauri::State<'_, AssociationApprovalBrokerV4>,
    request: ProfileAssociationCommitDesktopRequestV4,
) -> Result<ActionReceipt<ProfileAssociationCommitReadModelV4>, DesktopCommandError> {
    let broker = state.inner().clone();
    run_worker(move || profile_association_commit_impl(broker, request)).await
}

#[tauri::command]
pub(crate) async fn evidence_association_commit(
    state: tauri::State<'_, AssociationApprovalBrokerV4>,
    request: EvidenceAssociationCommitDesktopRequestV4,
) -> Result<ActionReceipt<EvidenceAssociationCommitReadModelV4>, DesktopCommandError> {
    let broker = state.inner().clone();
    run_worker(move || evidence_association_commit_impl(broker, request)).await
}

#[tauri::command]
pub(crate) fn profile_association_discard(
    state: tauri::State<'_, AssociationApprovalBrokerV4>,
    request: AssociationDiscardDesktopRequestV4,
) -> Result<(), DesktopCommandError> {
    state
        .discard_profile(
            &request.workspace,
            &request.application_id,
            &request.preview_token,
        )
        .map_err(association_error)
}

#[tauri::command]
pub(crate) fn evidence_association_discard(
    state: tauri::State<'_, AssociationApprovalBrokerV4>,
    request: AssociationDiscardDesktopRequestV4,
) -> Result<(), DesktopCommandError> {
    state
        .discard_evidence(
            &request.workspace,
            &request.application_id,
            &request.preview_token,
        )
        .map_err(association_error)
}
