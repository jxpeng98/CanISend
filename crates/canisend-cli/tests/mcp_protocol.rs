#![forbid(unsafe_code)]

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use canisend_app::{
    Application, ApplicationFlowCreateRequestV3, ApplicationFlowCreateRequestV4,
    ApplicationFlowRequirementDraftV3, CANISEND_MCP_GUARDED_WRITE_TOOLS,
    CANISEND_MCP_PROTOCOL_VERSION, CANISEND_MCP_READ_ONLY_TOOLS, CANISEND_MCP_TOOLS,
    GENERIC_APPLICATION_WORKFLOW_PACK_ID, PrivateReadConsent,
};
use canisend_contracts::{
    PrivacyClassification, RequirementPriorityV3, WorkflowPackId, WorkflowPackItemId,
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
fn negotiates_current_protocol_and_lists_only_clean_v4_tools() {
    let root = temporary_root("list");
    Application::initialize_workspace_v4(&root).expect("initialize Workspace v4");
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
            .contains("clean Workspace v4 state")
    );

    let listed = mcp.request(2, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("tool array");
    let names = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    assert_eq!(names, CANISEND_MCP_TOOLS);
    assert_eq!(CANISEND_MCP_READ_ONLY_TOOLS.len(), 9);
    assert_eq!(CANISEND_MCP_GUARDED_WRITE_TOOLS.len(), 2);
    for tool in tools {
        let name = tool["name"].as_str().expect("tool name");
        let read_only = CANISEND_MCP_READ_ONLY_TOOLS.contains(&name);
        let guarded_write = CANISEND_MCP_GUARDED_WRITE_TOOLS.contains(&name);
        assert_ne!(read_only, guarded_write, "tool class must be exact: {name}");
        assert_eq!(tool["annotations"]["readOnlyHint"], json!(read_only));
        assert_eq!(tool["annotations"]["destructiveHint"], json!(guarded_write));
        assert_eq!(
            tool["annotations"]["idempotentHint"],
            json!(!name.ends_with("_preview") && !name.ends_with("_commit"))
        );
        assert_eq!(tool["annotations"]["openWorldHint"], json!(false));
    }

    drop(mcp);
    fs::remove_dir_all(root).expect("remove workspace");
}

#[test]
fn serves_v4_reads_guarded_association_writes_and_refuses_legacy_tools() {
    let root = temporary_root("calls");
    let profile_source = temporary_root("calls-profile-source").with_extension("md");
    let private_sentinel = "PRIVATE-MCP-PROFILE-BODY-MUST-NOT-LEAK";
    fs::write(
        &profile_source,
        format!("# Profile\n\n{private_sentinel}\n"),
    )
    .expect("write Profile Source fixture");
    Application::initialize_workspace_v4(&root).expect("initialize Workspace v4");
    let application_id = Application::create_application_flow_v4(
        &root,
        ApplicationFlowCreateRequestV4 {
            pack_id: WorkflowPackId::try_new(GENERIC_APPLICATION_WORKFLOW_PACK_ID)
                .expect("Pack ID"),
            application: ApplicationFlowCreateRequestV3 {
                title: "MCP association fixture".to_owned(),
                opportunity_metadata: Default::default(),
                application_metadata: Default::default(),
                source_text: "Provide a narrative.".to_owned(),
                requirements: vec![ApplicationFlowRequirementDraftV3 {
                    category: WorkflowPackItemId::try_new("format").expect("category"),
                    statement: "Provide a narrative.".to_owned(),
                    priority: RequirementPriorityV3::Mandatory,
                    start_byte: 0,
                    end_byte: 20,
                }],
            },
        },
    )
    .expect("create Application fixture")
    .data
    .stored
    .snapshot
    .application
    .id;
    let imported_source = Application::import_profile_source_v4(
        &root,
        &profile_source,
        PrivacyClassification::PrivateLocal,
        Some(PrivateReadConsent::granted_by_user()),
    )
    .expect("import Profile Source fixture")
    .data
    .source;
    let mut mcp = McpProcess::start(&root);
    mcp.initialize();

    let listed = mcp.request(
        2,
        "tools/call",
        json!({"name": "canisend_application_list", "arguments": {}}),
    );
    assert_eq!(listed["result"]["isError"], json!(false));
    assert_eq!(
        listed["result"]["structuredContent"]["operation"],
        json!("application.list")
    );

    let status = mcp.request(
        3,
        "tools/call",
        json!({"name": "canisend_workspace_status", "arguments": {}}),
    );
    assert_eq!(status["result"]["isError"], json!(false));
    assert_eq!(
        status["result"]["structuredContent"]["data"]["status"]["workspace_format"],
        json!(canisend_contracts::WORKSPACE_V4_FORMAT)
    );

    let check = mcp.request(
        4,
        "tools/call",
        json!({"name": "canisend_workspace_check", "arguments": {}}),
    );
    assert_eq!(check["result"]["isError"], json!(false));
    assert_eq!(
        check["result"]["structuredContent"]["data"]["check"]["ok"],
        json!(true)
    );

    let profile_sources = mcp.request(
        5,
        "tools/call",
        json!({"name": "canisend_profile_source_list", "arguments": {}}),
    );
    assert_eq!(profile_sources["result"]["isError"], json!(false));
    assert_eq!(
        profile_sources["result"]["structuredContent"]["operation"],
        json!("profile-source.list")
    );
    assert_eq!(
        profile_sources["result"]["structuredContent"]["data"]["sources"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert!(!profile_sources.to_string().contains(private_sentinel));

    let profile_links = mcp.request(
        6,
        "tools/call",
        json!({
            "name": "canisend_profile_association_list",
            "arguments": {"application_id": application_id.as_str()}
        }),
    );
    assert_eq!(profile_links["result"]["isError"], json!(false));
    assert_eq!(
        profile_links["result"]["structuredContent"]["operation"],
        json!("profile.association.list")
    );
    assert!(!profile_links.to_string().contains(private_sentinel));

    let evidence_links = mcp.request(
        7,
        "tools/call",
        json!({
            "name": "canisend_evidence_association_list",
            "arguments": {"application_id": application_id.as_str()}
        }),
    );
    assert_eq!(evidence_links["result"]["isError"], json!(false));
    assert_eq!(
        evidence_links["result"]["structuredContent"]["operation"],
        json!("evidence.association.list")
    );

    let profile_reference = json!({
        "id": imported_source.id,
        "revision": imported_source.revision,
        "sha256": imported_source.original.sha256
    });
    let denied_preview = mcp.request(
        8,
        "tools/call",
        json!({
            "name": "canisend_profile_association_preview",
            "arguments": {
                "application_id": application_id.as_str(),
                "profile_source": profile_reference,
                "change": "associate"
            }
        }),
    );
    assert_eq!(denied_preview["result"]["isError"], json!(false));
    assert!(!denied_preview.to_string().contains(private_sentinel));
    let denied_token = denied_preview["result"]["structuredContent"]["preview_token"]
        .as_str()
        .expect("preview token");
    assert_eq!(
        denied_preview["result"]["structuredContent"]["preview"]["operation"],
        json!("profile.association.preview")
    );
    let denied_digest =
        denied_preview["result"]["structuredContent"]["preview"]["data"]["preview_sha256"]
            .as_str()
            .expect("preview digest");
    let denied = mcp.request(
        9,
        "tools/call",
        json!({
            "name": "canisend_profile_association_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": denied_token,
                "preview_sha256": denied_digest,
                "approved": false,
                "confirmed_private_read": false
            }
        }),
    );
    assert!(denied["error"].is_object() || denied["result"]["isError"] == json!(true));
    let denied_replay = mcp.request(
        10,
        "tools/call",
        json!({
            "name": "canisend_profile_association_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": denied_token,
                "preview_sha256": denied_digest,
                "approved": true,
                "confirmed_private_read": true
            }
        }),
    );
    assert!(
        denied_replay["error"].is_object() || denied_replay["result"]["isError"] == json!(true)
    );

    let approved_preview = mcp.request(
        11,
        "tools/call",
        json!({
            "name": "canisend_profile_association_preview",
            "arguments": {
                "application_id": application_id.as_str(),
                "profile_source": profile_reference,
                "change": "associate"
            }
        }),
    );
    let approved_token = approved_preview["result"]["structuredContent"]["preview_token"]
        .as_str()
        .expect("approved token");
    let approved_digest =
        approved_preview["result"]["structuredContent"]["preview"]["data"]["preview_sha256"]
            .as_str()
            .expect("approved digest");
    let committed = mcp.request(
        12,
        "tools/call",
        json!({
            "name": "canisend_profile_association_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": approved_token,
                "preview_sha256": approved_digest,
                "approved": true,
                "confirmed_private_read": true
            }
        }),
    );
    assert_eq!(committed["result"]["isError"], json!(false));
    assert_eq!(
        committed["result"]["structuredContent"]["operation"],
        json!("profile.association.commit")
    );
    assert!(!committed.to_string().contains(private_sentinel));
    let replay = mcp.request(
        13,
        "tools/call",
        json!({
            "name": "canisend_profile_association_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": approved_token,
                "preview_sha256": approved_digest,
                "approved": true,
                "confirmed_private_read": true
            }
        }),
    );
    assert!(replay["error"].is_object() || replay["result"]["isError"] == json!(true));
    let linked = mcp.request(
        14,
        "tools/call",
        json!({
            "name": "canisend_profile_association_list",
            "arguments": {"application_id": application_id.as_str()}
        }),
    );
    assert_eq!(
        linked["result"]["structuredContent"]["data"]["associations"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert!(!linked.to_string().contains(private_sentinel));

    for (id, legacy) in [
        (15, "canisend_agent_v3_context"),
        (16, "canisend_application_create"),
        (17, "canisend_job_intake_commit"),
    ] {
        let refused = mcp.request(id, "tools/call", json!({"name": legacy, "arguments": {}}));
        assert!(
            refused["error"].is_object() || refused["result"]["isError"] == json!(true),
            "legacy MCP tool must be unavailable: {refused}"
        );
    }

    drop(mcp);
    fs::remove_dir_all(root).expect("remove workspace");
    fs::remove_file(profile_source).expect("remove Profile Source fixture");
}

#[test]
fn rejects_malformed_v4_arguments_without_mutation() {
    let root = temporary_root("malformed");
    Application::initialize_workspace_v4(&root).expect("initialize Workspace v4");
    let before = Application::workspace_status_v4(&root)
        .expect("workspace before")
        .data
        .status;
    let mut mcp = McpProcess::start(&root);
    mcp.initialize();

    let malformed = mcp.request(
        2,
        "tools/call",
        json!({
            "name": "canisend_application_show",
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
    let after = Application::workspace_status_v4(&root)
        .expect("workspace after")
        .data
        .status;
    assert_eq!(before, after);
    fs::remove_dir_all(root).expect("remove workspace");
}
