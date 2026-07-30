use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use canisend_contracts::{
    ActorKind, ArtifactKind, ArtifactReference, NextAction, PrivacyClassification,
    StageExecutionStatus, WorkflowStage,
};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, AgentContextReadModel, Application, ApplicationDossierReadModel,
    ApplicationError, ContentCatalogEntryReadModel, ContentCatalogFilter, ContentCatalogStatus,
    ContentCategory, ContentSourceRole, ContentSourceScope,
};

const MAX_CONTENT_REFERENCES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentWorkspaceSection {
    Overview,
    JobCriteria,
    EvidenceFit,
    Materials,
    ReviewExport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentProposalKind {
    Criteria,
    Evidence,
    Matches,
    Plan,
    Draft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentProposalState {
    Blocked,
    Ready,
    Proposed,
    Current,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentProposalCommitBoundary {
    UserConfirmation,
    TaskPreviewCommit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentContentProvenanceReadModel {
    pub actor: ActorKind,
    pub reason: String,
    pub source_id: Option<canisend_contracts::EntityId>,
    pub source_scope: Option<ContentSourceScope>,
    pub source_role: Option<ContentSourceRole>,
    pub source_kind: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentContentReferenceReadModel {
    pub artifact: ArtifactReference,
    pub title: String,
    pub category: ContentCategory,
    pub stage: WorkflowStage,
    pub status: ContentCatalogStatus,
    pub privacy: PrivacyClassification,
    pub provenance: AgentContentProvenanceReadModel,
    pub relationships: Vec<ArtifactReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentContentGraphReadModel {
    pub total_entries: u64,
    pub entries: Vec<AgentContentReferenceReadModel>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentProposalTargetReadModel {
    pub kind: AgentProposalKind,
    pub stage: WorkflowStage,
    pub section: AgentWorkspaceSection,
    pub state: AgentProposalState,
    pub operation: String,
    pub current_artifacts: Vec<ArtifactReference>,
    pub upstream_artifacts: Vec<ArtifactReference>,
    pub validation_rules: Vec<String>,
    pub intended_mutation: String,
    pub commit_boundary: AgentProposalCommitBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRecommendationReadModel {
    pub skill_id: String,
    pub section: AgentWorkspaceSection,
    pub reason: String,
    pub next_action: Option<NextAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentExecutionBoundaryReadModel {
    pub recommended_integration: String,
    pub session_authority: String,
    pub state_authority: String,
    pub in_app_runtime: String,
    pub transcript_persisted_by_canisend: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentAssistanceReadModel {
    pub workspace: PathBuf,
    pub selected_job_id: String,
    pub dossier: ApplicationDossierReadModel,
    pub context: AgentContextReadModel,
    pub content: AgentContentGraphReadModel,
    pub recommendation: AgentRecommendationReadModel,
    pub proposal_targets: Vec<AgentProposalTargetReadModel>,
    pub execution_boundary: AgentExecutionBoundaryReadModel,
    pub privacy: PrivacyClassification,
}

impl Application {
    pub fn agent_assistance(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<AgentAssistanceReadModel>, ApplicationError> {
        let dossier = Self::application_dossier(root, job_id)?.data;
        let context = Self::agent_context(Some(root), Some(job_id))?.data;
        let catalog = Self::content_catalog(
            root,
            ContentCatalogFilter {
                job_id: Some(job_id.to_owned()),
                ..ContentCatalogFilter::default()
            },
        )?
        .data;
        let recommendation = recommendation(&dossier);
        let proposal_targets = proposal_targets(&dossier, &catalog.entries);
        let entries = catalog
            .entries
            .iter()
            .take(MAX_CONTENT_REFERENCES)
            .map(project_content_reference)
            .collect::<Vec<_>>();
        let data = AgentAssistanceReadModel {
            workspace: dossier.workspace.clone(),
            selected_job_id: dossier.job.id.as_str().to_owned(),
            dossier,
            context,
            content: AgentContentGraphReadModel {
                total_entries: catalog.total_entries,
                truncated: catalog.total_entries > entries.len() as u64,
                entries,
            },
            recommendation,
            proposal_targets,
            execution_boundary: AgentExecutionBoundaryReadModel {
                recommended_integration: "external-host".to_owned(),
                session_authority: "external-agent-host".to_owned(),
                state_authority: "canisend".to_owned(),
                in_app_runtime: "optional-read-only".to_owned(),
                transcript_persisted_by_canisend: false,
            },
            privacy: PrivacyClassification::Public,
        };
        let next_actions: Vec<_> = data
            .recommendation
            .next_action
            .clone()
            .into_iter()
            .collect();
        Ok(ActionReceipt::new(
            "agent.assistance",
            "available",
            format!(
                "Prepared body-free contextual assistance for {}",
                data.dossier.job.title
            ),
            data,
        )
        .with_next_actions(next_actions))
    }
}

fn recommendation(dossier: &ApplicationDossierReadModel) -> AgentRecommendationReadModel {
    let (skill_id, section, reason) = match dossier.current_stage {
        Some(WorkflowStage::Intake | WorkflowStage::Parse | WorkflowStage::Criteria) => (
            "canisend-job-intake",
            AgentWorkspaceSection::JobCriteria,
            "The application still needs source interpretation or reviewed selection criteria.",
        ),
        Some(
            WorkflowStage::Evidence
            | WorkflowStage::Match
            | WorkflowStage::Plan
            | WorkflowStage::Draft,
        ) => (
            "canisend-application-materials",
            if dossier.current_stage == Some(WorkflowStage::Draft) {
                AgentWorkspaceSection::Materials
            } else {
                AgentWorkspaceSection::EvidenceFit
            },
            "The next bounded work converts confirmed evidence and fit decisions into application materials.",
        ),
        Some(WorkflowStage::Review | WorkflowStage::Package | WorkflowStage::Render) => (
            "canisend-application-review",
            AgentWorkspaceSection::ReviewExport,
            "The application is in validation, packaging, or export preparation.",
        ),
        None => (
            "canisend-application",
            AgentWorkspaceSection::Overview,
            "No narrower unfinished workflow stage is available.",
        ),
    };
    AgentRecommendationReadModel {
        skill_id: skill_id.to_owned(),
        section,
        reason: reason.to_owned(),
        next_action: dossier.next_actions.first().cloned(),
    }
}

fn project_content_reference(
    entry: &ContentCatalogEntryReadModel,
) -> AgentContentReferenceReadModel {
    AgentContentReferenceReadModel {
        artifact: entry.artifact.clone(),
        title: entry.title.clone(),
        category: entry.category,
        stage: entry.stage,
        status: entry.status,
        privacy: entry.privacy,
        provenance: AgentContentProvenanceReadModel {
            actor: entry.provenance.actor,
            reason: entry.provenance.reason.clone(),
            source_id: entry.provenance.source_id.clone(),
            source_scope: entry.provenance.source_scope,
            source_role: entry.provenance.source_role,
            source_kind: entry.provenance.source_kind.clone(),
            content_type: entry.provenance.content_type.clone(),
        },
        relationships: entry.relationships.clone(),
    }
}

fn proposal_targets(
    dossier: &ApplicationDossierReadModel,
    entries: &[ContentCatalogEntryReadModel],
) -> Vec<AgentProposalTargetReadModel> {
    [
        ProposalDefinition {
            kind: AgentProposalKind::Criteria,
            stage: WorkflowStage::Criteria,
            section: AgentWorkspaceSection::JobCriteria,
            operation: "criteria.confirm",
            artifact_kinds: &[ArtifactKind::Criteria],
            validation_rules: &[
                "Validate the criteria schema and source spans.",
                "Bind the candidate to the exact current job and parsed-job revisions.",
                "Revalidate immediately before confirmation.",
            ],
            intended_mutation:
                "Replace the current reviewed criteria artifact and invalidate dependent stages.",
            commit_boundary: AgentProposalCommitBoundary::UserConfirmation,
        },
        ProposalDefinition {
            kind: AgentProposalKind::Evidence,
            stage: WorkflowStage::Evidence,
            section: AgentWorkspaceSection::EvidenceFit,
            operation: "profile.evidence.confirm",
            artifact_kinds: &[ArtifactKind::EvidenceCatalog],
            validation_rules: &[
                "Validate the evidence schema, source spans, and profile source revisions.",
                "Reject unsupported claims and stale profile inputs.",
                "Revalidate immediately before confirmation.",
            ],
            intended_mutation:
                "Replace the reusable evidence catalog and invalidate dependent application matches.",
            commit_boundary: AgentProposalCommitBoundary::UserConfirmation,
        },
        ProposalDefinition {
            kind: AgentProposalKind::Matches,
            stage: WorkflowStage::Match,
            section: AgentWorkspaceSection::EvidenceFit,
            operation: "task.complete:evidence-match",
            artifact_kinds: &[ArtifactKind::EvidenceMatches],
            validation_rules: &[
                "Validate the task result against its declared output schema.",
                "Require the exact current criteria and evidence revisions.",
                "Commit only the single-use reviewed task preview.",
            ],
            intended_mutation:
                "Accept reviewed criterion-to-evidence matches for this application revision.",
            commit_boundary: AgentProposalCommitBoundary::TaskPreviewCommit,
        },
        ProposalDefinition {
            kind: AgentProposalKind::Plan,
            stage: WorkflowStage::Plan,
            section: AgentWorkspaceSection::EvidenceFit,
            operation: "plan.confirm",
            artifact_kinds: &[ArtifactKind::ApplicationPlan],
            validation_rules: &[
                "Validate the plan schema, coverage, gaps, and prohibited claims.",
                "Require the exact current evidence-match revision.",
                "Revalidate immediately before confirmation.",
            ],
            intended_mutation:
                "Replace the reviewed application plan and invalidate dependent draft stages.",
            commit_boundary: AgentProposalCommitBoundary::UserConfirmation,
        },
        ProposalDefinition {
            kind: AgentProposalKind::Draft,
            stage: WorkflowStage::Draft,
            section: AgentWorkspaceSection::Materials,
            operation: "task.complete:*-draft",
            artifact_kinds: &[
                ArtifactKind::CoverLetter,
                ArtifactKind::ResearchStatement,
                ArtifactKind::TeachingStatement,
                ArtifactKind::Cv,
                ArtifactKind::DocumentSet,
            ],
            validation_rules: &[
                "Validate each task result against its declared document schema.",
                "Require the exact current application-plan and declared input revisions.",
                "Commit only the single-use reviewed task preview.",
            ],
            intended_mutation:
                "Accept reviewed application documents without submitting them externally.",
            commit_boundary: AgentProposalCommitBoundary::TaskPreviewCommit,
        },
    ]
    .into_iter()
    .map(|definition| proposal_target(definition, dossier, entries))
    .collect()
}

struct ProposalDefinition {
    kind: AgentProposalKind,
    stage: WorkflowStage,
    section: AgentWorkspaceSection,
    operation: &'static str,
    artifact_kinds: &'static [ArtifactKind],
    validation_rules: &'static [&'static str],
    intended_mutation: &'static str,
    commit_boundary: AgentProposalCommitBoundary,
}

fn proposal_target(
    definition: ProposalDefinition,
    dossier: &ApplicationDossierReadModel,
    entries: &[ContentCatalogEntryReadModel],
) -> AgentProposalTargetReadModel {
    let matching = entries
        .iter()
        .filter(|entry| definition.artifact_kinds.contains(&entry.artifact.kind))
        .collect::<Vec<_>>();
    let current_artifacts = matching
        .iter()
        .map(|entry| entry.artifact.clone())
        .collect::<Vec<_>>();
    let mut seen_upstream = BTreeSet::new();
    let upstream_artifacts = matching
        .iter()
        .flat_map(|entry| entry.relationships.iter())
        .filter(|artifact| seen_upstream.insert(artifact_identity(artifact)))
        .cloned()
        .collect::<Vec<_>>();
    let workflow_status = dossier
        .workflow
        .as_ref()
        .and_then(|workflow| {
            workflow
                .stages
                .iter()
                .find(|stage| stage.stage == definition.stage)
        })
        .map(|stage| stage.status);
    let state = proposal_state(&matching, workflow_status);

    AgentProposalTargetReadModel {
        kind: definition.kind,
        stage: definition.stage,
        section: definition.section,
        state,
        operation: definition.operation.to_owned(),
        current_artifacts,
        upstream_artifacts,
        validation_rules: definition
            .validation_rules
            .iter()
            .map(|rule| (*rule).to_owned())
            .collect(),
        intended_mutation: definition.intended_mutation.to_owned(),
        commit_boundary: definition.commit_boundary,
    }
}

fn proposal_state(
    entries: &[&ContentCatalogEntryReadModel],
    workflow_status: Option<StageExecutionStatus>,
) -> AgentProposalState {
    if entries
        .iter()
        .any(|entry| entry.status == ContentCatalogStatus::Proposed)
    {
        return AgentProposalState::Proposed;
    }
    if entries
        .iter()
        .any(|entry| entry.status == ContentCatalogStatus::Stale)
        || workflow_status == Some(StageExecutionStatus::Stale)
    {
        return AgentProposalState::Stale;
    }
    if !entries.is_empty() {
        return AgentProposalState::Current;
    }
    if matches!(
        workflow_status,
        Some(
            StageExecutionStatus::Ready
                | StageExecutionStatus::Running
                | StageExecutionStatus::AwaitingUser
        )
    ) {
        return AgentProposalState::Ready;
    }
    AgentProposalState::Blocked
}

fn artifact_identity(artifact: &ArtifactReference) -> String {
    format!(
        "{:?}:{}:{}:{}",
        artifact.kind,
        artifact.id.as_str(),
        artifact.revision.get(),
        artifact.sha256.as_str()
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use super::MAX_CONTENT_REFERENCES;
    use crate::{Application, PrivateReadConsent};

    fn temporary_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "canisend-agent-assistance-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn assistance_is_body_free_bounded_and_recommends_the_smallest_skill() {
        let root = temporary_root("workspace");
        let source = temporary_root("PRIVATE-ASSISTANCE-SENTINEL").with_extension("txt");
        fs::write(&source, "PRIVATE-ASSISTANCE-BODY-SENTINEL").expect("source");
        Application::initialize_workspace(&root).expect("workspace");
        let job = Application::create_job(&root, "Lecturer", "University X")
            .expect("job")
            .data;
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import");

        let assistance = Application::agent_assistance(&root, job.id.as_str()).expect("assistance");
        assert_eq!(assistance.operation, "agent.assistance");
        assert_eq!(
            assistance.data.recommendation.skill_id,
            "canisend-job-intake"
        );
        assert_eq!(assistance.data.proposal_targets.len(), 5);
        assert!(
            !assistance
                .data
                .execution_boundary
                .transcript_persisted_by_canisend
        );
        assert!(assistance.data.content.entries.len() <= MAX_CONTENT_REFERENCES);
        let encoded = serde_json::to_string(&assistance).expect("JSON");
        assert!(!encoded.contains("PRIVATE-ASSISTANCE-BODY-SENTINEL"));
        assert!(!encoded.contains("PRIVATE-ASSISTANCE-SENTINEL"));
        assert!(!encoded.contains("\"locator\""));

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove source");
    }
}
