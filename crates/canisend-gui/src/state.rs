use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::i18n::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Page {
    Overview,
    Jobs,
    Workspaces,
    CommandLine,
    Diagnostics,
}

impl Page {
    pub(crate) const ALL: [Self; 5] = [
        Self::Overview,
        Self::Jobs,
        Self::Workspaces,
        Self::CommandLine,
        Self::Diagnostics,
    ];

    pub(crate) fn label(self, language: Language) -> &'static str {
        language.text(match self {
            Self::Overview => "Overview",
            Self::Jobs => "Jobs",
            Self::Workspaces => "Workspaces",
            Self::CommandLine => "Command line",
            Self::Diagnostics => "Diagnostics",
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) enum PendingConfirmation {
    ArchiveJob { title: String },
    UninstallCli { restores_previous: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusTarget {
    JobTitle,
    ImportKind,
    WorkspaceAlias,
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

#[derive(Debug, Default)]
pub(crate) struct WorkspaceForm {
    pub(crate) alias: String,
    pub(crate) path: Option<PathBuf>,
    pub(crate) create_new: bool,
    pub(crate) error: Option<String>,
}
