use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::Value;

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        Self(std::env::temp_dir().join(format!(
            "canisend-cli-{label}-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        )))
    }

    fn text(&self) -> &str {
        self.0.to_str().expect("test path is UTF-8")
    }

    fn path(&self) -> &std::path::Path {
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
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful JSON command wrote stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert_eq!(
        stdout.lines().count(),
        1,
        "JSON stdout must contain one object"
    );
    serde_json::from_str(&stdout).expect("stdout is JSON")
}

fn run_json_stdin(arguments: &[&str], input: &[u8]) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_canisend"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("canisend binary starts");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(input)
        .expect("completion input");
    let output = child.wait_with_output().expect("canisend exits");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

fn assert_json_snapshot(arguments: &[&str], snapshot: &str) {
    let first = run(arguments);
    let second = run(arguments);
    assert_eq!(
        first.stdout, second.stdout,
        "JSON output is not deterministic"
    );
    let actual: Value = serde_json::from_slice(&first.stdout).expect("actual snapshot is JSON");
    let expected: Value = serde_json::from_str(snapshot).expect("committed snapshot is JSON");
    assert_eq!(actual, expected);
}

#[test]
fn version_reports_native_v2_contract() {
    let value = run_json(&["version", "--json"]);

    assert_eq!(value["protocol"], "canisend.agent/v2");
    assert_eq!(value["operation"], "product.version");
    assert_eq!(value["data"]["workspace_format"], "canisend.workspace/v2");
    assert_eq!(value["data"]["version"], "0.7.0-alpha.1");
}

#[test]
fn doctor_proves_embedded_resources_and_no_python_requirement() {
    let value = run_json(&["doctor", "--json"]);

    assert_eq!(value["status"], "healthy");
    assert_eq!(value["data"]["resource_manifest"], "verified");
    assert_eq!(value["data"]["python_required"], false);
}

#[test]
fn capabilities_distinguish_available_from_planned_work() {
    let value = run_json(&["agent", "capabilities", "--json"]);
    let capabilities = value["data"]["capabilities"]
        .as_array()
        .expect("capabilities are an array");

    assert!(
        capabilities
            .iter()
            .any(|item| { item["id"] == "agent.capabilities" && item["status"] == "available" })
    );
    assert!(
        capabilities
            .iter()
            .any(|item| { item["id"] == "workspace.lifecycle" && item["status"] == "available" })
    );
    assert!(
        capabilities
            .iter()
            .any(|item| { item["id"] == "job.intake" && item["status"] == "available" })
    );
    assert_eq!(
        value["data"]["discovery_adapters"].as_array().map(Vec::len),
        Some(4)
    );
    assert_eq!(
        value["data"]["stages"].as_array().map(|stages| stages
            .iter()
            .filter(|stage| stage["status"] == "available")
            .count()),
        Some(7)
    );
}

#[test]
fn public_catalogs_are_available_without_a_workspace() {
    let schemas = run_json(&["schema", "list", "--json"]);
    let resources = run_json(&["resource", "list", "--json"]);

    assert_eq!(
        schemas["data"]["schemas"].as_array().map(Vec::len),
        Some(29)
    );
    assert!(
        resources["data"]["resources"]
            .as_array()
            .is_some_and(|items| items.len() >= 20)
    );
}

#[test]
fn capabilities_and_context_match_committed_json_snapshots() {
    assert_json_snapshot(
        &["agent", "capabilities", "--json"],
        include_str!("snapshots/agent-capabilities.json"),
    );
    assert_json_snapshot(
        &["agent", "context", "--json"],
        include_str!("snapshots/agent-context.json"),
    );
}

#[test]
fn agent_host_pack_export_is_versioned_and_self_contained() {
    let parent = TestDirectory::new("agent-pack-parent");
    fs::create_dir_all(parent.path()).expect("pack parent");
    let pack = parent.path().join("codex");
    let exported = run_json(&[
        "agent",
        "assets",
        "export",
        "--host",
        "codex",
        "--destination",
        pack.to_str().expect("pack path"),
        "--json",
    ]);
    assert_eq!(exported["status"], "exported");
    assert_eq!(exported["data"]["manifest"]["host"], "codex");
    assert_eq!(
        exported["data"]["manifest"]["files"]
            .as_array()
            .map(Vec::len),
        Some(17)
    );
    assert!(pack.join("AGENTS.md").is_file());
    assert!(pack.join("prompts/job-parse.md").is_file());
    assert!(pack.join("prompts/evidence-normalize.md").is_file());
    assert!(pack.join("prompts/evidence-match.md").is_file());
    assert!(
        pack.join("schemas/v2/task-completion.schema.json")
            .is_file()
    );
    assert!(pack.join("schemas/v2/parsed-job.schema.json").is_file());
    assert!(pack.join("schemas/v2/criteria.schema.json").is_file());
    assert!(
        pack.join("schemas/v2/evidence-proposals.schema.json")
            .is_file()
    );
    assert!(
        pack.join("schemas/v2/evidence-catalog.schema.json")
            .is_file()
    );
    assert!(
        pack.join("schemas/v2/evidence-match-proposals.schema.json")
            .is_file()
    );
    assert!(
        pack.join("schemas/v2/evidence-matches.schema.json")
            .is_file()
    );
    assert!(pack.join("canisend-agent-pack.json").is_file());
}

#[test]
fn known_json_error_uses_stdout_only_and_validation_exit_code() {
    let output = run(&["schema", "show", "missing", "--json"]);

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert_eq!(stdout.lines().count(), 1);
    let response: Value = serde_json::from_str(&stdout).expect("error response is JSON");
    assert_eq!(response["error"]["code"], "schema.not_found");
    assert_eq!(response["ok"], false);
}

#[test]
fn native_workspace_commands_initialize_check_backup_and_restore() {
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

    assert_eq!(initialized["status"], "initialized");
    assert_eq!(
        status["data"]["workspace_id"],
        initialized["data"]["workspace_id"]
    );
    assert_eq!(check["data"]["ok"], true);
    assert_eq!(backup_result["status"], "verified");
    assert_eq!(
        restore_result["data"]["workspace_id"],
        status["data"]["workspace_id"]
    );
}

#[test]
fn native_job_commands_import_original_and_normalized_local_text() {
    let workspace = TestDirectory::new("job-workspace");
    let input = TestDirectory::new("job-input");
    fs::create_dir_all(input.path()).expect("input directory");
    let advert = input.path().join("advert.md");
    fs::write(
        &advert,
        b"# Lecturer in Economics  \r\n\r\nTeach economics.\r\n",
    )
    .expect("write advert");

    run_json(&[
        "--workspace",
        workspace.text(),
        "workspace",
        "init",
        "--json",
    ]);
    let created = run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "create",
        "--title",
        "Lecturer in Economics",
        "--institution",
        "University X",
        "--json",
    ]);
    let job_id = created["data"]["id"].as_str().expect("job ID");
    let imported = run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "import",
        job_id,
        "--file",
        advert.to_str().expect("advert path is UTF-8"),
        "--json",
    ]);
    assert_eq!(imported["data"]["kind"], "local-file");
    assert_ne!(
        imported["data"]["original"]["sha256"],
        imported["data"]["normalized_text"]["sha256"]
    );

    let pdf = input.path().join("advert.pdf");
    write_pdf(&pdf, Some("Text PDF job advert"));
    let pdf_imported = run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "import",
        job_id,
        "--file",
        pdf.to_str().expect("PDF path is UTF-8"),
        "--json",
    ]);
    assert_eq!(pdf_imported["data"]["content_type"], "application/pdf");

    let image_only = input.path().join("image-only.pdf");
    write_pdf(&image_only, None);
    let unavailable = run(&[
        "--workspace",
        workspace.text(),
        "job",
        "import",
        job_id,
        "--file",
        image_only.to_str().expect("image-only path is UTF-8"),
        "--json",
    ]);
    assert_eq!(unavailable.status.code(), Some(3));
    assert!(unavailable.stderr.is_empty());
    let unavailable: Value =
        serde_json::from_slice(&unavailable.stdout).expect("PDF error is JSON");
    assert_eq!(unavailable["error"]["code"], "pdf_text_unavailable");

    let private_url = run(&[
        "--workspace",
        workspace.text(),
        "job",
        "import",
        job_id,
        "--url",
        "http://127.0.0.1:9/private",
        "--json",
    ]);
    assert_eq!(private_url.status.code(), Some(3));
    assert!(private_url.stderr.is_empty());

    let shown = run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "show",
        job_id,
        "--json",
    ]);
    assert_eq!(shown["data"]["job"]["revision"], 3);
    assert_eq!(shown["data"]["sources"].as_array().map(Vec::len), Some(2));
    let context = run_json(&[
        "--workspace",
        workspace.text(),
        "agent",
        "context",
        "--job",
        job_id,
        "--json",
    ]);
    assert_eq!(context["data"]["selected_job"]["source_count"], 2);
    assert_eq!(context["data"]["active_job_id"], job_id);
    assert!(
        !serde_json::to_string(&context)
            .expect("context JSON")
            .contains("Teach economics")
    );
    let listed = run_json(&["--workspace", workspace.text(), "job", "list", "--json"]);
    assert_eq!(listed["data"]["jobs"].as_array().map(Vec::len), Some(1));

    let archived = run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "archive",
        job_id,
        "--json",
    ]);
    assert_eq!(archived["data"]["archived"], true);
    let active = run_json(&["--workspace", workspace.text(), "job", "list", "--json"]);
    assert_eq!(active["data"]["jobs"].as_array().map(Vec::len), Some(0));
    let all = run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "list",
        "--include-archived",
        "--json",
    ]);
    assert_eq!(all["data"]["jobs"].as_array().map(Vec::len), Some(1));
    assert_eq!(all["data"]["jobs"][0]["revision"], 4);
}

