use std::path::PathBuf;

use canisend_app::{
    AgentCapabilitiesReadModel, AgentContextReadModel, AgentHost, AgentPackExportReadModel,
    ApplicationFailure, DiscoveryNetworkAdapter, TaskCompletionPreviewReadModel, TaskExecutionMode,
    TaskOperation, WorkflowControlReadModel, WorkflowRerunPreview,
};
use canisend_contracts::{
    ApplicationPlanCandidate, ApplicationPlanRecord, ArtifactKind, CriteriaSetRecord,
    DiscoveryImportReport, DocumentRecord, DocumentSetRecord, EntityId, EvidenceCatalogRecord,
    EvidenceMatchSetRecord, ExecutionMode, FindingDisposition, PrivacyClassification,
    ReviewDispositionCandidate, ReviewFindingsRecord, StageExecutionStatus, TaskInputExportData,
    TaskStateData, TaskStatus, WorkflowStage,
};
use serde::{Deserialize, Serialize};

use crate::i18n::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Page {
    Overview,
    Jobs,
    Discovery,
    Profile,
    AgentIntegration,
    Workspaces,
    CommandLine,
    Diagnostics,
}

impl Page {
    pub(crate) const ALL: [Self; 8] = [
        Self::Overview,
        Self::Jobs,
        Self::Discovery,
        Self::Profile,
        Self::AgentIntegration,
        Self::Workspaces,
        Self::CommandLine,
        Self::Diagnostics,
    ];

