#![cfg(feature = "app-server-test-fixture")]
#![forbid(unsafe_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use canisend_gui::agent_runtime::exercise_app_server_fixture;
use serde_json::Value;

fn temporary_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "canisend-app-server-fixture-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ))
}

fn run_fixture(
    root: &Path,
    scenario: &str,
    existing_session_id: Option<&str>,
    turns: usize,
    cancel: bool,
) -> Value {
    let session_directory = root.join(scenario);
    fs::create_dir_all(&session_directory).expect("create fixture session directory");
    let result = exercise_app_server_fixture(
        Path::new(env!("CARGO_BIN_EXE_canisend-fake-app-server")),
        session_directory.clone(),
        existing_session_id,
        turns,
        cancel,
        Duration::from_millis(150),
    );
    assert!(
        !session_directory.exists(),
        "the exact child session directory must be cleaned up"
    );
    result
}

#[test]
#[ignore = "requires a signed-in Codex CLI and provider access"]
fn signed_in_codex_smoke_starts_resumes_restarts_and_cancels() {
    let executable = std::env::var("CANISEND_CODEX_SMOKE_BINARY")
        .expect("set CANISEND_CODEX_SMOKE_BINARY to the signed-in Codex executable");
    let root = temporary_root();
    fs::create_dir_all(&root).expect("create smoke root");
    let run = |scenario: &str, existing: Option<&str>, cancel: bool| {
        let session_directory = root.join(scenario);
        fs::create_dir_all(&session_directory).expect("create smoke session directory");
        exercise_app_server_fixture(
            Path::new(&executable),
            session_directory,
            existing,
            1,
            cancel,
            Duration::from_secs(10),
        )
    };

    let started = run("smoke-start", None, false);
    assert_eq!(
        started["ok"],
        true,
        "{}",
        started["error_message"]
            .as_str()
            .unwrap_or("unknown failure")
    );
    let thread_id = started["external_session_id"]
        .as_str()
        .expect("started thread ID");
    assert_eq!(run("smoke-resume", Some(thread_id), false)["ok"], true);
    assert_eq!(run("smoke-restart", None, false)["ok"], true);
    assert_eq!(
        run("smoke-cancel", None, true)["error_code"],
        "agent-runtime-cancelled"
    );

    fs::remove_dir_all(root).expect("remove smoke root");
}

#[test]
fn bounded_app_server_lifecycle_is_streamed_resumable_and_fail_closed() {
    let root = temporary_root();
    fs::create_dir_all(&root).expect("create fixture root");

    let normal = run_fixture(&root, "normal", None, 2, false);
    assert_eq!(normal["ok"], true);
    assert_eq!(normal["resumed"], false);
    assert_eq!(normal["external_session_id"], "thread-new");
    assert_eq!(
        normal["responses"],
        serde_json::json!(["Hello world", "Hello world"])
    );
    let sequences = normal["events"]
        .as_array()
        .expect("streamed events")
        .iter()
        .filter_map(|event| event["sequence"].as_u64())
        .collect::<Vec<_>>();
    assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(
        normal["events"]
            .as_array()
            .expect("events")
            .iter()
            .any(|event| event["kind"] == "assistant-delta" && event["text"] == "Hello")
    );

    let resumed = run_fixture(&root, "resume", Some("thread-saved"), 1, false);
    assert_eq!(resumed["ok"], true);
    assert_eq!(resumed["resumed"], true);
    assert_eq!(resumed["external_session_id"], "thread-saved");

    let approval = run_fixture(&root, "approval", None, 1, false);
    assert_eq!(approval["ok"], true);
    assert!(
        approval["events"]
            .as_array()
            .expect("events")
            .iter()
            .any(|event| {
                event["kind"] == "server-request"
                    && event["method"] == "item/commandExecution/requestApproval"
            })
    );
    assert!(!approval.to_string().contains("must-not-cross-boundary"));

    let cancelled = run_fixture(&root, "cancel", None, 1, true);
    assert_eq!(cancelled["error_code"], "agent-runtime-cancelled");
    assert!(
        cancelled["events"]
            .as_array()
            .expect("events")
            .iter()
            .any(|event| event["kind"] == "interrupted" && event["state"] == "ready")
    );

    let unsupported = run_fixture(&root, "unsupported-request", None, 1, false);
    assert_eq!(unsupported["error_code"], "agent-runtime-incompatible");
    assert!(!unsupported.to_string().contains("must-not-cross-boundary"));

    for scenario in ["malformed", "oversized", "timeout"] {
        let failed = run_fixture(&root, scenario, None, 1, false);
        assert_eq!(failed["ok"], false, "scenario {scenario}");
    }
    let child_exit = run_fixture(&root, "child-exit", None, 1, false);
    assert!(!child_exit.to_string().contains("PRIVATE_FIXTURE_BODY"));
    assert!(
        !child_exit
            .to_string()
            .contains(root.to_string_lossy().as_ref())
    );

    let authentication = run_fixture(&root, "authentication-required", None, 0, false);
    assert_eq!(
        authentication["error_code"],
        "agent-runtime-authentication-required"
    );

    fs::remove_dir_all(root).expect("remove fixture root");
}