#[test]
fn profile_source_commands_import_json_without_returning_private_bodies() {
    let workspace = TestDirectory::new("profile-workspace");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/v2-spec/profile-evidence.json");
    run_json(&[
        "--workspace",
        workspace.text(),
        "workspace",
        "init",
        "--json",
    ]);
    let imported = run_json(&[
        "--workspace",
        workspace.text(),
        "profile",
        "source",
        "add",
        "--file",
        fixture.to_str().expect("profile fixture path"),
        "--json",
    ]);
    assert_eq!(imported["status"], "imported");
    assert_eq!(imported["data"]["kind"], "json");
    assert_eq!(imported["data"]["sensitivity"], "private-local");
    assert!(
        !serde_json::to_string(&imported)
            .expect("profile response")
            .contains("PhD in Economics awarded")
    );
    let source_id = imported["data"]["id"].as_str().expect("source ID");
    let listed = run_json(&[
        "--workspace",
        workspace.text(),
        "profile",
        "source",
        "list",
        "--json",
    ]);
    assert_eq!(listed["data"]["sources"].as_array().map(Vec::len), Some(1));
    let shown = run_json(&[
        "--workspace",
        workspace.text(),
        "profile",
        "source",
        "show",
        source_id,
        "--json",
    ]);
    assert_eq!(shown["data"], imported["data"]);
}

