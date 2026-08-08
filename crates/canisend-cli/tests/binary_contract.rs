use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::Value;

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        Self(std::env::temp_dir().join(format!(
            "canisend-cli-v4-{label}-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        )))
    }

    fn text(&self) -> &str {
        self.0.to_str().expect("test path is UTF-8")
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_canisend"))
        .args(arguments)
        .output()
        .expect("canisend binary runs")
}

fn run_json(arguments: &[&str]) -> Value {
    let output = run(arguments);
    assert!(
        output.status.success(),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stderr.is_empty(),
        "successful JSON command wrote stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert_eq!(stdout.lines().count(), 1, "JSON stdout is one object");
    serde_json::from_str(&stdout).expect("stdout is JSON")
}

fn file_snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, snapshot: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let mut entries = fs::read_dir(path)
            .expect("read snapshot directory")
            .collect::<Result<Vec<_>, _>>()
            .expect("read snapshot entries");
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).expect("snapshot metadata");
            if metadata.is_dir() {
                visit(root, &path, snapshot);
            } else {
                snapshot.insert(
                    path.strip_prefix(root)
                        .expect("snapshot relative path")
                        .to_owned(),
                    fs::read(path).expect("snapshot file bytes"),
                );
            }
        }
    }

    let mut snapshot = BTreeMap::new();
    visit(root, root, &mut snapshot);
    snapshot
}

