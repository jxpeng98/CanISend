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
    if env::args().skip(1).collect::<Vec<_>>() != ["app-server", "--listen", "stdio://"] {
        return Err("unexpected arguments".to_owned());
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
        match method {
            "initialize" => {
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
                        "codexHome": "/fixture/codex",
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