#[test]
fn workflow_cli_exposes_body_free_graph_modes_and_rerun() {
    let workspace = TestDirectory::new("workflow-workspace");
    let input = TestDirectory::new("workflow-input");
    fs::create_dir_all(input.path()).expect("input directory");
    let advert = input.path().join("advert.md");
    fs::write(&advert, "Private workflow sentinel\n").expect("advert");
    run_json(&[
        "--workspace",
        workspace.text(),
        "workspace",
        "init",
        "--json",
    ]);
    let job = run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "create",
        "--title",
        "Lecturer",
        "--institution",
        "University X",
        "--json",
    ]);
    let job_id = job["data"]["id"].as_str().expect("job ID");
    run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "import",
        job_id,
        "--file",
        advert.to_str().expect("advert path"),
        "--json",
    ]);
    let started = run_json(&[
        "--workspace",
        workspace.text(),
        "workflow",
        "start",
        "--job",
        job_id,
        "--json",
    ]);
    assert_eq!(started["data"]["stages"].as_array().map(Vec::len), Some(10));
    assert_eq!(started["data"]["stages"][0]["stage"], "intake");
    assert_eq!(started["data"]["stages"][0]["status"], "complete");
    assert!(
        !serde_json::to_string(&started)
            .expect("workflow JSON")
            .contains("Private workflow sentinel")
    );
    let wrong_mode = run(&[
        "--workspace",
        workspace.text(),
        "workflow",
        "begin",
        "--job",
        job_id,
        "--stage",
        "parse",
        "--mode",
        "deterministic",
        "--json",
    ]);
    assert_eq!(wrong_mode.status.code(), Some(4));
    let wrong_mode: Value = serde_json::from_slice(&wrong_mode.stdout).expect("mode error JSON");
    assert_eq!(wrong_mode["error"]["code"], "workflow.conflict");
    let running = run_json(&[
        "--workspace",
        workspace.text(),
        "workflow",
        "begin",
        "--job",
        job_id,
        "--stage",
        "parse",
        "--mode",
        "host-agent",
        "--json",
    ]);
    assert_eq!(running["data"]["stages"][1]["status"], "running");
    assert_eq!(running["data"]["stages"][1]["execution_mode"], "host-agent");
    let rerun = run_json(&[
        "--workspace",
        workspace.text(),
        "workflow",
        "rerun",
        "--job",
        job_id,
        "--stage",
        "parse",
        "--json",
    ]);
    assert_eq!(rerun["data"]["stages"][1]["status"], "ready");
    let shown = run_json(&[
        "--workspace",
        workspace.text(),
        "workflow",
        "status",
        "--job",
        job_id,
        "--json",
    ]);
    assert_eq!(shown["data"]["run_id"], started["data"]["run_id"]);
}

