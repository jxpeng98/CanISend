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
        Some(2)
    );
}

#[test]
fn public_catalogs_are_available_without_a_workspace() {
    let schemas = run_json(&["schema", "list", "--json"]);
    let resources = run_json(&["resource", "list", "--json"]);

    assert_eq!(
        schemas["data"]["schemas"].as_array().map(Vec::len),
        Some(20)
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
        "job-criterion",
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
        "kind": "teaching",
        "requirement": "Evidence of university-level teaching",
        "importance": "essential",
        "source_quote": "Teach economics",
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
    invalid["candidate"]["requirement"] = Value::String(" ".to_owned());
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
