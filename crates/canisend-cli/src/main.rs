#![forbid(unsafe_code)]

use std::{io::IsTerminal, path::PathBuf, process::ExitCode, str::FromStr};

use canisend_contracts::{
    AGENT_PROTOCOL, ActorKind, AgentContextBlocker, AgentContextData, AgentError, AgentResponse,
    CapabilitiesData, EntityId, ErrorCode, ExecutionMode, ExitClass, NextAction,
    PUBLIC_SCHEMA_VERSION, PrivacyClassification, PublicSchemaId, RESOURCE_FORMAT,
    ResourceCatalogData, ResourceCatalogEntry, SchemaCatalogData, SchemaCatalogEntry,
    SemanticVersion, Sha256Digest, SourceKind, VersionData, WORKSPACE_FORMAT,
};
use canisend_core::{CapabilityRegistry, StageRegistry};
use canisend_io::{
    DiscoveryAdapter, DiscoveryFileKind, GreenhouseAdapter, HttpFetcher, IoAdapterError,
    JobsAcUkAdapter, LeverAdapter, RemoteDocumentKind, RssAtomAdapter,
    discovery_adapter_capabilities, extract_pdf_text, parse_csv_batch, parse_host_agent_batch,
    parse_json_batch, read_discovery_file, read_local_pdf, read_local_text,
    read_task_completion_file, read_task_completion_stdin,
};
use canisend_resources::{ResourceId, ResourceKind};
use canisend_store::{
    AgentContextService, ArtifactService, DiscoveryService, JobService, NewSource, StoreError,
    TaskService, Workspace, current_utc_timestamp,
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::json;

#[derive(Debug, Parser)]
#[command(
    name = "canisend",
    about = "Evidence-backed application preparation",
    disable_version_flag = true
)]
struct Cli {
    /// Resolve commands against this workspace instead of discovering from the current directory.
    #[arg(long, global = true, value_name = "PATH")]
    workspace: Option<PathBuf>,
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
    /// Initialize, inspect, check, back up, restore, or repair a workspace.
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    /// Create, import, inspect, list, or archive jobs.
    Job {
        #[command(subcommand)]
        command: JobCommand,
    },
    /// Import, inspect, compare, or promote discovered job leads.
    Discovery {
        #[command(subcommand)]
        command: DiscoveryCommand,
    },
    /// Prepare, inspect, complete, or cancel bounded agent tasks.
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    /// List compiled capabilities and their availability.
    Capabilities(OutputArgs),
    /// Return the body-free public execution context.
    Context(AgentContextArgs),
}

#[derive(Debug, Subcommand)]
enum SchemaCommand {
    /// List generated schemas with version and integrity metadata.
    List(OutputArgs),
    /// Inspect one generated schema by logical ID or short slug.
    Show(SchemaShowArgs),
}

#[derive(Debug, Subcommand)]
enum ResourceCommand {
    /// List embedded resources with version and integrity metadata.
    List(OutputArgs),
}

#[derive(Debug, Subcommand)]
enum WorkspaceCommand {
    /// Initialize a new v2 workspace at --workspace or the current directory.
    Init(OutputArgs),
    /// Report authoritative workspace and SQLite status.
    Status(OutputArgs),
    /// Verify database, blob, freshness, and projection invariants.
    Check(OutputArgs),
    /// Create and verify a consistent backup directory.
    Backup(WorkspaceBackupArgs),
    /// Restore a verified backup into a new empty directory.
    Restore(WorkspaceRestoreArgs),
    /// Repair derived projections marked repair-required.
    Repair(OutputArgs),
}

#[derive(Debug, Subcommand)]
enum JobCommand {
    /// Create an empty job record ready for one or more sources.
    Create(JobCreateArgs),
    /// Import a supported local source into an active job.
    Import(JobImportArgs),
    /// List active jobs, or include archived jobs explicitly.
    List(JobListArgs),
    /// Show one job and its body-free source metadata.
    Show(JobIdArgs),
    /// Archive a job without deleting its history.
    Archive(JobIdArgs),
}

#[derive(Debug, Subcommand)]
enum DiscoveryCommand {
    /// Validate or commit a normalized CSV, JSON, or host-agent lead batch.
    Import(DiscoveryImportArgs),
    /// List the compiled discovery adapter registry and its limits.
    Adapters(OutputArgs),
    /// Fetch and optionally commit one configured public discovery source.
    Refresh(DiscoveryRefreshArgs),
    /// List registered discovery sources.
    Sources(OutputArgs),
    /// List active leads, or include preserved history explicitly.
    List(DiscoveryListArgs),
    /// Show one discovery lead.
    Show(DiscoveryIdArgs),
    /// Suggest bounded possible duplicates without merging records.
    Suggest(DiscoverySuggestArgs),
    /// Create a direct-intake job from a selected lead.
    Promote(DiscoveryIdArgs),
}

#[derive(Debug, Subcommand)]
enum TaskCommand {
    /// Freeze exact source revisions and create a leased host-agent task.
    Prepare(TaskPrepareArgs),
    /// Inspect a task descriptor, state, and committed result metadata.
    Show(TaskIdArgs),
    /// Validate and atomically commit a host-agent completion request.
    Complete(TaskCompleteArgs),
    /// Cancel a prepared task without deleting its audit history.
    Cancel(TaskIdArgs),
}

