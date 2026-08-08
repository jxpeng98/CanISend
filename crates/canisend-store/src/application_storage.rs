use canisend_contracts::WORKSPACE_V4_FORMAT;
use rusqlite::Connection;

use crate::StoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplicationStorage {
    V3,
    V4,
}

impl ApplicationStorage {
    pub(crate) fn detect(connection: &Connection) -> Result<Self, StoreError> {
        let workspace_format: String = connection.query_row(
            "SELECT workspace_format FROM workspace_metadata WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        if workspace_format == WORKSPACE_V4_FORMAT {
            Ok(Self::V4)
        } else {
            Ok(Self::V3)
        }
    }

    pub(crate) const fn heads(self) -> &'static str {
        match self {
            Self::V3 => "application_model_v3_heads",
            Self::V4 => "application_v4_heads",
        }
    }

    pub(crate) const fn revisions(self) -> &'static str {
        match self {
            Self::V3 => "application_model_v3_revisions",
            Self::V4 => "application_v4_revisions",
        }
    }

    pub(crate) const fn dependencies(self) -> &'static str {
        match self {
            Self::V3 => "application_model_v3_dependencies",
            Self::V4 => "application_v4_dependencies",
        }
    }

    pub(crate) const fn projections(self) -> &'static str {
        match self {
            Self::V3 => "application_projection_v3_manifests",
            Self::V4 => "application_projection_v4_manifests",
        }
    }

    pub(crate) const fn pack_migrations(self) -> &'static str {
        match self {
            Self::V3 => "application_pack_v3_migrations",
            Self::V4 => "application_pack_v4_migrations",
        }
    }

    pub(crate) const fn source_associations(self) -> &'static str {
        match self {
            Self::V3 => "application_source_associations_v4",
            Self::V4 => "application_source_v4_associations",
        }
    }

    pub(crate) const fn profile_associations(self) -> &'static str {
        match self {
            Self::V3 => "application_profile_associations_v4",
            Self::V4 => "application_profile_v4_associations",
        }
    }

    pub(crate) const fn evidence_associations(self) -> &'static str {
        match self {
            Self::V3 => "application_evidence_associations_v4",
            Self::V4 => "application_evidence_v4_associations",
        }
    }
}
