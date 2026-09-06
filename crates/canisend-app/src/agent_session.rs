use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use canisend_contracts::EntityId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const REGISTRY_FORMAT: &str = "canisend.agent-session-registry/v3";
const PREVIOUS_REGISTRY_FORMAT: &str = "canisend.agent-session-registry/v2";
const LEGACY_REGISTRY_FORMAT: &str = "canisend.agent-session-registry/v1";
const MAX_REGISTRY_BYTES: u64 = 256 * 1024;
const MAX_REGISTRY_ENTRIES: usize = 128;
const MAX_EXTERNAL_SESSION_ID_BYTES: usize = 128;
const MAX_PROVIDER_VERSION_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentRuntimeKind {
    Codex,
    Claude,
}

impl AgentRuntimeKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentSessionStatus {
    #[default]
    Disconnected,
    Connecting,
    AuthenticationRequired,
    Ready,
    Running,
    Cancelling,
    Interrupted,
    RecoverableDisconnect,
    Incompatible,
    Failed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentSessionMetadata {
    pub desktop_session_id: Option<String>,
    pub desktop_turn_id: Option<String>,
    pub external_turn_id: Option<String>,
    pub provider_version: Option<String>,
    pub last_status: AgentSessionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSessionEntry {
    pub workspace: PathBuf,
    pub runtime: AgentRuntimeKind,
    pub job_id: Option<String>,
    pub application_id: Option<String>,
    pub external_session_id: String,
    pub desktop_session_id: Option<String>,
    pub desktop_turn_id: Option<String>,
    pub external_turn_id: Option<String>,
    pub provider_version: Option<String>,
    pub last_status: AgentSessionStatus,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSessionRegistry {
    pub format: String,
    pub entries: Vec<AgentSessionEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentSessionEntryV1 {
    workspace: PathBuf,
    runtime: AgentRuntimeKind,
    job_id: Option<String>,
    external_session_id: String,
    created_at_unix: u64,
    updated_at_unix: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentSessionRegistryV1 {
    format: String,
    entries: Vec<AgentSessionEntryV1>,
}

impl Default for AgentSessionRegistry {
    fn default() -> Self {
        Self {
            format: REGISTRY_FORMAT.to_owned(),
            entries: Vec::new(),
        }
    }
}

impl AgentSessionRegistry {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("Cannot inspect agent session registry: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("Agent session registry must be a regular file".to_owned());
        }
        if metadata.len() > MAX_REGISTRY_BYTES {
            return Err(format!(
                "Agent session registry exceeds the {MAX_REGISTRY_BYTES}-byte limit"
            ));
        }
        let bytes = fs::read(path)
            .map_err(|error| format!("Cannot read agent session registry: {error}"))?;
        let mut value: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Agent session registry is invalid: {error}"))?;
        let format = value
            .get("format")
            .and_then(Value::as_str)
            .ok_or_else(|| "Agent session registry format is missing".to_owned())?
            .to_owned();
        let registry = match format.as_str() {
            REGISTRY_FORMAT => serde_json::from_value(value)
                .map_err(|error| format!("Agent session registry is invalid: {error}"))?,
            PREVIOUS_REGISTRY_FORMAT => {
                // Older rows have no Application identity. Never reinterpret a Job UUID.
                let entries = value
                    .get_mut("entries")
                    .and_then(Value::as_array_mut)
                    .ok_or("Agent session registry entries are invalid")?;
                for entry in entries {
                    let entry = entry
                        .as_object_mut()
                        .ok_or("Agent session registry entry is invalid")?;
                    if entry
                        .insert("application_id".to_owned(), Value::Null)
                        .is_some()
                    {
                        return Err(
                            "Version two registry cannot contain an Application binding".to_owned()
                        );
                    }
                }
                value["format"] = Value::String(REGISTRY_FORMAT.to_owned());
                serde_json::from_value(value)
                    .map_err(|error| format!("Agent session registry is invalid: {error}"))?
            }
            LEGACY_REGISTRY_FORMAT => {
                let legacy: AgentSessionRegistryV1 = serde_json::from_value(value)
                    .map_err(|error| format!("Agent session registry is invalid: {error}"))?;
                Self::migrate_v1(legacy)?
            }
            other => {
                return Err(format!(
                    "Unsupported agent session registry format: {other}"
                ));
            }
        };
        registry.validate()?;
        Ok(registry)
    }

    pub fn find(
        &self,
        workspace: &Path,
        runtime: AgentRuntimeKind,
        job_id: Option<&str>,
        application_id: Option<&str>,
    ) -> Option<&AgentSessionEntry> {
        self.entries.iter().find(|entry| {
            entry.workspace == workspace
                && entry.runtime == runtime
                && entry.job_id.as_deref() == job_id
                && entry.application_id.as_deref() == application_id
        })
    }

    pub fn upsert(
        &mut self,
        workspace: &Path,
        runtime: AgentRuntimeKind,
        job_id: Option<&str>,
        application_id: Option<&str>,
        external_session_id: &str,
    ) -> Result<AgentSessionEntry, String> {
        self.upsert_with_metadata(
            workspace,
            runtime,
            job_id,
            application_id,
            external_session_id,
            AgentSessionMetadata::default(),
        )
    }

    pub fn upsert_with_metadata(
        &mut self,
        workspace: &Path,
        runtime: AgentRuntimeKind,
        job_id: Option<&str>,
        application_id: Option<&str>,
        external_session_id: &str,
        metadata: AgentSessionMetadata,
    ) -> Result<AgentSessionEntry, String> {
        validate_active_workspace(workspace)?;
        validate_job_id(job_id)?;
        validate_application_scope(job_id, application_id)?;
        validate_external_session_id(external_session_id)?;
        validate_metadata(&metadata)?;
        let now = unix_now()?;
        if let Some(entry) = self.entries.iter_mut().find(|entry| {
            entry.workspace == workspace
                && entry.runtime == runtime
                && entry.job_id.as_deref() == job_id
                && entry.application_id.as_deref() == application_id
        }) {
            entry.external_session_id = external_session_id.to_owned();
            entry.desktop_session_id = metadata.desktop_session_id;
            entry.desktop_turn_id = metadata.desktop_turn_id;
            entry.external_turn_id = metadata.external_turn_id;
            entry.provider_version = metadata.provider_version;
            entry.last_status = metadata.last_status;
            entry.updated_at_unix = now;
            return Ok(entry.clone());
        }
        if self.entries.len() >= MAX_REGISTRY_ENTRIES {
            return Err(format!(
                "Agent session registry supports at most {MAX_REGISTRY_ENTRIES} entries"
            ));
        }
        let entry = AgentSessionEntry {
            workspace: workspace.to_path_buf(),
            runtime,
            job_id: job_id.map(ToOwned::to_owned),
            application_id: application_id.map(ToOwned::to_owned),
            external_session_id: external_session_id.to_owned(),
            desktop_session_id: metadata.desktop_session_id,
            desktop_turn_id: metadata.desktop_turn_id,
            external_turn_id: metadata.external_turn_id,
            provider_version: metadata.provider_version,
            last_status: metadata.last_status,
            created_at_unix: now,
            updated_at_unix: now,
        };
        self.entries.push(entry.clone());
        self.sort_entries();
        Ok(entry)
    }

    pub fn remove(
        &mut self,
        workspace: &Path,
        runtime: AgentRuntimeKind,
        job_id: Option<&str>,
        application_id: Option<&str>,
    ) {
        self.entries.retain(|entry| {
            entry.workspace != workspace
                || entry.runtime != runtime
                || entry.job_id.as_deref() != job_id
                || entry.application_id.as_deref() != application_id
        });
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let parent = path
            .parent()
            .ok_or_else(|| "Agent session registry has no parent directory".to_owned())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create application data directory: {error}"))?;
        let temporary = parent.join(format!(
            ".agent-sessions-{}-{}.tmp",
            std::process::id(),
            unix_now()?
        ));
        let mut bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| format!("Cannot encode agent session registry: {error}"))?;
        bytes.push(b'\n');
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_REGISTRY_BYTES {
            return Err(format!(
                "Agent session registry exceeds the {MAX_REGISTRY_BYTES}-byte limit"
            ));
        }
        fs::write(&temporary, bytes)
            .map_err(|error| format!("Cannot write agent session registry: {error}"))?;
        set_private_permissions(&temporary)?;
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(&temporary);
            return Err(format!("Cannot commit agent session registry: {error}"));
        }
        set_private_permissions(path)
    }

    fn validate(&self) -> Result<(), String> {
        if self.format != REGISTRY_FORMAT {
            return Err(format!(
                "Unsupported agent session registry format: {}",
                self.format
            ));
        }
        if self.entries.len() > MAX_REGISTRY_ENTRIES {
            return Err(format!(
                "Agent session registry supports at most {MAX_REGISTRY_ENTRIES} entries"
            ));
        }
        let mut scopes = BTreeSet::new();
        for entry in &self.entries {
            validate_workspace_path(&entry.workspace)?;
            validate_job_id(entry.job_id.as_deref())?;
            validate_application_scope(entry.job_id.as_deref(), entry.application_id.as_deref())?;
            validate_external_session_id(&entry.external_session_id)?;
            validate_metadata(&AgentSessionMetadata {
                desktop_session_id: entry.desktop_session_id.clone(),
                desktop_turn_id: entry.desktop_turn_id.clone(),
                external_turn_id: entry.external_turn_id.clone(),
                provider_version: entry.provider_version.clone(),
                last_status: entry.last_status,
            })?;
            if entry.created_at_unix > entry.updated_at_unix {
                return Err(
                    "Agent session registry contains an invalid update timestamp".to_owned(),
                );
            }
            if !scopes.insert((
                &entry.workspace,
                entry.runtime,
                entry.job_id.as_deref(),
                entry.application_id.as_deref(),
            )) {
                return Err("Agent session registry contains a duplicate scope".to_owned());
            }
        }
        Ok(())
    }

    fn sort_entries(&mut self) {
        self.entries.sort_by(|left, right| {
            right
                .updated_at_unix
                .cmp(&left.updated_at_unix)
                .then_with(|| left.workspace.cmp(&right.workspace))
                .then_with(|| left.runtime.cmp(&right.runtime))
                .then_with(|| left.job_id.cmp(&right.job_id))
                .then_with(|| left.application_id.cmp(&right.application_id))
        });
    }

    fn migrate_v1(legacy: AgentSessionRegistryV1) -> Result<Self, String> {
        if legacy.format != LEGACY_REGISTRY_FORMAT {
            return Err(format!(
                "Unsupported agent session registry format: {}",
                legacy.format
            ));
        }
        let registry = Self {
            format: REGISTRY_FORMAT.to_owned(),
            entries: legacy
                .entries
                .into_iter()
                .map(|entry| AgentSessionEntry {
                    workspace: entry.workspace,
                    runtime: entry.runtime,
                    job_id: entry.job_id,
                    application_id: None,
                    external_session_id: entry.external_session_id,
                    desktop_session_id: None,
                    desktop_turn_id: None,
                    external_turn_id: None,
                    provider_version: None,
                    last_status: AgentSessionStatus::Disconnected,
                    created_at_unix: entry.created_at_unix,
                    updated_at_unix: entry.updated_at_unix,
                })
                .collect(),
        };
        registry.validate()?;
        Ok(registry)
    }
}

#[must_use]
pub fn default_agent_session_registry_path() -> PathBuf {
    super::default_registry_path()
        .parent()
        .map(|parent| parent.join("agent-sessions.json"))
        .unwrap_or_else(|| std::env::temp_dir().join("canisend/agent-sessions.json"))
}

fn validate_workspace_path(workspace: &Path) -> Result<(), String> {
    if !workspace.is_absolute() {
        return Err("Agent session workspace paths must be absolute".to_owned());
    }
    Ok(())
}

fn validate_active_workspace(workspace: &Path) -> Result<(), String> {
    validate_workspace_path(workspace)?;
    if !workspace.join("canisend.toml").is_file() {
        return Err("Agent session workspace is not a CanISend workspace".to_owned());
    }
    Ok(())
}

fn validate_application_scope(
    job_id: Option<&str>,
    application_id: Option<&str>,
) -> Result<(), String> {
    if job_id.is_some() && application_id.is_some() {
        return Err("Agent session cannot bind both a Job and an Application".to_owned());
    }
    application_id
        .map(|value| EntityId::try_new(value.to_owned()).map(|_| ()))
        .transpose()
        .map(|_| ())
        .map_err(|error| format!("Agent session Application ID is invalid: {error}"))
}

fn validate_job_id(job_id: Option<&str>) -> Result<(), String> {
    job_id
        .map(|value| EntityId::try_new(value.to_owned()).map(|_| ()))
        .transpose()
        .map(|_| ())
        .map_err(|error| format!("Agent session job ID is invalid: {error}"))
}

fn validate_external_session_id(value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_EXTERNAL_SESSION_ID_BYTES {
        return Err(format!(
            "External agent session ID must contain 1 to {MAX_EXTERNAL_SESSION_ID_BYTES} bytes"
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err("External agent session ID contains unsupported characters".to_owned());
    }
    Ok(())
}

fn validate_metadata(metadata: &AgentSessionMetadata) -> Result<(), String> {
    for (label, value) in [
        ("Desktop session ID", metadata.desktop_session_id.as_deref()),
        ("Desktop turn ID", metadata.desktop_turn_id.as_deref()),
        ("External turn ID", metadata.external_turn_id.as_deref()),
    ] {
        if let Some(value) = value {
            validate_external_session_id(value)
                .map_err(|error| error.replacen("External agent session ID", label, 1))?;
        }
    }
    if let Some(version) = metadata.provider_version.as_deref() {
        if version.is_empty() || version.len() > MAX_PROVIDER_VERSION_BYTES {
            return Err(format!(
                "Agent provider version must contain 1 to {MAX_PROVIDER_VERSION_BYTES} bytes"
            ));
        }
        if version.chars().any(char::is_control) {
            return Err("Agent provider version contains control characters".to_owned());
        }
    }
    Ok(())
}

fn unix_now() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| "System clock is before the Unix epoch".to_owned())
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| format!("Cannot inspect agent session registry permissions: {error}"))?
        .permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("Cannot protect agent session registry: {error}"))
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{AgentRuntimeKind, AgentSessionMetadata, AgentSessionRegistry, AgentSessionStatus};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-agent-session-registry-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn v2_migration_keeps_job_and_application_sessions_separate() {
        let root = root();
        fs::create_dir_all(&root).expect("fixture");
        fs::write(root.join("canisend.toml"), "fixture").expect("marker");
        let path = root.join("sessions.json");
        let id = "019f4876-016d-7b41-b959-f4f2543ffd9f";
        let mut registry = AgentSessionRegistry::default();
        registry
            .upsert(
                &root,
                AgentRuntimeKind::Codex,
                Some(id),
                None,
                "legacy-thread",
            )
            .expect("job");
        let mut old = serde_json::to_value(&registry).expect("v2 fixture");
        old["format"] = serde_json::json!("canisend.agent-session-registry/v2");
        old["entries"][0]
            .as_object_mut()
            .expect("row")
            .remove("application_id");
        let bytes = serde_json::to_vec(&old).expect("encode");
        fs::write(&path, &bytes).expect("v2 registry");
        let mut migrated = AgentSessionRegistry::load(&path).expect("migrate v2");
        assert_eq!(fs::read(&path).expect("unchanged source"), bytes);
        assert!(
            migrated
                .find(&root, AgentRuntimeKind::Codex, None, Some(id))
                .is_none()
        );
        migrated
            .upsert(
                &root,
                AgentRuntimeKind::Codex,
                None,
                Some(id),
                "application-thread",
            )
            .expect("Application");
        migrated
            .upsert(
                &root,
                AgentRuntimeKind::Codex,
                None,
                None,
                "workspace-thread",
            )
            .expect("Workspace");
        assert!(
            migrated
                .upsert(
                    &root,
                    AgentRuntimeKind::Codex,
                    Some(id),
                    Some(id),
                    "ambiguous"
                )
                .is_err()
        );
        assert!(
            migrated
                .upsert(
                    &root,
                    AgentRuntimeKind::Codex,
                    None,
                    Some("invalid"),
                    "invalid"
                )
                .is_err()
        );
        migrated.save(&path).expect("v3 save");
        let mut loaded = AgentSessionRegistry::load(&path).expect("v3 reload");
        assert_eq!(loaded.entries.len(), 3);
        assert_eq!(
            loaded
                .find(&root, AgentRuntimeKind::Codex, None, Some(id))
                .expect("Application")
                .external_session_id,
            "application-thread"
        );
        assert_eq!(
            loaded
                .find(&root, AgentRuntimeKind::Codex, Some(id), None)
                .expect("Job")
                .external_session_id,
            "legacy-thread"
        );
        assert!(
            loaded
                .find(&root, AgentRuntimeKind::Claude, None, Some(id))
                .is_none()
        );
        loaded.remove(&root, AgentRuntimeKind::Codex, None, Some(id));
        assert_eq!(loaded.entries.len(), 2);
        assert!(
            loaded
                .find(&root, AgentRuntimeKind::Codex, Some(id), None)
                .is_some()
        );
        old["entries"][0]["application_id"] = serde_json::json!(id);
        fs::write(&path, serde_json::to_vec(&old).expect("invalid v2")).expect("write invalid v2");
        assert!(AgentSessionRegistry::load(&path).is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn session_bindings_are_body_free_scoped_and_replaceable() {
        let root = root();
        let workspace = root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::write(workspace.join("canisend.toml"), "format = \"fixture\"\n")
            .expect("workspace marker");
        let path = root.join("config/agent-sessions.json");

        let mut registry = AgentSessionRegistry::default();
        registry
            .upsert_with_metadata(
                &workspace,
                AgentRuntimeKind::Codex,
                None,
                None,
                "thread-1",
                AgentSessionMetadata {
                    desktop_session_id: Some("desktop-session-1".to_owned()),
                    desktop_turn_id: Some("desktop-turn-1".to_owned()),
                    external_turn_id: Some("provider-turn-1".to_owned()),
                    provider_version: Some("codex-cli 0.152.0".to_owned()),
                    last_status: AgentSessionStatus::Ready,
                },
            )
            .expect("workspace session");
        registry
            .upsert(&workspace, AgentRuntimeKind::Codex, None, None, "thread-2")
            .expect("replace workspace session");
        registry
            .upsert(
                &workspace,
                AgentRuntimeKind::Claude,
                Some("019f4876-016d-7b41-b959-f4f2543ffd9f"),
                None,
                "session-3",
            )
            .expect("job session");
        registry.save(&path).expect("save");

        let loaded = AgentSessionRegistry::load(&path).expect("load");
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(
            loaded
                .find(&workspace, AgentRuntimeKind::Codex, None, None)
                .expect("codex binding")
                .external_session_id,
            "thread-2"
        );
        let encoded = serde_json::to_string(&loaded).expect("JSON");
        assert!(!encoded.contains("prompt"));
        assert!(!encoded.contains("response"));
        assert!(!encoded.contains("transcript"));
        let value = serde_json::to_value(&loaded).expect("registry value");
        let entry = value["entries"][0].as_object().expect("entry object");
        assert_eq!(
            entry.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "application_id",
                "created_at_unix",
                "desktop_session_id",
                "desktop_turn_id",
                "external_session_id",
                "external_turn_id",
                "job_id",
                "last_status",
                "provider_version",
                "runtime",
                "updated_at_unix",
                "workspace",
            ])
        );

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn version_one_registry_migrates_without_content_or_identity_loss() {
        let root = root();
        let workspace = root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::write(workspace.join("canisend.toml"), "format = \"fixture\"\n")
            .expect("workspace marker");
        let path = root.join("agent-sessions.json");
        let legacy = serde_json::json!({
            "format": "canisend.agent-session-registry/v1",
            "entries": [{
                "workspace": workspace,
                "runtime": "codex",
                "job_id": null,
                "external_session_id": "thread-legacy",
                "created_at_unix": 10,
                "updated_at_unix": 20
            }]
        });
        fs::write(
            &path,
            serde_json::to_vec_pretty(&legacy).expect("legacy registry"),
        )
        .expect("write legacy registry");

        let migrated = AgentSessionRegistry::load(&path).expect("migrate registry");
        assert_eq!(migrated.format, "canisend.agent-session-registry/v3");
        assert_eq!(migrated.entries[0].external_session_id, "thread-legacy");
        assert_eq!(
            migrated.entries[0].last_status,
            AgentSessionStatus::Disconnected
        );
        migrated.save(&path).expect("write canonical v3 registry");
        let encoded = fs::read_to_string(&path).expect("read canonical registry");
        assert!(encoded.contains("canisend.agent-session-registry/v3"));
        for private_key in ["prompt", "response", "transcript", "approval_token"] {
            assert!(!encoded.contains(private_key));
        }

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn session_registry_rejects_unsafe_host_ids_and_duplicate_scopes() {
        let root = root();
        let workspace = root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::write(workspace.join("canisend.toml"), "format = \"fixture\"\n")
            .expect("workspace marker");
        let mut registry = AgentSessionRegistry::default();
        assert!(
            registry
                .upsert(
                    &workspace,
                    AgentRuntimeKind::Claude,
                    None,
                    None,
                    "--resume=other"
                )
                .is_err()
        );
        assert!(
            registry
                .upsert(
                    &workspace,
                    AgentRuntimeKind::Claude,
                    None,
                    None,
                    "safe-session"
                )
                .is_ok()
        );

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn stale_workspace_binding_does_not_block_an_active_workspace() {
        let root = root();
        let stale_workspace = root.join("stale-workspace");
        let active_workspace = root.join("active-workspace");
        fs::create_dir_all(&stale_workspace).expect("stale workspace");
        fs::write(
            stale_workspace.join("canisend.toml"),
            "format = \"fixture\"\n",
        )
        .expect("stale workspace marker");
        let path = root.join("config/agent-sessions.json");

        let mut registry = AgentSessionRegistry::default();
        registry
            .upsert(
                &stale_workspace,
                AgentRuntimeKind::Codex,
                None,
                None,
                "stale-session",
            )
            .expect("stale binding");
        registry.save(&path).expect("save stale binding");
        fs::remove_dir_all(&stale_workspace).expect("remove stale workspace");

        let mut loaded = AgentSessionRegistry::load(&path).expect("load body-free stale binding");
        fs::create_dir_all(&active_workspace).expect("active workspace");
        fs::write(
            active_workspace.join("canisend.toml"),
            "format = \"fixture\"\n",
        )
        .expect("active workspace marker");
        loaded
            .upsert(
                &active_workspace,
                AgentRuntimeKind::Claude,
                None,
                None,
                "active-session",
            )
            .expect("active binding");
        loaded.save(&path).expect("save mixed registry");

        let reloaded = AgentSessionRegistry::load(&path).expect("reload mixed registry");
        assert_eq!(reloaded.entries.len(), 2);
        assert_eq!(
            reloaded
                .find(&active_workspace, AgentRuntimeKind::Claude, None, None)
                .expect("active session")
                .external_session_id,
            "active-session"
        );

        fs::remove_dir_all(root).expect("remove fixture");
    }
}
