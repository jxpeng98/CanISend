use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use canisend_contracts::Sha256Digest;
use sha2::{Digest, Sha256};

use crate::{StoreError, generate_id, io_error};

pub const DEFAULT_MAX_BLOB_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobAudit {
    pub present: BTreeSet<Sha256Digest>,
    pub unreferenced: BTreeSet<Sha256Digest>,
}

#[derive(Debug, Clone)]
pub struct BlobStore {
    root: PathBuf,
    temporary: PathBuf,
}

impl BlobStore {
    #[must_use]
    pub fn new(root: PathBuf, temporary: PathBuf) -> Self {
        Self { root, temporary }
    }

    pub fn put_bytes(&self, bytes: &[u8]) -> Result<Sha256Digest, StoreError> {
        self.put_reader(bytes, DEFAULT_MAX_BLOB_BYTES)
    }

    pub fn put_reader<R: Read>(
        &self,
        mut reader: R,
        limit: u64,
    ) -> Result<Sha256Digest, StoreError> {
        self.ensure_layout()?;
        let temporary_path = self.temporary.join(format!("blob-{}.tmp", generate_id()?));
        let mut guard = TemporaryFile::create(&temporary_path)?;
        let mut hasher = Sha256::new();
        let mut total = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = reader
                .read(&mut buffer)
                .map_err(|source| io_error(&temporary_path, source))?;
            if count == 0 {
                break;
            }
            total = total
                .checked_add(u64::try_from(count).expect("buffer length fits u64"))
                .ok_or(StoreError::BlobTooLarge { limit })?;
            if total > limit {
                return Err(StoreError::BlobTooLarge { limit });
            }
            hasher.update(&buffer[..count]);
            guard
                .file
                .write_all(&buffer[..count])
                .map_err(|source| io_error(&temporary_path, source))?;
        }
        guard
            .file
            .sync_all()
            .map_err(|source| io_error(&temporary_path, source))?;
        let digest = Sha256Digest::try_new(hex::encode(hasher.finalize()))?;
        let destination = self.path_for(&digest);
        let parent = destination
            .parent()
            .expect("blob digest path always has a parent");
        ensure_directory(parent)?;

        match fs::hard_link(&temporary_path, &destination) {
            Ok(()) => {
                guard.remove()?;
                sync_directory(parent)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if self.verify_path(&digest, &destination, limit).is_err() {
                    return Err(StoreError::BlobCollision(destination));
                }
                guard.remove()?;
            }
            Err(source) => return Err(io_error(&destination, source)),
        }
        self.verify_path(&digest, &destination, limit)?;
        Ok(digest)
    }

    pub fn read_verified(&self, digest: &Sha256Digest, limit: u64) -> Result<Vec<u8>, StoreError> {
        self.ensure_layout()?;
        let path = self.path_for(digest);
        self.verify_path(digest, &path, limit)?;
        let metadata = fs::symlink_metadata(&path).map_err(|source| io_error(&path, source))?;
        if metadata.len() > limit {
            return Err(StoreError::BlobTooLarge { limit });
        }
        fs::read(&path).map_err(|source| io_error(path, source))
    }

    pub fn verify(&self, digest: &Sha256Digest, limit: u64) -> Result<u64, StoreError> {
        self.ensure_layout()?;
        let path = self.path_for(digest);
        self.verify_path(digest, &path, limit)
    }

    fn verify_path(
        &self,
        digest: &Sha256Digest,
        path: &Path,
        limit: u64,
    ) -> Result<u64, StoreError> {
        let metadata = fs::symlink_metadata(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                StoreError::BlobMissing(digest.to_string())
            } else {
                io_error(path, error)
            }
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(StoreError::UnsafePath(path.to_path_buf()));
        }
        if metadata.len() > limit {
            return Err(StoreError::BlobTooLarge { limit });
        }
        let file = File::open(path).map_err(|source| io_error(path, source))?;
        let actual = digest_reader(file, limit)?;
        if &actual != digest {
            return Err(StoreError::BlobDigestMismatch {
                expected: digest.to_string(),
                actual: actual.to_string(),
            });
        }
        Ok(metadata.len())
    }

    #[must_use]
    pub fn path_for(&self, digest: &Sha256Digest) -> PathBuf {
        self.root.join(&digest.as_str()[..2]).join(digest.as_str())
    }

    pub fn audit(&self, referenced: &BTreeSet<String>) -> Result<BlobAudit, StoreError> {
        self.ensure_layout()?;
        let mut present = BTreeSet::new();
        if !self.root.exists() {
            return Ok(BlobAudit {
                present,
                unreferenced: BTreeSet::new(),
            });
        }
        for prefix_entry in
            fs::read_dir(&self.root).map_err(|source| io_error(&self.root, source))?
        {
            let prefix_entry = prefix_entry.map_err(|source| io_error(&self.root, source))?;
            let prefix_path = prefix_entry.path();
            let metadata = fs::symlink_metadata(&prefix_path)
                .map_err(|source| io_error(&prefix_path, source))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(StoreError::UnsafePath(prefix_path));
            }
            for blob_entry in
                fs::read_dir(&prefix_path).map_err(|source| io_error(&prefix_path, source))?
            {
                let blob_entry = blob_entry.map_err(|source| io_error(&prefix_path, source))?;
                let path = blob_entry.path();
                let metadata =
                    fs::symlink_metadata(&path).map_err(|source| io_error(&path, source))?;
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(StoreError::UnsafePath(path));
                }
                let name = blob_entry.file_name().to_string_lossy().into_owned();
                let digest = Sha256Digest::try_new(name)?;
                if digest.as_str()[..2] != *prefix_entry.file_name().to_string_lossy() {
                    return Err(StoreError::UnsafePath(path));
                }
                present.insert(digest);
            }
        }
        let unreferenced = present
            .iter()
            .filter(|digest| !referenced.contains(digest.as_str()))
            .cloned()
            .collect();
        Ok(BlobAudit {
            present,
            unreferenced,
        })
    }

    fn ensure_layout(&self) -> Result<(), StoreError> {
        for path in [
            self.root.parent().and_then(Path::parent),
            self.root.parent(),
            Some(self.root.as_path()),
            self.temporary.parent(),
            Some(self.temporary.as_path()),
        ]
        .into_iter()
        .flatten()
        {
            validate_directory(path)?;
        }
        Ok(())
    }
}

fn digest_reader<R: Read>(mut reader: R, limit: u64) -> Result<Sha256Digest, StoreError> {
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|source| io_error("<blob-read>", source))?;
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

fn ensure_directory(path: &Path) -> Result<(), StoreError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err(StoreError::UnsafePath(path.to_path_buf()));
        }
        if !metadata.is_dir() {
            return Err(StoreError::NotDirectory(path.to_path_buf()));
        }
        return Ok(());
    }
    create_private_directory(path)
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

fn create_private_directory(path: &Path) -> Result<(), StoreError> {
    fs::create_dir_all(path).map_err(|source| io_error(path, source))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|source| io_error(path, source))?;
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), StoreError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error(path, source))
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), StoreError> {
    Ok(())
}

struct TemporaryFile {
    path: PathBuf,
    file: File,
    removed: bool,
}

impl TemporaryFile {
    fn create(path: &Path) -> Result<Self, StoreError> {
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
            .open(path)
            .map_err(|source| io_error(path, source))?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            removed: false,
        })
    }

    fn remove(&mut self) -> Result<(), StoreError> {
        fs::remove_file(&self.path).map_err(|source| io_error(&self.path, source))?;
        self.removed = true;
        Ok(())
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if !self.removed {
            let _ = fs::remove_file(&self.path);
        }
    }
}
