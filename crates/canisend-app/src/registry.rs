use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

const REGISTRY_FORMAT: &str = "canisend.workspace-registry/v1";
const MAX_REGISTRY_BYTES: u64 = 1024 * 1024;
const MAX_REGISTRY_ENTRIES: usize = 256;
const MAX_ALIAS_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceEntry {
    pub alias: String,
    pub path: PathBuf,
    pub pinned: bool,
    pub last_opened_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRegistry {
    pub format: String,
    pub default_path: Option<PathBuf>,
    pub entries: Vec<WorkspaceEntry>,
}

impl Default for WorkspaceRegistry {
    fn default() -> Self {
        Self {
            format: REGISTRY_FORMAT.to_owned(),
            default_path: None,
            entries: Vec::new(),
        }
    }
}

impl WorkspaceRegistry {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("Cannot inspect workspace registry: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("Workspace registry must be a regular file".to_owned());
        }
        if metadata.len() > MAX_REGISTRY_BYTES {
            return Err(format!(
                "Workspace registry exceeds the {MAX_REGISTRY_BYTES}-byte limit"
            ));
        }
        let bytes =
            fs::read(path).map_err(|error| format!("Cannot read workspace registry: {error}"))?;
        let registry: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Workspace registry is invalid: {error}"))?;
        registry.validate()?;
        Ok(registry)
    }

    pub fn register(&mut self, alias: &str, path: &Path) -> Result<PathBuf, String> {
        let alias = alias.trim();
        validate_workspace_alias(alias)?;
        if !path.join("canisend.toml").is_file() {
            return Err("The selected directory is not a CanISend workspace".to_owned());
        }
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("Cannot resolve workspace path: {error}"))?;
        let opened = unix_now()?;
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|entry| entry.path == canonical)
        {
            existing.alias = alias.to_owned();
            existing.last_opened_unix = opened;
        } else {
            if self.entries.len() >= MAX_REGISTRY_ENTRIES {
                return Err(format!(
                    "Workspace registry supports at most {MAX_REGISTRY_ENTRIES} entries"
                ));
            }
            self.entries.push(WorkspaceEntry {
                alias: alias.to_owned(),
                path: canonical.clone(),
                pinned: false,
                last_opened_unix: opened,
            });
        }
        self.default_path = Some(canonical.clone());
        self.sort_entries();
        Ok(canonical)
    }

    pub fn touch(&mut self, path: &Path) -> Result<(), String> {
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("Cannot resolve workspace path: {error}"))?;
        let opened = unix_now()?;
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.path == canonical)
            .ok_or_else(|| "Workspace is not registered".to_owned())?;
        entry.last_opened_unix = opened;
        self.default_path = Some(canonical);
        self.sort_entries();
        Ok(())
    }

    pub fn remove(&mut self, path: &Path) {
        self.entries.retain(|entry| entry.path != path);
        if self.default_path.as_deref() == Some(path) {
            self.default_path = self.entries.first().map(|entry| entry.path.clone());
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let parent = path
            .parent()
            .ok_or_else(|| "Workspace registry has no parent directory".to_owned())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create application data directory: {error}"))?;
        let temporary = parent.join(format!(
            ".workspace-registry-{}-{}.tmp",
            std::process::id(),
            unix_now()?
        ));
        let mut bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| format!("Cannot encode workspace registry: {error}"))?;
        bytes.push(b'\n');
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_REGISTRY_BYTES {
            return Err(format!(
                "Workspace registry exceeds the {MAX_REGISTRY_BYTES}-byte limit"
            ));
        }
        fs::write(&temporary, bytes)
            .map_err(|error| format!("Cannot write workspace registry: {error}"))?;
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(&temporary);
            return Err(format!("Cannot commit workspace registry: {error}"));
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), String> {
        if self.format != REGISTRY_FORMAT {
            return Err(format!(
                "Unsupported workspace registry format: {}",
                self.format
            ));
        }
        if self.entries.len() > MAX_REGISTRY_ENTRIES {
            return Err(format!(
                "Workspace registry supports at most {MAX_REGISTRY_ENTRIES} entries"
            ));
        }
        let mut paths = BTreeSet::new();
        for entry in &self.entries {
            validate_workspace_alias(&entry.alias)?;
            if !entry.path.is_absolute() {
                return Err("Workspace registry paths must be absolute".to_owned());
            }
            if !paths.insert(&entry.path) {
                return Err("Workspace registry contains a duplicate path".to_owned());
            }
        }
        if let Some(default) = &self.default_path
            && !paths.contains(default)
        {
            return Err("Workspace registry default path is not registered".to_owned());
        }
        Ok(())
    }

    fn sort_entries(&mut self) {
        self.entries.sort_by(|left, right| {
            right
                .pinned
                .cmp(&left.pinned)
                .then_with(|| right.last_opened_unix.cmp(&left.last_opened_unix))
                .then_with(|| left.alias.cmp(&right.alias))
        });
    }
}

