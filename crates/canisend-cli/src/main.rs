#![forbid(unsafe_code)]

use std::{io::IsTerminal, path::PathBuf, process::ExitCode, str::FromStr};

use canisend_contracts::{
    AGENT_PROTOCOL, ActorKind, AgentContextData, AgentError, AgentResponse, CapabilitiesData,
    EntityId, ErrorCode, ExecutionMode, ExitClass, PUBLIC_SCHEMA_VERSION, PrivacyClassification,
    PublicSchemaId, RESOURCE_FORMAT, ResourceCatalogData, ResourceCatalogEntry, SchemaCatalogData,
    SchemaCatalogEntry, SemanticVersion, Sha256Digest, SourceKind, VersionData, WORKSPACE_FORMAT,
};
use canisend_core::CapabilityRegistry;
use canisend_io::{
    HttpFetcher, IoAdapterError, RemoteDocumentKind, extract_pdf_text, read_local_pdf,
    read_local_text,
};
use canisend_resources::{ResourceId, ResourceKind};
use canisend_store::{ArtifactService, JobService, NewSource, StoreError, Workspace};
use clap::{Args, Parser, Subcommand};
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
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    /// List compiled capabilities and their availability.
    Capabilities(OutputArgs),
    /// Return the body-free public execution context.
    Context(OutputArgs),
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

#[derive(Debug, Args)]
struct OutputArgs {
    /// Emit exactly one canisend.agent/v2 JSON object on stdout.
    #[arg(long)]
    json: bool,
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

impl Cli {
    fn explicit_json(&self) -> bool {
        match &self.command {
            Command::Version(output) | Command::Doctor(output) => output.json,
            Command::Agent {
                command: AgentCommand::Capabilities(output) | AgentCommand::Context(output),
            }
            | Command::Schema {
                command: SchemaCommand::List(output),
            }
            | Command::Resource {
                command: ResourceCommand::List(output),
            } => output.json,
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
            command: AgentCommand::Context(_),
        } => context(),
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

fn context() -> CommandResult<CommandOutput> {
    let data = AgentContextData {
        product_version: product_version()?,
        actor: ActorKind::HostAgent,
        execution_mode: ExecutionMode::HostAgent,
        workspace_id: None,
        active_job_id: None,
        privacy: PrivacyClassification::Public,
    };
    success(
        "agent.context",
        "available",
        &data,
        vec![
            "CanISend public agent context".to_owned(),
            "Workspace: not selected".to_owned(),
            "Privacy: public metadata only".to_owned(),
        ],
    )
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
        | IoAdapterError::DiscoveryInput(_) => ("invalid", ErrorCode::InputInvalid, false),
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
    CommandFailure::new(operation, status, code, error.to_string(), retryable)
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
