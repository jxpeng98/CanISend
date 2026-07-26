use std::path::PathBuf;

use canisend_app::WorkflowRerunPreview;
use canisend_contracts::{
    ApplicationPlanCandidate, ApplicationPlanRecord, ArtifactKind, CriteriaSetRecord, EntityId,
    EvidenceCatalogRecord, EvidenceMatchSetRecord, ExecutionMode, PrivacyClassification,
    WorkflowStage,
};
use serde::{Deserialize, Serialize};

use crate::i18n::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Page {
    Overview,
    Jobs,
    Profile,
    Workspaces,
    CommandLine,
    Diagnostics,
}

impl Page {
    pub(crate) const ALL: [Self; 6] = [
        Self::Overview,
        Self::Jobs,
        Self::Profile,
        Self::Workspaces,
        Self::CommandLine,
        Self::Diagnostics,
    ];

    pub(crate) fn label(self, language: Language) -> &'static str {
        language.text(match self {
            Self::Overview => "Overview",
            Self::Jobs => "Jobs",
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
    use super::parse_workflow_artifact_id;

    #[test]
    fn workflow_artifact_input_requires_a_canonical_uuidv7() {
        assert!(parse_workflow_artifact_id("019f2f55-7c00-7000-8000-000000000002").is_ok());
        for invalid in ["", "artifact-1", "550e8400-e29b-41d4-a716-446655440000"] {
            assert!(parse_workflow_artifact_id(invalid).is_err());
        }
    }
}
