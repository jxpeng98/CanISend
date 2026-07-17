#![forbid(unsafe_code)]

use std::process::ExitCode;

use canisend_contracts::AgentResponse;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<String>) -> Result<(), String> {
    match arguments.as_slice() {
        [area, command] if area == "schemas" && command == "check" => check_schemas(),
        [area, command] if area == "resources" && command == "check" => check_resources(),
        [area, command] if area == "release" && command == "check" => {
            check_schemas()?;
            check_resources()
        }
        _ => Err("usage: cargo run -p xtask -- <schemas|resources|release> check".to_owned()),
    }
}

fn check_schemas() -> Result<(), String> {
    let schema = schemars::schema_for!(AgentResponse);
    let json = serde_json::to_string(&schema).map_err(|error| error.to_string())?;
    if json.is_empty() || !json.contains("canisend.agent/v2") {
        return Err("agent response schema generation was incomplete".to_owned());
    }
    println!("schemas: ok");
    Ok(())
}

fn check_resources() -> Result<(), String> {
    canisend_resources::verify()?;
    if canisend_resources::manifest().is_empty() {
        return Err("embedded resource manifest is empty".to_owned());
    }
    println!("resources: ok");
    Ok(())
}
