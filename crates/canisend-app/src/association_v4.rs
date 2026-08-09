use std::path::Path;

use canisend_contracts::{
    ActorKind, ApplicationEvidenceAssociationV4, ApplicationId, ApplicationProfileAssociationV4,
    ConsentScope, ContentRevisionReferenceV3, PrivacyClassification, ProfileSourceRecord, Revision,
    Sha256Digest, WorkspaceEvidenceSummaryV4,
};
use canisend_store::{ApplicationAssociationServiceV4, ApplicationModelRepository, ProfileService};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ActionReceipt, Application, ApplicationError, PrivateReadConsent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssociationChangeV4 {
    Associate,
    Unlink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileAssociationListReadModelV4 {
    pub application_id: ApplicationId,
    pub application_revision: Revision,
    pub profile_sources: Vec<ProfileSourceRecord>,
    pub associations: Vec<ApplicationProfileAssociationV4>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAssociationListReadModelV4 {
    pub application_id: ApplicationId,
    pub application_revision: Revision,
    pub evidence: Vec<WorkspaceEvidenceSummaryV4>,
    pub associations: Vec<ApplicationEvidenceAssociationV4>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileAssociationPreviewRequestV4 {
    pub application_id: ApplicationId,
    pub profile_source: ContentRevisionReferenceV3,
    pub change: AssociationChangeV4,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAssociationPreviewRequestV4 {
    pub application_id: ApplicationId,
    pub evidence: ContentRevisionReferenceV3,
    pub change: AssociationChangeV4,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileAssociationPreviewReadModelV4 {
    pub request: ProfileAssociationPreviewRequestV4,
    pub application_revision: Revision,
    pub requires_private_read: bool,
    pub preview_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAssociationPreviewReadModelV4 {
    pub request: EvidenceAssociationPreviewRequestV4,
    pub application_revision: Revision,
    pub requires_private_read: bool,
    pub preview_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileAssociationCommitRequestV4 {
    pub preview: ProfileAssociationPreviewRequestV4,
    pub expected_preview_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAssociationCommitRequestV4 {
    pub preview: EvidenceAssociationPreviewRequestV4,
    pub expected_preview_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileAssociationCommitReadModelV4 {
    pub change: AssociationChangeV4,
    pub association: Option<ApplicationProfileAssociationV4>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAssociationCommitReadModelV4 {
    pub change: AssociationChangeV4,
    pub association: Option<ApplicationEvidenceAssociationV4>,
}

impl Application {
    pub fn list_profile_associations_v4(
        root: &Path,
        application_id: &str,
    ) -> Result<ActionReceipt<ProfileAssociationListReadModelV4>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let mut workspace = crate::application::open_workspace_v4(root)?;
        let application_revision = current_application_revision(&mut workspace, &application_id)?;
        let profile_sources =
            ProfileService::new(&mut workspace.database, &workspace.blobs).list_sources()?;
        let associations =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs)
                .profile_associations(&application_id)?;
        Ok(ActionReceipt::new(
            "profile.association.list",
            "available",
            format!(
                "Loaded {} Workspace Profile Source(s) and {} explicit Application link(s)",
                profile_sources.len(),
                associations.len()
            ),
            ProfileAssociationListReadModelV4 {
                application_id,
                application_revision,
                profile_sources,
                associations,
            },
        ))
    }

    pub fn list_evidence_associations_v4(
        root: &Path,
        application_id: &str,
    ) -> Result<ActionReceipt<EvidenceAssociationListReadModelV4>, ApplicationError> {
        let application_id = parse_application_id(application_id)?;
        let mut workspace = crate::application::open_workspace_v4(root)?;
        let application_revision = current_application_revision(&mut workspace, &application_id)?;
        let service =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);
        let evidence = service.confirmed_evidence()?;
        let associations = service.evidence_associations(&application_id)?;
        Ok(ActionReceipt::new(
            "evidence.association.list",
            "available",
            format!(
                "Loaded {} confirmed Workspace Evidence item(s) and {} explicit Application link(s)",
                evidence.len(),
                associations.len()
            ),
            EvidenceAssociationListReadModelV4 {
                application_id,
                application_revision,
                evidence,
                associations,
            },
        ))
    }

    pub fn preview_profile_association_v4(
        root: &Path,
        request: ProfileAssociationPreviewRequestV4,
    ) -> Result<ActionReceipt<ProfileAssociationPreviewReadModelV4>, ApplicationError> {
        let mut workspace = crate::application::open_workspace_v4(root)?;
        let application_revision =
            current_application_revision(&mut workspace, &request.application_id)?;
        let associations =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs)
                .profile_associations(&request.application_id)?;
        validate_change(
            request.change,
            &request.profile_source,
            associations
                .iter()
                .map(|association| &association.profile_source),
            "Profile Source",
        )?;
        let requires_private_read = if request.change == AssociationChangeV4::Associate {
            let source = ProfileService::new(&mut workspace.database, &workspace.blobs)
                .get_source(&request.profile_source.id)?;
            require_exact_profile_source(&source, &request.profile_source)?;
            source.sensitivity != PrivacyClassification::Public
        } else {
            false
        };
        let preview_sha256 = preview_digest(&(
            "canisend.profile-association-preview/v4",
            &request,
            application_revision,
            requires_private_read,
        ))?;
        Ok(ActionReceipt::new(
            "profile.association.preview",
            "previewed",
            "Prepared an exact Profile Source association change without Workspace mutation",
            ProfileAssociationPreviewReadModelV4 {
                request,
                application_revision,
                requires_private_read,
                preview_sha256,
            },
        ))
    }

    pub fn commit_profile_association_v4(
        root: &Path,
        request: ProfileAssociationCommitRequestV4,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<ProfileAssociationCommitReadModelV4>, ApplicationError> {
        let preview = Self::preview_profile_association_v4(root, request.preview)?.data;
        if preview.preview_sha256 != request.expected_preview_sha256 {
            return Err(ApplicationError::InvalidInput(
                "Profile Source association preview is stale or differs from the reviewed change"
                    .to_owned(),
            ));
        }
        require_private_consent(preview.requires_private_read, consent)?;
        let mut workspace = crate::application::open_workspace_v4(root)?;
        let mut service =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);
        let association = match preview.request.change {
            AssociationChangeV4::Associate => Some(service.associate_profile_source(
                &preview.request.application_id,
                &preview.request.profile_source,
                consent_scope(preview.requires_private_read),
                ActorKind::User,
            )?),
            AssociationChangeV4::Unlink => {
                service.unlink_profile_source(
                    &preview.request.application_id,
                    &preview.request.profile_source.id,
                    ActorKind::User,
                )?;
                None
            }
        };
        Ok(ActionReceipt::new(
            "profile.association.commit",
            "committed",
            "Committed the reviewed explicit Profile Source association change",
            ProfileAssociationCommitReadModelV4 {
                change: preview.request.change,
                association,
            },
        ))
    }

    pub fn preview_evidence_association_v4(
        root: &Path,
        request: EvidenceAssociationPreviewRequestV4,
    ) -> Result<ActionReceipt<EvidenceAssociationPreviewReadModelV4>, ApplicationError> {
        let mut workspace = crate::application::open_workspace_v4(root)?;
        let application_revision =
            current_application_revision(&mut workspace, &request.application_id)?;
        let service =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);
        let associations = service.evidence_associations(&request.application_id)?;
        validate_change(
            request.change,
            &request.evidence,
            associations.iter().map(|association| &association.evidence),
            "Evidence",
        )?;
        let requires_private_read = if request.change == AssociationChangeV4::Associate {
            let evidence = service
                .confirmed_evidence()?
                .into_iter()
                .find(|item| item.evidence.id == request.evidence.id)
                .ok_or_else(|| {
                    ApplicationError::InvalidInput(
                        "Evidence is not a current confirmed, non-excluded Workspace item"
                            .to_owned(),
                    )
                })?;
            require_exact_reference(&evidence.evidence, &request.evidence, "Evidence")?;
            evidence.sensitivity != PrivacyClassification::Public
        } else {
            service.evidence_revision_summary(&request.evidence)?;
            false
        };
        let preview_sha256 = preview_digest(&(
            "canisend.evidence-association-preview/v4",
            &request,
            application_revision,
            requires_private_read,
        ))?;
        Ok(ActionReceipt::new(
            "evidence.association.preview",
            "previewed",
            "Prepared an exact Evidence association change without Workspace mutation",
            EvidenceAssociationPreviewReadModelV4 {
                request,
                application_revision,
                requires_private_read,
                preview_sha256,
            },
        ))
    }

    pub fn commit_evidence_association_v4(
        root: &Path,
        request: EvidenceAssociationCommitRequestV4,
        consent: Option<PrivateReadConsent>,
    ) -> Result<ActionReceipt<EvidenceAssociationCommitReadModelV4>, ApplicationError> {
        let preview = Self::preview_evidence_association_v4(root, request.preview)?.data;
        if preview.preview_sha256 != request.expected_preview_sha256 {
            return Err(ApplicationError::InvalidInput(
                "Evidence association preview is stale or differs from the reviewed change"
                    .to_owned(),
            ));
        }
        require_private_consent(preview.requires_private_read, consent)?;
        let mut workspace = crate::application::open_workspace_v4(root)?;
        let mut service =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);
        let association = match preview.request.change {
            AssociationChangeV4::Associate => Some(service.associate_evidence(
                &preview.request.application_id,
                &preview.request.evidence,
                consent_scope(preview.requires_private_read),
                ActorKind::User,
            )?),
            AssociationChangeV4::Unlink => {
                service.unlink_evidence(
                    &preview.request.application_id,
                    &preview.request.evidence.id,
                    ActorKind::User,
                )?;
                None
            }
        };
        Ok(ActionReceipt::new(
            "evidence.association.commit",
            "committed",
            "Committed the reviewed explicit Evidence association change",
            EvidenceAssociationCommitReadModelV4 {
                change: preview.request.change,
                association,
            },
        ))
    }
}

fn current_application_revision(
    workspace: &mut canisend_store::Workspace,
    application_id: &ApplicationId,
) -> Result<Revision, ApplicationError> {
    Ok(ApplicationModelRepository::new(&mut workspace.database)
        .get(application_id)?
        .snapshot
        .application
        .revision)
}

fn parse_application_id(value: &str) -> Result<ApplicationId, ApplicationError> {
    ApplicationId::try_new(value)
        .map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))
}

fn require_exact_profile_source(
    source: &ProfileSourceRecord,
    selected: &ContentRevisionReferenceV3,
) -> Result<(), ApplicationError> {
    let current = ContentRevisionReferenceV3 {
        id: source.id.clone(),
        revision: source.revision,
        sha256: source.original.sha256.clone(),
    };
    require_exact_reference(&current, selected, "Profile Source")
}

fn require_exact_reference(
    current: &ContentRevisionReferenceV3,
    selected: &ContentRevisionReferenceV3,
    label: &str,
) -> Result<(), ApplicationError> {
    if current != selected {
        return Err(ApplicationError::InvalidInput(format!(
            "{label} selection is stale or its exact revision digest does not match"
        )));
    }
    Ok(())
}

fn validate_change<'a>(
    change: AssociationChangeV4,
    selected: &ContentRevisionReferenceV3,
    mut current: impl Iterator<Item = &'a ContentRevisionReferenceV3>,
    label: &str,
) -> Result<(), ApplicationError> {
    let existing = current.find(|reference| reference.id == selected.id);
    match (change, existing) {
        (AssociationChangeV4::Associate, Some(_)) => Err(ApplicationError::InvalidInput(format!(
            "{label} is already associated; unlink its current revision before selecting another"
        ))),
        (AssociationChangeV4::Unlink, Some(reference)) if reference == selected => Ok(()),
        (AssociationChangeV4::Unlink, Some(_)) => Err(ApplicationError::InvalidInput(format!(
            "{label} unlink selection is stale"
        ))),
        (AssociationChangeV4::Unlink, None) => Err(ApplicationError::InvalidInput(format!(
            "{label} is not associated with this Application"
        ))),
        (AssociationChangeV4::Associate, None) => Ok(()),
    }
}

fn require_private_consent(
    required: bool,
    consent: Option<PrivateReadConsent>,
) -> Result<(), ApplicationError> {
    if required && consent.is_none() {
        return Err(ApplicationError::ConsentRequired {
            message: "The selected association reads private Workspace input".to_owned(),
            remediation: canisend_contracts::NextAction {
                action: "grant private read consent".to_owned(),
                description:
                    "Review the exact Profile Source or Evidence revision and explicitly authorize its use in this Application"
                        .to_owned(),
            },
        });
    }
    Ok(())
}

fn consent_scope(required: bool) -> Option<ConsentScope> {
    required.then_some(ConsentScope::ReadPrivateInputs)
}

fn preview_digest(value: &impl Serialize) -> Result<Sha256Digest, ApplicationError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        ApplicationError::InvalidInput(format!("could not encode association preview: {error}"))
    })?;
    Sha256Digest::try_new(hex::encode(Sha256::digest(bytes)))
        .map_err(|error| ApplicationError::InvalidInput(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::{
        ApplicationId, PrivacyClassification, ProfileSourceKind, Sha256Digest,
    };
    use canisend_store::{
        ApplicationFlowCreateRequestV3, ApplicationFlowRequirementDraftV3, NewProfileSource,
        ProfileService,
    };

    use super::*;
    use crate::{ApplicationFlowCreateRequestV4, GENERIC_APPLICATION_WORKFLOW_PACK_ID};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-association-v4-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn application(root: &Path) -> ApplicationId {
        Application::create_application_flow_v4(
            root,
            ApplicationFlowCreateRequestV4 {
                pack_id: canisend_contracts::WorkflowPackId::try_new(
                    GENERIC_APPLICATION_WORKFLOW_PACK_ID,
                )
                .expect("Pack ID"),
                application: ApplicationFlowCreateRequestV3 {
                    title: "Association fixture".to_owned(),
                    opportunity_metadata: Default::default(),
                    application_metadata: Default::default(),
                    source_text: "Provide a narrative.".to_owned(),
                    requirements: vec![ApplicationFlowRequirementDraftV3 {
                        category: canisend_contracts::WorkflowPackItemId::try_new("format")
                            .expect("category"),
                        statement: "Provide a narrative.".to_owned(),
                        priority: canisend_contracts::RequirementPriorityV3::Mandatory,
                        start_byte: 0,
                        end_byte: 20,
                    }],
                },
            },
        )
        .expect("Application")
        .data
        .stored
        .snapshot
        .application
        .id
    }

    #[test]
    fn profile_association_preview_commit_is_exact_consent_bound_and_body_free() {
        let root = root("profile");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let application_id = application(&root);
        let mut workspace = canisend_store::Workspace::open_v4(Some(&root)).expect("Workspace");
        let source = ProfileService::new(&mut workspace.database, &workspace.blobs)
            .import_source(
                NewProfileSource {
                    kind: ProfileSourceKind::PlainText,
                    original_bytes: b"PRIVATE-PROFILE-BODY".to_vec(),
                    normalized_text: "PRIVATE-PROFILE-BODY".to_owned(),
                    content_type: "text/plain".to_owned(),
                    sensitivity: PrivacyClassification::PrivateLocal,
                },
                ActorKind::User,
            )
            .expect("Profile Source");
        drop(workspace);
        let selection = ContentRevisionReferenceV3 {
            id: source.id,
            revision: source.revision,
            sha256: source.original.sha256,
        };
        let request = ProfileAssociationPreviewRequestV4 {
            application_id: application_id.clone(),
            profile_source: selection,
            change: AssociationChangeV4::Associate,
        };
        let preview = Application::preview_profile_association_v4(&root, request.clone())
            .expect("preview")
            .data;
        assert!(preview.requires_private_read);
        assert!(matches!(
            Application::commit_profile_association_v4(
                &root,
                ProfileAssociationCommitRequestV4 {
                    preview: request.clone(),
                    expected_preview_sha256: preview.preview_sha256.clone(),
                },
                None,
            ),
            Err(ApplicationError::ConsentRequired { .. })
        ));
        Application::commit_profile_association_v4(
            &root,
            ProfileAssociationCommitRequestV4 {
                preview: request,
                expected_preview_sha256: preview.preview_sha256,
            },
            Some(PrivateReadConsent::granted_by_user()),
        )
        .expect("commit");
        let listed = Application::list_profile_associations_v4(&root, application_id.as_str())
            .expect("list");
        assert_eq!(listed.data.associations.len(), 1);
        assert!(
            !serde_json::to_string(&listed)
                .expect("receipt")
                .contains("PRIVATE-PROFILE-BODY")
        );
        let unlink_request = ProfileAssociationPreviewRequestV4 {
            application_id: application_id.clone(),
            profile_source: listed.data.associations[0].profile_source.clone(),
            change: AssociationChangeV4::Unlink,
        };
        let unlink_preview =
            Application::preview_profile_association_v4(&root, unlink_request.clone())
                .expect("unlink preview")
                .data;
        assert!(!unlink_preview.requires_private_read);
        Application::commit_profile_association_v4(
            &root,
            ProfileAssociationCommitRequestV4 {
                preview: unlink_request,
                expected_preview_sha256: unlink_preview.preview_sha256,
            },
            None,
        )
        .expect("unlink without rereading private body");
        assert!(
            Application::list_profile_associations_v4(&root, application_id.as_str())
                .expect("list after unlink")
                .data
                .associations
                .is_empty()
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn stale_evidence_preview_digest_fails_without_mutation() {
        let root = root("evidence");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let application_id = application(&root);
        let request = EvidenceAssociationPreviewRequestV4 {
            application_id: application_id.clone(),
            evidence: ContentRevisionReferenceV3 {
                id: application_id.as_entity_id().clone(),
                revision: Revision::try_new(1).expect("revision"),
                sha256: Sha256Digest::try_new("0".repeat(64)).expect("digest"),
            },
            change: AssociationChangeV4::Associate,
        };
        assert!(Application::preview_evidence_association_v4(&root, request).is_err());
        assert!(
            Application::list_evidence_associations_v4(&root, application_id.as_str())
                .expect("list")
                .data
                .associations
                .is_empty()
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
