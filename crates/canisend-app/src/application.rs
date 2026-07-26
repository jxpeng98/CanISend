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

#[derive(Debug, Clone, Copy, Default)]
pub struct Application;

pub(crate) fn open_workspace(root: &Path) -> Result<Workspace, StoreError> {
    Workspace::open(Some(root))
}

pub(crate) fn parse_entity_id(value: &str) -> Result<EntityId, ApplicationError> {
    EntityId::try_new(value).map_err(|error| ApplicationError::InvalidEntityId(error.to_string()))
}
