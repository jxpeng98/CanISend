use std::{
    env, fs,
    path::{Path, PathBuf},
};

use canisend_contracts::{
    CheckSeverity, EntityId, UtcTimestamp, WORKSPACE_FORMAT, WorkspaceCheckData,
    WorkspaceCheckIssue, WorkspaceStatusData,
};
use serde::{Deserialize, Serialize};

use crate::{
    BlobStore, DEFAULT_MAX_BLOB_BYTES, Database, StoreError, generate_id, io_error, now_utc,
};

const CONFIG_FILE: &str = "canisend.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceConfig {
    pub format: String,
    pub workspace_id: EntityId,
    pub created_at: UtcTimestamp,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageConfig {
    pub database: String,
    pub blob_root: String,
}

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub config: PathBuf,
    pub internal: PathBuf,
    pub database: PathBuf,
    pub blob_container: PathBuf,
    pub blobs: PathBuf,
    pub temporary: PathBuf,
    pub backups: PathBuf,
}

impl WorkspacePaths {
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        let internal = root.join(".canisend");
        Self {
            config: root.join(CONFIG_FILE),
            database: internal.join("state.sqlite3"),
            blob_container: internal.join("blobs"),
            blobs: internal.join("blobs/sha256"),
            temporary: internal.join("tmp"),
            backups: internal.join("backups"),
            internal,
            root,
        }
    }

    pub fn discover(explicit: Option<&Path>, cwd: &Path) -> Result<Self, StoreError> {
        if let Some(explicit) = explicit {
            let root = if explicit.file_name().is_some_and(|name| name == CONFIG_FILE) {
                explicit.parent().unwrap_or(explicit)
            } else {
                explicit
            };
            let paths = Self::new(root.to_path_buf());
            if paths.config.is_file() {
                return Ok(paths);
            }
            return Err(StoreError::WorkspaceNotFound(root.to_path_buf()));
        }
        for candidate in cwd.ancestors() {
            let paths = Self::new(candidate.to_path_buf());
            if paths.config.is_file() {
                return Ok(paths);
            }
        }
        Err(StoreError::WorkspaceNotFound(cwd.to_path_buf()))
    }
}

pub struct Workspace {
    pub paths: WorkspacePaths,
    pub config: WorkspaceConfig,
    pub database: Database,
    pub blobs: BlobStore,
}

impl Workspace {
    pub fn init(root: &Path) -> Result<Self, StoreError> {
        if root.exists() {
            validate_directory(root)?;
        } else {
            create_directory(root, false)?;
        }
        let paths = WorkspacePaths::new(root.to_path_buf());
        if paths.config.exists() {
            return Err(StoreError::WorkspaceExists(root.to_path_buf()));
        }
        for directory in [
            &paths.internal,
            &paths.blob_container,
            &paths.blobs,
            &paths.temporary,
            &paths.backups,
            &paths.root.join("jobs"),
            &paths.root.join("profile"),
            &paths.root.join("agent"),
        ] {
            create_directory(directory, directory.starts_with(&paths.internal))?;
        }

        let workspace_id = generate_id()?;
        let created_at = now_utc()?;
        let config = WorkspaceConfig {
            format: WORKSPACE_FORMAT.to_owned(),
            workspace_id: workspace_id.clone(),
            created_at: created_at.clone(),
            storage: StorageConfig {
                database: ".canisend/state.sqlite3".to_owned(),
                blob_root: ".canisend/blobs/sha256".to_owned(),
            },
        };
        let mut database = Database::open(&paths.database)?;
        database.initialize_workspace(&workspace_id, &created_at)?;
        write_config(&paths.config, &config)?;
        Ok(Self {
            blobs: BlobStore::new(paths.blobs.clone(), paths.temporary.clone()),
            paths,
            config,
            database,
        })
    }

    pub fn open(explicit: Option<&Path>) -> Result<Self, StoreError> {
        let cwd = env::current_dir().map_err(|source| io_error(".", source))?;
        Self::open_from(explicit, &cwd)
    }

    pub fn open_from(explicit: Option<&Path>, cwd: &Path) -> Result<Self, StoreError> {
        let paths = WorkspacePaths::discover(explicit, cwd)?;
        validate_workspace_paths(&paths)?;
        let config_text =
            fs::read_to_string(&paths.config).map_err(|source| io_error(&paths.config, source))?;
        let config: WorkspaceConfig = toml::from_str(&config_text)?;
        if config.format != WORKSPACE_FORMAT
            || config.storage.database != ".canisend/state.sqlite3"
            || config.storage.blob_root != ".canisend/blobs/sha256"
        {
            return Err(StoreError::Invariant(
                "workspace configuration format or storage paths do not match v2".to_owned(),
            ));
        }
        let database = Database::open(&paths.database)?;
        if !database.metadata_exists()? {
            return Err(StoreError::Invariant(
                "workspace database has no identity metadata".to_owned(),
            ));
        }
        let (database_id, database_created_at) = database.workspace_identity()?;
        if database_id != config.workspace_id || database_created_at != config.created_at {
            return Err(StoreError::Invariant(
                "workspace configuration and database identity differ".to_owned(),
            ));
        }
        Ok(Self {
            blobs: BlobStore::new(paths.blobs.clone(), paths.temporary.clone()),
            paths,
            config,
            database,
        })
    }

