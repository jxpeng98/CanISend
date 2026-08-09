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

fn mutation_preview_binding(response: &Value) -> (String, String) {
    assert_eq!(response["result"]["isError"], json!(false));
    let structured = &response["result"]["structuredContent"];
    (
        structured["preview_token"]
            .as_str()
            .expect("mutation preview token")
            .to_owned(),
        structured["preview"]["data"]["preview_sha256"]
            .as_str()
            .expect("mutation preview digest")
            .to_owned(),
    )
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
    assert_eq!(CANISEND_MCP_READ_ONLY_TOOLS.len(), 26);
    assert_eq!(CANISEND_MCP_GUARDED_WRITE_TOOLS.len(), 10);
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
fn completes_the_guarded_requirement_plan_and_deliverable_lifecycle() {
    let root = temporary_root("application-lifecycle");
    Application::initialize_workspace_v4(&root).expect("initialize Workspace v4");
    let created = Application::create_application_flow_v4(
        &root,
        ApplicationFlowCreateRequestV4 {
            pack_id: WorkflowPackId::try_new(GENERIC_APPLICATION_WORKFLOW_PACK_ID)
                .expect("Pack ID"),
            application: ApplicationFlowCreateRequestV3 {
                title: "MCP lifecycle fixture".to_owned(),
                opportunity_metadata: Default::default(),
                application_metadata: Default::default(),
                source_text: "Provide a reviewed primary document.".to_owned(),
                requirements: vec![ApplicationFlowRequirementDraftV3 {
                    category: WorkflowPackItemId::try_new("format").expect("category"),
                    statement: "Provide a reviewed primary document.".to_owned(),
                    priority: RequirementPriorityV3::Mandatory,
                    start_byte: 0,
                    end_byte: 36,
                }],
            },
        },
    )
    .expect("create Application fixture")
    .data
    .stored;
    let source = created.snapshot.requirements[0].source_span.content.clone();
    let application_id = created.snapshot.application.id;
    let requirement_id = created.snapshot.requirements[0].id.clone();
    let mut mcp = McpProcess::start(&root);
    mcp.initialize();

    let extraction_preview = mcp.request(
        2,
        "tools/call",
        json!({
            "name": "canisend_requirement_extract_preview",
            "arguments": {
                "application_id": application_id.as_str(),
                "expected_revision": 1,
                "source": source,
                "requirements": [{
                    "category": "format",
                    "statement": "reviewed primary document",
                    "priority": "recommended",
                    "start_byte": 10,
                    "end_byte": 35
                }],
                "confirmed_private_read": false
            }
        }),
    );
    assert_eq!(
        extraction_preview["result"]["structuredContent"]["preview"]["operation"],
        json!("requirement.extract.preview")
    );
    let (token, digest) = mutation_preview_binding(&extraction_preview);
    let extracted = mcp.request(
        3,
        "tools/call",
        json!({
            "name": "canisend_requirement_extract_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": token,
                "preview_sha256": digest,
                "approved": true,
                "confirmed_private_read": false
            }
        }),
    );
    let extracted_requirement_id =
        extracted["result"]["structuredContent"]["data"]["snapshot"]["requirements"][1]["id"]
            .as_str()
            .expect("extracted Requirement ID")
            .to_owned();
    let requirement_preview = mcp.request(
        4,
        "tools/call",
        json!({
            "name": "canisend_requirement_confirm_preview",
            "arguments": {
                "application_id": application_id.as_str(),
                "expected_revision": 2,
                "decisions": [
                    {
                        "requirement_id": requirement_id.as_str(),
                        "decision": "confirm"
                    },
                    {
                        "requirement_id": extracted_requirement_id,
                        "decision": "confirm"
                    }
                ]
            }
        }),
    );
    assert_eq!(
        requirement_preview["result"]["structuredContent"]["preview"]["operation"],
        json!("requirement.confirm.preview")
    );
    let (token, digest) = mutation_preview_binding(&requirement_preview);
    let requirements = mcp.request(
        5,
        "tools/call",
        json!({
            "name": "canisend_requirement_confirm_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": token,
                "preview_sha256": digest,
                "approved": true
            }
        }),
    );
    assert_eq!(
        requirements["result"]["structuredContent"]["operation"],
        json!("requirement.confirm.commit")
    );
    assert_eq!(
        requirements["result"]["structuredContent"]["data"]["snapshot"]["application"]["revision"],
        json!(3)
    );

    let plan_preview = mcp.request(
        6,
        "tools/call",
        json!({
            "name": "canisend_plan_propose_preview",
            "arguments": {
                "application_id": application_id.as_str(),
                "expected_revision": 3,
                "decision": "proceed",
                "deliverables": [{
                    "kind": "primary-document",
                    "disposition": "required",
                    "rationale": "Required by the reviewed source",
                    "constraints": [],
                    "execution_mode": "host-agent"
                }]
            }
        }),
    );
    let (token, digest) = mutation_preview_binding(&plan_preview);
    let proposed = mcp.request(
        7,
        "tools/call",
        json!({
            "name": "canisend_plan_propose_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": token,
                "preview_sha256": digest,
                "approved": true
            }
        }),
    );
    assert_eq!(
        proposed["result"]["structuredContent"]["data"]["snapshot"]["plan"]["state"],
        json!("draft")
    );

    let confirmation_preview = mcp.request(
        8,
        "tools/call",
        json!({
            "name": "canisend_plan_confirm_preview",
            "arguments": {
                "application_id": application_id.as_str(),
                "expected_revision": 4
            }
        }),
    );
    let (token, digest) = mutation_preview_binding(&confirmation_preview);
    let confirmed = mcp.request(
        9,
        "tools/call",
        json!({
            "name": "canisend_plan_confirm_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": token,
                "preview_sha256": digest,
                "approved": true
            }
        }),
    );
    assert_eq!(
        confirmed["result"]["structuredContent"]["data"]["snapshot"]["plan"]["state"],
        json!("confirmed")
    );

    let draft_body = "PRIVATE-MCP-DELIVERABLE-V1";
    let draft_preview = mcp.request(
        10,
        "tools/call",
        json!({
            "name": "canisend_deliverable_draft_preview",
            "arguments": {
                "application_id": application_id.as_str(),
                "expected_revision": 5,
                "deliverables": [{
                    "kind": "primary-document",
                    "title": "Reviewed primary document",
                    "media_type": "text/markdown",
                    "content": draft_body
                }]
            }
        }),
    );
    let (draft_token, draft_digest) = mutation_preview_binding(&draft_preview);
    let drafted = mcp.request(
        11,
        "tools/call",
        json!({
            "name": "canisend_deliverable_draft_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": draft_token,
                "preview_sha256": draft_digest,
                "approved": true
            }
        }),
    );
    let deliverable_id = drafted["result"]["structuredContent"]["data"]["snapshot"]["deliverables"]
        [0]["id"]
        .as_str()
        .expect("Deliverable ID")
        .to_owned();

    let refused_audit = mcp.request(
        12,
        "tools/call",
        json!({
            "name": "canisend_deliverable_audit",
            "arguments": {
                "application_id": application_id.as_str(),
                "confirmed_private_read": false
            }
        }),
    );
    assert!(
        refused_audit["error"].is_object() || refused_audit["result"]["isError"] == json!(true)
    );
    assert!(!refused_audit.to_string().contains(draft_body));
    let audit = mcp.request(
        13,
        "tools/call",
        json!({
            "name": "canisend_deliverable_audit",
            "arguments": {
                "application_id": application_id.as_str(),
                "confirmed_private_read": true
            }
        }),
    );
    assert_eq!(
        audit["result"]["structuredContent"]["operation"],
        json!("deliverable.audit")
    );
    assert!(audit.to_string().contains(draft_body));

    let revised_body = "PRIVATE-MCP-DELIVERABLE-V2";
    let revision_preview = mcp.request(
        14,
        "tools/call",
        json!({
            "name": "canisend_deliverable_revise_preview",
            "arguments": {
                "application_id": application_id.as_str(),
                "expected_revision": 6,
                "deliverable_id": deliverable_id,
                "title": "Revised primary document",
                "media_type": "text/markdown",
                "content": revised_body
            }
        }),
    );
    let (token, digest) = mutation_preview_binding(&revision_preview);
    let revised = mcp.request(
        15,
        "tools/call",
        json!({
            "name": "canisend_deliverable_revise_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": token,
                "preview_sha256": digest,
                "approved": true
            }
        }),
    );
    assert_eq!(
        revised["result"]["structuredContent"]["data"]["snapshot"]["application"]["revision"],
        json!(7)
    );
    let replay = mcp.request(
        16,
        "tools/call",
        json!({
            "name": "canisend_deliverable_revise_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": token,
                "preview_sha256": digest,
                "approved": true
            }
        }),
    );
    assert!(replay["error"].is_object() || replay["result"]["isError"] == json!(true));
    let revised_audit = mcp.request(
        17,
        "tools/call",
        json!({
            "name": "canisend_deliverable_audit",
            "arguments": {
                "application_id": application_id.as_str(),
                "confirmed_private_read": true
            }
        }),
    );
    assert!(revised_audit.to_string().contains(revised_body));
    assert!(!revised_audit.to_string().contains(draft_body));

    let refused_review = mcp.request(
        18,
        "tools/call",
        json!({
            "name": "canisend_review_inspect",
            "arguments": {
                "application_id": application_id.as_str(),
                "confirmed_private_read": false
            }
        }),
    );
    assert!(
        refused_review["error"].is_object() || refused_review["result"]["isError"] == json!(true)
    );
    assert!(!refused_review.to_string().contains(revised_body));
    let review = mcp.request(
        19,
        "tools/call",
        json!({
            "name": "canisend_review_inspect",
            "arguments": {
                "application_id": application_id.as_str(),
                "confirmed_private_read": true
            }
        }),
    );
    assert_eq!(
        review["result"]["structuredContent"]["operation"],
        json!("review.inspect")
    );
    assert!(review.to_string().contains(revised_body));

    let disposition_preview = mcp.request(
        20,
        "tools/call",
        json!({
            "name": "canisend_review_disposition_preview",
            "arguments": {
                "application_id": application_id.as_str(),
                "expected_revision": 7,
                "confirmed_private_read": true
            }
        }),
    );
    let (token, digest) = mutation_preview_binding(&disposition_preview);
    let disposition = mcp.request(
        21,
        "tools/call",
        json!({
            "name": "canisend_review_disposition_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": token,
                "preview_sha256": digest,
                "approved": true,
                "confirmed_private_read": true
            }
        }),
    );
    assert_eq!(
        disposition["result"]["structuredContent"]["data"]["snapshot"]["application"]["revision"],
        json!(8)
    );

    let destination = format!("applications/{application_id}/exports/mcp-lifecycle");
    let export_preview = mcp.request(
        22,
        "tools/call",
        json!({
            "name": "canisend_export_prepare_preview",
            "arguments": {
                "application_id": application_id.as_str(),
                "expected_revision": 8,
                "destination": destination,
                "confirmed_private_export": true
            }
        }),
    );
    let (export_token, export_digest) = mutation_preview_binding(&export_preview);
    let exported = mcp.request(
        23,
        "tools/call",
        json!({
            "name": "canisend_export_prepare_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": export_token,
                "preview_sha256": export_digest,
                "approved": true,
                "confirmed_private_export": true
            }
        }),
    );
    assert_eq!(
        exported["result"]["structuredContent"]["operation"],
        json!("export.prepare.commit")
    );
    assert_eq!(
        exported["result"]["structuredContent"]["data"]["render"]["submission_performed"],
        json!(false)
    );
    let exports = mcp.request(
        24,
        "tools/call",
        json!({
            "name": "canisend_export_list",
            "arguments": {"application_id": application_id.as_str()}
        }),
    );
    assert_eq!(
        exports["result"]["structuredContent"]["data"]["exports"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    let shown = mcp.request(
        25,
        "tools/call",
        json!({
            "name": "canisend_export_show",
            "arguments": {
                "application_id": application_id.as_str(),
                "destination": destination
            }
        }),
    );
    assert_eq!(
        shown["result"]["structuredContent"]["operation"],
        json!("export.show")
    );
    let export_replay = mcp.request(
        26,
        "tools/call",
        json!({
            "name": "canisend_export_prepare_commit",
            "arguments": {
                "application_id": application_id.as_str(),
                "preview_token": export_token,
                "preview_sha256": export_digest,
                "approved": true,
                "confirmed_private_export": true
            }
        }),
    );
    assert!(
        export_replay["error"].is_object() || export_replay["result"]["isError"] == json!(true)
    );

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
    let created = Application::create_application_flow_v4(
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
    .stored;
    let application_id = created.snapshot.application.id;
    let requirement_id = created.snapshot.requirements[0].id.clone();
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

    for (id, name, arguments, operation) in [
        (
            20,
            "canisend_requirement_list",
            json!({"application_id": application_id.as_str()}),
            "requirement.list",
        ),
        (
            21,
            "canisend_requirement_show",
            json!({
                "application_id": application_id.as_str(),
                "requirement_id": requirement_id.as_str()
            }),
            "requirement.show",
        ),
        (
            22,
            "canisend_plan_show",
            json!({"application_id": application_id.as_str()}),
            "plan.show",
        ),
        (
            23,
            "canisend_deliverable_list",
            json!({"application_id": application_id.as_str()}),
            "deliverable.list",
        ),
    ] {
        let response = mcp.request(
            id,
            "tools/call",
            json!({"name": name, "arguments": arguments}),
        );
        assert_eq!(response["result"]["isError"], json!(false));
        assert_eq!(
            response["result"]["structuredContent"]["operation"],
            operation
        );
    }

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