#[test]
fn discovery_csv_dry_run_commit_and_promotion_are_agent_callable() {
    let workspace = TestDirectory::new("discovery-workspace");
    let input = TestDirectory::new("discovery-input");
    fs::create_dir_all(input.path()).expect("input directory");
    let batch = input.path().join("leads.csv");
    fs::write(
        &batch,
        b"external_id,title,organization,url,deadline\n1,Lecturer in Economics,University X,https://example.edu/jobs/1,2099-08-31\n2,Bad URL,University X,ftp://example.edu/jobs/2,2099-08-31\n",
    )
    .expect("write discovery batch");
    let batch_path = batch.to_str().expect("batch path is UTF-8");

    let adapters = run_json(&["discovery", "adapters", "--json"]);
    assert_eq!(
        adapters["data"]["adapters"].as_array().map(Vec::len),
        Some(4)
    );

    let dry_run = run_json(&[
        "discovery",
        "import",
        "--file",
        batch_path,
        "--source-name",
        "University export",
        "--dry-run",
        "--json",
    ]);
    assert_eq!(dry_run["status"], "validated");
    assert_eq!(dry_run["data"]["accepted"], 1);
    assert_eq!(dry_run["data"]["rejected"], 1);
    assert_eq!(dry_run["data"]["receipt"], Value::Null);

    run_json(&[
        "--workspace",
        workspace.text(),
        "workspace",
        "init",
        "--json",
    ]);
    let imported = run_json(&[
        "--workspace",
        workspace.text(),
        "discovery",
        "import",
        "--file",
        batch_path,
        "--source-name",
        "University export",
        "--json",
    ]);
    assert_eq!(imported["status"], "imported");
    assert_eq!(imported["data"]["receipt"]["inserted"], 1);
    assert_eq!(imported["data"]["receipt"]["rejected"], 1);

    let listed = run_json(&[
        "--workspace",
        workspace.text(),
        "discovery",
        "list",
        "--json",
    ]);
    let lead_id = listed["data"]["leads"][0]["id"].as_str().expect("lead ID");
    let promoted = run_json(&[
        "--workspace",
        workspace.text(),
        "discovery",
        "promote",
        lead_id,
        "--json",
    ]);
    assert_eq!(promoted["status"], "promoted");
    assert_eq!(promoted["data"]["job"]["title"], "Lecturer in Economics");
    assert_eq!(promoted["next_actions"].as_array().map(Vec::len), Some(1));
    let history = run_json(&[
        "--workspace",
        workspace.text(),
        "discovery",
        "list",
        "--include-history",
        "--json",
    ]);
    assert_eq!(history["data"]["leads"][0]["status"], "promoted");
}

