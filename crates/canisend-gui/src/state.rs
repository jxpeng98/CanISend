use std::path::PathBuf;

use canisend_app::{DiscoveryNetworkAdapter, WorkflowRerunPreview};
use canisend_contracts::{
    ApplicationPlanCandidate, ApplicationPlanRecord, ArtifactKind, CriteriaSetRecord,
    DiscoveryImportReport, EntityId, EvidenceCatalogRecord, EvidenceMatchSetRecord, ExecutionMode,
    PrivacyClassification, WorkflowStage,
};
use serde::{Deserialize, Serialize};

use crate::i18n::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Page {
    Overview,
    Jobs,
    Discovery,
    Profile,
    Workspaces,
    CommandLine,
    Diagnostics,
}

impl Page {
    pub(crate) const ALL: [Self; 7] = [
        Self::Overview,
        Self::Jobs,
        Self::Discovery,
        Self::Profile,
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
    use canisend_contracts::DiscoveryImportReport;

    use super::{DiscoveryImportForm, DiscoveryRefreshForm, parse_workflow_artifact_id};
    use super::{validate_discovery_import_form, validate_discovery_refresh_form};
    use crate::i18n::Language;

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
}
