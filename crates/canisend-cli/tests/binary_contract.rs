use std::process::Command;

use serde_json::Value;

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
            .any(|item| { item["id"] == "workspace.lifecycle" && item["status"] == "planned" })
    );
}

#[test]
fn public_catalogs_are_available_without_a_workspace() {
    let schemas = run_json(&["schema", "list", "--json"]);
    let resources = run_json(&["resource", "list", "--json"]);

    assert_eq!(
        schemas["data"]["schemas"].as_array().map(Vec::len),
        Some(15)
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
