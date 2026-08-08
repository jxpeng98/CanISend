use std::path::Path;

use canisend_contracts::EntityId;
use canisend_store::{StoreError, Workspace};

use crate::ApplicationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivateReadConsent(());

impl PrivateReadConsent {
    #[must_use]
    pub const fn granted_by_user() -> Self {
        Self(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkFetchConsent(());

impl NetworkFetchConsent {
    #[must_use]
    pub const fn granted_by_user() -> Self {
        Self(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderSendConsent(());

impl ProviderSendConsent {
    #[must_use]
    pub const fn granted_by_user() -> Self {
        Self(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivateExportConsent(());

impl PrivateExportConsent {
    #[must_use]
    pub const fn granted_by_user() -> Self {
        Self(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Application;

impl Application {
    pub fn resolve_workspace_root(
        explicit: Option<&Path>,
    ) -> Result<std::path::PathBuf, ApplicationError> {
        Ok(Workspace::open(explicit)?.paths.root)
    }

    pub fn resolve_workspace_root_v4(
        explicit: Option<&Path>,
    ) -> Result<std::path::PathBuf, ApplicationError> {
        Ok(open_workspace_v4_discovered(explicit)?.paths.root)
    }
}

pub(crate) fn open_workspace(root: &Path) -> Result<Workspace, StoreError> {
    Workspace::open(Some(root))
}

pub(crate) fn open_workspace_v4(root: &Path) -> Result<Workspace, ApplicationError> {
    open_workspace_v4_discovered(Some(root))
}

fn open_workspace_v4_discovered(explicit: Option<&Path>) -> Result<Workspace, ApplicationError> {
    match Workspace::open_v4(explicit) {
        Ok(workspace) => Ok(workspace),
        Err(StoreError::WorkspaceFormatUnsupported { found, required }) => {
            Err(ApplicationError::CompatibilityUnavailable {
                message: format!("Workspace format {found} is unsupported by the v4 surface"),
                details: serde_json::json!({
                    "found": found,
                    "required": required,
                }),
                remediation: canisend_contracts::NextAction {
                    action: "initialize a clean Workspace v4".to_owned(),
                    description: "Choose a new or empty directory; compatibility detection does not open, migrate, or mutate the unsupported Workspace".to_owned(),
                },
            })
        }
        Err(StoreError::WorkspaceV4StorageUnsupported { found, required }) => {
            Err(ApplicationError::CompatibilityUnavailable {
                message: format!(
                    "Workspace v4 database schema {found} uses the retired application storage bridge"
                ),
                details: serde_json::json!({
                    "found_schema": found,
                    "required_schema": required,
                    "compatibility": "unsupported",
                }),
                remediation: canisend_contracts::NextAction {
                    action: "initialize a clean Workspace v4".to_owned(),
                    description: "Alpha.7 does not migrate or silently reuse pre-native v4 Application storage; create a clean Workspace and import reviewed Sources explicitly".to_owned(),
                },
            })
        }
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn parse_entity_id(value: &str) -> Result<EntityId, ApplicationError> {
    EntityId::try_new(value).map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))
}
