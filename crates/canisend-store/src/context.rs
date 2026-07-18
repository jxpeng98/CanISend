use canisend_contracts::{AgentJobSummary, AgentWorkspaceSummary, EntityId, WORKSPACE_FORMAT};
use rusqlite::{Connection, params};

use crate::{Database, StoreError, job::load_job};

pub struct AgentContextService<'a> {
    database: &'a Database,
}

impl<'a> AgentContextService<'a> {
    #[must_use]
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn workspace_summary(&self) -> Result<AgentWorkspaceSummary, StoreError> {
        let (workspace_id, _) = self.database.workspace_identity()?;
        Ok(AgentWorkspaceSummary {
            workspace_id,
            workspace_format: WORKSPACE_FORMAT.to_owned(),
            active_job_count: count(
                self.database.connection(),
                "SELECT COUNT(*) FROM jobs WHERE archived = 0",
            )?,
            total_job_count: count(self.database.connection(), "SELECT COUNT(*) FROM jobs")?,
            active_lead_count: count(
                self.database.connection(),
                "SELECT COUNT(*) FROM job_leads WHERE status = 'active'",
            )?,
            historical_lead_count: count(
                self.database.connection(),
                "SELECT COUNT(*) FROM job_leads WHERE status != 'active'",
            )?,
            open_task_count: count(
                self.database.connection(),
                "SELECT COUNT(*) FROM tasks WHERE status IN ('prepared', 'leased')",
            )?,
            stale_artifact_count: count(
                self.database.connection(),
                "SELECT COUNT(*) FROM artifacts WHERE stale = 1",
            )?,
        })
    }

    pub fn job_summary(&self, job_id: &EntityId) -> Result<AgentJobSummary, StoreError> {
        let job = load_job(self.database.connection(), job_id)?;
        Ok(AgentJobSummary {
            id: job.id,
            title: job.title,
            institution: job.institution,
            revision: job.revision,
            source_count: u64::try_from(job.source_ids.len())
                .map_err(|_| StoreError::Invariant("source count does not fit u64".to_owned()))?,
            archived: job.archived,
        })
    }
}

fn count(connection: &Connection, sql: &str) -> Result<u64, StoreError> {
    let count: i64 = connection.query_row(sql, params![], |row| row.get(0))?;
    u64::try_from(count).map_err(|_| StoreError::Invariant("negative context count".to_owned()))
}

#[cfg(test)]
mod tests {
    use canisend_contracts::ActorKind;

    use super::AgentContextService;
    use crate::{JobService, Workspace};

    #[test]
    fn summary_is_body_free_and_counts_authoritative_state() {
        let root =
            std::env::temp_dir().join(format!("canisend-agent-context-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut workspace = Workspace::init(&root).expect("workspace");
        let job = JobService::new(&mut workspace.database, &workspace.blobs)
            .create("Lecturer", "University X", ActorKind::User)
            .expect("job");
        let service = AgentContextService::new(&workspace.database);
        let summary = service.workspace_summary().expect("workspace summary");
        assert_eq!(summary.active_job_count, 1);
        let job_summary = service.job_summary(&job.id).expect("job summary");
        assert_eq!(job_summary.source_count, 0);
        let encoded = serde_json::to_string(&(summary, job_summary)).expect("JSON");
        assert!(!encoded.contains("source body"));
        drop(service);
        drop(workspace);
        std::fs::remove_dir_all(root).expect("remove workspace");
    }
}