    pub(crate) fn label(self, language: Language) -> &'static str {
        language.text(match self {
            Self::Overview => "Overview",
            Self::Jobs => "Jobs",
            Self::Discovery => "Discovery",
            Self::Profile => "Profile",
            Self::AgentIntegration => "Agent integration",
            Self::Workspaces => "Workspaces",
            Self::CommandLine => "Command line",
            Self::Diagnostics => "Diagnostics",
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) enum PendingConfirmation {
    ArchiveJob {
        title: String,
    },
    RestoreWorkspace {
        alias: String,
        backup: PathBuf,
        destination: PathBuf,
    },
    RepairWorkspace {
        path: PathBuf,
    },
    RerunWorkflow {
        preview: WorkflowRerunPreview,
    },
    PromoteDiscoveryLead {
        lead_id: String,
        title: String,
        organization: String,
    },
    CancelTask {
        task_id: String,
        operation: String,
    },
    UninstallCli {
        restores_previous: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusTarget {
    JobTitle,
    ImportKind,
    ProfileSensitivity,
    WorkspaceAlias,
    RestoreWorkspaceAlias,
    WorkflowArtifact,
    TaskPrepare,
    TaskExport,
    TaskCompletionFile,
    TaskCommit,
    TaskCancel,
    TaskPrepareAgain,
    AgentContextRefresh,
    AgentExport,
    DocumentLoad,
    ReviewLoad,
    ReviewConfirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JobPanel {
    Workflow,
    Documents,
    ReviewExport,
}

impl JobPanel {
    pub(crate) const ALL: [Self; 3] = [Self::Workflow, Self::Documents, Self::ReviewExport];

    pub(crate) fn label(self, language: Language) -> &'static str {
        language.select(
            match self {
                Self::Workflow => "Workflow",
                Self::Documents => "Documents",
                Self::ReviewExport => "Review & export",
            },
            match self {
                Self::Workflow => "工作流",
                Self::Documents => "申请文档",
                Self::ReviewExport => "审阅与导出",
            },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GuiPreferences {
    pub(crate) dark_mode: bool,
    pub(crate) compact: bool,
    pub(crate) reduce_motion: bool,
    #[serde(default)]
    pub(crate) language: Language,
}

#[derive(Debug, Default)]
pub(crate) struct JobForm {
    pub(crate) title: String,
    pub(crate) institution: String,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportKind {
    File,
    Url,
}

#[derive(Debug)]
pub(crate) struct ImportForm {
    pub(crate) kind: ImportKind,
    pub(crate) file: Option<PathBuf>,
    pub(crate) url: String,
    pub(crate) network_consent: bool,
    pub(crate) private_read_consent: bool,
    pub(crate) error: Option<String>,
}

impl Default for ImportForm {
    fn default() -> Self {
        Self {
            kind: ImportKind::File,
            file: None,
            url: String::new(),
            network_consent: false,
            private_read_consent: false,
            error: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ProfileSourceForm {
    pub(crate) file: Option<PathBuf>,
    pub(crate) sensitivity: PrivacyClassification,
    pub(crate) private_read_consent: bool,
    pub(crate) error: Option<String>,
}

impl Default for ProfileSourceForm {
    fn default() -> Self {
        Self {
            file: None,
            sensitivity: PrivacyClassification::PrivateLocal,
            private_read_consent: false,
            error: None,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct EvidenceReviewForm {
    pub(crate) job_id: Option<String>,
    pub(crate) private_read_consent: bool,
    pub(crate) candidate: Option<EvidenceCatalogRecord>,
    pub(crate) downstream_effects_confirmed: bool,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct CriteriaMatchForm {
    pub(crate) job_id: Option<String>,
    pub(crate) criteria_private_read_consent: bool,
    pub(crate) candidate: Option<CriteriaSetRecord>,
    pub(crate) downstream_effects_confirmed: bool,
    pub(crate) criteria_error: Option<String>,
    pub(crate) match_private_read_consent: bool,
    pub(crate) matches: Option<EvidenceMatchSetRecord>,
    pub(crate) match_error: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct PlanReviewForm {
    pub(crate) job_id: Option<String>,
    pub(crate) private_read_consent: bool,
    pub(crate) candidate: Option<ApplicationPlanCandidate>,
    pub(crate) current: Option<ApplicationPlanRecord>,
    pub(crate) decision_confirmed: bool,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct DocumentWorkspaceForm {
    pub(crate) job_id: Option<String>,
    pub(crate) private_read_consent: bool,
    pub(crate) documents: Option<Vec<DocumentRecord>>,
    pub(crate) accepted_set: Option<DocumentSetRecord>,
    pub(crate) acceptance_blocker: Option<String>,
    pub(crate) error: Option<String>,
}

impl DocumentWorkspaceForm {
    pub(crate) fn select_job(&mut self, job_id: &str) {
        if self.job_id.as_deref() != Some(job_id) {
            *self = Self {
                job_id: Some(job_id.to_owned()),
                ..Self::default()
            };
        }
    }

    pub(crate) fn clear_loaded_private_data(&mut self) {
        self.documents = None;
        self.accepted_set = None;
        self.acceptance_blocker = None;
        self.error = None;
    }
}

#[derive(Debug, Default)]
pub(crate) struct ReviewWorkspaceForm {
    pub(crate) job_id: Option<String>,
    pub(crate) private_read_consent: bool,
    pub(crate) current: Option<ReviewFindingsRecord>,
    pub(crate) candidate: Option<ReviewDispositionCandidate>,
    pub(crate) downstream_effects_confirmed: bool,
    pub(crate) error: Option<String>,
}

impl ReviewWorkspaceForm {
    pub(crate) fn select_job(&mut self, job_id: &str) {
        if self.job_id.as_deref() != Some(job_id) {
            *self = Self {
                job_id: Some(job_id.to_owned()),
                ..Self::default()
            };
        }
    }

    pub(crate) fn clear_loaded_private_data(&mut self) {
        self.current = None;
        self.candidate = None;
        self.downstream_effects_confirmed = false;
        self.error = None;
    }

    pub(crate) fn validation_issue(&self) -> Option<ReviewValidationIssue> {
        let candidate = self.candidate.as_ref()?;
        let selected = candidate
            .decisions
            .iter()
            .filter(|decision| decision.disposition.is_some())
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Some(ReviewValidationIssue::NoSelection);
        }
        if selected.iter().any(|decision| {
            decision
                .rationale
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        }) {
            return Some(ReviewValidationIssue::MissingRationale);
        }
        if !self.downstream_effects_confirmed {
            return Some(ReviewValidationIssue::DownstreamEffectsNotConfirmed);
        }
        None
    }

    pub(crate) fn candidate_mut(
        &mut self,
        finding_id: &EntityId,
    ) -> Option<&mut canisend_contracts::FindingDispositionCandidateRecord> {
        self.candidate
            .as_mut()?
            .decisions
            .iter_mut()
            .find(|decision| decision.finding_id == *finding_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewValidationIssue {
    NoSelection,
    MissingRationale,
    DownstreamEffectsNotConfirmed,
}

pub(crate) fn finding_disposition_values() -> [Option<FindingDisposition>; 3] {
    [
        None,
        Some(FindingDisposition::AcceptedRisk),
        Some(FindingDisposition::Dismissed),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiscoveryPanel {
    Leads,
    Sources,
    Import,
    Refresh,
}

impl DiscoveryPanel {
    pub(crate) const ALL: [Self; 4] = [Self::Leads, Self::Sources, Self::Import, Self::Refresh];

    pub(crate) fn label(self, language: Language) -> &'static str {
        language.text(match self {
            Self::Leads => "Leads",
            Self::Sources => "Discovery sources",
            Self::Import => "Import batch",
            Self::Refresh => "Refresh public source",
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct DiscoveryImportForm {
    pub(crate) file: Option<PathBuf>,
    pub(crate) source_name: String,
    pub(crate) source_url: String,
    pub(crate) host_agent: bool,
    pub(crate) private_read_consent: bool,
    pub(crate) preview: Option<DiscoveryImportReport>,
    pub(crate) error: Option<String>,
}

impl DiscoveryImportForm {
    pub(crate) fn invalidate_preview(&mut self) {
        self.preview = None;
        self.error = None;
    }
}

#[derive(Debug)]
pub(crate) struct DiscoveryRefreshForm {
    pub(crate) adapter: DiscoveryNetworkAdapter,
    pub(crate) endpoint: String,
    pub(crate) source_name: String,
    pub(crate) organization: String,
    pub(crate) network_consent: bool,
    pub(crate) preview: Option<DiscoveryImportReport>,
    pub(crate) error: Option<String>,
}

impl Default for DiscoveryRefreshForm {
    fn default() -> Self {
        Self {
            adapter: DiscoveryNetworkAdapter::RssAtom,
            endpoint: String::new(),
            source_name: String::new(),
            organization: String::new(),
            network_consent: false,
            preview: None,
            error: None,
        }
    }
}

impl DiscoveryRefreshForm {
    pub(crate) fn invalidate_preview(&mut self) {
        self.preview = None;
        self.error = None;
    }
}

#[derive(Debug)]
pub(crate) struct TaskPanelForm {
    pub(crate) job_id: Option<String>,
    pub(crate) operation: TaskOperation,
    pub(crate) mode: TaskExecutionMode,
    pub(crate) state: Option<TaskStateData>,
    pub(crate) export_destination: Option<PathBuf>,
    pub(crate) private_read_consent: bool,
    pub(crate) provider_send_consent: bool,
    pub(crate) exported: Option<TaskInputExportData>,
    pub(crate) completion_file: Option<PathBuf>,
    pub(crate) completion_read_consent: bool,
    pub(crate) completion_preview: Option<TaskCompletionPreviewReadModel>,
    pub(crate) failure: Option<ApplicationFailure>,
    pub(crate) stale_detected: bool,
}

impl Default for TaskPanelForm {
    fn default() -> Self {
        Self {
            job_id: None,
            operation: TaskOperation::JobParse,
            mode: TaskExecutionMode::HostAgent,
            state: None,
            export_destination: None,
            private_read_consent: false,
            provider_send_consent: false,
            exported: None,
            completion_file: None,
            completion_read_consent: false,
            completion_preview: None,
            failure: None,
            stale_detected: false,
        }
    }
}

impl TaskPanelForm {
    pub(crate) fn select_job(&mut self, job_id: &str) {
        if self.job_id.as_deref() != Some(job_id) {
            *self = Self {
                job_id: Some(job_id.to_owned()),
                ..Self::default()
            };
        }
    }

    pub(crate) fn apply_state(&mut self, state: Option<TaskStateData>) {
        let next_id = state.as_ref().map(|state| state.descriptor.id.as_str());
        let current_id = self
            .state
            .as_ref()
            .map(|state| state.descriptor.id.as_str());
        let terminal_state = state
            .as_ref()
            .is_some_and(|state| state.status != TaskStatus::Prepared);
        if current_id != next_id || terminal_state {
            self.export_destination = None;
            self.private_read_consent = false;
            self.provider_send_consent = false;
            self.exported = None;
            self.completion_file = None;
            self.completion_read_consent = false;
            self.completion_preview = None;
            self.failure = None;
            self.stale_detected = false;
        }
        self.state = state;
    }

    pub(crate) fn invalidate_completion_preview(&mut self) {
        self.completion_preview = None;
        self.failure = None;
        self.stale_detected = false;
    }

    pub(crate) fn requires_provider_send(&self) -> bool {
        self.state.as_ref().is_some_and(|state| {
            state.descriptor.required_consents.iter().any(|consent| {
                consent.scope == canisend_contracts::ConsentScope::SendToConfiguredProvider
            })
        })
    }

    pub(crate) fn can_prepare_again(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|state| matches!(state.status, TaskStatus::Cancelled | TaskStatus::Stale))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentDestinationPreview {
    New,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentDestinationIssue {
    InsideWorkspace,
    Symlink,
    NotDirectory,
    NotEmpty,
    MissingParent,
    Unreadable,
}

#[derive(Debug)]
pub(crate) struct AgentIntegrationForm {
    pub(crate) selected_job_id: Option<String>,
    pub(crate) capabilities: Option<AgentCapabilitiesReadModel>,
    pub(crate) context: Option<AgentContextReadModel>,
    pub(crate) host: AgentHost,
    pub(crate) destination: Option<PathBuf>,
    pub(crate) destination_preview: Option<AgentDestinationPreview>,
    pub(crate) destination_issue: Option<AgentDestinationIssue>,
    pub(crate) exported: Option<AgentPackExportReadModel>,
    pub(crate) failure: Option<ApplicationFailure>,
}

impl Default for AgentIntegrationForm {
    fn default() -> Self {
        Self {
            selected_job_id: None,
            capabilities: None,
            context: None,
            host: AgentHost::Codex,
            destination: None,
            destination_preview: None,
            destination_issue: None,
            exported: None,
            failure: None,
        }
    }
}

impl AgentIntegrationForm {
    pub(crate) fn select_job(&mut self, job_id: Option<String>) {
        if self.selected_job_id != job_id {
            self.selected_job_id = job_id;
            self.context = None;
            self.failure = None;
        }
    }

    pub(crate) fn select_host(&mut self, host: AgentHost) {
        if self.host != host {
            self.host = host;
            self.exported = None;
            self.failure = None;
        }
    }

    pub(crate) fn select_destination(&mut self, destination: PathBuf) {
        let preview = inspect_agent_export_destination(&destination);
        self.destination = Some(destination);
        self.destination_preview = preview.ok();
        self.destination_issue = preview.err();
        self.exported = None;
        self.failure = None;
    }

    pub(crate) fn export_ready(&self) -> bool {
        self.destination.is_some()
            && self.destination_preview.is_some()
            && self.destination_issue.is_none()
    }
}

pub(crate) fn inspect_agent_export_destination(
    destination: &std::path::Path,
) -> Result<AgentDestinationPreview, AgentDestinationIssue> {
    if destination
        .components()
        .any(|component| component.as_os_str().eq_ignore_ascii_case(".canisend"))
    {
        return Err(AgentDestinationIssue::InsideWorkspace);
    }
    match std::fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(AgentDestinationIssue::Symlink),
        Ok(metadata) if !metadata.is_dir() => Err(AgentDestinationIssue::NotDirectory),
        Ok(_) => match std::fs::read_dir(destination) {
            Ok(mut entries) => {
                if entries.next().is_none() {
                    Ok(AgentDestinationPreview::Empty)
                } else {
                    Err(AgentDestinationIssue::NotEmpty)
                }
            }
            Err(_) => Err(AgentDestinationIssue::Unreadable),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let Some(parent) = destination.parent() else {
                return Err(AgentDestinationIssue::MissingParent);
            };
            match std::fs::symlink_metadata(parent) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    Err(AgentDestinationIssue::Symlink)
                }
                Ok(metadata) if metadata.is_dir() => Ok(AgentDestinationPreview::New),
                Ok(_) | Err(_) => Err(AgentDestinationIssue::MissingParent),
            }
        }
        Err(_) => Err(AgentDestinationIssue::Unreadable),
    }
}

pub(crate) fn task_operation_stage(operation: TaskOperation) -> WorkflowStage {
    match operation {
        TaskOperation::JobParse => WorkflowStage::Parse,
        TaskOperation::EvidenceNormalize => WorkflowStage::Evidence,
        TaskOperation::EvidenceMatch => WorkflowStage::Match,
        TaskOperation::CoverLetterDraft
        | TaskOperation::ResearchStatementDraft
        | TaskOperation::TeachingStatementDraft
        | TaskOperation::CvDraft => WorkflowStage::Draft,
        TaskOperation::DocumentReview => WorkflowStage::Review,
    }
}

pub(crate) fn task_operations_for_ready_stages(
    ready_stages: impl IntoIterator<Item = WorkflowStage>,
) -> Vec<TaskOperation> {
    let ready_stages = ready_stages.into_iter().collect::<Vec<_>>();
    TaskOperation::ALL
        .into_iter()
        .filter(|operation| ready_stages.contains(&task_operation_stage(*operation)))
        .collect()
}

pub(crate) fn available_task_operations(
    controls: Option<&WorkflowControlReadModel>,
) -> Vec<TaskOperation> {
    task_operations_for_ready_stages(controls.into_iter().flat_map(|controls| {
        controls
            .status
            .stages
            .iter()
            .filter(|stage| stage.status == StageExecutionStatus::Ready)
            .map(|stage| stage.stage)
    }))
}

pub(crate) fn available_task_modes(
    controls: Option<&WorkflowControlReadModel>,
    operation: TaskOperation,
) -> Vec<TaskExecutionMode> {
    let stage = task_operation_stage(operation);
    controls
        .and_then(|controls| {
            controls
                .stage_descriptors
                .iter()
                .find(|descriptor| descriptor.stage == stage)
        })
        .into_iter()
        .flat_map(|descriptor| descriptor.execution_modes.iter().copied())
        .filter_map(|mode| match mode {
            ExecutionMode::HostAgent => Some(TaskExecutionMode::HostAgent),
            ExecutionMode::ConfiguredProvider => Some(TaskExecutionMode::ConfiguredProvider),
            _ => None,
        })
        .collect()
}

pub(crate) fn validate_discovery_import_form(
    file: Option<&std::path::Path>,
    host_agent: bool,
    private_read_consent: bool,
    language: Language,
) -> Result<(), String> {
    let Some(file) = file else {
        return Err(language
            .select(
                "Choose a CSV or JSON batch first",
                "请先选择 CSV 或 JSON 批次",
            )
            .to_owned());
    };
    let extension = file
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case("csv") && !extension.eq_ignore_ascii_case("json") {
        return Err(language
            .select(
                "Discovery batches must be CSV or JSON",
                "发现批次必须是 CSV 或 JSON",
            )
            .to_owned());
    }
    if host_agent && !extension.eq_ignore_ascii_case("json") {
        return Err(language
            .select(
                "Host-agent discovery requires a JSON batch",
                "宿主 Agent 发现导入必须使用 JSON 批次",
            )
            .to_owned());
    }
    if !private_read_consent {
        return Err(language
            .select(
                "Confirm local batch read consent before preview",
                "预览前请确认允许读取本地批次",
            )
            .to_owned());
    }
    Ok(())
}

pub(crate) fn validate_discovery_refresh_form(
    endpoint: &str,
    source_name: &str,
    network_consent: bool,
    language: Language,
) -> Result<(), String> {
    if endpoint.trim().is_empty() {
        return Err(language
            .select("Enter a public refresh endpoint", "请输入公开刷新端点")
            .to_owned());
    }
    if source_name.trim().is_empty() {
        return Err(language
            .select("Enter a discovery source name", "请输入发现来源名称")
            .to_owned());
    }
    if !network_consent {
        return Err(language
            .select(
                "Confirm public network consent before fetching",
                "读取前请确认允许公开网络访问",
            )
            .to_owned());
    }
    Ok(())
}

#[derive(Debug, Default)]
pub(crate) struct WorkspaceForm {
    pub(crate) alias: String,
    pub(crate) path: Option<PathBuf>,
    pub(crate) create_new: bool,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct RestoreWorkspaceForm {
    pub(crate) alias: String,
    pub(crate) backup: Option<PathBuf>,
    pub(crate) destination: Option<PathBuf>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum WorkflowActionForm {
    Begin {
        stage: WorkflowStage,
        modes: Vec<ExecutionMode>,
        selected_mode: ExecutionMode,
        error: Option<String>,
    },
    Complete {
        stage: WorkflowStage,
        expected_kind: ArtifactKind,
        artifact_id: String,
        error: Option<String>,
    },
}

impl WorkflowActionForm {
    pub(crate) fn set_error(&mut self, message: String) {
        match self {
            Self::Begin { error, .. } | Self::Complete { error, .. } => *error = Some(message),
        }
    }
}

pub(crate) fn parse_workflow_artifact_id(value: &str) -> Result<EntityId, ()> {
    EntityId::try_new(value.trim()).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use canisend_app::{AgentHost, TaskExecutionMode, TaskOperation};
    use canisend_contracts::{DiscoveryImportReport, WorkflowStage};

    use super::{
        AgentDestinationIssue, AgentDestinationPreview, AgentIntegrationForm, DiscoveryImportForm,
        DiscoveryRefreshForm, DocumentWorkspaceForm, ReviewWorkspaceForm, TaskPanelForm,
        inspect_agent_export_destination, parse_workflow_artifact_id, task_operation_stage,
        task_operations_for_ready_stages,
    };
    use super::{validate_discovery_import_form, validate_discovery_refresh_form};
    use crate::i18n::Language;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-gui-state-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn workflow_artifact_input_requires_a_canonical_uuidv7() {
        assert!(parse_workflow_artifact_id("019f2f55-7c00-7000-8000-000000000002").is_ok());
        for invalid in ["", "artifact-1", "550e8400-e29b-41d4-a716-446655440000"] {
            assert!(parse_workflow_artifact_id(invalid).is_err());
        }
    }

    #[test]
    fn discovery_form_changes_discard_the_reviewed_report() {
        let report = DiscoveryImportReport {
            dry_run: true,
            accepted: 0,
            rejected: 0,
            diagnostics: Vec::new(),
            batch: None,
            receipt: None,
        };
        let mut import = DiscoveryImportForm {
            preview: Some(report.clone()),
            error: Some("old error".to_owned()),
            ..DiscoveryImportForm::default()
        };
        import.invalidate_preview();
        assert!(import.preview.is_none());
        assert!(import.error.is_none());

        let mut refresh = DiscoveryRefreshForm {
            preview: Some(report),
            error: Some("old error".to_owned()),
            ..DiscoveryRefreshForm::default()
        };
        refresh.invalidate_preview();
        assert!(refresh.preview.is_none());
        assert!(refresh.error.is_none());
    }

    #[test]
    fn discovery_forms_enforce_separate_read_and_network_consent() {
        let csv = std::path::Path::new("/tmp/leads.csv");
        let json = std::path::Path::new("/tmp/leads.json");
        assert!(validate_discovery_import_form(Some(csv), false, true, Language::English).is_ok());
        assert!(validate_discovery_import_form(Some(json), true, true, Language::English).is_ok());
        assert_eq!(
            validate_discovery_import_form(Some(csv), true, true, Language::SimplifiedChinese)
                .expect_err("host-agent CSV"),
            "宿主 Agent 发现导入必须使用 JSON 批次"
        );
        assert!(
            validate_discovery_import_form(Some(json), false, false, Language::English).is_err()
        );
        assert!(
            validate_discovery_refresh_form(
                "https://example.edu/feed",
                "University feed",
                true,
                Language::English
            )
            .is_ok()
        );
        assert_eq!(
            validate_discovery_refresh_form(
                "https://example.edu/feed",
                "University feed",
                false,
                Language::SimplifiedChinese
            )
            .expect_err("network consent"),
            "读取前请确认允许公开网络访问"
        );
    }

    #[test]
    fn task_operations_follow_only_ready_workflow_stages() {
        assert_eq!(
            task_operation_stage(TaskOperation::EvidenceNormalize),
            WorkflowStage::Evidence
        );
        assert_eq!(
            task_operations_for_ready_stages([WorkflowStage::Parse, WorkflowStage::Draft]),
            vec![
                TaskOperation::JobParse,
                TaskOperation::CoverLetterDraft,
                TaskOperation::ResearchStatementDraft,
                TaskOperation::TeachingStatementDraft,
                TaskOperation::CvDraft,
            ]
        );
        assert!(task_operations_for_ready_stages([]).is_empty());
    }

    #[test]
    fn task_form_changes_discard_reviewed_completion_and_private_choices() {
        let mut form = TaskPanelForm {
            job_id: Some("job-a".to_owned()),
            mode: TaskExecutionMode::ConfiguredProvider,
            completion_file: Some(std::path::PathBuf::from("/tmp/completion.json")),
            completion_read_consent: true,
            stale_detected: true,
            ..TaskPanelForm::default()
        };
        form.invalidate_completion_preview();
        assert!(!form.stale_detected);
        assert!(form.failure.is_none());
        form.select_job("job-b");
        assert_eq!(form.job_id.as_deref(), Some("job-b"));
        assert_eq!(form.mode, TaskExecutionMode::HostAgent);
        assert!(form.completion_file.is_none());
        assert!(!form.completion_read_consent);
    }

    #[test]
    fn document_workspace_drops_private_state_when_the_job_changes() {
        let mut form = DocumentWorkspaceForm {
            job_id: Some("job-a".to_owned()),
            private_read_consent: true,
            acceptance_blocker: Some("old blocker".to_owned()),
            error: Some("old error".to_owned()),
            ..DocumentWorkspaceForm::default()
        };
        form.select_job("job-b");
        assert_eq!(form.job_id.as_deref(), Some("job-b"));
        assert!(!form.private_read_consent);
        assert!(form.documents.is_none());
        assert!(form.accepted_set.is_none());
        assert!(form.acceptance_blocker.is_none());
        assert!(form.error.is_none());
    }

    #[test]
    fn review_workspace_drops_private_state_and_consent_when_the_job_changes() {
        let mut form = ReviewWorkspaceForm {
            job_id: Some("job-a".to_owned()),
            private_read_consent: true,
            downstream_effects_confirmed: true,
            error: Some("old error".to_owned()),
            ..ReviewWorkspaceForm::default()
        };
        form.select_job("job-b");
        assert_eq!(form.job_id.as_deref(), Some("job-b"));
        assert!(!form.private_read_consent);
        assert!(form.current.is_none());
        assert!(form.candidate.is_none());
        assert!(!form.downstream_effects_confirmed);
        assert!(form.error.is_none());
    }

    #[test]
    fn agent_destination_preview_accepts_only_new_or_empty_directories() {
        let root = temporary_root("agent-destination");
        let empty = root.join("empty");
        let new = root.join("new");
        let non_empty = root.join("non-empty");
        std::fs::create_dir_all(&empty).expect("create empty destination");
        std::fs::create_dir_all(&non_empty).expect("create non-empty destination");
        std::fs::write(non_empty.join("keep.txt"), "user-owned").expect("write sentinel");

        assert_eq!(
            inspect_agent_export_destination(&empty),
            Ok(AgentDestinationPreview::Empty)
        );
        assert_eq!(
            inspect_agent_export_destination(&new),
            Ok(AgentDestinationPreview::New)
        );
        assert_eq!(
            inspect_agent_export_destination(&non_empty),
            Err(AgentDestinationIssue::NotEmpty)
        );
        assert_eq!(
            inspect_agent_export_destination(&root.join(".canisend/export")),
            Err(AgentDestinationIssue::InsideWorkspace)
        );

        std::fs::remove_dir_all(root).expect("remove destination fixtures");
    }

    #[test]
    fn agent_form_invalidates_stale_context_and_export_selections() {
        let mut form = AgentIntegrationForm::default();
        form.select_job(Some("job-a".to_owned()));
        assert_eq!(form.selected_job_id.as_deref(), Some("job-a"));
        assert!(form.context.is_none());

        form.select_host(AgentHost::Generic);
        assert_eq!(form.host, AgentHost::Generic);
        assert!(form.exported.is_none());

        let root = temporary_root("agent-form");
        std::fs::create_dir_all(&root).expect("create empty destination");
        form.select_destination(root.clone());
        assert!(form.export_ready());
        assert_eq!(
            form.destination_preview,
            Some(AgentDestinationPreview::Empty)
        );
        std::fs::remove_dir_all(root).expect("remove empty destination");
    }
}
