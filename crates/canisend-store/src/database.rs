use std::{collections::BTreeSet, path::Path, time::Duration};

use canisend_contracts::{EntityId, UtcTimestamp, WORKSPACE_FORMAT, WorkspaceStatusData};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};

use crate::{StoreError, now_utc};

pub const DATABASE_SCHEMA_VERSION: u32 = 4;
const INITIAL_MIGRATION: &str = include_str!("../migrations/0001_initial.sql");
const INTAKE_MIGRATION: &str = include_str!("../migrations/0002_job_intake.sql");
const DISCOVERY_MIGRATION: &str = include_str!("../migrations/0003_discovery.sql");
const AGENT_TASK_MIGRATION: &str = include_str!("../migrations/0004_agent_tasks.sql");

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;
        let mut database = Self { connection };
        database.configure()?;
        database.migrate()?;
        Ok(database)
    }

    fn configure(&mut self) -> Result<(), StoreError> {
        self.connection.busy_timeout(Duration::from_secs(2))?;
        self.connection.pragma_update(None, "foreign_keys", true)?;
        self.connection.pragma_update(None, "journal_mode", "WAL")?;
        self.connection.pragma_update(None, "synchronous", "FULL")?;
        Ok(())
    }

    fn migrate(&mut self) -> Result<(), StoreError> {
        let version: u32 = self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > DATABASE_SCHEMA_VERSION {
            return Err(StoreError::Invariant(format!(
                "database schema {version} is newer than supported {DATABASE_SCHEMA_VERSION}"
            )));
        }
        let mut version = version;
        if version == 0 {
            let applied_at = now_utc()?;
            self.apply_migration(1, INITIAL_MIGRATION, &applied_at)?;
            version = 1;
        }
        if version == 1 {
            let applied_at = now_utc()?;
            self.apply_migration(2, INTAKE_MIGRATION, &applied_at)?;
            version = 2;
        }
        if version == 2 {
            let applied_at = now_utc()?;
            self.apply_migration(3, DISCOVERY_MIGRATION, &applied_at)?;
            version = 3;
        }
        if version == 3 {
            let applied_at = now_utc()?;
            self.apply_migration(4, AGENT_TASK_MIGRATION, &applied_at)?;
        }
        Ok(())
    }

    fn apply_migration(
        &mut self,
        version: u32,
        sql: &str,
        applied_at: &UtcTimestamp,
    ) -> Result<(), StoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
            params![version, applied_at.as_str()],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn initialize_workspace(
        &mut self,
        workspace_id: &EntityId,
        created_at: &UtcTimestamp,
    ) -> Result<(), StoreError> {
        self.connection.execute(
            "INSERT INTO workspace_metadata(singleton, workspace_id, workspace_format, created_at)
             VALUES (1, ?1, ?2, ?3)",
            params![workspace_id.as_str(), WORKSPACE_FORMAT, created_at.as_str()],
        )?;
        Ok(())
    }

    pub fn workspace_identity(&self) -> Result<(EntityId, UtcTimestamp), StoreError> {
        let (id, created_at): (String, String) = self.connection.query_row(
            "SELECT workspace_id, created_at FROM workspace_metadata WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok((EntityId::try_new(id)?, UtcTimestamp::try_new(created_at)?))
    }

    pub fn status(&self) -> Result<WorkspaceStatusData, StoreError> {
        let (workspace_id, created_at) = self.workspace_identity()?;
        Ok(WorkspaceStatusData {
            workspace_id,
            workspace_format: WORKSPACE_FORMAT.to_owned(),
            created_at,
            database_schema_version: self.connection.pragma_query_value(
                None,
                "user_version",
                |row| row.get(0),
            )?,
            sqlite_version: rusqlite::version().to_owned(),
            journal_mode: self
                .connection
                .pragma_query_value(None, "journal_mode", |row| row.get(0))?,
            job_count: self.count("jobs")?,
            artifact_count: self.count("artifacts")?,
            referenced_blob_count: self.count("blob_references")?,
        })
    }

    fn count(&self, table: &str) -> Result<u64, StoreError> {
        let sql = match table {
            "jobs" => "SELECT COUNT(*) FROM jobs",
            "artifacts" => "SELECT COUNT(*) FROM artifacts",
            "blob_references" => "SELECT COUNT(DISTINCT sha256) FROM blob_references",
            _ => return Err(StoreError::Invariant("unsupported count table".to_owned())),
        };
        let count: i64 = self.connection.query_row(sql, [], |row| row.get(0))?;
        u64::try_from(count)
            .map_err(|_| StoreError::Invariant("negative SQLite row count".to_owned()))
    }

    pub fn integrity_check(&self) -> Result<String, StoreError> {
        self.connection
            .pragma_query_value(None, "integrity_check", |row| row.get(0))
            .map_err(StoreError::from)
    }

    pub fn referenced_digests(&self) -> Result<BTreeSet<String>, StoreError> {
        let mut statement = self
            .connection
            .prepare("SELECT DISTINCT sha256 FROM blob_references ORDER BY sha256")?;
        statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(StoreError::from)
    }

    pub(crate) fn immediate_transaction(&mut self) -> Result<Transaction<'_>, StoreError> {
        self.connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StoreError::from)
    }

    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }

    pub(crate) fn projection_repairs(&self) -> Result<Vec<String>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT DISTINCT artifact_id FROM projection_manifests
             WHERE status = 'repair-required' ORDER BY artifact_id",
        )?;
        statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub(crate) fn stale_artifacts(&self) -> Result<Vec<String>, StoreError> {
        let mut statement = self
            .connection
            .prepare("SELECT id FROM artifacts WHERE stale = 1 ORDER BY id")?;
        statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub(crate) fn metadata_exists(&self) -> Result<bool, StoreError> {
        self.connection
            .query_row(
                "SELECT 1 FROM workspace_metadata WHERE singleton = 1",
                [],
                |_| Ok(true),
            )
            .optional()
            .map(|value| value.unwrap_or(false))
            .map_err(StoreError::from)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use rusqlite::TransactionBehavior;

    use super::{Database, INITIAL_MIGRATION};
    use crate::now_utc;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-database-{label}-{}-{}.sqlite3",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn readers_coexist_and_second_writer_conflicts() {
        let path = path("concurrency");
        let mut first = Database::open(&path).expect("first database");
        let mut second = Database::open(&path).expect("second database");
        assert_eq!(second.integrity_check().expect("reader works"), "ok");

        let transaction = first
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("first writer starts");
        second
            .connection
            .busy_timeout(std::time::Duration::from_millis(5))
            .expect("short timeout");
        assert!(
            second
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .is_err()
        );
        transaction.rollback().expect("rollback writer");
        drop(first);
        drop(second);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn migration_failure_rolls_back_and_corrupt_database_fails_closed() {
        let path = path("migration");
        let mut database = Database::open(&path).expect("database");
        let before: i64 = database
            .connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("migration count");
        assert!(
            database
                .apply_migration(99, "CREATE TABLE broken (", &now_utc().expect("timestamp"))
                .is_err()
        );
        let after: i64 = database
            .connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("migration count after failure");
        assert_eq!(before, after);
        drop(database);
        fs::write(&path, b"not sqlite").expect("corrupt database");
        assert!(Database::open(&path).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn version_one_workspace_upgrades_transactionally_to_current_schema() {
        let path = path("upgrade-v1");
        let connection = rusqlite::Connection::open(&path).expect("version one database");
        connection
            .execute_batch(INITIAL_MIGRATION)
            .expect("initial migration");
        connection
            .execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (1, ?1)",
                [now_utc().expect("timestamp").as_str()],
            )
            .expect("migration record");
        drop(connection);

        let database = Database::open(&path).expect("upgrade database");
        let version: u32 = database
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("schema version");
        assert_eq!(version, 4);
        let revision_column: i64 = database
            .connection
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('jobs') WHERE name = 'revision'",
                [],
                |row| row.get(0),
            )
            .expect("revision column");
        assert_eq!(revision_column, 1);
        let discovery_column: i64 = database
            .connection
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('job_leads') WHERE name = 'promoted_job_id'",
                [],
                |row| row.get(0),
            )
            .expect("discovery column");
        assert_eq!(discovery_column, 1);
        let task_descriptor_column: i64 = database
            .connection
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name = 'descriptor_json'",
                [],
                |row| row.get(0),
            )
            .expect("task descriptor column");
        assert_eq!(task_descriptor_column, 1);
        drop(database);
        let _ = fs::remove_file(path);
    }
}
