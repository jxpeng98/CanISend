use std::{
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ActionReceipt, ApplicationError};

const INSTALL_FORMAT: &str = "canisend.cli-install/v1";
const MANIFEST_NAME: &str = ".canisend-install-v1.json";
const MAX_CLI_BYTES: u64 = 256 * 1024 * 1024;
const MAX_VERSION_OUTPUT_BYTES: u64 = 64 * 1024;
const VERSION_PROBE_TIMEOUT: Duration = Duration::from_millis(750);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalInstallConsent(());

impl TerminalInstallConsent {
    #[must_use]
    pub const fn granted_by_user() -> Self {
        Self(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CliInstallState {
    NotInstalled,
    Current,
    UpdateAvailable,
    MigrationAvailable,
    NewerInstalled,
    Modified,
    SourceUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CliVersionRelation {
    Unknown,
    Older,
    Same,
    Newer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliInstallStatus {
    pub state: CliInstallState,
    pub bundled_version: String,
    pub installed_version: Option<String>,
    pub version_relation: CliVersionRelation,
    pub source_path: Option<PathBuf>,
    pub destination: PathBuf,
    pub manifest_path: PathBuf,
    pub installed: bool,
    pub managed: bool,
    pub path_configured: bool,
    pub active_command: Option<PathBuf>,
    pub active_is_managed: bool,
    pub previous_installation_preserved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstallManifest {
    format: String,
    version: String,
    destination: PathBuf,
    digest: String,
    installed_at_unix: u64,
    backup_path: Option<PathBuf>,
}

pub(crate) fn inspect(
    source: Option<&Path>,
    destination: &Path,
) -> Result<ActionReceipt<CliInstallStatus>, ApplicationError> {
    validate_destination(destination)?;
    let manifest_path = manifest_path(destination)?;
    let source = source.and_then(usable_source);
    let source_digest = source
        .as_deref()
        .map(file_digest)
        .transpose()
        .map_err(cli_io)?;
    let manifest = read_manifest(&manifest_path)?;
    let target_metadata = fs::symlink_metadata(destination).ok();
    let installed = target_metadata.is_some();
    let bundled_version = env!("CARGO_PKG_VERSION").to_owned();
    let installed_version = if installed {
        manifest
            .as_ref()
            .map(|manifest| manifest.version.clone())
            .or_else(|| probe_cli_version(destination))
    } else {
        None
    };
    let version_relation = compare_versions(installed_version.as_deref(), &bundled_version);

    let (state, managed, previous_installation_preserved) =
        match (target_metadata, manifest.as_ref()) {
            (None, _) if source.is_none() => (CliInstallState::SourceUnavailable, false, false),
            (None, _) => (CliInstallState::NotInstalled, false, false),
            (Some(metadata), None) if !metadata.is_file() && !metadata.file_type().is_symlink() => {
                (CliInstallState::Modified, false, false)
            }
            (Some(_), None) if version_relation == CliVersionRelation::Newer => {
                (CliInstallState::NewerInstalled, false, false)
            }
            (Some(_), None) => (CliInstallState::MigrationAvailable, false, false),
            (Some(metadata), Some(manifest))
                if manifest.destination == destination
                    && metadata.is_file()
                    && !metadata.file_type().is_symlink() =>
            {
                let destination_digest = file_digest(destination).map_err(cli_io)?;
                if destination_digest != manifest.digest {
                    (
                        CliInstallState::Modified,
                        true,
                        manifest.backup_path.is_some(),
                    )
                } else if source_digest.as_deref() == Some(destination_digest.as_str()) {
                    (
                        CliInstallState::Current,
                        true,
                        manifest.backup_path.is_some(),
                    )
                } else if source.is_some() {
                    (
                        CliInstallState::UpdateAvailable,
                        true,
                        manifest.backup_path.is_some(),
                    )
                } else {
                    (
                        CliInstallState::SourceUnavailable,
                        true,
                        manifest.backup_path.is_some(),
                    )
                }
            }
            (Some(_), Some(manifest)) => (
                CliInstallState::Modified,
                true,
                manifest.backup_path.is_some(),
            ),
        };

    let active_command = find_on_path(destination);
    let active_is_managed = managed
        && active_command
            .as_deref()
            .is_some_and(|active| paths_refer_to_same_file(active, destination));
    let path_configured = destination.parent().is_some_and(directory_is_on_path);
    let status = CliInstallStatus {
        state,
        bundled_version,
        installed_version,
        version_relation,
        source_path: source,
        destination: destination.to_path_buf(),
        manifest_path,
        installed,
        managed,
        path_configured,
        active_command,
        active_is_managed,
        previous_installation_preserved,
    };
    Ok(ActionReceipt::new(
        "cli.install.status",
        "available",
        status_summary(&status),
        status,
    ))
}

pub(crate) fn install(
    source: &Path,
    destination: &Path,
    replace_existing: bool,
    _consent: TerminalInstallConsent,
) -> Result<ActionReceipt<CliInstallStatus>, ApplicationError> {
    validate_destination(destination)?;
    let source = usable_source(source).ok_or_else(|| {
        ApplicationError::CliInstall(format!(
            "bundled CLI is not a regular non-symlink file: {}",
            source.display()
        ))
    })?;
    if paths_refer_to_same_file(&source, destination) {
        return Err(ApplicationError::CliInstall(
            "bundled CLI and terminal destination must be different files".to_owned(),
        ));
    }

    let parent = destination
        .parent()
        .ok_or_else(|| ApplicationError::CliInstall("CLI destination has no parent".to_owned()))?;
    fs::create_dir_all(parent).map_err(cli_io)?;
    let manifest_path = manifest_path(destination)?;
    let existing_manifest = read_manifest(&manifest_path)?;
    let existing_metadata = fs::symlink_metadata(destination).ok();
    if existing_metadata
        .as_ref()
        .is_some_and(|metadata| !metadata.is_file() && !metadata.file_type().is_symlink())
    {
        return Err(ApplicationError::CliInstall(
            "CLI destination exists but is not a regular file or symlink".to_owned(),
        ));
    }

    if existing_manifest.is_some() {
        let status = inspect(Some(&source), destination)?.data;
        if status.state == CliInstallState::Modified {
            return Err(ApplicationError::CliInstall(
                "managed CLI was modified outside CanISend; move it aside before reinstalling"
                    .to_owned(),
            ));
        }
        if status.state == CliInstallState::NewerInstalled {
            return Err(ApplicationError::CliInstall(
                "a newer CanISend CLI is already installed; this GUI will not downgrade it"
                    .to_owned(),
            ));
        }
    } else if existing_metadata.is_some() && !replace_existing {
        return Err(ApplicationError::CliInstall(
            "another CanISend installation already exists; preserve it before upgrading".to_owned(),
        ));
    } else if existing_metadata.is_some()
        && inspect(Some(&source), destination)?.data.state == CliInstallState::NewerInstalled
    {
        return Err(ApplicationError::CliInstall(
            "a newer CanISend CLI is already installed; this GUI will not downgrade it".to_owned(),
        ));
    }

    let nonce = nonce()?;
    let temporary_binary = parent.join(format!(".canisend-install-{nonce}.tmp"));
    let temporary_manifest = parent.join(format!(".canisend-manifest-{nonce}.tmp"));
    copy_cli(&source, &temporary_binary)?;
    let digest = file_digest(&temporary_binary).map_err(cli_io)?;

    let mut backup_path = existing_manifest
        .as_ref()
        .and_then(|manifest| manifest.backup_path.clone());
    let rollback_path = if existing_metadata.is_some() {
        if existing_manifest.is_some() {
            Some(parent.join(format!(".canisend-rollback-{nonce}.tmp")))
        } else {
            let backup = parent.join(format!(".canisend.previous-{nonce}"));
            backup_path = Some(backup.clone());
            Some(backup)
        }
    } else {
        None
    };

    let manifest = InstallManifest {
        format: INSTALL_FORMAT.to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        destination: destination.to_path_buf(),
        digest,
        installed_at_unix: unix_now()?,
        backup_path,
    };
    write_manifest(&temporary_manifest, &manifest)?;

    if let Some(rollback) = &rollback_path {
        fs::rename(destination, rollback).map_err(cli_io)?;
    }
    if let Err(error) = fs::rename(&temporary_binary, destination) {
        restore_rollback(destination, rollback_path.as_deref());
        let _ = fs::remove_file(&temporary_manifest);
        return Err(cli_io(error));
    }
    if let Err(error) = fs::rename(&temporary_manifest, &manifest_path) {
        let _ = fs::remove_file(destination);
        restore_rollback(destination, rollback_path.as_deref());
        return Err(cli_io(error));
    }
    if existing_manifest.is_some()
        && let Some(rollback) = rollback_path
    {
        let _ = fs::remove_file(rollback);
    }

    let status = inspect(Some(&source), destination)?.data;
    Ok(ActionReceipt::new(
        "cli.install",
        "installed",
        format!(
            "Installed CanISend CLI {} at {}",
            status.bundled_version,
            status.destination.display()
        ),
        status,
    ))
}

pub(crate) fn uninstall(
    source: Option<&Path>,
    destination: &Path,
    _consent: TerminalInstallConsent,
) -> Result<ActionReceipt<CliInstallStatus>, ApplicationError> {
    validate_destination(destination)?;
    let manifest_path = manifest_path(destination)?;
    let manifest = read_manifest(&manifest_path)?.ok_or_else(|| {
        ApplicationError::CliInstall(
            "this CLI installation is not managed by the CanISend GUI".to_owned(),
        )
    })?;
    if manifest.destination != destination {
        return Err(ApplicationError::CliInstall(
            "CLI install manifest points to a different destination".to_owned(),
        ));
    }
    let metadata = fs::symlink_metadata(destination).map_err(cli_io)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(ApplicationError::CliInstall(
            "managed CLI is no longer a regular file; it was not removed".to_owned(),
        ));
    }
    if file_digest(destination).map_err(cli_io)? != manifest.digest {
        return Err(ApplicationError::CliInstall(
            "managed CLI contents changed after installation; it was not removed".to_owned(),
        ));
    }

    let parent = destination
        .parent()
        .ok_or_else(|| ApplicationError::CliInstall("CLI destination has no parent".to_owned()))?;
    let staged = parent.join(format!(".canisend-removing-{}.tmp", nonce()?));
    fs::rename(destination, &staged).map_err(cli_io)?;
    if let Some(backup) = manifest.backup_path.as_deref() {
        let backup_metadata = fs::symlink_metadata(backup).ok();
        if !valid_backup_path(parent, backup)
            || !backup_metadata
                .is_some_and(|metadata| metadata.is_file() || metadata.file_type().is_symlink())
        {
            let _ = fs::rename(&staged, destination);
            return Err(ApplicationError::CliInstall(
                "the preserved previous installation is missing or invalid; nothing was removed"
                    .to_owned(),
            ));
        }
        if let Err(error) = fs::rename(backup, destination) {
            let _ = fs::rename(&staged, destination);
            return Err(cli_io(error));
        }
    }
    fs::remove_file(&staged).map_err(cli_io)?;
    fs::remove_file(&manifest_path).map_err(cli_io)?;

    let status = inspect(source, destination)?.data;
    let summary = if manifest.backup_path.is_some() {
        "Removed the GUI-managed Rust CLI and restored the previous CanISend installation"
            .to_owned()
    } else {
        "Removed the GUI-managed CanISend CLI; workspace data was not changed".to_owned()
    };
    Ok(ActionReceipt::new(
        "cli.uninstall",
        "uninstalled",
        summary,
        status,
    ))
}

fn status_summary(status: &CliInstallStatus) -> String {
    match status.state {
        CliInstallState::NotInstalled => "CanISend CLI is ready to install".to_owned(),
        CliInstallState::Current if status.active_is_managed => {
            "The bundled CanISend CLI is active in the terminal".to_owned()
        }
        CliInstallState::Current => {
            "The bundled CanISend CLI is installed, but another command may take precedence"
                .to_owned()
        }
        CliInstallState::UpdateAvailable => {
            "A different bundled CanISend CLI build is ready to install".to_owned()
        }
        CliInstallState::MigrationAvailable => {
            "An existing CanISend installation is ready to migrate or upgrade".to_owned()
        }
        CliInstallState::NewerInstalled => {
            "A newer CanISend CLI is already installed; no downgrade is offered".to_owned()
        }
        CliInstallState::Modified => {
            "The GUI-managed CLI or its install record was changed externally".to_owned()
        }
        CliInstallState::SourceUnavailable => {
            "The GUI package does not contain an installable CanISend CLI".to_owned()
        }
    }
}

fn compare_versions(installed: Option<&str>, bundled: &str) -> CliVersionRelation {
    let (Some(installed), Ok(bundled)) = (
        installed.and_then(parse_comparable_version),
        Version::parse(bundled),
    ) else {
        return CliVersionRelation::Unknown;
    };
    match installed.cmp(&bundled) {
        std::cmp::Ordering::Less => CliVersionRelation::Older,
        std::cmp::Ordering::Equal => CliVersionRelation::Same,
        std::cmp::Ordering::Greater => CliVersionRelation::Newer,
    }
}

fn parse_comparable_version(value: &str) -> Option<Version> {
    let value = value.trim().strip_prefix('v').unwrap_or(value.trim());
    if let Ok(version) = Version::parse(value) {
        return Some(version);
    }
    for (marker, replacement) in [("rc", "-rc."), ("b", "-beta."), ("a", "-alpha.")] {
        let Some((base, suffix)) = value.rsplit_once(marker) else {
            continue;
        };
        if !base.is_empty()
            && !suffix.is_empty()
            && base
                .chars()
                .last()
                .is_some_and(|character| character.is_ascii_digit())
            && suffix.chars().all(|character| character.is_ascii_digit())
            && let Ok(version) = Version::parse(&format!("{base}{replacement}{suffix}"))
        {
            return Some(version);
        }
    }
    None
}

fn probe_cli_version(path: &Path) -> Option<String> {
    for arguments in [
        &["version", "--json"][..],
        &["--version"][..],
        &["version"][..],
    ] {
        let Some(output) = run_version_probe(path, arguments) else {
            continue;
        };
        if let Some(version) = parse_version_output(&output) {
            return Some(version);
        }
    }
    None
}

fn run_version_probe(path: &Path, arguments: &[&str]) -> Option<Vec<u8>> {
    let mut child = Command::new(path)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .by_ref()
            .take(MAX_VERSION_OUTPUT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < VERSION_PROBE_TIMEOUT => {
                thread::sleep(Duration::from_millis(20));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
        }
    }
    let bytes = reader.join().ok()?.ok()?;
    (u64::try_from(bytes.len()).ok()? <= MAX_VERSION_OUTPUT_BYTES).then_some(bytes)
}

fn parse_version_output(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?.trim();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text)
        && value
            .pointer("/data/product")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|product| product.eq_ignore_ascii_case("canisend"))
        && let Some(version) = value
            .pointer("/data/version")
            .and_then(serde_json::Value::as_str)
            .and_then(clean_version)
    {
        return Some(version);
    }
    for line in text.lines().map(str::trim) {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("local package")
            && let Some(version) = line.split_whitespace().last().and_then(clean_version)
        {
            return Some(version);
        }
        if lower.starts_with("canisend ")
            && let Some(version) = line.split_whitespace().nth(1).and_then(clean_version)
        {
            return Some(version);
        }
    }
    None
}

fn clean_version(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('v');
    (!value.is_empty()
        && value.starts_with(|character: char| character.is_ascii_digit())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".+-_".contains(character)))
    .then(|| value.to_owned())
}

fn usable_source(path: &Path) -> Option<PathBuf> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_CLI_BYTES
    {
        return None;
    }
    Some(path.to_path_buf())
}

fn validate_destination(destination: &Path) -> Result<(), ApplicationError> {
    let expected = if cfg!(windows) {
        "canisend.exe"
    } else {
        "canisend"
    };
    if destination.file_name().and_then(|name| name.to_str()) != Some(expected) {
        return Err(ApplicationError::CliInstall(format!(
            "CLI destination file must be named {expected}"
        )));
    }
    Ok(())
}

fn manifest_path(destination: &Path) -> Result<PathBuf, ApplicationError> {
    destination
        .parent()
        .map(|parent| parent.join(MANIFEST_NAME))
        .ok_or_else(|| ApplicationError::CliInstall("CLI destination has no parent".to_owned()))
}

fn read_manifest(path: &Path) -> Result<Option<InstallManifest>, ApplicationError> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(None);
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ApplicationError::CliInstall(
            "CLI install manifest must be a regular non-symlink file".to_owned(),
        ));
    }
    let bytes = fs::read(path).map_err(cli_io)?;
    let manifest: InstallManifest = serde_json::from_slice(&bytes).map_err(|error| {
        ApplicationError::CliInstall(format!("CLI install manifest is invalid: {error}"))
    })?;
    if manifest.format != INSTALL_FORMAT {
        return Err(ApplicationError::CliInstall(format!(
            "unsupported CLI install manifest format: {}",
            manifest.format
        )));
    }
    Ok(Some(manifest))
}

