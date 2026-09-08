use std::path::Path;

use canisend_contracts::{
    ApplicationId, ContentRevisionReferenceV3, EvidenceCatalogRecord, EvidenceProposalSet,
    NextAction, PrivacyClassification, Sha256Digest,
};
use canisend_store::{EvidenceService, ProfileService};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ActionReceipt, Application, ApplicationError, ApplicationResourceContextV4, ApprovalBinding,
    ApprovalBroker, ApprovalBrokerError, ApprovalDisposition, ApprovalKind, ApprovalScope,
    ApprovalSourceVersion, PrivateReadConsent,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidencePreviewReadModelV4 {
    pub context: ApplicationResourceContextV4,
    pub profile_source: ContentRevisionReferenceV3,
    pub catalog: EvidenceCatalogRecord,
    pub requires_private_read: bool,
    pub preview_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceApprovalPreviewV4 {
    pub preview_token: String,
    pub expires_at_unix_ms: u64,
    pub remaining_ttl_seconds: u64,
    pub preview: ActionReceipt<EvidencePreviewReadModelV4>,
}

#[derive(Debug, thiserror::Error)]
pub enum EvidenceApprovalErrorV4 {
    #[error("{0}")]
    Application(#[from] ApplicationError),
    #[error("{0}")]
    Approval(#[from] ApprovalBrokerError),
    #[error("the Evidence confirmation was explicitly denied")]
    Denied,
    #[error("the Evidence confirmation differs from its exact preview")]
    BindingMismatch,
}

#[derive(Debug, Clone, Default)]
pub struct EvidenceApprovalBrokerV4 {
    broker: ApprovalBroker<EvidencePreviewReadModelV4>,
}

impl EvidenceApprovalBrokerV4 {
    pub fn preview(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        profile_source: ContentRevisionReferenceV3,
        proposals: EvidenceProposalSet,
        consent: Option<PrivateReadConsent>,
    ) -> Result<EvidenceApprovalPreviewV4, EvidenceApprovalErrorV4> {
        let (scope, context) = current_context(root, application_id)?;
        let mut workspace = crate::application::open_workspace_v4(&scope.workspace)?;
        let source = ProfileService::new(&mut workspace.database, &workspace.blobs)
            .get_source(&profile_source.id)
            .map_err(ApplicationError::from)?;
        let requires_private_read = source.sensitivity != PrivacyClassification::Public;
        require_consent(requires_private_read, consent)?;
        let catalog = EvidenceService::new(&mut workspace.database, &workspace.blobs)
            .prepare_v4(&profile_source, &proposals)
            .map_err(ApplicationError::from)?;
        let bytes =
            serde_json::to_vec(&(&context, &profile_source, &catalog, requires_private_read))
                .map_err(|error| ApplicationError::InvalidInput(error.to_string()))?;
        let digest = Sha256Digest::try_new(hex::encode(Sha256::digest(bytes)))
            .map_err(|error| ApplicationError::InvalidInput(error.to_string()))?;
        let preview = EvidencePreviewReadModelV4 {
            context,
            profile_source,
            catalog,
            requires_private_read,
            preview_sha256: digest,
        };
        let lease = self
            .broker
            .insert(binding(scope, &preview), preview.clone())?;
        Ok(EvidenceApprovalPreviewV4 {
            preview_token: lease.token,
            expires_at_unix_ms: lease.expires_at_unix_ms,
            remaining_ttl_seconds: lease.remaining_ttl_seconds,
            preview: ActionReceipt::new(
                "evidence.confirm.preview",
                "previewed",
                "Prepared exact source-bound Workspace Evidence; no change committed",
                preview,
            ),
        })
    }

    pub fn confirmation_preview(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        token: &str,
        digest: &Sha256Digest,
    ) -> Result<EvidencePreviewReadModelV4, EvidenceApprovalErrorV4> {
        let (scope, context) = current_context(root, application_id)?;
        let expected = ApprovalBinding::new(
            ApprovalKind::EvidenceConfirmation,
            scope,
            Some(application_id.to_string()),
            ApprovalSourceVersion::RevisionAndSnapshot {
                revision: context.application_revision,
                snapshot_sha256: digest.clone(),
            },
        );
        let preview = self.broker.review(token, &expected)?;
        if preview.context != context {
            self.broker.discard(token, expected.kind, &expected.scope)?;
            return Err(EvidenceApprovalErrorV4::BindingMismatch);
        }
        Ok(preview)
    }

    pub fn commit(
        &self,
        root: &Path,
        application_id: &ApplicationId,
        token: &str,
        digest: &Sha256Digest,
        approved: bool,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<EvidenceCatalogRecord>, EvidenceApprovalErrorV4> {
        let (scope, context) = current_context(root, application_id)?;
        let grant = self
            .broker
            .take(token, ApprovalKind::EvidenceConfirmation, &scope)?;
        let preview = grant.payload().clone();
        let matches = grant.binding() == &binding(scope.clone(), &preview)
            && preview.context == context
            && preview.preview_sha256 == *digest;
        // All attempts consume this grant, including denial and failed revalidation.
        self.broker.resolve(grant, ApprovalDisposition::Consume)?;
        if !approved {
            return Err(EvidenceApprovalErrorV4::Denied);
        }
        if !matches {
            return Err(EvidenceApprovalErrorV4::BindingMismatch);
        }
        require_consent(preview.requires_private_read, consent)?;
        let mut workspace = crate::application::open_workspace_v4(&scope.workspace)?;
        let artifact = EvidenceService::new(&mut workspace.database, &workspace.blobs)
            .confirm_v4(
                &preview.profile_source,
                &preview.catalog,
                application_id,
                context.application_revision,
                &context.snapshot_sha256,
            )
            .map_err(ApplicationError::from)?;
        Ok(ActionReceipt::new(
            "evidence.confirm.commit",
            "confirmed",
            "Committed the exact approved Workspace Evidence catalog",
            preview.catalog,
        )
        .with_artifacts([artifact]))
    }
}

fn binding(scope: ApprovalScope, preview: &EvidencePreviewReadModelV4) -> ApprovalBinding {
    ApprovalBinding::new(
        ApprovalKind::EvidenceConfirmation,
        scope,
        Some(preview.context.application_id.to_string()),
        ApprovalSourceVersion::RevisionAndSnapshot {
            revision: preview.context.application_revision,
            snapshot_sha256: preview.preview_sha256.clone(),
        },
    )
}

fn current_context(
    root: &Path,
    application_id: &ApplicationId,
) -> Result<(ApprovalScope, ApplicationResourceContextV4), ApplicationError> {
    let status = Application::workspace_status_v4(root)?.data;
    let stored = Application::application_model_v4(&status.path, application_id.as_str())?.data;
    let context = ApplicationResourceContextV4 {
        application_id: application_id.clone(),
        pack: stored.snapshot.pack.clone(),
        application_revision: stored.snapshot.application.revision,
        snapshot_sha256: stored.snapshot_sha256,
    };
    Ok((
        ApprovalScope {
            workspace: status.path,
            workspace_id: status.status.workspace_id,
            pack: context.pack.clone(),
        },
        context,
    ))
}

fn require_consent(
    required: bool,
    consent: Option<PrivateReadConsent>,
) -> Result<(), ApplicationError> {
    if required && consent.is_none() {
        return Err(ApplicationError::ConsentRequired {
            message: "Evidence confirmation reads the selected private Profile Source".to_owned(),
            remediation: NextAction {
                action: "grant private read consent".to_owned(),
                description: "Review the selected Profile Source and approve its use".to_owned(),
            },
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ApplicationFlowCreateRequestV4, GENERIC_APPLICATION_WORKFLOW_PACK_ID};
    use canisend_contracts::{
        ActorKind, EvidenceKind, EvidenceProposalRecord, ProfileSourceKind, RequirementPriorityV3,
        Revision, SourceTextSpan, WorkflowPackId, WorkflowPackItemId,
    };
    use canisend_store::{
        ApplicationFlowCreateRequestV3, ApplicationFlowRequirementDraftV3, NewProfileSource,
        Workspace,
    };

    fn application(root: &Path) -> ApplicationId {
        Application::create_application_flow_v4(
            root,
            ApplicationFlowCreateRequestV4 {
                pack_id: WorkflowPackId::try_new(GENERIC_APPLICATION_WORKFLOW_PACK_ID).unwrap(),
                application: ApplicationFlowCreateRequestV3 {
                    title: "Synthetic Evidence fixture".to_owned(),
                    opportunity_metadata: Default::default(),
                    application_metadata: Default::default(),
                    source_text: "Provide a narrative.".to_owned(),
                    requirements: vec![ApplicationFlowRequirementDraftV3 {
                        category: WorkflowPackItemId::try_new("format").unwrap(),
                        statement: "Provide a narrative.".to_owned(),
                        priority: RequirementPriorityV3::Mandatory,
                        start_byte: 0,
                        end_byte: 20,
                    }],
                },
            },
        )
        .unwrap()
        .data
        .stored
        .snapshot
        .application
        .id
    }

    fn import_source(root: &Path) -> (ContentRevisionReferenceV3, EvidenceProposalSet) {
        let mut workspace = Workspace::open_v4(Some(root)).unwrap();
        let mut profile = ProfileService::new(&mut workspace.database, &workspace.blobs);
        let text = "Synthetic fixture person coordinated a local project.";
        let source = profile
            .import_source(
                NewProfileSource {
                    kind: ProfileSourceKind::PlainText,
                    original_bytes: text.as_bytes().to_vec(),
                    normalized_text: text.to_owned(),
                    content_type: "text/plain".to_owned(),
                    sensitivity: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .unwrap();
        let revision = Revision::try_new(profile.revision().unwrap()).unwrap();
        (
            ContentRevisionReferenceV3 {
                id: source.id,
                revision: source.revision,
                sha256: source.original.sha256,
            },
            EvidenceProposalSet {
                profile_revision: revision,
                proposals: vec![EvidenceProposalRecord {
                    kind: EvidenceKind::Other,
                    summary: text.to_owned(),
                    source_quote: text.to_owned(),
                    source_span: SourceTextSpan {
                        source: source.normalized_text,
                        start_byte: 0,
                        end_byte: text.len() as u64,
                    },
                    sensitivity: PrivacyClassification::PrivateLocal,
                }],
            },
        )
    }

    #[test]
    fn source_bound_confirmation_is_exact_private_atomic_and_single_use() {
        let root = std::env::temp_dir().join(format!(
            "canisend-evidence-v4-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        Application::initialize_workspace_v4(&root).unwrap();
        let id = application(&root);
        let other_id = application(&root);
        let (source, proposals) = import_source(&root);
        let broker = EvidenceApprovalBrokerV4::default();
        let consent = || Some(PrivateReadConsent::granted_by_user());
        assert!(
            broker
                .preview(&root, &id, source.clone(), proposals.clone(), None)
                .is_err()
        );
        for field in ["quote", "range", "privacy", "digest", "profile-revision"] {
            let mut wrong = proposals.clone();
            match field {
                "quote" => wrong.proposals[0].source_quote = "Unsupported claim".to_owned(),
                "range" => wrong.proposals[0].source_span.end_byte = u64::MAX,
                "privacy" => wrong.proposals[0].sensitivity = PrivacyClassification::Public,
                "digest" => {
                    wrong.proposals[0].source_span.source.sha256 =
                        Sha256Digest::try_new("f".repeat(64)).unwrap()
                }
                _ => {
                    wrong.profile_revision =
                        Revision::try_new(proposals.profile_revision.get() + 1).unwrap()
                }
            }
            assert!(
                broker
                    .preview(&root, &id, source.clone(), wrong, consent())
                    .is_err(),
                "{field}"
            );
        }
        let mut wrong_source = source.clone();
        wrong_source.sha256 = Sha256Digest::try_new("f".repeat(64)).unwrap();
        assert!(
            broker
                .preview(&root, &id, wrong_source, proposals.clone(), consent())
                .is_err()
        );
        for failure in ["denied", "private", "digest", "application"] {
            let lease = broker
                .preview(&root, &id, source.clone(), proposals.clone(), consent())
                .unwrap();
            let digest = if failure == "digest" {
                Sha256Digest::try_new("f".repeat(64)).unwrap()
            } else {
                lease.preview.data.preview_sha256.clone()
            };
            let target = if failure == "application" {
                &other_id
            } else {
                &id
            };
            assert!(
                broker
                    .commit(
                        &root,
                        target,
                        &lease.preview_token,
                        &digest,
                        failure != "denied",
                        if failure == "private" {
                            None
                        } else {
                            consent()
                        }
                    )
                    .is_err()
            );
            assert!(
                broker
                    .commit(
                        &root,
                        &id,
                        &lease.preview_token,
                        &lease.preview.data.preview_sha256,
                        true,
                        consent()
                    )
                    .is_err()
            );
            assert!(
                Application::list_evidence_associations_v4(&root, id.as_str())
                    .unwrap()
                    .data
                    .evidence
                    .is_empty()
            );
        }
        let stale = broker
            .preview(&root, &id, source.clone(), proposals.clone(), consent())
            .unwrap();
        let (new_source, new_proposals) = import_source(&root);
        assert!(
            broker
                .commit(
                    &root,
                    &id,
                    &stale.preview_token,
                    &stale.preview.data.preview_sha256,
                    true,
                    consent()
                )
                .is_err()
        );
        assert!(
            Application::list_evidence_associations_v4(&root, id.as_str())
                .unwrap()
                .data
                .evidence
                .is_empty()
        );
        let lease = broker
            .preview(&root, &id, new_source, new_proposals, consent())
            .unwrap();
        assert!(
            EvidenceApprovalBrokerV4::default()
                .confirmation_preview(
                    &root,
                    &id,
                    &lease.preview_token,
                    &lease.preview.data.preview_sha256
                )
                .is_err()
        );
        let exact = broker
            .confirmation_preview(
                &root,
                &id,
                &lease.preview_token,
                &lease.preview.data.preview_sha256,
            )
            .unwrap();
        assert_eq!(exact, lease.preview.data);
        let result = broker
            .commit(
                &root,
                &id,
                &lease.preview_token,
                &exact.preview_sha256,
                true,
                consent(),
            )
            .unwrap();
        assert_eq!(result.data, exact.catalog);
        let listed = Application::list_evidence_associations_v4(&root, id.as_str())
            .unwrap()
            .data;
        assert_eq!(listed.evidence.len(), 1);
        assert!(listed.associations.is_empty());
        assert_eq!(listed.evidence[0].evidence.id, exact.catalog.items[0].id);
        assert!(
            broker
                .commit(
                    &root,
                    &id,
                    &lease.preview_token,
                    &exact.preview_sha256,
                    true,
                    consent()
                )
                .is_err()
        );
        assert_eq!(
            Application::application_model_v4(&root, id.as_str())
                .unwrap()
                .data
                .snapshot
                .application
                .revision
                .get(),
            1
        );
        assert!(
            Application::check_workspace_v4(&root)
                .unwrap()
                .data
                .check
                .ok
        );
        drop(broker);
        std::fs::remove_dir_all(root).unwrap();
    }
}
