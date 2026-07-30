use std::path::{Path, PathBuf};

use canisend_contracts::{
    DiscoveryFreshness, EntityId, JobRecord, NextAction, SourceRecord, StageExecutionStatus,
    UtcTimestamp, WorkflowStage, WorkflowStatusData,
};
use canisend_store::{
    DiscoveryService, JobService, ProfileService, StoreError, WorkflowService, Workspace,
};
use serde::{Deserialize, Serialize};

use crate::{
    ActionReceipt, Application, ApplicationError,
    application::{open_workspace, parse_entity_id},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplicationOrigin {
    Direct,
    Discovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplicationDossierState {
    NeedsSource,
    ReadyToStart,
    InProgress,
    AwaitingUser,
    Blocked,
    Complete,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationMetadataReadModel {
    pub origin: ApplicationOrigin,
    pub discovery_lead_id: Option<EntityId>,
    pub discovery_source_id: Option<EntityId>,
    pub location: Option<String>,
    pub deadline: Option<String>,
    pub source_url: Option<String>,
    pub freshness: Option<DiscoveryFreshness>,
    pub last_seen_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationDossierBlockerReadModel {
    pub code: String,
    pub description: String,
    pub stage: Option<WorkflowStage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationDossierReadModel {
    pub workspace: PathBuf,
    pub job: JobRecord,
    pub metadata: ApplicationMetadataReadModel,
    pub source_count: u64,
    pub profile_source_count: u64,
    pub state: ApplicationDossierState,
    pub current_stage: Option<WorkflowStage>,
    pub completed_stages: u64,
    pub total_stages: u64,
    pub workflow: Option<WorkflowStatusData>,
    pub blockers: Vec<ApplicationDossierBlockerReadModel>,
    pub next_actions: Vec<NextAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationDossierListReadModel {
    pub workspace: PathBuf,
    pub include_archived: bool,
    pub applications: Vec<ApplicationDossierReadModel>,
}

impl Application {
    pub fn application_dossier(
        root: &Path,
        job_id: &str,
    ) -> Result<ActionReceipt<ApplicationDossierReadModel>, ApplicationError> {
        let job_id = parse_entity_id(job_id)?;
        let mut workspace = open_workspace(root)?;
        let dossier = application_dossier_from_workspace(&mut workspace, &job_id)?;
        let next_actions = dossier.next_actions.clone();
        Ok(ActionReceipt::new(
            "application.dossier.show",
            "available",
            format!(
                "Loaded the body-free application dossier for {}",
                dossier.job.title
            ),
            dossier,
        )
        .with_next_actions(next_actions))
    }

    pub fn list_application_dossiers(
        root: &Path,
        include_archived: bool,
    ) -> Result<ActionReceipt<ApplicationDossierListReadModel>, ApplicationError> {
        let mut workspace = open_workspace(root)?;
        let jobs =
            JobService::new(&mut workspace.database, &workspace.blobs).list(include_archived)?;
        let profile_source_count = profile_source_count(&mut workspace)?;
        let mut applications = Vec::with_capacity(jobs.len());
        for job in jobs {
            applications.push(application_dossier_for_job(
                &mut workspace,
                job,
                profile_source_count,
            )?);
        }
        Ok(ActionReceipt::new(
            "application.dossier.list",
            "available",
            format!("Loaded {} application dossier(s)", applications.len()),
            ApplicationDossierListReadModel {
                workspace: workspace.paths.root,
                include_archived,
                applications,
            },
        ))
    }
}

pub(crate) fn application_dossier_from_workspace(
    workspace: &mut Workspace,
    job_id: &EntityId,
) -> Result<ApplicationDossierReadModel, ApplicationError> {
    let job = JobService::new(&mut workspace.database, &workspace.blobs).get(job_id)?;
    let profile_source_count = profile_source_count(workspace)?;
    application_dossier_for_job(workspace, job, profile_source_count)
}

fn application_dossier_for_job(
    workspace: &mut Workspace,
    job: JobRecord,
    profile_source_count: u64,
) -> Result<ApplicationDossierReadModel, ApplicationError> {
    let sources = JobService::new(&mut workspace.database, &workspace.blobs).sources(&job.id)?;
    let promoted_lead =
        DiscoveryService::new(&mut workspace.database).promoted_lead_for_job(&job.id)?;
    let workflow = match WorkflowService::new(&mut workspace.database).status(&job.id) {
        Ok(status) => Some(status),
        Err(StoreError::WorkflowNotFound(_)) => None,
        Err(error) => return Err(error.into()),
    };
    let source_count = count(sources.len(), "application source count")?;
    let total_stages = count(WorkflowStage::ALL.len(), "workflow stage count")?;
    let completed_stages = if let Some(status) = &workflow {
        count(
            status
                .stages
                .iter()
                .filter(|stage| stage.status == StageExecutionStatus::Complete)
                .count(),
            "completed workflow stage count",
        )?
    } else {
        u64::from(source_count > 0)
    };
    let current_stage = workflow.as_ref().map_or_else(
        || {
            Some(if source_count == 0 {
                WorkflowStage::Intake
            } else {
                WorkflowStage::Parse
            })
        },
        |status| {
            status
                .stages
                .iter()
                .find(|stage| stage.status != StageExecutionStatus::Complete)
                .map(|stage| stage.stage)
        },
    );
    let state = dossier_state(&job, source_count, workflow.as_ref(), current_stage);
    let blockers = dossier_blockers(&job, source_count, workflow.as_ref(), current_stage);
    let next_actions =
        dossier_next_actions(&job, source_count, profile_source_count, workflow.as_ref());
    let metadata = application_metadata(promoted_lead.as_ref(), &sources);

    Ok(ApplicationDossierReadModel {
        workspace: workspace.paths.root.clone(),
        job,
        metadata,
        source_count,
        profile_source_count,
        state,
        current_stage,
        completed_stages,
        total_stages,
        workflow,
        blockers,
        next_actions,
    })
}

fn profile_source_count(workspace: &mut Workspace) -> Result<u64, ApplicationError> {
    let sources = ProfileService::new(&mut workspace.database, &workspace.blobs).list_sources()?;
    count(sources.len(), "profile source count").map_err(Into::into)
}

fn application_metadata(
    promoted_lead: Option<&canisend_contracts::DiscoveryLeadRecord>,
    sources: &[SourceRecord],
) -> ApplicationMetadataReadModel {
    let source_url = promoted_lead
        .map(|lead| lead.url.clone())
        .or_else(|| primary_source_url(sources));
    ApplicationMetadataReadModel {
        origin: if promoted_lead.is_some() {
            ApplicationOrigin::Discovery
        } else {
            ApplicationOrigin::Direct
        },
        discovery_lead_id: promoted_lead.map(|lead| lead.id.clone()),
        discovery_source_id: promoted_lead.map(|lead| lead.source_id.clone()),
        location: promoted_lead.and_then(|lead| lead.location.clone()),
        deadline: promoted_lead.and_then(|lead| lead.deadline.clone()),
        source_url,
        freshness: promoted_lead.map(|lead| lead.freshness),
        last_seen_at: promoted_lead.map(|lead| lead.last_seen_at.clone()),
    }
}

fn primary_source_url(sources: &[SourceRecord]) -> Option<String> {
    sources.iter().rev().find_map(|source| {
        source
            .final_url
            .clone()
            .or_else(|| source.source_url.clone())
    })
}

fn dossier_state(
    job: &JobRecord,
    source_count: u64,
    workflow: Option<&WorkflowStatusData>,
    current_stage: Option<WorkflowStage>,
) -> ApplicationDossierState {
    if job.archived {
        return ApplicationDossierState::Archived;
    }
    if source_count == 0 {
        return ApplicationDossierState::NeedsSource;
    }
    let Some(workflow) = workflow else {
        return ApplicationDossierState::ReadyToStart;
    };
    let Some(current_stage) = current_stage else {
        return ApplicationDossierState::Complete;
    };
    match workflow
        .stages
        .iter()
        .find(|stage| stage.stage == current_stage)
        .map(|stage| stage.status)
    {
        Some(StageExecutionStatus::AwaitingUser) => ApplicationDossierState::AwaitingUser,
        Some(StageExecutionStatus::Blocked | StageExecutionStatus::Stale) => {
            ApplicationDossierState::Blocked
        }
        Some(
            StageExecutionStatus::Ready
            | StageExecutionStatus::Running
            | StageExecutionStatus::Complete,
        )
        | None => ApplicationDossierState::InProgress,
    }
}

fn dossier_blockers(
    job: &JobRecord,
    source_count: u64,
    workflow: Option<&WorkflowStatusData>,
    current_stage: Option<WorkflowStage>,
) -> Vec<ApplicationDossierBlockerReadModel> {
    if job.archived {
        return vec![ApplicationDossierBlockerReadModel {
            code: "job.archived".to_owned(),
            description: "The selected application is archived".to_owned(),
            stage: None,
        }];
    }
    if source_count == 0 {
        return vec![ApplicationDossierBlockerReadModel {
            code: "job.source_missing".to_owned(),
            description: "Import a job advert source before preparing application work".to_owned(),
            stage: Some(WorkflowStage::Intake),
        }];
    }
    workflow.map_or_else(Vec::new, |status| {
        status
            .blockers
            .iter()
            .filter(|blocker| Some(blocker.stage) == current_stage)
            .map(|blocker| ApplicationDossierBlockerReadModel {
                code: blocker.code.clone(),
                description: blocker.description.clone(),
                stage: Some(blocker.stage),
            })
            .collect()
    })
}

fn dossier_next_actions(
    job: &JobRecord,
    source_count: u64,
    profile_source_count: u64,
    workflow: Option<&WorkflowStatusData>,
) -> Vec<NextAction> {
    if job.archived {
        return Vec::new();
    }
    if source_count == 0 {
        return vec![NextAction {
            action: format!("canisend job import {} --file PATH", job.id),
            description: "Import a local advert, PDF, or use --url before preparing work"
                .to_owned(),
        }];
    }
    if let Some(workflow) = workflow {
        return workflow.next_actions.clone();
    }
    if profile_source_count == 0 {
        return vec![NextAction {
            action: "canisend profile source add --file PROFILE.md --json".to_owned(),
            description: "Add reusable profile evidence before starting the application workflow"
                .to_owned(),
        }];
    }
    vec![NextAction {
        action: format!("canisend workflow start --job {} --json", job.id),
        description: "Start the durable application stage graph".to_owned(),
    }]
}

fn count(value: usize, label: &str) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Invariant(format!("{label} does not fit u64")))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use canisend_contracts::PrivacyClassification;

    use super::{ApplicationDossierState, ApplicationOrigin};
    use crate::{Application, DiscoveryImportRequest, PrivateReadConsent};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-dossier-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn dossier_projects_discovery_metadata_without_a_storage_copy() {
        let root = temporary_root("discovery");
        let batch = temporary_root("leads").with_extension("csv");
        fs::write(
            &batch,
            "title,organization,url,location,deadline\n\
             Lecturer in Economics,University X,https://example.edu/jobs/1,London,2099-08-31\n",
        )
        .expect("write discovery fixture");
        Application::initialize_workspace(&root).expect("workspace");
        let preview = Application::preview_discovery_import(
            &DiscoveryImportRequest {
                path: batch.clone(),
                source_name: Some("Reviewed opportunities".to_owned()),
                source_url: None,
                host_agent: false,
            },
            PrivateReadConsent::granted_by_user(),
        )
        .expect("preview");
        Application::commit_discovery_import(&root, preview.data).expect("commit");
        let lead = Application::list_discovery_leads(&root, false)
            .expect("leads")
            .data
            .leads
            .into_iter()
            .next()
            .expect("lead");
        let job = Application::promote_discovery_lead(&root, lead.id.as_str())
            .expect("promote")
            .data
            .job;

        let dossier = Application::application_dossier(&root, job.id.as_str())
            .expect("dossier")
            .data;
        assert_eq!(dossier.metadata.origin, ApplicationOrigin::Discovery);
        assert_eq!(dossier.metadata.discovery_lead_id, Some(lead.id));
        assert_eq!(dossier.metadata.location.as_deref(), Some("London"));
        assert_eq!(dossier.metadata.deadline.as_deref(), Some("2099-08-31"));
        assert_eq!(
            dossier.metadata.source_url.as_deref(),
            Some("https://example.edu/jobs/1")
        );
        assert_eq!(dossier.state, ApplicationDossierState::NeedsSource);

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(batch).expect("remove batch");
    }

    #[test]
    fn dossier_is_body_free_and_reuses_workflow_next_actions() {
        let root = temporary_root("workflow");
        let advert = temporary_root("advert").with_extension("md");
        let sentinel = "PRIVATE-DOSSIER-SENTINEL";
        fs::write(&advert, format!("# Lecturer\n\n{sentinel}\n")).expect("advert");
        Application::initialize_workspace(&root).expect("workspace");
        let job = Application::create_job(&root, "Lecturer", "University X")
            .expect("job")
            .data;
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &advert,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("source");
        Application::initialize_profile(
            &root,
            "# Academic profile\n\n- Research and teaching evidence\n",
            PrivacyClassification::PrivateLocal,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("profile");
        let workflow = Application::start_workflow(&root, job.id.as_str())
            .expect("workflow")
            .data;

        let dossier = Application::application_dossier(&root, job.id.as_str())
            .expect("dossier")
            .data;
        assert_eq!(dossier.state, ApplicationDossierState::InProgress);
        assert_eq!(dossier.completed_stages, 1);
        assert_eq!(
            dossier.current_stage,
            Some(canisend_contracts::WorkflowStage::Parse)
        );
        assert_eq!(dossier.next_actions, workflow.next_actions);
        let encoded = serde_json::to_string(&dossier).expect("serialize dossier");
        assert!(!encoded.contains(sentinel));

        let list = Application::list_application_dossiers(&root, false)
            .expect("dossier list")
            .data;
        assert_eq!(list.applications, vec![dossier]);

        Application::archive_job(&root, job.id.as_str()).expect("archive job");
        let archived = Application::application_dossier(&root, job.id.as_str())
            .expect("archived dossier")
            .data;
        assert_eq!(archived.state, ApplicationDossierState::Archived);
        assert!(archived.next_actions.is_empty());
        assert!(
            Application::list_application_dossiers(&root, false)
                .expect("active dossiers")
                .data
                .applications
                .is_empty()
        );
        assert_eq!(
            Application::list_application_dossiers(&root, true)
                .expect("all dossiers")
                .data
                .applications,
            vec![archived]
        );

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(advert).expect("remove advert");
    }
}
