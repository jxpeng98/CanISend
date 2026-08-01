use std::{
    env, fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(windows)]
use winreg::{
    RegKey, RegValue,
    enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, RegType},
    types::FromRegValue,
};

use crate::{ActionReceipt, ApplicationError};

const INSTALL_FORMAT: &str = "canisend.cli-install/v1";
const MANIFEST_NAME: &str = ".canisend-install-v1.json";
const MAX_CLI_BYTES: u64 = 256 * 1024 * 1024;
const MAX_VERSION_OUTPUT_BYTES: u64 = 64 * 1024;
const VERSION_PROBE_TIMEOUT: Duration = Duration::from_millis(750);
const MAX_SHELL_PROFILE_BYTES: u64 = 1024 * 1024;
const PATH_BLOCK_START: &str = "# >>> CanISend CLI PATH >>>";
const PATH_BLOCK_END: &str = "# <<< CanISend CLI PATH <<<";
#[cfg(windows)]
const WINDOWS_USER_ENVIRONMENT_KEY: &str = "Environment";
#[cfg(windows)]
const WINDOWS_USER_PATH_VALUE: &str = "Path";
#[cfg(any(windows, test))]
const MAX_WINDOWS_USER_PATH_CHARS: usize = 32_767;

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
    pub path_active: bool,
    pub path_configuration_file: Option<PathBuf>,
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
    let path_active = destination.parent().is_some_and(directory_is_on_path);
    let path_configuration_file = default_path_configuration();
    let path_configured = path_active
        || destination.parent().is_some_and(|directory| {
            path_configuration_file
                .as_deref()
                .is_some_and(|configuration| persistent_path_configures(configuration, directory))
        });
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
        path_active,
        path_configuration_file,
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

pub(crate) fn configure_path(
    source: Option<&Path>,
    destination: &Path,
    _consent: TerminalInstallConsent,
) -> Result<ActionReceipt<CliInstallStatus>, ApplicationError> {
    validate_destination(destination)?;
    let configuration = default_path_configuration().ok_or_else(|| {
        ApplicationError::CliInstall(
            "Automatic PATH configuration is not supported on this platform".to_owned(),
        )
    })?;
    configure_persistent_path(destination, &configuration)?;
    let status = inspect(source, destination)?.data;
    let warning = (!status.path_active).then(|| {
        if cfg!(windows) {
            "Sign out or restart Windows before expecting the updated PATH to become active"
                .to_owned()
        } else {
            "Open a new terminal window before expecting the updated PATH to become active"
                .to_owned()
        }
    });
    Ok(ActionReceipt::new(
        "cli.path.configure",
        "configured",
        format!(
            "Configured {} for future terminal sessions",
            destination.parent().map_or_else(
                || destination.display().to_string(),
                |path| path.display().to_string()
            )
        ),
        status,
    )
    .with_warnings(warning))
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

#[cfg(unix)]
fn default_path_configuration() -> Option<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from)?;
    #[cfg(target_os = "macos")]
    {
        return Some(home.join(".zprofile"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return Some(home.join(".profile"));
    }
    #[allow(unreachable_code)]
    None
}

#[cfg(windows)]
fn default_path_configuration() -> Option<PathBuf> {
    Some(PathBuf::from(r"HKCU\Environment\Path"))
}

#[cfg(not(any(unix, windows)))]
fn default_path_configuration() -> Option<PathBuf> {
    None
}

#[cfg(unix)]
fn persistent_path_configures(configuration: &Path, directory: &Path) -> bool {
    profile_configures_path(configuration, directory)
}

#[cfg(windows)]
fn persistent_path_configures(_configuration: &Path, directory: &Path) -> bool {
    windows_user_path()
        .ok()
        .flatten()
        .is_some_and(|path| windows_path_contains(&path, &directory.to_string_lossy()))
}

#[cfg(not(any(unix, windows)))]
fn persistent_path_configures(_configuration: &Path, _directory: &Path) -> bool {
    false
}

#[cfg(unix)]
fn configure_persistent_path(
    destination: &Path,
    configuration: &Path,
) -> Result<(), ApplicationError> {
    configure_path_file(destination, configuration)
}

#[cfg(windows)]
fn configure_persistent_path(
    destination: &Path,
    _configuration: &Path,
) -> Result<(), ApplicationError> {
    configure_windows_user_path(destination)
}

#[cfg(not(any(unix, windows)))]
fn configure_persistent_path(
    _destination: &Path,
    _configuration: &Path,
) -> Result<(), ApplicationError> {
    Err(ApplicationError::CliInstall(
        "Automatic PATH configuration is not supported on this platform".to_owned(),
    ))
}

#[cfg(unix)]
fn profile_configures_path(profile: &Path, directory: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(profile) else {
        return false;
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_SHELL_PROFILE_BYTES
    {
        return false;
    }
    let Ok(text) = fs::read_to_string(profile) else {
        return false;
    };
    path_export_line(directory)
        .is_ok_and(|line| text.lines().any(|candidate| candidate.trim() == line))
}

#[cfg(unix)]
fn configure_path_file(destination: &Path, profile: &Path) -> Result<(), ApplicationError> {
    let directory = destination
        .parent()
        .ok_or_else(|| ApplicationError::CliInstall("CLI destination has no parent".to_owned()))?;
    let export_line = path_export_line(directory)?;
    let block = format!("{PATH_BLOCK_START}\n{export_line}\n{PATH_BLOCK_END}\n");
    if let Ok(metadata) = fs::symlink_metadata(profile) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(ApplicationError::CliInstall(
                "Shell profile must be a regular non-symlink file".to_owned(),
            ));
        }
        if metadata.len() > MAX_SHELL_PROFILE_BYTES {
            return Err(ApplicationError::CliInstall(format!(
                "Shell profile exceeds the {MAX_SHELL_PROFILE_BYTES}-byte limit"
            )));
        }
        let existing = fs::read_to_string(profile).map_err(cli_io)?;
        if existing.lines().any(|line| line.trim() == export_line) {
            return Ok(());
        }
        if existing.contains(PATH_BLOCK_START) || existing.contains(PATH_BLOCK_END) {
            return Err(ApplicationError::CliInstall(
                "Existing CanISend PATH block is incomplete or points to another directory"
                    .to_owned(),
            ));
        }
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(profile)
            .map_err(cli_io)?;
        if !existing.is_empty() && !existing.ends_with('\n') {
            file.write_all(b"\n").map_err(cli_io)?;
        }
        file.write_all(block.as_bytes()).map_err(cli_io)?;
        file.sync_all().map_err(cli_io)?;
        return Ok(());
    }
    let parent = profile.parent().ok_or_else(|| {
        ApplicationError::CliInstall("Shell profile has no parent directory".to_owned())
    })?;
    fs::create_dir_all(parent).map_err(cli_io)?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options.open(profile).map_err(cli_io)?;
    file.write_all(block.as_bytes()).map_err(cli_io)?;
    file.sync_all().map_err(cli_io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = file.metadata().map_err(cli_io)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(profile, permissions).map_err(cli_io)?;
    }
    Ok(())
}

