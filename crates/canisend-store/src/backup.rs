use std::{
    fs,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use canisend_contracts::{BackupBlobEntry, BackupManifestData, EntityId, Sha256Digest};
use rusqlite::{Connection, MAIN_DB, OpenFlags};
use sha2::{Digest, Sha256};

use crate::{
    BACKUP_FORMAT, DEFAULT_MAX_BLOB_BYTES, StoreError, Workspace, WorkspaceConfig, generate_id,
    io_error, now_utc,
};

const MANIFEST_FILE: &str = "backup-manifest.json";
const MAX_BACKUP_DATABASE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct BackupResult {
    pub directory: PathBuf,
    pub manifest: BackupManifestData,
}

impl Workspace {
    pub fn backup(&mut self, destination: &Path) -> Result<BackupResult, StoreError> {
        ensure_destination_available(destination)?;
        let staging = destination.with_extension(format!("partial-{}", generate_id()?));
        if staging.exists() {
            return Err(StoreError::BackupInvalid(format!(
                "staging path already exists: {}",
                staging.display()
            )));
        }
        let mut staging_guard = TemporaryDirectory::create(staging)?;
        let staging = staging_guard.path();
        let staging_internal = staging.join(".canisend");
        let staging_blobs = staging_internal.join("blobs/sha256");
        create_private_directory(&staging_internal)?;
        create_private_directory(&staging_blobs)?;

        let backup_database = staging_internal.join("state.sqlite3");
        self.database.connection().execute_batch("BEGIN")?;
        let backup_result = (|| {
            let references = self.database.referenced_digests()?;
            self.database
                .connection()
                .backup(MAIN_DB, &backup_database, None)?;
            Ok::<_, StoreError>(references)
        })();
        self.database
            .connection()
            .execute_batch(if backup_result.is_ok() {
                "COMMIT"
            } else {
                "ROLLBACK"
            })?;
        let references = backup_result?;

        let configuration_bytes =
            fs::read(&self.paths.config).map_err(|source| io_error(&self.paths.config, source))?;
        write_private_file(&staging.join("canisend.toml"), &configuration_bytes)?;
        let mut blob_entries = Vec::new();
        for digest in references {
            let digest = Sha256Digest::try_new(digest)?;
            let bytes = self.blobs.read_verified(&digest, DEFAULT_MAX_BLOB_BYTES)?;
            let path = staging_blobs
                .join(&digest.as_str()[..2])
                .join(digest.as_str());
            create_private_directory(path.parent().expect("backup blob path always has a parent"))?;
            write_private_file(&path, &bytes)?;
            blob_entries.push(BackupBlobEntry {
                sha256: digest,
                size: u64::try_from(bytes.len()).expect("blob length fits u64"),
            });
        }
        blob_entries.sort_unstable_by(|left, right| left.sha256.cmp(&right.sha256));
        let manifest = BackupManifestData {
            format: BACKUP_FORMAT.to_owned(),
            workspace_id: self.config.workspace_id.clone(),
            created_at: now_utc()?,
            database_sha256: digest_file(&backup_database, MAX_BACKUP_DATABASE_BYTES)?,
            configuration_sha256: digest_bytes(&configuration_bytes)?,
            blobs: blob_entries,
        };
        let mut manifest_json = serde_json::to_string_pretty(&manifest)?;
        manifest_json.push('\n');
        write_private_file(&staging.join(MANIFEST_FILE), manifest_json.as_bytes())?;
        verify_backup(staging)?;
        fs::rename(staging, destination).map_err(|source| io_error(destination, source))?;
        staging_guard.persist();
        Ok(BackupResult {
            directory: destination.to_path_buf(),
            manifest,
        })
    }

    pub fn restore(backup: &Path, destination: &Path) -> Result<Self, StoreError> {
        verify_backup(backup)?;
        ensure_destination_available(destination)?;
        let staging = destination.with_extension(format!("partial-{}", generate_id()?));
        let mut staging_guard = TemporaryDirectory::create(staging)?;
        let staging = staging_guard.path();
        copy_tree(backup, staging)?;
        let manifest_path = staging.join(MANIFEST_FILE);
        fs::remove_file(&manifest_path).map_err(|source| io_error(&manifest_path, source))?;
        for directory in [
            staging.join(".canisend/tmp"),
            staging.join(".canisend/backups"),
            staging.join("jobs"),
            staging.join("profile"),
            staging.join("agent"),
        ] {
            create_private_directory(&directory)?;
        }
        let mut staged_workspace = Self::open_from(Some(staging), staging)?;
        let staged_root = staged_workspace.paths.root.clone();
        crate::ProjectionService::new(
            &mut staged_workspace.database,
            &staged_workspace.blobs,
            &staged_root,
        )
        .repair_all()?;
        drop(staged_workspace);
        fs::rename(staging, destination).map_err(|source| io_error(destination, source))?;
        staging_guard.persist();
        Self::open_from(Some(destination), destination)
    }
}

struct TemporaryDirectory {
    path: PathBuf,
    persisted: bool,
}

impl TemporaryDirectory {
    fn create(path: PathBuf) -> Result<Self, StoreError> {
        if path.exists() {
            return Err(StoreError::BackupInvalid(format!(
                "staging path already exists: {}",
                path.display()
            )));
        }
        create_private_directory(&path)?;
        Ok(Self {
            path,
            persisted: false,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn persist(&mut self) {
        self.persisted = true;
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        if !self.persisted {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

pub fn verify_backup(root: &Path) -> Result<BackupManifestData, StoreError> {
    let manifest_path = root.join(MANIFEST_FILE);
    let manifest: BackupManifestData = serde_json::from_slice(
        &fs::read(&manifest_path).map_err(|source| io_error(&manifest_path, source))?,
    )?;
    if manifest.format != BACKUP_FORMAT {
        return Err(StoreError::BackupInvalid(
            "unsupported backup format".to_owned(),
        ));
    }
    let config_path = root.join("canisend.toml");
    let config_bytes = fs::read(&config_path).map_err(|source| io_error(&config_path, source))?;
    if digest_bytes(&config_bytes)? != manifest.configuration_sha256 {
        return Err(StoreError::BackupInvalid(
            "configuration digest mismatch".to_owned(),
        ));
    }
    let config: WorkspaceConfig = toml::from_str(
        std::str::from_utf8(&config_bytes)
            .map_err(|error| StoreError::BackupInvalid(error.to_string()))?,
    )?;
    if config.workspace_id != manifest.workspace_id {
        return Err(StoreError::BackupInvalid(
            "configuration workspace identity mismatch".to_owned(),
        ));
    }

    let database_path = root.join(".canisend/state.sqlite3");
    if digest_file(&database_path, MAX_BACKUP_DATABASE_BYTES)? != manifest.database_sha256 {
        return Err(StoreError::BackupInvalid(
            "database digest mismatch".to_owned(),
        ));
    }
    let connection = Connection::open_with_flags(&database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let integrity: String =
        connection.pragma_query_value(None, "integrity_check", |row| row.get(0))?;
    if integrity != "ok" {
        return Err(StoreError::BackupInvalid(format!(
            "database integrity check failed: {integrity}"
        )));
    }
    let database_id: String = connection.query_row(
        "SELECT workspace_id FROM workspace_metadata WHERE singleton = 1",
        [],
        |row| row.get(0),
    )?;
    if EntityId::try_new(database_id)? != manifest.workspace_id {
        return Err(StoreError::BackupInvalid(
            "database workspace identity mismatch".to_owned(),
        ));
    }
    let referenced = {
        let mut statement = connection.prepare("SELECT DISTINCT sha256 FROM blob_references")?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<std::collections::BTreeSet<_>, _>>()?
    };
    let declared = manifest
        .blobs
        .iter()
        .map(|blob| blob.sha256.to_string())
        .collect::<std::collections::BTreeSet<_>>();
    if referenced != declared {
        return Err(StoreError::BackupInvalid(
            "backup blob manifest does not equal database references".to_owned(),
        ));
    }
    for blob in &manifest.blobs {
        let path = root
            .join(".canisend/blobs/sha256")
            .join(&blob.sha256.as_str()[..2])
            .join(blob.sha256.as_str());
        let metadata = fs::symlink_metadata(&path).map_err(|source| io_error(&path, source))?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() != blob.size
            || digest_file(&path, DEFAULT_MAX_BLOB_BYTES)? != blob.sha256
        {
            return Err(StoreError::BackupInvalid(format!(
                "backup blob failed verification: {}",
                blob.sha256
            )));
        }
    }
    Ok(manifest)
}

fn digest_file(path: &Path, limit: u64) -> Result<Sha256Digest, StoreError> {
    let mut file = File::open(path).map_err(|source| io_error(path, source))?;
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|source| io_error(path, source))?;
        if count == 0 {
            break;
        }
        total += u64::try_from(count).expect("buffer length fits u64");
        if total > limit {
            return Err(StoreError::BlobTooLarge { limit });
        }
        hasher.update(&buffer[..count]);
    }
    Sha256Digest::try_new(hex::encode(hasher.finalize())).map_err(StoreError::from)
}

fn digest_bytes(bytes: &[u8]) -> Result<Sha256Digest, StoreError> {
    Sha256Digest::try_new(hex::encode(Sha256::digest(bytes))).map_err(StoreError::from)
}

fn ensure_destination_available(path: &Path) -> Result<(), StoreError> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(StoreError::UnsafePath(path.to_path_buf()));
    }
    if fs::read_dir(path)
        .map_err(|source| io_error(path, source))?
        .next()
        .is_some()
    {
        return Err(StoreError::BackupInvalid(format!(
            "destination is not empty: {}",
            path.display()
        )));
    }
    fs::remove_dir(path).map_err(|source| io_error(path, source))
}

fn create_private_directory(path: &Path) -> Result<(), StoreError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(StoreError::UnsafePath(path.to_path_buf()));
        }
        return Ok(());
    }
    fs::create_dir_all(path).map_err(|source| io_error(path, source))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|source| io_error(path, source))?;
    }
    Ok(())
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|source| io_error(path, source))?;
    file.write_all(bytes)
        .map_err(|source| io_error(path, source))?;
    file.sync_all().map_err(|source| io_error(path, source))
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), StoreError> {
    let metadata = fs::symlink_metadata(source).map_err(|error| io_error(source, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(StoreError::UnsafePath(source.to_path_buf()));
    }
    create_private_directory(destination)?;
    for entry in fs::read_dir(source).map_err(|error| io_error(source, error))? {
        let entry = entry.map_err(|error| io_error(source, error))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata =
            fs::symlink_metadata(&source_path).map_err(|error| io_error(&source_path, error))?;
        if metadata.file_type().is_symlink() {
            return Err(StoreError::UnsafePath(source_path));
        }
        if metadata.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            let bytes = fs::read(&source_path).map_err(|error| io_error(&source_path, error))?;
            write_private_file(&destination_path, &bytes)?;
        } else {
            return Err(StoreError::UnsafePath(source_path));
        }
    }
    Ok(())
}
