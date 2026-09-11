#![forbid(unsafe_code)]

mod app_adapter;
mod local_task;

use std::{
    ffi::OsString,
    fs,
    io::{IsTerminal, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use canisend_app::{
    AgentHost, AgentMcpConfigurationRequest, AgentSkillsInstallRequest, AgentSkillsInstallScope,
    Application, ApplicationArchiveRequest, ApplicationError, ApplicationFlowCreateRequestV3,
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
    about = "Prepare, review and export evidence-backed applications",
    after_help = "Examples:\n  canisend -w ./applications ws init --host codex\n  canisend -w ./applications ws upgrade\n  canisend app list\n  canisend app show -a APPLICATION_ID\n\nUse COMMAND --help for details. --json and --text work before or after commands, except mcp serve.",
    disable_version_flag = true
)]
struct Cli {
    /// Workspace directory; otherwise search the current directory and its parents.
    #[arg(short = 'w', long, global = true, value_name = "DIR")]
    workspace: Option<PathBuf>,
    #[command(flatten)]
    output: OutputArgs,
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
    /// Coordinate local workers and collect their draft results.
    #[command(display_order = 12)]
    LocalTask {
        #[command(subcommand)]
        command: local_task::LocalTaskCommand,
    },
    /// Show the CLI version and build details.
    #[command(display_order = 13)]
    Version,
    /// Check installation, bundled resources and PDF rendering.
    #[command(display_order = 14)]
    Doctor,
    /// Connect an AI Host through MCP.
    #[command(display_order = 15)]
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
    /// Find JSON request schemas and their versions.
    #[command(display_order = 16)]
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    /// List bundled templates, Skills and other resources.
    #[command(display_order = 17)]
    Resource {
        #[command(subcommand)]
        command: ResourceCommand,
    },
    /// Create, upgrade and maintain a workspace.
    #[command(visible_alias = "ws")]
    #[command(display_order = 1)]
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    /// Create, view and archive applications.
    #[command(visible_alias = "app")]
    #[command(display_order = 3)]
    Application {
        #[command(subcommand)]
        command: ApplicationCommand,
    },
    /// Import and list reusable profile sources.
    #[command(visible_alias = "source")]
    #[command(display_order = 4)]
    ProfileSource {
        #[command(subcommand)]
        command: ProfileSourceCommand,
    },
    /// View profile sources linked to an application.
    #[command(display_order = 5)]
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    /// View confirmed evidence linked to an application.
    #[command(display_order = 6)]
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    /// List and view application requirements.
    #[command(display_order = 7)]
    Requirement {
        #[command(subcommand)]
        command: RequirementCommand,
    },
    /// View the current document plan.
    #[command(display_order = 8)]
    Plan {
        #[command(subcommand)]
        command: PlanCommand,
    },
    /// List and view document metadata.
    #[command(display_order = 9)]
    Deliverable {
        #[command(subcommand)]
        command: DeliverableCommand,
    },
    /// Review current documents with permission to read them.
    #[command(display_order = 10)]
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    /// Find and verify exported files.
    #[command(display_order = 11)]
    Export {
        #[command(subcommand)]
        command: ExportCommand,
    },
    /// Manage Skills and MCP setup for an AI Host.
    #[command(display_order = 2)]
    Host {
        #[command(subcommand)]
        command: HostCommand,
    },
}

#[derive(Debug, Subcommand)]
enum McpCommand {
    /// Start the MCP server over standard input/output.
    #[command(after_help = "Do not pass --json or --text; stdout is reserved for JSON-RPC.")]
    Serve {
        /// Limit this connection to one existing application.
        #[arg(short = 'a', long, value_name = "ID")]
        application: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum SchemaCommand {
    /// List available schemas.
    List,
    /// Show schema metadata by ID or short name.
    Show(SchemaShowArgs),
}

#[derive(Debug, Subcommand)]
enum ResourceCommand {
    /// List bundled resources and versions.
    List,
}

#[derive(Debug, Subcommand)]
enum WorkspaceCommand {
    /// Create a workspace and optionally install Skills.
    Init(WorkspaceInitArgs),
    /// Update workspace storage and installed project Skills.
    Upgrade(WorkspaceUpgradeArgs),
    /// Show workspace identity, version and application count.
    Status,
    /// Check workspace data and generated files for problems.
    Check,
    /// Create a verified backup in a new directory.
    Backup(WorkspaceBackupArgs),
    /// Restore a backup into a new or empty directory.
    Restore(WorkspaceRestoreArgs),
    /// Restore generated files while preserving user edits.
    Repair,
}

#[derive(Debug, Subcommand)]
enum ApplicationCommand {
    /// List applications and their current states.
    List,
    /// Show an application and what to do next.
    Show(ApplicationIdArgs),
    /// View the application's workflow Pack.
    Pack {
        #[command(subcommand)]
        command: ApplicationPackCommand,
    },
    /// Archive an application while retaining its history.
    Archive(ApplicationArchiveArgs),
    /// Create an application from a JSON file and workflow Pack.
    Create(ApplicationCreateArgs),
}

#[derive(Debug, Subcommand)]
enum ApplicationPackCommand {
    /// Show the workflow Pack and required document types.
    Show(ApplicationIdArgs),
}

#[derive(Debug, Subcommand)]
enum ProfileSourceCommand {
    /// List profile source metadata without reading source content.
    List,
    /// Import a Typst, Markdown, text or JSON profile source.
    Import(ProfileSourceImportArgs),
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    /// View profile source links for an application.
    #[command(visible_alias = "links")]
    Association {
        #[command(subcommand)]
        command: AssociationCommand,
    },
}

#[derive(Debug, Subcommand)]
enum EvidenceCommand {
    /// View confirmed evidence links for an application.
    #[command(visible_alias = "links")]
    Association {
        #[command(subcommand)]
        command: AssociationCommand,
    },
}

#[derive(Debug, Subcommand)]
enum AssociationCommand {
    /// List available records and their application links.
    List(ApplicationIdArgs),
}

#[derive(Debug, Subcommand)]
enum RequirementCommand {
    /// List requirements and confirmation states.
    List(ApplicationIdArgs),
    /// Show a requirement and its confirmation state.
    Show(RequirementIdArgs),
}

#[derive(Debug, Subcommand)]
enum PlanCommand {
    /// Show the document plan and any blockers.
    Show(ApplicationIdArgs),
}

#[derive(Debug, Subcommand)]
enum DeliverableCommand {
    /// List documents and their current states.
    List(ApplicationIdArgs),
    /// Show document metadata without reading its content.
    Show(DeliverableIdArgs),
}

#[derive(Debug, Subcommand)]
enum ReviewCommand {
    /// Read current documents for review; requires private-read consent.
    #[command(visible_alias = "show")]
    Inspect(PrivateApplicationIdArgs),
}

#[derive(Debug, Subcommand)]
enum ExportCommand {
    /// List exported directories and document counts.
    List(ApplicationIdArgs),
    /// Verify an export directory and show its files.
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
    /// Install or update Skills and print the MCP registration command.
    Setup(HostConfigurationArgs),
    /// Show Skills status and MCP setup instructions.
    Status(HostConfigurationArgs),
    /// Remove unmodified CanISend Skills; keep Host configuration.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum AgentSkillsScopeArgument {
    Project,
    Global,
}

impl AgentSkillsScopeArgument {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Global => "global",
        }
    }
}

impl From<AgentSkillsScopeArgument> for AgentSkillsInstallScope {
    fn from(value: AgentSkillsScopeArgument) -> Self {
        match value {
            AgentSkillsScopeArgument::Project => Self::Project,
            AgentSkillsScopeArgument::Global => Self::Global,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Args)]
struct OutputArgs {
    /// Print JSON (also the default when output is piped).
    #[arg(long, global = true, conflicts_with = "text")]
    json: bool,
    /// Print readable text, including when output is piped.
    #[arg(long, global = true, conflicts_with = "json")]
    text: bool,
}

impl OutputArgs {
    fn wants_json(self) -> bool {
        self.json || (!self.text && !std::io::stdout().is_terminal())
    }
}

#[derive(Debug, Args)]
struct SchemaShowArgs {
    /// Schema ID or short name from `schema list`.
    id: String,
}

#[derive(Debug, Args)]
struct WorkspaceBackupArgs {
    /// New directory for the backup.
    #[arg(value_name = "DIR")]
    destination: PathBuf,
}

#[derive(Debug, Args)]
struct WorkspaceInitArgs {
    /// Install Skills for codex, claude or generic.
    #[arg(long, value_enum, conflicts_with = "no_skills")]
    host: Option<HostArgument>,
    /// Skills location; defaults to project when --host is supplied.
    #[arg(long, value_enum, requires = "host")]
    scope: Option<AgentSkillsScopeArgument>,
    /// Create the workspace without prompting to install Skills.
    #[arg(long)]
    no_skills: bool,
}

#[derive(Debug, Args)]
struct WorkspaceUpgradeArgs {
    /// Update or install one Host; defaults to all installed project Skills.
    #[arg(long, value_enum)]
    host: Option<HostArgument>,
}

#[derive(Debug, Args)]
struct WorkspaceRestoreArgs {
    /// Backup directory created by `workspace backup`.
    #[arg(value_name = "BACKUP_DIR")]
    backup: PathBuf,
    /// New or empty directory for the restored workspace.
    #[arg(value_name = "DIR")]
    destination: PathBuf,
}

#[derive(Debug, Args)]
struct ApplicationIdArgs {
    /// Application ID from `app list`.
    #[arg(short = 'a', long, value_name = "ID")]
    application: String,
}

#[derive(Debug, Args)]
struct RequirementIdArgs {
    /// Application ID from `app list`.
    #[arg(short = 'a', long, value_name = "ID")]
    application: String,
    /// Requirement ID from `requirement list`.
    #[arg(long, value_name = "ID")]
    requirement: String,
}

#[derive(Debug, Args)]
struct DeliverableIdArgs {
    /// Application ID from `app list`.
    #[arg(short = 'a', long, value_name = "ID")]
    application: String,
    /// Deliverable ID from `deliverable list`.
    #[arg(long, value_name = "ID")]
    deliverable: String,
}

#[derive(Debug, Args)]
struct PrivateApplicationIdArgs {
    /// Application ID from `app list`.
    #[arg(short = 'a', long, value_name = "ID")]
    application: String,
    /// Allow reading current private documents for this request.
    #[arg(long)]
    confirm_private_read: bool,
}

#[derive(Debug, Args)]
struct ExportShowArgs {
    /// Application ID from `app list`.
    #[arg(short = 'a', long, value_name = "ID")]
    application: String,
    /// Export directory relative to the workspace, from `export list`.
    #[arg(long, value_name = "DIR")]
    destination: String,
}

#[derive(Debug, Args)]
struct ApplicationCreateArgs {
    /// Workflow Pack ID, such as org.canisend.generic-application.
    #[arg(long, value_name = "PACK_ID")]
    pack: String,
    /// JSON application request file.
    #[arg(short = 'f', long, visible_alias = "file", value_name = "JSON_FILE")]
    candidate: PathBuf,
}

#[derive(Debug, Args)]
struct ApplicationArchiveArgs {
    /// Application ID from `app list`.
    #[arg(short = 'a', long, value_name = "ID")]
    application: String,
    /// Current Application revision; rejects changes made since it was read.
    #[arg(long, value_name = "REVISION")]
    expected_revision: u64,
}

#[derive(Debug, Args)]
struct ProfileSourceImportArgs {
    /// Local Typst, Markdown, text or JSON profile source.
    #[arg(value_name = "FILE")]
    source: PathBuf,
    /// Source privacy: public or private-local (requires consent).
    #[arg(long, value_enum)]
    sensitivity: ProfileSourceSensitivityArgument,
    /// Allow reading this private-local source file.
    #[arg(long)]
    confirm_private_read: bool,
}

#[derive(Debug, Args)]
struct HostConfigurationArgs {
    /// Host to configure: codex, claude or generic.
    #[arg(long, value_enum)]
    host: HostArgument,
    /// Skills location: this project or your user home.
    #[arg(long, value_enum, default_value = "project")]
    scope: AgentSkillsScopeArgument,
    /// Absolute CLI path for MCP; defaults to this executable.
    #[arg(long, value_name = "PATH")]
    executable: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct HostRemoveArgs {
    /// Host whose CanISend Skills should be removed.
    #[arg(long, value_enum)]
    host: HostArgument,
    /// Skills location: this project or your user home.
    #[arg(long, value_enum, default_value = "project")]
    scope: AgentSkillsScopeArgument,
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
            OutputArgs {
                json: arguments.iter().any(|arg| arg == "--json"),
                text: arguments.iter().any(|arg| arg == "--text"),
            }
            .wants_json(),
        );
    }
    let cli = Cli::parse_from(arguments);
    // Clap can miss global flag conflicts across different command levels.
    if cli.output.json && cli.output.text {
        Cli::command()
            .error(
                clap::error::ErrorKind::ArgumentConflict,
                "choose one output format: --json or --text",
            )
            .exit();
    }
    if let Command::Mcp {
        command: McpCommand::Serve { application },
    } = &cli.command
    {
        if cli.output.json || cli.output.text {
            Cli::command()
                .error(
                    clap::error::ErrorKind::ArgumentConflict,
                    "mcp serve uses JSON-RPC over stdio; omit --json and --text",
                )
                .exit();
        }
        return match canisend_mcp::serve_stdio_with_application(
            cli.workspace.as_deref(),
            application.as_deref(),
        ) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("canisend mcp serve: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let json_output = cli.output.wants_json();
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
        if argument == "--workspace" || argument == "-w" {
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
            .is_some_and(|command| !matches!(command.as_str(), "association" | "links"))
    {
        return Some(command_path.join(" "));
    }
    if matches!(top_level.as_str(), "application" | "app")
        && command_path
            .get(1)
            .is_some_and(|leaf| leaf.starts_with("generic-"))
    {
        return Some(command_path.join(" "));
    }
    if top_level == "review"
        && command_path
            .get(1)
            .is_some_and(|leaf| !matches!(leaf.as_str(), "inspect" | "show"))
    {
        return Some(command_path.join(" "));
    }
    None
}

fn render_unsupported_legacy_surface(surface: &str, json_output: bool) -> ExitCode {
    let message = format!(
        "unsupported legacy command `{surface}`; use `canisend --help` for current Workspace v4 commands"
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
    let Cli {
        workspace,
        command,
        output,
    } = cli;
    match command {
        Command::LocalTask { command } => local_task::execute(workspace, command),
        Command::Version => version(),
        Command::Doctor => doctor(),
        Command::Mcp {
            command: McpCommand::Serve { .. },
        } => unreachable!("MCP server is dispatched before command rendering"),
        Command::Schema {
            command: SchemaCommand::List,
        } => schema_list(),
        Command::Schema {
            command: SchemaCommand::Show(arguments),
        } => schema_show(&arguments.id),
        Command::Resource {
            command: ResourceCommand::List,
        } => resource_list(),
        Command::Workspace {
            command: WorkspaceCommand::Init(arguments),
        } => workspace_init(workspace, arguments, output),
        Command::Workspace {
            command: WorkspaceCommand::Upgrade(arguments),
        } => workspace_upgrade(workspace, arguments),
        Command::Workspace {
            command: WorkspaceCommand::Status,
        } => workspace_status(workspace),
        Command::Workspace {
            command: WorkspaceCommand::Check,
        } => workspace_check(workspace),
        Command::Workspace {
            command: WorkspaceCommand::Backup(arguments),
        } => workspace_backup(workspace, arguments.destination),
        Command::Workspace {
            command: WorkspaceCommand::Restore(arguments),
        } => workspace_restore(arguments.backup, arguments.destination),
        Command::Workspace {
            command: WorkspaceCommand::Repair,
        } => workspace_repair(workspace),
        Command::Application {
            command: ApplicationCommand::List,
        } => application_list(workspace),
        Command::Application {
            command: ApplicationCommand::Show(arguments),
        } => application_show(workspace, &arguments.application),
        Command::Application {
            command:
                ApplicationCommand::Pack {
                    command: ApplicationPackCommand::Show(arguments),
                },
        } => application_pack_show(workspace, &arguments.application),
        Command::Application {
            command: ApplicationCommand::Archive(arguments),
        } => application_archive(workspace, arguments),
        Command::Application {
            command: ApplicationCommand::Create(arguments),
        } => application_create(workspace, arguments),
        Command::ProfileSource {
            command: ProfileSourceCommand::List,
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
        .map(|schema| format!("{}  {}", schema.id, schema.version))
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
        .map(|resource| format!("{}  {}  [{}]", resource.id, resource.version, resource.kind))
        .collect();
    success("resource.list", "available", &data, human)
}

fn workspace_init(
    workspace_path: Option<PathBuf>,
    arguments: WorkspaceInitArgs,
    output: OutputArgs,
) -> CommandResult<CommandOutput> {
    let root = workspace_path.unwrap_or_else(|| PathBuf::from("."));
    let selection = if let Some(host) = arguments.host {
        Some((
            host,
            arguments.scope.unwrap_or(AgentSkillsScopeArgument::Project),
        ))
    } else if !arguments.no_skills
        && !output.wants_json()
        && std::io::stdout().is_terminal()
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal()
    {
        prompt_init_skills(&root)?
    } else {
        None
    };
    let receipt = Application::initialize_workspace_v4_with_policy(
        &root,
        WorkspaceInitPolicy::PreserveExistingFiles,
    )
    .map(|receipt| (receipt.data.path, receipt.data.status))
    .map_err(|error| app_adapter::failure("workspace.initialize.commit", error))?;
    let (path, data) = receipt;
    let mut output = success(
        "workspace.initialize.commit",
        "initialized",
        &data,
        vec![
            format!("Initialized CanISend Workspace at {}", path.display()),
            format!("Workspace ID: {}", data.workspace_id),
            format!("Workspace format: {}", data.workspace_format),
            "Workflow Packs bind to individual Applications".to_owned(),
        ],
    )?;
    if let Some((host, scope)) = selection {
        let setup = host_setup(Some(path.clone()), HostConfigurationArgs {
            host, scope, executable: None,
        }).map_err(|mut failure| {
            let message = format!("Workspace initialized at {}; Skills setup failed: {}. Use host setup to retry installation.", path.display(), failure.error.message);
            failure.error.message.clone_from(&message);
            failure.human = message;
            failure
        })?;
        output.human.extend(setup.human);
        output.response.data.as_mut().expect("initialization data")["host_setup"] =
            setup.response.data.expect("host setup data");
    } else {
        let guidance = "Skills were not installed by this invocation. Run host setup --host codex (or claude); project scope uses <workspace>/.agents/skills or .claude/skills. Add --scope global for the corresponding directory in your home. Existing installations are preserved.";
        output.response.next_actions.push(NextAction {
            action: "host.setup".to_owned(),
            description: guidance.to_owned(),
        });
    }
    Ok(output)
}

fn workspace_upgrade(
    workspace_path: Option<PathBuf>,
    arguments: WorkspaceUpgradeArgs,
) -> CommandResult<CommandOutput> {
    let operation = "workspace.upgrade";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let data = Application::upgrade_workspace_v4(&root, arguments.host.map(AgentHost::from))
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    let mut human = vec![format!(
        "Workspace ready for CanISend {}: {}",
        data.product_version,
        data.workspace.path.display()
    )];
    for skills in &data.skills {
        human.push(format!(
            "Project Skills ready: {}",
            skills.directory.display()
        ));
    }
    if data.skills.is_empty() {
        human.push(
            "No project Skills installed. Add --host codex (or claude) to install them.".to_owned(),
        );
    }
    human.push("Reconnect your Host to load the updated Skills and tools. If the executable path changed, run host setup for its new MCP registration command. Global Skills use host setup --scope global.".to_owned());
    success(operation, "ready", &data, human)
}

fn prompt_init_skills(
    root: &Path,
) -> CommandResult<Option<(HostArgument, AgentSkillsScopeArgument)>> {
    eprintln!("Optional Skills installation (MCP registration is a separate step):");
    eprintln!("  1. Codex, project: {}/.agents/skills", root.display());
    eprintln!(
        "  2. Claude Code, project: {}/.claude/skills",
        root.display()
    );
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
    if let Some(home) = home.as_ref() {
        eprintln!(
            "  3. Codex, user: {}/.agents/skills",
            Path::new(home).display()
        );
        eprintln!(
            "  4. Claude Code, user: {}/.claude/skills",
            Path::new(home).display()
        );
    }
    eprint!("  0. Skip [default]\nChoose [0-4]: ");
    let read = || -> std::io::Result<String> {
        std::io::stderr().flush()?;
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        Ok(answer)
    };
    let answer = read().map_err(|error| {
        app_adapter::failure(
            "workspace.initialize.commit",
            ApplicationError::InvalidInput(error.to_string()),
        )
    })?;
    let selection = match answer.trim() {
        "" | "0" => None,
        "1" => Some((HostArgument::Codex, AgentSkillsScopeArgument::Project)),
        "2" => Some((HostArgument::Claude, AgentSkillsScopeArgument::Project)),
        "3" if home.is_some() => Some((HostArgument::Codex, AgentSkillsScopeArgument::Global)),
        "4" if home.is_some() => Some((HostArgument::Claude, AgentSkillsScopeArgument::Global)),
        _ => {
            return Err(app_adapter::failure(
                "workspace.initialize.commit",
                ApplicationError::InvalidInput(
                    "Choose one of the listed Skills installation options".to_owned(),
                ),
            ));
        }
    };
    Ok(selection)
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
    let scope = AgentSkillsInstallScope::from(arguments.scope);
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
        scope,
    })
    .map_err(|error| app_adapter::failure(operation, error))?
    .data;
    let skills_directory = skills.directory.clone();
    let registration = mcp
        .registration_command
        .as_deref()
        .unwrap_or("run host setup --host generic --json for the Host configuration snippet");
    let data = json!({
        "host": host,
        "scope": arguments.scope.as_str(),
        "skills": skills,
        "mcp": mcp,
        "mcp_configuration_mutated": false,
    });
    success(
        operation,
        "ready",
        &data,
        vec![
            format!(
                "Skills ready ({}; {})",
                arguments.scope.as_str(),
                host.as_str()
            ),
            format!("Skills directory: {}", skills_directory.display()),
            format!("MCP registration: {registration}"),
            "Run the registration command, then open this workspace in your Host and reconnect."
                .to_owned(),
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
    let scope = AgentSkillsInstallScope::from(arguments.scope);
    let executable = host_executable(arguments.executable, operation)?;
    let skills = Application::agent_skills_status(&AgentSkillsInstallRequest {
        host,
        workspace: root.clone(),
        scope,
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
        "scope": arguments.scope.as_str(),
        "skills": skills,
        "mcp": mcp,
        "mcp_configuration_mutated": false,
    });
    let mut output = success(
        operation,
        status,
        &data,
        vec![
            format!(
                "Skills ({}; {}): {status}",
                arguments.scope.as_str(),
                host.as_str()
            ),
            format!("Skills directory: {}", skills.directory.display()),
            format!(
                "MCP registration: {}",
                mcp.registration_command
                    .as_deref()
                    .unwrap_or("use the configuration returned by --json")
            ),
        ],
    )?;
    let (action, advice) = match skills.state {
        canisend_app::AgentSkillsStatusState::UpToDate => (
            "host.reconnect",
            "Resources match this binary. After an upgrade, reconnect the Host, rediscover tools and discard old previews. MCP connection has not been verified.",
        ),
        canisend_app::AgentSkillsStatusState::NotInstalled
        | canisend_app::AgentSkillsStatusState::UpdateAvailable
        | canisend_app::AgentSkillsStatusState::Incomplete => (
            "host.setup",
            "Install, update or repair bundled Skills using this binary and the same --workspace, --host and --scope. Pause active Host tasks first; setup preserves user-modified files. Then reconnect and rediscover tools.",
        ),
        canisend_app::AgentSkillsStatusState::UserModified
        | canisend_app::AgentSkillsStatusState::Unmanaged => (
            "host.review-conflicts",
            "Preserve and review the conflicting Skills before setup. Do not edit the ownership manifest or force overwrite. Keep custom guidance outside managed files; resolve only the reviewed conflicts.",
        ),
    };
    output.response.next_actions.push(NextAction {
        action: action.to_owned(),
        description: format!("{advice} Selected host: {}; scope: {}; directory: {}. If the executable moved, use the returned MCP registration command.",
            host.as_str(), arguments.scope.as_str(), skills.directory.display()),
    });
    Ok(output)
}

fn host_remove(
    workspace_path: Option<PathBuf>,
    arguments: HostRemoveArgs,
) -> CommandResult<CommandOutput> {
    let operation = "host.remove";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let host = AgentHost::from(arguments.host);
    let scope = AgentSkillsInstallScope::from(arguments.scope);
    let skills = Application::uninstall_agent_skills(&AgentSkillsInstallRequest {
        host,
        workspace: root,
        scope,
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
        "scope": arguments.scope.as_str(),
        "skills": skills,
        "mcp_configuration_removed": false,
    });
    success(
        operation,
        status,
        &data,
        vec![
            format!(
                "Removed {removed_files} unchanged CanISend-managed {} files for {}",
                arguments.scope.as_str(),
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
            std::env::current_exe()
                .and_then(fs::canonicalize)
                .map_err(|error| {
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
    use canisend_contracts::RequirementConfirmationV3;
    let count = |state| {
        stored
            .snapshot
            .requirements
            .iter()
            .filter(|requirement| requirement.confirmation == state)
            .count()
    };
    let proposed = count(RequirementConfirmationV3::Proposed);
    let confirmed = count(RequirementConfirmationV3::Confirmed);
    let excluded = count(RequirementConfirmationV3::Excluded);
    let plan = stored.snapshot.plan.as_ref().map_or_else(
        || "not created".to_owned(),
        |plan| {
            format!(
                "{:?}; decision {}; {} blocker(s)",
                plan.state,
                plan.decision
                    .as_ref()
                    .map_or("not set", |decision| decision.as_str()),
                plan.blockers.len()
            )
        },
    );
    // Navigation from current metadata, not permission or a claim of export readiness.
    let (action, description) = if stored.snapshot.requirements.is_empty() || proposed > 0 {
        (
            "requirement.list",
            "Review the current Requirements and decide only unfinished work through the Host",
        )
    } else if stored.snapshot.plan.is_none() {
        (
            "application.pack.show",
            "Requirement decisions are complete; inspect the Pack catalog and associated Evidence before proposing a Plan",
        )
    } else if stored.snapshot.deliverables.is_empty() {
        (
            "plan.show",
            "Inspect Plan state, decision and blockers; verify associated Evidence before preparing materials",
        )
    } else {
        (
            "deliverable.list",
            "Inspect existing material states before revision or review; local export still requires current readiness and consent",
        )
    };
    let mut human = vec![
        format!("Application: {}", stored.snapshot.opportunity.title),
        format!("Revision: {}", stored.snapshot.application.revision.get()),
        format!("Requirements: {proposed} proposed; {confirmed} confirmed; {excluded} excluded"),
        format!("Plan: {plan}"),
        format!("Deliverables: {}", stored.snapshot.deliverables.len()),
    ];
    human.extend(stored.snapshot.deliverables.iter().map(|item| {
        format!(
            "  {}: {:?} (revision {})",
            item.kind.as_str(),
            item.state,
            item.revision.get()
        )
    }));
    let mut output = success(operation, "current", &stored, human)?;
    output.response.next_actions.push(NextAction {
        action: action.to_owned(),
        description: description.to_owned(),
    });
    Ok(output)
}

fn application_pack_show(
    workspace_path: Option<PathBuf>,
    application_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "application.pack.show";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let manifest = Application::application_pack_manifest_v4(&root, application_id)
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    let mut human = vec![format!("Pack: {} {}", manifest.id, manifest.version)];
    human.extend(manifest.deliverables.kinds.iter().map(|kind| {
        format!(
            "{}: minimum {}, maximum {}",
            kind.id, kind.minimum, kind.maximum
        )
    }));
    success(operation, "current", &manifest, human)
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
            format!(
                "Next: canisend app show -a {}",
                model.stored.snapshot.application.id
            ),
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
            "Source content saved locally; this output shows metadata only".to_owned(),
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
    success(operation, "available", &model, {
        let mut lines = vec![
            format!("Application: {}", model.application_id),
            format!("Workspace Profile Sources: {}", model.profile_sources.len()),
            format!("Explicit links: {}", model.associations.len()),
        ];
        lines.extend(
            model
                .profile_sources
                .iter()
                .map(|source| {
                    format!(
                        "Source: {} [{:?}; revision {}]",
                        source.id,
                        source.kind,
                        source.revision.get()
                    )
                })
                .chain(model.associations.iter().map(|link| {
                    format!(
                        "Linked source: {} [revision {}; stale: {}]",
                        link.profile_source.id,
                        link.profile_source.revision.get(),
                        link.stale
                    )
                })),
        );
        lines
    })
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
    success(operation, "available", &model, {
        let mut lines = vec![
            format!("Application: {}", model.application_id),
            format!("Confirmed Workspace Evidence: {}", model.evidence.len()),
            format!("Explicit links: {}", model.associations.len()),
        ];
        lines.extend(
            model
                .evidence
                .iter()
                .map(|item| {
                    format!(
                        "Evidence: {} [{}; revision {}]",
                        item.evidence.id,
                        item.kind,
                        item.evidence.revision.get()
                    )
                })
                .chain(model.associations.iter().map(|link| {
                    format!(
                        "Linked evidence: {} [revision {}; stale: {}]",
                        link.evidence.id,
                        link.evidence.revision.get(),
                        link.stale
                    )
                })),
        );
        lines
    })
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
    success(operation, &receipt.status, &receipt.data, {
        let mut lines = vec![
            format!("Application: {}", receipt.data.context.application_id),
            format!("Pack: {}", receipt.data.context.pack.id),
            format!("Requirements: {count}"),
        ];
        lines.extend(receipt.data.requirements.iter().map(|item| {
            format!(
                "{}  [{:?}; {:?}] {}",
                item.id, item.confirmation, item.priority, item.statement
            )
        }));
        lines
    })
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
            receipt.data.requirement.statement.clone(),
            format!(
                "State: {:?}; priority: {:?}; revision: {}",
                receipt.data.requirement.confirmation,
                receipt.data.requirement.priority,
                receipt.data.requirement.revision.get()
            ),
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
    success(operation, &receipt.status, &receipt.data, {
        let mut lines = vec![
            format!("Application: {}", receipt.data.context.application_id),
            format!("Pack: {}", receipt.data.context.pack.id),
            state.to_owned(),
        ];
        lines.extend(receipt.data.plan.iter().flat_map(|plan| {
            let mut details = vec![format!(
                "State: {:?}; revision: {}",
                plan.state,
                plan.revision.get()
            )];
            if let Some(decision) = &plan.decision {
                details.push(format!("Decision: {decision}"));
            }
            details.extend(
                plan.deliverables.iter().map(|item| {
                    format!("{} [{:?}]: {}", item.kind, item.disposition, item.rationale)
                }),
            );
            details.extend(
                plan.blockers
                    .iter()
                    .map(|item| format!("{:?}: {}", item.severity, item.description)),
            );
            details
        }));
        lines
    })
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
    success(operation, &receipt.status, &receipt.data, {
        let mut lines = vec![
            format!("Application: {}", receipt.data.context.application_id),
            format!("Pack: {}", receipt.data.context.pack.id),
            format!("Deliverables: {count}"),
        ];
        lines.extend(receipt.data.deliverables.iter().map(|item| {
            format!(
                "{}  {}  [{:?}; revision {}]",
                item.id,
                item.title,
                item.state,
                item.revision.get()
            )
        }));
        lines
    })
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
            format!("Deliverable: {}", receipt.data.deliverable.title),
            format!("ID: {}", receipt.data.deliverable.id),
            format!(
                "Type: {}; state: {:?}; revision: {}",
                receipt.data.deliverable.kind,
                receipt.data.deliverable.state,
                receipt.data.deliverable.revision.get()
            ),
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
    success(operation, &receipt.status, &receipt.data, {
        let mut lines = vec![
            format!("Application: {application_id}"),
            format!("Deliverables reviewed: {}", receipt.data.deliverables.len()),
            "Submission performed: no".to_owned(),
        ];
        lines.extend(receipt.data.deliverables.iter().flat_map(|item| {
            vec![
                format!("\n## {}\n", item.deliverable.title),
                item.content.clone(),
            ]
        }));
        lines
    })
}

fn export_list(
    workspace_path: Option<PathBuf>,
    application_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "export.list";
    let root = app_adapter::workspace_root_v4(workspace_path, operation)?;
    let receipt = Application::list_exports_v4(&root, application_id)
        .map_err(|error| app_adapter::failure(operation, error))?;
    success(operation, &receipt.status, &receipt.data, {
        let mut lines = vec![
            format!("Application: {}", receipt.data.context.application_id),
            format!("Verified local exports: {}", receipt.data.exports.len()),
        ];
        lines.extend(receipt.data.exports.iter().map(|item| {
            format!(
                "{}  [{} documents; revision {}]",
                item.destination,
                item.document_count,
                item.application_revision.get()
            )
        }));
        lines
    })
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
    success(operation, &receipt.status, &receipt.data, {
        let mut lines = vec![
            format!("Destination: {}", receipt.data.manifest.destination),
            format!(
                "Verified documents: {}",
                receipt.data.manifest.documents.len()
            ),
            "Submission performed: no".to_owned(),
        ];
        lines.extend(
            receipt
                .data
                .manifest
                .documents
                .iter()
                .map(|item| format!("{}  [{} pages]", item.relative_path, item.page_count)),
        );
        lines
    })
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
        AgentSkillsScopeArgument, ApplicationCommand, ApplicationPackCommand, AssociationCommand,
        Cli, Command, CommandFailure, EvidenceCommand, ExitClass, HostCommand, ProfileCommand,
        ProfileSourceCommand, WorkspaceCommand, clap_leaf_paths, human_failure_lines,
        public_clap_leaf_paths, unsupported_legacy_surface,
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
        assert_eq!(actual.len(), 40);
    }

    #[test]
    fn canonical_v4_commands_parse_and_legacy_paths_are_preflight_rejected() {
        for flag in ["--candidate", "--file", "-f"] {
            let parsed = Cli::try_parse_from([
                "canisend",
                "--text",
                "app",
                "create",
                "--pack",
                "org.canisend.generic-application",
                flag,
                "request.json",
                "-w",
                "/tmp/canisend-cli-alias",
            ])
            .expect("short commands and candidate aliases");
            assert!(parsed.output.text);
            assert_eq!(
                parsed.workspace.unwrap(),
                std::path::Path::new("/tmp/canisend-cli-alias")
            );
            let Command::Application {
                command: ApplicationCommand::Create(request),
            } = parsed.command
            else {
                panic!("expected application create");
            };
            assert_eq!(request.candidate, std::path::Path::new("request.json"));
        }
        let pack = Cli::try_parse_from([
            "canisend",
            "application",
            "pack",
            "show",
            "--application",
            "019f3e88-6630-7000-8000-000000000001",
            "--json",
        ])
        .expect("Application Pack show command");
        assert!(pack.output.json);
        assert!(matches!(
            pack.command,
            Command::Application {
                command: ApplicationCommand::Pack {
                    command: ApplicationPackCommand::Show(_)
                }
            }
        ));

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
                command: ProfileSourceCommand::List
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
        let Command::Host {
            command: HostCommand::Setup(arguments),
        } = host_setup.command
        else {
            panic!("expected host setup");
        };
        assert_eq!(arguments.scope, AgentSkillsScopeArgument::Project);

        let host_status = Cli::try_parse_from([
            "canisend", "host", "status", "--host", "claude", "--scope", "global",
        ])
        .expect("global Agent v4 host status command");
        let Command::Host {
            command: HostCommand::Status(arguments),
        } = host_status.command
        else {
            panic!("expected host status");
        };
        assert_eq!(arguments.scope, AgentSkillsScopeArgument::Global);

        let host_remove = Cli::try_parse_from([
            "canisend", "host", "remove", "--host", "codex", "--scope", "global",
        ])
        .expect("global Agent v4 host remove command");
        let Command::Host {
            command: HostCommand::Remove(arguments),
        } = host_remove.command
        else {
            panic!("expected host remove");
        };
        assert_eq!(arguments.scope, AgentSkillsScopeArgument::Global);

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
