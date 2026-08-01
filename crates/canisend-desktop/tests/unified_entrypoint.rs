#![forbid(unsafe_code)]
#![cfg(target_os = "macos")]

use std::process::Command;

use serde_json::Value;

fn run(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_canisend-gui"))
        .args(arguments)
        .output()
        .expect("unified CanISend executable runs")
}

#[test]
fn unified_executable_dispatches_the_public_cli_contract_without_opening_the_gui() {
    let output = run(&["version", "--json"]);

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
    let value: Value = serde_json::from_slice(&output.stdout).expect("stdout is JSON");
    assert_eq!(value["protocol"], "canisend.agent/v2");
    assert_eq!(value["operation"], "product.version");
    assert_eq!(value["data"]["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn unified_executable_exposes_the_mcp_command_tree() {
    let output = run(&["mcp", "--help"]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(stdout.contains("Model Context Protocol"));
    assert!(stdout.contains("serve"));
}