#[cfg(unix)]
fn path_export_line(directory: &Path) -> Result<String, ApplicationError> {
    let directory = directory.to_str().ok_or_else(|| {
        ApplicationError::CliInstall("CLI PATH directory must be valid UTF-8".to_owned())
    })?;
    if directory.is_empty()
        || directory
            .chars()
            .any(|character| character.is_control() || matches!(character, '"' | '\\' | '$' | '`'))
    {
        return Err(ApplicationError::CliInstall(
            "CLI PATH directory contains characters that cannot be written safely".to_owned(),
        ));
    }
    Ok(format!("export PATH=\"{directory}:$PATH\""))
}

#[cfg(windows)]
fn is_windows_path_registry_type(value_type: &RegType) -> bool {
    value_type == &RegType::REG_SZ || value_type == &RegType::REG_EXPAND_SZ
}

#[cfg(windows)]
fn windows_user_path() -> io::Result<Option<String>> {
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let environment =
        match current_user.open_subkey_with_flags(WINDOWS_USER_ENVIRONMENT_KEY, KEY_QUERY_VALUE) {
            Ok(environment) => environment,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
    match environment.get_raw_value(WINDOWS_USER_PATH_VALUE) {
        Ok(value) if is_windows_path_registry_type(&value.vtype) => {
            String::from_reg_value(&value).map(Some)
        }
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "HKCU Environment Path is not a string registry value",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
fn configure_windows_user_path(destination: &Path) -> Result<(), ApplicationError> {
    let directory = destination
        .parent()
        .ok_or_else(|| ApplicationError::CliInstall("CLI destination has no parent".to_owned()))?;
    let directory = directory.to_str().ok_or_else(|| {
        ApplicationError::CliInstall("CLI PATH directory must be valid Unicode".to_owned())
    })?;
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let (environment, _) = current_user
        .create_subkey_with_flags(
            WINDOWS_USER_ENVIRONMENT_KEY,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
        )
        .map_err(cli_io)?;
    let (current, value_type) = match environment.get_raw_value(WINDOWS_USER_PATH_VALUE) {
        Ok(value) if is_windows_path_registry_type(&value.vtype) => {
            let current = String::from_reg_value(&value).map_err(cli_io)?;
            (current, value.vtype.clone())
        }
        Ok(_) => {
            return Err(ApplicationError::CliInstall(
                "HKCU Environment Path is not a string registry value".to_owned(),
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            (String::new(), RegType::REG_EXPAND_SZ)
        }
        Err(error) => return Err(cli_io(error)),
    };
    let updated = windows_path_with_entry(&current, directory)?;
    if updated == current {
        return Ok(());
    }
    let mut bytes = Vec::with_capacity((updated.encode_utf16().count() + 1) * 2);
    for unit in updated.encode_utf16().chain(std::iter::once(0)) {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    environment
        .set_raw_value(
            WINDOWS_USER_PATH_VALUE,
            &RegValue {
                bytes,
                vtype: value_type,
            },
        )
        .map_err(cli_io)
}

#[cfg(any(windows, test))]
fn windows_path_with_entry(current: &str, directory: &str) -> Result<String, ApplicationError> {
    if directory.is_empty()
        || directory
            .chars()
            .any(|character| character.is_control() || character == ';')
    {
        return Err(ApplicationError::CliInstall(
            "CLI PATH directory cannot be represented in the Windows user PATH".to_owned(),
        ));
    }
    if windows_path_contains(current, directory) {
        return Ok(current.to_owned());
    }
    let separator = (!current.is_empty() && !current.ends_with(';')).then_some(';');
    let additional_chars = directory.encode_utf16().count() + usize::from(separator.is_some());
    if current.encode_utf16().count() + additional_chars > MAX_WINDOWS_USER_PATH_CHARS {
        return Err(ApplicationError::CliInstall(format!(
            "Windows user PATH would exceed {MAX_WINDOWS_USER_PATH_CHARS} UTF-16 code units"
        )));
    }
    let mut updated = String::with_capacity(current.len() + directory.len() + 1);
    updated.push_str(current);
    if let Some(separator) = separator {
        updated.push(separator);
    }
    updated.push_str(directory);
    Ok(updated)
}

#[cfg(any(windows, test))]
fn windows_path_contains(current: &str, directory: &str) -> bool {
    let expected = directory
        .trim()
        .trim_matches('"')
        .trim_end_matches(['\\', '/']);
    current.split(';').any(|entry| {
        entry
            .trim()
            .trim_matches('"')
            .trim_end_matches(['\\', '/'])
            .eq_ignore_ascii_case(expected)
    })
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
        install, parse_version_output, uninstall, windows_path_contains, windows_path_with_entry,
    };
    #[cfg(unix)]
    use super::{configure_path_file, profile_configures_path};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canisend-cli-install-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn cli_name() -> &'static str {
        if cfg!(windows) {
            "canisend.exe"
        } else {
            "canisend"
        }
    }

    #[test]
    fn installs_updates_and_uninstalls_managed_cli() {
        let root = root("lifecycle");
        fs::create_dir_all(&root).expect("root");
        let source = root.join("bundle").join(cli_name());
        let destination = root.join("bin").join(cli_name());
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
        let source = root.join("bundle").join(cli_name());
        let destination = root.join("bin").join(cli_name());
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
        let source = root.join("bundle").join(cli_name());
        let destination = root.join("bin").join(cli_name());
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
        let source = root.join("bundle").join(cli_name());
        let destination = root.join("bin").join(cli_name());
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
        let source = root.join("bundle").join(cli_name());
        let destination = root.join("bin").join(cli_name());
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
        let source = root.join("bundle").join(cli_name());
        let destination = root.join("bin").join(cli_name());
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

    #[cfg(unix)]
    #[test]
    fn path_configuration_is_bounded_idempotent_and_never_follows_symlinks() {
        let root = root("path-config");
        fs::create_dir_all(root.join("bin")).expect("bin");
        let destination = root.join("bin/canisend");
        let profile = root.join(".zprofile");

        configure_path_file(&destination, &profile).expect("configure path");
        let first = fs::read_to_string(&profile).expect("profile");
        configure_path_file(&destination, &profile).expect("configure idempotently");
        assert_eq!(fs::read_to_string(&profile).expect("profile again"), first);
        assert!(profile_configures_path(
            &profile,
            destination.parent().expect("parent")
        ));

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&profile, root.join("linked-profile"))
                .expect("profile symlink");
            assert!(configure_path_file(&destination, &root.join("linked-profile")).is_err());
        }
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn windows_path_configuration_is_case_insensitive_bounded_and_idempotent() {
        let current = r"C:\Windows;C:\Users\Example\Tools";
        assert!(windows_path_contains(current, r"c:\users\example\tools\"));
        assert_eq!(
            windows_path_with_entry(current, r"C:\Users\Example\Tools").expect("existing entry"),
            current
        );
        assert_eq!(
            windows_path_with_entry(current, r"C:\Users\Example\CanISend\bin").expect("new entry"),
            r"C:\Windows;C:\Users\Example\Tools;C:\Users\Example\CanISend\bin"
        );
        assert!(windows_path_with_entry(current, "C:\\bad;entry").is_err());
        assert!(windows_path_with_entry(&"x".repeat(32_767), "y").is_err());
    }
}
