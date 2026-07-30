use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use canisend_contracts::EntityId;
use serde::{Deserialize, Serialize};

const REGISTRY_FORMAT: &str = "canisend.agent-session-registry/v1";
const MAX_REGISTRY_BYTES: u64 = 256 * 1024;
const MAX_REGISTRY_ENTRIES: usize = 128;
const MAX_EXTERNAL_SESSION_ID_BYTES: usize = 128;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSessionEntry {
    pub workspace: PathBuf,
    pub runtime: AgentRuntimeKind,
    pub job_id: Option<String>,
    pub external_session_id: String,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSessionRegistry {
    pub format: String,
    pub entries: Vec<AgentSessionEntry>,
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
        let registry: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Agent session registry is invalid: {error}"))?;
        registry.validate()?;
        Ok(registry)
    }

    pub fn find(
        &self,
        workspace: &Path,
        runtime: AgentRuntimeKind,
        job_id: Option<&str>,
    ) -> Option<&AgentSessionEntry> {
        self.entries.iter().find(|entry| {
            entry.workspace == workspace
                && entry.runtime == runtime
                && entry.job_id.as_deref() == job_id
        })
    }

    pub fn upsert(
        &mut self,
        workspace: &Path,
        runtime: AgentRuntimeKind,
        job_id: Option<&str>,
        external_session_id: &str,
    ) -> Result<AgentSessionEntry, String> {
        validate_active_workspace(workspace)?;
        validate_job_id(job_id)?;
        validate_external_session_id(external_session_id)?;
        let now = unix_now()?;
        if let Some(entry) = self.entries.iter_mut().find(|entry| {
            entry.workspace == workspace
                && entry.runtime == runtime
                && entry.job_id.as_deref() == job_id
        }) {
            entry.external_session_id = external_session_id.to_owned();
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
            external_session_id: external_session_id.to_owned(),
            created_at_unix: now,
            updated_at_unix: now,
        };
        self.entries.push(entry.clone());
        self.sort_entries();
        Ok(entry)
    }

    pub fn remove(&mut self, workspace: &Path, runtime: AgentRuntimeKind, job_id: Option<&str>) {
        self.entries.retain(|entry| {
            entry.workspace != workspace
                || entry.runtime != runtime
                || entry.job_id.as_deref() != job_id
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
            validate_external_session_id(&entry.external_session_id)?;
            if entry.created_at_unix > entry.updated_at_unix {
                return Err(
                    "Agent session registry contains an invalid update timestamp".to_owned(),
                );
            }
            if !scopes.insert((&entry.workspace, entry.runtime, entry.job_id.as_deref())) {
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
        });
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

    use super::{AgentRuntimeKind, AgentSessionRegistry};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-agent-session-registry-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
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
            .upsert(&workspace, AgentRuntimeKind::Codex, None, "thread-1")
            .expect("workspace session");
        registry
            .upsert(&workspace, AgentRuntimeKind::Codex, None, "thread-2")
            .expect("replace workspace session");
        registry
            .upsert(
                &workspace,
                AgentRuntimeKind::Claude,
                Some("019f4876-016d-7b41-b959-f4f2543ffd9f"),
                "session-3",
            )
            .expect("job session");
        registry.save(&path).expect("save");

        let loaded = AgentSessionRegistry::load(&path).expect("load");
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(
            loaded
                .find(&workspace, AgentRuntimeKind::Codex, None)
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
                "created_at_unix",
                "external_session_id",
                "job_id",
                "runtime",
                "updated_at_unix",
                "workspace",
            ])
        );

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
                .upsert(&workspace, AgentRuntimeKind::Claude, None, "--resume=other")
                .is_err()
        );
        assert!(
            registry
                .upsert(&workspace, AgentRuntimeKind::Claude, None, "safe-session")
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
                "active-session",
            )
            .expect("active binding");
        loaded.save(&path).expect("save mixed registry");

        let reloaded = AgentSessionRegistry::load(&path).expect("reload mixed registry");
        assert_eq!(reloaded.entries.len(), 2);
        assert_eq!(
            reloaded
                .find(&active_workspace, AgentRuntimeKind::Claude, None)
                .expect("active session")
                .external_session_id,
            "active-session"
        );

        fs::remove_dir_all(root).expect("remove fixture");
    }
}
