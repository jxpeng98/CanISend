#![cfg(target_os = "macos")]
#![forbid(unsafe_code)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use canisend_app::{Application, CliInstallState, CliVersionRelation, TerminalInstallConsent};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn temporary_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "canisend-macos-gui-cli-lifecycle-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn cli_version(path: &Path) -> String {
    let output = Command::new(path)
        .args(["version", "--json"])
        .output()
        .expect("run lifecycle CLI");
    assert!(
        output.status.success(),
        "lifecycle CLI version command failed"
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse lifecycle CLI version");
    value
        .pointer("/data/version")
        .and_then(serde_json::Value::as_str)
        .expect("lifecycle CLI version field")
        .to_owned()
}

#[test]
#[ignore = "requires two distinct signed CLI fixtures supplied by the macOS qualification script"]
fn packaged_cli_migrates_updates_rolls_back_and_retains_workspace() {
    let first_source = std::env::var_os("CANISEND_TEST_CLI_FIRST")
        .map(std::path::PathBuf::from)
        .expect("CANISEND_TEST_CLI_FIRST");
    let second_source = std::env::var_os("CANISEND_TEST_CLI_SECOND")
        .map(std::path::PathBuf::from)
        .expect("CANISEND_TEST_CLI_SECOND");
    assert!(first_source.is_file());
    assert!(second_source.is_file());
    assert_ne!(
        fs::read(&first_source).expect("read first CLI"),
        fs::read(&second_source).expect("read second CLI"),
        "qualification requires two different packaged byte units of the same product version"
    );

    let root = temporary_root();
    let destination = root.join("home/.local/bin/canisend");
    let workspace = root.join("workspace");
    fs::create_dir_all(destination.parent().expect("CLI destination parent"))
        .expect("create disposable CLI directory");
    let previous = b"#!/bin/sh\nprintf '%s\\n' \
        '{\"data\":{\"product\":\"canisend\",\"version\":\"0.7.0-rc.2\"}}'\n";
    fs::write(&destination, previous).expect("write previous CLI fixture");
    let mut permissions = fs::metadata(&destination)
        .expect("previous CLI metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&destination, permissions).expect("make previous CLI executable");

    Application::initialize_workspace(&workspace).expect("initialize retained workspace");
    Application::create_job(&workspace, "Synthetic Lecturer", "Synthetic University")
        .expect("create retained workspace job");

    let before = Application::cli_install_status(Some(&first_source), &destination)
        .expect("inspect previous installation")
        .data;
    assert_eq!(before.state, CliInstallState::MigrationAvailable);
    assert_eq!(before.version_relation, CliVersionRelation::Older);
    assert_eq!(before.installed_version.as_deref(), Some("0.7.0-rc.2"));

    let installed = Application::install_cli(
        &first_source,
        &destination,
        true,
        TerminalInstallConsent::granted_by_user(),
    )
    .expect("migrate previous CLI")
    .data;
    assert_eq!(installed.state, CliInstallState::Current);
    assert!(installed.managed);
    assert!(installed.previous_installation_preserved);
    assert_eq!(cli_version(&destination), env!("CARGO_PKG_VERSION"));

    let update = Application::cli_install_status(Some(&second_source), &destination)
        .expect("inspect second build")
        .data;
    assert_eq!(update.state, CliInstallState::UpdateAvailable);
    Application::install_cli(
        &second_source,
        &destination,
        false,
        TerminalInstallConsent::granted_by_user(),
    )
    .expect("update managed CLI");
    assert_eq!(cli_version(&destination), env!("CARGO_PKG_VERSION"));
    assert_eq!(
        fs::read(&destination).expect("read updated CLI"),
        fs::read(&second_source).expect("read second source")
    );

    let removed = Application::uninstall_cli(
        Some(&second_source),
        &destination,
        TerminalInstallConsent::granted_by_user(),
    )
    .expect("uninstall managed CLI and roll back")
    .data;
    assert_eq!(removed.state, CliInstallState::MigrationAvailable);
    assert!(!removed.managed);
    assert_eq!(fs::read(&destination).expect("read restored CLI"), previous);
    assert_eq!(cli_version(&destination), "0.7.0-rc.2");

    let retained = Application::workspace_status(&workspace).expect("reopen retained workspace");
    assert_eq!(retained.data.status.job_count, 1);
    assert!(workspace.join("canisend.toml").is_file());
    fs::remove_dir_all(root).expect("remove disposable lifecycle root");
}
