#![forbid(unsafe_code)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const RESOURCE_VERSION: &str = "canisend.resources/v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceKind {
    Agent,
    Example,
    Prompt,
    Schema,
    Template,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ResourceDescriptor {
    pub id: &'static str,
    pub kind: ResourceKind,
    pub path: &'static str,
    pub version: &'static str,
    pub size: usize,
    pub sha256: &'static str,
}

#[derive(Debug)]
pub struct EmbeddedResource {
    pub id: ResourceId,
    pub descriptor: ResourceDescriptor,
    pub bytes: &'static [u8],
}

#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("unknown embedded resource ID: {0}")]
    UnknownId(String),
    #[error("resource export path is unsafe: {0}")]
    UnsafeExportPath(PathBuf),
    #[error("resource export failed at {path}: {source}")]
    ExportIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

include!(concat!(env!("OUT_DIR"), "/resource_manifest.rs"));

#[must_use]
pub fn manifest() -> Vec<ResourceDescriptor> {
    EMBEDDED_RESOURCES
        .iter()
        .map(|resource| resource.descriptor)
        .collect()
}

#[must_use]
pub fn get(id: ResourceId) -> &'static EmbeddedResource {
    EMBEDDED_RESOURCES
        .iter()
        .find(|resource| resource.id == id)
        .expect("generated ResourceId always has one embedded resource")
}

pub fn verify() -> Result<(), String> {
    if EMBEDDED_RESOURCES.len() != ResourceId::ALL.len() {
        return Err("resource ID and embedded resource counts differ".to_owned());
    }
    for resource in EMBEDDED_RESOURCES {
        let actual = hex::encode(Sha256::digest(resource.bytes));
        if actual != resource.descriptor.sha256 {
            return Err(format!(
                "embedded resource digest mismatch: {}",
                resource.id
            ));
        }
        if resource.descriptor.kind == ResourceKind::Schema
            && resource.descriptor.version != canisend_contracts::PUBLIC_SCHEMA_VERSION
        {
            return Err(format!("embedded schema version mismatch: {}", resource.id));
        }
    }
    Ok(())
}

pub fn export(id: ResourceId, root: &Path) -> Result<PathBuf, ResourceError> {
    let resource = get(id);
    ensure_export_root(root)?;
    let components = resource.descriptor.path.split('/').collect::<Vec<_>>();
    let mut directory = root.to_path_buf();
    for component in &components[..components.len() - 1] {
        directory.push(component);
        ensure_directory(&directory)?;
    }
    let destination = root.join(resource.descriptor.path);
    if let Ok(metadata) = fs::symlink_metadata(&destination)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(ResourceError::UnsafeExportPath(destination));
    }
    fs::write(&destination, resource.bytes).map_err(|source| ResourceError::ExportIo {
        path: destination.clone(),
        source,
    })?;
    Ok(destination)
}

pub fn export_all(root: &Path) -> Result<Vec<PathBuf>, ResourceError> {
    ResourceId::ALL
        .into_iter()
        .map(|resource_id| export(resource_id, root))
        .collect()
}

fn ensure_export_root(root: &Path) -> Result<(), ResourceError> {
    if let Ok(metadata) = fs::symlink_metadata(root) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ResourceError::UnsafeExportPath(root.to_path_buf()));
        }
        return Ok(());
    }
    fs::create_dir_all(root).map_err(|source| ResourceError::ExportIo {
        path: root.to_path_buf(),
        source,
    })
}

fn ensure_directory(path: &Path) -> Result<(), ResourceError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ResourceError::UnsafeExportPath(path.to_path_buf()));
        }
        return Ok(());
    }
    fs::create_dir(path).map_err(|source| ResourceError::ExportIo {
        path: path.to_path_buf(),
        source,
    })
}
