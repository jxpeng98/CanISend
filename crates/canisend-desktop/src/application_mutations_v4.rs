use std::path::PathBuf;

use canisend_app::{
    ActionReceipt, Application, ApplicationDeliverableReviseRequestV4,
    ApplicationFlowComposeRequestV3, ApplicationFlowReviewReadModelV3,
    ApplicationMutationApprovalBrokerV4, ApplicationMutationApprovalErrorV4,
    ApplicationMutationApprovalPreviewV4, ApplicationPlanConfirmRequestV4,
    ApplicationPlanProposeRequestV4, ApplicationRequirementConfirmRequestV4,
    ApplicationRequirementExtractRequestV4, PrivateReadConsent, StoredApplicationModelV3,
};
use canisend_contracts::{ApplicationId, Sha256Digest};
use serde::Deserialize;

use crate::commands::{DesktopCommandError, run_worker};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequirementConfirmPreviewDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    mutation: ApplicationRequirementConfirmRequestV4,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequirementExtractPreviewDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    mutation: ApplicationRequirementExtractRequestV4,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanProposePreviewDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    mutation: ApplicationPlanProposeRequestV4,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanConfirmPreviewDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    mutation: ApplicationPlanConfirmRequestV4,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeliverableDraftPreviewDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    mutation: ApplicationFlowComposeRequestV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeliverableRevisePreviewDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    mutation: ApplicationDeliverableReviseRequestV4,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplicationMutationCommitDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    preview_token: String,
    preview_sha256: Sha256Digest,
    approved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequirementExtractCommitDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    preview_token: String,
    preview_sha256: Sha256Digest,
    approved: bool,
    confirmed_private_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeliverableAuditDesktopRequestV4 {
    workspace: PathBuf,
    application_id: ApplicationId,
    confirmed_private_read: bool,
}

fn mutation_error(error: ApplicationMutationApprovalErrorV4) -> DesktopCommandError {
    match error {
        ApplicationMutationApprovalErrorV4::Application(error) => {
            DesktopCommandError::application(error)
        }
        ApplicationMutationApprovalErrorV4::Approval(error) => DesktopCommandError::approval(error),
        ApplicationMutationApprovalErrorV4::Denied => {
            DesktopCommandError::state("Application mutation approval was denied.")
        }
        ApplicationMutationApprovalErrorV4::BindingMismatch => DesktopCommandError::state(
            "Application mutation approval does not match the reviewed operation or preview.",
        ),
    }
}

#[tauri::command]
pub(crate) async fn requirement_extract_preview(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: RequirementExtractPreviewDesktopRequestV4,
) -> Result<
    ApplicationMutationApprovalPreviewV4<ApplicationRequirementExtractRequestV4>,
    DesktopCommandError,
> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .preview_requirement_extraction(
                &request.workspace,
                &request.application_id,
                request.mutation,
                request
                    .confirmed_private_read
                    .then_some(PrivateReadConsent::granted_by_user()),
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn requirement_extract_commit(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: RequirementExtractCommitDesktopRequestV4,
) -> Result<ActionReceipt<StoredApplicationModelV3>, DesktopCommandError> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .commit_requirement_extraction(
                &request.workspace,
                &request.application_id,
                &request.preview_token,
                &request.preview_sha256,
                request.approved,
                request
                    .confirmed_private_read
                    .then_some(PrivateReadConsent::granted_by_user()),
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn requirement_confirm_preview(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: RequirementConfirmPreviewDesktopRequestV4,
) -> Result<
    ApplicationMutationApprovalPreviewV4<ApplicationRequirementConfirmRequestV4>,
    DesktopCommandError,
> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .preview_requirement_confirmation(
                &request.workspace,
                &request.application_id,
                request.mutation,
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn requirement_confirm_commit(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: ApplicationMutationCommitDesktopRequestV4,
) -> Result<ActionReceipt<StoredApplicationModelV3>, DesktopCommandError> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .commit_requirement_confirmation(
                &request.workspace,
                &request.application_id,
                &request.preview_token,
                &request.preview_sha256,
                request.approved,
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn plan_propose_preview(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: PlanProposePreviewDesktopRequestV4,
) -> Result<
    ApplicationMutationApprovalPreviewV4<ApplicationPlanProposeRequestV4>,
    DesktopCommandError,
> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .preview_plan_proposal(
                &request.workspace,
                &request.application_id,
                request.mutation,
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn plan_propose_commit(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: ApplicationMutationCommitDesktopRequestV4,
) -> Result<ActionReceipt<StoredApplicationModelV3>, DesktopCommandError> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .commit_plan_proposal(
                &request.workspace,
                &request.application_id,
                &request.preview_token,
                &request.preview_sha256,
                request.approved,
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn plan_confirm_preview(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: PlanConfirmPreviewDesktopRequestV4,
) -> Result<
    ApplicationMutationApprovalPreviewV4<ApplicationPlanConfirmRequestV4>,
    DesktopCommandError,
> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .preview_plan_confirmation(
                &request.workspace,
                &request.application_id,
                request.mutation,
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn plan_confirm_commit(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: ApplicationMutationCommitDesktopRequestV4,
) -> Result<ActionReceipt<StoredApplicationModelV3>, DesktopCommandError> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .commit_plan_confirmation(
                &request.workspace,
                &request.application_id,
                &request.preview_token,
                &request.preview_sha256,
                request.approved,
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn deliverable_draft_preview(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: DeliverableDraftPreviewDesktopRequestV4,
) -> Result<
    ApplicationMutationApprovalPreviewV4<ApplicationFlowComposeRequestV3>,
    DesktopCommandError,
> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .preview_deliverable_draft(
                &request.workspace,
                &request.application_id,
                request.mutation,
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn deliverable_draft_commit(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: ApplicationMutationCommitDesktopRequestV4,
) -> Result<ActionReceipt<StoredApplicationModelV3>, DesktopCommandError> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .commit_deliverable_draft(
                &request.workspace,
                &request.application_id,
                &request.preview_token,
                &request.preview_sha256,
                request.approved,
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn deliverable_revise_preview(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: DeliverableRevisePreviewDesktopRequestV4,
) -> Result<
    ApplicationMutationApprovalPreviewV4<ApplicationDeliverableReviseRequestV4>,
    DesktopCommandError,
> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .preview_deliverable_revision(
                &request.workspace,
                &request.application_id,
                request.mutation,
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn deliverable_revise_commit(
    state: tauri::State<'_, ApplicationMutationApprovalBrokerV4>,
    request: ApplicationMutationCommitDesktopRequestV4,
) -> Result<ActionReceipt<StoredApplicationModelV3>, DesktopCommandError> {
    let broker = state.inner().clone();
    run_worker(move || {
        broker
            .commit_deliverable_revision(
                &request.workspace,
                &request.application_id,
                &request.preview_token,
                &request.preview_sha256,
                request.approved,
            )
            .map_err(mutation_error)
    })
    .await
}

#[tauri::command]
pub(crate) async fn deliverable_audit(
    request: DeliverableAuditDesktopRequestV4,
) -> Result<ActionReceipt<ApplicationFlowReviewReadModelV3>, DesktopCommandError> {
    run_worker(move || {
        Application::audit_deliverables_v4(
            &request.workspace,
            &request.application_id,
            request
                .confirmed_private_read
                .then(PrivateReadConsent::granted_by_user),
        )
        .map_err(DesktopCommandError::application)
    })
    .await
}