#[test]
fn version_truth_remains_bound_to_the_current_source_release() {
    let value = run_json(&["version", "--json"]);
    assert_eq!(value["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["data"]["product"], "canisend");
}

#[test]
fn doctor_proves_embedded_resources_and_no_python_requirement() {
    let value = run_json(&["doctor", "--json"]);
    assert_eq!(value["status"], "healthy");
    assert_eq!(value["data"]["resource_manifest"], "verified");
    assert_eq!(value["data"]["embedded_typst"], "verified");
    assert_eq!(value["data"]["runtime_package_downloads"], false);
    assert_eq!(value["data"]["python_required"], false);
    assert_eq!(value["data"]["render_probe"]["page_count"], 2);
}

#[test]
fn public_catalogs_are_available_without_a_workspace() {
    let schemas = run_json(&["schema", "list", "--json"]);
    let resources = run_json(&["resource", "list", "--json"]);
    assert!(
        schemas["data"]["schemas"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(
        resources["data"]["resources"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
}

#[test]
fn known_json_error_uses_stdout_only_and_validation_exit_code() {
    let output = run(&["schema", "show", "missing", "--json"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let response: Value = serde_json::from_slice(&output.stdout).expect("error response JSON");
    assert_eq!(response["error"]["code"], "schema.not_found");
    assert_eq!(response["ok"], false);
}

#[test]
fn public_help_excludes_every_alpha6_legacy_command_family() {
    let help = run(&["--help"]);
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).expect("root help is UTF-8");
    for legacy in [
        "agent",
        "job",
        "content",
        "profile",
        "discovery",
        "task",
        "criteria",
        "match",
        "plan",
        "document",
        "review",
        "package",
        "render",
        "workflow",
    ] {
        assert!(
            !help
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{legacy} "))),
            "legacy command `{legacy}` remains public:\n{help}"
        );
    }

    let application_help = run(&["application", "--help"]);
    assert!(application_help.status.success());
    let application_help =
        String::from_utf8(application_help.stdout).expect("Application help is UTF-8");
    assert!(!application_help.contains("generic-plan"));
    assert!(!application_help.contains("generic-compose"));
    assert!(!application_help.contains("generic-approve"));
    assert!(!application_help.contains("generic-export"));
}

#[test]
fn legacy_commands_fail_before_workspace_discovery_or_mutation() {
    let workspace = TestDirectory::new("legacy-refusal");
    fs::create_dir_all(workspace.path().join(".canisend")).expect("legacy fixture directory");
    fs::write(
        workspace.path().join("canisend.toml"),
        b"format = \"canisend.workspace/v3\"\nprivate = \"LEGACY-SENTINEL\"\n",
    )
    .expect("legacy config fixture");
    fs::write(
        workspace.path().join(".canisend/state.sqlite3"),
        b"LEGACY-DATABASE-SENTINEL",
    )
    .expect("legacy database fixture");
    let before = file_snapshot(workspace.path());

    for command in [
        vec!["job", "list", "--json"],
        vec!["agent", "capabilities", "--json"],
        vec![
            "application",
            "generic-plan",
            "--application",
            "019f3e88-6630-7000-8000-000000000001",
            "--candidate",
            "/does/not/exist.json",
            "--json",
        ],
    ] {
        let mut arguments = vec!["--workspace", workspace.text()];
        arguments.extend(command);
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(4));
        assert!(output.stderr.is_empty());
        let response: Value =
            serde_json::from_slice(&output.stdout).expect("legacy refusal is JSON");
        assert_eq!(response["protocol"], "canisend.agent/v4");
        assert_eq!(response["operation"], "compatibility.refuse");
        assert_eq!(response["status"], "unsupported-legacy-surface");
        assert_eq!(response["error"]["code"], "compatibility.unavailable");
        assert_eq!(response["error"]["details"]["mutation_attempted"], false);
        assert_eq!(response["submission_performed"], false);
        assert_eq!(file_snapshot(workspace.path()), before);
    }

    let missing = TestDirectory::new("legacy-refusal-missing");
    let output = run(&[
        "--workspace",
        missing.text(),
        "task",
        "show",
        "019f3e88-6630-7000-8000-000000000001",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(4));
    assert!(!missing.path().exists(), "refusal created a Workspace path");
}

#[test]
fn workspace_v4_initialize_check_backup_restore_and_repair_round_trip() {
    let workspace = TestDirectory::new("workspace");
    let backup = TestDirectory::new("backup");
    let restored = TestDirectory::new("restored");

    let initialized = run_json(&[
        "--workspace",
        workspace.text(),
        "workspace",
        "init",
        "--json",
    ]);
    let status = run_json(&[
        "--workspace",
        workspace.text(),
        "workspace",
        "status",
        "--json",
    ]);
    let check = run_json(&[
        "--workspace",
        workspace.text(),
        "workspace",
        "check",
        "--json",
    ]);
    let backup_result = run_json(&[
        "--workspace",
        workspace.text(),
        "workspace",
        "backup",
        backup.text(),
        "--json",
    ]);
    let restore_result = run_json(&[
        "workspace",
        "restore",
        backup.text(),
        restored.text(),
        "--json",
    ]);
    let repair_result = run_json(&[
        "--workspace",
        restored.text(),
        "workspace",
        "repair",
        "--json",
    ]);

    assert_eq!(initialized["operation"], "workspace.initialize.commit");
    assert_eq!(status["data"]["workspace_format"], "canisend.workspace/v4");
    assert_eq!(check["data"]["ok"], true);
    assert_eq!(backup_result["operation"], "workspace.backup.commit");
    assert_eq!(restore_result["operation"], "workspace.restore.commit");
    assert_eq!(repair_result["operation"], "workspace.repair.commit");
    assert_eq!(
        restore_result["data"]["workspace_id"],
        status["data"]["workspace_id"]
    );
}

#[test]
fn workspace_v4_holds_generic_and_academic_applications_together() {
    let workspace = TestDirectory::new("mixed-pack");
    let candidates = TestDirectory::new("mixed-pack-candidates");
    fs::create_dir_all(candidates.path()).expect("candidate directory");
    let generic_candidate = candidates.path().join("generic.json");
    let academic_candidate = candidates.path().join("academic.json");
    write_application_candidate(
        &generic_candidate,
        "Community programme",
        "organization",
        "Example Foundation",
        "Applicants must provide a project narrative.",
        "format",
    );
    write_application_candidate(
        &academic_candidate,
        "Research fellowship",
        "institution",
        "Example University",
        "Applicants must submit an academic CV.",
        "qualification",
    );

    run_json(&[
        "--workspace",
        workspace.text(),
        "workspace",
        "init",
        "--json",
    ]);
    let generic = create_application(
        &workspace,
        "org.canisend.generic-application",
        &generic_candidate,
    );
    let academic = create_application(&workspace, "org.canisend.academic-job", &academic_candidate);
    assert_eq!(generic["operation"], "application.create.commit");
    assert_eq!(academic["operation"], "application.create.commit");

    let listed = run_json(&[
        "--workspace",
        workspace.text(),
        "application",
        "list",
        "--json",
    ]);
    assert_eq!(listed["data"].as_array().map(Vec::len), Some(2));
    let academic_id = academic["data"]["stored"]["snapshot"]["application"]["id"]
        .as_str()
        .expect("academic Application ID");
    let shown = run_json(&[
        "--workspace",
        workspace.text(),
        "application",
        "show",
        "--application",
        academic_id,
        "--json",
    ]);
    assert_eq!(
        shown["data"]["snapshot"]["pack"]["id"],
        "org.canisend.academic-job"
    );
}

fn write_application_candidate(
    path: &Path,
    title: &str,
    metadata_key: &str,
    metadata_value: &str,
    source: &str,
    category: &str,
) {
    fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({
            "title": title,
            "opportunity_metadata": {
                (metadata_key): {"type": "short-text", "value": metadata_value}
            },
            "application_metadata": {},
            "source_text": source,
            "requirements": [{
                "category": category,
                "statement": source,
                "priority": "mandatory",
                "start_byte": 0,
                "end_byte": source.len()
            }]
        }))
        .expect("candidate JSON"),
    )
    .expect("write Application candidate");
}

fn create_application(workspace: &TestDirectory, pack: &str, candidate: &Path) -> Value {
    run_json(&[
        "--workspace",
        workspace.text(),
        "application",
        "create",
        "--pack",
        pack,
        "--candidate",
        candidate.to_str().expect("candidate path is UTF-8"),
        "--json",
    ])
}
