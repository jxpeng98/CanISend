use std::collections::BTreeSet;

use canisend_contracts::{
    ApplicationId, ApplicationPackBindingV3, EntityId, SemanticVersion, Sha256Digest,
    WORKSPACE_V4_FORMAT, WorkflowPackId,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{Database, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LegacyCompatibilityAuthority {
    WorkspaceV2,
    WorkspaceV3,
    WorkspaceV4,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyApplicationBindingV3 {
    pub legacy_job_id: Option<EntityId>,
    pub application_id: ApplicationId,
    pub pack: ApplicationPackBindingV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyCompatibilityContextV3 {
    pub authority: LegacyCompatibilityAuthority,
    pub bindings: Vec<LegacyApplicationBindingV3>,
}

pub struct LegacyCompatibilityService<'a> {
    database: &'a Database,
}

impl<'a> LegacyCompatibilityService<'a> {
    #[must_use]
    pub const fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn workspace_context(&self) -> Result<LegacyCompatibilityContextV3, StoreError> {
        let authority = compatibility_authority(self.database)?;
        if authority != LegacyCompatibilityAuthority::WorkspaceV3 {
            return Ok(LegacyCompatibilityContextV3 {
                authority,
                bindings: Vec::new(),
            });
        }

        let mut statement = self.database.connection().prepare(
            "SELECT link.legacy_job_id, head.application_id,
                    head.pack_id, head.pack_version, head.pack_digest
             FROM application_model_v3_heads AS head
             LEFT JOIN workspace_v3_application_links AS link
               ON link.application_id = head.application_id
             ORDER BY head.created_at, head.application_id",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut bindings = Vec::with_capacity(rows.len());
        let mut application_ids = BTreeSet::new();
        let mut legacy_job_ids = BTreeSet::new();
        for (legacy_job_id, application_id, pack_id, pack_version, pack_digest) in rows {
            if !application_ids.insert(application_id.clone())
                || legacy_job_id
                    .as_ref()
                    .is_some_and(|job_id| !legacy_job_ids.insert(job_id.clone()))
            {
                return Err(StoreError::ApplicationModelIntegrity(
                    "Workspace v3 legacy links are not globally one-to-one".to_owned(),
                ));
            }
            bindings.push(parse_binding(
                legacy_job_id,
                application_id,
                pack_id,
                pack_version,
                pack_digest,
            )?);
        }
        let link_count: i64 = self.database.connection().query_row(
            "SELECT COUNT(*) FROM workspace_v3_application_links",
            [],
            |row| row.get(0),
        )?;
        let linked_binding_count = i64::try_from(
            bindings
                .iter()
                .filter(|binding| binding.legacy_job_id.is_some())
                .count(),
        )
        .map_err(|_| {
            StoreError::ApplicationModelIntegrity(
                "legacy compatibility binding count exceeds SQLite limits".to_owned(),
            )
        })?;
        if link_count != linked_binding_count {
            return Err(StoreError::ApplicationModelIntegrity(
                "Workspace v3 legacy links do not map one-to-one to current Applications"
                    .to_owned(),
            ));
        }
        Ok(LegacyCompatibilityContextV3 {
            authority: LegacyCompatibilityAuthority::WorkspaceV3,
            bindings,
        })
    }

    pub fn job_context(
        &self,
        legacy_job_id: &EntityId,
    ) -> Result<LegacyCompatibilityContextV3, StoreError> {
        let authority = compatibility_authority(self.database)?;
        if authority != LegacyCompatibilityAuthority::WorkspaceV3 {
            return Ok(LegacyCompatibilityContextV3 {
                authority,
                bindings: Vec::new(),
            });
        }
        let row = self
            .database
            .connection()
            .query_row(
                "SELECT link.legacy_job_id, head.application_id,
                        head.pack_id, head.pack_version, head.pack_digest
                 FROM workspace_v3_application_links AS link
                 JOIN application_model_v3_heads AS head
                   ON head.application_id = link.application_id
                 WHERE link.legacy_job_id = ?1",
                [legacy_job_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((legacy_job_id, application_id, pack_id, pack_version, pack_digest)) = row else {
            return Err(StoreError::ApplicationModelConflict(format!(
                "legacy Job {legacy_job_id} has no verified Workspace v3 Application mapping"
            )));
        };
        Ok(LegacyCompatibilityContextV3 {
            authority: LegacyCompatibilityAuthority::WorkspaceV3,
            bindings: vec![parse_binding(
                Some(legacy_job_id),
                application_id,
                pack_id,
                pack_version,
                pack_digest,
            )?],
        })
    }

    pub fn task_context(
        &self,
        task_id: &EntityId,
    ) -> Result<LegacyCompatibilityContextV3, StoreError> {
        if compatibility_authority(self.database)? == LegacyCompatibilityAuthority::WorkspaceV4 {
            return Ok(LegacyCompatibilityContextV3 {
                authority: LegacyCompatibilityAuthority::WorkspaceV4,
                bindings: Vec::new(),
            });
        }
        let legacy_job_id = self
            .database
            .connection()
            .query_row(
                "SELECT job_id FROM tasks WHERE id = ?1",
                params![task_id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| StoreError::TaskNotFound(task_id.to_string()))?;
        self.job_context(&EntityId::try_new(legacy_job_id)?)
    }
}

fn compatibility_authority(
    database: &Database,
) -> Result<LegacyCompatibilityAuthority, StoreError> {
    let workspace_format: String = database.connection().query_row(
        "SELECT workspace_format FROM workspace_metadata WHERE singleton = 1",
        [],
        |row| row.get(0),
    )?;
    if workspace_format == WORKSPACE_V4_FORMAT {
        return Ok(LegacyCompatibilityAuthority::WorkspaceV4);
    }
    let v3_active = database
        .connection()
        .query_row(
            "SELECT 1 FROM workspace_v3_authority WHERE singleton = 1",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(if v3_active {
        LegacyCompatibilityAuthority::WorkspaceV3
    } else {
        LegacyCompatibilityAuthority::WorkspaceV2
    })
}

fn parse_binding(
    legacy_job_id: Option<String>,
    application_id: String,
    pack_id: String,
    pack_version: String,
    pack_digest: String,
) -> Result<LegacyApplicationBindingV3, StoreError> {
    Ok(LegacyApplicationBindingV3 {
        legacy_job_id: legacy_job_id.map(EntityId::try_new).transpose()?,
        application_id: ApplicationId::try_new(application_id)?,
        pack: ApplicationPackBindingV3 {
            id: WorkflowPackId::try_new(pack_id).map_err(|error| {
                StoreError::ApplicationModelIntegrity(format!(
                    "stored compatibility Pack ID is invalid: {error}"
                ))
            })?,
            version: SemanticVersion::try_new(pack_version)?,
            content_digest: Sha256Digest::try_new(pack_digest)?,
        },
    })
}

#[cfg(test)]
mod tests {
    use canisend_contracts::ActorKind;

    use crate::{
        JobService, Workspace, application_v3::activate_workspace_v3_authority, generate_id,
    };

    use super::*;

    #[test]
    fn v2_workspace_has_implicit_legacy_authority() {
        let root =
            std::env::temp_dir().join(format!("canisend-compatibility-v2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let workspace = Workspace::init(&root).expect("workspace");
        let context = LegacyCompatibilityService::new(&workspace.database)
            .workspace_context()
            .expect("context");
        assert_eq!(context.authority, LegacyCompatibilityAuthority::WorkspaceV2);
        assert!(context.bindings.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn v4_workspace_is_identified_before_any_legacy_entity_lookup() {
        let root =
            std::env::temp_dir().join(format!("canisend-compatibility-v4-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let workspace = Workspace::init_v4(&root).expect("Workspace v4");
        let service = LegacyCompatibilityService::new(&workspace.database);
        let missing = generate_id().expect("synthetic ID");

        for context in [
            service.workspace_context().expect("Workspace context"),
            service.job_context(&missing).expect("Job context"),
            service.task_context(&missing).expect("task context"),
        ] {
            assert_eq!(context.authority, LegacyCompatibilityAuthority::WorkspaceV4);
            assert!(context.bindings.is_empty());
        }

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn active_v3_without_a_legacy_application_mapping_fails_closed() {
        let root =
            std::env::temp_dir().join(format!("canisend-compatibility-v3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut workspace = Workspace::init(&root).expect("workspace");
        let job = JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Role", "Institution", ActorKind::User)
            .expect("job");
        activate_workspace_v3_authority(
            &mut workspace.database,
            ActorKind::User,
            "test-activation",
        )
        .expect("authority");
        let error = LegacyCompatibilityService::new(&workspace.database)
            .job_context(&job.id)
            .expect_err("unmapped job must fail");
        assert!(matches!(error, StoreError::ApplicationModelConflict(_)));
        let _ = std::fs::remove_dir_all(root);
    }
}
