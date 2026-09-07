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
    confirmation: Option<Value>,
    confirmations: Vec<Value>,
}

impl McpProcess {
    fn start(workspace: &Path) -> Self {
        Self::start_with_application(workspace, None)
    }

    fn start_with_application(workspace: &Path, application: Option<&str>) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_canisend"));
        command.args([
            "--workspace",
            workspace.to_str().expect("UTF-8 fixture path"),
            "mcp",
            "serve",
        ]);
        if let Some(application) = application {
            command.args(["--application", application]);
        }
        let mut child = command
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
            // Synthetic protocol peer only; this is not real Host/user acceptance evidence.
            confirmation: Some(json!({"action": "accept", "content": {"confirm": true}})),
            confirmations: Vec::new(),
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
            if response["method"] == "elicitation/create" {
                self.confirmations.push(response["params"].clone());
                self.send(&json!({
                    "jsonrpc": "2.0", "id": response["id"],
                    "result": self.confirmation.as_ref().expect("peer advertised confirmation"),
                }));
                continue;
            }
            if response["id"] == json!(id) {
                return response;
            }
        }
    }

    /// Stop at a real server request so a test can kill the process before replying or
    /// accept synthetically without consuming the business receipt from stdout.
    fn pending_confirmation(&mut self, id: u64, params: Value) -> Value {
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params}));
        let mut line = String::new();
        assert_ne!(self.stdout.read_line(&mut line).unwrap(), 0);
        let response: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["method"], "elicitation/create", "{response}");
        self.confirmations.push(response["params"].clone());
        response["id"].clone()
    }

    fn initialize(&mut self) -> Value {
        let response = self.request(
            1,
            "initialize",
            json!({
                "protocolVersion": CANISEND_MCP_PROTOCOL_VERSION,
                "capabilities": if self.confirmation.is_some() { json!({"elicitation": {"form": {}}}) } else { json!({}) },
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
    assert_eq!(CANISEND_MCP_READ_ONLY_TOOLS.len(), 29);
    assert_eq!(CANISEND_MCP_GUARDED_WRITE_TOOLS.len(), 11);
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
        let properties = &tool["inputSchema"]["properties"];
        assert!(properties.get("approved").is_none());
        assert!(properties.get("confirmed_private_read").is_none());
        assert!(properties.get("confirmed_private_export").is_none());
        if guarded_write {
            assert_eq!(properties["request_confirmation"]["type"], "boolean");
        }
    }

    drop(mcp);
    fs::remove_dir_all(root).expect("remove workspace");
}

#[test]
fn completes_the_guarded_requirement_plan_and_deliverable_lifecycle() {
    guarded_lifecycle(false);
}

#[test]
fn local_task_candidate_uses_native_draft_confirmation() {
    guarded_lifecycle(true);
}

fn guarded_lifecycle(local_candidate: bool) {
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
    for response in [
        None,
        Some(json!({"action": "decline", "content": null})),
        Some(json!({"action": "cancel", "content": null})),
        Some(json!({"action": "accept", "content": {"confirm": false}})),
        Some(json!({"action": "accept", "content": {"confirm": "true"}})),
        Some(json!({"action": "accept", "content": {"confirm": true, "unexpected": true}})),
    ] {
        let mut peer = McpProcess::start(&root);
        peer.confirmation = response;
        peer.initialize();
        let preview = peer.request(2, "tools/call", json!({
            "name": "canisend_requirement_confirm_preview",
            "arguments": {"application_id": application_id.as_str(), "expected_revision": 1,
                "decisions": [{"requirement_id": requirement_id.as_str(), "decision": "confirm"}]}
        }));
        let (token, digest) = mutation_preview_binding(&preview);
        let request = json!({"name": "canisend_requirement_confirm_commit", "arguments": {
            "application_id": application_id.as_str(), "preview_token": token,
            "preview_sha256": digest, "request_confirmation": true
        }});
        let refused = peer.request(3, "tools/call", request.clone());
        assert_eq!(
            refused["error"]["data"]["code"],
            "consent.host-confirmation-required"
        );
        if let Some(confirmation) = peer.confirmations.first() {
            let message = confirmation["message"].as_str().unwrap();
            assert!(message.contains(&digest));
            assert!(message.contains(requirement_id.as_str()));
            assert!(!message.contains(&token));
            assert_eq!(
                confirmation["requestedSchema"]["properties"]["confirm"]["default"],
                false
            );
        }
        let replay = peer.request(4, "tools/call", request.clone());
        assert!(replay["error"].is_object() || replay["result"]["isError"] == true);
        assert_eq!(
            Application::application_model_v4(&root, application_id.as_str())
                .unwrap()
                .data
                .snapshot
                .application
                .revision
                .get(),
            1
        );
        drop(peer);
        let mut restarted = McpProcess::start(&root);
        restarted.initialize();
        let replay = restarted.request(2, "tools/call", request);
        assert!(replay["error"].is_object() || replay["result"]["isError"] == true);
        assert!(restarted.confirmations.is_empty());
    }
    let mut mcp = McpProcess::start(&root);
    mcp.initialize();

    // An old model-supplied approval assertion cannot authorize the new interface.
    let legacy = mcp.request(
        90,
        "tools/call",
        json!({
            "name": "canisend_requirement_confirm_commit", "arguments": {
                "application_id": application_id.as_str(), "preview_token": "unissued",
                "preview_sha256": "0".repeat(64), "approved": true
            }
        }),
    );
    assert!(legacy["error"].is_object() || legacy["result"]["isError"] == true);
    assert!(mcp.confirmations.is_empty());

    let catalog = mcp.request(
        91,
        "tools/call",
        json!({
            "name": "canisend_application_pack_show", "arguments": {
                "application_id": application_id.as_str()
            }
        }),
    );
    assert_eq!(
        catalog["result"]["structuredContent"]["operation"],
        "application.pack.show"
    );
    assert_eq!(
        catalog["result"]["structuredContent"]["data"]["id"],
        GENERIC_APPLICATION_WORKFLOW_PACK_ID
    );

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
                "request_private_read": false
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
                "request_confirmation": true,
                "request_private_read": false
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
                "request_confirmation": true
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
                "request_confirmation": true
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
                "request_confirmation": true
            }
        }),
    );
    assert_eq!(
        confirmed["result"]["structuredContent"]["data"]["snapshot"]["plan"]["state"],
        json!("confirmed")
    );

    let cli_output = Command::new(env!("CARGO_BIN_EXE_canisend"))
        .args([
            "--workspace",
            root.to_str().expect("UTF-8 fixture path"),
            "plan",
            "show",
            "--application",
            application_id.as_str(),
            "--json",
        ])
        .output()
        .expect("read the MCP-confirmed Plan through the CLI");
    assert!(
        cli_output.status.success(),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&cli_output.stderr),
        String::from_utf8_lossy(&cli_output.stdout)
    );
    assert!(cli_output.stderr.is_empty());
    let cli_plan: Value =
        serde_json::from_slice(&cli_output.stdout).expect("CLI Plan response is JSON");
    assert_eq!(cli_plan["operation"], json!("plan.show"));
    assert_eq!(
        cli_plan["data"]["context"]["application_id"],
        json!(application_id.as_str())
    );
    assert_eq!(cli_plan["data"]["plan"]["state"], json!("confirmed"));

    let draft_body = "PRIVATE-MCP-DELIVERABLE-V1";
    let submitted =
        if local_candidate {
            let task = Application::prepare_local_task_v4(
                &root,
                application_id.as_str(),
                canisend_contracts::Revision::try_new(5).unwrap(),
            )
            .unwrap()
            .data;
            let claimed =
                Application::claim_local_task_v4(&root, task.id.as_str(), task.generation)
                    .unwrap()
                    .data;
            let candidate_path = root.join("worker-candidate.json");
            fs::write(&candidate_path, serde_json::to_vec(&json!({
            "expected_revision": 5,
            "deliverables": [{"kind": "primary-document", "title": "Reviewed primary document",
                "media_type": "text/markdown", "content": draft_body}]
        })).unwrap()).unwrap();
            let submitted = Application::submit_local_task_v4(
                &root,
                claimed.id.as_str(),
                claimed.generation,
                claimed.lease_id.as_ref().unwrap().as_str(),
                &candidate_path,
            )
            .unwrap()
            .data;
            // A new reviewer process recovers the persisted candidate, not another process's approval.
            drop(mcp);
            mcp = McpProcess::start(&root);
            mcp.initialize();
            Some(submitted)
        } else {
            None
        };
    let draft_preview = if let Some(task) = &submitted {
        let mut arguments = json!({"application_id": application_id.as_str(), "task_id": task.id,
            "expected_generation": task.generation, "candidate_sha256": task.candidate_sha256,
            "request_private_read": false});
        let refused = mcp.request(
            100,
            "tools/call",
            json!({"name": "canisend_local_task_draft_preview", "arguments": arguments}),
        );
        assert!(refused["error"].is_object() || refused["result"]["isError"] == true);
        assert!(!refused.to_string().contains(draft_body));
        arguments["request_private_read"] = json!(true);
        let preview = mcp.request(
            101,
            "tools/call",
            json!({"name": "canisend_local_task_draft_preview", "arguments": arguments}),
        );
        let form = mcp.confirmations.last().unwrap().to_string();
        assert!(form.contains(task.id.as_str()));
        assert!(form.contains(task.candidate_sha256.as_ref().unwrap().as_str()));
        let (token, digest) = mutation_preview_binding(&preview);
        mcp.confirmation = Some(json!({"action": "accept", "content": {"confirm": false}}));
        let commit_args = json!({"application_id": application_id.as_str(), "preview_token": token,
            "preview_sha256": digest, "request_confirmation": true});
        let denied = mcp.request(
            102,
            "tools/call",
            json!({"name": "canisend_deliverable_draft_commit", "arguments": commit_args}),
        );
        assert!(denied["error"].is_object() || denied["result"]["isError"] == true);
        assert_eq!(
            Application::show_local_task_v4(&root, task.id.as_str())
                .unwrap()
                .data,
            *task
        );
        assert_eq!(
            Application::application_model_v4(&root, application_id.as_str())
                .unwrap()
                .data
                .snapshot
                .application
                .revision
                .get(),
            5
        );
        mcp.confirmation = Some(json!({"action": "accept", "content": {"confirm": true}}));
        let replay = mcp.request(
            103,
            "tools/call",
            json!({"name": "canisend_deliverable_draft_commit", "arguments": commit_args}),
        );
        assert!(replay["error"].is_object() || replay["result"]["isError"] == true);
        let preview = mcp.request(
            104,
            "tools/call",
            json!({"name": "canisend_local_task_draft_preview", "arguments": arguments}),
        );
        let (token, digest) = mutation_preview_binding(&preview);
        let commit = json!({"name": "canisend_deliverable_draft_commit", "arguments": {
            "application_id": application_id.as_str(), "preview_token": token,
            "preview_sha256": digest, "request_confirmation": true}});
        mcp.pending_confirmation(105, commit.clone());
        // Kill while the native form is unanswered: the durable candidate must survive,
        // and the dead process's grant must never authorize a restarted reviewer.
        mcp.child
            .kill()
            .expect("kill reviewer with pending consent");
        assert!(!mcp.child.wait().unwrap().success());
        drop(mcp);
        assert_eq!(
            Application::show_local_task_v4(&root, task.id.as_str())
                .unwrap()
                .data,
            *task
        );
        mcp = McpProcess::start(&root);
        mcp.initialize();
        let replay = mcp.request(106, "tools/call", commit);
        assert!(replay["error"].is_object() || replay["result"]["isError"] == true);
        assert!(mcp.confirmations.is_empty());
        mcp.request(
            107,
            "tools/call",
            json!({"name": "canisend_local_task_draft_preview", "arguments": arguments}),
        )
    } else {
        mcp.request(
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
        )
    };
    let (draft_token, draft_digest) = mutation_preview_binding(&draft_preview);
    let commit = json!({
        "name": "canisend_deliverable_draft_commit",
        "arguments": {"application_id": application_id.as_str(), "preview_token": draft_token,
            "preview_sha256": draft_digest, "request_confirmation": true}
    });
    let drafted = if let Some(task) = &submitted {
        let confirmation_id = mcp.pending_confirmation(11, commit.clone());
        // Isolated synthetic peer only. Deliberately do not read the commit receipt.
        mcp.send(&json!({"jsonrpc": "2.0", "id": confirmation_id,
            "result": {"action": "accept", "content": {"confirm": true}}}));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let state = Application::show_local_task_v4(&root, task.id.as_str())
                .unwrap()
                .data;
            if state.state == canisend_contracts::LocalTaskStateV4::Committed {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "commit did not become durable"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let form = mcp.confirmations.last().unwrap().to_string();
        assert!(form.contains(task.id.as_str()));
        assert!(form.contains(task.candidate_sha256.as_ref().unwrap().as_str()));
        assert!(form.contains(draft_body));
        // A lost receipt is resolved by canonical state, never a duplicate commit.
        mcp.child
            .kill()
            .expect("kill reviewer without reading receipt");
        assert!(!mcp.child.wait().unwrap().success());
        drop(mcp);
        mcp = McpProcess::start(&root);
        mcp.initialize();
        let replay = mcp.request(108, "tools/call", commit);
        assert!(replay["error"].is_object() || replay["result"]["isError"] == true);
        assert!(mcp.confirmations.is_empty());
        mcp.request(
            109,
            "tools/call",
            json!({"name": "canisend_application_show",
            "arguments": {"application_id": application_id.as_str()}}),
        )
    } else {
        mcp.request(11, "tools/call", commit)
    };
    if let Some(task) = &submitted {
        let recovered = Application::show_local_task_v4(&root, task.id.as_str())
            .unwrap()
            .data;
        assert_eq!(
            recovered.state,
            canisend_contracts::LocalTaskStateV4::Committed
        );
        let binding = recovered.committed_application.unwrap();
        assert_eq!(binding.expected_revision.get(), 6);
        assert_eq!(
            json!(binding.snapshot_sha256),
            drafted["result"]["structuredContent"]["data"]["snapshot_sha256"]
        );
    }
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
                "request_private_read": false
            }
        }),
    );
    assert!(
        refused_audit["error"].is_object() || refused_audit["result"]["isError"] == json!(true)
    );
    assert!(!refused_audit.to_string().contains(draft_body));
    mcp.confirmation = Some(json!({"action": "decline", "content": null}));
    let refused = mcp.request(
        13,
        "tools/call",
        json!({
            "name": "canisend_deliverable_audit",
            "arguments": {"application_id": application_id.as_str(), "request_private_read": true}
        }),
    );
    assert_eq!(
        refused["error"]["data"]["code"],
        "consent.host-confirmation-required"
    );
    assert!(!refused.to_string().contains(draft_body));
    assert!(
        !mcp.confirmations
            .last()
            .unwrap()
            .to_string()
            .contains(draft_body)
    );
    mcp.confirmation = Some(json!({"action": "accept", "content": {"confirm": true}}));
    let audit = mcp.request(
        13,
        "tools/call",
        json!({
            "name": "canisend_deliverable_audit",
            "arguments": {
                "application_id": application_id.as_str(),
                "request_private_read": true
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
                "request_confirmation": true
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
                "request_confirmation": true
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
                "request_private_read": true
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
                "request_private_read": false
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
                "request_private_read": true
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
                "request_private_read": true
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
                "request_confirmation": true,
                "request_private_read": true
            }
        }),
    );
    assert_eq!(
        disposition["result"]["structuredContent"]["data"]["snapshot"]["application"]["revision"],
        json!(8)
    );

    let destination = format!("applications/{application_id}/exports/mcp-lifecycle");
    mcp.confirmation = Some(json!({"action": "decline", "content": null}));
    let refused = mcp.request(
        22,
        "tools/call",
        json!({
            "name": "canisend_export_prepare_preview", "arguments": {
                "application_id": application_id.as_str(), "expected_revision": 8,
                "destination": "exports/refused", "request_private_export": true
            }
        }),
    );
    assert_eq!(
        refused["error"]["data"]["code"],
        "consent.host-confirmation-required"
    );
    assert!(!root.join("exports/refused").exists());
    mcp.confirmation = Some(json!({"action": "accept", "content": {"confirm": true}}));
    let export_preview = mcp.request(
        22,
        "tools/call",
        json!({
            "name": "canisend_export_prepare_preview",
            "arguments": {
                "application_id": application_id.as_str(),
                "expected_revision": 8,
                "destination": destination,
                "request_private_export": true
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
                "request_confirmation": true,
                "request_private_export": true
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
                "request_confirmation": true,
                "request_private_export": true
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
                "request_confirmation": false,
                "request_private_read": false
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
                "request_confirmation": true,
                "request_private_read": true
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
                "request_confirmation": true,
                "request_private_read": true
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
                "request_confirmation": true,
                "request_private_read": true
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

    let evidence_request = json!({
        "name": "canisend_evidence_confirm_preview", "arguments": {
            "application_id": application_id.as_str(),
            "profile_source": {"id": imported_source.id, "revision": imported_source.revision,
                "sha256": imported_source.original.sha256},
            "proposals": {"profile_revision": 1, "proposals": [{
                "kind": "employment", "summary": "Synthetic source-backed fact",
                "source_quote": private_sentinel,
                "source_span": {"source": imported_source.normalized_text,
                    "start_byte": 11, "end_byte": 11 + private_sentinel.len()},
                "sensitivity": "private-local"
            }]}, "request_private_read": true
        }
    });
    for accepted in [false, true] {
        mcp.confirmation = Some(json!({"action": "accept", "content": {"confirm": true}}));
        let preview = mcp.request(60, "tools/call", evidence_request.clone());
        let (token, digest) = mutation_preview_binding(&preview);
        mcp.confirmation = Some(json!({"action": "accept", "content": {"confirm": accepted}}));
        let commit_request = json!({"name": "canisend_evidence_confirm_commit", "arguments": {
            "application_id": application_id.as_str(), "preview_token": token,
            "preview_sha256": digest, "request_confirmation": true, "request_private_read": true
        }});
        let form_count = mcp.confirmations.len();
        let result = mcp.request(61, "tools/call", commit_request.clone());
        if accepted {
            assert_eq!(
                result["result"]["structuredContent"]["operation"],
                "evidence.confirm.commit"
            );
            assert_eq!(
                mcp.confirmations.len() - form_count,
                2,
                "private read and mutation use separate forms"
            );
        } else {
            assert_eq!(
                result["error"]["data"]["code"],
                "consent.host-confirmation-required"
            );
            assert!(!result.to_string().contains(private_sentinel));
        }
        let replay = mcp.request(62, "tools/call", commit_request);
        assert!(replay["error"].is_object() || replay["result"]["isError"] == true);
        let inventory = mcp.request(63, "tools/call", json!({
            "name": "canisend_evidence_association_list", "arguments": {"application_id": application_id.as_str()}
        }));
        let data = &inventory["result"]["structuredContent"]["data"];
        assert_eq!(
            data["evidence"].as_array().unwrap().len(),
            usize::from(accepted)
        );
        assert_eq!(
            data["associations"],
            json!([]),
            "confirmation does not associate Evidence"
        );
        assert!(!inventory.to_string().contains(private_sentinel));
    }

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

#[test]
fn application_binding_covers_every_tool_and_preserves_unbound_discovery() {
    let root = temporary_root("application-binding");
    Application::initialize_workspace_v4(&root).expect("initialize Workspace");
    let mut applications = Vec::new();
    for (pack, category) in [
        (GENERIC_APPLICATION_WORKFLOW_PACK_ID, "format"),
        (canisend_app::ACADEMIC_JOB_WORKFLOW_PACK_ID, "qualification"),
    ] {
        let source_text = "Synthetic bound Application requirement.";
        let created = Application::create_application_flow_v4(
            &root,
            ApplicationFlowCreateRequestV4 {
                pack_id: WorkflowPackId::try_new(pack).expect("Pack ID"),
                application: ApplicationFlowCreateRequestV3 {
                    title: format!("PRIVATE-TITLE-{category}"),
                    opportunity_metadata: if pack == canisend_app::ACADEMIC_JOB_WORKFLOW_PACK_ID {
                        std::collections::BTreeMap::from([(
                            WorkflowPackItemId::try_new("institution").expect("institution field"),
                            canisend_contracts::ApplicationFieldValueV3::ShortText(
                                "Fixture University".to_owned(),
                            ),
                        )])
                    } else {
                        Default::default()
                    },
                    application_metadata: Default::default(),
                    source_text: source_text.to_owned(),
                    requirements: vec![ApplicationFlowRequirementDraftV3 {
                        category: WorkflowPackItemId::try_new(category).expect("category"),
                        statement: source_text.to_owned(),
                        priority: RequirementPriorityV3::Mandatory,
                        start_byte: 0,
                        end_byte: source_text.len() as u64,
                    }],
                },
            },
        )
        .expect("create Application")
        .data
        .stored;
        applications.push(created);
    }
    let before = Application::workspace_status_v4(&root)
        .expect("before")
        .data
        .status;
    for (selected, other) in [
        (&applications[0], &applications[1]),
        (&applications[1], &applications[0]),
    ] {
        let selected_id = selected.snapshot.application.id.as_str();
        let other_id = other.snapshot.application.id.as_str();
        let mut mcp = McpProcess::start_with_application(&root, Some(selected_id));
        mcp.initialize();
        let listed = mcp.request(2, "tools/list", json!({}));
        let tools = listed["result"]["tools"].as_array().expect("tools");
        assert_eq!(tools.len(), CANISEND_MCP_TOOLS.len());
        // Validly typed inputs ensure rejection comes from binding, not deserialization.
        let source = &other.snapshot.requirements[0].source_span.content;
        let samples = json!({
            "application_id": other_id,
            "requirement_id": other.snapshot.requirements[0].id,
            "deliverable_id": "deliverable-fixture",
            "task_id": "task-fixture", "expected_generation": 3, "candidate_sha256": "0".repeat(64),
            "expected_revision": 1,
            "source": source, "profile_source": source, "evidence": source,
            "change": "associate", "preview_token": "unissued-fixture-token",
            "preview_sha256": "0".repeat(64), "request_confirmation": true,
            "request_private_read": true, "request_private_export": true,
            "decisions": [], "requirements": [], "deliverables": [],
            "proposals": {"profile_revision": 1, "proposals": []},
            "decision": "fixture", "title": "fixture", "media_type": "text/plain",
            "content": "fixture", "destination": "exports/fixture"
        });
        let mut scoped_count = 0;
        for (index, tool) in tools.iter().enumerate() {
            let properties = tool["inputSchema"]["properties"]
                .as_object()
                .expect("properties");
            let arguments = properties
                .keys()
                .map(|key| {
                    (
                        key.clone(),
                        samples
                            .get(key)
                            .unwrap_or_else(|| panic!("missing typed fixture for {key}"))
                            .clone(),
                    )
                })
                .collect::<serde_json::Map<_, _>>();
            let scoped = properties.contains_key("application_id");
            scoped_count += usize::from(scoped);
            let response = mcp.request(
                10 + index as u64,
                "tools/call",
                json!({
                    "name": tool["name"], "arguments": arguments
                }),
            );
            assert_eq!(
                response["error"]["code"], -32602,
                "{}: {response}",
                tool["name"]
            );
            assert_eq!(
                response["error"]["data"]["code"],
                if scoped {
                    "application.binding-mismatch"
                } else {
                    "application.workspace-scope-required"
                }
            );
            let text = response["error"]["message"].as_str().expect("error text");
            assert!(
                text.contains(if scoped {
                    "does not match the bound Application"
                } else {
                    "Workspace-wide results are unavailable"
                }),
                "{}: {text}",
                tool["name"]
            );
            assert!(!response.to_string().contains("PRIVATE-TITLE"));
        }
        assert_eq!(scoped_count, 36);
        let own = mcp.request(
            100,
            "tools/call",
            json!({
                "name": "canisend_application_show", "arguments": { "application_id": selected_id }
            }),
        );
        assert_eq!(own["result"]["isError"], false);
        assert_eq!(
            own["result"]["structuredContent"]["data"]["snapshot"]["application"]["id"],
            selected_id
        );
        assert!(!own.to_string().contains(other_id));
        for (index, name) in [
            "canisend_profile_association_list",
            "canisend_evidence_association_list",
        ]
        .iter()
        .enumerate()
        {
            let response = mcp.request(
                110 + index as u64,
                "tools/call",
                json!({
                    "name": name, "arguments": { "application_id": selected_id }
                }),
            );
            assert_eq!(response["error"]["code"], -32602);
            assert_eq!(
                response["error"]["data"]["code"],
                "application.workspace-scope-required"
            );
        }
    }
    let mut unbound = McpProcess::start(&root);
    unbound.initialize();
    let discovered = unbound.request(
        2,
        "tools/call",
        json!({"name": "canisend_application_list", "arguments": {}}),
    );
    assert_eq!(discovered["result"]["isError"], false);
    for application in &applications {
        assert!(
            discovered
                .to_string()
                .contains(application.snapshot.application.id.as_str())
        );
    }
    drop(unbound);
    for invalid in ["", "unknown-application"] {
        let result = Command::new(env!("CARGO_BIN_EXE_canisend"))
            .arg("--workspace")
            .arg(&root)
            .args(["mcp", "serve", "--application", invalid])
            .stdin(Stdio::null())
            .output()
            .expect("invalid binding startup");
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
    }
    assert_eq!(
        before,
        Application::workspace_status_v4(&root)
            .expect("after")
            .data
            .status
    );
    fs::remove_dir_all(root).expect("remove workspace");
}
