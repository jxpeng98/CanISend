#![cfg(target_os = "macos")]
#![forbid(unsafe_code)]

use std::{
    fs,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use canisend_app::{Application, CliInstallState, CliVersionRelation, TerminalInstallConsent};
use sha2::{Digest, Sha256};

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

fn run_json(path: &Path, arguments: &[&str]) -> serde_json::Value {
    let output = Command::new(path)
        .args(arguments)
        .output()
        .expect("run lifecycle CLI command");
    assert!(
        output.status.success(),
        "lifecycle CLI command failed: {}\nstdout: {}\nstderr: {}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("parse lifecycle CLI JSON")
}

fn file_digest(path: &Path) -> String {
    let mut file = fs::File::open(path).expect("open lifecycle file for digest");
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).expect("read lifecycle file");
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    hex::encode(hasher.finalize())
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

#[test]
#[ignore = "requires the exact verified public 0.7 CLI and a signed current CLI fixture"]
fn public_07_cli_upgrades_with_verified_backup_and_exact_rollback() {
    let old_source = std::env::var_os("CANISEND_TEST_PUBLIC_07_CLI")
        .map(std::path::PathBuf::from)
        .expect("CANISEND_TEST_PUBLIC_07_CLI");
    let new_source = std::env::var_os("CANISEND_TEST_CURRENT_CLI")
        .map(std::path::PathBuf::from)
        .expect("CANISEND_TEST_CURRENT_CLI");
    let expected_old_digest =
        std::env::var("CANISEND_TEST_PUBLIC_07_CLI_SHA256").expect("old CLI digest");
    assert_eq!(cli_version(&old_source), "0.7.0-rc.2");
    assert_eq!(cli_version(&new_source), env!("CARGO_PKG_VERSION"));
    assert_eq!(file_digest(&old_source), expected_old_digest);

    let root = temporary_root();
    let destination = root.join("home/.local/bin/canisend");
    let workspace = root.join("workspace");
    let backup = root.join("pre-upgrade-backup");
    let restored_by_old = root.join("restored-by-0.7");
    let restored_by_new = root.join("restored-by-1.0");
    fs::create_dir_all(destination.parent().expect("CLI destination parent"))
        .expect("create disposable CLI directory");
    fs::copy(&old_source, &destination).expect("install exact public 0.7 CLI");
    let mut permissions = fs::metadata(&destination)
        .expect("public 0.7 CLI metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&destination, permissions).expect("make public 0.7 CLI executable");

    let workspace_text = workspace.to_str().expect("UTF-8 workspace path");
    let backup_text = backup.to_str().expect("UTF-8 backup path");
    assert_eq!(file_digest(&destination), expected_old_digest);
    assert_eq!(cli_version(&destination), "0.7.0-rc.2");
    assert_eq!(
        run_json(
            &destination,
            &["--workspace", workspace_text, "workspace", "init", "--json"]
        )["ok"],
        true
    );
    assert_eq!(
        run_json(
            &destination,
            &[
                "--workspace",
                workspace_text,
                "job",
                "create",
                "--title",
                "Synthetic public archive upgrade role",
                "--institution",
                "CanISend local qualification",
                "--json",
            ]
        )["ok"],
        true
    );
    assert_eq!(
        run_json(
            &destination,
            &[
                "--workspace",
                workspace_text,
                "workspace",
                "check",
                "--json",
            ]
        )
        .pointer("/data/ok"),
        Some(&serde_json::Value::Bool(true))
    );
    assert_eq!(
        run_json(
            &destination,
            &[
                "--workspace",
                workspace_text,
                "workspace",
                "backup",
                backup_text,
                "--json",
            ]
        )["status"],
        "verified"
    );
    canisend_store::verify_backup(&backup).expect("verify public 0.7 workspace backup");
    let backup_manifest = backup.join("backup-manifest.json");
    let backup_manifest_digest = file_digest(&backup_manifest);
    let database = workspace.join(".canisend/state.sqlite3");
    let database_before_refusal = file_digest(&database);

    let before = Application::cli_install_status(Some(&new_source), &destination)
        .expect("inspect public 0.7 installation")
        .data;
    assert_eq!(before.state, CliInstallState::MigrationAvailable);
    assert_eq!(before.version_relation, CliVersionRelation::Older);
    assert_eq!(before.installed_version.as_deref(), Some("0.7.0-rc.2"));

    assert!(
        Application::install_cli(
            &new_source,
            &destination,
            false,
            TerminalInstallConsent::granted_by_user(),
        )
        .is_err(),
        "an unmanaged public CLI must require explicit replacement"
    );
    assert_eq!(file_digest(&destination), expected_old_digest);
    assert_eq!(file_digest(&database), database_before_refusal);
    assert_eq!(file_digest(&backup_manifest), backup_manifest_digest);

    let installed = Application::install_cli(
        &new_source,
        &destination,
        true,
        TerminalInstallConsent::granted_by_user(),
    )
    .expect("upgrade exact public 0.7 CLI")
    .data;
    assert_eq!(installed.state, CliInstallState::Current);
    assert!(installed.managed);
    assert!(installed.previous_installation_preserved);
    assert_eq!(cli_version(&destination), env!("CARGO_PKG_VERSION"));
    assert_eq!(file_digest(&destination), file_digest(&new_source));

    let upgraded = Application::workspace_status(&workspace).expect("open workspace with 1.0");
    assert_eq!(upgraded.data.status.job_count, 1);
    assert!(
        Application::check_workspace(&workspace)
            .expect("check upgraded workspace")
            .data
            .check
            .ok
    );
    assert_eq!(file_digest(&backup_manifest), backup_manifest_digest);
    canisend_store::verify_backup(&backup).expect("backup remains verified after upgrade");

    let restored_by_new_text = restored_by_new.to_str().expect("UTF-8 restore path");
    assert_eq!(
        run_json(
            &destination,
            &[
                "workspace",
                "restore",
                backup_text,
                restored_by_new_text,
                "--json",
            ]
        )["ok"],
        true
    );
    let current_restore =
        Application::workspace_status(&restored_by_new).expect("open current restore");
    assert_eq!(current_restore.data.status.job_count, 1);

    let database_before_old_attempt = file_digest(&database);
    let old_attempt = Command::new(&old_source)
        .args([
            "--workspace",
            workspace_text,
            "workspace",
            "status",
            "--json",
        ])
        .output()
        .expect("probe upgraded workspace with public 0.7 CLI");
    let old_attempt_json: serde_json::Value =
        serde_json::from_slice(&old_attempt.stdout).expect("parse old CLI probe");
    if old_attempt.status.success() {
        assert_eq!(old_attempt_json["ok"], true);
    } else {
        assert_eq!(
            old_attempt_json
                .pointer("/error/code")
                .and_then(|value| value.as_str()),
            Some("workspace.conflict")
        );
        assert_eq!(file_digest(&database), database_before_old_attempt);
    }

    let restored_by_old_text = restored_by_old.to_str().expect("UTF-8 restore path");
    assert_eq!(
        run_json(
            &old_source,
            &[
                "workspace",
                "restore",
                backup_text,
                restored_by_old_text,
                "--json",
            ]
        )["ok"],
        true
    );
    assert_eq!(
        run_json(
            &old_source,
            &[
                "--workspace",
                restored_by_old_text,
                "workspace",
                "status",
                "--json",
            ]
        )
        .pointer("/data/job_count")
        .and_then(serde_json::Value::as_u64),
        Some(1)
    );

    let removed = Application::uninstall_cli(
        Some(&new_source),
        &destination,
        TerminalInstallConsent::granted_by_user(),
    )
    .expect("uninstall 1.0 CLI and restore exact public 0.7 CLI")
    .data;
    assert_eq!(removed.state, CliInstallState::MigrationAvailable);
    assert!(!removed.managed);
    assert_eq!(cli_version(&destination), "0.7.0-rc.2");
    assert_eq!(file_digest(&destination), expected_old_digest);
    assert_eq!(file_digest(&backup_manifest), backup_manifest_digest);
    assert!(workspace.join("canisend.toml").is_file());
    assert!(backup_manifest.is_file());
    assert!(restored_by_old.join("canisend.toml").is_file());
    assert!(restored_by_new.join("canisend.toml").is_file());
    fs::remove_dir_all(root).expect("remove disposable public upgrade root");
}