fn write_manifest(path: &Path, manifest: &InstallManifest) -> Result<(), ApplicationError> {
    let mut bytes = serde_json::to_vec_pretty(manifest).map_err(|error| {
        ApplicationError::CliInstall(format!("cannot encode CLI install manifest: {error}"))
    })?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(cli_io)
}

fn copy_cli(source: &Path, destination: &Path) -> Result<(), ApplicationError> {
    fs::copy(source, destination).map_err(cli_io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(destination).map_err(cli_io)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(destination, permissions).map_err(cli_io)?;
    }
    Ok(())
}

fn file_digest(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        if total > MAX_CLI_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CLI binary exceeds the supported size limit",
            ));
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex::encode(digest.finalize()))
}

fn find_on_path(destination: &Path) -> Option<PathBuf> {
    let command_name = destination.file_name()?;
    env::var_os("PATH")
        .into_iter()
        .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join(command_name))
        .find(|candidate| {
            fs::symlink_metadata(candidate)
                .is_ok_and(|metadata| metadata.is_file() || metadata.file_type().is_symlink())
        })
}

fn directory_is_on_path(directory: &Path) -> bool {
    env::var_os("PATH")
        .into_iter()
        .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .any(|entry| entry == directory || paths_refer_to_same_file(&entry, directory))
}

