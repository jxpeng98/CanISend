#![forbid(unsafe_code)]

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use canisend_app::{
    Application, CANISEND_MCP_GUARDED_WRITE_TOOLS, CANISEND_MCP_PROTOCOL_VERSION,
    CANISEND_MCP_READ_ONLY_TOOLS, CANISEND_MCP_TOOLS, PrivateReadConsent,
};
use serde_json::{Value, json};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn temporary_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "canisend-mcp-protocol-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

struct McpProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl McpProcess {
    fn start(workspace: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_canisend"))
            .args([
                "--workspace",
                workspace.to_str().expect("UTF-8 fixture path"),
                "mcp",
                "serve",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("start MCP server");
        let stdin = child.stdin.take().expect("MCP stdin");
        let stdout = BufReader::new(child.stdout.take().expect("MCP stdout"));
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn send(&mut self, message: &Value) {
        serde_json::to_writer(&mut self.stdin, message).expect("write MCP request");
        self.stdin.write_all(b"\n").expect("terminate MCP request");
        self.stdin.flush().expect("flush MCP request");
    }

    fn send_raw(&mut self, message: &[u8]) {
        self.stdin
            .write_all(message)
            .expect("write raw MCP request");
        self.stdin
            .write_all(b"\n")
            .expect("terminate raw MCP request");
        self.stdin.flush().expect("flush raw MCP request");
    }

    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }));
        loop {
            let mut line = String::new();
            let bytes = self.stdout.read_line(&mut line).expect("read MCP response");
            assert_ne!(bytes, 0, "MCP server closed before response {id}");
            let response: Value = serde_json::from_str(&line).expect("valid JSON-RPC response");
            if response["id"] == json!(id) {
                return response;
            }
        }
    }

    fn initialize(&mut self) -> Value {
        let response = self.request(
            1,
            "initialize",
            json!({
                "protocolVersion": CANISEND_MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "canisend-protocol-test",
                    "version": "1.0"
                }
            }),
        );
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));
        response
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn negotiates_current_protocol_and_lists_guarded_tools_deterministically() {
    let root = temporary_root("list");
    Application::initialize_workspace(&root).expect("initialize workspace");
    let mut mcp = McpProcess::start(&root);

    mcp.send_raw(b"{malformed-json");
    let initialized = mcp.initialize();
    assert_eq!(
        initialized["result"]["protocolVersion"],
        json!(CANISEND_MCP_PROTOCOL_VERSION)
    );
    assert_eq!(
        initialized["result"]["serverInfo"]["version"],
        json!(env!("CARGO_PKG_VERSION"))
    );
    assert!(
        initialized["result"]["instructions"]
            .as_str()
            .expect("instructions")
            .contains("deprecated compatibility surface")
    );

    let listed = mcp.request(2, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("tool array");
    let names = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    assert_eq!(names, CANISEND_MCP_TOOLS);
    for tool in tools {
        let name = tool["name"].as_str().expect("tool name");
        let read_only = CANISEND_MCP_READ_ONLY_TOOLS.contains(&name);
        assert_eq!(CANISEND_MCP_GUARDED_WRITE_TOOLS.contains(&name), !read_only);
        let idempotent = matches!(
            name,
            "canisend_capabilities"
                | "canisend_context"
                | "canisend_job_detail"
                | "canisend_jobs_list"
                | "canisend_profile_sources"
                | "canisend_task_latest"
                | "canisend_workflow_status"
        );
        assert_eq!(tool["annotations"]["readOnlyHint"], json!(read_only));
        assert_eq!(tool["annotations"]["destructiveHint"], json!(false));
        assert_eq!(tool["annotations"]["idempotentHint"], json!(idempotent));
        assert_eq!(
            tool["annotations"]["openWorldHint"],
            json!(name == "canisend_job_intake_preview")
        );
    }

    drop(mcp);
    fs::remove_dir_all(root).expect("remove workspace");
}

#[test]
fn returns_structured_facade_results_and_rejects_malformed_arguments() {
    let root = temporary_root("calls");
    Application::initialize_workspace(&root).expect("initialize workspace");
    Application::create_job(&root, "Lecturer", "University X").expect("create job");
    let before = Application::workspace_status(&root)
        .expect("workspace before")
        .data
        .status;
    let mut mcp = McpProcess::start(&root);
    mcp.initialize();

    let listed = mcp.request(
        2,
        "tools/call",
        json!({
            "name": "canisend_jobs_list",
            "arguments": {}
        }),
    );
    assert_eq!(
        listed["result"]["structuredContent"]["operation"],
        json!("job.list")
    );
    assert_eq!(
        listed["result"]["structuredContent"]["data"]["jobs"]
            .as_array()
            .expect("jobs")
            .len(),
        1
    );
    assert_eq!(listed["result"]["isError"], json!(false));

    let malformed = mcp.request(
        3,
        "tools/call",
        json!({
            "name": "canisend_context",
            "arguments": {"unexpected": true}
        }),
    );
    assert_eq!(malformed["result"]["isError"], json!(true));
    assert!(
        malformed["result"]["content"][0]["text"]
            .as_str()
            .expect("error text")
            .contains("unknown field")
    );

    drop(mcp);
    let after = Application::workspace_status(&root)
        .expect("workspace after")
        .data
        .status;
    assert_eq!(before, after);
    fs::remove_dir_all(root).expect("remove workspace");
}

#[test]
fn previews_and_commits_exact_job_intake_with_a_single_use_token() {
    let root = temporary_root("job-intake");
    let source = temporary_root("private-advert").with_extension("txt");
    let sentinel = "MCP-JOB-INTAKE-PRIVATE-SENTINEL";
    fs::write(&source, format!("Lecturer advert\n{sentinel}\n")).expect("write source");
    Application::initialize_workspace(&root).expect("initialize workspace");
    let job = Application::create_job(&root, "Lecturer", "University X")
        .expect("create job")
        .data;
    let mut mcp = McpProcess::start(&root);
    mcp.initialize();

    let previewed = mcp.request(
        2,
        "tools/call",
        json!({
            "name": "canisend_job_intake_preview",
            "arguments": {
                "job_id": job.id,
                "source_kind": "local-file",
                "locator": source,
                "confirmed_private_read": true
            }
        }),
    );
    assert_eq!(previewed["result"]["isError"], json!(false));
    let structured = &previewed["result"]["structuredContent"];
    let encoded = serde_json::to_string(structured).expect("serialize preview");
    assert!(!encoded.contains(sentinel));
    let token = structured["preview_token"]
        .as_str()
        .expect("preview token")
        .to_owned();
    assert!(
        Application::job_detail(&root, job.id.as_str())
            .expect("job before commit")
            .data
            .sources
            .is_empty()
    );

    let committed = mcp.request(
        3,
        "tools/call",
        json!({
            "name": "canisend_job_intake_commit",
            "arguments": {"preview_token": token}
        }),
    );
    assert_eq!(committed["result"]["isError"], json!(false));
    assert_eq!(
        committed["result"]["structuredContent"]["operation"],
        json!("job.intake.commit")
    );
    assert_eq!(
        Application::job_detail(&root, job.id.as_str())
            .expect("job after commit")
            .data
            .sources
            .len(),
        1
    );

    let reused = mcp.request(
        4,
        "tools/call",
        json!({
            "name": "canisend_job_intake_commit",
            "arguments": {"preview_token": token}
        }),
    );
    assert!(
        reused["error"].is_object() || reused["result"]["isError"] == json!(true),
        "single-use token must be rejected: {reused}"
    );

    drop(mcp);
    fs::remove_dir_all(root).expect("remove workspace");
    fs::remove_file(source).expect("remove source");
}

#[test]
fn prepares_and_exports_versioned_task_inputs_through_the_same_adapter() {
    let root = temporary_root("task");
    let source = temporary_root("task-advert").with_extension("txt");
    let destination = temporary_root("task-inputs");
    fs::write(&source, "Lecturer advert").expect("write source");
    Application::initialize_workspace(&root).expect("initialize workspace");
    let job = Application::create_job(&root, "Lecturer", "University X")
        .expect("create job")
        .data;
    Application::import_local_job_source(
        &root,
        job.id.as_str(),
        &source,
        PrivateReadConsent::granted_by_user(),
    )
    .expect("import source");
    let mut mcp = McpProcess::start(&root);
    mcp.initialize();

    let prepared = mcp.request(
        2,
        "tools/call",
        json!({
            "name": "canisend_task_prepare",
            "arguments": {
                "job_id": job.id,
                "operation": "job-parse",
                "mode": "host-agent"
            }
        }),
    );
    assert_eq!(prepared["result"]["isError"], json!(false));
    assert_eq!(
        prepared["result"]["structuredContent"]["operation"],
        json!("task.prepare")
    );
    let task_id = prepared["result"]["structuredContent"]["data"]["id"]
        .as_str()
        .expect("task ID")
        .to_owned();

    let latest = mcp.request(
        3,
        "tools/call",
        json!({
            "name": "canisend_task_latest",
            "arguments": {"job_id": job.id}
        }),
    );
    assert_eq!(
        latest["result"]["structuredContent"]["data"]["descriptor"]["id"],
        json!(task_id)
    );

    let exported = mcp.request(
        4,
        "tools/call",
        json!({
            "name": "canisend_task_inputs",
            "arguments": {
                "task_id": task_id,
                "destination": destination,
                "confirmed_private_read": true
            }
        }),
    );
    assert_eq!(exported["result"]["isError"], json!(false));
    assert_eq!(
        exported["result"]["structuredContent"]["operation"],
        json!("task.inputs")
    );
    assert!(destination.join("canisend-task-inputs.json").is_file());

    let unapproved_completion = mcp.request(
        5,
        "tools/call",
        json!({
            "name": "canisend_task_completion_preview",
            "arguments": {
                "file": "/tmp/private-completion.json",
                "confirmed_private_read": false
            }
        }),
    );
    assert!(
        unapproved_completion["error"].is_object()
            || unapproved_completion["result"]["isError"] == json!(true),
        "missing consent must be rejected: {unapproved_completion}"
    );

    drop(mcp);
    fs::remove_dir_all(root).expect("remove workspace");
    fs::remove_dir_all(destination).expect("remove task inputs");
    fs::remove_file(source).expect("remove source");
}
