#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use canisend_contracts::{
    AGENT_V4_PROTOCOL, RESOURCE_FORMAT, SafeRelativePath, WORKSPACE_V4_FORMAT,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const RESOURCE_VERSION: &str = "canisend.resources/v2";
pub const AGENT_HOST_RESOURCE_FORMAT: &str = "canisend.agent-host-resources/v4";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceKind {
    Agent,
    Example,
    Prompt,
    Schema,
    Template,
    WorkflowPack,
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
    #[error("embedded resource selection is invalid: {0}")]
    InvalidSelection(String),
    #[error("embedded resources failed verification: {0}")]
    Integrity(String),
    #[error("resource export path is unsafe: {0}")]
    UnsafeExportPath(PathBuf),
    #[error("managed Agent skill was modified outside CanISend: {0}")]
    ManagedSkillModified(PathBuf),
    #[error("Agent skill files are not owned by a CanISend manifest: {0}")]
    UnmanagedSkillFiles(PathBuf),
    #[error(
        "unsupported pre-v4 host resources detected at {0}; remove them explicitly and perform a clean Agent v4 install"
    )]
    UnsupportedHostResources(PathBuf),
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
    pub workspace_format: String,
    pub resource_format: String,
    pub task_resource_model_sha256: String,
    pub host: AgentHost,
    pub files: Vec<AgentPackFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentPackExportData {
    pub directory: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: AgentPackManifest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentSkillsInstallState {
    Installed,
    Updated,
    UpToDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillsManifest {
    pub format: String,
    pub product_version: String,
    pub protocol: String,
    pub workspace_format: String,
    pub resource_format: String,
    pub task_resource_model_sha256: String,
    pub host: AgentHost,
    pub files: Vec<AgentPackFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillsInstallData {
    pub workspace: PathBuf,
    pub directory: PathBuf,
    pub manifest_path: PathBuf,
    pub state: AgentSkillsInstallState,
    pub files: Vec<AgentPackFile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentSkillsStatusState {
    NotInstalled,
    UpToDate,
    UpdateAvailable,
    Incomplete,
    UserModified,
    Unmanaged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillStatus {
    pub id: String,
    pub resource_version: String,
    pub state: AgentSkillsStatusState,
    pub file_count: usize,
    pub installed_file_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillsStatusData {
    pub workspace: PathBuf,
    pub directory: PathBuf,
    pub manifest_path: PathBuf,
    pub host: AgentHost,
    pub bundled_product_version: String,
    pub installed_product_version: Option<String>,
    pub state: AgentSkillsStatusState,
    pub skills: Vec<AgentSkillStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentSkillsUninstallState {
    NotInstalled,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillsUninstallData {
    pub workspace: PathBuf,
    pub directory: PathBuf,
    pub manifest_path: PathBuf,
    pub host: AgentHost,
    pub state: AgentSkillsUninstallState,
    pub removed_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceCatalogFile {
    pub resource_id: String,
    pub kind: ResourceKind,
    pub resource_version: String,
    pub path: String,
    pub size: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceCatalogManifest {
    pub format: String,
    pub product_version: String,
    pub resource_format: String,
    pub files: Vec<ResourceCatalogFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceCatalogExportData {
    pub directory: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: ResourceCatalogManifest,
}

pub const ACADEMIC_JOB_WORKFLOW_PACK_ID: &str = "org.canisend.academic-job";
pub const GENERIC_APPLICATION_WORKFLOW_PACK_ID: &str = "org.canisend.generic-application";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedWorkflowPack {
    id: &'static str,
    manifest_bytes: &'static [u8],
    resources: BTreeMap<SafeRelativePath, Vec<u8>>,
}

impl EmbeddedWorkflowPack {
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    #[must_use]
    pub const fn manifest_bytes(&self) -> &'static [u8] {
        self.manifest_bytes
    }

    #[must_use]
    pub const fn resources(&self) -> &BTreeMap<SafeRelativePath, Vec<u8>> {
        &self.resources
    }

    #[must_use]
    pub fn into_resources(self) -> BTreeMap<SafeRelativePath, Vec<u8>> {
        self.resources
    }
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

#[must_use]
pub fn academic_job_workflow_pack() -> EmbeddedWorkflowPack {
    let resources = [
        ResourceId::PromptJobParse,
        ResourceId::PromptEvidenceNormalize,
        ResourceId::PromptEvidenceMatch,
        ResourceId::PromptDocumentDraft,
        ResourceId::PromptDocumentReview,
        ResourceId::TemplateModernproCoverletter,
        ResourceId::TemplateModernproCv,
    ]
    .into_iter()
    .map(|id| {
        let resource = get(id);
        (
            SafeRelativePath::try_new(resource.descriptor.path)
                .expect("embedded Pack resource paths are build-time validated"),
            resource.bytes.to_vec(),
        )
    })
    .collect();
    EmbeddedWorkflowPack {
        id: ACADEMIC_JOB_WORKFLOW_PACK_ID,
        manifest_bytes: get(ResourceId::WorkflowPackOrgCanisendAcademicJob).bytes,
        resources,
    }
}

#[must_use]
pub fn generic_application_workflow_pack() -> EmbeddedWorkflowPack {
    let resource = get(ResourceId::TemplateApplicationDocument);
    let resources = [(
        SafeRelativePath::try_new(resource.descriptor.path)
            .expect("embedded Pack resource paths are build-time validated"),
        resource.bytes.to_vec(),
    )]
    .into_iter()
    .collect();
    EmbeddedWorkflowPack {
        id: GENERIC_APPLICATION_WORKFLOW_PACK_ID,
        manifest_bytes: get(ResourceId::WorkflowPackOrgCanisendGenericApplication).bytes,
        resources,
    }
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
        if resource.descriptor.kind == ResourceKind::Schema {
            let schema: serde_json::Value =
                serde_json::from_slice(resource.bytes).map_err(|error| {
                    format!("embedded schema is invalid JSON: {}: {error}", resource.id)
                })?;
            let Some(schema_version) = schema
                .get("x-canisend-version")
                .and_then(serde_json::Value::as_str)
            else {
                return Err(format!(
                    "embedded schema has no contract version: {}",
                    resource.id
                ));
            };
            if resource.descriptor.version != schema_version {
                return Err(format!("embedded schema version mismatch: {}", resource.id));
            }
        }
        if resource.descriptor.kind == ResourceKind::WorkflowPack {
            let manifest: serde_json::Value =
                serde_json::from_slice(resource.bytes).map_err(|error| {
                    format!(
                        "embedded workflow Pack is invalid JSON: {}: {error}",
                        resource.id
                    )
                })?;
            if manifest.get("format").and_then(serde_json::Value::as_str)
                != Some("canisend.workflow-pack/v1")
                || manifest.get("version").and_then(serde_json::Value::as_str)
                    != Some(resource.descriptor.version)
            {
                return Err(format!(
                    "embedded workflow Pack metadata mismatch: {}",
                    resource.id
                ));
            }
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

pub fn export_if_missing(id: ResourceId, root: &Path) -> Result<PathBuf, ResourceError> {
    let resource = get(id);
    ensure_export_root(root)?;
    let destination = root.join(resource.descriptor.path);
    match fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(ResourceError::UnsafeExportPath(destination));
        }
        Ok(_) => return Ok(destination),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(ResourceError::ExportIo {
                path: destination,
                source,
            });
        }
    }
    write_new_file(root, &destination, resource.bytes)?;
    Ok(destination)
}

pub fn export_all(root: &Path) -> Result<Vec<PathBuf>, ResourceError> {
    ResourceId::ALL
        .into_iter()
        .map(|resource_id| export(resource_id, root))
        .collect()
}

pub fn export_catalog(
    resource_ids: &[ResourceId],
    root: &Path,
) -> Result<ResourceCatalogExportData, ResourceError> {
    verify().map_err(ResourceError::Integrity)?;
    if resource_ids.is_empty() {
        return Err(ResourceError::InvalidSelection(
            "at least one resource is required".to_owned(),
        ));
    }
    let mut unique = BTreeSet::new();
    let mut resources = Vec::with_capacity(resource_ids.len());
    for resource_id in resource_ids {
        if !unique.insert(*resource_id) {
            return Err(ResourceError::InvalidSelection(format!(
                "duplicate resource ID: {resource_id}"
            )));
        }
        let resource = get(*resource_id);
        validate_resource_path(resource.descriptor.path)?;
        resources.push(resource);
    }

    let root_was_created = fs::symlink_metadata(root).is_err();
    ensure_empty_pack_root(root)?;
    let mut created = Vec::with_capacity(resources.len() + 1);
    let result = (|| {
        let mut files = Vec::with_capacity(resources.len());
        for resource in resources {
            let destination = root.join(resource.descriptor.path);
            write_new_file(root, &destination, resource.bytes)?;
            created.push(destination);
            files.push(ResourceCatalogFile {
                resource_id: resource.descriptor.id.to_owned(),
                kind: resource.descriptor.kind,
                resource_version: resource.descriptor.version.to_owned(),
                path: resource.descriptor.path.to_owned(),
                size: resource.descriptor.size,
                sha256: resource.descriptor.sha256.to_owned(),
            });
        }
        let manifest = ResourceCatalogManifest {
            format: "canisend.resource-catalog-export/v1".to_owned(),
            product_version: env!("CARGO_PKG_VERSION").to_owned(),
            resource_format: RESOURCE_FORMAT.to_owned(),
            files,
        };
        let mut manifest_bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|_| ResourceError::UnsafeExportPath(root.to_path_buf()))?;
        manifest_bytes.push(b'\n');
        let manifest_path = root.join("canisend-resource-catalog.json");
        write_new_file(root, &manifest_path, &manifest_bytes)?;
        created.push(manifest_path.clone());
        Ok(ResourceCatalogExportData {
            directory: root.to_path_buf(),
            manifest_path,
            manifest,
        })
    })();
    if result.is_err() {
        rollback_new_files(root, &created, root_was_created);
    }
    result
}

pub fn export_agent_pack(
    host: AgentHost,
    root: &Path,
) -> Result<AgentPackExportData, ResourceError> {
    verify().map_err(ResourceError::Integrity)?;
    ensure_empty_pack_root(root)?;
    let guide = match host {
        AgentHost::Codex => ("agent.codex.guide", "AGENTS.md"),
        AgentHost::Claude => ("agent.claude.guide", "CLAUDE.md"),
        AgentHost::Generic => ("agent.generic.guide", "README.md"),
    };
    let mut resources = vec![
        (guide.0, guide.1.to_owned()),
        (
            "agent.v4.task-resource-model",
            "agent/v4/task-resource-model.json".to_owned(),
        ),
        (
            "agent.v4.operation-registry",
            "operations/v4/registry.json".to_owned(),
        ),
        (
            "schema.agent-v4.task-request",
            "schemas/agent/v4/task-request.schema.json".to_owned(),
        ),
        (
            "schema.agent-v4.operation-registry",
            "schemas/agent/v4/operation-registry.schema.json".to_owned(),
        ),
        (
            "schema.agent-v4.proposal",
            "schemas/agent/v4/proposal.schema.json".to_owned(),
        ),
        (
            "schema.agent-v4.mutation-preview",
            "schemas/agent/v4/mutation-preview.schema.json".to_owned(),
        ),
        (
            "schema.agent-v4.approval",
            "schemas/agent/v4/approval.schema.json".to_owned(),
        ),
        (
            "schema.agent-v4.commit-request",
            "schemas/agent/v4/commit-request.schema.json".to_owned(),
        ),
        (
            "schema.agent-v4.receipt",
            "schemas/agent/v4/receipt.schema.json".to_owned(),
        ),
        (
            "example.agent-v4.orientation-request",
            "examples/agent-v4/orientation-request.json".to_owned(),
        ),
        (
            "example.agent-v4.source-intake-commit",
            "examples/agent-v4/source-intake-commit.json".to_owned(),
        ),
    ];
    resources.extend(agent_skill_resource_paths(host));
    let mut files = Vec::with_capacity(resources.len());
    for (resource_id, relative_path) in resources {
        let resource_id = ResourceId::from_str(resource_id)?;
        let resource = get(resource_id);
        let destination = root.join(&relative_path);
        write_new_file(root, &destination, resource.bytes)?;
        files.push(AgentPackFile {
            resource_id: resource.descriptor.id.to_owned(),
            resource_version: resource.descriptor.version.to_owned(),
            path: relative_path,
            size: resource.descriptor.size,
            sha256: resource.descriptor.sha256.to_owned(),
        });
    }
    let manifest = AgentPackManifest {
        format: "canisend.agent-pack/v4".to_owned(),
        product_version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol: AGENT_V4_PROTOCOL.to_owned(),
        workspace_format: WORKSPACE_V4_FORMAT.to_owned(),
        resource_format: AGENT_HOST_RESOURCE_FORMAT.to_owned(),
        task_resource_model_sha256: get(ResourceId::AgentV4TaskResourceModel)
            .descriptor
            .sha256
            .to_owned(),
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

pub fn install_agent_skills(
    host: AgentHost,
    workspace: &Path,
) -> Result<AgentSkillsInstallData, ResourceError> {
    verify().map_err(ResourceError::Integrity)?;
    ensure_managed_workspace(workspace)?;
    ensure_no_unsupported_host_resources(host, workspace)?;
    let resources = agent_skill_resource_paths(host);
    let manifest_relative_path = match host {
        AgentHost::Codex => ".agents/canisend-agent-v4.json",
        AgentHost::Claude => ".claude/canisend-agent-v4.json",
        AgentHost::Generic => "canisend-agent-v4.json",
    };
    let manifest_path = workspace.join(manifest_relative_path);
    ensure_managed_parent_chain(workspace, &manifest_path)?;
    let existing = read_agent_skills_manifest(&manifest_path, host)?;
    let previous_files = existing
        .as_ref()
        .map(|manifest| {
            manifest
                .files
                .iter()
                .map(|file| (file.path.as_str(), file.sha256.as_str()))
                .collect::<std::collections::BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let current_paths = resources
        .iter()
        .map(|(_, path)| path.as_str())
        .collect::<BTreeSet<_>>();
    let stale_files = existing
        .as_ref()
        .map(|manifest| {
            manifest
                .files
                .iter()
                .filter(|file| !current_paths.contains(file.path.as_str()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for file in &stale_files {
        let destination = workspace.join(&file.path);
        let Some(current_sha) = managed_file_sha256(workspace, &destination)? else {
            continue;
        };
        if current_sha != file.sha256 {
            return Err(ResourceError::ManagedSkillModified(destination));
        }
    }

    let mut files = Vec::with_capacity(resources.len());
    let mut changed = existing.is_none() || !stale_files.is_empty();
    for (resource_id, relative_path) in &resources {
        validate_resource_path(relative_path)?;
        let resource_id = ResourceId::from_str(resource_id)?;
        let resource = get(resource_id);
        let destination = workspace.join(relative_path);
        if let Some(current_sha) = managed_file_sha256(workspace, &destination)? {
            if current_sha != resource.descriptor.sha256 {
                if previous_files.get(relative_path.as_str()).copied() != Some(current_sha.as_str())
                {
                    return Err(ResourceError::ManagedSkillModified(destination));
                }
                changed = true;
            }
        } else {
            changed = true;
        }
        files.push(AgentPackFile {
            resource_id: resource.descriptor.id.to_owned(),
            resource_version: resource.descriptor.version.to_owned(),
            path: relative_path.clone(),
            size: resource.descriptor.size,
            sha256: resource.descriptor.sha256.to_owned(),
        });
    }

    for ((resource_id, relative_path), file) in resources.iter().zip(&files) {
        let resource = get(ResourceId::from_str(resource_id)?);
        let destination = workspace.join(relative_path);
        let current_matches = managed_file_sha256(workspace, &destination)?
            .is_some_and(|sha256| sha256 == file.sha256);
        if !current_matches {
            write_managed_file(workspace, &destination, resource.bytes)?;
        }
    }
    for file in stale_files {
        let destination = workspace.join(&file.path);
        if managed_file_sha256(workspace, &destination)?.is_some() {
            fs::remove_file(&destination).map_err(|source| ResourceError::ExportIo {
                path: destination,
                source,
            })?;
        }
    }

    let manifest = AgentSkillsManifest {
        format: AGENT_HOST_RESOURCE_FORMAT.to_owned(),
        product_version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol: AGENT_V4_PROTOCOL.to_owned(),
        workspace_format: WORKSPACE_V4_FORMAT.to_owned(),
        resource_format: AGENT_HOST_RESOURCE_FORMAT.to_owned(),
        task_resource_model_sha256: get(ResourceId::AgentV4TaskResourceModel)
            .descriptor
            .sha256
            .to_owned(),
        host,
        files: files.clone(),
    };
    let manifest_matches = existing.as_ref() == Some(&manifest);
    if !manifest_matches {
        let mut bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|_| ResourceError::UnsafeExportPath(manifest_path.clone()))?;
        bytes.push(b'\n');
        write_managed_file(workspace, &manifest_path, &bytes)?;
    }

    let directory = match host {
        AgentHost::Codex => workspace.join(".agents/skills"),
        AgentHost::Claude => workspace.join(".claude/skills"),
        AgentHost::Generic => workspace.join("skills"),
    };
    let state = if existing.is_none() {
        AgentSkillsInstallState::Installed
    } else if changed || !manifest_matches {
        AgentSkillsInstallState::Updated
    } else {
        AgentSkillsInstallState::UpToDate
    };
    Ok(AgentSkillsInstallData {
        workspace: workspace.to_path_buf(),
        directory,
        manifest_path,
        state,
        files,
    })
}

pub fn inspect_agent_skills(
    host: AgentHost,
    workspace: &Path,
) -> Result<AgentSkillsStatusData, ResourceError> {
    verify().map_err(ResourceError::Integrity)?;
    ensure_managed_workspace(workspace)?;
    ensure_no_unsupported_host_resources(host, workspace)?;
    let resources = agent_skill_resource_paths(host);
    let (directory, manifest_path) = agent_skills_paths(host, workspace);
    ensure_managed_parent_chain(workspace, &manifest_path)?;
    let existing = read_agent_skills_manifest(&manifest_path, host)?;
    let previous_files = existing
        .as_ref()
        .map(|manifest| {
            manifest
                .files
                .iter()
                .map(|file| (file.path.as_str(), file.sha256.as_str()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let bundled_version = env!("CARGO_PKG_VERSION");
    let installed_version = existing
        .as_ref()
        .map(|manifest| manifest.product_version.clone());
    let version_changed = existing.as_ref().is_some_and(|manifest| {
        manifest.product_version != bundled_version
            || manifest.protocol != AGENT_V4_PROTOCOL
            || manifest.workspace_format != WORKSPACE_V4_FORMAT
            || manifest.resource_format != AGENT_HOST_RESOURCE_FORMAT
            || manifest.task_resource_model_sha256
                != get(ResourceId::AgentV4TaskResourceModel).descriptor.sha256
    });
    let expected_files = resources
        .iter()
        .map(|(resource_id, path)| {
            let resource = get(ResourceId::from_str(resource_id)?);
            Ok((path.as_str(), resource.descriptor.sha256))
        })
        .collect::<Result<BTreeMap<_, _>, ResourceError>>()?;
    let manifest_changed = existing.as_ref().is_some_and(|manifest| {
        manifest.files.len() != expected_files.len()
            || manifest.files.iter().any(|file| {
                expected_files.get(file.path.as_str()).copied() != Some(file.sha256.as_str())
            })
    });

    let mut skills = Vec::with_capacity(AGENT_SKILLS.len());
    let mut any_installed = false;
    for (skill_id, _, _) in AGENT_SKILLS {
        let files = resources
            .iter()
            .filter(|(resource_id, _)| {
                skill_id_for_resource(resource_id).is_ok_and(|id| id == skill_id)
            })
            .collect::<Vec<_>>();
        let mut installed_file_count = 0;
        let mut missing = false;
        let mut update_available = version_changed || manifest_changed;
        let mut user_modified = false;
        let mut resource_version = None;
        for (resource_id, relative_path) in &files {
            let resource = get(ResourceId::from_str(resource_id)?);
            resource_version.get_or_insert(resource.descriptor.version);
            let destination = workspace.join(relative_path);
            match inspect_skill_file(
                workspace,
                &destination,
                resource.descriptor.sha256,
                previous_files.get(relative_path.as_str()).copied(),
            )? {
                SkillFileState::Missing => missing = true,
                SkillFileState::Current => {
                    installed_file_count += 1;
                    any_installed = true;
                }
                SkillFileState::ManagedOld => {
                    installed_file_count += 1;
                    any_installed = true;
                    update_available = true;
                }
                SkillFileState::UserModified => {
                    installed_file_count += 1;
                    any_installed = true;
                    user_modified = true;
                }
            }
        }
        let state = if existing.is_none() {
            if installed_file_count == 0 {
                AgentSkillsStatusState::NotInstalled
            } else {
                AgentSkillsStatusState::Unmanaged
            }
        } else if user_modified {
            AgentSkillsStatusState::UserModified
        } else if missing {
            AgentSkillsStatusState::Incomplete
        } else if update_available {
            AgentSkillsStatusState::UpdateAvailable
        } else {
            AgentSkillsStatusState::UpToDate
        };
        skills.push(AgentSkillStatus {
            id: skill_id.to_owned(),
            resource_version: resource_version.unwrap_or(RESOURCE_VERSION).to_owned(),
            state,
            file_count: files.len(),
            installed_file_count,
        });
    }

    let current_paths = agent_skill_resource_paths(host)
        .into_iter()
        .map(|(_, path)| path)
        .collect::<BTreeSet<_>>();
    let mut stale_managed = false;
    let mut stale_modified = false;
    if let Some(manifest) = &existing {
        for file in manifest
            .files
            .iter()
            .filter(|file| !current_paths.contains(&file.path))
        {
            let destination = workspace.join(&file.path);
            if let Some(current_sha) = managed_file_sha256(workspace, &destination)? {
                stale_managed = true;
                if current_sha != file.sha256 {
                    stale_modified = true;
                }
            }
        }
    }

    let state = if existing.is_none() {
        if any_installed {
            AgentSkillsStatusState::Unmanaged
        } else {
            AgentSkillsStatusState::NotInstalled
        }
    } else if stale_modified
        || skills
            .iter()
            .any(|skill| skill.state == AgentSkillsStatusState::UserModified)
    {
        AgentSkillsStatusState::UserModified
    } else if skills
        .iter()
        .any(|skill| skill.state == AgentSkillsStatusState::Incomplete)
    {
        AgentSkillsStatusState::Incomplete
    } else if stale_managed
        || skills
            .iter()
            .any(|skill| skill.state == AgentSkillsStatusState::UpdateAvailable)
    {
        AgentSkillsStatusState::UpdateAvailable
    } else {
        AgentSkillsStatusState::UpToDate
    };
    Ok(AgentSkillsStatusData {
        workspace: workspace.to_path_buf(),
        directory,
        manifest_path,
        host,
        bundled_product_version: bundled_version.to_owned(),
        installed_product_version: installed_version,
        state,
        skills,
    })
}

pub fn uninstall_agent_skills(
    host: AgentHost,
    workspace: &Path,
) -> Result<AgentSkillsUninstallData, ResourceError> {
    verify().map_err(ResourceError::Integrity)?;
    ensure_managed_workspace(workspace)?;
    let status = inspect_agent_skills(host, workspace)?;
    if status.state == AgentSkillsStatusState::Unmanaged {
        return Err(ResourceError::UnmanagedSkillFiles(status.directory));
    }
    let Some(manifest) = read_agent_skills_manifest(&status.manifest_path, host)? else {
        return Ok(AgentSkillsUninstallData {
            workspace: workspace.to_path_buf(),
            directory: status.directory,
            manifest_path: status.manifest_path,
            host,
            state: AgentSkillsUninstallState::NotInstalled,
            removed_files: 0,
        });
    };

    for file in &manifest.files {
        let destination = workspace.join(&file.path);
        let Some(current_sha) = managed_file_sha256(workspace, &destination)? else {
            continue;
        };
        if current_sha != file.sha256 {
            return Err(ResourceError::ManagedSkillModified(destination));
        }
    }
    let mut removed_files = 0;
    for file in &manifest.files {
        let destination = workspace.join(&file.path);
        if let Some(current_sha) = managed_file_sha256(workspace, &destination)? {
            if current_sha != file.sha256 {
                return Err(ResourceError::ManagedSkillModified(destination));
            }
            fs::remove_file(&destination).map_err(|source| ResourceError::ExportIo {
                path: destination,
                source,
            })?;
            removed_files += 1;
        }
    }
    fs::remove_file(&status.manifest_path).map_err(|source| ResourceError::ExportIo {
        path: status.manifest_path.clone(),
        source,
    })?;
    prune_agent_skill_directories(host, workspace);
    Ok(AgentSkillsUninstallData {
        workspace: workspace.to_path_buf(),
        directory: status.directory,
        manifest_path: status.manifest_path,
        host,
        state: AgentSkillsUninstallState::Removed,
        removed_files,
    })
}

const AGENT_SKILLS: [(&str, &str, &str); 4] = [
    (
        "canisend-workspace",
        "skill.canisend-workspace",
        "skill.canisend-workspace.openai",
    ),
    (
        "canisend-intake",
        "skill.canisend-intake",
        "skill.canisend-intake.openai",
    ),
    (
        "canisend-materials",
        "skill.canisend-materials",
        "skill.canisend-materials.openai",
    ),
    (
        "canisend-review-export",
        "skill.canisend-review-export",
        "skill.canisend-review-export.openai",
    ),
];

fn agent_skill_resource_paths(host: AgentHost) -> Vec<(&'static str, String)> {
    let root = match host {
        AgentHost::Codex => ".agents/skills",
        AgentHost::Claude => ".claude/skills",
        AgentHost::Generic => "skills",
    };
    let mut resources = Vec::with_capacity(if host == AgentHost::Codex { 8 } else { 4 });
    for (name, skill_id, openai_id) in AGENT_SKILLS {
        resources.push((skill_id, format!("{root}/{name}/SKILL.md")));
        if host == AgentHost::Codex {
            resources.push((openai_id, format!("{root}/{name}/agents/openai.yaml")));
        }
    }
    resources
}

fn agent_skills_paths(host: AgentHost, workspace: &Path) -> (PathBuf, PathBuf) {
    match host {
        AgentHost::Codex => (
            workspace.join(".agents/skills"),
            workspace.join(".agents/canisend-agent-v4.json"),
        ),
        AgentHost::Claude => (
            workspace.join(".claude/skills"),
            workspace.join(".claude/canisend-agent-v4.json"),
        ),
        AgentHost::Generic => (
            workspace.join("skills"),
            workspace.join("canisend-agent-v4.json"),
        ),
    }
}

fn ensure_no_unsupported_host_resources(
    host: AgentHost,
    workspace: &Path,
) -> Result<(), ResourceError> {
    let (host_root, old_manifest) = match host {
        AgentHost::Codex => (workspace.join(".agents"), "canisend-skills.json"),
        AgentHost::Claude => (workspace.join(".claude"), "canisend-skills.json"),
        AgentHost::Generic => (workspace.to_path_buf(), "canisend-skills.json"),
    };
    let old_skill_root = match host {
        AgentHost::Codex | AgentHost::Claude => host_root.join("skills"),
        AgentHost::Generic => workspace.join("skills"),
    };
    let unsupported = [
        host_root.join(old_manifest),
        old_skill_root.join("canisend-application"),
        old_skill_root.join("canisend-job-intake"),
        old_skill_root.join("canisend-application-materials"),
        old_skill_root.join("canisend-application-review"),
    ];
    if let Some(path) = unsupported
        .into_iter()
        .find(|path| fs::symlink_metadata(path).is_ok())
    {
        return Err(ResourceError::UnsupportedHostResources(path));
    }
    Ok(())
}

fn skill_id_for_resource(resource_id: &str) -> Result<&'static str, ResourceError> {
    AGENT_SKILLS
        .iter()
        .find(|(_, skill_id, openai_id)| resource_id == *skill_id || resource_id == *openai_id)
        .map(|(name, _, _)| *name)
        .ok_or_else(|| ResourceError::InvalidSelection(resource_id.to_owned()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillFileState {
    Missing,
    Current,
    ManagedOld,
    UserModified,
}

fn inspect_skill_file(
    root: &Path,
    path: &Path,
    bundled_sha256: &str,
    previous_sha256: Option<&str>,
) -> Result<SkillFileState, ResourceError> {
    let Some(current_sha256) = managed_file_sha256(root, path)? else {
        return Ok(SkillFileState::Missing);
    };
    if current_sha256 == bundled_sha256 {
        Ok(SkillFileState::Current)
    } else if previous_sha256 == Some(current_sha256.as_str()) {
        Ok(SkillFileState::ManagedOld)
    } else {
        Ok(SkillFileState::UserModified)
    }
}

fn managed_file_sha256(root: &Path, path: &Path) -> Result<Option<String>, ResourceError> {
    if !ensure_managed_parent_chain(root, path)? {
        return Ok(None);
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ResourceError::ExportIo {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Ok(Some(String::new()));
    }
    let bytes = fs::read(path).map_err(|source| ResourceError::ExportIo {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Some(hex::encode(Sha256::digest(bytes))))
}

fn ensure_managed_parent_chain(root: &Path, path: &Path) -> Result<bool, ResourceError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ResourceError::UnsafeExportPath(path.to_path_buf()))?;
    let parent = relative
        .parent()
        .ok_or_else(|| ResourceError::UnsafeExportPath(path.to_path_buf()))?;
    let mut current = root.to_path_buf();
    for component in parent.components() {
        current.push(component);
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(source) => {
                return Err(ResourceError::ExportIo {
                    path: current,
                    source,
                });
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ResourceError::UnsafeExportPath(current));
        }
    }
    Ok(true)
}

fn prune_agent_skill_directories(host: AgentHost, workspace: &Path) {
    let root = match host {
        AgentHost::Codex => workspace.join(".agents/skills"),
        AgentHost::Claude => workspace.join(".claude/skills"),
        AgentHost::Generic => workspace.join("skills"),
    };
    for (name, _, _) in AGENT_SKILLS {
        let directory = root.join(name);
        let _ = fs::remove_dir(directory.join("agents"));
        let _ = fs::remove_dir(directory);
    }
}

fn read_agent_skills_manifest(
    manifest_path: &Path,
    host: AgentHost,
) -> Result<Option<AgentSkillsManifest>, ResourceError> {
    let metadata = match fs::symlink_metadata(manifest_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ResourceError::ExportIo {
                path: manifest_path.to_path_buf(),
                source,
            });
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ResourceError::UnsafeExportPath(manifest_path.to_path_buf()));
    }
    let bytes = fs::read(manifest_path).map_err(|source| ResourceError::ExportIo {
        path: manifest_path.to_path_buf(),
        source,
    })?;
    let manifest: AgentSkillsManifest = serde_json::from_slice(&bytes)
        .map_err(|_| ResourceError::UnsafeExportPath(manifest_path.to_path_buf()))?;
    if manifest.format != AGENT_HOST_RESOURCE_FORMAT
        || manifest.protocol != AGENT_V4_PROTOCOL
        || manifest.workspace_format != WORKSPACE_V4_FORMAT
        || manifest.resource_format != AGENT_HOST_RESOURCE_FORMAT
        || manifest.host != host
    {
        return Err(ResourceError::UnsupportedHostResources(
            manifest_path.to_path_buf(),
        ));
    }
    let mut paths = BTreeSet::new();
    for file in &manifest.files {
        validate_resource_path(&file.path)?;
        if !paths.insert(file.path.as_str()) || file.sha256.len() != 64 {
            return Err(ResourceError::UnsafeExportPath(manifest_path.to_path_buf()));
        }
    }
    Ok(Some(manifest))
}

fn ensure_managed_workspace(workspace: &Path) -> Result<(), ResourceError> {
    if workspace.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(".canisend")
    }) {
        return Err(ResourceError::UnsafeExportPath(workspace.to_path_buf()));
    }
    let metadata = fs::symlink_metadata(workspace).map_err(|source| ResourceError::ExportIo {
        path: workspace.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ResourceError::UnsafeExportPath(workspace.to_path_buf()));
    }
    Ok(())
}

fn write_managed_file(root: &Path, destination: &Path, bytes: &[u8]) -> Result<(), ResourceError> {
    let parent = destination
        .parent()
        .ok_or_else(|| ResourceError::UnsafeExportPath(destination.to_path_buf()))?;
    let relative = parent
        .strip_prefix(root)
        .map_err(|_| ResourceError::UnsafeExportPath(destination.to_path_buf()))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        ensure_directory(&current)?;
    }
    if let Ok(metadata) = fs::symlink_metadata(destination)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(ResourceError::UnsafeExportPath(destination.to_path_buf()));
    }
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ResourceError::UnsafeExportPath(destination.to_path_buf()))?;
    let temporary = parent.join(format!(".{file_name}.canisend-{}-tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|source| ResourceError::ExportIo {
            path: temporary.clone(),
            source,
        })?;
    if let Err(source) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(ResourceError::ExportIo {
            path: temporary,
            source,
        });
    }
    drop(file);
    set_private_file_permissions(&temporary)?;
    if destination.exists() {
        let backup = parent.join(format!(
            ".{file_name}.canisend-{}-backup",
            std::process::id()
        ));
        if backup.exists() {
            let _ = fs::remove_file(&temporary);
            return Err(ResourceError::UnsafeExportPath(backup));
        }
        fs::rename(destination, &backup).map_err(|source| {
            let _ = fs::remove_file(&temporary);
            ResourceError::ExportIo {
                path: destination.to_path_buf(),
                source,
            }
        })?;
        if let Err(source) = fs::rename(&temporary, destination) {
            let _ = fs::rename(&backup, destination);
            let _ = fs::remove_file(&temporary);
            return Err(ResourceError::ExportIo {
                path: destination.to_path_buf(),
                source,
            });
        }
        let _ = fs::remove_file(backup);
    } else if let Err(source) = fs::rename(&temporary, destination) {
        let _ = fs::remove_file(&temporary);
        return Err(ResourceError::ExportIo {
            path: destination.to_path_buf(),
            source,
        });
    }
    Ok(())
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
    let result = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|source| ResourceError::ExportIo {
            path: destination.to_path_buf(),
            source,
        })
        .and_then(|()| set_private_file_permissions(destination));
    if result.is_err() {
        drop(file);
        let _ = fs::remove_file(destination);
    }
    result
}

fn validate_resource_path(path: &str) -> Result<(), ResourceError> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            !matches!(component, std::path::Component::Normal(_))
                || component.as_os_str().eq_ignore_ascii_case(".canisend")
        })
    {
        return Err(ResourceError::UnsafeExportPath(path.to_path_buf()));
    }
    Ok(())
}

fn rollback_new_files(root: &Path, files: &[PathBuf], remove_root: bool) {
    let mut directories = BTreeSet::new();
    for file in files.iter().rev() {
        let _ = fs::remove_file(file);
        let mut parent = file.parent();
        while let Some(directory) = parent {
            if directory == root {
                break;
            }
            directories.insert(directory.to_path_buf());
            parent = directory.parent();
        }
    }
    let mut directories = directories.into_iter().collect::<Vec<_>>();
    directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for directory in directories {
        let _ = fs::remove_dir(directory);
    }
    if remove_root {
        let _ = fs::remove_dir(root);
    }
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::rollback_new_files;

    #[test]
    fn export_rollback_removes_only_paths_owned_by_the_failed_operation() {
        let root =
            std::env::temp_dir().join(format!("canisend-resource-rollback-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let managed = root.join("schemas/v2/managed.json");
        let sentinel = root.join("concurrent-user-file.txt");
        fs::create_dir_all(managed.parent().expect("managed parent")).expect("managed directory");
        fs::write(&managed, "partial").expect("managed file");
        fs::write(&sentinel, "preserve").expect("sentinel");

        rollback_new_files(&root, std::slice::from_ref(&managed), false);

        assert!(!managed.exists());
        assert!(!root.join("schemas").exists());
        assert_eq!(
            fs::read_to_string(&sentinel).expect("preserved sentinel"),
            "preserve"
        );
        assert!(root.is_dir());
        fs::remove_dir_all(root).expect("cleanup rollback fixture");
    }
}