fn paths_refer_to_same_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn restore_rollback(destination: &Path, rollback: Option<&Path>) {
    if let Some(rollback) = rollback {
        let _ = fs::rename(rollback, destination);
    }
}

fn valid_backup_path(parent: &Path, backup: &Path) -> bool {
    backup.parent() == Some(parent)
        && backup
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".canisend.previous-"))
}

fn unix_now() -> Result<u64, ApplicationError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| ApplicationError::CliInstall("system clock is before Unix epoch".to_owned()))
}

fn nonce() -> Result<String, ApplicationError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApplicationError::CliInstall("system clock is before Unix epoch".to_owned()))?
        .as_nanos();
    Ok(format!("{}-{nanos}", std::process::id()))
}

fn cli_io(error: io::Error) -> ApplicationError {
    ApplicationError::CliInstall(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{
        CliInstallState, CliVersionRelation, TerminalInstallConsent, compare_versions, inspect,
        install, parse_version_output, uninstall,
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-cli-install-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn installs_updates_and_uninstalls_managed_cli() {
        let root = root("lifecycle");
        fs::create_dir_all(&root).expect("root");
        let source = root.join("bundle/canisend");
        let destination = root.join("bin/canisend");
        fs::create_dir_all(source.parent().expect("source parent")).expect("bundle");
        fs::write(&source, b"rust-cli-v1").expect("source");

        let before = inspect(Some(&source), &destination).expect("inspect");
        assert_eq!(before.data.state, CliInstallState::NotInstalled);
        let installed = install(
            &source,
            &destination,
            false,
            TerminalInstallConsent::granted_by_user(),
        )
        .expect("install");
        assert_eq!(installed.data.state, CliInstallState::Current);
        assert_eq!(
            fs::read(&destination).expect("installed bytes"),
            b"rust-cli-v1"
        );

        fs::write(&source, b"rust-cli-v2").expect("updated source");
        assert_eq!(
            inspect(Some(&source), &destination)
                .expect("update status")
                .data
                .state,
            CliInstallState::UpdateAvailable
        );
        install(
            &source,
            &destination,
            false,
            TerminalInstallConsent::granted_by_user(),
        )
        .expect("update");
        assert_eq!(
            fs::read(&destination).expect("updated bytes"),
            b"rust-cli-v2"
        );

        let removed = uninstall(
            Some(&source),
            &destination,
            TerminalInstallConsent::granted_by_user(),
        )
        .expect("uninstall");
        assert_eq!(removed.data.state, CliInstallState::NotInstalled);
        assert!(!destination.exists());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn replacement_preserves_and_restores_the_previous_installation() {
        let root = root("replacement");
        fs::create_dir_all(&root).expect("root");
        let source = root.join("bundle/canisend");
        let destination = root.join("bin/canisend");
        fs::create_dir_all(source.parent().expect("source parent")).expect("bundle");
        fs::create_dir_all(destination.parent().expect("destination parent")).expect("bin");
        fs::write(&source, b"rust-cli").expect("source");
        fs::write(&destination, b"previous-canisend").expect("existing");

        assert!(
            install(
                &source,
                &destination,
                false,
                TerminalInstallConsent::granted_by_user(),
            )
            .is_err()
        );
        let installed = install(
            &source,
            &destination,
            true,
            TerminalInstallConsent::granted_by_user(),
        )
        .expect("replace");
        assert!(installed.data.previous_installation_preserved);
        assert_eq!(fs::read(&destination).expect("rust bytes"), b"rust-cli");

        uninstall(
            Some(&source),
            &destination,
            TerminalInstallConsent::granted_by_user(),
        )
        .expect("restore");
        assert_eq!(
            fs::read(&destination).expect("restored bytes"),
            b"previous-canisend"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn parses_native_and_earlier_human_version_outputs() {
        let native = br#"{
          "data": {
            "product": "canisend",
            "version": "0.7.0-rc.2"
          }
        }"#;
        assert_eq!(parse_version_output(native).as_deref(), Some("0.7.0-rc.2"));
        assert_eq!(
            parse_version_output(b"CanISend version\nLocal package      0.2.0\n").as_deref(),
            Some("0.2.0")
        );
        assert_eq!(
            compare_versions(Some("0.6.0b1"), "0.7.0-rc.2"),
            CliVersionRelation::Older
        );
    }

    #[cfg(unix)]
    #[test]
    fn inspection_detects_an_older_canisend_version_without_a_shell() {
        use std::os::unix::fs::PermissionsExt;

        let root = root("version-probe");
        let source = root.join("bundle/canisend");
        let destination = root.join("bin/canisend");
        fs::create_dir_all(source.parent().expect("source parent")).expect("bundle");
        fs::create_dir_all(destination.parent().expect("destination parent")).expect("bin");
        fs::write(&source, b"bundled-cli").expect("source");
        fs::write(
            &destination,
            b"#!/bin/sh\nprintf '{\"data\":{\"product\":\"canisend\",\"version\":\"0.6.0\"}}\\n'\n",
        )
        .expect("fixture");
        let mut permissions = fs::metadata(&destination).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&destination, permissions).expect("executable");

        let status = inspect(Some(&source), &destination).expect("inspect").data;
        assert_eq!(status.state, CliInstallState::MigrationAvailable);
        assert_eq!(status.installed_version.as_deref(), Some("0.6.0"));
        assert_eq!(status.version_relation, CliVersionRelation::Older);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn installer_refuses_to_downgrade_a_newer_canisend_version() {
        use std::os::unix::fs::PermissionsExt;

        let root = root("newer-version");
        let source = root.join("bundle/canisend");
        let destination = root.join("bin/canisend");
        fs::create_dir_all(source.parent().expect("source parent")).expect("bundle");
        fs::create_dir_all(destination.parent().expect("destination parent")).expect("bin");
        fs::write(&source, b"bundled-cli").expect("source");
        let newer =
            b"#!/bin/sh\nprintf '{\"data\":{\"product\":\"canisend\",\"version\":\"9.0.0\"}}\\n'\n";
        fs::write(&destination, newer).expect("fixture");
        let mut permissions = fs::metadata(&destination).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&destination, permissions).expect("executable");

        let status = inspect(Some(&source), &destination).expect("inspect").data;
        assert_eq!(status.state, CliInstallState::NewerInstalled);
        assert_eq!(status.version_relation, CliVersionRelation::Newer);
        assert!(
            install(
                &source,
                &destination,
                true,
                TerminalInstallConsent::granted_by_user(),
            )
            .is_err()
        );
        assert_eq!(fs::read(&destination).expect("preserved"), newer);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn uninstall_refuses_modified_managed_binary() {
        let root = root("modified");
        fs::create_dir_all(&root).expect("root");
        let source = root.join("bundle/canisend");
        let destination = root.join("bin/canisend");
        fs::create_dir_all(source.parent().expect("source parent")).expect("bundle");
        fs::write(&source, b"rust-cli").expect("source");
        install(
            &source,
            &destination,
            false,
            TerminalInstallConsent::granted_by_user(),
        )
        .expect("install");
        fs::write(&destination, b"changed").expect("tamper fixture");

        assert_eq!(
            inspect(Some(&source), &destination)
                .expect("inspect")
                .data
                .state,
            CliInstallState::Modified
        );
        assert!(
            uninstall(
                Some(&source),
                &destination,
                TerminalInstallConsent::granted_by_user(),
            )
            .is_err()
        );
        assert_eq!(fs::read(&destination).expect("preserved"), b"changed");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn install_never_replaces_a_directory_destination() {
        let root = root("directory");
        fs::create_dir_all(&root).expect("root");
        let source = root.join("bundle/canisend");
        let destination = root.join("bin/canisend");
        fs::create_dir_all(source.parent().expect("source parent")).expect("bundle");
        fs::create_dir_all(&destination).expect("destination directory");
        fs::write(&source, b"rust-cli").expect("source");

        assert!(
            install(
                &source,
                &destination,
                true,
                TerminalInstallConsent::granted_by_user(),
            )
            .is_err()
        );
        assert!(destination.is_dir());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
