use std::path::{Path, PathBuf};

use canisend_contracts::{
    ActorKind, ApplicationId, PrivacyClassification, Sha256Digest, WorkspaceSourceKindV4,
};
use canisend_store::{
    ApplicationAssociationServiceV4, ApplicationDeliverableReviseRequestV4,
    ApplicationFlowApproveRequestV3, ApplicationFlowComposeRequestV3,
    ApplicationFlowExportReadModelV3, ApplicationFlowReviewReadModelV3, ApplicationFlowServiceV3,
    ApplicationModelCommitResultV3, ApplicationMutationServiceV4, ApplicationPlanConfirmRequestV4,
    ApplicationPlanProposeRequestV4, ApplicationRequirementConfirmRequestV4,
    ApplicationRequirementExtractRequestV4, StoredApplicationModelV3,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ActionReceipt, Application, ApplicationError, ApprovalBinding, ApprovalBroker,
    ApprovalBrokerError, ApprovalDisposition, ApprovalKind, ApprovalScope, ApprovalSourceVersion,
    PrivateExportConsent, PrivateReadConsent,
    application::open_workspace_v4,
    application_flow_v3::{ApplicationFlowExportRequestV3, requested_built_in_pack},
    approval_disposition_for_application_error,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationMutationPreviewV4<T> {
    pub context: crate::ApplicationResourceContextV4,
    pub request: T,
    pub preview_sha256: Sha256Digest,
    pub changes: Vec<String>,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationMutationApprovalPreviewV4<T> {
    pub preview_token: String,
    pub expires_at_unix_ms: u64,
    pub remaining_ttl_seconds: u64,
    pub preview: ActionReceipt<ApplicationMutationPreviewV4<T>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingApplicationMutationV4 {
    RequirementExtract {
        workspace: PathBuf,
        preview: ApplicationMutationPreviewV4<ApplicationRequirementExtractRequestV4>,
    },
    RequirementConfirm {
        workspace: PathBuf,
        preview: ApplicationMutationPreviewV4<ApplicationRequirementConfirmRequestV4>,
    },
    PlanPropose {
        workspace: PathBuf,
        preview: ApplicationMutationPreviewV4<ApplicationPlanProposeRequestV4>,
    },
    PlanConfirm {
        workspace: PathBuf,
        preview: ApplicationMutationPreviewV4<ApplicationPlanConfirmRequestV4>,
    },
    DeliverableDraft {
        workspace: PathBuf,
        preview: ApplicationMutationPreviewV4<ApplicationFlowComposeRequestV3>,
    },
    DeliverableRevise {
        workspace: PathBuf,
        preview: ApplicationMutationPreviewV4<ApplicationDeliverableReviseRequestV4>,
    },
    ReviewDisposition {
        workspace: PathBuf,
        preview: ApplicationMutationPreviewV4<ApplicationFlowApproveRequestV3>,
    },
    ExportPrepare {
        workspace: PathBuf,
        preview: ApplicationMutationPreviewV4<ApplicationFlowExportRequestV3>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationMutationApprovalErrorV4 {
    #[error("{0}")]
    Application(#[from] ApplicationError),
    #[error("{0}")]
    Approval(#[from] ApprovalBrokerError),
    #[error("the Application mutation approval was explicitly denied")]
    Denied,
    #[error("the Application mutation approval does not match the reviewed operation or preview")]
    BindingMismatch,
}

#[derive(Debug, Clone, Default)]
pub struct ApplicationMutationApprovalBrokerV4 {
    broker: ApprovalBroker<PendingApplicationMutationV4>,
}

impl ApplicationMutationApprovalBrokerV4 {
    pub fn preview_requirement_extraction(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationRequirementExtractRequestV4,
        consent: Option<PrivateReadConsent>,
    ) -> Result<
        ApplicationMutationApprovalPreviewV4<ApplicationRequirementExtractRequestV4>,
        ApplicationMutationApprovalErrorV4,
    > {
        let receipt =
            Application::preview_requirement_extraction_v4(root, application_id, request, consent)?;
        self.insert(
            root,
            ApprovalKind::ApplicationRequirementExtraction,
            application_id,
            receipt,
            |workspace, preview| PendingApplicationMutationV4::RequirementExtract {
                workspace,
                preview,
            },
        )
    }

    pub fn preview_requirement_confirmation(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationRequirementConfirmRequestV4,
    ) -> Result<
        ApplicationMutationApprovalPreviewV4<ApplicationRequirementConfirmRequestV4>,
        ApplicationMutationApprovalErrorV4,
    > {
        let receipt =
            Application::preview_requirement_confirmation_v4(root, application_id, request)?;
        self.insert(
            root,
            ApprovalKind::ApplicationRequirementConfirmation,
            application_id,
            receipt,
            |workspace, preview| PendingApplicationMutationV4::RequirementConfirm {
                workspace,
                preview,
            },
        )
    }

    pub fn preview_plan_proposal(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationPlanProposeRequestV4,
    ) -> Result<
        ApplicationMutationApprovalPreviewV4<ApplicationPlanProposeRequestV4>,
        ApplicationMutationApprovalErrorV4,
    > {
        let receipt = Application::preview_plan_proposal_v4(root, application_id, request)?;
        self.insert(
            root,
            ApprovalKind::ApplicationPlanProposal,
            application_id,
            receipt,
            |workspace, preview| PendingApplicationMutationV4::PlanPropose { workspace, preview },
        )
    }

    pub fn preview_plan_confirmation(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationPlanConfirmRequestV4,
    ) -> Result<
        ApplicationMutationApprovalPreviewV4<ApplicationPlanConfirmRequestV4>,
        ApplicationMutationApprovalErrorV4,
    > {
        let receipt = Application::preview_plan_confirmation_v4(root, application_id, request)?;
        self.insert(
            root,
            ApprovalKind::ApplicationPlanConfirmation,
            application_id,
            receipt,
            |workspace, preview| PendingApplicationMutationV4::PlanConfirm { workspace, preview },
        )
    }

    pub fn preview_deliverable_draft(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationFlowComposeRequestV3,
    ) -> Result<
        ApplicationMutationApprovalPreviewV4<ApplicationFlowComposeRequestV3>,
        ApplicationMutationApprovalErrorV4,
    > {
        let receipt = Application::preview_deliverable_draft_v4(root, application_id, request)?;
        self.insert(
            root,
            ApprovalKind::DeliverableDraft,
            application_id,
            receipt,
            |workspace, preview| PendingApplicationMutationV4::DeliverableDraft {
                workspace,
                preview,
            },
        )
    }

    pub fn preview_deliverable_revision(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationDeliverableReviseRequestV4,
    ) -> Result<
        ApplicationMutationApprovalPreviewV4<ApplicationDeliverableReviseRequestV4>,
        ApplicationMutationApprovalErrorV4,
    > {
        let receipt = Application::preview_deliverable_revision_v4(root, application_id, request)?;
        self.insert(
            root,
            ApprovalKind::DeliverableRevision,
            application_id,
            receipt,
            |workspace, preview| PendingApplicationMutationV4::DeliverableRevise {
                workspace,
                preview,
            },
        )
    }

    pub fn preview_review_disposition(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationFlowApproveRequestV3,
        consent: Option<PrivateReadConsent>,
    ) -> Result<
        ApplicationMutationApprovalPreviewV4<ApplicationFlowApproveRequestV3>,
        ApplicationMutationApprovalErrorV4,
    > {
        let receipt =
            Application::preview_review_disposition_v4(root, application_id, request, consent)?;
        self.insert(
            root,
            ApprovalKind::ReviewDisposition,
            application_id,
            receipt,
            |workspace, preview| PendingApplicationMutationV4::ReviewDisposition {
                workspace,
                preview,
            },
        )
    }

    pub fn preview_export_prepare(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationFlowExportRequestV3,
        consent: Option<PrivateExportConsent>,
    ) -> Result<
        ApplicationMutationApprovalPreviewV4<ApplicationFlowExportRequestV3>,
        ApplicationMutationApprovalErrorV4,
    > {
        let receipt =
            Application::preview_export_prepare_v4(root, application_id, request, consent)?;
        self.insert(
            root,
            ApprovalKind::ExportPrepare,
            application_id,
            receipt,
            |workspace, preview| PendingApplicationMutationV4::ExportPrepare { workspace, preview },
        )
    }

    pub fn commit_requirement_confirmation(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        preview_token: &str,
        preview_sha256: &Sha256Digest,
        approved: bool,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationMutationApprovalErrorV4> {
        self.commit(
            root,
            application_id,
            preview_token,
            preview_sha256,
            approved,
            ApprovalKind::ApplicationRequirementConfirmation,
            |pending| match pending {
                PendingApplicationMutationV4::RequirementConfirm { workspace, preview } => {
                    Ok(Application::commit_requirement_confirmation_v4(
                        &workspace,
                        &preview.context.application_id,
                        preview.request,
                        preview.preview_sha256,
                    ))
                }
                _ => Err(ApplicationMutationApprovalErrorV4::BindingMismatch),
            },
        )
    }

    pub fn commit_requirement_extraction(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        preview_token: &str,
        preview_sha256: &Sha256Digest,
        approved: bool,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationMutationApprovalErrorV4> {
        self.commit(
            root,
            application_id,
            preview_token,
            preview_sha256,
            approved,
            ApprovalKind::ApplicationRequirementExtraction,
            |pending| match pending {
                PendingApplicationMutationV4::RequirementExtract { workspace, preview } => {
                    Ok(Application::commit_requirement_extraction_v4(
                        &workspace,
                        &preview.context.application_id,
                        preview.request,
                        preview.preview_sha256,
                        consent,
                    ))
                }
                _ => Err(ApplicationMutationApprovalErrorV4::BindingMismatch),
            },
        )
    }

    pub fn commit_plan_proposal(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        preview_token: &str,
        preview_sha256: &Sha256Digest,
        approved: bool,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationMutationApprovalErrorV4> {
        self.commit(
            root,
            application_id,
            preview_token,
            preview_sha256,
            approved,
            ApprovalKind::ApplicationPlanProposal,
            |pending| match pending {
                PendingApplicationMutationV4::PlanPropose { workspace, preview } => {
                    Ok(Application::commit_plan_proposal_v4(
                        &workspace,
                        &preview.context.application_id,
                        preview.request,
                        preview.preview_sha256,
                    ))
                }
                _ => Err(ApplicationMutationApprovalErrorV4::BindingMismatch),
            },
        )
    }

    pub fn commit_plan_confirmation(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        preview_token: &str,
        preview_sha256: &Sha256Digest,
        approved: bool,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationMutationApprovalErrorV4> {
        self.commit(
            root,
            application_id,
            preview_token,
            preview_sha256,
            approved,
            ApprovalKind::ApplicationPlanConfirmation,
            |pending| match pending {
                PendingApplicationMutationV4::PlanConfirm { workspace, preview } => {
                    Ok(Application::commit_plan_confirmation_v4(
                        &workspace,
                        &preview.context.application_id,
                        preview.request,
                        preview.preview_sha256,
                    ))
                }
                _ => Err(ApplicationMutationApprovalErrorV4::BindingMismatch),
            },
        )
    }

    pub fn commit_deliverable_draft(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        preview_token: &str,
        preview_sha256: &Sha256Digest,
        approved: bool,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationMutationApprovalErrorV4> {
        self.commit(
            root,
            application_id,
            preview_token,
            preview_sha256,
            approved,
            ApprovalKind::DeliverableDraft,
            |pending| match pending {
                PendingApplicationMutationV4::DeliverableDraft { workspace, preview } => {
                    Ok(Application::commit_deliverable_draft_v4(
                        &workspace,
                        &preview.context.application_id,
                        preview.request,
                        preview.preview_sha256,
                    ))
                }
                _ => Err(ApplicationMutationApprovalErrorV4::BindingMismatch),
            },
        )
    }

    pub fn commit_deliverable_revision(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        preview_token: &str,
        preview_sha256: &Sha256Digest,
        approved: bool,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationMutationApprovalErrorV4> {
        self.commit(
            root,
            application_id,
            preview_token,
            preview_sha256,
            approved,
            ApprovalKind::DeliverableRevision,
            |pending| match pending {
                PendingApplicationMutationV4::DeliverableRevise { workspace, preview } => {
                    Ok(Application::commit_deliverable_revision_v4(
                        &workspace,
                        &preview.context.application_id,
                        preview.request,
                        preview.preview_sha256,
                    ))
                }
                _ => Err(ApplicationMutationApprovalErrorV4::BindingMismatch),
            },
        )
    }

    pub fn commit_review_disposition(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        preview_token: &str,
        preview_sha256: &Sha256Digest,
        approved: bool,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationMutationApprovalErrorV4> {
        self.commit(
            root,
            application_id,
            preview_token,
            preview_sha256,
            approved,
            ApprovalKind::ReviewDisposition,
            |pending| match pending {
                PendingApplicationMutationV4::ReviewDisposition { workspace, preview } => {
                    Ok(Application::commit_review_disposition_v4(
                        &workspace,
                        &preview.context.application_id,
                        preview.request,
                        preview.preview_sha256,
                        consent,
                    ))
                }
                _ => Err(ApplicationMutationApprovalErrorV4::BindingMismatch),
            },
        )
    }

    pub fn commit_export_prepare(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        preview_token: &str,
        preview_sha256: &Sha256Digest,
        approved: bool,
        consent: Option<PrivateExportConsent>,
    ) -> Result<ActionReceipt<ApplicationFlowExportReadModelV3>, ApplicationMutationApprovalErrorV4>
    {
        self.commit(
            root,
            application_id,
            preview_token,
            preview_sha256,
            approved,
            ApprovalKind::ExportPrepare,
            |pending| match pending {
                PendingApplicationMutationV4::ExportPrepare { workspace, preview } => {
                    Ok(Application::commit_export_prepare_v4(
                        &workspace,
                        &preview.context.application_id,
                        preview.request,
                        preview.preview_sha256,
                        consent,
                    ))
                }
                _ => Err(ApplicationMutationApprovalErrorV4::BindingMismatch),
            },
        )
    }

    fn insert<T, F>(
        &self,
        root: &Path,
        kind: ApprovalKind,
        application_id: &ApplicationId,
        receipt: ActionReceipt<ApplicationMutationPreviewV4<T>>,
        pending: F,
    ) -> Result<ApplicationMutationApprovalPreviewV4<T>, ApplicationMutationApprovalErrorV4>
    where
        T: Clone,
        F: FnOnce(PathBuf, ApplicationMutationPreviewV4<T>) -> PendingApplicationMutationV4,
    {
        let preview = receipt.data.clone();
        let scope = mutation_scope(root, application_id, &preview.context)?;
        let workspace = scope.workspace.clone();
        let lease = self.broker.insert(
            ApprovalBinding::new(
                kind,
                scope,
                Some(application_id.to_string()),
                ApprovalSourceVersion::RevisionAndSnapshot {
                    revision: preview.context.application_revision,
                    snapshot_sha256: preview.preview_sha256.clone(),
                },
            ),
            pending(workspace, preview),
        )?;
        Ok(ApplicationMutationApprovalPreviewV4 {
            preview_token: lease.token,
            expires_at_unix_ms: lease.expires_at_unix_ms,
            remaining_ttl_seconds: lease.remaining_ttl_seconds,
            preview: receipt,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn commit<R, F>(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        preview_token: &str,
        preview_sha256: &Sha256Digest,
        approved: bool,
        kind: ApprovalKind,
        operation: F,
    ) -> Result<ActionReceipt<R>, ApplicationMutationApprovalErrorV4>
    where
        F: FnOnce(
            PendingApplicationMutationV4,
        ) -> Result<
            Result<ActionReceipt<R>, ApplicationError>,
            ApplicationMutationApprovalErrorV4,
        >,
    {
        let current = Application::application_model_v4(root, application_id.as_str())?.data;
        let resource_context = resource_context(&current);
        let scope = mutation_scope(root, application_id, &resource_context)?;
        let grant = self.broker.take(preview_token, kind, &scope)?;
        if !approved {
            self.broker.resolve(grant, ApprovalDisposition::Consume)?;
            return Err(ApplicationMutationApprovalErrorV4::Denied);
        }
        let binding_matches = grant.binding().application_id.as_deref()
            == Some(application_id.as_str())
            && grant.binding().source
                == ApprovalSourceVersion::RevisionAndSnapshot {
                    revision: resource_context.application_revision,
                    snapshot_sha256: preview_sha256.clone(),
                };
        if !binding_matches {
            self.broker.resolve(grant, ApprovalDisposition::Consume)?;
            return Err(ApplicationMutationApprovalErrorV4::BindingMismatch);
        }
        let result = match operation(grant.payload().clone()) {
            Ok(result) => result,
            Err(error) => {
                self.broker.resolve(grant, ApprovalDisposition::Consume)?;
                return Err(error);
            }
        };
        match result {
            Ok(receipt) => {
                self.broker.resolve(grant, ApprovalDisposition::Consume)?;
                Ok(receipt)
            }
            Err(error) => {
                let disposition = approval_disposition_for_application_error(&error);
                self.broker.resolve(grant, disposition)?;
                Err(error.into())
            }
        }
    }
}

impl Application {
    pub fn preview_requirement_extraction_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationRequirementExtractRequestV4,
        consent: Option<PrivateReadConsent>,
    ) -> Result<
        ActionReceipt<ApplicationMutationPreviewV4<ApplicationRequirementExtractRequestV4>>,
        ApplicationError,
    > {
        require_requirement_source_consent(root, &request, consent)?;
        let (_, context) = validate_mutation(root, application_id, |service, pack| {
            service.validate_requirement_extraction(pack, application_id, &request)
        })?;
        mutation_preview(
            "requirement.extract.preview",
            context,
            request.clone(),
            vec![format!(
                "Add {} exact Source-bound Requirement proposal(s)",
                request.requirements.len()
            )],
        )
    }

    pub fn preview_requirement_confirmation_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationRequirementConfirmRequestV4,
    ) -> Result<
        ActionReceipt<ApplicationMutationPreviewV4<ApplicationRequirementConfirmRequestV4>>,
        ApplicationError,
    > {
        let (stored, context) = validate_mutation(root, application_id, |service, pack| {
            service.validate_requirement_confirmation(pack, application_id, &request)
        })?;
        mutation_preview(
            "requirement.confirm.preview",
            context,
            request,
            vec![format!(
                "Decide {} current Requirement(s)",
                stored.snapshot.requirements.len()
            )],
        )
    }

    pub fn preview_plan_proposal_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationPlanProposeRequestV4,
    ) -> Result<
        ActionReceipt<ApplicationMutationPreviewV4<ApplicationPlanProposeRequestV4>>,
        ApplicationError,
    > {
        let (_, context) = validate_mutation(root, application_id, |service, pack| {
            service.validate_plan_proposal(pack, application_id, &request)
        })?;
        mutation_preview(
            "plan.propose.preview",
            context,
            request.clone(),
            vec![format!(
                "Create one draft Plan with {} Deliverable selection(s)",
                request.deliverables.len()
            )],
        )
    }

    pub fn preview_plan_confirmation_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationPlanConfirmRequestV4,
    ) -> Result<
        ActionReceipt<ApplicationMutationPreviewV4<ApplicationPlanConfirmRequestV4>>,
        ApplicationError,
    > {
        let (_, context) = validate_mutation(root, application_id, |service, pack| {
            service.validate_plan_confirmation(pack, application_id, &request)
        })?;
        mutation_preview(
            "plan.confirm.preview",
            context,
            request,
            vec!["Record explicit user authority on the current draft Plan".to_owned()],
        )
    }

    pub fn preview_deliverable_draft_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationFlowComposeRequestV3,
    ) -> Result<
        ActionReceipt<ApplicationMutationPreviewV4<ApplicationFlowComposeRequestV3>>,
        ApplicationError,
    > {
        let (_, context) = validate_mutation(root, application_id, |service, pack| {
            service.validate_deliverable_draft(pack, application_id, &request)
        })?;
        mutation_preview(
            "deliverable.draft.preview",
            context,
            request.clone(),
            vec![format!(
                "Create {} private Deliverable body/bodies for review",
                request.deliverables.len()
            )],
        )
    }

    pub fn preview_deliverable_revision_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationDeliverableReviseRequestV4,
    ) -> Result<
        ActionReceipt<ApplicationMutationPreviewV4<ApplicationDeliverableReviseRequestV4>>,
        ApplicationError,
    > {
        let (_, context) = validate_mutation(root, application_id, |service, pack| {
            service.validate_deliverable_revision(pack, application_id, &request)
        })?;
        mutation_preview(
            "deliverable.revise.preview",
            context,
            request.clone(),
            vec![format!(
                "Replace private content for Deliverable {}",
                request.deliverable_id
            )],
        )
    }

    pub fn audit_deliverables_v4(
        root: &Path,
        application_id: &ApplicationId,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<ApplicationFlowReviewReadModelV3>, ApplicationError> {
        let reviewed = Self::review_application_flow_v3(root, application_id.as_str(), consent)?;
        Ok(ActionReceipt::new(
            "deliverable.audit",
            "private-content-available",
            "Loaded exact current Deliverable bodies after explicit private-read consent",
            reviewed.data,
        ))
    }

    pub fn inspect_review_v4(
        root: &Path,
        application_id: &ApplicationId,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<ApplicationFlowReviewReadModelV3>, ApplicationError> {
        let reviewed = Self::review_application_flow_v3(root, application_id.as_str(), consent)?;
        Ok(ActionReceipt::new(
            "review.inspect",
            "private-content-available",
            "Loaded the exact current Deliverables for evidence-bound review",
            reviewed.data,
        ))
    }

    pub fn preview_review_disposition_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationFlowApproveRequestV3,
        consent: Option<PrivateReadConsent>,
    ) -> Result<
        ActionReceipt<ApplicationMutationPreviewV4<ApplicationFlowApproveRequestV3>>,
        ApplicationError,
    > {
        if consent.is_none() {
            return Err(private_read_required(
                "Review disposition preview reads the current private Deliverable bodies",
            ));
        }
        Self::inspect_review_v4(root, application_id, consent)?;
        review_disposition_preview(root, application_id, request)
    }

    pub fn preview_export_prepare_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationFlowExportRequestV3,
        consent: Option<PrivateExportConsent>,
    ) -> Result<
        ActionReceipt<ApplicationMutationPreviewV4<ApplicationFlowExportRequestV3>>,
        ApplicationError,
    > {
        if consent.is_none() {
            return Err(private_export_required());
        }
        export_prepare_preview(root, application_id, request)
    }

    fn commit_requirement_confirmation_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationRequirementConfirmRequestV4,
        expected_preview_sha256: Sha256Digest,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationError> {
        ensure_preview(
            Self::preview_requirement_confirmation_v4(root, application_id, request.clone())?
                .data
                .preview_sha256,
            expected_preview_sha256,
        )?;
        commit_mutation(
            root,
            application_id,
            |service, pack| service.confirm_requirements(pack, application_id, request),
            "requirement.confirm.commit",
            "confirmed",
        )
    }

    fn commit_requirement_extraction_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationRequirementExtractRequestV4,
        expected_preview_sha256: Sha256Digest,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationError> {
        ensure_preview(
            Self::preview_requirement_extraction_v4(
                root,
                application_id,
                request.clone(),
                consent,
            )?
            .data
            .preview_sha256,
            expected_preview_sha256,
        )?;
        commit_mutation(
            root,
            application_id,
            |service, pack| service.extract_requirements(pack, application_id, request),
            "requirement.extract.commit",
            "proposed",
        )
    }

    fn commit_plan_proposal_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationPlanProposeRequestV4,
        expected_preview_sha256: Sha256Digest,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationError> {
        ensure_preview(
            Self::preview_plan_proposal_v4(root, application_id, request.clone())?
                .data
                .preview_sha256,
            expected_preview_sha256,
        )?;
        commit_mutation(
            root,
            application_id,
            |service, pack| service.propose_plan(pack, application_id, request),
            "plan.propose.commit",
            "proposed",
        )
    }

    fn commit_plan_confirmation_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationPlanConfirmRequestV4,
        expected_preview_sha256: Sha256Digest,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationError> {
        ensure_preview(
            Self::preview_plan_confirmation_v4(root, application_id, request.clone())?
                .data
                .preview_sha256,
            expected_preview_sha256,
        )?;
        commit_mutation(
            root,
            application_id,
            |service, pack| service.confirm_plan(pack, application_id, request),
            "plan.confirm.commit",
            "confirmed",
        )
    }

    fn commit_deliverable_draft_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationFlowComposeRequestV3,
        expected_preview_sha256: Sha256Digest,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationError> {
        ensure_preview(
            Self::preview_deliverable_draft_v4(root, application_id, request.clone())?
                .data
                .preview_sha256,
            expected_preview_sha256,
        )?;
        let pack = exact_pack(root, application_id)?;
        let mut workspace = open_workspace_v4(root)?;
        let workspace_root = workspace.paths.root.clone();
        let committed = ApplicationFlowServiceV3::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .compose_with_actor(&pack, application_id, request, ActorKind::HostAgent)?;
        Ok(ActionReceipt::new(
            "deliverable.draft.commit",
            "review-required",
            "Committed reviewed private Deliverable drafts",
            committed.commit.stored,
        ))
    }

    fn commit_deliverable_revision_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationDeliverableReviseRequestV4,
        expected_preview_sha256: Sha256Digest,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationError> {
        ensure_preview(
            Self::preview_deliverable_revision_v4(root, application_id, request.clone())?
                .data
                .preview_sha256,
            expected_preview_sha256,
        )?;
        commit_mutation(
            root,
            application_id,
            |service, pack| service.revise_deliverable(pack, application_id, request),
            "deliverable.revise.commit",
            "review-required",
        )
    }

    fn commit_review_disposition_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationFlowApproveRequestV3,
        expected_preview_sha256: Sha256Digest,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationError> {
        if consent.is_none() {
            return Err(private_read_required(
                "Review disposition commit requires the reviewed private-read consent",
            ));
        }
        ensure_preview(
            review_disposition_preview(root, application_id, request.clone())?
                .data
                .preview_sha256,
            expected_preview_sha256,
        )?;
        let pack = exact_pack(root, application_id)?;
        let mut workspace = open_workspace_v4(root)?;
        let workspace_root = workspace.paths.root.clone();
        let committed = ApplicationFlowServiceV3::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .approve_with_actor(&pack, application_id, request, ActorKind::HostAgent)?;
        Ok(ActionReceipt::new(
            "review.disposition.commit",
            "approved",
            "Committed the exact approved review disposition",
            committed.commit.stored,
        ))
    }

    fn commit_export_prepare_v4(
        root: &Path,
        application_id: &ApplicationId,
        request: ApplicationFlowExportRequestV3,
        expected_preview_sha256: Sha256Digest,
        consent: Option<PrivateExportConsent>,
    ) -> Result<ActionReceipt<ApplicationFlowExportReadModelV3>, ApplicationError> {
        if consent.is_none() {
            return Err(private_export_required());
        }
        ensure_preview(
            export_prepare_preview(root, application_id, request.clone())?
                .data
                .preview_sha256,
            expected_preview_sha256,
        )?;
        let pack = exact_pack(root, application_id)?;
        let mut workspace = open_workspace_v4(root)?;
        let workspace_root = workspace.paths.root.clone();
        let exported = ApplicationFlowServiceV3::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace_root,
        )
        .export_with_actor(
            &pack,
            application_id,
            request.expected_revision,
            &request.destination,
            ActorKind::HostAgent,
        )?;
        Ok(ActionReceipt::new(
            "export.prepare.commit",
            "exported",
            "Rendered and exported the exact approved local artifacts; submission performed: no",
            exported,
        ))
    }
}

fn review_disposition_preview(
    root: &Path,
    application_id: &ApplicationId,
    request: ApplicationFlowApproveRequestV3,
) -> Result<
    ActionReceipt<ApplicationMutationPreviewV4<ApplicationFlowApproveRequestV3>>,
    ApplicationError,
> {
    let pack = exact_pack(root, application_id)?;
    let mut workspace = open_workspace_v4(root)?;
    let workspace_root = workspace.paths.root.clone();
    let stored =
        ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .validate_approval(&pack, application_id, request.expected_revision)?;
    let count = stored.snapshot.deliverables.len();
    mutation_preview(
        "review.disposition.preview",
        resource_context(&stored),
        request,
        vec![format!(
            "Approve {count} exact current Deliverable disposition(s)"
        )],
    )
}

fn export_prepare_preview(
    root: &Path,
    application_id: &ApplicationId,
    request: ApplicationFlowExportRequestV3,
) -> Result<
    ActionReceipt<ApplicationMutationPreviewV4<ApplicationFlowExportRequestV3>>,
    ApplicationError,
> {
    if request.application_id != *application_id {
        return Err(ApplicationError::InvalidInput(
            "Export request does not match the selected Application".to_owned(),
        ));
    }
    let pack = exact_pack(root, application_id)?;
    let mut workspace = open_workspace_v4(root)?;
    let workspace_root = workspace.paths.root.clone();
    let stored =
        ApplicationFlowServiceV3::new(&mut workspace.database, &workspace.blobs, &workspace_root)
            .validate_export(
            &pack,
            application_id,
            request.expected_revision,
            &request.destination,
        )?;
    let count = stored.snapshot.deliverables.len();
    mutation_preview(
        "export.prepare.preview",
        resource_context(&stored),
        request.clone(),
        vec![format!(
            "Render {count} approved Deliverable(s) to {}; submission performed: no",
            request.destination
        )],
    )
}

fn private_read_required(message: &str) -> ApplicationError {
    ApplicationError::ConsentRequired {
        message: message.to_owned(),
        remediation: canisend_contracts::NextAction {
            action: "grant private read consent".to_owned(),
            description: "Review the exact Application snapshot and authorize this local-only read"
                .to_owned(),
        },
    }
}

fn private_export_required() -> ApplicationError {
    ApplicationError::ConsentRequired {
        message: "Export preview and commit require explicit private-export consent".to_owned(),
        remediation: canisend_contracts::NextAction {
            action: "grant private export consent".to_owned(),
            description: "Review the exact destination and authorize this local-only export"
                .to_owned(),
        },
    }
}

fn validate_mutation<F>(
    root: &Path,
    application_id: &ApplicationId,
    operation: F,
) -> Result<
    (
        StoredApplicationModelV3,
        crate::ApplicationResourceContextV4,
    ),
    ApplicationError,
>
where
    F: FnOnce(
        &mut ApplicationMutationServiceV4<'_>,
        &canisend_core::VerifiedWorkflowPackBundle,
    ) -> Result<StoredApplicationModelV3, canisend_store::StoreError>,
{
    let pack = exact_pack(root, application_id)?;
    let mut workspace = open_workspace_v4(root)?;
    let stored = operation(
        &mut ApplicationMutationServiceV4::new(&mut workspace.database, &workspace.blobs),
        &pack,
    )?;
    let context = resource_context(&stored);
    Ok((stored, context))
}

fn require_requirement_source_consent(
    root: &Path,
    request: &ApplicationRequirementExtractRequestV4,
    consent: Option<PrivateReadConsent>,
) -> Result<(), ApplicationError> {
    let mut workspace = open_workspace_v4(root)?;
    let source = ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs)
        .source(&request.source.id, request.source.revision)?;
    let requires_private_read = matches!(
        source.kind,
        WorkspaceSourceKindV4::LocalFile | WorkspaceSourceKindV4::TextPdf
    ) && source.privacy != PrivacyClassification::Public;
    if requires_private_read && consent.is_none() {
        return Err(ApplicationError::ConsentRequired {
            message: "Requirement extraction reads an explicitly selected private Source"
                .to_owned(),
            remediation: canisend_contracts::NextAction {
                action: "grant private read consent".to_owned(),
                description: "Review the exact Source revision and authorize Requirement extraction for this Application"
                    .to_owned(),
            },
        });
    }
    Ok(())
}

fn commit_mutation<F>(
    root: &Path,
    application_id: &ApplicationId,
    operation: F,
    operation_id: &'static str,
    status: &'static str,
) -> Result<ActionReceipt<StoredApplicationModelV3>, ApplicationError>
where
    F: FnOnce(
        &mut ApplicationMutationServiceV4<'_>,
        &canisend_core::VerifiedWorkflowPackBundle,
    ) -> Result<ApplicationModelCommitResultV3, canisend_store::StoreError>,
{
    let pack = exact_pack(root, application_id)?;
    let mut workspace = open_workspace_v4(root)?;
    let committed = operation(
        &mut ApplicationMutationServiceV4::new(&mut workspace.database, &workspace.blobs),
        &pack,
    )?;
    Ok(ActionReceipt::new(
        operation_id,
        status,
        "Committed the exact approved Application mutation",
        committed.stored,
    ))
}

fn mutation_preview<T: Clone + Serialize>(
    operation: &'static str,
    context: crate::ApplicationResourceContextV4,
    request: T,
    changes: Vec<String>,
) -> Result<ActionReceipt<ApplicationMutationPreviewV4<T>>, ApplicationError> {
    let preview_sha256 = digest(&(
        "canisend.application-mutation-preview/v4",
        operation,
        &context,
        &request,
        &changes,
    ))?;
    Ok(ActionReceipt::new(
        operation,
        "previewed",
        "Validated an exact Application mutation without authoritative mutation",
        ApplicationMutationPreviewV4 {
            context,
            request,
            preview_sha256,
            changes,
            submission_performed: false,
        },
    ))
}

fn mutation_scope(
    root: &Path,
    application_id: &ApplicationId,
    context: &crate::ApplicationResourceContextV4,
) -> Result<ApprovalScope, ApplicationError> {
    if context.application_id != *application_id {
        return Err(ApplicationError::InvalidInput(
            "Application mutation context does not match the selected Application".to_owned(),
        ));
    }
    let scope = ApprovalScope::for_workspace_pack(root, &context.pack.id)?;
    if scope.pack != context.pack {
        return Err(ApplicationError::ResourceIntegrity(
            "Application Pack binding differs from the verified approval scope".to_owned(),
        ));
    }
    Ok(scope)
}

fn exact_pack(
    root: &Path,
    application_id: &ApplicationId,
) -> Result<canisend_core::VerifiedWorkflowPackBundle, ApplicationError> {
    let stored = Application::application_model_v4(root, application_id.as_str())?.data;
    let pack = requested_built_in_pack(&stored.snapshot.pack.id)?;
    let binding = canisend_contracts::ApplicationPackBindingV3 {
        id: pack.manifest().id.clone(),
        version: pack.manifest().version.clone(),
        content_digest: pack.manifest().content_digest.clone(),
    };
    if binding != stored.snapshot.pack {
        return Err(ApplicationError::ResourceIntegrity(
            "Application Pack binding differs from the verified embedded Pack".to_owned(),
        ));
    }
    Ok(pack)
}

fn resource_context(stored: &StoredApplicationModelV3) -> crate::ApplicationResourceContextV4 {
    crate::ApplicationResourceContextV4 {
        application_id: stored.snapshot.application.id.clone(),
        pack: stored.snapshot.pack.clone(),
        application_revision: stored.snapshot.application.revision,
        snapshot_sha256: stored.snapshot_sha256.clone(),
    }
}

fn ensure_preview(actual: Sha256Digest, expected: Sha256Digest) -> Result<(), ApplicationError> {
    if actual != expected {
        return Err(ApplicationError::InvalidInput(
            "Application mutation preview is stale or differs from the reviewed request".to_owned(),
        ));
    }
    Ok(())
}

fn digest<T: Serialize>(value: &T) -> Result<Sha256Digest, ApplicationError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        ApplicationError::InvalidInput(format!("could not encode Application preview: {error}"))
    })?;
    Sha256Digest::try_new(hex::encode(Sha256::digest(bytes)))
        .map_err(|error| ApplicationError::InvalidInput(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{
        ExecutionMode, PlanStateV3, PlannedDeliverableDispositionV3, RequirementPriorityV3,
        Revision, WorkflowPackId, WorkflowPackItemId,
    };
    use canisend_store::{
        ApplicationFlowCreateRequestV3, ApplicationFlowDeliverableDraftV3,
        ApplicationFlowPlannedDeliverableV3, ApplicationFlowRequirementDraftV3,
    };

    use super::*;
    use crate::{
        ApplicationFlowCreateRequestV4, GENERIC_APPLICATION_WORKFLOW_PACK_ID,
        LocalFileIntakeCommitRequestV4, LocalFileIntakePreviewRequestV4,
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-application-mutations-v4-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn item(value: &str) -> WorkflowPackItemId {
        WorkflowPackItemId::try_new(value).expect("Pack item ID")
    }

    fn create_application(root: &Path, title: &str) -> StoredApplicationModelV3 {
        Application::create_application_flow_v4(
            root,
            ApplicationFlowCreateRequestV4 {
                pack_id: WorkflowPackId::try_new(GENERIC_APPLICATION_WORKFLOW_PACK_ID)
                    .expect("Pack ID"),
                application: ApplicationFlowCreateRequestV3 {
                    title: title.to_owned(),
                    opportunity_metadata: Default::default(),
                    application_metadata: Default::default(),
                    source_text: "Provide one concise narrative.\nUse reviewed evidence."
                        .to_owned(),
                    requirements: vec![
                        ApplicationFlowRequirementDraftV3 {
                            category: item("format"),
                            statement: "Provide one concise narrative.".to_owned(),
                            priority: RequirementPriorityV3::Mandatory,
                            start_byte: 0,
                            end_byte: 30,
                        },
                        ApplicationFlowRequirementDraftV3 {
                            category: item("format"),
                            statement: "Use reviewed evidence.".to_owned(),
                            priority: RequirementPriorityV3::Recommended,
                            start_byte: 31,
                            end_byte: 53,
                        },
                    ],
                },
            },
        )
        .expect("create Application")
        .data
        .stored
    }

    #[test]
    fn requirement_extraction_adds_only_reviewed_proposals_from_an_exact_source() {
        let root = root();
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let created = create_application(&root, "Extract Requirements");
        let application_id = created.snapshot.application.id.clone();
        let source = created.snapshot.requirements[0].source_span.content.clone();
        let broker = ApplicationMutationApprovalBrokerV4::default();
        let preview = broker
            .preview_requirement_extraction(
                &root,
                &application_id,
                ApplicationRequirementExtractRequestV4 {
                    expected_revision: Revision::try_new(1).expect("revision"),
                    source,
                    requirements: vec![ApplicationFlowRequirementDraftV3 {
                        category: item("format"),
                        statement: "reviewed evidence".to_owned(),
                        priority: RequirementPriorityV3::Mandatory,
                        start_byte: 35,
                        end_byte: 52,
                    }],
                },
                None,
            )
            .expect("extraction preview");
        assert_eq!(preview.preview.data.request.requirements.len(), 1);
        let committed = broker
            .commit_requirement_extraction(
                &root,
                &application_id,
                &preview.preview_token,
                &preview.preview.data.preview_sha256,
                true,
                None,
            )
            .expect("extraction commit");
        assert_eq!(committed.data.snapshot.application.revision.get(), 2);
        assert_eq!(committed.data.snapshot.requirements.len(), 3);
        assert_eq!(
            committed.data.snapshot.requirements[2].statement,
            "reviewed evidence"
        );
        assert!(matches!(
            broker.commit_requirement_extraction(
                &root,
                &application_id,
                &preview.preview_token,
                &preview.preview.data.preview_sha256,
                true,
                None,
            ),
            Err(ApplicationMutationApprovalErrorV4::Approval(
                ApprovalBrokerError::Missing
            ))
        ));
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn requirement_extraction_rejects_wrong_application_invalid_spans_and_stale_preview() {
        let root = root();
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let first = create_application(&root, "First extraction");
        let second = create_application(&root, "Second extraction");
        let first_id = first.snapshot.application.id.clone();
        let second_id = second.snapshot.application.id.clone();
        let source = first.snapshot.requirements[0].source_span.content.clone();
        let broker = ApplicationMutationApprovalBrokerV4::default();
        let request =
            |statement: &str, start_byte, end_byte| ApplicationRequirementExtractRequestV4 {
                expected_revision: Revision::try_new(1).expect("revision"),
                source: source.clone(),
                requirements: vec![ApplicationFlowRequirementDraftV3 {
                    category: item("format"),
                    statement: statement.to_owned(),
                    priority: RequirementPriorityV3::Recommended,
                    start_byte,
                    end_byte,
                }],
            };

        assert!(
            broker
                .preview_requirement_extraction(
                    &root,
                    &second_id,
                    request("reviewed evidence", 35, 52),
                    None,
                )
                .is_err()
        );
        assert!(
            broker
                .preview_requirement_extraction(
                    &root,
                    &first_id,
                    request("invented statement", 35, 52),
                    None,
                )
                .is_err()
        );
        assert!(
            broker
                .preview_requirement_extraction(
                    &root,
                    &first_id,
                    request("Provide one concise narrative.", 0, 30),
                    None,
                )
                .is_err()
        );

        let stale = broker
            .preview_requirement_extraction(
                &root,
                &first_id,
                request("reviewed evidence", 35, 52),
                None,
            )
            .expect("stale preview");
        let winner = broker
            .preview_requirement_extraction(
                &root,
                &first_id,
                request("concise narrative", 12, 29),
                None,
            )
            .expect("winning preview");
        broker
            .commit_requirement_extraction(
                &root,
                &first_id,
                &winner.preview_token,
                &winner.preview.data.preview_sha256,
                true,
                None,
            )
            .expect("winning commit");
        assert!(matches!(
            broker.commit_requirement_extraction(
                &root,
                &first_id,
                &stale.preview_token,
                &stale.preview.data.preview_sha256,
                true,
                None,
            ),
            Err(ApplicationMutationApprovalErrorV4::BindingMismatch)
        ));
        let current = Application::application_model_v4(&root, first_id.as_str())
            .expect("current Application")
            .data;
        assert_eq!(current.snapshot.application.revision.get(), 2);
        assert_eq!(current.snapshot.requirements.len(), 3);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn requirement_extraction_requires_consent_for_private_local_source_reads() {
        let root = root();
        let source_path = root.with_extension("private-source.txt");
        fs::write(
            &source_path,
            "Provide one concise narrative.\nUse reviewed evidence.",
        )
        .expect("write private Source");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let request = LocalFileIntakePreviewRequestV4 {
            pack_id: WorkflowPackId::try_new(GENERIC_APPLICATION_WORKFLOW_PACK_ID)
                .expect("Pack ID"),
            title: "Private extraction".to_owned(),
            opportunity_metadata: Default::default(),
            application_metadata: Default::default(),
            path: source_path.clone(),
            requirement_category: item("format"),
            requirement_priority: RequirementPriorityV3::Mandatory,
        };
        let consent = PrivateReadConsent::granted_by_user();
        let preview =
            Application::preview_local_file_intake_v4(&root, request.clone(), Some(consent))
                .expect("local preview");
        let created = Application::commit_local_file_intake_v4(
            &root,
            LocalFileIntakeCommitRequestV4 {
                preview: request,
                expected_preview_sha256: preview.data.preview_sha256,
            },
            Some(consent),
        )
        .expect("local commit")
        .data
        .stored;
        let application_id = created.snapshot.application.id;
        let mutation = ApplicationRequirementExtractRequestV4 {
            expected_revision: Revision::try_new(1).expect("revision"),
            source: created.snapshot.requirements[0].source_span.content.clone(),
            requirements: vec![ApplicationFlowRequirementDraftV3 {
                category: item("format"),
                statement: "concise narrative".to_owned(),
                priority: RequirementPriorityV3::Recommended,
                start_byte: 12,
                end_byte: 29,
            }],
        };
        let broker = ApplicationMutationApprovalBrokerV4::default();
        assert!(matches!(
            broker.preview_requirement_extraction(&root, &application_id, mutation.clone(), None),
            Err(ApplicationMutationApprovalErrorV4::Application(
                ApplicationError::ConsentRequired { .. }
            ))
        ));
        broker
            .preview_requirement_extraction(&root, &application_id, mutation, Some(consent))
            .expect("consented extraction preview");

        fs::remove_dir_all(root).expect("remove fixture");
        fs::remove_file(source_path).expect("remove private Source");
    }

    #[test]
    fn guarded_mutations_split_requirements_plan_and_deliverables_without_replay() {
        let root = root();
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let created = create_application(&root, "Guarded workflow");
        let application_id = created.snapshot.application.id.clone();
        let broker = ApplicationMutationApprovalBrokerV4::default();

        let decisions = created
            .snapshot
            .requirements
            .iter()
            .enumerate()
            .map(|(index, requirement)| {
                (
                    requirement.id.clone(),
                    if index == 0 {
                        canisend_store::RequirementDecisionV4::Confirm
                    } else {
                        canisend_store::RequirementDecisionV4::Exclude
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let denied = broker
            .preview_requirement_confirmation(
                &root,
                &application_id,
                ApplicationRequirementConfirmRequestV4 {
                    expected_revision: Revision::try_new(1).expect("revision"),
                    decisions: decisions.clone(),
                },
            )
            .expect("denied preview");
        assert!(matches!(
            broker.commit_requirement_confirmation(
                &root,
                &application_id,
                &denied.preview_token,
                &denied.preview.data.preview_sha256,
                false,
            ),
            Err(ApplicationMutationApprovalErrorV4::Denied)
        ));
        assert_eq!(
            Application::application_model_v4(&root, application_id.as_str())
                .expect("unchanged")
                .data
                .snapshot
                .application
                .revision
                .get(),
            1
        );

        let requirement_preview = broker
            .preview_requirement_confirmation(
                &root,
                &application_id,
                ApplicationRequirementConfirmRequestV4 {
                    expected_revision: Revision::try_new(1).expect("revision"),
                    decisions,
                },
            )
            .expect("Requirement preview");
        let requirement_digest = requirement_preview.preview.data.preview_sha256.clone();
        let requirement_token = requirement_preview.preview_token.clone();
        let requirements = broker
            .commit_requirement_confirmation(
                &root,
                &application_id,
                &requirement_token,
                &requirement_digest,
                true,
            )
            .expect("Requirement commit");
        assert_eq!(requirements.data.snapshot.application.revision.get(), 2);
        assert!(
            broker
                .commit_requirement_confirmation(
                    &root,
                    &application_id,
                    &requirement_token,
                    &requirement_digest,
                    true,
                )
                .is_err()
        );

        let plan_preview = broker
            .preview_plan_proposal(
                &root,
                &application_id,
                ApplicationPlanProposeRequestV4 {
                    expected_revision: Revision::try_new(2).expect("revision"),
                    decision: item("proceed"),
                    deliverables: vec![ApplicationFlowPlannedDeliverableV3 {
                        kind: item("primary-document"),
                        disposition: PlannedDeliverableDispositionV3::Required,
                        rationale: "Required by the reviewed Source".to_owned(),
                        constraints: Vec::new(),
                        execution_mode: Some(ExecutionMode::HostAgent),
                    }],
                },
            )
            .expect("Plan preview");
        let plan = broker
            .commit_plan_proposal(
                &root,
                &application_id,
                &plan_preview.preview_token,
                &plan_preview.preview.data.preview_sha256,
                true,
            )
            .expect("Plan proposal");
        assert_eq!(
            plan.data.snapshot.plan.as_ref().expect("Plan").state,
            PlanStateV3::Draft
        );

        let confirmation_preview = broker
            .preview_plan_confirmation(
                &root,
                &application_id,
                ApplicationPlanConfirmRequestV4 {
                    expected_revision: Revision::try_new(3).expect("revision"),
                },
            )
            .expect("Plan confirmation preview");
        let confirmed = broker
            .commit_plan_confirmation(
                &root,
                &application_id,
                &confirmation_preview.preview_token,
                &confirmation_preview.preview.data.preview_sha256,
                true,
            )
            .expect("Plan confirmation");
        assert_eq!(
            confirmed.data.snapshot.plan.as_ref().expect("Plan").state,
            PlanStateV3::Confirmed
        );

        let draft_preview = broker
            .preview_deliverable_draft(
                &root,
                &application_id,
                ApplicationFlowComposeRequestV3 {
                    expected_revision: Revision::try_new(4).expect("revision"),
                    deliverables: vec![ApplicationFlowDeliverableDraftV3 {
                        kind: item("primary-document"),
                        title: "Private draft".to_owned(),
                        media_type: "text/markdown".to_owned(),
                        content: "PRIVATE-DRAFT-BODY-V1".to_owned(),
                    }],
                },
            )
            .expect("draft preview");
        let drafted = broker
            .commit_deliverable_draft(
                &root,
                &application_id,
                &draft_preview.preview_token,
                &draft_preview.preview.data.preview_sha256,
                true,
            )
            .expect("draft commit");
        let deliverable_id = drafted.data.snapshot.deliverables[0].id.clone();
        assert!(Application::audit_deliverables_v4(&root, &application_id, None).is_err());
        let audit = Application::audit_deliverables_v4(
            &root,
            &application_id,
            Some(PrivateReadConsent::granted_by_user()),
        )
        .expect("consented audit");
        assert_eq!(audit.data.deliverables[0].content, "PRIVATE-DRAFT-BODY-V1");

        let revision_preview = broker
            .preview_deliverable_revision(
                &root,
                &application_id,
                ApplicationDeliverableReviseRequestV4 {
                    expected_revision: Revision::try_new(5).expect("revision"),
                    deliverable_id: deliverable_id.clone(),
                    title: "Private revised draft".to_owned(),
                    media_type: "text/markdown".to_owned(),
                    content: "PRIVATE-DRAFT-BODY-V2".to_owned(),
                },
            )
            .expect("revision preview");
        let revised = broker
            .commit_deliverable_revision(
                &root,
                &application_id,
                &revision_preview.preview_token,
                &revision_preview.preview.data.preview_sha256,
                true,
            )
            .expect("revision commit");
        assert_eq!(revised.data.snapshot.application.revision.get(), 6);
        assert_eq!(revised.data.snapshot.deliverables[0].id, deliverable_id);
        assert_eq!(revised.data.snapshot.deliverables[0].revision.get(), 2);
        let audit = Application::audit_deliverables_v4(
            &root,
            &application_id,
            Some(PrivateReadConsent::granted_by_user()),
        )
        .expect("revised audit");
        assert_eq!(audit.data.deliverables[0].content, "PRIVATE-DRAFT-BODY-V2");

        assert!(Application::inspect_review_v4(&root, &application_id, None).is_err());
        let read_consent = PrivateReadConsent::granted_by_user();
        let inspected = Application::inspect_review_v4(&root, &application_id, Some(read_consent))
            .expect("review inspect");
        assert_eq!(inspected.operation, "review.inspect");
        assert_eq!(inspected.data.deliverables.len(), 1);

        let disposition_request = ApplicationFlowApproveRequestV3 {
            expected_revision: Revision::try_new(6).expect("revision"),
        };
        assert!(
            broker
                .preview_review_disposition(
                    &root,
                    &application_id,
                    disposition_request.clone(),
                    None,
                )
                .is_err()
        );
        let disposition = broker
            .preview_review_disposition(
                &root,
                &application_id,
                disposition_request,
                Some(read_consent),
            )
            .expect("review disposition preview");
        let approved = broker
            .commit_review_disposition(
                &root,
                &application_id,
                &disposition.preview_token,
                &disposition.preview.data.preview_sha256,
                true,
                Some(read_consent),
            )
            .expect("review disposition commit");
        assert_eq!(approved.data.snapshot.application.revision.get(), 7);

        let destination = format!("applications/{application_id}/exports/agent-v4-test");
        let export_request =
            ApplicationFlowExportRequestV3::try_new(application_id.as_str(), 7, &destination)
                .expect("export request");
        assert!(
            broker
                .preview_export_prepare(&root, &application_id, export_request.clone(), None,)
                .is_err()
        );
        let export_consent = PrivateExportConsent::granted_by_user();
        let export = broker
            .preview_export_prepare(&root, &application_id, export_request, Some(export_consent))
            .expect("export preview");
        let exported = broker
            .commit_export_prepare(
                &root,
                &application_id,
                &export.preview_token,
                &export.preview.data.preview_sha256,
                true,
                Some(export_consent),
            )
            .expect("export commit");
        assert_eq!(exported.operation, "export.prepare.commit");
        assert!(!exported.data.render.submission_performed);
        assert!(
            root.join(&destination)
                .join("render-manifest.json")
                .is_file()
        );
        let listed =
            Application::list_exports_v4(&root, application_id.as_str()).expect("export list");
        assert_eq!(listed.data.exports.len(), 1);
        let shown = Application::show_export_v4(&root, application_id.as_str(), &destination)
            .expect("export show");
        assert_eq!(shown.data.manifest, exported.data.render);
        assert!(
            broker
                .commit_export_prepare(
                    &root,
                    &application_id,
                    &export.preview_token,
                    &export.preview.data.preview_sha256,
                    true,
                    Some(export_consent),
                )
                .is_err()
        );
        let pdf = root.join(shown.data.manifest.documents[0].relative_path.as_str());
        let mut bytes = fs::read(&pdf).expect("exported PDF");
        bytes[0] ^= 1;
        fs::write(&pdf, bytes).expect("tamper exported PDF");
        assert!(Application::show_export_v4(&root, application_id.as_str(), &destination).is_err());

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn mutation_preview_is_bound_to_the_exact_application() {
        let root = root();
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let first = create_application(&root, "First");
        let second = create_application(&root, "Second");
        let first_id = first.snapshot.application.id;
        let second_id = second.snapshot.application.id;
        let decisions = first
            .snapshot
            .requirements
            .iter()
            .map(|requirement| {
                (
                    requirement.id.clone(),
                    canisend_store::RequirementDecisionV4::Confirm,
                )
            })
            .collect();
        let broker = ApplicationMutationApprovalBrokerV4::default();
        let preview = broker
            .preview_requirement_confirmation(
                &root,
                &first_id,
                ApplicationRequirementConfirmRequestV4 {
                    expected_revision: Revision::try_new(1).expect("revision"),
                    decisions,
                },
            )
            .expect("preview");
        assert!(
            broker
                .commit_requirement_confirmation(
                    &root,
                    &second_id,
                    &preview.preview_token,
                    &preview.preview.data.preview_sha256,
                    true,
                )
                .is_err()
        );
        assert_eq!(
            Application::application_model_v4(&root, first_id.as_str())
                .expect("first")
                .data
                .snapshot
                .application
                .revision
                .get(),
            1
        );
        assert_eq!(
            Application::application_model_v4(&root, second_id.as_str())
                .expect("second")
                .data
                .snapshot
                .application
                .revision
                .get(),
            1
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn stale_mutation_preview_cannot_overwrite_a_newer_application_revision() {
        let root = root();
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let created = create_application(&root, "Stale preview");
        let application_id = created.snapshot.application.id;
        let decisions = created
            .snapshot
            .requirements
            .iter()
            .map(|requirement| {
                (
                    requirement.id.clone(),
                    canisend_store::RequirementDecisionV4::Confirm,
                )
            })
            .collect::<BTreeMap<_, _>>();
        let stale_broker = ApplicationMutationApprovalBrokerV4::default();
        let stale = stale_broker
            .preview_requirement_confirmation(
                &root,
                &application_id,
                ApplicationRequirementConfirmRequestV4 {
                    expected_revision: Revision::try_new(1).expect("revision"),
                    decisions: decisions.clone(),
                },
            )
            .expect("stale preview");
        let current_broker = ApplicationMutationApprovalBrokerV4::default();
        let current = current_broker
            .preview_requirement_confirmation(
                &root,
                &application_id,
                ApplicationRequirementConfirmRequestV4 {
                    expected_revision: Revision::try_new(1).expect("revision"),
                    decisions,
                },
            )
            .expect("current preview");
        current_broker
            .commit_requirement_confirmation(
                &root,
                &application_id,
                &current.preview_token,
                &current.preview.data.preview_sha256,
                true,
            )
            .expect("current commit");

        assert!(
            stale_broker
                .commit_requirement_confirmation(
                    &root,
                    &application_id,
                    &stale.preview_token,
                    &stale.preview.data.preview_sha256,
                    true,
                )
                .is_err()
        );
        assert_eq!(
            Application::application_model_v4(&root, application_id.as_str())
                .expect("Application")
                .data
                .snapshot
                .application
                .revision
                .get(),
            2
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
