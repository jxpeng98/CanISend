#![forbid(unsafe_code)]

use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use canisend_contracts::{AGENT_PROTOCOL, RESOURCE_FORMAT};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentHost {
    Codex,
    Claude,
    Generic,
}

impl AgentHost {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPackFile {
    pub resource_id: String,
    pub resource_version: String,
    pub path: String,
    pub size: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPackManifest {
    pub format: String,
    pub product_version: String,
    pub protocol: String,
    pub resource_format: String,
    pub host: AgentHost,
    pub files: Vec<AgentPackFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPackExportData {
    pub directory: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: AgentPackManifest,
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

pub fn export_agent_pack(
    host: AgentHost,
    root: &Path,
) -> Result<AgentPackExportData, ResourceError> {
    verify().map_err(|_| ResourceError::UnsafeExportPath(root.to_path_buf()))?;
    ensure_empty_pack_root(root)?;
    let guide = match host {
        AgentHost::Codex => ("agent.codex.guide", "AGENTS.md"),
        AgentHost::Claude => ("agent.claude.guide", "CLAUDE.md"),
        AgentHost::Generic => ("agent.generic.guide", "README.md"),
    };
    let resources = [
        guide,
        ("prompt.job-parse", "prompts/job-parse.md"),
        ("prompt.evidence-normalize", "prompts/evidence-normalize.md"),
        ("prompt.evidence-match", "prompts/evidence-match.md"),
        ("prompt.document-draft", "prompts/document-draft.md"),
        ("prompt.document-review", "prompts/document-review.md"),
        ("example.task-complete", "examples/task-complete.json"),
        (
            "schema.task-descriptor",
            "schemas/v2/task-descriptor.schema.json",
        ),
        (
            "schema.task-completion",
            "schemas/v2/task-completion.schema.json",
        ),
        ("schema.parsed-job", "schemas/v2/parsed-job.schema.json"),
        ("schema.criterion", "schemas/v2/criterion.schema.json"),
        ("schema.criteria", "schemas/v2/criteria.schema.json"),
        (
            "schema.evidence-proposals",
            "schemas/v2/evidence-proposals.schema.json",
        ),
        (
            "schema.evidence-catalog",
            "schemas/v2/evidence-catalog.schema.json",
        ),
        ("schema.evidence", "schemas/v2/evidence.schema.json"),
        (
            "schema.evidence-match-proposals",
            "schemas/v2/evidence-match-proposals.schema.json",
        ),
        (
            "schema.evidence-matches",
            "schemas/v2/evidence-matches.schema.json",
        ),
        (
            "schema.application-plan-candidate",
            "schemas/v2/application-plan-candidate.schema.json",
        ),
        (
            "schema.application-plan",
            "schemas/v2/application-plan.schema.json",
        ),
        (
            "schema.document-candidate",
            "schemas/v2/document-candidate.schema.json",
        ),
        ("schema.document", "schemas/v2/document.schema.json"),
        ("schema.document-set", "schemas/v2/document-set.schema.json"),
        (
            "schema.review-candidate",
            "schemas/v2/review-candidate.schema.json",
        ),
        (
            "schema.review-findings",
            "schemas/v2/review-findings.schema.json",
        ),
        (
            "schema.review-disposition-candidate",
            "schemas/v2/review-disposition-candidate.schema.json",
        ),
        (
            "schema.package-manifest",
            "schemas/v2/package-manifest.schema.json",
        ),
        (
            "schema.package-export-manifest",
            "schemas/v2/package-export-manifest.schema.json",
        ),
        ("schema.projection", "schemas/v2/projection.schema.json"),
        (
            "schema.projection-reconcile",
            "schemas/v2/projection-reconcile.schema.json",
        ),
        (
            "schema.rendered-document",
            "schemas/v2/rendered-document.schema.json",
        ),
        (
            "schema.render-manifest",
            "schemas/v2/render-manifest.schema.json",
        ),
    ];
    let mut files = Vec::with_capacity(resources.len());
    for (resource_id, relative_path) in resources {
        let resource_id = ResourceId::from_str(resource_id)?;
        let resource = get(resource_id);
        let destination = root.join(relative_path);
        write_new_file(root, &destination, resource.bytes)?;
        files.push(AgentPackFile {
            resource_id: resource.descriptor.id.to_owned(),
            resource_version: resource.descriptor.version.to_owned(),
            path: relative_path.to_owned(),
            size: resource.descriptor.size,
            sha256: resource.descriptor.sha256.to_owned(),
        });
    }
    let manifest = AgentPackManifest {
        format: "canisend.agent-pack/v2".to_owned(),
        product_version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol: AGENT_PROTOCOL.to_owned(),
        resource_format: RESOURCE_FORMAT.to_owned(),
        host,
        files,
    };
    let mut manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|_| ResourceError::UnsafeExportPath(root.to_path_buf()))?;
    manifest_bytes.push(b'\n');
    let manifest_path = root.join("canisend-agent-pack.json");
    write_new_file(root, &manifest_path, &manifest_bytes)?;
    Ok(AgentPackExportData {
        directory: root.to_path_buf(),
        manifest_path,
        manifest,
    })
}

fn ensure_empty_pack_root(root: &Path) -> Result<(), ResourceError> {
    if root
        .components()
        .any(|component| component.as_os_str().eq_ignore_ascii_case(".canisend"))
    {
        return Err(ResourceError::UnsafeExportPath(root.to_path_buf()));
    }
    if let Ok(metadata) = fs::symlink_metadata(root) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ResourceError::UnsafeExportPath(root.to_path_buf()));
        }
        let empty = fs::read_dir(root)
            .map_err(|source| ResourceError::ExportIo {
                path: root.to_path_buf(),
                source,
            })?
            .next()
            .is_none();
        if !empty {
            return Err(ResourceError::UnsafeExportPath(root.to_path_buf()));
        }
    } else {
        let parent = root
            .parent()
            .ok_or_else(|| ResourceError::UnsafeExportPath(root.to_path_buf()))?;
        let parent_metadata =
            fs::symlink_metadata(parent).map_err(|source| ResourceError::ExportIo {
                path: parent.to_path_buf(),
                source,
            })?;
        if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
            return Err(ResourceError::UnsafeExportPath(parent.to_path_buf()));
        }
        fs::create_dir(root).map_err(|source| ResourceError::ExportIo {
            path: root.to_path_buf(),
            source,
        })?;
    }
    set_private_directory_permissions(root)?;
    Ok(())
}

fn write_new_file(root: &Path, destination: &Path, bytes: &[u8]) -> Result<(), ResourceError> {
    let parent = destination
        .parent()
        .ok_or_else(|| ResourceError::UnsafeExportPath(destination.to_path_buf()))?;
    let relative = parent
        .strip_prefix(root)
        .map_err(|_| ResourceError::UnsafeExportPath(destination.to_path_buf()))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        if let Ok(metadata) = fs::symlink_metadata(&current) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(ResourceError::UnsafeExportPath(current));
            }
        } else {
            fs::create_dir(&current).map_err(|source| ResourceError::ExportIo {
                path: current.clone(),
                source,
            })?;
            set_private_directory_permissions(&current)?;
        }
    }
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)
        .map_err(|source| ResourceError::ExportIo {
            path: destination.to_path_buf(),
            source,
        })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|source| ResourceError::ExportIo {
            path: destination.to_path_buf(),
            source,
        })?;
    set_private_file_permissions(destination)
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> Result<(), ResourceError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|source| {
        ResourceError::ExportIo {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> Result<(), ResourceError> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<(), ResourceError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|source| {
        ResourceError::ExportIo {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<(), ResourceError> {
    Ok(())
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
