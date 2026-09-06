#![forbid(unsafe_code)]

use std::{
    env,
    io::{self, BufRead, Write},
    thread,
    time::Duration,
};

use serde_json::{Value, json};

fn main() {
    if let Err(error) = run() {
        eprintln!("fake App Server failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let home = std::path::PathBuf::from(env::var_os("HOME").ok_or("missing dedicated HOME")?);
    let provider = home.join("provider");
    for (key, expected) in [
        ("USERPROFILE", home.clone()),
        ("CODEX_HOME", provider.clone()),
        ("XDG_CONFIG_HOME", home.join("config")),
        ("XDG_DATA_HOME", home.join("data")),
        ("APPDATA", home.join("data")),
        ("LOCALAPPDATA", home.join("local")),
    ] {
        if env::var_os(key).map(std::path::PathBuf::from) != Some(expected) {
            return Err(format!("incorrect dedicated directory for {key}"));
        }
    }
    if home.file_name().and_then(|name| name.to_str()) != Some("codex-isolated-v1") {
        return Err("unexpected provider namespace".to_owned());
    }
    for (key, _) in env::vars_os() {
        if ![
            "HOME",
            "USERPROFILE",
            "CODEX_HOME",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "APPDATA",
            "LOCALAPPDATA",
            "PATH",
            "SystemRoot",
            "WINDIR",
            "TMP",
            "TEMP",
            "TMPDIR",
            "LANG",
            "LC_ALL",
            "DISPLAY",
            "WAYLAND_DISPLAY",
            "XDG_RUNTIME_DIR",
            "DBUS_SESSION_BUS_ADDRESS",
        ]
        .iter()
        .any(|allowed| key == *allowed)
        {
            return Err("unexpected inherited environment".to_owned());
        }
    }
    if args.first().map(String::as_str) == Some("login") {
        if args
            != [
                "login",
                "-c",
                "cli_auth_credentials_store=\"file\"",
                "-c",
                "forced_login_method=\"chatgpt\"",
            ]
        {
            return Err("unexpected login arguments".to_owned());
        }
        if env::current_dir().map_err(|error| error.to_string())? != home {
            return Err("login did not use dedicated working directory".to_owned());
        }
        println!("PRIVATE-LOGIN-URL");
        eprintln!("PRIVATE-LOGIN-DIAGNOSTIC");
        if provider.join("fail-fixture").exists() {
            return Err("PRIVATE-LOGIN-FAILURE".to_owned());
        }
        if provider.join("timeout-fixture").exists() {
            thread::sleep(Duration::from_secs(2));
        }
        std::fs::write(provider.join("login-fixture.marker"), "provider-owned")
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    if args.get(..3)
        != Some(&[
            "app-server".to_owned(),
            "--listen".to_owned(),
            "stdio://".to_owned(),
        ])
    {
        return Err("unexpected arguments".to_owned());
    }
    let mut settings = std::collections::BTreeMap::new();
    for pair in args[3..].chunks(2) {
        if pair.len() != 2 || pair[0] != "-c" {
            return Err("unexpected bootstrap override".to_owned());
        }
        let (key, value) = pair[1].split_once('=').ok_or("missing override value")?;
        if settings.insert(key, value).is_some() {
            return Err("duplicate bootstrap override".to_owned());
        }
    }
    let profile = "\"fixture-session\"";
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let installation = executable.parent().ok_or("missing installation")?;
    let filesystem = format!(
        "{{ \":minimal\" = \"read\", {} = \"read\", {} = \"read\" }}",
        json!(cwd),
        json!(installation)
    );
    for (key, value) in [
        ("default_permissions", profile),
        (
            "permissions.fixture-session.filesystem",
            filesystem.as_str(),
        ),
        ("permissions.fixture-session.network.enabled", "false"),
        ("approval_policy", "\"never\""),
        ("approvals_reviewer", "\"user\""),
        ("web_search", "\"disabled\""),
        ("cli_auth_credentials_store", "\"file\""),
        ("forced_login_method", "\"chatgpt\""),
        ("features.shell_tool", "false"),
        ("features.unified_exec", "false"),
        ("features.multi_agent", "false"),
        ("features.shell_snapshot", "false"),
        ("features.plugins", "false"),
        ("features.apps", "false"),
        ("features.hooks", "false"),
        ("features.skill_mcp_dependency_install", "false"),
    ] {
        if settings.remove(key) != Some(value) {
            return Err(format!("missing restricted bootstrap setting: {key}"));
        }
    }
    if !settings.is_empty() {
        return Err("unexpected bootstrap settings".to_owned());
    }
    let scenario = env::current_dir()
        .map_err(|error| error.to_string())?
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "missing scenario directory".to_owned())?
        .to_owned();
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut active_thread = String::new();
    let mut active_turn = String::new();
    let mut turn_number = 0_u64;

    while let Some(message) = read_message(&mut input)? {
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let id = message.get("id").cloned();
        if matches!(method, "thread/start" | "thread/resume" | "turn/start") {
            if message["params"]["approvalPolicy"] != "never"
                || message["params"]["approvalsReviewer"] != "user"
            {
                return Err("client did not reassert the explicit user/deny policy".to_owned());
            }
            if message["params"]["permissions"] != "fixture-session"
                || message["params"].get("sandbox").is_some()
                || message["params"].get("sandboxPolicy").is_some()
            {
                return Err("named policy was not reasserted exclusively".to_owned());
            }
            if scenario == "policy-rejected" {
                write_value(
                    &mut output,
                    &json!({
                        "id": id,
                        "error": { "code": -32602, "message": "PRIVATE-POLICY-DETAIL" }
                    }),
                )?;
                // Any fallback request after rejection is a fixture failure.
                if read_message(&mut input)?.is_some() {
                    return Err("client retried after the named policy was rejected".to_owned());
                }
                return Ok(());
            }
        }
        match method {
            "initialize" => {
                if message["params"]["capabilities"]["experimentalApi"] != true {
                    return Err("named policies require the experimental API".to_owned());
                }
                if scenario == "timeout" {
                    thread::sleep(Duration::from_secs(1));
                    return Ok(());
                }
                if scenario == "malformed" {
                    output.write_all(b"{not-json}\n").map_err(io_error)?;
                    output.flush().map_err(io_error)?;
                    return Ok(());
                }
                if scenario == "oversized" {
                    output
                        .write_all(&vec![b'x'; 1024 * 1024 + 1])
                        .map_err(io_error)?;
                    output.write_all(b"\n").map_err(io_error)?;
                    output.flush().map_err(io_error)?;
                    return Ok(());
                }
                respond(
                    &mut output,
                    id,
                    json!({
                        "codexHome": if scenario == "wrong-config-home" { json!("/fixture/host-codex") } else { json!(provider) },
                        "platformFamily": "fixture",
                        "platformOs": "fixture",
                        "userAgent": "canisend-fake-app-server"
                    }),
                )?;
            }
            "initialized" => {}
            "account/read" => {
                respond(
                    &mut output,
                    id,
                    if scenario == "authentication-required" {
                        json!({ "account": null, "requiresOpenaiAuth": true })
                    } else {
                        json!({
                            "account": { "type": "fixture" },
                            "requiresOpenaiAuth": true
                        })
                    },
                )?;
            }
            "thread/start" => {
                active_thread = "thread-new".to_owned();
                respond(
                    &mut output,
                    id,
                    json!({ "thread": { "id": active_thread } }),
                )?;
            }
            "thread/resume" => {
                active_thread = message["params"]["threadId"]
                    .as_str()
                    .ok_or_else(|| "resume omitted threadId".to_owned())?
                    .to_owned();
                respond(
                    &mut output,
                    id,
                    json!({ "thread": { "id": active_thread } }),
                )?;
            }
            "turn/start" => {
                turn_number += 1;
                active_turn = format!("turn-{turn_number}");
                respond(&mut output, id, json!({ "turn": { "id": active_turn } }))?;
                notify(
                    &mut output,
                    "turn/started",
                    json!({
                        "threadId": active_thread,
                        "turn": { "id": active_turn }
                    }),
                )?;
                if scenario == "child-exit" {
                    eprintln!(
                        "PRIVATE_FIXTURE_BODY {}",
                        env::current_dir()
                            .map_err(|error| error.to_string())?
                            .display()
                    );
                    return Ok(());
                }
                if matches!(
                    scenario.as_str(),
                    "mcp-item" | "mcp-user-input" | "mcp-elicitation" | "host-permissions"
                ) {
                    let item = json!({
                        "id": "mcp-item-1", "type": "mcpToolCall", "server": "canisend",
                        "tool": "canisend_review_inspect", "status": "inProgress",
                        "arguments": { "confirmed_private_read": true, "private": "PRIVATE-MCP-BODY" },
                        "result": null, "error": null, "durationMs": null
                    });
                    notify(
                        &mut output,
                        "item/started",
                        json!({
                            "threadId": active_thread, "turnId": active_turn, "item": item
                        }),
                    )?;
                    if scenario == "mcp-item" {
                        let mut completed = item;
                        completed["status"] = json!("completed");
                        completed["result"] = json!({"content": [{"type": "text", "text": "PRIVATE-MCP-BODY"}], "structuredContent": {"private": "PRIVATE-MCP-BODY"}});
                        notify(
                            &mut output,
                            "item/completed",
                            json!({
                                "threadId": active_thread, "turnId": active_turn, "item": completed
                            }),
                        )?;
                    } else {
                        let (method, params) = match scenario.as_str() {
                            "mcp-user-input" => (
                                "item/tool/requestUserInput",
                                json!({
                                    "threadId": active_thread, "turnId": active_turn, "itemId": "mcp-item-1",
                                    "isBlocking": true, "questions": [{"id": "approval", "header": "Approve",
                                    "question": "PRIVATE-MCP-BODY", "options": [{"label": "Approve", "description": "fixture"}]}]
                                }),
                            ),
                            "mcp-elicitation" => (
                                "mcpServer/elicitation/request",
                                json!({
                                    "serverName": "canisend", "threadId": active_thread, "turnId": active_turn,
                                    "mode": "form", "message": "PRIVATE-MCP-BODY",
                                    "_meta": {
                                        "codex_approval_kind": "mcp_tool_call",
                                        "tool_description": "PRIVATE-MCP-BODY",
                                        "tool_params": {"confirmed_private_read": true, "private": "PRIVATE-MCP-BODY"},
                                        "tool_params_display": [{"name": "private", "value": "PRIVATE-MCP-BODY"}]
                                    },
                                    "requestedSchema": {"type": "object", "properties": {}}
                                }),
                            ),
                            _ => (
                                "item/permissions/requestApproval",
                                json!({
                                    "threadId": active_thread, "turnId": active_turn, "itemId": "mcp-item-1",
                                    "permissions": {}, "reason": "PRIVATE-MCP-BODY"
                                }),
                            ),
                        };
                        write_value(
                            &mut output,
                            &json!({"id": "unqualified-approval", "method": method, "params": params}),
                        )?;
                        let reply = read_message(&mut input)?
                            .ok_or_else(|| "missing rejection".to_owned())?;
                        if reply["id"] != "unqualified-approval"
                            || reply["error"]["code"] != -32601
                            || reply.get("result").is_some()
                        {
                            return Err(
                                "unqualified permission request was not rejected".to_owned()
                            );
                        }
                        return Ok(());
                    }
                }
                if matches!(scenario.as_str(), "approval" | "unsupported-request") {
                    let request_method = if scenario == "approval" {
                        "item/commandExecution/requestApproval"
                    } else {
                        "fixture/unsupported"
                    };
                    write_value(
                        &mut output,
                        &json!({
                            "id": "server-request-1",
                            "method": request_method,
                            "params": { "private": "must-not-cross-boundary" }
                        }),
                    )?;
                    let reply = read_message(&mut input)?
                        .ok_or_else(|| "client omitted server-request response".to_owned())?;
                    if scenario == "approval" {
                        if reply["result"]["decision"] != "decline" {
                            return Err("approval was not declined".to_owned());
                        }
                    } else {
                        if reply["error"]["code"] != -32601 {
                            return Err("unsupported request was not rejected".to_owned());
                        }
                        return Ok(());
                    }
                }
                delta(&mut output, &active_thread, &active_turn, "Hello")?;
                if scenario != "cancel" {
                    delta(&mut output, &active_thread, &active_turn, " world")?;
                    complete(&mut output, &active_thread, &active_turn, "completed")?;
                }
            }
            "turn/interrupt" => {
                respond(&mut output, id, json!({}))?;
                complete(&mut output, &active_thread, &active_turn, "interrupted")?;
            }
            _ => return Err(format!("unexpected method {method}")),
        }
    }
    Ok(())
}

fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut line = String::new();
    if reader.read_line(&mut line).map_err(io_error)? == 0 {
        return Ok(None);
    }
    serde_json::from_str(&line)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn respond(writer: &mut impl Write, id: Option<Value>, result: Value) -> Result<(), String> {
    write_value(writer, &json!({ "id": id, "result": result }))
}

fn notify(writer: &mut impl Write, method: &str, params: Value) -> Result<(), String> {
    write_value(writer, &json!({ "method": method, "params": params }))
}

fn delta(
    writer: &mut impl Write,
    thread_id: &str,
    turn_id: &str,
    text: &str,
) -> Result<(), String> {
    notify(
        writer,
        "item/agentMessage/delta",
        json!({
            "threadId": thread_id,
            "turnId": turn_id,
            "itemId": "message-1",
            "delta": text
        }),
    )
}

fn complete(
    writer: &mut impl Write,
    thread_id: &str,
    turn_id: &str,
    status: &str,
) -> Result<(), String> {
    notify(
        writer,
        "turn/completed",
        json!({
            "threadId": thread_id,
            "turn": { "id": turn_id, "status": status }
        }),
    )
}

fn write_value(writer: &mut impl Write, value: &Value) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, value).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(io_error)?;
    writer.flush().map_err(io_error)
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}
