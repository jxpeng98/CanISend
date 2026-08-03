#![forbid(unsafe_code)]

mod agent;
mod agent_session;
mod application;
mod application_v3;
mod assistance;
mod catalog;
mod cli_install;
mod compatibility;
mod content;
mod decision;
mod desktop_cli;
mod discovery;
mod document;
mod dossier;
mod error;
mod intake;
mod job;
mod migration_v3;
mod package;
mod plan;
mod profile;
mod receipt;
mod registry;
mod render;
mod review;
mod system;
mod task;
mod update;
mod workflow;
mod workflow_pack;
mod workspace;

pub use agent::{
    AgentCapabilitiesReadModel, AgentContextReadModel, AgentHandoffReadModel, AgentHandoffRequest,
    AgentHost, AgentMcpConfigurationReadModel, AgentMcpConfigurationRequest,
    AgentPackExportReadModel, AgentPackExportRequest, AgentSkillsInstallReadModel,
    AgentSkillsInstallRequest, AgentSkillsStatusReadModel, AgentSkillsStatusRequest,
    AgentSkillsUninstallReadModel, AgentSkillsUninstallRequest, CANISEND_MCP_GUARDED_WRITE_TOOLS,
    CANISEND_MCP_PROTOCOL_VERSION, CANISEND_MCP_READ_ONLY_TOOLS, CANISEND_MCP_TOOLS,
};
pub use agent_session::{
    AgentRuntimeKind, AgentSessionEntry, AgentSessionRegistry, default_agent_session_registry_path,
};
pub use application::{
    Application, NetworkFetchConsent, PrivateExportConsent, PrivateReadConsent, ProviderSendConsent,
};
pub use application_v3::{
    ApplicationModelCommitRequestV3, ApplicationModelCommitResultV3,
    ApplicationModelCreateRequestV3, ApplicationModelRevisionV3, StoredApplicationModelV3,
    WorkspaceV3AuthorityState,
};
pub use assistance::{
    AgentAssistanceReadModel, AgentContentGraphReadModel, AgentContentProvenanceReadModel,
    AgentContentReferenceReadModel, AgentExecutionBoundaryReadModel, AgentProposalCommitBoundary,
    AgentProposalKind, AgentProposalState, AgentProposalTargetReadModel,
    AgentRecommendationReadModel, AgentWorkspaceSection,
};
pub use catalog::{
    InspectionCatalogReadModel, ResourceCatalogExportReadModel, ResourceCatalogExportRequest,
    ResourceDetailReadModel,
};
pub use cli_install::{
    CliInstallState, CliInstallStatus, CliVersionRelation, TerminalInstallConsent,
};
pub use content::{
    ContentCatalogEntryReadModel, ContentCatalogFilter, ContentCatalogReadModel,
    ContentCatalogStatus, ContentCategory, ContentIndexReadModel, ContentMatchField,
    ContentProvenanceReadModel, ContentSearchReadModel, ContentSearchRequest,
    ContentSearchResultReadModel, ContentSourceRole, ContentSourceScope,
    ContentSubjectJobReadModel,
};
pub use desktop_cli::{default_cli_destination, desktop_cli_source_path};
pub use discovery::{
    DiscoveryAdapterCatalogReadModel, DiscoveryImportRequest, DiscoveryLeadListReadModel,
    DiscoveryNetworkAdapter, DiscoveryPromotionReadModel, DiscoveryRefreshRequest,
    DiscoverySourceListReadModel, DiscoverySuggestionReadModel,
    PackDiscoveryAdapterCatalogReadModel, PackDiscoveryAdapterReadModel,
};
pub use document::DocumentWorkspaceReadModel;
pub use dossier::{
    ApplicationDossierBlockerReadModel, ApplicationDossierListReadModel,
    ApplicationDossierReadModel, ApplicationDossierState, ApplicationMetadataReadModel,
    ApplicationOrigin,
};
pub use error::{ApplicationError, ApplicationFailure};
pub use intake::{
    IntakeCommitBoundary, IntakeDuplicateSignalReadModel, IntakeDuplicateState,
    IntakeExtractionReadModel, IntakeMutationReadModel, IntakeReviewReadModel,
    IntakeSourceIdentityReadModel, IntakeSourceKind, IntakeTargetKind, IntakeTargetReadModel,
    discovery_intake_review, job_intake_review,
};
pub use job::{
    JobDetailReadModel, JobIntakeExtractionReadModel, JobIntakeIssueSeverity,
    JobIntakeMutationReadModel, JobIntakePreviewReadModel, JobIntakeProvenanceReadModel,
    JobIntakeSourceKind, JobIntakeValidationIssue, JobListReadModel, PreparedJobSource,
    SourceImportReadModel,
};
pub use migration_v3::{
    WorkspaceV3MigrationPreview, WorkspaceV3MigrationReadModel, WorkspaceV3MigrationRequest,
    WorkspaceV3MigrationResult,
};
pub use package::{PackageExportRequest, ProjectionCopyAsNewRequest, ProjectionReplaceRequest};
pub use profile::{
    ProfileInitializationReadModel, ProfileSourceImportReadModel, ProfileSourceListReadModel,
};
pub use receipt::ActionReceipt;
pub use registry::{
    WorkspaceEntry, WorkspaceRegistry, default_registry_path, validate_workspace_alias,
};
pub use render::{RenderExportReadModel, RenderExportRequest, RenderPreviewReadModel};
pub use review::ReviewWorkspaceReadModel;
pub use system::{DoctorSummary, ProductSummary};
pub use task::{
    TaskCompletionPreviewReadModel, TaskExecutionMode, TaskInputExportRequest, TaskOperation,
    TaskPrepareAgainReadModel, TaskPrepareRequest,
};
pub use update::UpdateCheckReadModel;
pub use workflow::{
    WorkflowBeginRequest, WorkflowCompleteRequest, WorkflowControlReadModel, WorkflowRerunPreview,
    WorkflowRerunRequest,
};
pub use workflow_pack::built_in_academic_job_pack;
pub use workspace::{
    BackupReadModel, WorkspaceHealthReadModel, WorkspaceInitPolicy, WorkspaceReadModel,
    WorkspaceRepairReadModel, WorkspaceRestoreReadModel,
};

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{Application, PrivateReadConsent};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn facade_completes_workspace_job_import_and_workflow_slice() {
        let root = temporary_root("vertical");
        let source = temporary_root("advert").with_extension("md");
        fs::write(
            &source,
            "# Lecturer in Economics\n\nTeach and publish research.\n",
        )
        .expect("write fixture");

        let initialized = Application::initialize_workspace(&root).expect("initialize workspace");
        assert_eq!(initialized.operation, "workspace.init");
        assert_eq!(initialized.data.status.job_count, 0);

        let job = Application::create_job(&root, "Lecturer in Economics", "University X")
            .expect("create job")
            .data;
        let imported = Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import source");
        assert_eq!(imported.data.job.revision.get(), 2);
        assert_eq!(
            imported.data.source.content_type,
            "text/markdown; charset=utf-8"
        );

        let workflow = Application::start_workflow(&root, job.id.as_str()).expect("start workflow");
        assert_eq!(workflow.data.stages.len(), 10);
        assert_eq!(
            workflow.data.stages[0].status,
            canisend_contracts::StageExecutionStatus::Complete
        );

        let jobs = Application::list_jobs(&root, false).expect("list jobs");
        assert_eq!(jobs.data.jobs.len(), 1);
        let detail = Application::job_detail(&root, job.id.as_str()).expect("job detail");
        assert_eq!(detail.data.sources.len(), 1);
        assert!(detail.data.workflow.is_some());

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove fixture");
    }

    #[test]
    fn routine_read_models_do_not_contain_private_source_bodies() {
        let root = temporary_root("privacy");
        let source = temporary_root("private").with_extension("txt");
        let sentinel = "PRIVATE-SENTINEL-DO-NOT-LEAK";
        fs::write(&source, sentinel).expect("write fixture");
        Application::initialize_workspace(&root).expect("initialize workspace");
        let job = Application::create_job(&root, "Reader", "University Y")
            .expect("create job")
            .data;
        Application::import_local_job_source(
            &root,
            job.id.as_str(),
            &source,
            PrivateReadConsent::granted_by_user(),
        )
        .expect("import source");

        let detail = Application::job_detail(&root, job.id.as_str()).expect("job detail");
        let serialized = serde_json::to_string(&detail).expect("serialize read model");
        assert!(!serialized.contains(sentinel));

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_file(source).expect("remove fixture");
    }

    #[test]
    fn facade_rejects_initializing_over_existing_user_files() {
        let root = temporary_root("non-empty");
        fs::create_dir_all(&root).expect("create directory");
        let sentinel = root.join("keep-me.txt");
        fs::write(&sentinel, "user-owned").expect("write sentinel");

        assert!(Application::initialize_workspace(&root).is_err());
        assert_eq!(
            fs::read_to_string(&sentinel).expect("sentinel remains"),
            "user-owned"
        );
        assert!(!root.join("canisend.toml").exists());
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