#[derive(Debug, Args)]
struct OutputArgs {
    /// Emit exactly one canisend.agent/v2 JSON object on stdout.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct AgentContextArgs {
    /// Select one job for body-free stage blockers and next actions.
    #[arg(long)]
    job: Option<String>,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct SchemaShowArgs {
    /// Schema ID such as canisend.job/v2, or its short slug such as job.
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
struct WorkspaceRestoreArgs {
    /// Verified CanISend backup directory.
    backup: PathBuf,
    /// New or empty destination directory for the restored workspace.
    destination: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct JobCreateArgs {
    #[arg(long)]
    title: String,
    #[arg(long)]
    institution: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct JobImportArgs {
    /// Canonical UUIDv7 job ID.
    job_id: String,
    /// UTF-8 Markdown or plain-text job advert.
    #[arg(
        long,
        value_name = "PATH",
        required_unless_present = "url",
        conflicts_with = "url"
    )]
    file: Option<PathBuf>,
    /// Public HTTP(S) job advert URL fetched with SSRF-safe redirect handling.
    #[arg(
        long,
        value_name = "URL",
        required_unless_present = "file",
        conflicts_with = "file"
    )]
    url: Option<String>,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct JobListArgs {
    #[arg(long)]
    include_archived: bool,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct JobIdArgs {
    /// Canonical UUIDv7 job ID.
    job_id: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct DiscoveryImportArgs {
    /// Regular .csv or .json file containing normalized job leads.
    #[arg(long, value_name = "PATH")]
    file: PathBuf,
    /// Explicit source label for CSV imports; defaults to the file stem.
    #[arg(long)]
    source_name: Option<String>,
    /// Public source URL recorded for CSV provenance.
    #[arg(long)]
    source_url: Option<String>,
    /// Require the JSON batch to identify itself as a host-agent result.
    #[arg(long)]
    host_agent: bool,
    /// Validate and report rows without opening or changing a workspace.
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DiscoveryAdapterName {
    RssAtom,
    JobsAcUk,
    Greenhouse,
    Lever,
}

#[derive(Debug, Args)]
struct DiscoveryRefreshArgs {
    /// Compiled public source adapter.
    #[arg(long, value_enum)]
    adapter: DiscoveryAdapterName,
    /// Public read-only RSS, Atom, Greenhouse, or Lever endpoint.
    #[arg(long)]
    endpoint: String,
    /// Source label; for Greenhouse and Lever this is the hiring organization.
    #[arg(long)]
    source_name: String,
    /// Explicit organization fallback for feed entries that omit an author.
    #[arg(long)]
    organization: Option<String>,
    /// Fetch and validate without changing the workspace.
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct DiscoveryListArgs {
    #[arg(long)]
    include_history: bool,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct DiscoveryIdArgs {
    /// Canonical UUIDv7 discovery lead ID.
    lead_id: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct DiscoverySuggestArgs {
    /// Canonical UUIDv7 discovery lead ID.
    lead_id: String,
    /// Maximum suggestions to return, clamped to the safe range 1..=20.
    #[arg(long, default_value_t = 5)]
    limit: usize,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TaskOperationName {
    JobCriterion,
}

#[derive(Debug, Args)]
struct TaskPrepareArgs {
    /// Canonical UUIDv7 job ID whose current source revisions become task inputs.
    #[arg(long)]
    job: String,
    /// Bounded operation implemented by the compiled task registry.
    #[arg(long, value_enum)]
    operation: TaskOperationName,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct TaskIdArgs {
    /// Canonical UUIDv7 task ID.
    task_id: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct TaskCompleteArgs {
    /// Regular, non-symlink JSON file containing canisend.task-completion/v2.
    #[arg(
        long,
        value_name = "PATH",
        required_unless_present = "stdin",
        conflicts_with = "stdin"
    )]
    file: Option<PathBuf>,
    /// Read one bounded canisend.task-completion/v2 object from standard input.
    #[arg(long, required_unless_present = "file", conflicts_with = "file")]
    stdin: bool,
    #[command(flatten)]
    output: OutputArgs,
}

impl Cli {
    fn explicit_json(&self) -> bool {
        match &self.command {
            Command::Version(output) | Command::Doctor(output) => output.json,
            Command::Agent {
                command: AgentCommand::Capabilities(output),
            }
            | Command::Schema {
                command: SchemaCommand::List(output),
            }
            | Command::Resource {
                command: ResourceCommand::List(output),
            } => output.json,
            Command::Agent {
                command: AgentCommand::Context(arguments),
            } => arguments.output.json,
            Command::Schema {
                command: SchemaCommand::Show(arguments),
            } => arguments.output.json,
            Command::Workspace {
                command:
                    WorkspaceCommand::Init(output)
                    | WorkspaceCommand::Status(output)
                    | WorkspaceCommand::Check(output)
                    | WorkspaceCommand::Repair(output),
            } => output.json,
            Command::Workspace {
                command: WorkspaceCommand::Backup(arguments),
            } => arguments.output.json,
            Command::Workspace {
                command: WorkspaceCommand::Restore(arguments),
            } => arguments.output.json,
            Command::Job { command } => match command {
                JobCommand::Create(arguments) => arguments.output.json,
                JobCommand::Import(arguments) => arguments.output.json,
                JobCommand::List(arguments) => arguments.output.json,
                JobCommand::Show(arguments) | JobCommand::Archive(arguments) => {
                    arguments.output.json
                }
            },
            Command::Discovery { command } => match command {
                DiscoveryCommand::Import(arguments) => arguments.output.json,
                DiscoveryCommand::Adapters(output) | DiscoveryCommand::Sources(output) => {
                    output.json
                }
                DiscoveryCommand::Refresh(arguments) => arguments.output.json,
                DiscoveryCommand::List(arguments) => arguments.output.json,
                DiscoveryCommand::Show(arguments) | DiscoveryCommand::Promote(arguments) => {
                    arguments.output.json
                }
                DiscoveryCommand::Suggest(arguments) => arguments.output.json,
            },
            Command::Task { command } => match command {
                TaskCommand::Prepare(arguments) => arguments.output.json,
                TaskCommand::Show(arguments) | TaskCommand::Cancel(arguments) => {
                    arguments.output.json
                }
                TaskCommand::Complete(arguments) => arguments.output.json,
            },
        }
    }
}

struct CommandOutput {
    response: AgentResponse,
    human: Vec<String>,
}

struct CommandFailure {
    operation: &'static str,
    status: &'static str,
    error: AgentError,
    human: String,
}

type CommandResult<T> = Result<T, Box<CommandFailure>>;

impl CommandFailure {
    fn new(
        operation: &'static str,
        status: &'static str,
        code: ErrorCode,
        message: impl Into<String>,
        retryable: bool,
    ) -> Box<Self> {
        let message = message.into();
        Box::new(Self {
            operation,
            status,
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

    fn response(&self) -> AgentResponse {
        AgentResponse::failure(self.operation, self.status, self.error.clone())
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json_output = wants_json(cli.explicit_json());
    match execute(cli) {
        Ok(output) => render_success(output, json_output),
        Err(failure) => render_failure(*failure, json_output),
    }
}

fn execute(cli: Cli) -> CommandResult<CommandOutput> {
    let Cli { workspace, command } = cli;
    match command {
        Command::Version(_) => version(),
        Command::Doctor(_) => doctor(),
        Command::Agent {
            command: AgentCommand::Capabilities(_),
        } => capabilities(),
        Command::Agent {
            command: AgentCommand::Context(arguments),
        } => context(workspace, arguments.job.as_deref()),
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
        Command::Job {
            command: JobCommand::Create(arguments),
        } => job_create(workspace, arguments),
        Command::Job {
            command: JobCommand::Import(arguments),
        } => job_import(workspace, arguments),
        Command::Job {
            command: JobCommand::List(arguments),
        } => job_list(workspace, arguments.include_archived),
        Command::Job {
            command: JobCommand::Show(arguments),
        } => job_show(workspace, &arguments.job_id),
        Command::Job {
            command: JobCommand::Archive(arguments),
        } => job_archive(workspace, &arguments.job_id),
        Command::Discovery {
            command: DiscoveryCommand::Import(arguments),
        } => discovery_import(workspace, arguments),
        Command::Discovery {
            command: DiscoveryCommand::Adapters(_),
        } => discovery_adapters(),
        Command::Discovery {
            command: DiscoveryCommand::Refresh(arguments),
        } => discovery_refresh(workspace, arguments),
        Command::Discovery {
            command: DiscoveryCommand::Sources(_),
        } => discovery_sources(workspace),
        Command::Discovery {
            command: DiscoveryCommand::List(arguments),
        } => discovery_list(workspace, arguments.include_history),
        Command::Discovery {
            command: DiscoveryCommand::Show(arguments),
        } => discovery_show(workspace, &arguments.lead_id),
        Command::Discovery {
            command: DiscoveryCommand::Suggest(arguments),
        } => discovery_suggest(workspace, &arguments.lead_id, arguments.limit),
        Command::Discovery {
            command: DiscoveryCommand::Promote(arguments),
        } => discovery_promote(workspace, &arguments.lead_id),
        Command::Task {
            command: TaskCommand::Prepare(arguments),
        } => task_prepare(workspace, arguments),
        Command::Task {
            command: TaskCommand::Show(arguments),
        } => task_show(workspace, &arguments.task_id),
        Command::Task {
            command: TaskCommand::Complete(arguments),
        } => task_complete(workspace, arguments),
        Command::Task {
            command: TaskCommand::Cancel(arguments),
        } => task_cancel(workspace, &arguments.task_id),
    }
}

fn version() -> CommandResult<CommandOutput> {
    let data = VersionData {
        product: "canisend".to_owned(),
        version: product_version()?,
        protocol: AGENT_PROTOCOL.to_owned(),
        workspace_format: WORKSPACE_FORMAT.to_owned(),
        resource_format: RESOURCE_FORMAT.to_owned(),
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
    canisend_resources::verify().map_err(|message| {
        CommandFailure::new(
            "product.doctor",
            "unhealthy",
            ErrorCode::ResourcesIntegrityFailed,
            message,
            false,
        )
    })?;
    let data = json!({
        "resource_manifest": "verified",
        "resource_count": canisend_resources::manifest().len(),
        "schema_count": PublicSchemaId::ALL.len(),
        "python_required": false,
    });
    Ok(CommandOutput {
        response: AgentResponse::success("product.doctor", "healthy", data),
        human: vec![
            "CanISend native foundation: healthy".to_owned(),
            "Embedded resources: verified".to_owned(),
            "Generated schemas: verified".to_owned(),
            "Python runtime: not required".to_owned(),
        ],
    })
}

fn capabilities() -> CommandResult<CommandOutput> {
    let data = CapabilitiesData {
        product_version: product_version()?,
        protocol: AGENT_PROTOCOL.to_owned(),
        workspace_format: WORKSPACE_FORMAT.to_owned(),
        resource_format: RESOURCE_FORMAT.to_owned(),
        capabilities: CapabilityRegistry::built_in(),
        stages: StageRegistry::built_in(),
        discovery_adapters: discovery_adapter_capabilities(),
        error_codes: ErrorCode::ALL
            .into_iter()
            .map(|code| code.as_str().to_owned())
            .collect(),
    };
    let human = std::iter::once(format!("CanISend {} capabilities", data.product_version))
        .chain(
            data.capabilities
                .iter()
                .map(|capability| format!("{}: {:?}", capability.id, capability.status)),
        )
        .collect();
    success("agent.capabilities", "available", &data, human)
}

fn context(
    workspace_path: Option<PathBuf>,
    selected_job_id: Option<&str>,
) -> CommandResult<CommandOutput> {
    let workspace = if workspace_path.is_some() {
        Some(open_workspace(workspace_path, "agent.context")?)
    } else {
        match Workspace::open(None) {
            Ok(workspace) => Some(workspace),
            Err(StoreError::WorkspaceNotFound(_)) => None,
            Err(error) => return Err(store_failure("agent.context", error)),
        }
    };
    let mut blockers = Vec::new();
    let mut next_actions = Vec::new();
    let mut workspace_summary = None;
    let mut selected_job = None;
    if let Some(workspace) = &workspace {
        let service = AgentContextService::new(&workspace.database);
        let summary = service
            .workspace_summary()
            .map_err(|error| store_failure("agent.context", error))?;
        if let Some(job_id) = selected_job_id {
            let job_id = parse_entity_id("agent.context", job_id)?;
            let job = service
                .job_summary(&job_id)
                .map_err(|error| store_failure("agent.context", error))?;
            if job.archived {
                blockers.push(AgentContextBlocker {
                    code: "job.archived".to_owned(),
                    description: "The selected job is archived".to_owned(),
                    subject_id: Some(job.id.clone()),
                });
            } else if job.source_count == 0 {
                blockers.push(AgentContextBlocker {
                    code: "job.source_missing".to_owned(),
                    description: "The selected job has no imported advert source".to_owned(),
                    subject_id: Some(job.id.clone()),
                });
                next_actions.push(NextAction {
                    action: format!("canisend job import {} --file PATH", job.id),
                    description: "Import a local advert, PDF, or use --url before preparing work"
                        .to_owned(),
                });
            } else {
                next_actions.push(NextAction {
                    action: format!("canisend job show {} --json", job.id),
                    description: "Inspect body-free source and revision metadata".to_owned(),
                });
            }
            selected_job = Some(job);
        } else if summary.active_job_count > 0 {
            blockers.push(AgentContextBlocker {
                code: "job.not_selected".to_owned(),
                description: "Select an active job with agent context --job JOB_ID".to_owned(),
                subject_id: None,
            });
            next_actions.push(NextAction {
                action: "canisend job list --json".to_owned(),
                description: "Choose one active job for the next workflow operation".to_owned(),
            });
        } else if summary.active_lead_count > 0 {
            blockers.push(AgentContextBlocker {
                code: "job.missing".to_owned(),
                description: "Promote a discovery lead before preparing application work"
                    .to_owned(),
                subject_id: None,
            });
            next_actions.push(NextAction {
                action: "canisend discovery list --json".to_owned(),
                description: "Select and promote an active discovery lead".to_owned(),
            });
        } else {
            blockers.push(AgentContextBlocker {
                code: "job.missing".to_owned(),
                description: "Create or discover a job before preparing application work"
                    .to_owned(),
                subject_id: None,
            });
            next_actions.push(NextAction {
                action: "canisend job create --title TITLE --institution INSTITUTION --json"
                    .to_owned(),
                description: "Create a direct-intake job or import discovery leads".to_owned(),
            });
        }
        workspace_summary = Some(summary);
    } else {
        blockers.push(AgentContextBlocker {
            code: "workspace.not_selected".to_owned(),
            description: "No CanISend workspace was discovered or selected".to_owned(),
            subject_id: None,
        });
        next_actions.push(NextAction {
            action: "canisend --workspace PATH workspace init --json".to_owned(),
            description: "Initialize or explicitly select a workspace".to_owned(),
        });
    }
    let data = AgentContextData {
        product_version: product_version()?,
        protocol: AGENT_PROTOCOL.to_owned(),
        workspace_format: WORKSPACE_FORMAT.to_owned(),
        resource_format: RESOURCE_FORMAT.to_owned(),
        actor: ActorKind::HostAgent,
        execution_mode: ExecutionMode::HostAgent,
        workspace_id: workspace_summary
            .as_ref()
            .map(|summary| summary.workspace_id.clone()),
        active_job_id: selected_job.as_ref().map(|job| job.id.clone()),
        workspace: workspace_summary,
        selected_job,
        supported_stages: StageRegistry::built_in(),
        blockers,
        next_actions,
        privacy: PrivacyClassification::Public,
    };
    let mut output = success(
        "agent.context",
        "available",
        &data,
        vec![
            "CanISend body-free agent context".to_owned(),
            format!(
                "Workspace: {}",
                data.workspace_id
                    .as_ref()
                    .map_or("not selected", EntityId::as_str)
            ),
            format!("Blockers: {}", data.blockers.len()),
            "Privacy: public metadata only".to_owned(),
        ],
    )?;
    output.response.next_actions = data.next_actions.clone();
    Ok(output)
}

fn schema_list() -> CommandResult<CommandOutput> {
    let schemas = PublicSchemaId::ALL
        .into_iter()
        .map(schema_catalog_entry)
        .collect::<CommandResult<Vec<_>>>()?;
    let human = schemas
        .iter()
        .map(|schema| format!("{} {}", schema.id, schema.sha256))
        .collect();
    success(
        "schema.list",
        "available",
        &SchemaCatalogData { schemas },
        human,
    )
}

fn schema_show(query: &str) -> CommandResult<CommandOutput> {
    let schema_id = PublicSchemaId::ALL
        .into_iter()
        .find(|schema_id| schema_id.as_str() == query || schema_id.slug() == query)
        .ok_or_else(|| {
            CommandFailure::new(
                "schema.show",
                "not-found",
                ErrorCode::SchemaNotFound,
                format!("unknown public schema: {query}"),
                false,
            )
        })?;
    let schema = schema_catalog_entry(schema_id)?;
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

fn schema_catalog_entry(schema_id: PublicSchemaId) -> CommandResult<SchemaCatalogEntry> {
    let resource_id =
        ResourceId::from_str(&format!("schema.{}", schema_id.slug())).map_err(|error| {
            CommandFailure::new(
                "schema.catalog",
                "unavailable",
                ErrorCode::InternalInvariantFailed,
                error.to_string(),
                false,
            )
        })?;
    let descriptor = canisend_resources::get(resource_id).descriptor;
    Ok(SchemaCatalogEntry {
        id: schema_id.as_str().to_owned(),
        version: SemanticVersion::try_new(PUBLIC_SCHEMA_VERSION).map_err(internal_version)?,
        uri: schema_id.canonical_uri(),
        resource_id: resource_id.as_str().to_owned(),
        size: descriptor.size,
        sha256: Sha256Digest::try_new(descriptor.sha256).map_err(internal_version)?,
    })
}

fn resource_list() -> CommandResult<CommandOutput> {
    let resources = canisend_resources::manifest()
        .into_iter()
        .map(|resource| {
            Ok(ResourceCatalogEntry {
                id: resource.id.to_owned(),
                kind: resource_kind_name(resource.kind).to_owned(),
                version: SemanticVersion::try_new(resource.version).map_err(internal_version)?,
                size: resource.size,
                sha256: Sha256Digest::try_new(resource.sha256).map_err(internal_version)?,
            })
        })
        .collect::<CommandResult<Vec<_>>>()?;
    let human = resources
        .iter()
        .map(|resource| format!("{} [{}]", resource.id, resource.kind))
        .collect();
    success(
        "resource.list",
        "available",
        &ResourceCatalogData { resources },
        human,
    )
}

fn workspace_init(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let root = workspace_path.unwrap_or_else(|| PathBuf::from("."));
    let workspace =
        Workspace::init(&root).map_err(|error| store_failure("workspace.init", error))?;
    let data = workspace
        .status()
        .map_err(|error| store_failure("workspace.init", error))?;
    success(
        "workspace.init",
        "initialized",
        &data,
        vec![
            format!(
                "Initialized CanISend workspace at {}",
                workspace.paths.root.display()
            ),
            format!("Workspace ID: {}", data.workspace_id),
        ],
    )
}

fn workspace_status(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let workspace = open_workspace(workspace_path, "workspace.status")?;
    let data = workspace
        .status()
        .map_err(|error| store_failure("workspace.status", error))?;
    success(
        "workspace.status",
        "available",
        &data,
        vec![
            format!("Workspace: {}", data.workspace_id),
            format!("Format: {}", data.workspace_format),
            format!("SQLite: {} ({})", data.sqlite_version, data.journal_mode),
            format!("Artifacts: {}", data.artifact_count),
        ],
    )
}

fn workspace_check(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let workspace = open_workspace(workspace_path, "workspace.check")?;
    let data = workspace
        .check()
        .map_err(|error| store_failure("workspace.check", error))?;
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
    let mut workspace = open_workspace(workspace_path, "workspace.backup")?;
    let result = workspace
        .backup(&destination)
        .map_err(|error| store_failure("workspace.backup", error))?;
    success(
        "workspace.backup",
        "verified",
        &result.manifest,
        vec![
            format!("Verified backup: {}", result.directory.display()),
            format!("Blobs: {}", result.manifest.blobs.len()),
        ],
    )
}

fn workspace_restore(backup: PathBuf, destination: PathBuf) -> CommandResult<CommandOutput> {
    let workspace = Workspace::restore(&backup, &destination)
        .map_err(|error| store_failure("workspace.restore", error))?;
    let data = workspace
        .status()
        .map_err(|error| store_failure("workspace.restore", error))?;
    success(
        "workspace.restore",
        "restored",
        &data,
        vec![
            format!("Restored workspace at {}", destination.display()),
            format!("Workspace ID: {}", data.workspace_id),
        ],
    )
}

fn workspace_repair(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let mut workspace = open_workspace(workspace_path, "workspace.repair")?;
    let repaired = {
        let mut service = ArtifactService::new(
            &mut workspace.database,
            &workspace.blobs,
            &workspace.paths.root,
        );
        service
            .repair_projections()
            .map_err(|error| store_failure("workspace.repair", error))?
    };
    success(
        "workspace.repair",
        "repaired",
        &json!({"repaired_projections": repaired}),
        vec![format!("Repaired projections: {repaired}")],
    )
}

fn job_create(
    workspace_path: Option<PathBuf>,
    arguments: JobCreateArgs,
) -> CommandResult<CommandOutput> {
    let mut workspace = open_workspace(workspace_path, "job.create")?;
    let record = JobService::new(&mut workspace.database, &workspace.blobs)
        .create(&arguments.title, &arguments.institution, ActorKind::User)
        .map_err(|error| store_failure("job.create", error))?;
    success(
        "job.create",
        "created",
        &record,
        vec![
            format!("Created job: {}", record.id),
            format!("{} — {}", record.title, record.institution),
        ],
    )
}

fn job_import(
    workspace_path: Option<PathBuf>,
    arguments: JobImportArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("job.import", &arguments.job_id)?;
    let source = if let Some(path) = arguments.file {
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            let document =
                read_local_pdf(&path).map_err(|error| io_adapter_failure("job.import", error))?;
            NewSource {
                kind: SourceKind::LocalFile,
                original_bytes: document.original_bytes,
                normalized_text: document.normalized_text,
                source_url: None,
                final_url: None,
                content_type: "application/pdf".to_owned(),
                redirect_chain: Vec::new(),
                privacy: PrivacyClassification::PrivateLocal,
            }
        } else {
            let document =
                read_local_text(&path).map_err(|error| io_adapter_failure("job.import", error))?;
            NewSource {
                kind: SourceKind::LocalFile,
                original_bytes: document.original_bytes,
                normalized_text: document.normalized_text,
                source_url: None,
                final_url: None,
                content_type: document.content_type.to_owned(),
                redirect_chain: Vec::new(),
                privacy: PrivacyClassification::PrivateLocal,
            }
        }
    } else if let Some(url) = arguments.url {
        let document = HttpFetcher::new()
            .fetch(&url)
            .map_err(|error| io_adapter_failure("job.import", error))?;
        let normalized_text = if document.kind == RemoteDocumentKind::Pdf {
            extract_pdf_text(document.original_bytes.clone())
                .map_err(|error| io_adapter_failure("job.import", error))?
                .normalized_text
        } else {
            document
                .normalized_text
                .ok_or_else(|| io_adapter_failure("job.import", IoAdapterError::TextUnavailable))?
        };
        NewSource {
            kind: SourceKind::UserUrl,
            original_bytes: document.original_bytes,
            normalized_text,
            source_url: Some(document.source_url),
            final_url: Some(document.final_url),
            content_type: document.content_type,
            redirect_chain: document.redirect_chain,
            privacy: PrivacyClassification::PrivateLocal,
        }
    } else {
        return Err(CommandFailure::new(
            "job.import",
            "invalid",
            ErrorCode::InputInvalid,
            "exactly one of --file or --url is required",
            false,
        ));
    };
    let mut workspace = open_workspace(workspace_path, "job.import")?;
    let record = JobService::new(&mut workspace.database, &workspace.blobs)
        .import_source(&job_id, source, ActorKind::User)
        .map_err(|error| store_failure("job.import", error))?;
    success(
        "job.import",
        "imported",
        &record,
        vec![
            format!("Imported source: {}", record.id),
            format!("Job: {}", record.job_id),
            format!("Original: {}", record.original.sha256),
            format!(
                "Normalized: {}",
                record
                    .normalized_text
                    .as_ref()
                    .map(|reference| reference.sha256.as_str())
                    .unwrap_or("unavailable")
            ),
        ],
    )
}

fn job_list(
    workspace_path: Option<PathBuf>,
    include_archived: bool,
) -> CommandResult<CommandOutput> {
    let mut workspace = open_workspace(workspace_path, "job.list")?;
    let records = JobService::new(&mut workspace.database, &workspace.blobs)
        .list(include_archived)
        .map_err(|error| store_failure("job.list", error))?;
    let human = if records.is_empty() {
        vec!["No jobs found".to_owned()]
    } else {
        records
            .iter()
            .map(|record| {
                format!(
                    "{}  {} — {}{}",
                    record.id,
                    record.title,
                    record.institution,
                    if record.archived { " [archived]" } else { "" }
                )
            })
            .collect()
    };
    success("job.list", "available", &json!({"jobs": records}), human)
}

fn job_show(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("job.show", job_id)?;
    let mut workspace = open_workspace(workspace_path, "job.show")?;
    let service = JobService::new(&mut workspace.database, &workspace.blobs);
    let record = service
        .get(&job_id)
        .map_err(|error| store_failure("job.show", error))?;
    let sources = service
        .sources(&job_id)
        .map_err(|error| store_failure("job.show", error))?;
    let data = json!({"job": record, "sources": sources});
    success(
        "job.show",
        "available",
        &data,
        vec![
            format!("{} — {}", record.title, record.institution),
            format!("Job ID: {}", record.id),
            format!("Sources: {}", sources.len()),
            format!("Archived: {}", record.archived),
        ],
    )
}

fn job_archive(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("job.archive", job_id)?;
    let mut workspace = open_workspace(workspace_path, "job.archive")?;
    let record = JobService::new(&mut workspace.database, &workspace.blobs)
        .archive(&job_id, ActorKind::User)
        .map_err(|error| store_failure("job.archive", error))?;
    success(
        "job.archive",
        "archived",
        &record,
        vec![format!("Archived job: {}", record.id)],
    )
}

fn discovery_import(
    workspace_path: Option<PathBuf>,
    arguments: DiscoveryImportArgs,
) -> CommandResult<CommandOutput> {
    let document = read_discovery_file(&arguments.file)
        .map_err(|error| io_adapter_failure("discovery.import", error))?;
    let actor = if arguments.host_agent {
        ActorKind::HostAgent
    } else {
        ActorKind::User
    };
    let report = match document.kind {
        DiscoveryFileKind::Csv => {
            if arguments.host_agent {
                return Err(CommandFailure::new(
                    "discovery.import",
                    "invalid",
                    ErrorCode::InputInvalid,
                    "--host-agent requires a JSON batch",
                    false,
                ));
            }
            let source_name = arguments.source_name.unwrap_or_else(|| {
                document
                    .path
                    .file_stem()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "CSV import".to_owned())
            });
            let observed_at = current_utc_timestamp()
                .map_err(|error| store_failure("discovery.import", error))?;
            parse_csv_batch(
                &document.bytes,
                &source_name,
                arguments.source_url.as_deref(),
                observed_at,
            )
            .map_err(|error| io_adapter_failure("discovery.import", error))?
        }
        DiscoveryFileKind::Json => {
            if arguments.source_name.is_some() || arguments.source_url.is_some() {
                return Err(CommandFailure::new(
                    "discovery.import",
                    "invalid",
                    ErrorCode::InputInvalid,
                    "JSON batches declare source_name and source_url inside the versioned contract",
                    false,
                ));
            }
            if arguments.host_agent {
                parse_host_agent_batch(&document.bytes)
            } else {
                parse_json_batch(&document.bytes)
            }
            .map_err(|error| io_adapter_failure("discovery.import", error))?
        }
    };
    let report = if arguments.dry_run {
        report
    } else {
        let mut workspace = open_workspace(workspace_path, "discovery.import")?;
        DiscoveryService::new(&mut workspace.database)
            .import_report(report, actor)
            .map_err(|error| store_failure("discovery.import", error))?
    };
    let status = if report.dry_run {
        "validated"
    } else {
        "imported"
    };
    success(
        "discovery.import",
        status,
        &report,
        vec![
            format!("Discovery batch: {status}"),
            format!("Accepted leads: {}", report.accepted),
            format!("Rejected rows: {}", report.rejected),
        ],
    )
}

fn discovery_adapters() -> CommandResult<CommandOutput> {
    let adapters = discovery_adapter_capabilities();
    let human = adapters
        .iter()
        .map(|adapter| {
            format!(
                "{:?}: network={}, cursor={}, max_items={}",
                adapter.kind,
                adapter.network,
                adapter.supports_cursor,
                adapter.max_items_per_refresh
            )
        })
        .collect();
    success(
        "discovery.adapters",
        "available",
        &json!({"adapters": adapters}),
        human,
    )
}

fn discovery_refresh(
    workspace_path: Option<PathBuf>,
    arguments: DiscoveryRefreshArgs,
) -> CommandResult<CommandOutput> {
    let DiscoveryRefreshArgs {
        adapter,
        endpoint,
        source_name,
        organization,
        dry_run,
        output: _,
    } = arguments;
    let adapter: Box<dyn DiscoveryAdapter> = match adapter {
        DiscoveryAdapterName::RssAtom => Box::new(RssAtomAdapter::new(source_name, organization)),
        DiscoveryAdapterName::JobsAcUk => Box::new(JobsAcUkAdapter::new(organization)),
        DiscoveryAdapterName::Greenhouse => Box::new(GreenhouseAdapter::new(source_name)),
        DiscoveryAdapterName::Lever => Box::new(LeverAdapter::new(source_name)),
    };
    let observed_at =
        current_utc_timestamp().map_err(|error| store_failure("discovery.refresh", error))?;
    let report = adapter
        .refresh(&HttpFetcher::new(), &endpoint, observed_at)
        .map_err(|error| io_adapter_failure("discovery.refresh", error))?;
    let report = if dry_run {
        report
    } else {
        let mut workspace = open_workspace(workspace_path, "discovery.refresh")?;
        DiscoveryService::new(&mut workspace.database)
            .import_report(report, ActorKind::User)
            .map_err(|error| store_failure("discovery.refresh", error))?
    };
    let status = if report.dry_run {
        "validated"
    } else {
        "refreshed"
    };
    success(
        "discovery.refresh",
        status,
        &report,
        vec![
            format!("{} source: {status}", adapter.id()),
            format!("Accepted leads: {}", report.accepted),
            format!("Rejected rows: {}", report.rejected),
        ],
    )
}

fn discovery_sources(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let mut workspace = open_workspace(workspace_path, "discovery.sources")?;
    let sources = DiscoveryService::new(&mut workspace.database)
        .list_sources()
        .map_err(|error| store_failure("discovery.sources", error))?;
    let human = if sources.is_empty() {
        vec!["No discovery sources found".to_owned()]
    } else {
        sources
            .iter()
            .map(|source| format!("{}  {:?} — {}", source.id, source.kind, source.name))
            .collect()
    };
    success(
        "discovery.sources",
        "available",
        &json!({"sources": sources}),
        human,
    )
}

fn discovery_list(
    workspace_path: Option<PathBuf>,
    include_history: bool,
) -> CommandResult<CommandOutput> {
    let mut workspace = open_workspace(workspace_path, "discovery.list")?;
    let leads = DiscoveryService::new(&mut workspace.database)
        .list_leads(include_history)
        .map_err(|error| store_failure("discovery.list", error))?;
    let human = if leads.is_empty() {
        vec!["No discovery leads found".to_owned()]
    } else {
        leads
            .iter()
            .map(|lead| {
                format!(
                    "{}  {} — {} [{:?}]",
                    lead.id, lead.title, lead.organization, lead.status
                )
            })
            .collect()
    };
    success(
        "discovery.list",
        "available",
        &json!({"leads": leads}),
        human,
    )
}

fn discovery_show(workspace_path: Option<PathBuf>, lead_id: &str) -> CommandResult<CommandOutput> {
    let lead_id = parse_entity_id("discovery.show", lead_id)?;
    let mut workspace = open_workspace(workspace_path, "discovery.show")?;
    let lead = DiscoveryService::new(&mut workspace.database)
        .get_lead(&lead_id)
        .map_err(|error| store_failure("discovery.show", error))?;
    success(
        "discovery.show",
        "available",
        &lead,
        vec![
            format!("{} — {}", lead.title, lead.organization),
            format!("Lead ID: {}", lead.id),
            format!("Status: {:?}", lead.status),
            format!("URL: {}", lead.url),
        ],
    )
}

fn discovery_suggest(
    workspace_path: Option<PathBuf>,
    lead_id: &str,
    limit: usize,
) -> CommandResult<CommandOutput> {
    let lead_id = parse_entity_id("discovery.suggest", lead_id)?;
    let mut workspace = open_workspace(workspace_path, "discovery.suggest")?;
    let suggestions = DiscoveryService::new(&mut workspace.database)
        .suggestions(&lead_id, limit)
        .map_err(|error| store_failure("discovery.suggest", error))?;
    let human = if suggestions.is_empty() {
        vec!["No likely duplicate candidates found".to_owned()]
    } else {
        suggestions
            .iter()
            .map(|suggestion| {
                format!(
                    "{}%  {} — {} ({})",
                    suggestion.similarity_percent,
                    suggestion.lead.title,
                    suggestion.lead.organization,
                    suggestion.lead.id
                )
            })
            .collect()
    };
    success(
        "discovery.suggest",
        "available",
        &json!({"suggestions": suggestions, "automatic_merge": false}),
        human,
    )
}

fn discovery_promote(
    workspace_path: Option<PathBuf>,
    lead_id: &str,
) -> CommandResult<CommandOutput> {
    let lead_id = parse_entity_id("discovery.promote", lead_id)?;
    let mut workspace = open_workspace(workspace_path, "discovery.promote")?;
    let (lead, job) = {
        let mut service = DiscoveryService::new(&mut workspace.database);
        let lead = service
            .get_lead(&lead_id)
            .map_err(|error| store_failure("discovery.promote", error))?;
        let job = service
            .promote(&lead_id, ActorKind::User)
            .map_err(|error| store_failure("discovery.promote", error))?;
        (lead, job)
    };
    let import_action = format!("canisend job import {} --url {}", job.id, lead.url);
    let mut output = success(
        "discovery.promote",
        "promoted",
        &json!({"job": job, "lead_id": lead_id}),
        vec![
            format!("Promoted lead into job: {}", job.id),
            format!("Next: {import_action}"),
        ],
    )?;
    output.response.next_actions.push(NextAction {
        action: import_action,
        description: "Import the selected advert through the safe direct-intake URL boundary"
            .to_owned(),
    });
    Ok(output)
}

fn task_prepare(
    workspace_path: Option<PathBuf>,
    arguments: TaskPrepareArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("task.prepare", &arguments.job)?;
    let mut workspace = open_workspace(workspace_path, "task.prepare")?;
    let descriptor = match arguments.operation {
        TaskOperationName::JobCriterion => {
            TaskService::new(&mut workspace.database, &workspace.blobs)
                .prepare_job_criterion(&job_id)
                .map_err(|error| store_failure("task.prepare", error))?
        }
    };
    let mut output = success(
        "task.prepare",
        "prepared",
        &descriptor,
        vec![
            format!("Prepared task: {}", descriptor.id),
            format!("Operation: {}", descriptor.operation),
            format!("Inputs: {}", descriptor.input_artifacts.len()),
            format!("Lease expires: {}", descriptor.lease.expires_at),
        ],
    )?;
    output.response.required_consents = descriptor.required_consents.clone();
    output.response.next_actions.push(NextAction {
        action: "create a canisend.task-completion/v2 JSON file, then run canisend task complete --file PATH"
            .to_owned(),
        description:
            "Repeat the task ID, lease ID, job revision, and every exact input revision/hash"
                .to_owned(),
    });
    Ok(output)
}

fn task_show(workspace_path: Option<PathBuf>, task_id: &str) -> CommandResult<CommandOutput> {
    let task_id = parse_entity_id("task.show", task_id)?;
    let mut workspace = open_workspace(workspace_path, "task.show")?;
    let state = TaskService::new(&mut workspace.database, &workspace.blobs)
        .get(&task_id)
        .map_err(|error| store_failure("task.show", error))?;
    success(
        "task.show",
        "available",
        &state,
        vec![
            format!("Task: {}", state.descriptor.id),
            format!("Operation: {}", state.descriptor.operation),
            format!("Status: {:?}", state.status),
            format!("Inputs: {}", state.descriptor.input_artifacts.len()),
        ],
    )
}

fn task_complete(
    workspace_path: Option<PathBuf>,
    arguments: TaskCompleteArgs,
) -> CommandResult<CommandOutput> {
    let request = if let Some(path) = arguments.file {
        read_task_completion_file(&path)
            .map_err(|error| io_adapter_failure("task.complete", error))?
    } else if arguments.stdin {
        let stdin = std::io::stdin();
        read_task_completion_stdin(stdin.lock())
            .map_err(|error| io_adapter_failure("task.complete", error))?
    } else {
        return Err(CommandFailure::new(
            "task.complete",
            "invalid",
            ErrorCode::InputInvalid,
            "exactly one of --file or --stdin is required",
            false,
        ));
    };
    let mut workspace = open_workspace(workspace_path, "task.complete")?;
    let result = TaskService::new(&mut workspace.database, &workspace.blobs)
        .complete(&request)
        .map_err(|error| store_failure("task.complete", error))?;
    let mut output = success(
        "task.complete",
        "committed",
        &result,
        vec![
            format!("Completed task: {}", result.task_id),
            format!("Artifact: {}", result.artifact.id),
            format!("SHA-256: {}", result.artifact.sha256),
            format!("Idempotent replay: {}", result.idempotent),
        ],
    )?;
    output.response.artifacts.push(result.artifact.clone());
    Ok(output)
}

fn task_cancel(workspace_path: Option<PathBuf>, task_id: &str) -> CommandResult<CommandOutput> {
    let task_id = parse_entity_id("task.cancel", task_id)?;
    let mut workspace = open_workspace(workspace_path, "task.cancel")?;
    let state = TaskService::new(&mut workspace.database, &workspace.blobs)
        .cancel(&task_id)
        .map_err(|error| store_failure("task.cancel", error))?;
    success(
        "task.cancel",
        "cancelled",
        &state,
        vec![format!("Cancelled task: {}", state.descriptor.id)],
    )
}

fn parse_entity_id(operation: &'static str, value: &str) -> CommandResult<EntityId> {
    EntityId::try_new(value).map_err(|error| {
        CommandFailure::new(
            operation,
            "invalid",
            ErrorCode::InputInvalid,
            error.to_string(),
            false,
        )
    })
}

fn io_adapter_failure(operation: &'static str, error: IoAdapterError) -> Box<CommandFailure> {
    let (status, code, retryable) = match &error {
        IoAdapterError::PdfEncrypted => ("invalid", ErrorCode::PdfEncrypted, false),
        IoAdapterError::PdfMalformed(_) | IoAdapterError::PdfPageLimit { .. } => {
            ("invalid", ErrorCode::PdfMalformed, false)
        }
        IoAdapterError::PdfTextUnavailable => {
            ("text-unavailable", ErrorCode::PdfTextUnavailable, false)
        }
        IoAdapterError::Io { .. } => ("io-failed", ErrorCode::ExternalIoFailed, true),
        IoAdapterError::Http(_)
        | IoAdapterError::ResponseRead(_)
        | IoAdapterError::DnsResolution(_)
        | IoAdapterError::HttpStatus(_) => ("fetch-failed", ErrorCode::ExternalIoFailed, true),
        IoAdapterError::UnsafeLocalFile(_) | IoAdapterError::UnsupportedLocalType(_) => {
            ("invalid", ErrorCode::InputPathRejected, false)
        }
        IoAdapterError::InputTooLarge { .. }
        | IoAdapterError::InvalidTextEncoding
        | IoAdapterError::UnsafeTextControlCharacter
        | IoAdapterError::TextUnavailable
        | IoAdapterError::InvalidUrl(_)
        | IoAdapterError::UrlPolicy(_)
        | IoAdapterError::InvalidRedirect(_)
        | IoAdapterError::UnsupportedContentType(_)
        | IoAdapterError::Html(_)
        | IoAdapterError::PdfTimeBudget
        | IoAdapterError::DiscoveryInput(_)
        | IoAdapterError::CandidateInput(_) => ("invalid", ErrorCode::InputInvalid, false),
    };
    CommandFailure::new(operation, status, code, error.to_string(), retryable)
}

fn open_workspace(
    workspace_path: Option<PathBuf>,
    operation: &'static str,
) -> CommandResult<Workspace> {
    Workspace::open(workspace_path.as_deref()).map_err(|error| store_failure(operation, error))
}

fn store_failure(operation: &'static str, error: StoreError) -> Box<CommandFailure> {
    let (status, code, retryable) = match &error {
        StoreError::WorkspaceNotFound(_) => ("not-found", ErrorCode::WorkspaceNotFound, false),
        StoreError::JobNotFound(_) => ("not-found", ErrorCode::JobNotFound, false),
        StoreError::JobArchived(_) => ("archived", ErrorCode::JobArchived, false),
        StoreError::DiscoverySourceNotFound(_) => {
            ("not-found", ErrorCode::DiscoverySourceNotFound, false)
        }
        StoreError::DiscoveryLeadNotFound(_) => {
            ("not-found", ErrorCode::DiscoveryLeadNotFound, false)
        }
        StoreError::DiscoveryConflict(_) => ("conflict", ErrorCode::DiscoveryConflict, false),
        StoreError::TaskNotFound(_) => ("not-found", ErrorCode::TaskNotFound, false),
        StoreError::TaskStale(_) => ("stale", ErrorCode::TaskStale, true),
        StoreError::TaskConflict(_) => ("conflict", ErrorCode::TaskConflict, false),
        StoreError::CandidateStructural(_) => {
            ("validation-failed", ErrorCode::CandidateSchemaInvalid, true)
        }
        StoreError::CandidateSemantic(_) => (
            "validation-failed",
            ErrorCode::CandidateSemanticInvalid,
            true,
        ),
        StoreError::WorkspaceExists(_)
        | StoreError::Sqlite(_)
        | StoreError::DependencyConflict(_)
        | StoreError::ArtifactNotFound(_) => ("conflict", ErrorCode::WorkspaceConflict, true),
        StoreError::UnsafePath(_)
        | StoreError::NotDirectory(_)
        | StoreError::ProjectionPathRejected
        | StoreError::BlobTooLarge { .. }
        | StoreError::ConfigDecode(_)
        | StoreError::BackupInvalid(_) => ("invalid", ErrorCode::InputPathRejected, false),
        StoreError::InvalidInput(_) => ("invalid", ErrorCode::InputInvalid, false),
        StoreError::Io { .. } | StoreError::BlobMissing(_) => {
            ("io-failed", ErrorCode::ExternalIoFailed, true)
        }
        StoreError::BlobDigestMismatch { .. } | StoreError::BlobCollision(_) => {
            ("integrity-failed", ErrorCode::WorkspaceConflict, false)
        }
        StoreError::ConfigEncode(_)
        | StoreError::Json(_)
        | StoreError::Contract(_)
        | StoreError::Random(_)
        | StoreError::Clock
        | StoreError::Invariant(_) => (
            "invariant-failed",
            ErrorCode::InternalInvariantFailed,
            false,
        ),
    };
    let mut failure = CommandFailure::new(operation, status, code, error.to_string(), retryable);
    if let StoreError::CandidateStructural(violations) | StoreError::CandidateSemantic(violations) =
        &error
    {
        failure.error.details = serde_json::to_value(violations).ok();
        failure.error.remediation = Some(NextAction {
            action: "correct the candidate JSON and retry the same leased task".to_owned(),
            description:
                "Use each violation's JSON pointer and stable code; no task state was committed"
                    .to_owned(),
        });
    } else if matches!(error, StoreError::TaskStale(_)) {
        failure.error.remediation = Some(NextAction {
            action: "run canisend task prepare again for the current job revision".to_owned(),
            description:
                "A lease expired or a declared input changed; do not reuse the old candidate"
                    .to_owned(),
        });
    }
    failure
}

fn product_version() -> CommandResult<SemanticVersion> {
    SemanticVersion::try_new(env!("CARGO_PKG_VERSION")).map_err(internal_version)
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
    status: &'static str,
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
        response: AgentResponse::success(operation, status, value),
        human,
    })
}

fn resource_kind_name(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Agent => "agent",
        ResourceKind::Example => "example",
        ResourceKind::Prompt => "prompt",
        ResourceKind::Schema => "schema",
        ResourceKind::Template => "template",
    }
}

fn wants_json(explicit: bool) -> bool {
    explicit || !std::io::stdout().is_terminal()
}

fn render_success(output: CommandOutput, json_output: bool) -> ExitCode {
    if json_output {
        render_json(&output.response)
    } else {
        for line in output.human {
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
        eprintln!("canisend: {}", failure.human);
    }
    ExitCode::from(exit_class.code())
}

fn render_json(response: &AgentResponse) -> ExitCode {
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
    use super::{Cli, ExitClass};
    use clap::Parser;

    #[test]
    fn clap_usage_errors_are_reserved_for_exit_two() {
        let error = Cli::try_parse_from(["canisend", "unknown"]).expect_err("unknown command");
        assert_eq!(error.exit_code(), i32::from(ExitClass::CliUsage.code()));
    }
}
