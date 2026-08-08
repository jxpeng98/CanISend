use std::{collections::BTreeMap, path::Path};

use canisend_contracts::{
    APPLICATION_MODEL_V3_MAX_REQUIREMENTS, ApplicationFieldValueV3, RequirementPriorityV3,
    Sha256Digest, WorkflowPackId, WorkflowPackItemId,
};
use canisend_store::{
    ApplicationFlowCreateRequestV3, ApplicationFlowReadModelV3, ApplicationFlowRequirementDraftV3,
    MAX_APPLICATION_FLOW_SOURCE_BYTES_V3, validate_application_flow_create_request,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ActionReceipt, Application, ApplicationError, ApplicationFlowCreateRequestV4,
    application_flow_v3::requested_built_in_pack,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PastedTextIntakePreviewRequestV4 {
    pub pack_id: WorkflowPackId,
    pub title: String,
    pub opportunity_metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    pub application_metadata: BTreeMap<WorkflowPackItemId, ApplicationFieldValueV3>,
    pub source_text: String,
    pub requirement_category: WorkflowPackItemId,
    pub requirement_priority: RequirementPriorityV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PastedTextIntakePreviewReadModelV4 {
    pub preview_sha256: Sha256Digest,
    pub source_sha256: Sha256Digest,
    pub normalized_text_bytes: u64,
    pub normalized_lines: u64,
    pub requirement_count: u64,
    pub application: ApplicationFlowCreateRequestV3,
    pub submission_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PastedTextIntakeCommitRequestV4 {
    pub preview: PastedTextIntakePreviewRequestV4,
    pub expected_preview_sha256: Sha256Digest,
}

impl Application {
    pub fn preview_pasted_text_intake_v4(
        workspace_root: &Path,
        request: PastedTextIntakePreviewRequestV4,
    ) -> Result<ActionReceipt<PastedTextIntakePreviewReadModelV4>, ApplicationError> {
        Self::workspace_status_v4(workspace_root)?;
        let pack = requested_built_in_pack(&request.pack_id)?;
        if !pack
            .manifest()
            .requirements
            .categories
            .iter()
            .any(|category| category.id == request.requirement_category)
        {
            return Err(ApplicationError::InvalidInput(format!(
                "Requirement category {} is not declared by Pack {}",
                request.requirement_category, request.pack_id
            )));
        }
        validate_pasted_text_request(&request)?;
        let requirements = requirement_lines(
            &request.source_text,
            &request.requirement_category,
            request.requirement_priority,
        )?;
        let source_sha256 = digest(request.source_text.as_bytes())?;
        let normalized_lines = u64::try_from(request.source_text.lines().count())
            .map_err(|_| ApplicationError::InvalidInput("Source line count overflow".to_owned()))?;
        let application = ApplicationFlowCreateRequestV3 {
            title: request.title,
            opportunity_metadata: request.opportunity_metadata,
            application_metadata: request.application_metadata,
            source_text: request.source_text,
            requirements,
        };
        validate_application_flow_create_request(&pack, &application)?;
        let preview_bytes = serde_json::to_vec(&(
            "canisend.pasted-text-intake-preview/v4",
            &request.pack_id,
            &application,
        ))
        .map_err(|error| {
            ApplicationError::InvalidInput(format!("could not encode intake preview: {error}"))
        })?;
        let preview_sha256 = digest(&preview_bytes)?;
        let normalized_text_bytes = u64::try_from(application.source_text.len())
            .map_err(|_| ApplicationError::InvalidInput("Source byte count overflow".to_owned()))?;
        let requirement_count = u64::try_from(application.requirements.len())
            .map_err(|_| ApplicationError::InvalidInput("Requirement count overflow".to_owned()))?;
        Ok(ActionReceipt::new(
            "application.intake.pasted-text.preview",
            "previewed",
            "Prepared exact Source spans and proposed Requirements without Workspace mutation",
            PastedTextIntakePreviewReadModelV4 {
                preview_sha256,
                source_sha256,
                normalized_text_bytes,
                normalized_lines,
                requirement_count,
                application,
                submission_performed: false,
            },
        ))
    }

    pub fn commit_pasted_text_intake_v4(
        workspace_root: &Path,
        request: PastedTextIntakeCommitRequestV4,
    ) -> Result<ActionReceipt<ApplicationFlowReadModelV3>, ApplicationError> {
        let pack_id = request.preview.pack_id.clone();
        let preview = Self::preview_pasted_text_intake_v4(workspace_root, request.preview)?.data;
        if preview.preview_sha256 != request.expected_preview_sha256 {
            return Err(ApplicationError::InvalidInput(
                "pasted-text intake preview digest is stale or does not match the reviewed bytes"
                    .to_owned(),
            ));
        }
        let committed = Self::create_application_flow_v4(
            workspace_root,
            ApplicationFlowCreateRequestV4 {
                pack_id,
                application: preview.application,
            },
        )?;
        Ok(ActionReceipt::new(
            "application.intake.pasted-text.commit",
            "created",
            "Committed the reviewed Source, explicit Application link, and proposed Requirements",
            committed.data,
        ))
    }
}

fn validate_pasted_text_request(
    request: &PastedTextIntakePreviewRequestV4,
) -> Result<(), ApplicationError> {
    if request.title.trim().is_empty() || request.title.len() > 512 {
        return Err(ApplicationError::InvalidInput(
            "Application title must contain 1 to 512 bytes".to_owned(),
        ));
    }
    if request.source_text.len() > MAX_APPLICATION_FLOW_SOURCE_BYTES_V3 {
        return Err(ApplicationError::InvalidInput(format!(
            "pasted Source exceeds the {MAX_APPLICATION_FLOW_SOURCE_BYTES_V3}-byte limit"
        )));
    }
    if request.source_text.trim().is_empty() {
        return Err(ApplicationError::InvalidInput(
            "pasted Source text cannot be empty".to_owned(),
        ));
    }
    Ok(())
}

fn requirement_lines(
    source: &str,
    category: &WorkflowPackItemId,
    priority: RequirementPriorityV3,
) -> Result<Vec<ApplicationFlowRequirementDraftV3>, ApplicationError> {
    let mut requirements = Vec::new();
    let mut offset = 0usize;
    for line in source.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let content = content.strip_suffix('\r').unwrap_or(content);
        let statement = content.trim();
        if !statement.is_empty() {
            let relative_start = content.find(statement).ok_or_else(|| {
                ApplicationError::InvalidInput("could not locate exact Requirement span".to_owned())
            })?;
            let start = offset.checked_add(relative_start).ok_or_else(|| {
                ApplicationError::InvalidInput("Requirement span overflow".to_owned())
            })?;
            let end = start.checked_add(statement.len()).ok_or_else(|| {
                ApplicationError::InvalidInput("Requirement span overflow".to_owned())
            })?;
            requirements.push(ApplicationFlowRequirementDraftV3 {
                category: category.clone(),
                statement: statement.to_owned(),
                priority,
                start_byte: u64::try_from(start).map_err(|_| {
                    ApplicationError::InvalidInput("Requirement span overflow".to_owned())
                })?,
                end_byte: u64::try_from(end).map_err(|_| {
                    ApplicationError::InvalidInput("Requirement span overflow".to_owned())
                })?,
            });
        }
        offset = offset.checked_add(line.len()).ok_or_else(|| {
            ApplicationError::InvalidInput("Source byte offset overflow".to_owned())
        })?;
        if requirements.len() > APPLICATION_MODEL_V3_MAX_REQUIREMENTS {
            return Err(ApplicationError::InvalidInput(format!(
                "pasted Source proposes more than {APPLICATION_MODEL_V3_MAX_REQUIREMENTS} Requirements"
            )));
        }
    }
    if requirements.is_empty() {
        return Err(ApplicationError::InvalidInput(
            "pasted Source contains no non-empty Requirement lines".to_owned(),
        ));
    }
    Ok(requirements)
}

pub(super) fn digest(bytes: &[u8]) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::try_new(hex::encode(Sha256::digest(bytes)))
        .map_err(|error| ApplicationError::InvalidInput(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use canisend_contracts::{ApplicationFieldValueV3, PrivacyClassification, WorkflowPackItemId};
    use canisend_store::{ApplicationAssociationServiceV4, Workspace};

    use super::*;

    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-pasted-intake-v4-{label}-{}-{}",
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

    fn generic_request(source_text: &str) -> PastedTextIntakePreviewRequestV4 {
        PastedTextIntakePreviewRequestV4 {
            pack_id: WorkflowPackId::try_new(crate::GENERIC_APPLICATION_WORKFLOW_PACK_ID)
                .expect("Pack ID"),
            title: "Community programme".to_owned(),
            opportunity_metadata: BTreeMap::from([
                (
                    item("organization"),
                    ApplicationFieldValueV3::ShortText("Example Foundation".to_owned()),
                ),
                (
                    item("reference"),
                    ApplicationFieldValueV3::ShortText("INTAKE-V4-001".to_owned()),
                ),
            ]),
            application_metadata: BTreeMap::from([(
                item("status"),
                ApplicationFieldValueV3::Choice(item("planning")),
            )]),
            source_text: source_text.to_owned(),
            requirement_category: item("format"),
            requirement_priority: RequirementPriorityV3::Mandatory,
        }
    }

    fn academic_request(source_text: &str) -> PastedTextIntakePreviewRequestV4 {
        PastedTextIntakePreviewRequestV4 {
            pack_id: WorkflowPackId::try_new(crate::ACADEMIC_JOB_WORKFLOW_PACK_ID)
                .expect("Pack ID"),
            title: "Research fellowship".to_owned(),
            opportunity_metadata: BTreeMap::from([(
                item("institution"),
                ApplicationFieldValueV3::ShortText("Example University".to_owned()),
            )]),
            application_metadata: BTreeMap::new(),
            source_text: source_text.to_owned(),
            requirement_category: item("qualification"),
            requirement_priority: RequirementPriorityV3::Mandatory,
        }
    }

    #[test]
    fn preview_preserves_exact_utf8_spans_without_workspace_mutation() {
        let root = root("preview");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let source = "  First requirement  \r\n\r\nSecond requirement";
        let preview = Application::preview_pasted_text_intake_v4(&root, generic_request(source))
            .expect("preview")
            .data;

        assert_eq!(preview.requirement_count, 2);
        assert_eq!(
            preview.application.requirements[0].statement,
            "First requirement"
        );
        assert_eq!(
            preview.application.requirements[1].statement,
            "Second requirement"
        );
        for requirement in &preview.application.requirements {
            let start = usize::try_from(requirement.start_byte).expect("start");
            let end = usize::try_from(requirement.end_byte).expect("end");
            assert_eq!(&source[start..end], requirement.statement);
        }
        assert!(
            Application::list_application_models_v3(&root)
                .expect("Applications after preview")
                .data
                .is_empty()
        );
        let workspace = Workspace::open_from(Some(&root), &root).expect("open Workspace");
        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn exact_preview_commit_creates_connected_sources_for_both_packs() {
        let root = root("mixed-commit");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let generic_request = generic_request("Narrative required.\nBudget required.");
        let generic_preview =
            Application::preview_pasted_text_intake_v4(&root, generic_request.clone())
                .expect("generic preview")
                .data;
        let generic = Application::commit_pasted_text_intake_v4(
            &root,
            PastedTextIntakeCommitRequestV4 {
                preview: generic_request,
                expected_preview_sha256: generic_preview.preview_sha256,
            },
        )
        .expect("generic commit")
        .data;

        let academic_request = academic_request("Doctorate required.\nResearch plan required.");
        let academic_preview =
            Application::preview_pasted_text_intake_v4(&root, academic_request.clone())
                .expect("academic preview")
                .data;
        let academic = Application::commit_pasted_text_intake_v4(
            &root,
            PastedTextIntakeCommitRequestV4 {
                preview: academic_request,
                expected_preview_sha256: academic_preview.preview_sha256,
            },
        )
        .expect("academic commit")
        .data;

        assert_eq!(generic.stored.snapshot.requirements.len(), 2);
        assert_eq!(academic.stored.snapshot.requirements.len(), 2);
        let mut workspace = Workspace::open_from(Some(&root), &root).expect("open Workspace");
        let associations =
            ApplicationAssociationServiceV4::new(&mut workspace.database, &workspace.blobs);
        for application in [&generic, &academic] {
            let links = associations
                .source_associations(&application.stored.snapshot.application.id)
                .expect("Source association");
            assert_eq!(links.len(), 1);
            let source = associations
                .source(&links[0].source.id, links[0].source.revision)
                .expect("Source revision");
            assert_eq!(source.privacy, PrivacyClassification::PrivateLocal);
            assert_eq!(source.normalized_sha256, links[0].source.sha256);
        }
        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn stale_digest_unknown_category_and_empty_text_fail_without_mutation() {
        let root = root("failures");
        Application::initialize_workspace_v4(&root).expect("Workspace v4");
        let request = generic_request("One exact requirement.");
        let preview = Application::preview_pasted_text_intake_v4(&root, request.clone())
            .expect("preview")
            .data;
        let wrong_digest = Sha256Digest::try_new("f".repeat(64)).expect("digest");
        assert_ne!(preview.preview_sha256, wrong_digest);
        let stale = Application::commit_pasted_text_intake_v4(
            &root,
            PastedTextIntakeCommitRequestV4 {
                preview: request,
                expected_preview_sha256: wrong_digest,
            },
        )
        .expect_err("wrong preview digest");
        assert!(matches!(stale, ApplicationError::InvalidInput(_)));

        let mut unknown_category = generic_request("One exact requirement.");
        unknown_category.requirement_category = item("not-declared");
        assert!(matches!(
            Application::preview_pasted_text_intake_v4(&root, unknown_category),
            Err(ApplicationError::InvalidInput(_))
        ));
        assert!(matches!(
            Application::preview_pasted_text_intake_v4(&root, generic_request(" \r\n\t")),
            Err(ApplicationError::InvalidInput(_))
        ));
        assert!(
            Application::list_application_models_v3(&root)
                .expect("Applications after failures")
                .data
                .is_empty()
        );
        let workspace = Workspace::open_from(Some(&root), &root).expect("open Workspace");
        drop(workspace);
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
