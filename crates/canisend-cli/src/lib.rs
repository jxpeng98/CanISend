#![forbid(unsafe_code)]

mod app_adapter;

use std::{
    ffi::OsString,
    fs,
    io::IsTerminal,
    path::{Path, PathBuf},
    process::ExitCode,
};

use canisend_app::{
    AgentHost, AgentMcpConfigurationRequest, AgentSkillsInstallRequest, Application,
    ApplicationArchiveRequest, ApplicationError, ApplicationFlowCreateRequestV3,
    ApplicationFlowCreateRequestV4, PrivateReadConsent, WorkspaceInitPolicy,
};
use canisend_contracts::{
    AgentError, AgentProtocolV4, ArtifactReference, ConsentRequest, ErrorCode, ExitClass,
    NextAction, PrivacyClassification, Revision, SemanticVersion, VersionData, WorkflowPackId,
};
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Parser)]
#[command(
    name = "canisend",
    about = "Evidence-backed application preparation",
    disable_version_flag = true
)]
struct Cli {
    /// Resolve commands against this Workspace instead of discovering from the current directory.
    #[arg(long, global = true, value_name = "PATH")]
    workspace: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

/// Return the exact canonical leaves derived from the compiled clean-v4 Clap graph.
#[must_use]
pub fn clap_leaf_paths() -> Vec<String> {
    fn collect(command: &clap::Command, prefix: &[String], leaves: &mut Vec<String>) {
        let subcommands = command
            .get_subcommands()
            .filter(|subcommand| subcommand.get_name() != "help")
            .collect::<Vec<_>>();
        if subcommands.is_empty() {
            if !prefix.is_empty() {
                leaves.push(prefix.join(" "));
            }
            return;
        }
        for subcommand in subcommands {
            let mut path = prefix.to_vec();
            path.push(subcommand.get_name().to_owned());
            collect(subcommand, &path, leaves);
        }
    }

    let command = Cli::command();
    let mut leaves = Vec::new();
    collect(&command, &[], &mut leaves);
    leaves.sort();
    leaves
}

/// Return the clean-v4 public inventory.
#[must_use]
pub fn public_clap_leaf_paths() -> Vec<String> {
    clap_leaf_paths()
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print native product and protocol versions.
    Version(OutputArgs),
    /// Check the native binary's embedded foundation.
    Doctor(OutputArgs),
    /// Serve the canonical CanISend v4 tool surface over Model Context Protocol.
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
    /// Inspect generated public JSON Schemas.
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    /// Inspect resources embedded in this executable.
    Resource {
        #[command(subcommand)]
        command: ResourceCommand,
    },
    /// Initialize, inspect, check, back up, restore, or repair a Workspace v4.
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    /// Create and inspect Pack-bound Applications in a Workspace v4.
    Application {
        #[command(subcommand)]
        command: ApplicationCommand,
    },
    /// Import and inspect neutral Workspace-level Profile Sources.
    ProfileSource {
        #[command(subcommand)]
        command: ProfileSourceCommand,
    },
    /// Inspect Application-scoped Profile Source links.
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    /// Inspect Application-scoped confirmed Evidence links.
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    /// Inspect Pack-bound Requirements for one Application.
    Requirement {
        #[command(subcommand)]
        command: RequirementCommand,
    },
    /// Inspect the current Pack-bound Plan for one Application.
    Plan {
        #[command(subcommand)]
        command: PlanCommand,
    },
    /// Inspect Pack-bound Deliverable metadata for one Application.
    Deliverable {
        #[command(subcommand)]
        command: DeliverableCommand,
    },
    /// Inspect exact current private Deliverables for evidence-bound review.
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    /// Inspect verified local exports for one exact Application.
    Export {
        #[command(subcommand)]
        command: ExportCommand,
    },
    /// Install and inspect clean Agent v4 host resources for this Workspace.
    Host {
        #[command(subcommand)]
        command: HostCommand,
    },
}

#[derive(Debug, Subcommand)]
enum McpCommand {
    /// Serve versioned read-only tools over newline-delimited JSON-RPC on stdio.
    Serve,
}

#[derive(Debug, Subcommand)]
enum SchemaCommand {
    /// List generated Schemas with version and integrity metadata.
    List(OutputArgs),
    /// Inspect one generated Schema by logical ID or short slug.
    Show(SchemaShowArgs),
}

#[derive(Debug, Subcommand)]
enum ResourceCommand {
    /// List embedded resources with version and integrity metadata.
    List(OutputArgs),
}

#[derive(Debug, Subcommand)]
enum WorkspaceCommand {
    /// Initialize a neutral Workspace v4 at --workspace or the current directory.
    Init(WorkspaceInitArgs),
    /// Report authoritative Workspace and SQLite status.
    Status(OutputArgs),
    /// Verify database, blob, freshness, and projection invariants.
    Check(OutputArgs),
    /// Create and verify a consistent backup directory.
    Backup(WorkspaceBackupArgs),
    /// Restore a verified backup into a new empty directory.
    Restore(WorkspaceRestoreArgs),
    /// Rebuild missing or repair-required projections while preserving user edits.
    Repair(OutputArgs),
}

#[derive(Debug, Subcommand)]
enum ApplicationCommand {
    /// List Pack-bound Applications in the current Workspace v4.
    List(OutputArgs),
    /// Show one Pack-bound Application in the current Workspace v4.
    Show(ApplicationIdArgs),
    /// Archive one Application without deleting history or shared Workspace data.
    Archive(ApplicationArchiveArgs),
    /// Create a Pack-bound Application from a reviewed JSON request.
    Create(ApplicationCreateArgs),
}

#[derive(Debug, Subcommand)]
enum ProfileSourceCommand {
    /// List body-free Profile Source metadata from the current Workspace v4.
    List(OutputArgs),
    /// Import one reviewed Markdown, plain-text, or JSON file into Workspace v4 authority.
    Import(ProfileSourceImportArgs),
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    /// Inspect exact Profile Source links for one Application.
    Association {
        #[command(subcommand)]
        command: AssociationCommand,
    },
}

#[derive(Debug, Subcommand)]
enum EvidenceCommand {
    /// Inspect exact confirmed Evidence links for one Application.
    Association {
        #[command(subcommand)]
        command: AssociationCommand,
    },
}

#[derive(Debug, Subcommand)]
enum AssociationCommand {
    /// List body-free Workspace candidates and explicit links for one Application.
    List(ApplicationIdArgs),
}

#[derive(Debug, Subcommand)]
enum RequirementCommand {
    /// List Requirements for one exact Application revision and Pack binding.
    List(ApplicationIdArgs),
    /// Show one Requirement that belongs to the selected Application.
    Show(RequirementIdArgs),
}

#[derive(Debug, Subcommand)]
enum PlanCommand {
    /// Show the current Plan, or an explicit not-created state, for one Application.
    Show(ApplicationIdArgs),
}

#[derive(Debug, Subcommand)]
enum DeliverableCommand {
    /// List body-free Deliverable metadata for one exact Application revision.
    List(ApplicationIdArgs),
    /// Show one body-free Deliverable metadata record for the selected Application.
    Show(DeliverableIdArgs),
}

#[derive(Debug, Subcommand)]
enum ReviewCommand {
    /// Inspect exact current Deliverable bodies after explicit private-read consent.
    Inspect(PrivateApplicationIdArgs),
}

#[derive(Debug, Subcommand)]
enum ExportCommand {
    /// List verified local exports for one exact Application.
    List(ApplicationIdArgs),
    /// Show and verify one exact local export manifest and its document digests.
    Show(ExportShowArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ProfileSourceSensitivityArgument {
    Public,
    PrivateLocal,
}

impl From<ProfileSourceSensitivityArgument> for PrivacyClassification {
    fn from(value: ProfileSourceSensitivityArgument) -> Self {
        match value {
            ProfileSourceSensitivityArgument::Public => Self::Public,
            ProfileSourceSensitivityArgument::PrivateLocal => Self::PrivateLocal,
        }
    }
}

#[derive(Debug, Subcommand)]
enum HostCommand {
    /// Install managed Skills and prepare the exact MCP registration for one host.
    Setup(HostConfigurationArgs),
    /// Inspect managed Skills and show the current MCP registration guidance.
    Status(HostConfigurationArgs),
    /// Remove only unchanged, manifest-owned Skills; preserve host MCP configuration.
    Remove(HostRemoveArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum HostArgument {
    Codex,
    Claude,
    Generic,
}

impl From<HostArgument> for AgentHost {
    fn from(value: HostArgument) -> Self {
        match value {
            HostArgument::Codex => Self::Codex,
            HostArgument::Claude => Self::Claude,
            HostArgument::Generic => Self::Generic,
        }
    }
}

#[derive(Debug, Args)]
struct OutputArgs {
    /// Emit exactly one canisend.agent/v4 JSON object on stdout.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct SchemaShowArgs {
    /// Schema ID such as canisend.application/v3, or its short slug.
    id: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct WorkspaceBackupArgs {
    /// New or empty destination directory for the verified backup.
    destination: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct WorkspaceInitArgs {
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct WorkspaceRestoreArgs {
    /// Verified CanISend backup directory.
    backup: PathBuf,
    /// New or empty destination directory for the restored Workspace.
    destination: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ApplicationIdArgs {
    #[arg(long)]
    application: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct RequirementIdArgs {
    #[arg(long)]
    application: String,
    #[arg(long)]
    requirement: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct DeliverableIdArgs {
    #[arg(long)]
    application: String,
    #[arg(long)]
    deliverable: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct PrivateApplicationIdArgs {
    #[arg(long)]
    application: String,
    /// Confirm that CanISend may read current private Deliverable bodies.
    #[arg(long)]
    confirm_private_read: bool,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ExportShowArgs {
    #[arg(long)]
    application: String,
    /// Exact Workspace-relative export directory.
    #[arg(long)]
    destination: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ApplicationCreateArgs {
    /// Exact Workflow Pack ID bound to the new Application.
    #[arg(long)]
    pack: String,
    /// Reviewed bounded JSON request matching the canonical operation contract.
    #[arg(long, value_name = "PATH")]
    candidate: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ApplicationArchiveArgs {
    #[arg(long)]
    application: String,
    #[arg(long)]
    expected_revision: u64,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ProfileSourceImportArgs {
    /// Reviewed local Markdown, plain-text, or JSON file.
    source: PathBuf,
    /// Privacy classification stored with the source.
    #[arg(long, value_enum)]
    sensitivity: ProfileSourceSensitivityArgument,
    /// Confirm that CanISend may read the selected private-local file.
    #[arg(long)]
    confirm_private_read: bool,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct HostConfigurationArgs {
    /// Agent host that will load the Workspace-local resources.
    #[arg(long, value_enum)]
    host: HostArgument,
    /// Absolute CanISend executable path used by the MCP server registration.
    #[arg(long, value_name = "PATH")]
    executable: Option<PathBuf>,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct HostRemoveArgs {
    /// Agent host whose manifest-owned Skills should be removed.
    #[arg(long, value_enum)]
    host: HostArgument,
    #[command(flatten)]
    output: OutputArgs,
}

impl Cli {
    fn explicit_json(&self) -> bool {
        match &self.command {
            Command::Version(output) | Command::Doctor(output) => output.json,
            Command::Mcp {
                command: McpCommand::Serve,
            } => false,
            Command::Schema {
                command: SchemaCommand::List(output),
            }
            | Command::Resource {
                command: ResourceCommand::List(output),
            } => output.json,
            Command::Schema {
                command: SchemaCommand::Show(arguments),
            } => arguments.output.json,
            Command::Workspace {
                command: WorkspaceCommand::Init(arguments),
            } => arguments.output.json,
            Command::Workspace {
                command:
                    WorkspaceCommand::Status(output)
                    | WorkspaceCommand::Check(output)
                    | WorkspaceCommand::Repair(output),
            } => output.json,
            Command::Workspace {
                command: WorkspaceCommand::Backup(arguments),
            } => arguments.output.json,
            Command::Workspace {
                command: WorkspaceCommand::Restore(arguments),
            } => arguments.output.json,
            Command::Application { command } => match command {
                ApplicationCommand::List(output) => output.json,
                ApplicationCommand::Show(arguments) => arguments.output.json,
                ApplicationCommand::Archive(arguments) => arguments.output.json,
                ApplicationCommand::Create(arguments) => arguments.output.json,
            },
            Command::ProfileSource { command } => match command {
                ProfileSourceCommand::List(output) => output.json,
                ProfileSourceCommand::Import(arguments) => arguments.output.json,
            },
            Command::Profile {
                command: ProfileCommand::Association { command },
            }
            | Command::Evidence {
                command: EvidenceCommand::Association { command },
            } => match command {
                AssociationCommand::List(arguments) => arguments.output.json,
            },
            Command::Requirement { command } => match command {
                RequirementCommand::List(arguments) => arguments.output.json,
                RequirementCommand::Show(arguments) => arguments.output.json,
            },
            Command::Plan {
                command: PlanCommand::Show(arguments),
            } => arguments.output.json,
            Command::Deliverable { command } => match command {
                DeliverableCommand::List(arguments) => arguments.output.json,
                DeliverableCommand::Show(arguments) => arguments.output.json,
            },
            Command::Review {
                command: ReviewCommand::Inspect(arguments),
            } => arguments.output.json,
            Command::Export { command } => match command {
                ExportCommand::List(arguments) => arguments.output.json,
                ExportCommand::Show(arguments) => arguments.output.json,
            },
            Command::Host { command } => match command {
                HostCommand::Setup(arguments) | HostCommand::Status(arguments) => {
                    arguments.output.json
                }
                HostCommand::Remove(arguments) => arguments.output.json,
            },
        }
    }
}

struct CommandOutput {
    response: CommandResponseV4,
    human: Vec<String>,
}

#[derive(Serialize)]
struct CommandResponseV4 {
    protocol: AgentProtocolV4,
    operation: String,
    ok: bool,
    status: String,
    data: Option<Value>,
    artifacts: Vec<ArtifactReference>,
    required_consents: Vec<ConsentRequest>,
    warnings: Vec<String>,
    next_actions: Vec<NextAction>,
    error: Option<AgentError>,
}

impl CommandResponseV4 {
    fn success(operation: impl Into<String>, status: impl Into<String>, data: Value) -> Self {
        Self {
            protocol: AgentProtocolV4::V4,
            operation: operation.into(),
            ok: true,
            status: status.into(),
            data: Some(data),
            artifacts: Vec::new(),
            required_consents: Vec::new(),
            warnings: Vec::new(),
            next_actions: Vec::new(),
            error: None,
        }
    }

    fn failure(operation: impl Into<String>, status: impl Into<String>, error: AgentError) -> Self {
        Self {
            protocol: AgentProtocolV4::V4,
            operation: operation.into(),
            ok: false,
            status: status.into(),
            data: None,
            artifacts: Vec::new(),
            required_consents: Vec::new(),
            warnings: Vec::new(),
            next_actions: Vec::new(),
            error: Some(error),
        }
    }
}

struct CommandFailure {
    operation: &'static str,
    status: String,
    error: AgentError,
    human: String,
}

type CommandResult<T> = Result<T, Box<CommandFailure>>;

impl CommandFailure {
    fn new(
        operation: &'static str,
        status: impl Into<String>,
        code: ErrorCode,
        message: impl Into<String>,
        retryable: bool,
    ) -> Box<Self> {
        let message = message.into();
        Box::new(Self {
            operation,
            status: status.into(),
            error: AgentError {
                code,
                message: message.clone(),
                retryable,
                details: None,
                remediation: None,
            },
            human: message,
        })
    }

    fn exit_class(&self) -> ExitClass {
        self.error.code.exit_class()
    }

    fn response(&self) -> CommandResponseV4 {
        CommandResponseV4::failure(self.operation, self.status.clone(), self.error.clone())
    }
}

/// Run the clean-v4 CanISend command-line adapter using the current process arguments.
#[must_use]
pub fn run() -> ExitCode {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    if let Some(legacy_surface) = unsupported_legacy_surface(arguments.iter().cloned()) {
        return render_unsupported_legacy_surface(
            &legacy_surface,
            arguments
                .iter()
                .any(|argument| argument.to_str() == Some("--json")),
        );
    }
    let cli = Cli::parse_from(arguments);
    if matches!(
        &cli.command,
        Command::Mcp {
            command: McpCommand::Serve
        }
    ) {
        return match canisend_mcp::serve_stdio(cli.workspace.as_deref()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("canisend mcp serve: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let json_output = wants_json(cli.explicit_json());
    match execute(cli) {
        Ok(output) => render_success(output, json_output),
        Err(failure) => render_failure(*failure, json_output),
    }
}

const LEGACY_TOP_LEVEL_COMMANDS: &[&str] = &[
    "agent",
    "job",
    "content",
    "discovery",
    "task",
    "criteria",
    "match",
    "document",
    "review",
    "package",
    "render",
    "workflow",
];

fn unsupported_legacy_surface(arguments: impl IntoIterator<Item = OsString>) -> Option<String> {
    let mut arguments = arguments.into_iter();
    let _executable = arguments.next();
    let mut command_path = Vec::with_capacity(2);
    while let Some(argument) = arguments.next() {
        let argument = argument.to_str()?;
        if argument == "--workspace" {
            let _workspace = arguments.next();
            continue;
        }
        if argument.starts_with("--workspace=") || argument.starts_with('-') {
            continue;
        }
        command_path.push(argument.to_owned());
        if command_path.len() == 2 {
            break;
        }
    }

    let top_level = command_path.first()?;
    if LEGACY_TOP_LEVEL_COMMANDS.contains(&top_level.as_str()) {
        return Some(top_level.clone());
    }
    if top_level == "profile"
        && command_path
            .get(1)
            .is_some_and(|command| command != "association")
    {
        return Some(command_path.join(" "));
    }
    if top_level == "application"
        && command_path
            .get(1)
            .is_some_and(|leaf| leaf.starts_with("generic-"))
    {
        return Some(command_path.join(" "));
    }
    None
}

fn render_unsupported_legacy_surface(surface: &str, json_output: bool) -> ExitCode {
    let message = format!(
        "unsupported Alpha.6-era command `{surface}`; Alpha.7 accepts only clean Workspace v4 and neutral operation names"
    );
    if json_output {
        let response = json!({
            "protocol": "canisend.agent/v4",
            "operation": "compatibility.refuse",
            "ok": false,
            "status": "unsupported-legacy-surface",
            "error": {
                "code": ErrorCode::CompatibilityUnavailable.as_str(),
                "message": message,
                "retryable": false,
                "details": {
                    "legacy_surface": surface,
                    "required_workspace_format": "canisend.workspace/v4",
                    "required_agent_protocol": "canisend.agent/v4",
                    "mutation_attempted": false
                }
            },
            "next_actions": [{
                "action": "initialize a clean Workspace v4",
                "description": "Use `canisend workspace init` in a new or empty directory, then use neutral v4 Application and MCP operations; no legacy migration or compatibility negotiation is performed"
            }],
            "submission_performed": false
        });
        match serde_json::to_string(&response) {
            Ok(serialized) => println!("{serialized}"),
            Err(error) => {
                eprintln!("canisend: failed to serialize legacy refusal: {error}");
                return ExitCode::from(ExitClass::Internal.code());
            }
        }
    } else {
        eprintln!("canisend: {message}");
        eprintln!(
            "next: initialize a clean Workspace v4 with `canisend workspace init`; no legacy migration is performed"
        );
    }
    ExitCode::from(ExitClass::Conflict.code())
}

fn execute(cli: Cli) -> CommandResult<CommandOutput> {
    let Cli { workspace, command } = cli;
    match command {
        Command::Version(_) => version(),
        Command::Doctor(_) => doctor(),
        Command::Mcp {
            command: McpCommand::Serve,
        } => unreachable!("MCP server is dispatched before command rendering"),
        Command::Schema {
            command: SchemaCommand::List(_),
        } => schema_list(),
        Command::Schema {
            command: SchemaCommand::Show(arguments),
        } => schema_show(&arguments.id),
        Command::Resource {
            command: ResourceCommand::List(_),
        } => resource_list(),
        Command::Workspace {
            command: WorkspaceCommand::Init(_),
        } => workspace_init(workspace),
        Command::Workspace {
            command: WorkspaceCommand::Status(_),
        } => workspace_status(workspace),
        Command::Workspace {
            command: WorkspaceCommand::Check(_),
        } => workspace_check(workspace),
        Command::Workspace {
            command: WorkspaceCommand::Backup(arguments),
        } => workspace_backup(workspace, arguments.destination),
        Command::Workspace {
            command: WorkspaceCommand::Restore(arguments),
        } => workspace_restore(arguments.backup, arguments.destination),
        Command::Workspace {
            command: WorkspaceCommand::Repair(_),
        } => workspace_repair(workspace),
        Command::Application {
            command: ApplicationCommand::List(_),
        } => application_list(workspace),
        Command::Application {
            command: ApplicationCommand::Show(arguments),
        } => application_show(workspace, &arguments.application),
        Command::Application {
            command: ApplicationCommand::Archive(arguments),
        } => application_archive(workspace, arguments),
        Command::Application {
            command: ApplicationCommand::Create(arguments),
        } => application_create(workspace, arguments),
        Command::ProfileSource {
            command: ProfileSourceCommand::List(_),
        } => profile_source_list(workspace),
        Command::ProfileSource {
            command: ProfileSourceCommand::Import(arguments),
        } => profile_source_import(workspace, arguments),
        Command::Profile {
            command:
                ProfileCommand::Association {
                    command: AssociationCommand::List(arguments),
                },
        } => profile_association_list(workspace, &arguments.application),
        Command::Evidence {
            command:
                EvidenceCommand::Association {
                    command: AssociationCommand::List(arguments),
                },
        } => evidence_association_list(workspace, &arguments.application),
        Command::Requirement {
            command: RequirementCommand::List(arguments),
        } => requirement_list(workspace, &arguments.application),
        Command::Requirement {
            command: RequirementCommand::Show(arguments),
        } => requirement_show(workspace, &arguments.application, &arguments.requirement),
        Command::Plan {
            command: PlanCommand::Show(arguments),
        } => plan_show(workspace, &arguments.application),
        Command::Deliverable {
            command: DeliverableCommand::List(arguments),
        } => deliverable_list(workspace, &arguments.application),
        Command::Deliverable {
            command: DeliverableCommand::Show(arguments),
        } => deliverable_show(workspace, &arguments.application, &arguments.deliverable),
        Command::Review {
            command: ReviewCommand::Inspect(arguments),
        } => review_inspect(workspace, arguments),
        Command::Export {
            command: ExportCommand::List(arguments),
        } => export_list(workspace, &arguments.application),
        Command::Export {
            command: ExportCommand::Show(arguments),
        } => export_show(workspace, arguments),
        Command::Host {
            command: HostCommand::Setup(arguments),
        } => host_setup(workspace, arguments),
        Command::Host {
            command: HostCommand::Status(arguments),
        } => host_status(workspace, arguments),
        Command::Host {
            command: HostCommand::Remove(arguments),
        } => host_remove(workspace, arguments),
    }
}

fn version() -> CommandResult<CommandOutput> {
    let product = Application::product_summary();
    let data = VersionData {
        product: product.product,
        version: SemanticVersion::try_new(product.version).map_err(internal_version)?,
        protocol: product.protocol,
        workspace_format: product.workspace_format,
        resource_format: product.resource_format,
        rustc: env!("CANISEND_RUSTC_VERSION").to_owned(),
        target: env!("CANISEND_BUILD_TARGET").to_owned(),
        git_revision: env!("CANISEND_GIT_REVISION").to_owned(),
    };
    success(
        "product.version",
        "available",
        &data,
        vec![
            format!("canisend {}", data.version),
            format!("protocol: {}", data.protocol),
            format!("target: {}", data.target),
            format!("git: {}", data.git_revision),
        ],
    )
}

fn doctor() -> CommandResult<CommandOutput> {
    let receipt = Application::doctor().map_err(|error| {
        let mut failure = app_adapter::failure("product.doctor", error);
        failure.status = "unhealthy".to_owned();
        failure
    })?;
    let doctor = receipt.data;
    let data = json!({
        "resource_manifest": "verified",
        "resource_count": doctor.embedded_resources,
        "schema_count": doctor.schema_count,
        "embedded_typst": "verified",
        "default_fonts": "embedded",
        "system_font_scan": doctor.system_font_scan,
        "runtime_package_downloads": doctor.runtime_package_downloads,
        "python_required": doctor.python_required,
        "render_probe": {
            "target": env!("CANISEND_BUILD_TARGET"),
            "page_count": doctor.rendered_pages,
            "pdf_bytes": doctor.rendered_pdf_bytes,
            "warning_count": doctor.render_warning_count,
            "elapsed_millis": doctor.render_elapsed_millis,
            "binary_size_bytes": doctor.binary_size_bytes,
            "release_binary_budget_bytes": doctor.release_binary_budget_bytes,
        },
    });
    Ok(CommandOutput {
        response: CommandResponseV4::success("product.doctor", "healthy", data),
        human: vec![
            "CanISend native foundation: healthy".to_owned(),
            "Embedded resources: verified".to_owned(),
            "Generated schemas: verified".to_owned(),
            "Embedded Typst renderer: verified".to_owned(),
            format!(
                "Cross-platform probe: {} pages, {} bytes, {} ms",
                doctor.rendered_pages, doctor.rendered_pdf_bytes, doctor.render_elapsed_millis
            ),
            "System fonts and runtime packages: disabled".to_owned(),
            "Python runtime: not required".to_owned(),
        ],
    })
}

fn schema_list() -> CommandResult<CommandOutput> {
    let data = Application::schema_catalog()
        .map_err(|error| app_adapter::failure("schema.list", error))?
        .data;
    let human = data
        .schemas
        .iter()
        .map(|schema| format!("{} {}", schema.id, schema.sha256))
        .collect();
    success("schema.list", "available", &data, human)
}

fn schema_show(query: &str) -> CommandResult<CommandOutput> {
    let schema = Application::schema_detail(query)
        .map_err(|error| app_adapter::failure("schema.show", error))?
        .data;
    success(
        "schema.show",
        "available",
        &schema,
        vec![
            format!("{} {}", schema.id, schema.version),
            format!("resource: {}", schema.resource_id),
            format!("sha256: {}", schema.sha256),
        ],
    )
}

fn resource_list() -> CommandResult<CommandOutput> {
    let data = Application::resource_catalog()
        .map_err(|error| app_adapter::failure("resource.list", error))?
        .data;
    let human = data
        .resources
        .iter()
        .map(|resource| format!("{} [{}]", resource.id, resource.kind))
        .collect();
    success("resource.list", "available", &data, human)
}

fn workspace_init(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let root = workspace_path.unwrap_or_else(|| PathBuf::from("."));
    let receipt = Application::initialize_workspace_v4_with_policy(
        &root,
        WorkspaceInitPolicy::PreserveExistingFiles,
    )
    .map(|receipt| (receipt.data.path, receipt.data.status))
    .map_err(|error| app_adapter::failure("workspace.initialize.commit", error))?;
    let (path, data) = receipt;
    success(
        "workspace.initialize.commit",
        "initialized",
        &data,
        vec![
            format!("Initialized CanISend Workspace at {}", path.display()),
            format!("Workspace ID: {}", data.workspace_id),
            format!("Workspace format: {}", data.workspace_format),
            "Workflow Packs bind to individual Applications".to_owned(),
        ],
    )
}

fn workspace_status(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root_v4(workspace_path, "workspace.status")?;
    let data = Application::workspace_status_v4(&root)
        .map_err(|error| app_adapter::failure("workspace.status", error))?
        .data
        .status;
    success(
        "workspace.status",
        "available",
        &data,
        vec![
            format!("Workspace: {}", data.workspace_id),
            format!("Format: {}", data.workspace_format),
            format!("Applications: {}", data.application_count),
            format!("SQLite: {} ({})", data.sqlite_version, data.journal_mode),
            format!("Artifacts: {}", data.artifact_count),
        ],
    )
}

fn workspace_check(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root_v4(workspace_path, "workspace.check")?;
    let data = Application::check_workspace_v4(&root)
        .map_err(|error| app_adapter::failure("workspace.check", error))?
        .data
        .check;
    let status = if data.ok { "healthy" } else { "issues-found" };
    success(
        "workspace.check",
        status,
        &data,
        vec![
            format!("Workspace check: {status}"),
            format!("Database integrity: {}", data.database_integrity),
            format!("Issues: {}", data.issues.len()),
        ],
    )
}

fn workspace_backup(
    workspace_path: Option<PathBuf>,
    destination: PathBuf,
) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root_v4(workspace_path, "workspace.backup.commit")?;
    let result = Application::backup_workspace_v4(&root, &destination)
        .map_err(|error| app_adapter::failure("workspace.backup.commit", error))?
        .data;
    success(
        "workspace.backup.commit",
        "verified",
        &result.manifest,
        vec![
            format!("Verified backup: {}", result.destination.display()),
            format!("Blobs: {}", result.manifest.blobs.len()),
        ],
    )
}

fn workspace_restore(backup: PathBuf, destination: PathBuf) -> CommandResult<CommandOutput> {
    let data = Application::restore_workspace_v4(&backup, &destination)
        .map_err(|error| app_adapter::failure("workspace.restore.commit", error))?
        .data
        .workspace;
    success(
        "workspace.restore.commit",
        "restored",
        &data,
        vec![
            format!("Restored Workspace at {}", destination.display()),
            format!("Workspace ID: {}", data.workspace_id),
        ],
    )
}

fn workspace_repair(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root_v4(workspace_path, "workspace.repair.commit")?;
    let repaired = Application::repair_workspace_v4(&root)
        .map_err(|error| app_adapter::failure("workspace.repair.commit", error))?
        .data
        .repaired_projections;
    success(
        "workspace.repair.commit",
        "repaired",
        &json!({"repaired_projections": repaired}),
        vec![format!("Repaired projections: {repaired}")],
    )
}

fn host_setup(
    workspace_path: Option<PathBuf>,
    arguments: HostConfigurationArgs,
) -> CommandResult<CommandOutput> {
    let operation = "host.setup";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let host = AgentHost::from(arguments.host);
    let executable = host_executable(arguments.executable, operation)?;
    // Validate every non-mutating input before installing managed Workspace files.
    let mcp = Application::prepare_agent_mcp_configuration(&AgentMcpConfigurationRequest {
        host,
        workspace: root.clone(),
        executable,
    })
    .map_err(|error| app_adapter::failure(operation, error))?
    .data;
    let skills = Application::install_agent_skills(&AgentSkillsInstallRequest {
        host,
        workspace: root,
    })
    .map_err(|error| app_adapter::failure(operation, error))?
    .data;
    let registration = mcp
        .registration_command
        .as_deref()
        .unwrap_or("merge the returned configuration snippet into the selected host");
    let data = json!({
        "host": host,
        "skills": skills,
        "mcp": mcp,
        "mcp_configuration_mutated": false,
    });
    success(
        operation,
        "ready",
        &data,
        vec![
            format!("Agent v4 resources are ready for {}", host.as_str()),
            format!("MCP registration: {registration}"),
            "Host MCP configuration was not modified automatically".to_owned(),
        ],
    )
}

fn host_status(
    workspace_path: Option<PathBuf>,
    arguments: HostConfigurationArgs,
) -> CommandResult<CommandOutput> {
    let operation = "host.status";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let host = AgentHost::from(arguments.host);
    let executable = host_executable(arguments.executable, operation)?;
    let skills = Application::agent_skills_status(&AgentSkillsInstallRequest {
        host,
        workspace: root.clone(),
    })
    .map_err(|error| app_adapter::failure(operation, error))?
    .data;
    let mcp = Application::prepare_agent_mcp_configuration(&AgentMcpConfigurationRequest {
        host,
        workspace: root,
        executable,
    })
    .map_err(|error| app_adapter::failure(operation, error))?
    .data;
    let status = match skills.state {
        canisend_app::AgentSkillsStatusState::UpToDate => "ready",
        canisend_app::AgentSkillsStatusState::NotInstalled => "not-installed",
        canisend_app::AgentSkillsStatusState::UpdateAvailable => "update-available",
        canisend_app::AgentSkillsStatusState::Incomplete => "incomplete",
        canisend_app::AgentSkillsStatusState::UserModified => "user-modified",
        canisend_app::AgentSkillsStatusState::Unmanaged => "unmanaged",
    };
    let data = json!({
        "host": host,
        "skills": skills,
        "mcp": mcp,
        "mcp_configuration_mutated": false,
    });
    success(
        operation,
        status,
        &data,
        vec![
            format!("Agent v4 resource status for {}: {status}", host.as_str()),
            "The response includes the deterministic MCP registration and verification commands"
                .to_owned(),
        ],
    )
}

fn host_remove(
    workspace_path: Option<PathBuf>,
    arguments: HostRemoveArgs,
) -> CommandResult<CommandOutput> {
    let operation = "host.remove";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let host = AgentHost::from(arguments.host);
    let skills = Application::uninstall_agent_skills(&AgentSkillsInstallRequest {
        host,
        workspace: root,
    })
    .map_err(|error| app_adapter::failure(operation, error))?
    .data;
    let status = match skills.state {
        canisend_app::AgentSkillsUninstallState::Removed => "removed",
        canisend_app::AgentSkillsUninstallState::NotInstalled => "not-installed",
    };
    let removed_files = skills.removed_files;
    let data = json!({
        "host": host,
        "skills": skills,
        "mcp_configuration_removed": false,
    });
    success(
        operation,
        status,
        &data,
        vec![
            format!(
                "Removed {removed_files} unchanged CanISend-managed files for {}",
                host.as_str()
            ),
            "Host MCP configuration was preserved; remove its `canisend` server entry explicitly if desired"
                .to_owned(),
        ],
    )
}

fn host_executable(explicit: Option<PathBuf>, operation: &'static str) -> CommandResult<PathBuf> {
    explicit.map_or_else(
        || {
            std::env::current_exe().map_err(|error| {
                CommandFailure::new(
                    operation,
                    "io-failed",
                    ErrorCode::ExternalIoFailed,
                    format!("could not resolve the current CanISend executable: {error}"),
                    true,
                )
            })
        },
        Ok,
    )
}

fn application_list(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let operation = "application.list";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let applications = Application::list_application_models_v4(&root)
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    let human = if applications.is_empty() {
        vec!["No Applications found".to_owned()]
    } else {
        applications
            .iter()
            .map(|stored| {
                format!(
                    "{}  {}  [{}; revision {}; {:?}]",
                    stored.snapshot.application.id,
                    stored.snapshot.opportunity.title,
                    stored.snapshot.pack.id,
                    stored.snapshot.application.revision.get(),
                    stored.snapshot.application.lifecycle
                )
            })
            .collect()
    };
    success(operation, "current", &applications, human)
}

fn application_show(
    workspace_path: Option<PathBuf>,
    application_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "application.show";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let stored = Application::application_model_v4(&root, application_id)
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    success(
        operation,
        "current",
        &stored,
        vec![
            format!("Application: {}", stored.snapshot.opportunity.title),
            format!("Revision: {}", stored.snapshot.application.revision.get()),
            format!("Requirements: {}", stored.snapshot.requirements.len()),
            format!("Deliverables: {}", stored.snapshot.deliverables.len()),
        ],
    )
}

const MAX_APPLICATION_CANDIDATE_BYTES: u64 = 4 * 1024 * 1024;

fn application_create(
    workspace_path: Option<PathBuf>,
    arguments: ApplicationCreateArgs,
) -> CommandResult<CommandOutput> {
    let operation = "application.create.commit";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let request = read_application_candidate::<ApplicationFlowCreateRequestV3>(
        operation,
        &arguments.candidate,
    )?;
    let pack_id = WorkflowPackId::try_new(arguments.pack).map_err(|error| {
        app_adapter::failure(operation, ApplicationError::InvalidInput(error.to_string()))
    })?;
    let model = Application::create_application_flow_v4(
        &root,
        ApplicationFlowCreateRequestV4 {
            pack_id,
            application: request,
        },
    )
    .map_err(|error| app_adapter::failure(operation, error))?
    .data;
    success(
        operation,
        "created",
        &model,
        vec![
            format!("Application: {}", model.stored.snapshot.application.id),
            format!(
                "Revision: {}",
                model.stored.snapshot.application.revision.get()
            ),
            "Next: use a reviewed Agent v4 write operation to continue".to_owned(),
        ],
    )
}

fn application_archive(
    workspace_path: Option<PathBuf>,
    arguments: ApplicationArchiveArgs,
) -> CommandResult<CommandOutput> {
    let operation = "application.archive";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let expected_revision = Revision::try_new(arguments.expected_revision).map_err(|error| {
        CommandFailure::new(
            operation,
            "invalid",
            ErrorCode::InputInvalid,
            error.to_string(),
            false,
        )
    })?;
    let archived = Application::archive_application(
        &root,
        &arguments.application,
        ApplicationArchiveRequest {
            expected_revision,
            reason: "archive-application".to_owned(),
        },
    )
    .map_err(|error| app_adapter::failure(operation, error))?
    .data;
    success(
        operation,
        "archived",
        &archived,
        vec![
            format!("Application: {}", archived.stored.snapshot.application.id),
            format!(
                "Revision: {}",
                archived.stored.snapshot.application.revision.get()
            ),
            "History and shared Workspace data were preserved".to_owned(),
        ],
    )
}

fn profile_source_list(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let operation = "profile-source.list";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let model = Application::list_profile_sources_v4(&root)
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    let human = if model.sources.is_empty() {
        vec!["No Workspace Profile Sources found".to_owned()]
    } else {
        model
            .sources
            .iter()
            .map(|source| {
                format!(
                    "{}  {:?}  [{:?}; revision {}]",
                    source.id,
                    source.kind,
                    source.sensitivity,
                    source.revision.get()
                )
            })
            .collect()
    };
    success(operation, "available", &model, human)
}

fn profile_source_import(
    workspace_path: Option<PathBuf>,
    arguments: ProfileSourceImportArgs,
) -> CommandResult<CommandOutput> {
    let operation = "profile-source.import";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let sensitivity = PrivacyClassification::from(arguments.sensitivity);
    let consent = arguments
        .confirm_private_read
        .then(PrivateReadConsent::granted_by_user);
    let model =
        Application::import_profile_source_v4(&root, &arguments.source, sensitivity, consent)
            .map_err(|error| app_adapter::failure(operation, error))?
            .data;
    success(
        operation,
        "imported",
        &model,
        vec![
            format!("Profile Source: {}", model.source.id),
            format!("Profile revision: {}", model.profile_revision),
            "Only body-free metadata is returned; original bytes remain in local Workspace authority"
                .to_owned(),
        ],
    )
}

fn profile_association_list(
    workspace_path: Option<PathBuf>,
    application_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "profile.association.list";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let model = Application::list_profile_associations_v4(&root, application_id)
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    success(
        operation,
        "available",
        &model,
        vec![
            format!("Application: {}", model.application_id),
            format!("Workspace Profile Sources: {}", model.profile_sources.len()),
            format!("Explicit links: {}", model.associations.len()),
        ],
    )
}

fn evidence_association_list(
    workspace_path: Option<PathBuf>,
    application_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "evidence.association.list";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let model = Application::list_evidence_associations_v4(&root, application_id)
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    success(
        operation,
        "available",
        &model,
        vec![
            format!("Application: {}", model.application_id),
            format!("Confirmed Workspace Evidence: {}", model.evidence.len()),
            format!("Explicit links: {}", model.associations.len()),
        ],
    )
}

fn requirement_list(
    workspace_path: Option<PathBuf>,
    application_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "requirement.list";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let receipt = Application::list_requirements_v4(&root, application_id)
        .map_err(|error| app_adapter::failure(operation, error))?;
    let count = receipt.data.requirements.len();
    success(
        operation,
        &receipt.status,
        &receipt.data,
        vec![
            format!("Application: {}", receipt.data.context.application_id),
            format!("Pack: {}", receipt.data.context.pack.id),
            format!("Requirements: {count}"),
        ],
    )
}

fn requirement_show(
    workspace_path: Option<PathBuf>,
    application_id: &str,
    requirement_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "requirement.show";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let receipt = Application::show_requirement_v4(&root, application_id, requirement_id)
        .map_err(|error| app_adapter::failure(operation, error))?;
    success(
        operation,
        &receipt.status,
        &receipt.data,
        vec![
            format!("Requirement: {}", receipt.data.requirement.id),
            format!("Application: {}", receipt.data.context.application_id),
            format!("Pack: {}", receipt.data.context.pack.id),
        ],
    )
}

fn plan_show(
    workspace_path: Option<PathBuf>,
    application_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "plan.show";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let receipt = Application::show_plan_v4(&root, application_id)
        .map_err(|error| app_adapter::failure(operation, error))?;
    let state = if receipt.data.plan.is_some() {
        "Plan: current"
    } else {
        "Plan: not created"
    };
    success(
        operation,
        &receipt.status,
        &receipt.data,
        vec![
            format!("Application: {}", receipt.data.context.application_id),
            format!("Pack: {}", receipt.data.context.pack.id),
            state.to_owned(),
        ],
    )
}

fn deliverable_list(
    workspace_path: Option<PathBuf>,
    application_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "deliverable.list";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let receipt = Application::list_deliverables_v4(&root, application_id)
        .map_err(|error| app_adapter::failure(operation, error))?;
    let count = receipt.data.deliverables.len();
    success(
        operation,
        &receipt.status,
        &receipt.data,
        vec![
            format!("Application: {}", receipt.data.context.application_id),
            format!("Pack: {}", receipt.data.context.pack.id),
            format!("Deliverables: {count}"),
        ],
    )
}

fn deliverable_show(
    workspace_path: Option<PathBuf>,
    application_id: &str,
    deliverable_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "deliverable.show";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let receipt = Application::show_deliverable_v4(&root, application_id, deliverable_id)
        .map_err(|error| app_adapter::failure(operation, error))?;
    success(
        operation,
        &receipt.status,
        &receipt.data,
        vec![
            format!("Deliverable: {}", receipt.data.deliverable.id),
            format!("Application: {}", receipt.data.context.application_id),
            format!("Pack: {}", receipt.data.context.pack.id),
            "Content body remains behind the private-read boundary".to_owned(),
        ],
    )
}

fn review_inspect(
    workspace_path: Option<PathBuf>,
    arguments: PrivateApplicationIdArgs,
) -> CommandResult<CommandOutput> {
    let operation = "review.inspect";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let application_id = canisend_contracts::ApplicationId::try_new(arguments.application)
        .map_err(|error| {
            app_adapter::failure(
                operation,
                ApplicationError::InvalidEntityId(error.to_string()),
            )
        })?;
    let receipt = Application::inspect_review_v4(
        &root,
        &application_id,
        arguments
            .confirm_private_read
            .then(PrivateReadConsent::granted_by_user),
    )
    .map_err(|error| app_adapter::failure(operation, error))?;
    success(
        operation,
        &receipt.status,
        &receipt.data,
        vec![
            format!("Application: {application_id}"),
            format!("Deliverables reviewed: {}", receipt.data.deliverables.len()),
            "Submission performed: no".to_owned(),
        ],
    )
}

fn export_list(
    workspace_path: Option<PathBuf>,
    application_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "export.list";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let receipt = Application::list_exports_v4(&root, application_id)
        .map_err(|error| app_adapter::failure(operation, error))?;
    success(
        operation,
        &receipt.status,
        &receipt.data,
        vec![
            format!("Application: {}", receipt.data.context.application_id),
            format!("Verified local exports: {}", receipt.data.exports.len()),
        ],
    )
}

fn export_show(
    workspace_path: Option<PathBuf>,
    arguments: ExportShowArgs,
) -> CommandResult<CommandOutput> {
    let operation = "export.show";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let receipt =
        Application::show_export_v4(&root, &arguments.application, &arguments.destination)
            .map_err(|error| app_adapter::failure(operation, error))?;
    success(
        operation,
        &receipt.status,
        &receipt.data,
        vec![
            format!("Destination: {}", receipt.data.manifest.destination),
            format!(
                "Verified documents: {}",
                receipt.data.manifest.documents.len()
            ),
            "Submission performed: no".to_owned(),
        ],
    )
}

fn read_application_candidate<T>(operation: &'static str, path: &Path) -> CommandResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        CommandFailure::new(
            operation,
            "io-failed",
            ErrorCode::ExternalIoFailed,
            format!(
                "could not inspect candidate file {}: {error}",
                path.display()
            ),
            true,
        )
    })?;
    if !metadata.file_type().is_file() {
        return Err(CommandFailure::new(
            operation,
            "invalid",
            ErrorCode::InputPathRejected,
            format!(
                "candidate path must be a regular non-symlink file: {}",
                path.display()
            ),
            false,
        ));
    }
    if metadata.len() > MAX_APPLICATION_CANDIDATE_BYTES {
        return Err(CommandFailure::new(
            operation,
            "invalid",
            ErrorCode::InputInvalid,
            format!(
                "candidate file exceeds the {} byte limit",
                MAX_APPLICATION_CANDIDATE_BYTES
            ),
            false,
        ));
    }
    let bytes = fs::read(path).map_err(|error| {
        CommandFailure::new(
            operation,
            "io-failed",
            ErrorCode::ExternalIoFailed,
            format!("could not read candidate file {}: {error}", path.display()),
            true,
        )
    })?;
    if u64::try_from(bytes.len()).expect("candidate length fits u64")
        > MAX_APPLICATION_CANDIDATE_BYTES
    {
        return Err(CommandFailure::new(
            operation,
            "invalid",
            ErrorCode::InputInvalid,
            format!(
                "candidate file exceeds the {} byte limit",
                MAX_APPLICATION_CANDIDATE_BYTES
            ),
            false,
        ));
    }
    serde_json::from_slice(&bytes).map_err(|error| {
        CommandFailure::new(
            operation,
            "invalid",
            ErrorCode::InputInvalid,
            format!("candidate JSON does not match the operation contract: {error}"),
            false,
        )
    })
}

fn internal_version(error: impl std::fmt::Display) -> Box<CommandFailure> {
    CommandFailure::new(
        "product.contract",
        "invariant-failed",
        ErrorCode::InternalInvariantFailed,
        error.to_string(),
        false,
    )
}

fn success<T: serde::Serialize>(
    operation: &'static str,
    status: &str,
    data: &T,
    human: Vec<String>,
) -> CommandResult<CommandOutput> {
    let value = serde_json::to_value(data).map_err(|error| {
        CommandFailure::new(
            operation,
            "invariant-failed",
            ErrorCode::InternalInvariantFailed,
            error.to_string(),
            false,
        )
    })?;
    Ok(CommandOutput {
        response: CommandResponseV4::success(operation, status, value),
        human,
    })
}

fn wants_json(explicit: bool) -> bool {
    explicit || !std::io::stdout().is_terminal()
}

fn render_success(output: CommandOutput, json_output: bool) -> ExitCode {
    if json_output {
        render_json(&output.response)
    } else {
        for line in human_success_lines(&output) {
            println!("{line}");
        }
        ExitCode::SUCCESS
    }
}

fn render_failure(failure: CommandFailure, json_output: bool) -> ExitCode {
    let exit_class = failure.exit_class();
    if json_output {
        if render_json(&failure.response()) == ExitCode::from(ExitClass::Internal.code()) {
            return ExitCode::from(ExitClass::Internal.code());
        }
    } else {
        for line in human_failure_lines(&failure) {
            eprintln!("{line}");
        }
    }
    ExitCode::from(exit_class.code())
}

fn human_success_lines(output: &CommandOutput) -> Vec<String> {
    let mut lines = output.human.clone();
    lines.extend(
        output
            .response
            .warnings
            .iter()
            .map(|warning| format!("Warning: {warning}")),
    );
    lines.extend(
        output
            .response
            .next_actions
            .iter()
            .map(|action| format!("Next: {} — {}", action.action, action.description)),
    );
    lines
}

fn human_failure_lines(failure: &CommandFailure) -> Vec<String> {
    let mut lines = vec![format!(
        "canisend [{}]: {}",
        failure.error.code.as_str(),
        failure.human
    )];
    if let Some(remediation) = &failure.error.remediation {
        lines.push(format!(
            "Next: {} — {}",
            remediation.action, remediation.description
        ));
    }
    if failure.error.retryable {
        lines.push("Retryable: yes".to_owned());
    }
    lines
}

fn render_json(response: &CommandResponseV4) -> ExitCode {
    match serde_json::to_string(response) {
        Ok(serialized) => {
            println!("{serialized}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("canisend: failed to serialize protocol response: {error}");
            ExitCode::from(ExitClass::Internal.code())
        }
    }
}

#[cfg(test)]
mod tests {
    use canisend_contracts::{ErrorCode, NextAction, OperationRegistry, OperationSurface};
    use clap::Parser;

    use super::{
        ApplicationCommand, AssociationCommand, Cli, Command, CommandFailure, EvidenceCommand,
        ExitClass, HostCommand, ProfileCommand, ProfileSourceCommand, WorkspaceCommand,
        clap_leaf_paths, human_failure_lines, public_clap_leaf_paths, unsupported_legacy_surface,
    };

    #[test]
    fn clap_usage_errors_are_reserved_for_exit_two() {
        let error = Cli::try_parse_from(["canisend", "unknown"]).expect_err("unknown command");
        assert_eq!(error.exit_code(), i32::from(ExitClass::CliUsage.code()));
    }

    #[test]
    fn compiled_and_public_inventories_are_the_same_clean_v4_surface() {
        let actual = clap_leaf_paths()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let public = public_clap_leaf_paths()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let registered = OperationRegistry::built_in()
            .expect("operation registry")
            .surface_leaves(OperationSurface::Cli)
            .expect("CLI leaves");
        assert_eq!(actual, public);
        assert_eq!(actual, registered);
        assert_eq!(actual.len(), 28);
    }

    #[test]
    fn canonical_v4_commands_parse_and_legacy_paths_are_preflight_rejected() {
        let initialized = Cli::try_parse_from([
            "canisend",
            "--workspace",
            "/tmp/canisend-generic",
            "workspace",
            "init",
        ])
        .expect("neutral Workspace v4 init");
        assert!(matches!(
            initialized.command,
            Command::Workspace {
                command: WorkspaceCommand::Init(_)
            }
        ));

        let archive = Cli::try_parse_from([
            "canisend",
            "application",
            "archive",
            "--application",
            "019f3e88-6630-7000-8000-000000000001",
            "--expected-revision",
            "4",
            "--json",
        ])
        .expect("Application archive command");
        assert!(matches!(
            archive.command,
            Command::Application {
                command: ApplicationCommand::Archive(_)
            }
        ));

        let profile_sources = Cli::try_parse_from([
            "canisend",
            "--workspace",
            "/tmp/canisend-generic",
            "profile-source",
            "list",
        ])
        .expect("neutral Workspace Profile Source list");
        assert!(matches!(
            profile_sources.command,
            Command::ProfileSource {
                command: ProfileSourceCommand::List(_)
            }
        ));

        let profile_links = Cli::try_parse_from([
            "canisend",
            "profile",
            "association",
            "list",
            "--application",
            "019f3e88-6630-7000-8000-000000000001",
        ])
        .expect("canonical Profile association list");
        assert!(matches!(
            profile_links.command,
            Command::Profile {
                command: ProfileCommand::Association {
                    command: AssociationCommand::List(_)
                }
            }
        ));

        let evidence_links = Cli::try_parse_from([
            "canisend",
            "evidence",
            "association",
            "list",
            "--application",
            "019f3e88-6630-7000-8000-000000000001",
        ])
        .expect("canonical Evidence association list");
        assert!(matches!(
            evidence_links.command,
            Command::Evidence {
                command: EvidenceCommand::Association {
                    command: AssociationCommand::List(_)
                }
            }
        ));

        let host_setup =
            Cli::try_parse_from(["canisend", "host", "setup", "--host", "codex", "--json"])
                .expect("Agent v4 host setup command");
        assert!(matches!(
            host_setup.command,
            Command::Host {
                command: HostCommand::Setup(_)
            }
        ));

        assert!(Cli::try_parse_from(["canisend", "job", "list"]).is_err());
        assert!(Cli::try_parse_from(["canisend", "application", "generic-compose"]).is_err());
        assert_eq!(
            unsupported_legacy_surface(
                ["canisend", "--workspace", "/tmp/legacy", "job", "list"]
                    .into_iter()
                    .map(Into::into)
            ),
            Some("job".to_owned())
        );
        assert_eq!(
            unsupported_legacy_surface(
                ["canisend", "application", "generic-compose"]
                    .into_iter()
                    .map(Into::into)
            ),
            Some("application generic-compose".to_owned())
        );
    }

    #[test]
    fn human_failures_include_stable_code_remediation_and_retry_hint() {
        let mut failure = CommandFailure::new(
            "application.create.commit",
            "stale",
            ErrorCode::WorkspaceConflict,
            "Application input changed",
            true,
        );
        failure.error.remediation = Some(NextAction {
            action: "refresh the Application".to_owned(),
            description: "do not reuse the old candidate".to_owned(),
        });
        assert_eq!(
            human_failure_lines(&failure),
            [
                "canisend [workspace.conflict]: Application input changed",
                "Next: refresh the Application — do not reuse the old candidate",
                "Retryable: yes",
            ]
        );
    }
}