pub fn validate_workspace_alias(alias: &str) -> Result<(), String> {
    if alias.is_empty() {
        return Err("Workspace name is required".to_owned());
    }
    if alias.trim() != alias {
        return Err("Workspace name cannot start or end with whitespace".to_owned());
    }
    if alias.len() > MAX_ALIAS_BYTES {
        return Err(format!(
            "Workspace name must be at most {MAX_ALIAS_BYTES} bytes"
        ));
    }
    if alias.chars().any(char::is_control) {
        return Err("Workspace name cannot contain control characters".to_owned());
    }
    Ok(())
}

pub fn default_registry_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library/Application Support/CanISend/workspaces.json");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(app_data) = std::env::var_os("APPDATA") {
            return PathBuf::from(app_data).join("CanISend/workspaces.json");
        }
    }
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config_home).join("canisend/workspaces.json");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config/canisend/workspaces.json");
    }
    std::env::temp_dir().join("canisend/workspaces.json")
}

fn unix_now() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| "System clock is before the Unix epoch".to_owned())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{MAX_ALIAS_BYTES, MAX_REGISTRY_BYTES, WorkspaceRegistry};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn temporary_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-app-registry-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn registry_round_trip_and_removal_never_delete_workspace() {
        let root = temporary_root();
        let workspace = root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace directory");
        fs::write(workspace.join("canisend.toml"), "format = \"fixture\"\n")
            .expect("workspace marker");
        let registry_path = root.join("config/workspaces.json");

        let mut registry = WorkspaceRegistry::default();
        let canonical = registry
            .register("Applications", &workspace)
            .expect("register workspace");
        registry.save(&registry_path).expect("save registry");
        let mut loaded = WorkspaceRegistry::load(&registry_path).expect("load registry");
        assert_eq!(loaded.entries.len(), 1);
        loaded.remove(&canonical);
        loaded.save(&registry_path).expect("save removal");

        assert!(workspace.join("canisend.toml").is_file());
        assert!(
            WorkspaceRegistry::load(&registry_path)
                .expect("reload")
                .entries
                .is_empty()
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn registry_rejects_oversized_and_inconsistent_local_input() {
        let root = temporary_root();
        fs::create_dir_all(&root).expect("fixture root");
        let registry_path = root.join("workspaces.json");

        fs::write(
            &registry_path,
            vec![b' '; usize::try_from(MAX_REGISTRY_BYTES + 1).expect("bounded fixture")],
        )
        .expect("oversized registry");
        assert!(WorkspaceRegistry::load(&registry_path).is_err());

        fs::write(
            &registry_path,
            "{\"format\":\"canisend.workspace-registry/v1\",\
             \"default_path\":\"/tmp/not-registered\",\
             \"entries\":[]}\n",
        )
        .expect("inconsistent registry");
        assert!(WorkspaceRegistry::load(&registry_path).is_err());

        let workspace = root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace directory");
        fs::write(workspace.join("canisend.toml"), "format = \"fixture\"\n")
            .expect("workspace marker");
        let mut registry = WorkspaceRegistry::default();
        assert!(
            registry
                .register(&"x".repeat(MAX_ALIAS_BYTES + 1), &workspace)
                .is_err()
        );
        assert!(registry.register("bad\nname", &workspace).is_err());

        fs::remove_dir_all(root).expect("remove fixture");
    }
}