#[test]
fn leased_task_completion_is_validated_atomic_and_idempotent() {
    let workspace = TestDirectory::new("task-workspace");
    let input = TestDirectory::new("task-input");
    fs::create_dir_all(input.path()).expect("input directory");
    let advert = input.path().join("advert.md");
    fs::write(&advert, "Teach economics\n").expect("advert");
    run_json(&[
        "--workspace",
        workspace.text(),
        "workspace",
        "init",
        "--json",
    ]);
    let job = run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "create",
        "--title",
        "Lecturer in Economics",
        "--institution",
        "University X",
        "--json",
    ]);
    let job_id = job["data"]["id"].as_str().expect("job ID");
    run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "import",
        job_id,
        "--file",
        advert.to_str().expect("advert path"),
        "--json",
    ]);
    let prepared = run_json(&[
        "--workspace",
        workspace.text(),
        "task",
        "prepare",
        "--job",
        job_id,
        "--operation",
        "job-parse",
        "--json",
    ]);
    assert_eq!(prepared["status"], "prepared");
    assert_eq!(
        prepared["data"]["input_artifacts"].as_array().map(Vec::len),
        Some(1)
    );
    assert!(
        !serde_json::to_string(&prepared)
            .expect("prepared response")
            .contains("Teach economics")
    );
    let descriptor = &prepared["data"];
    let task_id = descriptor["id"].as_str().expect("task ID");
    let export_directory = input.path().join("task-work");
    let consent_required = run(&[
        "--workspace",
        workspace.text(),
        "task",
        "inputs",
        task_id,
        "--destination",
        export_directory.to_str().expect("export path"),
        "--json",
    ]);
    assert_eq!(consent_required.status.code(), Some(3));
    let consent_required: Value =
        serde_json::from_slice(&consent_required.stdout).expect("consent failure JSON");
    assert_eq!(consent_required["error"]["code"], "consent.required");
    let exported = run_json(&[
        "--workspace",
        workspace.text(),
        "task",
        "inputs",
        task_id,
        "--destination",
        export_directory.to_str().expect("export path"),
        "--allow-private-read",
        "--json",
    ]);
    assert_eq!(exported["status"], "exported");
    assert_eq!(exported["data"]["files"].as_array().map(Vec::len), Some(1));
    let relative = exported["data"]["files"][0]["relative_path"]
        .as_str()
        .expect("relative input path");
    assert_eq!(
        fs::read_to_string(export_directory.join(relative)).expect("scoped source"),
        "Teach economics\n"
    );
    let expected_inputs = descriptor["input_artifacts"]
        .as_array()
        .expect("inputs")
        .iter()
        .map(|input| {
            serde_json::json!({
                "artifact_id": input["id"],
                "revision": input["revision"],
                "sha256": input["sha256"]
            })
        })
        .collect::<Vec<_>>();
    let candidate = serde_json::json!({
        "id": "019f2f55-7c00-7000-8000-000000000201",
        "job_id": job_id,
        "title": "Lecturer in Economics",
        "institution": "University X",
        "summary": "Teach economics",
        "responsibilities": ["Teach economics"],
        "criteria": [{
            "id": "019f2f55-7c00-7000-8000-000000000202",
            "job_id": job_id,
            "kind": "teaching",
            "requirement": "Evidence of university-level teaching",
            "importance": "essential",
            "source_quote": "Teach economics",
            "source_span": {
                "source": descriptor["input_artifacts"][0],
                "start_byte": 0,
                "end_byte": 15
            },
            "confidence_milli": 950,
            "confirmed": false,
            "revision": 1
        }],
        "revision": 1
    });
    let request = serde_json::json!({
        "task_id": descriptor["id"],
        "lease_id": descriptor["lease"]["id"],
        "expected_job_revision": descriptor["job_revision"],
        "expected_inputs": expected_inputs,
        "candidate": candidate
    });
    let invalid_path = input.path().join("invalid.json");
    let mut invalid = request.clone();
    invalid["candidate"]["criteria"][0]["requirement"] = Value::String(" ".to_owned());
    fs::write(
        &invalid_path,
        serde_json::to_vec(&invalid).expect("invalid request"),
    )
    .expect("invalid file");
    let failure = run(&[
        "--workspace",
        workspace.text(),
        "task",
        "complete",
        "--file",
        invalid_path.to_str().expect("invalid path"),
        "--json",
    ]);
    assert_eq!(failure.status.code(), Some(3));
    let failure: Value = serde_json::from_slice(&failure.stdout).expect("failure JSON");
    assert_eq!(failure["error"]["code"], "candidate.semantic_invalid");
    assert!(
        failure["error"]["details"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let bytes = serde_json::to_vec(&request).expect("completion request");
    let committed = run_json_stdin(
        &[
            "--workspace",
            workspace.text(),
            "task",
            "complete",
            "--stdin",
            "--json",
        ],
        &bytes,
    );
    assert_eq!(committed["status"], "committed");
    assert_eq!(committed["data"]["idempotent"], false);
    let request_path = input.path().join("completion.json");
    fs::write(&request_path, bytes).expect("completion file");
    let replay = run_json(&[
        "--workspace",
        workspace.text(),
        "task",
        "complete",
        "--file",
        request_path.to_str().expect("request path"),
        "--json",
    ]);
    assert_eq!(replay["data"]["idempotent"], true);
    assert_eq!(replay["data"]["artifact"], committed["data"]["artifact"]);

    let criteria_path = input.path().join("criteria.json");
    let exported_criteria = run_json(&[
        "--workspace",
        workspace.text(),
        "criteria",
        "export",
        "--job",
        job_id,
        "--destination",
        criteria_path.to_str().expect("criteria path"),
        "--json",
    ]);
    assert_eq!(exported_criteria["data"]["criterion_count"], 1);
    let confirmed_criteria = run_json(&[
        "--workspace",
        workspace.text(),
        "criteria",
        "confirm",
        "--job",
        job_id,
        "--file",
        criteria_path.to_str().expect("criteria path"),
        "--json",
    ]);
    assert_eq!(confirmed_criteria["status"], "confirmed");
    assert_eq!(confirmed_criteria["data"]["criteria"][0]["confirmed"], true);
    let shown_criteria = run_json(&[
        "--workspace",
        workspace.text(),
        "criteria",
        "show",
        "--job",
        job_id,
        "--json",
    ]);
    assert_eq!(shown_criteria["data"], confirmed_criteria["data"]);

    run_json(&[
        "--workspace",
        workspace.text(),
        "workflow",
        "rerun",
        "--job",
        job_id,
        "--stage",
        "parse",
        "--json",
    ]);
    let prepared_stale = run_json(&[
        "--workspace",
        workspace.text(),
        "task",
        "prepare",
        "--job",
        job_id,
        "--operation",
        "job-parse",
        "--json",
    ]);
    let stale_descriptor = &prepared_stale["data"];
    let stale_inputs = stale_descriptor["input_artifacts"]
        .as_array()
        .expect("stale inputs")
        .iter()
        .map(|input| {
            serde_json::json!({
                "artifact_id": input["id"],
                "revision": input["revision"],
                "sha256": input["sha256"]
            })
        })
        .collect::<Vec<_>>();
    let stale_request = serde_json::json!({
        "task_id": stale_descriptor["id"],
        "lease_id": stale_descriptor["lease"]["id"],
        "expected_job_revision": stale_descriptor["job_revision"],
        "expected_inputs": stale_inputs,
        "candidate": candidate
    });
    run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "import",
        job_id,
        "--file",
        advert.to_str().expect("advert path"),
        "--json",
    ]);
    let stale_path = input.path().join("stale.json");
    fs::write(
        &stale_path,
        serde_json::to_vec(&stale_request).expect("stale request"),
    )
    .expect("stale file");
    let stale = run(&[
        "--workspace",
        workspace.text(),
        "task",
        "complete",
        "--file",
        stale_path.to_str().expect("stale path"),
        "--json",
    ]);
    assert_eq!(stale.status.code(), Some(4));
    let stale: Value = serde_json::from_slice(&stale.stdout).expect("stale JSON");
    assert_eq!(stale["error"]["code"], "task.stale");
    assert!(stale["error"]["remediation"].is_object());
}