    pub fn status(&self) -> Result<WorkspaceStatusData, StoreError> {
        self.database.status()
    }

    pub fn check(&self) -> Result<WorkspaceCheckData, StoreError> {
        let integrity = self.database.integrity_check()?;
        let references = self.database.referenced_digests()?;
        let audit = self.blobs.audit(&references)?;
        let mut issues = Vec::new();
        let mut checked = 0_u64;
        for digest in &references {
            let parsed = canisend_contracts::Sha256Digest::try_new(digest)?;
            match self.blobs.verify(&parsed, DEFAULT_MAX_BLOB_BYTES) {
                Ok(_) => checked += 1,
                Err(error) => issues.push(WorkspaceCheckIssue {
                    code: "blob.reference_invalid".to_owned(),
                    severity: CheckSeverity::Error,
                    subject: digest.clone(),
                    message: error.to_string(),
                }),
            }
        }
        for digest in audit
            .present
            .iter()
            .filter(|digest| !references.contains(digest.as_str()))
        {
            if let Err(error) = self.blobs.verify(digest, DEFAULT_MAX_BLOB_BYTES) {
                issues.push(WorkspaceCheckIssue {
                    code: "blob.unreferenced_invalid".to_owned(),
                    severity: CheckSeverity::Error,
                    subject: digest.to_string(),
                    message: error.to_string(),
                });
            }
        }
        for digest in &audit.unreferenced {
            issues.push(WorkspaceCheckIssue {
                code: "blob.unreferenced".to_owned(),
                severity: CheckSeverity::Warning,
                subject: digest.to_string(),
                message: "immutable blob is not referenced; automatic deletion is disabled"
                    .to_owned(),
            });
        }
        if integrity != "ok" {
            issues.push(WorkspaceCheckIssue {
                code: "database.integrity_failed".to_owned(),
                severity: CheckSeverity::Error,
                subject: "state.sqlite3".to_owned(),
                message: integrity.clone(),
            });
        }
        let stale_artifact_ids = self
            .database
            .stale_artifacts()?
            .into_iter()
            .map(EntityId::try_new)
            .collect::<Result<Vec<_>, _>>()?;
        let projection_repairs_required = self
            .database
            .projection_repairs()?
            .into_iter()
            .map(EntityId::try_new)
            .collect::<Result<Vec<_>, _>>()?;
        let ok = !issues
            .iter()
            .any(|issue| issue.severity == CheckSeverity::Error);
        Ok(WorkspaceCheckData {
            workspace_id: self.config.workspace_id.clone(),
            ok,
            database_integrity: integrity,
            checked_referenced_blobs: checked,
            unreferenced_blobs: audit.unreferenced.into_iter().collect(),
            stale_artifact_ids,
            projection_repairs_required,
            issues,
        })
    }
}

fn validate_workspace_paths(paths: &WorkspacePaths) -> Result<(), StoreError> {
    validate_directory(&paths.root)?;
    for directory in [
        &paths.internal,
        &paths.blob_container,
        &paths.blobs,
        &paths.temporary,
        &paths.backups,
    ] {
        validate_directory(directory)?;
    }
    for file in [&paths.config, &paths.database] {
        let metadata = fs::symlink_metadata(file).map_err(|source| io_error(file, source))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(StoreError::UnsafePath(file.to_path_buf()));
        }
    }
    Ok(())
}

fn validate_directory(path: &Path) -> Result<(), StoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if metadata.file_type().is_symlink() {
        return Err(StoreError::UnsafePath(path.to_path_buf()));
    }
    if !metadata.is_dir() {
        return Err(StoreError::NotDirectory(path.to_path_buf()));
    }
    Ok(())
}

fn create_directory(path: &Path, private: bool) -> Result<(), StoreError> {
    if path.exists() {
        return validate_directory(path);
    }
    fs::create_dir_all(path).map_err(|source| io_error(path, source))?;
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|source| io_error(path, source))?;
    }
    #[cfg(not(unix))]
    let _ = private;
    Ok(())
}

fn write_config(path: &Path, config: &WorkspaceConfig) -> Result<(), StoreError> {
    let encoded = toml::to_string_pretty(config)?;
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    use std::io::Write;
    let mut file = options
        .open(path)
        .map_err(|source| io_error(path, source))?;
    file.write_all(encoded.as_bytes())
        .map_err(|source| io_error(path, source))?;
    file.sync_all().map_err(|source| io_error(path, source))
}
