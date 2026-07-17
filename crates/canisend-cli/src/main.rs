#![forbid(unsafe_code)]

use std::{io::IsTerminal, process::ExitCode};

use canisend_contracts::{
    AGENT_PROTOCOL, AgentError, AgentResponse, CapabilitiesData, RESOURCE_FORMAT, VersionData,
    WORKSPACE_FORMAT,
};
use canisend_core::CapabilityRegistry;
use clap::{Args, Parser, Subcommand};
use serde_json::json;

#[derive(Debug, Parser)]
#[command(
    name = "canisend",
    about = "Evidence-backed application preparation",
    disable_version_flag = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print native product and protocol versions.
    Version(OutputArgs),
    /// Check the native binary's embedded foundation.
    Doctor(OutputArgs),
    /// Inspect interfaces intended for agent hosts.
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    /// List compiled capabilities and their availability.
    Capabilities(OutputArgs),
}

#[derive(Debug, Args)]
struct OutputArgs {
    /// Emit exactly one canisend.agent/v2 JSON object on stdout.
    #[arg(long)]
    json: bool,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("canisend: {error}");
            ExitCode::from(6)
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Version(output) => version(output.json),
        Command::Doctor(output) => doctor(output.json),
        Command::Agent {
            command: AgentCommand::Capabilities(output),
        } => capabilities(output.json),
    }
}

fn version(json_output: bool) -> Result<(), Box<dyn std::error::Error>> {
    let data = VersionData {
        product: "canisend".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol: AGENT_PROTOCOL.to_owned(),
        workspace_format: WORKSPACE_FORMAT.to_owned(),
        resource_format: RESOURCE_FORMAT.to_owned(),
        rustc: env!("CANISEND_RUSTC_VERSION").to_owned(),
        target: env!("CANISEND_BUILD_TARGET").to_owned(),
        git_revision: env!("CANISEND_GIT_REVISION").to_owned(),
    };

    if wants_json(json_output) {
        write_response(&AgentResponse::success(
            "product.version",
            "available",
            serde_json::to_value(data)?,
        ))?;
    } else {
        println!("canisend {}", data.version);
        println!("protocol: {}", data.protocol);
        println!("target: {}", data.target);
        println!("git: {}", data.git_revision);
    }
    Ok(())
}

fn doctor(json_output: bool) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(message) = canisend_resources::verify() {
        let response = AgentResponse::failure(
            "product.doctor",
            "unhealthy",
            AgentError {
                code: "resources.integrity_failed".to_owned(),
                message,
                retryable: false,
                details: None,
            },
        );
        if wants_json(json_output) {
            write_response(&response)?;
        }
        return Err("embedded resource integrity check failed".into());
    }

    let data = json!({
        "resource_manifest": "verified",
        "resource_count": canisend_resources::manifest().len(),
        "python_required": false,
    });
    if wants_json(json_output) {
        write_response(&AgentResponse::success("product.doctor", "healthy", data))?;
    } else {
        println!("CanISend native foundation: healthy");
        println!("Embedded resources: verified");
        println!("Python runtime: not required");
    }
    Ok(())
}

fn capabilities(json_output: bool) -> Result<(), Box<dyn std::error::Error>> {
    let data = CapabilitiesData {
        product_version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol: AGENT_PROTOCOL.to_owned(),
        workspace_format: WORKSPACE_FORMAT.to_owned(),
        resource_format: RESOURCE_FORMAT.to_owned(),
        capabilities: CapabilityRegistry::built_in(),
    };

    if wants_json(json_output) {
        write_response(&AgentResponse::success(
            "agent.capabilities",
            "available",
            serde_json::to_value(data)?,
        ))?;
    } else {
        println!("CanISend {} capabilities", data.product_version);
        for capability in data.capabilities {
            println!("{}: {:?}", capability.id, capability.status);
        }
    }
    Ok(())
}

fn wants_json(explicit: bool) -> bool {
    explicit || !std::io::stdout().is_terminal()
}

fn write_response(response: &AgentResponse) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string(response)?);
    Ok(())
}