#[test]
fn evidence_workflow_is_agent_callable_and_user_confirmed_through_the_binary() {
    let workspace = TestDirectory::new("evidence-cli-workspace");
    let input = TestDirectory::new("evidence-cli-input");
    fs::create_dir_all(input.path()).expect("input directory");
    let advert = input.path().join("advert.txt");
    let profile = input.path().join("profile.txt");
    fs::write(&advert, "Economics lecturer\n").expect("advert");
    fs::write(&profile, "PhD in Economics\n").expect("profile");
    run_json(&[
        "--workspace",
        workspace.text(),
        "workspace",
        "init",
        "--json",
    ]);
    let job = run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "create",
        "--title",
        "Lecturer in Economics",
        "--institution",
        "University X",
        "--json",
    ]);
    let job_id = job["data"]["id"].as_str().expect("job ID");
    run_json(&[
        "--workspace",
        workspace.text(),
        "job",
        "import",
        job_id,
        "--file",
        advert.to_str().expect("advert path"),
        "--json",
    ]);
    run_json(&[
        "--workspace",
        workspace.text(),
        "profile",
        "source",
        "add",
        "--file",
        profile.to_str().expect("profile path"),
        "--json",
    ]);
    let prepared = run_json(&[
        "--workspace",
        workspace.text(),
        "task",
        "prepare",
        "--job",
        job_id,
        "--operation",
        "evidence-normalize",
        "--json",
    ]);
    let descriptor = &prepared["data"];
    assert_eq!(descriptor["operation"], "profile.evidence.normalize");
    assert_eq!(
        descriptor["candidate_schema"]["id"],
        "canisend.evidence-proposals/v2"
    );
    let expected_inputs = descriptor["input_artifacts"]
        .as_array()
        .expect("profile inputs")
        .iter()
        .map(|input| {
            serde_json::json!({
                "artifact_id": input["id"],
                "revision": input["revision"],
                "sha256": input["sha256"]
            })
        })
        .collect::<Vec<_>>();
    let completion = serde_json::json!({
        "task_id": descriptor["id"],
        "lease_id": descriptor["lease"]["id"],
        "expected_job_revision": descriptor["job_revision"],
        "expected_inputs": expected_inputs,
        "candidate": {
            "profile_revision": descriptor["profile_revision"],
            "proposals": [{
                "kind": "qualification",
                "summary": "Doctorate in economics",
                "source_quote": "PhD in Economics",
                "source_span": {
                    "source": descriptor["input_artifacts"][0],
                    "start_byte": 0,
                    "end_byte": 16
                },
                "sensitivity": "private-local"
            }]
        }
    });
    let completion_path = input.path().join("evidence-completion.json");
    fs::write(
        &completion_path,
        serde_json::to_vec(&completion).expect("completion JSON"),
    )
    .expect("completion file");
    let committed = run_json(&[
        "--workspace",
        workspace.text(),
        "task",
        "complete",
        "--file",
        completion_path.to_str().expect("completion path"),
        "--json",
    ]);
    assert_eq!(committed["data"]["artifact"]["kind"], "evidence-catalog");
    let proposed = run_json(&[
        "--workspace",
        workspace.text(),
        "profile",
        "evidence",
        "proposed",
        "--job",
        job_id,
        "--json",
    ]);
    assert_eq!(proposed["data"]["items"][0]["confirmed"], false);
    let generated_id = proposed["data"]["items"][0]["id"].clone();

    let decision_path = input.path().join("evidence-decision.json");
    let exported = run_json(&[
        "--workspace",
        workspace.text(),
        "profile",
        "evidence",
        "export",
        "--job",
        job_id,
        "--destination",
        decision_path.to_str().expect("decision path"),
        "--json",
    ]);
    assert_eq!(exported["data"]["evidence_count"], 1);
    let mut decision: Value =
        serde_json::from_slice(&fs::read(&decision_path).expect("decision bytes"))
            .expect("decision JSON");
    decision["items"][0]["summary"] = Value::String("Reviewed doctorate".to_owned());
    decision["items"][0]["excluded"] = Value::Bool(true);
    fs::write(
        &decision_path,
        serde_json::to_vec(&decision).expect("revised decision JSON"),
    )
    .expect("revised decision file");
    let confirmed = run_json(&[
        "--workspace",
        workspace.text(),
        "profile",
        "evidence",
        "confirm",
        "--job",
        job_id,
        "--file",
        decision_path.to_str().expect("decision path"),
        "--json",
    ]);
    assert_eq!(confirmed["status"], "confirmed");
    assert_eq!(confirmed["data"]["items"][0]["id"], generated_id);
    assert_eq!(confirmed["data"]["items"][0]["excluded"], true);
    let shown = run_json(&[
        "--workspace",
        workspace.text(),
        "profile",
        "evidence",
        "show",
        "--job",
        job_id,
        "--json",
    ]);
    assert_eq!(shown["data"], confirmed["data"]);
}

fn write_pdf(path: &std::path::Path, text: Option<&str>) {
    use lopdf::{
        Document, Object, Stream,
        content::{Content, Operation},
        dictionary,
    };

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding"
    });
    let resources_id = document.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });
    let content = Content {
        operations: text.map_or_else(Vec::new, |text| {
            vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Td", vec![50.into(), 750.into()]),
                Operation::new("Tj", vec![Object::string_literal(text)]),
                Operation::new("ET", vec![]),
            ]
        }),
    };
    let content_id = document.add_object(Stream::new(
        dictionary! {},
        content.encode().expect("PDF content"),
    ));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);
    document.save(path).expect("write text PDF");
}
