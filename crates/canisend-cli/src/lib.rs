#![forbid(unsafe_code)]

mod app_adapter;

use std::{
    fs::{self, OpenOptions},
    io::{IsTerminal, Write},
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

use canisend_app::{
    ACADEMIC_JOB_WORKFLOW_PACK_ID, AgentHost, AgentPackExportRequest, Application,
    ApplicationError, ApplicationFlowApproveRequestV3, ApplicationFlowComposeRequestV3,
    ApplicationFlowCreateRequestV3, ApplicationFlowExportRequestV3, ApplicationFlowPlanRequestV3,
    ContentCatalogFilter, ContentCatalogStatus, ContentCategory, ContentSearchRequest,
    DiscoveryImportRequest, DiscoveryNetworkAdapter, DiscoveryRefreshRequest,
    GENERIC_APPLICATION_WORKFLOW_PACK_ID, NetworkFetchConsent, PackageExportRequest,
    PrivateExportConsent, PrivateReadConsent, ProjectionCopyAsNewRequest, ProjectionReplaceRequest,
    ProviderSendConsent, RenderExportRequest, TaskExecutionMode, TaskInputExportRequest,
    TaskOperation, TaskPrepareRequest, WorkflowBeginRequest, WorkflowCompleteRequest,
    WorkflowRerunRequest, WorkspaceInitPolicy, WorkspaceV3MigrationRequest,
};
use canisend_contracts::{
    AgentError, AgentResponse, CompatibilityNotice, CompatibilitySurface, DocumentKind, EntityId,
    ErrorCode, ExecutionMode, ExitClass, NextAction, PrivacyClassification, PublicSchemaId,
    Revision, SemanticVersion, Sha256Digest, VersionData, WorkflowStage,
};
use canisend_io::{
    IoAdapterError, read_criteria_file, read_task_completion_file, read_task_completion_stdin,
};
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
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

/// Return the exact canonical leaf paths derived from the compiled Clap command graph.
///
/// This is an adapter inventory, not a second operation registry. The source gate compares it to
/// the typed registry in `canisend-contracts` so adding or removing a Clap leaf cannot silently
/// drift from the cross-surface operation contract.
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
    /// Serve the read-only CanISend tool surface over Model Context Protocol.
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
    /// Inspect unified application dossiers assembled from authoritative workspace state.
    Application {
        #[command(subcommand)]
        command: ApplicationCommand,
    },
    /// Browse and search the body-free content catalog or explicitly approved private text.
    Content {
        #[command(subcommand)]
        command: ContentCommand,
    },
    /// Import and inspect reusable profile evidence sources.
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
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
    /// Inspect, correct, and explicitly confirm parsed job criteria.
    Criteria {
        #[command(subcommand)]
        command: CriteriaCommand,
    },
    /// Inspect validated criterion-to-evidence matches.
    Match {
        #[command(subcommand)]
        command: MatchCommand,
    },
    /// Choose whether to apply and confirm the revision-bound document plan.
    Plan {
        #[command(subcommand)]
        command: PlanCommand,
    },
    /// Inspect current structured drafts and their complete revision-bound set.
    Document {
        #[command(subcommand)]
        command: DocumentCommand,
    },
    /// Inspect review findings and record explicit user dispositions.
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    /// Compute and inspect deterministic package readiness without submitting.
    Package {
        #[command(subcommand)]
        command: PackageCommand,
    },
    /// Build, inspect, or explicitly export validated PDFs without submitting.
    Render {
        #[command(subcommand)]
        command: RenderCommand,
    },
    /// Start, inspect, advance, or rerun the durable application workflow.
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommand,
    },
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    /// List compiled capabilities and their availability.
    Capabilities(OutputArgs),
    /// Return the body-free public execution context.
    Context(AgentContextArgs),
    /// Return body-free, job-scoped guidance for the smallest applicable workflow skill.
    Assist(AgentAssistArgs),
    /// Export self-contained integration assets for an agent host.
    Assets {
        #[command(subcommand)]
        command: AgentAssetsCommand,
    },
}

#[derive(Debug, Subcommand)]
enum McpCommand {
    /// Serve versioned read-only tools over newline-delimited JSON-RPC on stdio.
    Serve,
}

#[derive(Debug, Subcommand)]
enum AgentAssetsCommand {
    /// Export a versioned host pack into a new or empty directory.
    Export(AgentAssetsExportArgs),
    /// Inspect bundled and installed CanISend workflow skills.
    Status(AgentAssetsInstallArgs),
    /// Install or safely upgrade CanISend workflow skills in this workspace.
    Install(AgentAssetsInstallArgs),
    /// Remove only unchanged files owned by the CanISend skills manifest.
    Uninstall(AgentAssetsInstallArgs),
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
    /// Initialize a new Pack-qualified workspace at --workspace or the current directory.
    Init(WorkspaceInitArgs),
    /// Report authoritative workspace and SQLite status.
    Status(OutputArgs),
    /// Verify database, blob, freshness, and projection invariants.
    Check(OutputArgs),
    /// Create and verify a consistent backup directory.
    Backup(WorkspaceBackupArgs),
    /// Restore a verified backup into a new empty directory.
    Restore(WorkspaceRestoreArgs),
    /// Rebuild missing or repair-required projections while preserving user edits.
    Repair(OutputArgs),
    /// Preview the body-free v2-to-v3 migration plan without writing.
    MigrationPreview(OutputArgs),
    /// Migrate using the exact reviewed plan digest and a new verified backup.
    Migrate(WorkspaceMigrateArgs),
}

#[derive(Debug, Subcommand)]
enum JobCommand {
    /// Create an empty job record ready for one or more sources.
    Create(JobCreateArgs),
    /// Import a supported local file or user-supplied public URL into an active job.
    Import(JobImportArgs),
    /// List active jobs, or include archived jobs explicitly.
    List(JobListArgs),
    /// Show one job and its body-free source metadata.
    Show(JobIdArgs),
    /// Archive a job without deleting its history.
    Archive(JobIdArgs),
}

#[derive(Debug, Subcommand)]
enum ApplicationCommand {
    /// List body-free application dossiers with metadata and current progress.
    List(ApplicationListArgs),
    /// Show one body-free application dossier and its exact next actions.
    Show(ApplicationJobArgs),
    /// List canonical v3 Applications in the current Workspace.
    V3List(OutputArgs),
    /// Show one canonical v3 Application.
    V3Show(ApplicationV3IdArgs),
    /// Create a generic v3 Application from a reviewed JSON request.
    GenericCreate(ApplicationV3CreateArgs),
    /// Confirm Requirements and commit a generic v3 Plan from JSON.
    GenericPlan(ApplicationV3CandidateArgs),
    /// Commit generic v3 Deliverables for review from JSON.
    GenericCompose(ApplicationV3CandidateArgs),
    /// Approve every current generic v3 Deliverable.
    GenericApprove(ApplicationV3ApproveArgs),
    /// Render and export approved generic v3 Deliverables without submitting.
    GenericExport(ApplicationV3ExportArgs),
}

#[derive(Debug, Subcommand)]
enum ContentCommand {
    /// List body-free content metadata and provenance.
    List(ContentListArgs),
    /// Search metadata, or explicitly include bounded private full text.
    Search(ContentSearchArgs),
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    /// Manage local profile source documents.
    Source {
        #[command(subcommand)]
        command: ProfileSourceCommand,
    },
    /// Normalize, review, confirm, and revise reusable profile evidence.
    Evidence {
        #[command(subcommand)]
        command: ProfileEvidenceCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ProfileSourceCommand {
    /// Import one bounded Markdown, plain-text, or JSON profile source.
    Add(ProfileSourceAddArgs),
    /// List profile source metadata without private bodies.
    List(OutputArgs),
    /// Show one profile source and exact artifact references.
    Show(ProfileSourceIdArgs),
}

#[derive(Debug, Subcommand)]
enum ProfileEvidenceCommand {
    /// Show the unconfirmed core-ID-assigned evidence proposal.
    Proposed(ProfileEvidenceJobArgs),
    /// Export an editable, pre-confirmed evidence catalog candidate.
    Export(ProfileEvidenceExportArgs),
    /// Validate corrections, exclusions, or revisions and commit user confirmation.
    Confirm(ProfileEvidenceConfirmArgs),
    /// Show the current confirmed evidence catalog.
    Show(ProfileEvidenceJobArgs),
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
    /// Freeze exact source revisions and create a leased agent task.
    Prepare(TaskPrepareArgs),
    /// Inspect a task descriptor, state, and committed result metadata.
    Show(TaskIdArgs),
    /// Export only the task's declared private inputs after explicit consent.
    Inputs(TaskInputsArgs),
    /// Validate and atomically commit a host-agent completion request.
    Complete(TaskCompleteArgs),
    /// Cancel a prepared task without deleting its audit history.
    Cancel(TaskIdArgs),
}

#[derive(Debug, Subcommand)]
enum CriteriaCommand {
    /// Show the unconfirmed parsed-job proposal and its source spans.
    Proposed(CriteriaJobArgs),
    /// Export a new editable, pre-confirmed criteria JSON candidate.
    Export(CriteriaExportArgs),
    /// Validate corrections and commit an explicitly confirmed criteria artifact.
    Confirm(CriteriaConfirmArgs),
    /// Show the current confirmed criteria artifact.
    Show(CriteriaJobArgs),
}

#[derive(Debug, Subcommand)]
enum MatchCommand {
    /// Show the current validated match set for one job.
    Show(MatchJobArgs),
}

#[derive(Debug, Subcommand)]
enum PlanCommand {
    /// Export a new editable decision and document-plan candidate.
    Export(PlanExportArgs),
    /// Validate and commit an explicit application decision.
    Confirm(PlanConfirmArgs),
    /// Show the current confirmed application plan.
    Show(PlanJobArgs),
}

#[derive(Debug, Subcommand)]
enum DocumentCommand {
    /// List all current structured drafts already completed for one job.
    List(DocumentJobArgs),
    /// Show one current structured draft by document kind.
    Show(DocumentShowArgs),
    /// Show the complete current document set after Draft finishes.
    Set(DocumentJobArgs),
}

#[derive(Debug, Subcommand)]
enum ReviewCommand {
    /// Export an editable disposition candidate for human-review findings.
    Export(ReviewExportArgs),
    /// Confirm selected accepted-risk or dismissed dispositions as the user.
    Confirm(ReviewConfirmArgs),
    /// Show the current deterministic and human review findings.
    Show(ReviewJobArgs),
}

#[derive(Debug, Subcommand)]
enum PackageCommand {
    /// Freeze exact current inputs and compute machine-readable readiness.
    Check(PackageJobArgs),
    /// Show the current body-free package manifest and readiness reasons.
    Show(PackageJobArgs),
    /// Export editable Markdown, structured JSON, and Typst after explicit private-export consent.
    Export(PackageExportArgs),
    /// Show the current export receipt and projection hashes.
    Exports(PackageJobArgs),
    /// Inspect managed projections and record current, edited, or missing state.
    Reconcile(PackageJobArgs),
    /// Discard one projection edit and restore the authoritative generated form.
    Replace(PackageProjectionArgs),
    /// Preserve an edit at a new unmanaged path, then restore the managed projection.
    CopyAsNew(PackageCopyAsNewArgs),
}

#[derive(Debug, Subcommand)]
enum RenderCommand {
    /// Compile trusted structured documents to validated PDF artifacts in process.
    Build(PackageJobArgs),
    /// Show the current revision-bound render manifest.
    Show(PackageJobArgs),
    /// Export validated PDFs and their manifest after explicit private-export consent.
    Export(RenderExportArgs),
}

#[derive(Debug, Subcommand)]
enum WorkflowCommand {
    /// Create the durable stage graph from current job state, idempotently.
    Start(WorkflowJobArgs),
    /// Return body-free stage status, blockers, and next actions.
    Status(WorkflowJobArgs),
    /// Begin one ready stage in a mode allowed by the compiled graph.
    Begin(WorkflowBeginArgs),
    /// Complete a running or awaiting-user stage with a current artifact.
    Complete(WorkflowCompleteArgs),
    /// Reset one stage and invalidate only its transitive descendants.
    Rerun(WorkflowRerunArgs),
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
struct AgentAssistArgs {
    /// Select the application whose revisions and content relationships guide the Agent.
    #[arg(long)]
    job: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AgentHostName {
    Codex,
    Claude,
    Generic,
}

#[derive(Debug, Args)]
struct AgentAssetsExportArgs {
    /// Host-specific instruction entrypoint to include.
    #[arg(long, value_enum)]
    host: AgentHostName,
    /// New or empty destination directory outside .canisend internal state.
    #[arg(long, value_name = "PATH")]
    destination: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct AgentAssetsInstallArgs {
    /// Agent host whose project skill discovery layout should be used.
    #[arg(long, value_enum)]
    host: AgentHostName,
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

#[derive(Debug, Clone, Copy, ValueEnum)]
enum BuiltInPackName {
    AcademicJob,
    GenericApplication,
}

impl BuiltInPackName {
    const fn id(self) -> &'static str {
        match self {
            Self::AcademicJob => ACADEMIC_JOB_WORKFLOW_PACK_ID,
            Self::GenericApplication => GENERIC_APPLICATION_WORKFLOW_PACK_ID,
        }
    }
}

#[derive(Debug, Args)]
struct WorkspaceInitArgs {
    /// Select the exact built-in workflow Pack and Workspace authority generation.
    #[arg(long, value_enum, default_value_t = BuiltInPackName::GenericApplication)]
    pack: BuiltInPackName,
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
struct WorkspaceMigrateArgs {
    /// Exact digest returned by workspace migration-preview.
    #[arg(long)]
    expected_plan_sha256: String,
    /// New or empty destination for the verified pre-migration backup.
    #[arg(long, value_name = "PATH")]
    backup_destination: PathBuf,
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
struct ApplicationListArgs {
    #[arg(long)]
    include_archived: bool,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ApplicationJobArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ApplicationV3IdArgs {
    #[arg(long)]
    application: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ApplicationV3CreateArgs {
    /// Reviewed bounded JSON request matching the canonical operation contract.
    #[arg(long, value_name = "PATH")]
    candidate: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ApplicationV3CandidateArgs {
    #[arg(long)]
    application: String,
    /// Reviewed bounded JSON request matching the canonical operation contract.
    #[arg(long, value_name = "PATH")]
    candidate: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ApplicationV3ApproveArgs {
    #[arg(long)]
    application: String,
    #[arg(long)]
    expected_revision: u64,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ApplicationV3ExportArgs {
    #[arg(long)]
    application: String,
    #[arg(long)]
    expected_revision: u64,
    #[arg(long)]
    destination: String,
    /// Confirm local private-artifact export for this operation.
    #[arg(long)]
    allow_private_export: bool,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ContentFilterArgs {
    /// Limit results to content related to one canonical job ID.
    #[arg(long)]
    job: Option<String>,
    /// Limit results to one user-facing content category.
    #[arg(long, value_enum)]
    category: Option<ContentCategoryName>,
    /// Limit results to one workflow stage.
    #[arg(long, value_enum)]
    stage: Option<WorkflowStageName>,
    /// Limit results to one lifecycle status.
    #[arg(long, value_enum)]
    status: Option<ContentStatusName>,
    /// Limit results to one privacy classification.
    #[arg(long, value_enum)]
    privacy: Option<ContentPrivacyName>,
    /// Include artifacts created at or after this UTC RFC 3339 timestamp.
    #[arg(long, value_name = "UTC_TIMESTAMP")]
    created_after: Option<String>,
    /// Include artifacts created at or before this UTC RFC 3339 timestamp.
    #[arg(long, value_name = "UTC_TIMESTAMP")]
    created_before: Option<String>,
}

#[derive(Debug, Args)]
struct ContentListArgs {
    #[command(flatten)]
    filter: ContentFilterArgs,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ContentSearchArgs {
    /// Bounded search text. An empty query applies only the selected filters.
    query: String,
    #[command(flatten)]
    filter: ContentFilterArgs,
    /// Build a bounded in-memory index from eligible private artifact bodies.
    #[arg(long)]
    include_private_bodies: bool,
    /// Confirm read-private-inputs consent for this one local search.
    #[arg(long, requires = "include_private_bodies")]
    allow_private_read: bool,
    /// Maximum returned results.
    #[arg(long, default_value_t = 50)]
    limit: usize,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ContentCategoryName {
    Source,
    Profile,
    JobAnalysis,
    Evidence,
    Planning,
    Materials,
    Review,
    Delivery,
}

impl From<ContentCategoryName> for ContentCategory {
    fn from(value: ContentCategoryName) -> Self {
        match value {
            ContentCategoryName::Source => Self::Source,
            ContentCategoryName::Profile => Self::Profile,
            ContentCategoryName::JobAnalysis => Self::JobAnalysis,
            ContentCategoryName::Evidence => Self::Evidence,
            ContentCategoryName::Planning => Self::Planning,
            ContentCategoryName::Materials => Self::Materials,
            ContentCategoryName::Review => Self::Review,
            ContentCategoryName::Delivery => Self::Delivery,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ContentStatusName {
    Imported,
    Proposed,
    Confirmed,
    Generated,
    Stale,
}

impl From<ContentStatusName> for ContentCatalogStatus {
    fn from(value: ContentStatusName) -> Self {
        match value {
            ContentStatusName::Imported => Self::Imported,
            ContentStatusName::Proposed => Self::Proposed,
            ContentStatusName::Confirmed => Self::Confirmed,
            ContentStatusName::Generated => Self::Generated,
            ContentStatusName::Stale => Self::Stale,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ContentPrivacyName {
    Public,
    PrivateLocal,
    ProviderBound,
    Secret,
}

impl From<ContentPrivacyName> for PrivacyClassification {
    fn from(value: ContentPrivacyName) -> Self {
        match value {
            ContentPrivacyName::Public => Self::Public,
            ContentPrivacyName::PrivateLocal => Self::PrivateLocal,
            ContentPrivacyName::ProviderBound => Self::ProviderBound,
            ContentPrivacyName::Secret => Self::Secret,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ProfileSensitivityName {
    Public,
    PrivateLocal,
}

#[derive(Debug, Args)]
struct ProfileSourceAddArgs {
    /// Regular, non-symlink UTF-8 Markdown, text, or JSON file.
    #[arg(long, value_name = "PATH")]
    file: PathBuf,
    /// Classification retained with the source; defaults to local private data.
    #[arg(long, value_enum, default_value = "private-local")]
    sensitivity: ProfileSensitivityName,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ProfileSourceIdArgs {
    /// Canonical UUIDv7 profile source ID.
    source_id: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ProfileEvidenceJobArgs {
    /// Canonical UUIDv7 job whose workflow consumes the reusable profile evidence.
    #[arg(long)]
    job: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ProfileEvidenceExportArgs {
    /// Canonical UUIDv7 job whose evidence decision is being reviewed.
    #[arg(long)]
    job: String,
    /// New external JSON file that the user or host agent can edit.
    #[arg(long, value_name = "PATH")]
    destination: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ProfileEvidenceConfirmArgs {
    /// Canonical UUIDv7 job whose evidence decision is being confirmed or revised.
    #[arg(long)]
    job: String,
    /// Regular, non-symlink evidence JSON exported by `profile evidence export`.
    #[arg(long, value_name = "PATH")]
    file: PathBuf,
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

impl DiscoveryAdapterName {
    const fn id(self) -> &'static str {
        match self {
            Self::RssAtom => "rss-atom",
            Self::JobsAcUk => "jobs-ac-uk",
            Self::Greenhouse => "greenhouse",
            Self::Lever => "lever",
        }
    }
}

impl From<DiscoveryAdapterName> for DiscoveryNetworkAdapter {
    fn from(value: DiscoveryAdapterName) -> Self {
        match value {
            DiscoveryAdapterName::RssAtom => Self::RssAtom,
            DiscoveryAdapterName::JobsAcUk => Self::JobsAcUk,
            DiscoveryAdapterName::Greenhouse => Self::Greenhouse,
            DiscoveryAdapterName::Lever => Self::Lever,
        }
    }
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
    JobParse,
    EvidenceNormalize,
    EvidenceMatch,
    CoverLetterDraft,
    ResearchStatementDraft,
    TeachingStatementDraft,
    CvDraft,
    DocumentReview,
}

impl From<TaskOperationName> for TaskOperation {
    fn from(value: TaskOperationName) -> Self {
        match value {
            TaskOperationName::JobParse => Self::JobParse,
            TaskOperationName::EvidenceNormalize => Self::EvidenceNormalize,
            TaskOperationName::EvidenceMatch => Self::EvidenceMatch,
            TaskOperationName::CoverLetterDraft => Self::CoverLetterDraft,
            TaskOperationName::ResearchStatementDraft => Self::ResearchStatementDraft,
            TaskOperationName::TeachingStatementDraft => Self::TeachingStatementDraft,
            TaskOperationName::CvDraft => Self::CvDraft,
            TaskOperationName::DocumentReview => Self::DocumentReview,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DocumentKindName {
    CoverLetter,
    ResearchStatement,
    TeachingStatement,
    Cv,
}

impl From<DocumentKindName> for DocumentKind {
    fn from(value: DocumentKindName) -> Self {
        match value {
            DocumentKindName::CoverLetter => Self::CoverLetter,
            DocumentKindName::ResearchStatement => Self::ResearchStatement,
            DocumentKindName::TeachingStatement => Self::TeachingStatement,
            DocumentKindName::Cv => Self::Cv,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TaskExecutionModeName {
    HostAgent,
    ConfiguredProvider,
}

impl From<TaskExecutionModeName> for TaskExecutionMode {
    fn from(value: TaskExecutionModeName) -> Self {
        match value {
            TaskExecutionModeName::HostAgent => Self::HostAgent,
            TaskExecutionModeName::ConfiguredProvider => Self::ConfiguredProvider,
        }
    }
}

#[derive(Debug, Args)]
struct TaskPrepareArgs {
    /// Canonical UUIDv7 job ID whose current source revisions become task inputs.
    #[arg(long)]
    job: String,
    /// Bounded operation implemented by the compiled task registry.
    #[arg(long, value_enum)]
    operation: TaskOperationName,
    /// Reasoning executor; both modes use the same candidate validator.
    #[arg(long, value_enum, default_value = "host-agent")]
    mode: TaskExecutionModeName,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct DocumentJobArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct DocumentShowArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    /// Structured document kind to inspect.
    #[arg(long, value_enum)]
    kind: DocumentKindName,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ReviewJobArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ReviewExportArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    /// New external JSON file for explicit user dispositions.
    #[arg(long, value_name = "PATH")]
    destination: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct ReviewConfirmArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    /// Regular, non-symlink JSON exported by `review export`.
    #[arg(long, value_name = "PATH")]
    file: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct PackageJobArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct PackageExportArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    /// Safe workspace-relative directory under jobs/JOB_ID/.
    #[arg(long, value_name = "RELATIVE_PATH")]
    destination: String,
    /// Confirm export of private application material bodies into the workspace projection tree.
    #[arg(long)]
    allow_private_export: bool,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct PackageProjectionArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    /// Managed workspace-relative projection path.
    #[arg(long, value_name = "RELATIVE_PATH")]
    path: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct PackageCopyAsNewArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    /// Edited managed workspace-relative projection path.
    #[arg(long, value_name = "RELATIVE_PATH")]
    path: String,
    /// New unmanaged workspace-relative path that preserves the edited bytes.
    #[arg(long, value_name = "RELATIVE_PATH")]
    destination: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct RenderExportArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    /// New or empty safe workspace-relative directory under jobs/JOB_ID/.
    #[arg(long, value_name = "RELATIVE_PATH")]
    destination: String,
    /// Confirm export of private PDF bodies into the workspace projection tree.
    #[arg(long)]
    allow_private_export: bool,
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

#[derive(Debug, Args)]
struct TaskInputsArgs {
    /// Canonical UUIDv7 task ID.
    task_id: String,
    /// New or empty external directory for the scoped inputs and manifest.
    #[arg(long, value_name = "PATH")]
    destination: PathBuf,
    /// Confirm that the user granted the descriptor's read-private-inputs consent.
    #[arg(long)]
    allow_private_read: bool,
    /// Confirm sending the exported scope to the configured provider, when required.
    #[arg(long)]
    allow_provider_send: bool,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct CriteriaJobArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct CriteriaExportArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    /// New external JSON file that the user or host agent can edit.
    #[arg(long, value_name = "PATH")]
    destination: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct CriteriaConfirmArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    /// Regular, non-symlink criteria JSON exported by `criteria export`.
    #[arg(long, value_name = "PATH")]
    file: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct MatchJobArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct PlanJobArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct PlanExportArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    /// New external JSON file that the user or host agent can edit.
    #[arg(long, value_name = "PATH")]
    destination: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct PlanConfirmArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    /// Regular, non-symlink JSON exported by `plan export`.
    #[arg(long, value_name = "PATH")]
    file: PathBuf,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum WorkflowStageName {
    Intake,
    Parse,
    Criteria,
    Evidence,
    Match,
    Plan,
    Draft,
    Review,
    Package,
    Render,
}

impl From<WorkflowStageName> for WorkflowStage {
    fn from(value: WorkflowStageName) -> Self {
        match value {
            WorkflowStageName::Intake => Self::Intake,
            WorkflowStageName::Parse => Self::Parse,
            WorkflowStageName::Criteria => Self::Criteria,
            WorkflowStageName::Evidence => Self::Evidence,
            WorkflowStageName::Match => Self::Match,
            WorkflowStageName::Plan => Self::Plan,
            WorkflowStageName::Draft => Self::Draft,
            WorkflowStageName::Review => Self::Review,
            WorkflowStageName::Package => Self::Package,
            WorkflowStageName::Render => Self::Render,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ExecutionModeName {
    Deterministic,
    HostAgent,
    ConfiguredProvider,
    UserDecision,
    ManualImport,
}

impl From<ExecutionModeName> for ExecutionMode {
    fn from(value: ExecutionModeName) -> Self {
        match value {
            ExecutionModeName::Deterministic => Self::Deterministic,
            ExecutionModeName::HostAgent => Self::HostAgent,
            ExecutionModeName::ConfiguredProvider => Self::ConfiguredProvider,
            ExecutionModeName::UserDecision => Self::UserDecision,
            ExecutionModeName::ManualImport => Self::ManualImport,
        }
    }
}

#[derive(Debug, Args)]
struct WorkflowJobArgs {
    /// Canonical UUIDv7 job ID.
    #[arg(long)]
    job: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct WorkflowBeginArgs {
    #[arg(long)]
    job: String,
    #[arg(long, value_enum)]
    stage: WorkflowStageName,
    #[arg(long, value_enum)]
    mode: ExecutionModeName,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct WorkflowCompleteArgs {
    #[arg(long)]
    job: String,
    #[arg(long, value_enum)]
    stage: WorkflowStageName,
    /// Current output artifact UUIDv7; kind must match the compiled stage descriptor.
    #[arg(long)]
    artifact: String,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct WorkflowRerunArgs {
    #[arg(long)]
    job: String,
    #[arg(long, value_enum)]
    stage: WorkflowStageName,
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
            Command::Agent {
                command: AgentCommand::Assist(arguments),
            } => arguments.output.json,
            Command::Mcp {
                command: McpCommand::Serve,
            } => false,
            Command::Agent {
                command:
                    AgentCommand::Assets {
                        command: AgentAssetsCommand::Export(arguments),
                    },
            } => arguments.output.json,
            Command::Agent {
                command:
                    AgentCommand::Assets {
                        command:
                            AgentAssetsCommand::Status(arguments)
                            | AgentAssetsCommand::Install(arguments)
                            | AgentAssetsCommand::Uninstall(arguments),
                    },
            } => arguments.output.json,
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
                    | WorkspaceCommand::Repair(output)
                    | WorkspaceCommand::MigrationPreview(output),
            } => output.json,
            Command::Workspace {
                command: WorkspaceCommand::Backup(arguments),
            } => arguments.output.json,
            Command::Workspace {
                command: WorkspaceCommand::Restore(arguments),
            } => arguments.output.json,
            Command::Workspace {
                command: WorkspaceCommand::Migrate(arguments),
            } => arguments.output.json,
            Command::Job { command } => match command {
                JobCommand::Create(arguments) => arguments.output.json,
                JobCommand::Import(arguments) => arguments.output.json,
                JobCommand::List(arguments) => arguments.output.json,
                JobCommand::Show(arguments) | JobCommand::Archive(arguments) => {
                    arguments.output.json
                }
            },
            Command::Application { command } => match command {
                ApplicationCommand::List(arguments) => arguments.output.json,
                ApplicationCommand::Show(arguments) => arguments.output.json,
                ApplicationCommand::V3List(output) => output.json,
                ApplicationCommand::V3Show(arguments) => arguments.output.json,
                ApplicationCommand::GenericCreate(arguments) => arguments.output.json,
                ApplicationCommand::GenericPlan(arguments)
                | ApplicationCommand::GenericCompose(arguments) => arguments.output.json,
                ApplicationCommand::GenericApprove(arguments) => arguments.output.json,
                ApplicationCommand::GenericExport(arguments) => arguments.output.json,
            },
            Command::Content { command } => match command {
                ContentCommand::List(arguments) => arguments.output.json,
                ContentCommand::Search(arguments) => arguments.output.json,
            },
            Command::Profile { command } => match command {
                ProfileCommand::Source { command } => match command {
                    ProfileSourceCommand::Add(arguments) => arguments.output.json,
                    ProfileSourceCommand::List(output) => output.json,
                    ProfileSourceCommand::Show(arguments) => arguments.output.json,
                },
                ProfileCommand::Evidence { command } => match command {
                    ProfileEvidenceCommand::Proposed(arguments)
                    | ProfileEvidenceCommand::Show(arguments) => arguments.output.json,
                    ProfileEvidenceCommand::Export(arguments) => arguments.output.json,
                    ProfileEvidenceCommand::Confirm(arguments) => arguments.output.json,
                },
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
                TaskCommand::Inputs(arguments) => arguments.output.json,
                TaskCommand::Complete(arguments) => arguments.output.json,
            },
            Command::Criteria { command } => match command {
                CriteriaCommand::Proposed(arguments) | CriteriaCommand::Show(arguments) => {
                    arguments.output.json
                }
                CriteriaCommand::Export(arguments) => arguments.output.json,
                CriteriaCommand::Confirm(arguments) => arguments.output.json,
            },
            Command::Match { command } => match command {
                MatchCommand::Show(arguments) => arguments.output.json,
            },
            Command::Plan { command } => match command {
                PlanCommand::Export(arguments) => arguments.output.json,
                PlanCommand::Confirm(arguments) => arguments.output.json,
                PlanCommand::Show(arguments) => arguments.output.json,
            },
            Command::Document { command } => match command {
                DocumentCommand::List(arguments) | DocumentCommand::Set(arguments) => {
                    arguments.output.json
                }
                DocumentCommand::Show(arguments) => arguments.output.json,
            },
            Command::Review { command } => match command {
                ReviewCommand::Export(arguments) => arguments.output.json,
                ReviewCommand::Confirm(arguments) => arguments.output.json,
                ReviewCommand::Show(arguments) => arguments.output.json,
            },
            Command::Package { command } => match command {
                PackageCommand::Check(arguments) | PackageCommand::Show(arguments) => {
                    arguments.output.json
                }
                PackageCommand::Export(arguments) => arguments.output.json,
                PackageCommand::Exports(arguments) | PackageCommand::Reconcile(arguments) => {
                    arguments.output.json
                }
                PackageCommand::Replace(arguments) => arguments.output.json,
                PackageCommand::CopyAsNew(arguments) => arguments.output.json,
            },
            Command::Render { command } => match command {
                RenderCommand::Build(arguments) | RenderCommand::Show(arguments) => {
                    arguments.output.json
                }
                RenderCommand::Export(arguments) => arguments.output.json,
            },
            Command::Workflow { command } => match command {
                WorkflowCommand::Start(arguments) | WorkflowCommand::Status(arguments) => {
                    arguments.output.json
                }
                WorkflowCommand::Begin(arguments) => arguments.output.json,
                WorkflowCommand::Complete(arguments) => arguments.output.json,
                WorkflowCommand::Rerun(arguments) => arguments.output.json,
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

    fn response(&self) -> AgentResponse {
        AgentResponse::failure(self.operation, self.status.clone(), self.error.clone())
    }
}

/// Run the versioned CanISend command-line adapter using the current process arguments.
///
/// The desktop binary reuses this entrypoint for its explicit CLI mode so the packaged App can
/// eventually ship one native executable without duplicating command or MCP behavior.
#[must_use]
pub fn run() -> ExitCode {
    let cli = Cli::parse();
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
        Command::Agent {
            command: AgentCommand::Assist(arguments),
        } => assistance(workspace, &arguments.job),
        Command::Agent {
            command:
                AgentCommand::Assets {
                    command: AgentAssetsCommand::Export(arguments),
                },
        } => agent_assets_export(arguments),
        Command::Agent {
            command:
                AgentCommand::Assets {
                    command: AgentAssetsCommand::Status(arguments),
                },
        } => agent_assets_status(workspace, arguments),
        Command::Agent {
            command:
                AgentCommand::Assets {
                    command: AgentAssetsCommand::Install(arguments),
                },
        } => agent_assets_install(workspace, arguments),
        Command::Agent {
            command:
                AgentCommand::Assets {
                    command: AgentAssetsCommand::Uninstall(arguments),
                },
        } => agent_assets_uninstall(workspace, arguments),
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
            command: WorkspaceCommand::Init(arguments),
        } => workspace_init(workspace, arguments.pack),
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
        Command::Workspace {
            command: WorkspaceCommand::MigrationPreview(_),
        } => workspace_migration_preview(workspace),
        Command::Workspace {
            command: WorkspaceCommand::Migrate(arguments),
        } => workspace_migrate(workspace, arguments),
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
        Command::Application {
            command: ApplicationCommand::List(arguments),
        } => application_list(workspace, arguments.include_archived),
        Command::Application {
            command: ApplicationCommand::Show(arguments),
        } => application_show(workspace, &arguments.job),
        Command::Application {
            command: ApplicationCommand::V3List(_),
        } => application_v3_list(workspace),
        Command::Application {
            command: ApplicationCommand::V3Show(arguments),
        } => application_v3_show(workspace, &arguments.application),
        Command::Application {
            command: ApplicationCommand::GenericCreate(arguments),
        } => application_v3_create(workspace, arguments),
        Command::Application {
            command: ApplicationCommand::GenericPlan(arguments),
        } => application_v3_plan(workspace, arguments),
        Command::Application {
            command: ApplicationCommand::GenericCompose(arguments),
        } => application_v3_compose(workspace, arguments),
        Command::Application {
            command: ApplicationCommand::GenericApprove(arguments),
        } => application_v3_approve(workspace, arguments),
        Command::Application {
            command: ApplicationCommand::GenericExport(arguments),
        } => application_v3_export(workspace, arguments),
        Command::Content {
            command: ContentCommand::List(arguments),
        } => content_list(workspace, arguments),
        Command::Content {
            command: ContentCommand::Search(arguments),
        } => content_search(workspace, arguments),
        Command::Profile {
            command:
                ProfileCommand::Source {
                    command: ProfileSourceCommand::Add(arguments),
                },
        } => profile_source_add(workspace, arguments),
        Command::Profile {
            command:
                ProfileCommand::Source {
                    command: ProfileSourceCommand::List(_),
                },
        } => profile_source_list(workspace),
        Command::Profile {
            command:
                ProfileCommand::Source {
                    command: ProfileSourceCommand::Show(arguments),
                },
        } => profile_source_show(workspace, &arguments.source_id),
        Command::Profile {
            command:
                ProfileCommand::Evidence {
                    command: ProfileEvidenceCommand::Proposed(arguments),
                },
        } => profile_evidence_proposed(workspace, &arguments.job),
        Command::Profile {
            command:
                ProfileCommand::Evidence {
                    command: ProfileEvidenceCommand::Export(arguments),
                },
        } => profile_evidence_export(workspace, arguments),
        Command::Profile {
            command:
                ProfileCommand::Evidence {
                    command: ProfileEvidenceCommand::Confirm(arguments),
                },
        } => profile_evidence_confirm(workspace, arguments),
        Command::Profile {
            command:
                ProfileCommand::Evidence {
                    command: ProfileEvidenceCommand::Show(arguments),
                },
        } => profile_evidence_show(workspace, &arguments.job),
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
            command: TaskCommand::Inputs(arguments),
        } => task_inputs(workspace, arguments),
        Command::Task {
            command: TaskCommand::Complete(arguments),
        } => task_complete(workspace, arguments),
        Command::Task {
            command: TaskCommand::Cancel(arguments),
        } => task_cancel(workspace, &arguments.task_id),
        Command::Criteria {
            command: CriteriaCommand::Proposed(arguments),
        } => criteria_proposed(workspace, &arguments.job),
        Command::Criteria {
            command: CriteriaCommand::Export(arguments),
        } => criteria_export(workspace, arguments),
        Command::Criteria {
            command: CriteriaCommand::Confirm(arguments),
        } => criteria_confirm(workspace, arguments),
        Command::Criteria {
            command: CriteriaCommand::Show(arguments),
        } => criteria_show(workspace, &arguments.job),
        Command::Match {
            command: MatchCommand::Show(arguments),
        } => match_show(workspace, &arguments.job),
        Command::Plan {
            command: PlanCommand::Export(arguments),
        } => plan_export(workspace, arguments),
        Command::Plan {
            command: PlanCommand::Confirm(arguments),
        } => plan_confirm(workspace, arguments),
        Command::Plan {
            command: PlanCommand::Show(arguments),
        } => plan_show(workspace, &arguments.job),
        Command::Document {
            command: DocumentCommand::List(arguments),
        } => document_list(workspace, &arguments.job),
        Command::Document {
            command: DocumentCommand::Show(arguments),
        } => document_show(workspace, &arguments.job, arguments.kind),
        Command::Document {
            command: DocumentCommand::Set(arguments),
        } => document_set(workspace, &arguments.job),
        Command::Review {
            command: ReviewCommand::Export(arguments),
        } => review_export(workspace, arguments),
        Command::Review {
            command: ReviewCommand::Confirm(arguments),
        } => review_confirm(workspace, arguments),
        Command::Review {
            command: ReviewCommand::Show(arguments),
        } => review_show(workspace, &arguments.job),
        Command::Package {
            command: PackageCommand::Check(arguments),
        } => package_check(workspace, &arguments.job),
        Command::Package {
            command: PackageCommand::Show(arguments),
        } => package_show(workspace, &arguments.job),
        Command::Package {
            command: PackageCommand::Export(arguments),
        } => package_export(workspace, arguments),
        Command::Package {
            command: PackageCommand::Exports(arguments),
        } => package_exports(workspace, &arguments.job),
        Command::Package {
            command: PackageCommand::Reconcile(arguments),
        } => package_reconcile(workspace, &arguments.job),
        Command::Package {
            command: PackageCommand::Replace(arguments),
        } => package_replace(workspace, arguments),
        Command::Package {
            command: PackageCommand::CopyAsNew(arguments),
        } => package_copy_as_new(workspace, arguments),
        Command::Render {
            command: RenderCommand::Build(arguments),
        } => render_build(workspace, &arguments.job),
        Command::Render {
            command: RenderCommand::Show(arguments),
        } => render_show(workspace, &arguments.job),
        Command::Render {
            command: RenderCommand::Export(arguments),
        } => render_export(workspace, arguments),
        Command::Workflow {
            command: WorkflowCommand::Start(arguments),
        } => workflow_start(workspace, &arguments.job),
        Command::Workflow {
            command: WorkflowCommand::Status(arguments),
        } => workflow_status(workspace, &arguments.job),
        Command::Workflow {
            command: WorkflowCommand::Begin(arguments),
        } => workflow_begin(workspace, arguments),
        Command::Workflow {
            command: WorkflowCommand::Complete(arguments),
        } => workflow_complete(workspace, arguments),
        Command::Workflow {
            command: WorkflowCommand::Rerun(arguments),
        } => workflow_rerun(workspace, arguments),
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
        response: AgentResponse::success("product.doctor", "healthy", data),
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

fn capabilities() -> CommandResult<CommandOutput> {
    let receipt = Application::agent_capabilities()
        .map_err(|error| app_adapter::failure("agent.capabilities", error))?;
    let compatibility = receipt.compatibility;
    let data = receipt.data;
    let human = std::iter::once(format!("CanISend {} capabilities", data.product_version))
        .chain(
            data.capabilities
                .iter()
                .map(|capability| format!("{}: {:?}", capability.id, capability.status)),
        )
        .collect();
    success_with_compatibility(
        "agent.capabilities",
        "available",
        &data,
        human,
        compatibility,
        CompatibilitySurface::AgentV2,
    )
}

fn context(
    workspace_path: Option<PathBuf>,
    selected_job_id: Option<&str>,
) -> CommandResult<CommandOutput> {
    let receipt = Application::agent_context(workspace_path.as_deref(), selected_job_id)
        .map_err(agent_context_failure)?;
    let compatibility = receipt.compatibility;
    let data = receipt.data;
    let mut output = success_with_compatibility(
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
        compatibility,
        CompatibilitySurface::AgentV2,
    )?;
    output.response.next_actions = data.next_actions.clone();
    Ok(output)
}

fn assistance(
    workspace_path: Option<PathBuf>,
    selected_job_id: &str,
) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root(workspace_path, "agent.assistance")?;
    let receipt = Application::agent_assistance(&root, selected_job_id)
        .map_err(|error| app_adapter::failure("agent.assistance", error))?;
    let data = receipt.data;
    let mut output = success(
        "agent.assistance",
        "available",
        &data,
        vec![
            "CanISend body-free contextual assistance".to_owned(),
            format!(
                "Application: {} — {}",
                data.dossier.job.title, data.dossier.job.institution
            ),
            format!("Recommended skill: {}", data.recommendation.skill_id),
            format!("Proposal targets: {}", data.proposal_targets.len()),
            format!(
                "Content references: {} of {}",
                data.content.entries.len(),
                data.content.total_entries
            ),
            "Privacy: public metadata and artifact identities only".to_owned(),
        ],
    )?;
    output.response.next_actions = receipt.next_actions;
    Ok(output)
}

fn agent_assets_export(arguments: AgentAssetsExportArgs) -> CommandResult<CommandOutput> {
    let host = match arguments.host {
        AgentHostName::Codex => AgentHost::Codex,
        AgentHostName::Claude => AgentHost::Claude,
        AgentHostName::Generic => AgentHost::Generic,
    };
    let exported =
        Application::export_agent_assets(&AgentPackExportRequest::new(host, arguments.destination))
            .map_err(|error| app_adapter::failure("agent.assets.export", error))?
            .data;
    success(
        "agent.assets.export",
        "exported",
        &exported,
        vec![
            format!("Exported {} agent pack", host.as_str()),
            format!("Directory: {}", exported.directory.display()),
            format!("Manifest: {}", exported.manifest_path.display()),
            format!("Resources: {}", exported.manifest.files.len()),
        ],
    )
}

fn agent_assets_install(
    workspace_path: Option<PathBuf>,
    arguments: AgentAssetsInstallArgs,
) -> CommandResult<CommandOutput> {
    let host = match arguments.host {
        AgentHostName::Codex => AgentHost::Codex,
        AgentHostName::Claude => AgentHost::Claude,
        AgentHostName::Generic => AgentHost::Generic,
    };
    let workspace = workspace_path.unwrap_or_else(|| PathBuf::from("."));
    let installed = Application::install_agent_skills(&canisend_app::AgentSkillsInstallRequest {
        host,
        workspace,
    })
    .map_err(|error| app_adapter::failure("agent.skills.install", error))?
    .data;
    success(
        "agent.skills.install",
        match installed.state {
            canisend_resources::AgentSkillsInstallState::Installed => "installed",
            canisend_resources::AgentSkillsInstallState::Updated => "updated",
            canisend_resources::AgentSkillsInstallState::UpToDate => "up-to-date",
        },
        &installed,
        vec![
            format!("CanISend workflow skills: {:?}", installed.state),
            format!("Directory: {}", installed.directory.display()),
            format!("Manifest: {}", installed.manifest_path.display()),
            format!("Resources: {}", installed.files.len()),
        ],
    )
}

fn agent_assets_status(
    workspace_path: Option<PathBuf>,
    arguments: AgentAssetsInstallArgs,
) -> CommandResult<CommandOutput> {
    let host = agent_host(arguments.host);
    let workspace = workspace_path.unwrap_or_else(|| PathBuf::from("."));
    let receipt = Application::agent_skills_status(&canisend_app::AgentSkillsStatusRequest {
        host,
        workspace,
    })
    .map_err(|error| app_adapter::failure("agent.skills.status", error))?;
    let data = receipt.data;
    let status = match data.state {
        canisend_resources::AgentSkillsStatusState::NotInstalled => "not-installed",
        canisend_resources::AgentSkillsStatusState::UpToDate => "up-to-date",
        canisend_resources::AgentSkillsStatusState::UpdateAvailable => "update-available",
        canisend_resources::AgentSkillsStatusState::Incomplete => "incomplete",
        canisend_resources::AgentSkillsStatusState::UserModified => "user-modified",
        canisend_resources::AgentSkillsStatusState::Unmanaged => "unmanaged",
    };
    success(
        "agent.skills.status",
        status,
        &data,
        vec![
            format!("CanISend workflow skills: {:?}", data.state),
            format!("Directory: {}", data.directory.display()),
            format!("Bundled version: {}", data.bundled_product_version),
            format!("Skills: {}", data.skills.len()),
        ],
    )
}

fn agent_assets_uninstall(
    workspace_path: Option<PathBuf>,
    arguments: AgentAssetsInstallArgs,
) -> CommandResult<CommandOutput> {
    let host = agent_host(arguments.host);
    let workspace = workspace_path.unwrap_or_else(|| PathBuf::from("."));
    let receipt = Application::uninstall_agent_skills(&canisend_app::AgentSkillsUninstallRequest {
        host,
        workspace,
    })
    .map_err(|error| app_adapter::failure("agent.skills.uninstall", error))?;
    let data = receipt.data;
    let status = match data.state {
        canisend_resources::AgentSkillsUninstallState::NotInstalled => "not-installed",
        canisend_resources::AgentSkillsUninstallState::Removed => "removed",
    };
    success(
        "agent.skills.uninstall",
        status,
        &data,
        vec![
            format!("CanISend workflow skills: {:?}", data.state),
            format!("Directory: {}", data.directory.display()),
            format!("Removed files: {}", data.removed_files),
        ],
    )
}

fn agent_host(host: AgentHostName) -> AgentHost {
    match host {
        AgentHostName::Codex => AgentHost::Codex,
        AgentHostName::Claude => AgentHost::Claude,
        AgentHostName::Generic => AgentHost::Generic,
    }
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

fn workspace_init(
    workspace_path: Option<PathBuf>,
    pack: BuiltInPackName,
) -> CommandResult<CommandOutput> {
    let root = workspace_path.unwrap_or_else(|| PathBuf::from("."));
    let receipt = match pack {
        BuiltInPackName::AcademicJob => Application::initialize_workspace_with_policy(
            &root,
            WorkspaceInitPolicy::PreserveExistingFiles,
        ),
        BuiltInPackName::GenericApplication => {
            Application::initialize_workspace_for_pack(&root, pack.id())
        }
    }
    .map_err(|error| app_adapter::failure("workspace.init", error))?;
    let path = receipt.data.path;
    let data = receipt.data.status;
    success(
        "workspace.init",
        "initialized",
        &data,
        vec![
            format!("Initialized CanISend workspace at {}", path.display()),
            format!("Workspace ID: {}", data.workspace_id),
            format!("Workflow Pack: {}", pack.id()),
        ],
    )
}

fn workspace_status(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root(workspace_path, "workspace.status")?;
    let model = Application::workspace_status(&root)
        .map_err(|error| app_adapter::failure("workspace.status", error))?
        .data;
    let pack_id = model.pack_id;
    let data = model.status;
    success(
        "workspace.status",
        "available",
        &data,
        vec![
            format!("Workspace: {}", data.workspace_id),
            format!("Format: {}", data.workspace_format),
            format!("Workflow Pack: {pack_id}"),
            format!("SQLite: {} ({})", data.sqlite_version, data.journal_mode),
            format!("Artifacts: {}", data.artifact_count),
        ],
    )
}

fn workspace_check(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root(workspace_path, "workspace.check")?;
    let data = Application::check_workspace(&root)
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
    let root = app_adapter::workspace_root(workspace_path, "workspace.backup")?;
    let result = Application::backup_workspace(&root, &destination)
        .map_err(|error| app_adapter::failure("workspace.backup", error))?
        .data;
    success(
        "workspace.backup",
        "verified",
        &result.manifest,
        vec![
            format!("Verified backup: {}", result.destination.display()),
            format!("Blobs: {}", result.manifest.blobs.len()),
        ],
    )
}

fn workspace_restore(backup: PathBuf, destination: PathBuf) -> CommandResult<CommandOutput> {
    let data = Application::restore_workspace(&backup, &destination)
        .map_err(|error| app_adapter::failure("workspace.restore", error))?
        .data
        .workspace;
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
    let root = app_adapter::workspace_root(workspace_path, "workspace.repair")?;
    let repaired = Application::repair_workspace(&root)
        .map_err(|error| app_adapter::failure("workspace.repair", error))?
        .data
        .repaired_projections;
    success(
        "workspace.repair",
        "repaired",
        &json!({"repaired_projections": repaired}),
        vec![format!("Repaired projections: {repaired}")],
    )
}

fn workspace_migration_preview(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let operation = "workspace-v3.migration-preview";
    let root = app_adapter::workspace_root(workspace_path, operation)?;
    let preview = Application::preview_workspace_v3_migration(&root)
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    success(
        operation,
        "ready",
        &preview,
        vec![
            format!("Applications: {}", preview.application_count),
            format!("Required backup bytes: {}", preview.required_backup_bytes),
            format!("Plan SHA-256: {}", preview.migration_plan_sha256),
            "Next: review this body-free plan and migrate with its exact digest".to_owned(),
        ],
    )
}

fn workspace_migrate(
    workspace_path: Option<PathBuf>,
    arguments: WorkspaceMigrateArgs,
) -> CommandResult<CommandOutput> {
    let operation = "workspace-v3.migrate";
    let root = app_adapter::workspace_root(workspace_path, operation)?;
    let expected_plan_sha256 =
        Sha256Digest::try_new(arguments.expected_plan_sha256).map_err(|error| {
            CommandFailure::new(
                operation,
                "invalid",
                ErrorCode::InputInvalid,
                error.to_string(),
                false,
            )
        })?;
    let result = Application::migrate_workspace_v3(
        &root,
        WorkspaceV3MigrationRequest {
            expected_plan_sha256,
            backup_destination: arguments.backup_destination,
        },
    )
    .map_err(|error| app_adapter::failure(operation, error))?
    .data;
    success(
        operation,
        "migrated",
        &result,
        vec![
            format!(
                "Migrated Applications: {}",
                result.migration.application_ids.len()
            ),
            format!("Verified backup: {}", result.backup_destination.display()),
            "Legacy authority and files were preserved for compatibility".to_owned(),
        ],
    )
}

fn job_create(
    workspace_path: Option<PathBuf>,
    arguments: JobCreateArgs,
) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root(workspace_path, "job.create")?;
    let receipt = Application::create_job(&root, &arguments.title, &arguments.institution)
        .map_err(|error| app_adapter::failure("job.create", error))?;
    let compatibility = receipt.compatibility;
    let record = receipt.data;
    success_with_compatibility(
        "job.create",
        "created",
        &record,
        vec![
            format!("Created job: {}", record.id),
            format!("{} — {}", record.title, record.institution),
        ],
        compatibility,
        CompatibilitySurface::JobCli,
    )
}

fn job_import(
    workspace_path: Option<PathBuf>,
    arguments: JobImportArgs,
) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("job.import", &arguments.job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "job.import")?;
    let receipt = if let Some(path) = arguments.file {
        Application::import_local_job_source(
            &root,
            &arguments.job_id,
            &path,
            PrivateReadConsent::granted_by_user(),
        )
    } else if let Some(url) = arguments.url {
        Application::import_url_job_source(
            &root,
            &arguments.job_id,
            &url,
            NetworkFetchConsent::granted_by_user(),
        )
    } else {
        return Err(CommandFailure::new(
            "job.import",
            "invalid",
            ErrorCode::InputInvalid,
            "exactly one of --file or --url is required",
            false,
        ));
    }
    .map_err(|error| app_adapter::failure("job.import", error))?;
    let compatibility = receipt.compatibility;
    let record = receipt.data.source;
    success_with_compatibility(
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
        compatibility,
        CompatibilitySurface::JobCli,
    )
}

fn job_list(
    workspace_path: Option<PathBuf>,
    include_archived: bool,
) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root(workspace_path, "job.list")?;
    let receipt = Application::list_jobs(&root, include_archived)
        .map_err(|error| app_adapter::failure("job.list", error))?;
    let compatibility = receipt.compatibility;
    let records = receipt.data.jobs;
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
    success_with_compatibility(
        "job.list",
        "available",
        &json!({"jobs": records}),
        human,
        compatibility,
        CompatibilitySurface::JobCli,
    )
}

fn job_show(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("job.show", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "job.show")?;
    let receipt = Application::job_detail(&root, job_id)
        .map_err(|error| app_adapter::failure("job.show", error))?;
    let compatibility = receipt.compatibility;
    let detail = receipt.data;
    let record = detail.job;
    let sources = detail.sources;
    let data = json!({"job": record, "sources": sources});
    success_with_compatibility(
        "job.show",
        "available",
        &data,
        vec![
            format!("{} — {}", record.title, record.institution),
            format!("Job ID: {}", record.id),
            format!("Sources: {}", sources.len()),
            format!("Archived: {}", record.archived),
        ],
        compatibility,
        CompatibilitySurface::JobCli,
    )
}

fn job_archive(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("job.archive", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "job.archive")?;
    let receipt = Application::archive_job(&root, job_id)
        .map_err(|error| app_adapter::failure("job.archive", error))?;
    let compatibility = receipt.compatibility;
    let record = receipt.data;
    success_with_compatibility(
        "job.archive",
        "archived",
        &record,
        vec![format!("Archived job: {}", record.id)],
        compatibility,
        CompatibilitySurface::JobCli,
    )
}

fn application_list(
    workspace_path: Option<PathBuf>,
    include_archived: bool,
) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root(workspace_path, "application.dossier.list")?;
    let list = Application::list_application_dossiers(&root, include_archived)
        .map_err(|error| app_adapter::failure("application.dossier.list", error))?
        .data;
    let human = if list.applications.is_empty() {
        vec!["No application dossiers found".to_owned()]
    } else {
        list.applications
            .iter()
            .map(|dossier| {
                let deadline = dossier
                    .metadata
                    .deadline
                    .as_deref()
                    .unwrap_or("no deadline");
                format!(
                    "{}  {} — {}  [{:?}; {}/{}; {}]",
                    dossier.job.id,
                    dossier.job.title,
                    dossier.job.institution,
                    dossier.state,
                    dossier.completed_stages,
                    dossier.total_stages,
                    deadline
                )
            })
            .collect()
    };
    success("application.dossier.list", "available", &list, human)
}

fn application_show(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("application.dossier.show", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "application.dossier.show")?;
    let receipt = Application::application_dossier(&root, job_id)
        .map_err(|error| app_adapter::failure("application.dossier.show", error))?;
    let dossier = receipt.data;
    let mut human = vec![
        format!("{} — {}", dossier.job.title, dossier.job.institution),
        format!("State: {:?}", dossier.state),
        format!(
            "Progress: {}/{} stages",
            dossier.completed_stages, dossier.total_stages
        ),
        format!(
            "Deadline: {}",
            dossier
                .metadata
                .deadline
                .as_deref()
                .unwrap_or("not recorded")
        ),
    ];
    if let Some(next) = dossier.next_actions.first() {
        human.push(format!("Next: {}", next.description));
    }
    let mut output = success("application.dossier.show", "available", &dossier, human)?;
    output.response.next_actions = receipt.next_actions;
    Ok(output)
}

const MAX_APPLICATION_V3_CANDIDATE_BYTES: u64 = 4 * 1024 * 1024;

fn application_v3_list(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let operation = "application-v3.list";
    let root = app_adapter::workspace_root(workspace_path, operation)?;
    let applications = Application::list_application_models_v3(&root)
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    let human = if applications.is_empty() {
        vec!["No canonical v3 Applications found".to_owned()]
    } else {
        applications
            .iter()
            .map(|stored| {
                format!(
                    "{}  {}  [revision {}; {:?}]",
                    stored.snapshot.application.id,
                    stored.snapshot.opportunity.title,
                    stored.snapshot.application.revision.get(),
                    stored.snapshot.application.lifecycle
                )
            })
            .collect()
    };
    success(operation, "current", &applications, human)
}

fn application_v3_show(
    workspace_path: Option<PathBuf>,
    application_id: &str,
) -> CommandResult<CommandOutput> {
    let operation = "application-v3.show";
    let root = app_adapter::workspace_root(workspace_path, operation)?;
    let stored = Application::application_model_v3(&root, application_id)
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

fn application_v3_create(
    workspace_path: Option<PathBuf>,
    arguments: ApplicationV3CreateArgs,
) -> CommandResult<CommandOutput> {
    let operation = "application-flow-v3.create";
    let root = app_adapter::workspace_root(workspace_path, operation)?;
    let request = read_application_v3_candidate::<ApplicationFlowCreateRequestV3>(
        operation,
        &arguments.candidate,
    )?;
    let model = Application::create_generic_application_v3(&root, request)
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
            "Next: confirm Requirements and commit a Plan".to_owned(),
        ],
    )
}

fn application_v3_plan(
    workspace_path: Option<PathBuf>,
    arguments: ApplicationV3CandidateArgs,
) -> CommandResult<CommandOutput> {
    let operation = "application-flow-v3.plan";
    let root = app_adapter::workspace_root(workspace_path, operation)?;
    let request = read_application_v3_candidate::<ApplicationFlowPlanRequestV3>(
        operation,
        &arguments.candidate,
    )?;
    let model = Application::plan_generic_application_v3(&root, &arguments.application, request)
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    success(
        operation,
        "confirmed",
        &model,
        vec![
            format!(
                "Application revision: {}",
                model.commit.stored.snapshot.application.revision.get()
            ),
            format!(
                "Planned Deliverables: {}",
                model
                    .commit
                    .stored
                    .snapshot
                    .plan
                    .as_ref()
                    .map_or(0, |plan| plan.deliverables.len())
            ),
            "Next: compose the approved Deliverables".to_owned(),
        ],
    )
}

fn application_v3_compose(
    workspace_path: Option<PathBuf>,
    arguments: ApplicationV3CandidateArgs,
) -> CommandResult<CommandOutput> {
    let operation = "application-flow-v3.compose";
    let root = app_adapter::workspace_root(workspace_path, operation)?;
    let request = read_application_v3_candidate::<ApplicationFlowComposeRequestV3>(
        operation,
        &arguments.candidate,
    )?;
    let model = Application::compose_generic_application_v3(&root, &arguments.application, request)
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    success(
        operation,
        "review-required",
        &model,
        vec![
            format!(
                "Application revision: {}",
                model.commit.stored.snapshot.application.revision.get()
            ),
            format!(
                "Deliverables ready for review: {}",
                model.commit.stored.snapshot.deliverables.len()
            ),
            "Next: review every current Deliverable before approval".to_owned(),
        ],
    )
}

fn application_v3_approve(
    workspace_path: Option<PathBuf>,
    arguments: ApplicationV3ApproveArgs,
) -> CommandResult<CommandOutput> {
    let operation = "application-flow-v3.approve";
    let root = app_adapter::workspace_root(workspace_path, operation)?;
    let expected_revision = Revision::try_new(arguments.expected_revision).map_err(|error| {
        CommandFailure::new(
            operation,
            "invalid",
            ErrorCode::InputInvalid,
            error.to_string(),
            false,
        )
    })?;
    let model = Application::approve_generic_application_v3(
        &root,
        &arguments.application,
        ApplicationFlowApproveRequestV3 { expected_revision },
    )
    .map_err(|error| app_adapter::failure(operation, error))?
    .data;
    success(
        operation,
        "approved",
        &model,
        vec![
            format!(
                "Application revision: {}",
                model.commit.stored.snapshot.application.revision.get()
            ),
            format!(
                "Approved Deliverables: {}",
                model.commit.stored.snapshot.deliverables.len()
            ),
            "Next: export with explicit private-artifact consent".to_owned(),
        ],
    )
}

fn application_v3_export(
    workspace_path: Option<PathBuf>,
    arguments: ApplicationV3ExportArgs,
) -> CommandResult<CommandOutput> {
    let operation = "application-flow-v3.export";
    let root = app_adapter::workspace_root(workspace_path, operation)?;
    let request = ApplicationFlowExportRequestV3::try_new(
        &arguments.application,
        arguments.expected_revision,
        &arguments.destination,
    )
    .map_err(|error| app_adapter::failure(operation, error))?;
    let consent = arguments
        .allow_private_export
        .then(PrivateExportConsent::granted_by_user);
    let model = Application::export_generic_application_v3(&root, request, consent)
        .map_err(|error| app_adapter::failure(operation, error))?
        .data;
    success(
        operation,
        "exported",
        &model,
        vec![
            format!("Exported PDFs: {}", model.render.documents.len()),
            format!("Destination: {}", model.render.destination),
            "Submission performed: no".to_owned(),
        ],
    )
}

fn read_application_v3_candidate<T>(operation: &'static str, path: &Path) -> CommandResult<T>
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
    if metadata.len() > MAX_APPLICATION_V3_CANDIDATE_BYTES {
        return Err(CommandFailure::new(
            operation,
            "invalid",
            ErrorCode::InputInvalid,
            format!(
                "candidate file exceeds the {} byte limit",
                MAX_APPLICATION_V3_CANDIDATE_BYTES
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
        > MAX_APPLICATION_V3_CANDIDATE_BYTES
    {
        return Err(CommandFailure::new(
            operation,
            "invalid",
            ErrorCode::InputInvalid,
            format!(
                "candidate file exceeds the {} byte limit",
                MAX_APPLICATION_V3_CANDIDATE_BYTES
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

fn content_list(
    workspace_path: Option<PathBuf>,
    arguments: ContentListArgs,
) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root(workspace_path, "content.catalog.list")?;
    let receipt = Application::content_catalog(&root, content_filter(arguments.filter))
        .map_err(|error| app_adapter::failure("content.catalog.list", error))?;
    let catalog = receipt.data;
    let human = if catalog.entries.is_empty() {
        vec!["No content matches the selected filters".to_owned()]
    } else {
        catalog
            .entries
            .iter()
            .map(|entry| {
                format!(
                    "{}  {}  [{:?}; {:?}; {:?}]",
                    entry.artifact.id, entry.title, entry.category, entry.status, entry.privacy
                )
            })
            .collect()
    };
    let status = if catalog.entries.is_empty() {
        "empty"
    } else {
        "available"
    };
    success("content.catalog.list", status, &catalog, human)
}

fn content_search(
    workspace_path: Option<PathBuf>,
    arguments: ContentSearchArgs,
) -> CommandResult<CommandOutput> {
    let include_private_bodies = arguments.include_private_bodies;
    let allow_private_read = arguments.allow_private_read;
    let request = ContentSearchRequest {
        query: arguments.query,
        filter: content_filter(arguments.filter),
        include_private_bodies,
        limit: arguments.limit,
    };
    let consent = allow_private_read.then(PrivateReadConsent::granted_by_user);

    // Preserve validation ordering: missing consent is reported before any
    // workspace discovery or filesystem access.
    let root = if include_private_bodies && !allow_private_read {
        PathBuf::from(".")
    } else {
        app_adapter::workspace_root(workspace_path, "content.search")?
    };
    let receipt = Application::search_content(&root, request, consent)
        .map_err(|error| app_adapter::failure("content.search", error))?;
    let search = receipt.data;
    let mut human = vec![
        format!("Matches: {}", search.total_matches),
        format!(
            "Index: {} metadata, {} private bodies",
            search.index.metadata_entries, search.index.private_body_entries
        ),
    ];
    human.extend(search.results.iter().map(|result| {
        let snippet = result
            .snippet
            .as_deref()
            .map(|value| format!(" — {value}"))
            .unwrap_or_default();
        format!(
            "{}  {}  [score {}; {:?}]{}",
            result.entry.artifact.id,
            result.entry.title,
            result.score,
            result.matched_fields,
            snippet
        )
    }));
    let status = if search.results.is_empty() {
        "empty"
    } else {
        "available"
    };
    let mut output = success("content.search", status, &search, human)?;
    output.response.warnings = receipt.warnings;
    Ok(output)
}

fn content_filter(arguments: ContentFilterArgs) -> ContentCatalogFilter {
    ContentCatalogFilter {
        job_id: arguments.job,
        category: arguments.category.map(Into::into),
        stage: arguments.stage.map(Into::into),
        status: arguments.status.map(Into::into),
        privacy: arguments.privacy.map(Into::into),
        created_after: arguments.created_after,
        created_before: arguments.created_before,
    }
}

fn profile_source_add(
    workspace_path: Option<PathBuf>,
    arguments: ProfileSourceAddArgs,
) -> CommandResult<CommandOutput> {
    let sensitivity = match arguments.sensitivity {
        ProfileSensitivityName::Public => PrivacyClassification::Public,
        ProfileSensitivityName::PrivateLocal => PrivacyClassification::PrivateLocal,
    };
    let root = app_adapter::workspace_root(workspace_path, "profile.source.add")?;
    let receipt = Application::import_profile_source(
        &root,
        &arguments.file,
        sensitivity,
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("profile.source.add", error))?;
    let source = receipt.data.source;
    let mut output = success(
        "profile.source.add",
        "imported",
        &source,
        vec![
            format!("Imported profile source: {}", source.id),
            format!("Kind: {:?}", source.kind),
            format!("Sensitivity: {:?}", source.sensitivity),
        ],
    )?;
    output.response.artifacts = receipt.artifacts;
    Ok(output)
}

fn profile_source_list(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root(workspace_path, "profile.source.list")?;
    let profile = Application::list_profile_sources(&root)
        .map_err(|error| app_adapter::failure("profile.source.list", error))?
        .data;
    let source_count = profile.sources.len();
    success(
        "profile.source.list",
        "available",
        &profile,
        vec![format!("Profile sources: {source_count}")],
    )
}

fn profile_source_show(
    workspace_path: Option<PathBuf>,
    source_id: &str,
) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("profile.source.show", source_id)?;
    let root = app_adapter::workspace_root(workspace_path, "profile.source.show")?;
    let source = Application::profile_source(&root, source_id)
        .map_err(|error| app_adapter::failure("profile.source.show", error))?
        .data;
    success(
        "profile.source.show",
        "available",
        &source,
        vec![
            format!("Profile source: {}", source.id),
            format!("Kind: {:?}", source.kind),
        ],
    )
}

fn profile_evidence_proposed(
    workspace_path: Option<PathBuf>,
    job_id: &str,
) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("profile.evidence.proposed", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "profile.evidence.proposed")?;
    let proposed = Application::proposed_profile_evidence(
        &root,
        job_id,
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("profile.evidence.proposed", error))?
    .data;
    success(
        "profile.evidence.proposed",
        "available",
        &proposed,
        vec![
            format!("Evidence proposal: {}", proposed.id),
            format!("Items proposed: {}", proposed.items.len()),
            "Agent proposals remain unconfirmed until an explicit user decision".to_owned(),
        ],
    )
}

fn profile_evidence_export(
    workspace_path: Option<PathBuf>,
    arguments: ProfileEvidenceExportArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("profile.evidence.export", &arguments.job)?;
    let root = app_adapter::workspace_root(workspace_path, "profile.evidence.export")?;
    let template = Application::profile_evidence_template(
        &root,
        job_id.as_str(),
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("profile.evidence.export", error))?
    .data;
    write_private_json_new(&arguments.destination, &template)
        .map_err(|error| io_adapter_failure("profile.evidence.export", error))?;
    let mut output = success(
        "profile.evidence.export",
        "exported",
        &json!({
            "job_id": job_id,
            "destination": arguments.destination,
            "evidence_count": template.items.len(),
            "profile_revision": template.profile_revision,
            "schema": PublicSchemaId::EvidenceCatalog.as_str(),
        }),
        vec![
            format!(
                "Exported evidence candidate: {}",
                arguments.destination.display()
            ),
            format!("Evidence items: {}", template.items.len()),
        ],
    )?;
    output.response.next_actions.push(NextAction {
        action: format!(
            "canisend profile evidence confirm --job {} --file {} --json",
            job_id,
            arguments.destination.display()
        ),
        description: "Review corrections, sensitivity, and excluded flags, then explicitly confirm"
            .to_owned(),
    });
    Ok(output)
}

fn profile_evidence_confirm(
    workspace_path: Option<PathBuf>,
    arguments: ProfileEvidenceConfirmArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("profile.evidence.confirm", &arguments.job)?;
    let candidate = read_criteria_file(&arguments.file)
        .map_err(|error| io_adapter_failure("profile.evidence.confirm", error))?;
    let root = app_adapter::workspace_root(workspace_path, "profile.evidence.confirm")?;
    let receipt = Application::confirm_profile_evidence(
        &root,
        job_id.as_str(),
        &candidate,
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("profile.evidence.confirm", error))?;
    let confirmed = receipt.data;
    let artifact = receipt.artifacts.first().cloned().ok_or_else(|| {
        CommandFailure::new(
            "profile.evidence.confirm",
            "invariant-failed",
            ErrorCode::InternalInvariantFailed,
            "profile evidence confirmation returned no artifact",
            false,
        )
    })?;
    let mut output = success(
        "profile.evidence.confirm",
        "confirmed",
        &confirmed,
        vec![
            format!("Confirmed evidence artifact: {}", artifact.id),
            format!("Artifact revision: {}", artifact.revision.get()),
            format!("Evidence items: {}", confirmed.items.len()),
        ],
    )?;
    output.response.artifacts.push(artifact);
    Ok(output)
}

fn profile_evidence_show(
    workspace_path: Option<PathBuf>,
    job_id: &str,
) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("profile.evidence.show", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "profile.evidence.show")?;
    let confirmed = Application::confirmed_profile_evidence(
        &root,
        job_id,
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("profile.evidence.show", error))?
    .data;
    success(
        "profile.evidence.show",
        "available",
        &confirmed,
        vec![
            format!("Confirmed evidence: {}", confirmed.id),
            format!("Revision: {}", confirmed.revision.get()),
            format!("Evidence items: {}", confirmed.items.len()),
        ],
    )
}

fn discovery_import(
    workspace_path: Option<PathBuf>,
    arguments: DiscoveryImportArgs,
) -> CommandResult<CommandOutput> {
    let preview = Application::preview_discovery_import(
        &DiscoveryImportRequest {
            path: arguments.file,
            source_name: arguments.source_name,
            source_url: arguments.source_url,
            host_agent: arguments.host_agent,
        },
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("discovery.import", error))?;
    let report = if arguments.dry_run {
        preview.data
    } else {
        let root = app_adapter::workspace_root(workspace_path, "discovery.import")?;
        Application::commit_discovery_import(&root, preview.data)
            .map_err(|error| app_adapter::failure("discovery.import", error))?
            .data
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
    let adapters = Application::discovery_adapters()
        .map_err(|error| app_adapter::failure("discovery.adapters", error))?
        .data
        .adapters;
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
    let adapter_id = adapter.id();
    let preview = Application::preview_discovery_refresh(
        &DiscoveryRefreshRequest {
            adapter: adapter.into(),
            endpoint,
            source_name,
            organization,
        },
        NetworkFetchConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("discovery.refresh", error))?;
    let report = if dry_run {
        preview.data
    } else {
        let root = app_adapter::workspace_root(workspace_path, "discovery.refresh")?;
        Application::commit_discovery_refresh(&root, preview.data)
            .map_err(|error| app_adapter::failure("discovery.refresh", error))?
            .data
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
            format!("{adapter_id} source: {status}"),
            format!("Accepted leads: {}", report.accepted),
            format!("Rejected rows: {}", report.rejected),
        ],
    )
}

fn discovery_sources(workspace_path: Option<PathBuf>) -> CommandResult<CommandOutput> {
    let root = app_adapter::workspace_root(workspace_path, "discovery.sources")?;
    let sources = Application::list_discovery_sources(&root)
        .map_err(|error| app_adapter::failure("discovery.sources", error))?
        .data
        .sources;
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
    let root = app_adapter::workspace_root(workspace_path, "discovery.list")?;
    let leads = Application::list_discovery_leads(&root, include_history)
        .map_err(|error| app_adapter::failure("discovery.list", error))?
        .data
        .leads;
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
    let _ = parse_entity_id("discovery.show", lead_id)?;
    let root = app_adapter::workspace_root(workspace_path, "discovery.show")?;
    let lead = Application::discovery_lead(&root, lead_id)
        .map_err(|error| app_adapter::failure("discovery.show", error))?
        .data;
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
    let _ = parse_entity_id("discovery.suggest", lead_id)?;
    let root = app_adapter::workspace_root(workspace_path, "discovery.suggest")?;
    let suggestions = Application::discovery_suggestions(&root, lead_id, limit)
        .map_err(|error| app_adapter::failure("discovery.suggest", error))?
        .data
        .suggestions;
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
    let _ = parse_entity_id("discovery.promote", lead_id)?;
    let root = app_adapter::workspace_root(workspace_path, "discovery.promote")?;
    let receipt = Application::promote_discovery_lead(&root, lead_id)
        .map_err(|error| app_adapter::failure("discovery.promote", error))?;
    let promoted = receipt.data;
    let import_action = receipt
        .next_actions
        .first()
        .map(|action| action.action.clone())
        .ok_or_else(|| {
            CommandFailure::new(
                "discovery.promote",
                "invariant-failed",
                ErrorCode::InternalInvariantFailed,
                "discovery promotion returned no next action",
                false,
            )
        })?;
    let mut output = success(
        "discovery.promote",
        "promoted",
        &promoted,
        vec![
            format!("Promoted lead into job: {}", promoted.job.id),
            format!("Next: {import_action}"),
        ],
    )?;
    output.response.next_actions = receipt.next_actions;
    Ok(output)
}

fn task_prepare(
    workspace_path: Option<PathBuf>,
    arguments: TaskPrepareArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("task.prepare", &arguments.job)?;
    let root = app_adapter::workspace_root(workspace_path, "task.prepare")?;
    let descriptor = Application::prepare_task(
        &root,
        TaskPrepareRequest {
            job_id,
            operation: arguments.operation.into(),
            mode: arguments.mode.into(),
        },
    )
    .map_err(|error| task_application_failure("task.prepare", error))?
    .data;
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
    let root = app_adapter::workspace_root(workspace_path, "task.show")?;
    let state = Application::task_state(&root, task_id.as_str())
        .map_err(|error| task_application_failure("task.show", error))?
        .data;
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

fn task_inputs(
    workspace_path: Option<PathBuf>,
    arguments: TaskInputsArgs,
) -> CommandResult<CommandOutput> {
    if !arguments.allow_private_read {
        let mut failure = CommandFailure::new(
            "task.inputs",
            "consent-required",
            ErrorCode::ConsentRequired,
            "read-private-inputs consent must be explicitly confirmed",
            false,
        );
        failure.error.remediation = Some(NextAction {
            action: "obtain user approval, then repeat with --allow-private-read".to_owned(),
            description:
                "The command exports only artifacts declared in the task's private read scope"
                    .to_owned(),
        });
        return Err(failure);
    }
    let task_id = parse_entity_id("task.inputs", &arguments.task_id)?;
    let root = app_adapter::workspace_root(workspace_path, "task.inputs")?;
    let request = TaskInputExportRequest {
        task_id,
        destination: arguments.destination.clone(),
    };
    let provider_send_consent = arguments
        .allow_provider_send
        .then(ProviderSendConsent::granted_by_user);
    let exported = Application::export_task_inputs(
        &root,
        request,
        Some(PrivateReadConsent::granted_by_user()),
        provider_send_consent,
    )
    .map_err(|error| task_application_failure("task.inputs", error))?
    .data;
    success(
        "task.inputs",
        "exported",
        &exported,
        vec![
            format!("Exported task inputs: {}", exported.task_id),
            format!("Directory: {}", arguments.destination.display()),
            format!("Files: {}", exported.files.len()),
            format!("Manifest SHA-256: {}", exported.manifest_sha256),
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
    let root = app_adapter::workspace_root(workspace_path, "task.complete")?;
    let result = Application::commit_task_completion(&root, request)
        .map_err(|error| task_application_failure("task.complete", error))?
        .data;
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
    let root = app_adapter::workspace_root(workspace_path, "task.cancel")?;
    let state = Application::cancel_task(&root, task_id.as_str())
        .map_err(|error| task_application_failure("task.cancel", error))?
        .data;
    success(
        "task.cancel",
        "cancelled",
        &state,
        vec![format!("Cancelled task: {}", state.descriptor.id)],
    )
}

fn criteria_proposed(
    workspace_path: Option<PathBuf>,
    job_id: &str,
) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("criteria.proposed", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "criteria.proposed")?;
    let proposed =
        Application::proposed_job_criteria(&root, job_id, PrivateReadConsent::granted_by_user())
            .map_err(|error| app_adapter::failure("criteria.proposed", error))?
            .data;
    success(
        "criteria.proposed",
        "available",
        &proposed,
        vec![
            format!("Parsed job proposal: {}", proposed.id),
            format!("Criteria proposed: {}", proposed.criteria.len()),
            "These criteria are unconfirmed until the user runs criteria confirm".to_owned(),
        ],
    )
}

fn criteria_export(
    workspace_path: Option<PathBuf>,
    arguments: CriteriaExportArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("criteria.export", &arguments.job)?;
    let root = app_adapter::workspace_root(workspace_path, "criteria.export")?;
    let template = Application::job_criteria_template(
        &root,
        job_id.as_str(),
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("criteria.export", error))?
    .data;
    write_private_json_new(&arguments.destination, &template)
        .map_err(|error| io_adapter_failure("criteria.export", error))?;
    let mut output = success(
        "criteria.export",
        "exported",
        &json!({
            "job_id": job_id,
            "destination": arguments.destination,
            "criterion_count": template.criteria.len(),
            "schema": PublicSchemaId::CriteriaSet.as_str(),
        }),
        vec![
            format!(
                "Exported criteria candidate: {}",
                arguments.destination.display()
            ),
            format!("Criteria: {}", template.criteria.len()),
        ],
    )?;
    output.response.next_actions.push(NextAction {
        action: format!(
            "canisend criteria confirm --job {} --file {} --json",
            job_id,
            arguments.destination.display()
        ),
        description: "Review or correct the JSON, then explicitly confirm it".to_owned(),
    });
    Ok(output)
}

fn criteria_confirm(
    workspace_path: Option<PathBuf>,
    arguments: CriteriaConfirmArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("criteria.confirm", &arguments.job)?;
    let candidate = read_criteria_file(&arguments.file)
        .map_err(|error| io_adapter_failure("criteria.confirm", error))?;
    let root = app_adapter::workspace_root(workspace_path, "criteria.confirm")?;
    let receipt = Application::confirm_job_criteria(
        &root,
        job_id.as_str(),
        &candidate,
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("criteria.confirm", error))?;
    let confirmed = receipt.data;
    let artifact = receipt.artifacts.first().cloned().ok_or_else(|| {
        CommandFailure::new(
            "criteria.confirm",
            "invariant-failed",
            ErrorCode::InternalInvariantFailed,
            "criteria confirmation returned no artifact",
            false,
        )
    })?;
    let mut output = success(
        "criteria.confirm",
        "confirmed",
        &confirmed,
        vec![
            format!("Confirmed criteria artifact: {}", artifact.id),
            format!("Criteria: {}", confirmed.criteria.len()),
        ],
    )?;
    output.response.artifacts.push(artifact);
    Ok(output)
}

fn criteria_show(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("criteria.show", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "criteria.show")?;
    let confirmed =
        Application::confirmed_job_criteria(&root, job_id, PrivateReadConsent::granted_by_user())
            .map_err(|error| app_adapter::failure("criteria.show", error))?
            .data;
    success(
        "criteria.show",
        "available",
        &confirmed,
        vec![
            format!("Confirmed criteria: {}", confirmed.id),
            format!("Criteria: {}", confirmed.criteria.len()),
        ],
    )
}

fn match_show(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("match.show", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "match.show")?;
    let matches =
        Application::current_evidence_matches(&root, job_id, PrivateReadConsent::granted_by_user())
            .map_err(|error| app_adapter::failure("match.show", error))?
            .data;
    success(
        "match.show",
        "available",
        &matches,
        vec![
            format!("Evidence matches: {}", matches.id),
            format!("Matches: {}", matches.matches.len()),
        ],
    )
}

fn plan_export(
    workspace_path: Option<PathBuf>,
    arguments: PlanExportArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("plan.export", &arguments.job)?;
    let root = app_adapter::workspace_root(workspace_path, "plan.export")?;
    let template = Application::application_plan_template(
        &root,
        job_id.as_str(),
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("plan.export", error))?
    .data;
    write_private_json_new(&arguments.destination, &template)
        .map_err(|error| io_adapter_failure("plan.export", error))?;
    let mut output = success(
        "plan.export",
        "exported",
        &json!({
            "job_id": job_id,
            "destination": arguments.destination,
            "decision": template.decision,
            "document_count": template.documents.len(),
            "blocker_count": template.blockers.len(),
            "schema": PublicSchemaId::ApplicationPlanCandidate.as_str(),
        }),
        vec![
            format!(
                "Exported application plan candidate: {}",
                arguments.destination.display()
            ),
            format!("Derived blockers: {}", template.blockers.len()),
            "The safe default decision is hold; review it before confirmation".to_owned(),
        ],
    )?;
    output.response.next_actions.push(NextAction {
        action: format!(
            "canisend plan confirm --job {} --file {} --json",
            job_id,
            arguments.destination.display()
        ),
        description: "Review the decision, strategy, and document requirements, then confirm"
            .to_owned(),
    });
    Ok(output)
}

fn plan_confirm(
    workspace_path: Option<PathBuf>,
    arguments: PlanConfirmArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("plan.confirm", &arguments.job)?;
    let candidate = read_criteria_file(&arguments.file)
        .map_err(|error| io_adapter_failure("plan.confirm", error))?;
    let root = app_adapter::workspace_root(workspace_path, "plan.confirm")?;
    let receipt = Application::confirm_application_plan(
        &root,
        job_id.as_str(),
        &candidate,
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("plan.confirm", error))?;
    let confirmed = receipt.data;
    let artifact = receipt.artifacts.first().cloned().ok_or_else(|| {
        CommandFailure::new(
            "plan.confirm",
            "invariant-failed",
            ErrorCode::InternalInvariantFailed,
            "plan confirmation returned no artifact",
            false,
        )
    })?;
    let mut output = success(
        "plan.confirm",
        "confirmed",
        &confirmed,
        vec![
            format!("Confirmed application plan: {}", confirmed.id),
            format!("Decision: {:?}", confirmed.decision),
            format!("Documents: {}", confirmed.documents.len()),
            format!("Derived blockers: {}", confirmed.blockers.len()),
        ],
    )?;
    output.response.artifacts.push(artifact);
    Ok(output)
}

fn plan_show(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("plan.show", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "plan.show")?;
    let plan =
        Application::current_application_plan(&root, job_id, PrivateReadConsent::granted_by_user())
            .map_err(|error| app_adapter::failure("plan.show", error))?
            .data;
    success(
        "plan.show",
        "available",
        &plan,
        vec![
            format!("Application plan: {}", plan.id),
            format!("Decision: {:?}", plan.decision),
            format!("Documents: {}", plan.documents.len()),
            format!("Derived blockers: {}", plan.blockers.len()),
        ],
    )
}

fn document_list(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("document.list", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "document.list")?;
    let documents =
        Application::current_documents(&root, job_id, PrivateReadConsent::granted_by_user())
            .map_err(|error| app_adapter::failure("document.list", error))?
            .data;
    success(
        "document.list",
        "available",
        &documents,
        vec![
            format!("Job: {job_id}"),
            format!("Current structured drafts: {}", documents.len()),
        ],
    )
}

fn document_show(
    workspace_path: Option<PathBuf>,
    job_id: &str,
    kind: DocumentKindName,
) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("document.show", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "document.show")?;
    let document = Application::current_document(
        &root,
        job_id,
        DocumentKind::from(kind),
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("document.show", error))?
    .data;
    success(
        "document.show",
        "available",
        &document,
        vec![
            format!("Document: {}", document.id),
            format!("Kind: {:?}", document.kind),
            format!("Sections: {}", document.sections.len()),
            format!("Placeholders: {}", document.placeholders.len()),
        ],
    )
}

fn document_set(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("document.set", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "document.set")?;
    let receipt = Application::current_document_set(&root, job_id)
        .map_err(|error| app_adapter::failure("document.set", error))?;
    let set = receipt.data;
    let mut output = success(
        "document.set",
        "complete",
        &set,
        vec![
            format!("Document set: {}", set.id),
            format!("Current documents: {}", set.documents.len()),
        ],
    )?;
    output.response.artifacts.extend(receipt.artifacts);
    Ok(output)
}

fn review_export(
    workspace_path: Option<PathBuf>,
    arguments: ReviewExportArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("review.export", &arguments.job)?;
    let root = app_adapter::workspace_root(workspace_path, "review.export")?;
    let candidate = Application::review_disposition_template(
        &root,
        job_id.as_str(),
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("review.export", error))?
    .data;
    write_private_json_new(&arguments.destination, &candidate)
        .map_err(|error| io_adapter_failure("review.export", error))?;
    let mut output = success(
        "review.export",
        "exported",
        &json!({
            "job_id": job_id,
            "destination": arguments.destination,
            "human_finding_count": candidate.decisions.len(),
            "review_artifact": candidate.review_artifact
        }),
        vec![
            format!(
                "Exported review dispositions: {}",
                arguments.destination.display()
            ),
            format!("Human-review findings: {}", candidate.decisions.len()),
        ],
    )?;
    output.response.next_actions.push(NextAction {
        action: format!(
            "canisend review confirm --job {} --file {} --json",
            job_id,
            arguments.destination.display()
        ),
        description: "Select accepted-risk or dismissed only after explicit user review".to_owned(),
    });
    Ok(output)
}

fn review_confirm(
    workspace_path: Option<PathBuf>,
    arguments: ReviewConfirmArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("review.confirm", &arguments.job)?;
    let candidate = read_criteria_file(&arguments.file)
        .map_err(|error| io_adapter_failure("review.confirm", error))?;
    let root = app_adapter::workspace_root(workspace_path, "review.confirm")?;
    let receipt = Application::confirm_review_dispositions(
        &root,
        job_id.as_str(),
        &candidate,
        PrivateReadConsent::granted_by_user(),
    )
    .map_err(|error| app_adapter::failure("review.confirm", error))?;
    let review = receipt.data;
    let mut output = review_output("review.confirm", "confirmed", &review)?;
    output.response.artifacts.extend(receipt.artifacts);
    Ok(output)
}

fn review_show(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("review.show", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "review.show")?;
    let review = Application::current_review(&root, job_id, PrivateReadConsent::granted_by_user())
        .map_err(|error| app_adapter::failure("review.show", error))?
        .data;
    review_output("review.show", "available", &review)
}

fn review_output(
    operation: &'static str,
    status: &'static str,
    review: &canisend_contracts::ReviewFindingsRecord,
) -> CommandResult<CommandOutput> {
    let deterministic_blockers = review
        .findings
        .iter()
        .filter(|finding| {
            finding.authority == canisend_contracts::FindingAuthority::Deterministic
                && finding.severity == canisend_contracts::FindingSeverity::Blocker
                && finding.status == canisend_contracts::FindingStatus::Open
        })
        .count();
    let pending_human = review
        .findings
        .iter()
        .filter(|finding| {
            finding.authority == canisend_contracts::FindingAuthority::HumanReview
                && finding.status == canisend_contracts::FindingStatus::Open
        })
        .count();
    success(
        operation,
        status,
        review,
        vec![
            format!("Review findings: {}", review.id),
            format!("Total findings: {}", review.findings.len()),
            format!("Deterministic blockers: {deterministic_blockers}"),
            format!("Pending human findings: {pending_human}"),
        ],
    )
}

fn package_check(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("package.check", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "package.check")?;
    let receipt = Application::check_package(&root, job_id)
        .map_err(|error| app_adapter::failure("package.check", error))?;
    let manifest = receipt.data;
    let mut output = package_output("package.check", &manifest)?;
    output.response.artifacts.extend(receipt.artifacts);
    Ok(output)
}

fn package_show(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("package.show", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "package.show")?;
    let manifest = Application::current_package(&root, job_id)
        .map_err(|error| app_adapter::failure("package.show", error))?
        .data;
    package_output("package.show", &manifest)
}

fn package_output(
    operation: &'static str,
    manifest: &canisend_contracts::PackageManifestRecord,
) -> CommandResult<CommandOutput> {
    let state = match manifest.readiness.state {
        canisend_contracts::ReadinessState::Blocked => "blocked",
        canisend_contracts::ReadinessState::NeedsReview => "needs-review",
        canisend_contracts::ReadinessState::ReadyToExport => "ready-to-export",
        canisend_contracts::ReadinessState::Exported => "exported",
    };
    let reason_codes = manifest
        .readiness
        .reasons
        .iter()
        .filter_map(|reason| {
            serde_json::to_value(reason.code)
                .ok()
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
        })
        .collect::<Vec<_>>();
    success(
        operation,
        state,
        manifest,
        vec![
            format!("Package manifest: {}", manifest.id),
            format!("Readiness: {state}"),
            format!("Reasons: {}", reason_codes.join(", ")),
            "Submission performed: no".to_owned(),
        ],
    )
}

fn package_export(
    workspace_path: Option<PathBuf>,
    arguments: PackageExportArgs,
) -> CommandResult<CommandOutput> {
    let request = PackageExportRequest::try_new(&arguments.job, &arguments.destination)
        .map_err(|error| app_adapter::failure("package.export", error))?;
    let destination = request.destination.clone();
    let consent = arguments
        .allow_private_export
        .then(PrivateExportConsent::granted_by_user);
    let root = if consent.is_some() {
        app_adapter::workspace_root(workspace_path, "package.export")?
    } else {
        PathBuf::from(".")
    };
    let result = Application::export_package(&root, request, consent)
        .map_err(|error| app_adapter::failure("package.export", error))?;
    let receipt = result.data;
    let mut output = success(
        "package.export",
        "exported",
        &receipt,
        vec![
            format!("Export receipt: {}", receipt.id),
            format!("Directory: {}", destination),
            format!("Managed projections: {}", receipt.projections.len()),
            "Submission performed: no".to_owned(),
        ],
    )?;
    output.response.artifacts.extend(result.artifacts);
    Ok(output)
}

fn package_exports(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("package.exports", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "package.exports")?;
    let result = Application::current_package_export(&root, job_id)
        .map_err(|error| app_adapter::failure("package.exports", error))?;
    let receipt = result.data;
    let mut output = success(
        "package.exports",
        "available",
        &receipt,
        vec![
            format!("Export receipt: {}", receipt.id),
            format!("Managed projections: {}", receipt.projections.len()),
            "Submission performed: no".to_owned(),
        ],
    )?;
    output.response.artifacts.extend(result.artifacts);
    Ok(output)
}

fn package_reconcile(
    workspace_path: Option<PathBuf>,
    job_id: &str,
) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("package.reconcile", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "package.reconcile")?;
    let records = Application::reconcile_package_projections(&root, job_id)
        .map_err(|error| app_adapter::failure("package.reconcile", error))?
        .data;
    let edited = records
        .iter()
        .filter(|record| {
            record.projection.edit_status == canisend_contracts::ProjectionEditStatus::Edited
        })
        .count();
    let missing = records
        .iter()
        .filter(|record| {
            record.projection.edit_status == canisend_contracts::ProjectionEditStatus::Missing
        })
        .count();
    success(
        "package.reconcile",
        if edited == 0 && missing == 0 {
            "current"
        } else {
            "attention-required"
        },
        &records,
        vec![
            format!("Managed projections: {}", records.len()),
            format!("Edited: {edited}"),
            format!("Missing: {missing}"),
            "Authoritative structured artifacts changed: no".to_owned(),
        ],
    )
}

fn package_replace(
    workspace_path: Option<PathBuf>,
    arguments: PackageProjectionArgs,
) -> CommandResult<CommandOutput> {
    let request = ProjectionReplaceRequest::try_new(&arguments.job, &arguments.path)
        .map_err(|error| app_adapter::failure("package.replace", error))?;
    let path = request.path.clone();
    let root = app_adapter::workspace_root(workspace_path, "package.replace")?;
    let record = Application::replace_package_projection(&root, request)
        .map_err(|error| app_adapter::failure("package.replace", error))?
        .data;
    success(
        "package.replace",
        "replaced",
        &record,
        vec![
            format!("Restored projection: {path}"),
            "User edit preserved: no".to_owned(),
            "Authoritative structured artifacts changed: no".to_owned(),
        ],
    )
}

fn package_copy_as_new(
    workspace_path: Option<PathBuf>,
    arguments: PackageCopyAsNewArgs,
) -> CommandResult<CommandOutput> {
    let request = ProjectionCopyAsNewRequest::try_new(
        &arguments.job,
        &arguments.path,
        &arguments.destination,
    )
    .map_err(|error| app_adapter::failure("package.copy-as-new", error))?;
    let path = request.path.clone();
    let destination = request.destination.clone();
    let root = app_adapter::workspace_root(workspace_path, "package.copy-as-new")?;
    let record = Application::copy_package_projection_as_new(&root, request)
        .map_err(|error| app_adapter::failure("package.copy-as-new", error))?
        .data;
    success(
        "package.copy-as-new",
        "preserved-and-restored",
        &record,
        vec![
            format!("Preserved user edit: {destination}"),
            format!("Restored managed projection: {path}"),
            "Authoritative structured artifacts changed: no".to_owned(),
        ],
    )
}

fn render_build(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("render.build", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "render.build")?;
    let receipt = Application::build_render(&root, job_id)
        .map_err(|error| app_adapter::failure("render.build", error))?;
    let manifest = receipt.data;
    let mut output = render_output("render.build", "rendered", &manifest)?;
    output.response.artifacts.extend(receipt.artifacts);
    Ok(output)
}

fn render_show(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("render.show", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "render.show")?;
    let receipt = Application::current_render(&root, job_id)
        .map_err(|error| app_adapter::failure("render.show", error))?;
    let manifest = receipt.data;
    let mut output = render_output("render.show", "available", &manifest)?;
    output.response.artifacts.extend(receipt.artifacts);
    Ok(output)
}

fn render_export(
    workspace_path: Option<PathBuf>,
    arguments: RenderExportArgs,
) -> CommandResult<CommandOutput> {
    let request = RenderExportRequest::try_new(&arguments.job, &arguments.destination)
        .map_err(|error| app_adapter::failure("render.export", error))?;
    let destination = request.destination.clone();
    let consent = arguments
        .allow_private_export
        .then(PrivateExportConsent::granted_by_user);
    let root = if consent.is_some() {
        app_adapter::workspace_root(workspace_path, "render.export")?
    } else {
        PathBuf::from(".")
    };
    let result = Application::export_render(&root, request, consent)
        .map_err(|error| app_adapter::failure("render.export", error))?;
    let data = result.data;
    let mut output = success(
        "render.export",
        "exported",
        &data,
        vec![
            format!("Directory: {destination}"),
            format!("Exported files: {}", data.files.len()),
            "Submission performed: no".to_owned(),
        ],
    )?;
    output.response.artifacts.extend(result.artifacts);
    Ok(output)
}

fn render_output(
    operation: &'static str,
    status: &'static str,
    manifest: &canisend_contracts::RenderManifestRecord,
) -> CommandResult<CommandOutput> {
    let pages = manifest
        .documents
        .iter()
        .map(|document| u64::from(document.page_count))
        .sum::<u64>();
    let bytes = manifest
        .documents
        .iter()
        .map(|document| document.byte_count)
        .sum::<u64>();
    success(
        operation,
        status,
        manifest,
        vec![
            format!("Render manifest: {}", manifest.id),
            format!("Rendered documents: {}", manifest.documents.len()),
            format!("PDF pages: {pages}"),
            format!("PDF bytes: {bytes}"),
            "Submission performed: no".to_owned(),
        ],
    )
}

fn write_private_json_new<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), IoAdapterError> {
    if path.components().any(|component| {
        matches!(component, Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case(".canisend"))
    }) {
        return Err(IoAdapterError::UnsafeLocalFile(path.to_path_buf()));
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return Err(IoAdapterError::UnsupportedLocalType(path.to_path_buf()));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent_metadata =
        std::fs::symlink_metadata(parent).map_err(|source| IoAdapterError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(IoAdapterError::UnsafeLocalFile(path.to_path_buf()));
    }
    let canonical_parent = std::fs::canonicalize(parent).map_err(|source| IoAdapterError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    if canonical_parent.components().any(|component| {
        matches!(component, Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case(".canisend"))
    }) {
        return Err(IoAdapterError::UnsafeLocalFile(path.to_path_buf()));
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|source| IoAdapterError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| IoAdapterError::CandidateInput(error.to_string()))?;
    bytes.push(b'\n');
    file.write_all(&bytes)
        .map_err(|source| IoAdapterError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    file.sync_all().map_err(|source| IoAdapterError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn workflow_start(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("workflow.start", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "workflow.start")?;
    let status = Application::start_workflow(&root, job_id)
        .map_err(|error| app_adapter::failure("workflow.start", error))?
        .data;
    workflow_command_output("workflow.start", "started", status)
}

fn workflow_status(workspace_path: Option<PathBuf>, job_id: &str) -> CommandResult<CommandOutput> {
    let _ = parse_entity_id("workflow.status", job_id)?;
    let root = app_adapter::workspace_root(workspace_path, "workflow.status")?;
    let status = Application::workflow_status(&root, job_id)
        .map_err(|error| app_adapter::failure("workflow.status", error))?
        .data;
    workflow_command_output("workflow.status", "available", status)
}

fn workflow_begin(
    workspace_path: Option<PathBuf>,
    arguments: WorkflowBeginArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("workflow.begin", &arguments.job)?;
    let stage = WorkflowStage::from(arguments.stage);
    let mode = ExecutionMode::from(arguments.mode);
    let root = app_adapter::workspace_root(workspace_path, "workflow.begin")?;
    let status = Application::begin_workflow_stage(
        &root,
        WorkflowBeginRequest {
            job_id,
            stage,
            mode,
        },
    )
    .map_err(|error| app_adapter::failure("workflow.begin", error))?
    .data
    .status;
    workflow_command_output("workflow.begin", "begun", status)
}

fn workflow_complete(
    workspace_path: Option<PathBuf>,
    arguments: WorkflowCompleteArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("workflow.complete", &arguments.job)?;
    let artifact_id = parse_entity_id("workflow.complete", &arguments.artifact)?;
    let stage = WorkflowStage::from(arguments.stage);
    let root = app_adapter::workspace_root(workspace_path, "workflow.complete")?;
    let receipt = Application::complete_workflow_stage(
        &root,
        WorkflowCompleteRequest {
            job_id,
            stage,
            artifact_id,
        },
    )
    .map_err(|error| app_adapter::failure("workflow.complete", error))?;
    let mut output = workflow_command_output("workflow.complete", "complete", receipt.data.status)?;
    output.response.artifacts = receipt.artifacts;
    Ok(output)
}

fn workflow_rerun(
    workspace_path: Option<PathBuf>,
    arguments: WorkflowRerunArgs,
) -> CommandResult<CommandOutput> {
    let job_id = parse_entity_id("workflow.rerun", &arguments.job)?;
    let stage = WorkflowStage::from(arguments.stage);
    let root = app_adapter::workspace_root(workspace_path, "workflow.rerun")?;
    let status = Application::rerun_workflow_stage(&root, WorkflowRerunRequest { job_id, stage })
        .map_err(|error| app_adapter::failure("workflow.rerun", error))?
        .data
        .status;
    workflow_command_output("workflow.rerun", "ready", status)
}

fn workflow_command_output(
    operation: &'static str,
    status_name: &'static str,
    status: canisend_contracts::WorkflowStatusData,
) -> CommandResult<CommandOutput> {
    let mut output = success(
        operation,
        status_name,
        &status,
        vec![
            format!("Workflow: {}", status.run_id),
            format!("Job: {}", status.job_id),
            format!("Status: {:?}", status.status),
            format!("Blockers: {}", status.blockers.len()),
        ],
    )?;
    output.response.next_actions = status.next_actions.clone();
    Ok(output)
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
    let mut failure = CommandFailure::new(operation, status, code, error.to_string(), retryable);
    match error {
        IoAdapterError::PdfTextUnavailable => {
            failure.error.remediation = Some(NextAction {
                action: "provide a text-based PDF, Markdown, or plain-text advert".to_owned(),
                description:
                    "CanISend does not run OCR; extract and review scanned text with a trusted tool before importing"
                        .to_owned(),
            });
        }
        IoAdapterError::PdfEncrypted => {
            failure.error.remediation = Some(NextAction {
                action: "decrypt the PDF or request an unencrypted advert".to_owned(),
                description: "CanISend never guesses, stores, or transmits PDF passwords"
                    .to_owned(),
            });
        }
        _ => {}
    }
    failure
}

fn task_application_failure(
    operation: &'static str,
    error: ApplicationError,
) -> Box<CommandFailure> {
    let mut failure = app_adapter::failure(operation, error);
    failure.error.remediation = match failure.error.code {
        ErrorCode::ConsentRequired
            if failure
                .human
                .contains("send-to-configured-provider consent") =>
        {
            Some(NextAction {
                action: "obtain user approval, then repeat with --allow-provider-send".to_owned(),
                description: "Only the exact artifact revisions declared by the task may be sent"
                    .to_owned(),
            })
        }
        ErrorCode::TaskStale => Some(NextAction {
            action: "run canisend task prepare again for the current job revision".to_owned(),
            description:
                "A lease expired or a declared input changed; do not reuse the old candidate"
                    .to_owned(),
        }),
        ErrorCode::WorkspaceNotFound => Some(NextAction {
            action: "run canisend --workspace PATH workspace init".to_owned(),
            description:
                "Choose a new workspace directory, or pass --workspace for an existing canisend.toml"
                    .to_owned(),
        }),
        _ => failure.error.remediation,
    };
    failure
}

fn agent_context_failure(error: ApplicationError) -> Box<CommandFailure> {
    let mut failure = app_adapter::failure("agent.context", error);
    if failure.error.code == ErrorCode::WorkspaceNotFound {
        failure.error.remediation = Some(NextAction {
            action: "run canisend --workspace PATH workspace init".to_owned(),
            description:
                "Choose a new workspace directory, or pass --workspace for an existing canisend.toml"
                    .to_owned(),
        });
    }
    failure
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

fn success_with_compatibility<T: serde::Serialize>(
    operation: &'static str,
    status: &'static str,
    data: &T,
    human: Vec<String>,
    compatibility: Option<CompatibilityNotice>,
    surface: CompatibilitySurface,
) -> CommandResult<CommandOutput> {
    let mut output = success(operation, status, data, human)?;
    output.response.compatibility =
        compatibility.map(|compatibility| compatibility.for_surface(surface));
    Ok(output)
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
    use std::fs;

    use canisend_app::{Application, TaskExecutionMode, TaskOperation};
    use canisend_contracts::{ErrorCode, NextAction, OperationRegistry, OperationSurface};
    use clap::Parser;

    use super::{
        AgentAssetsExportArgs, AgentHostName, ApplicationCommand, ApplicationV3ApproveArgs,
        ApplicationV3CandidateArgs, ApplicationV3CreateArgs, ApplicationV3ExportArgs,
        ApplicationV3IdArgs, BuiltInPackName, Cli, Command, CommandFailure, ExitClass, JobCommand,
        JobListArgs, OutputArgs, TaskExecutionModeName, TaskOperationName, WorkspaceCommand,
        WorkspaceInitArgs, agent_assets_export, assistance, capabilities, clap_leaf_paths, context,
        execute, human_failure_lines, human_success_lines,
    };

    fn command_ok(result: super::CommandResult<super::CommandOutput>) -> super::CommandOutput {
        match result {
            Ok(output) => output,
            Err(failure) => panic!("command failed: {}", failure.human),
        }
    }

    #[test]
    fn clap_usage_errors_are_reserved_for_exit_two() {
        let error = Cli::try_parse_from(["canisend", "unknown"]).expect_err("unknown command");
        assert_eq!(error.exit_code(), i32::from(ExitClass::CliUsage.code()));
    }

    #[test]
    fn compiled_clap_leaves_match_the_typed_operation_registry_exactly() {
        let actual = clap_leaf_paths()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let expected = OperationRegistry::built_in()
            .expect("operation registry")
            .surface_leaves(OperationSurface::Cli)
            .expect("CLI leaves");
        assert_eq!(actual, expected);
        assert_eq!(actual.len(), 86);
    }

    #[test]
    fn canonical_v3_commands_parse_with_explicit_pack_and_revision_boundaries() {
        let initialized = Cli::try_parse_from([
            "canisend",
            "--workspace",
            "/tmp/canisend-generic",
            "workspace",
            "init",
        ])
        .expect("default generic Workspace init");
        assert!(matches!(
            initialized.command,
            Command::Workspace {
                command: WorkspaceCommand::Init(super::WorkspaceInitArgs {
                    pack: BuiltInPackName::GenericApplication,
                    ..
                })
            }
        ));

        let compose = Cli::try_parse_from([
            "canisend",
            "application",
            "generic-compose",
            "--application",
            "019f3e88-6630-7000-8000-000000000001",
            "--candidate",
            "/tmp/compose.json",
            "--json",
        ])
        .expect("generic compose command");
        assert!(matches!(
            compose.command,
            Command::Application {
                command: ApplicationCommand::GenericCompose(_)
            }
        ));

        let migrate = Cli::try_parse_from([
            "canisend",
            "workspace",
            "migrate",
            "--expected-plan-sha256",
            &"a".repeat(64),
            "--backup-destination",
            "/tmp/canisend-v2-backup",
        ])
        .expect("revision-bound migration command");
        assert!(matches!(
            migrate.command,
            Command::Workspace {
                command: WorkspaceCommand::Migrate(_)
            }
        ));
    }

    #[test]
    fn canonical_v3_cli_preserves_full_semantic_lifecycle_and_failures() {
        let root = std::env::temp_dir().join(format!(
            "canisend-cli-generic-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let candidate = root.with_extension("json");
        let stale_plan_candidate = root.with_extension("stale-plan.json");
        let plan_candidate = root.with_extension("plan.json");
        let compose_candidate = root.with_extension("compose.json");
        command_ok(execute(Cli {
            workspace: Some(root.clone()),
            command: Command::Workspace {
                command: WorkspaceCommand::Init(WorkspaceInitArgs {
                    pack: BuiltInPackName::GenericApplication,
                    output: OutputArgs { json: true },
                }),
            },
        }));
        fs::write(
            &candidate,
            r#"{
              "title":"Synthetic application",
              "opportunity_metadata":{},
              "application_metadata":{},
              "source_text":"Narrative required.",
              "requirements":[{
                "category":"format",
                "statement":"Narrative required.",
                "priority":"mandatory",
                "start_byte":0,
                "end_byte":19
              }]
            }"#,
        )
        .expect("write bounded candidate");

        let created = command_ok(execute(Cli {
            workspace: Some(root.clone()),
            command: Command::Application {
                command: ApplicationCommand::GenericCreate(ApplicationV3CreateArgs {
                    candidate: candidate.clone(),
                    output: OutputArgs { json: true },
                }),
            },
        }));
        assert_eq!(created.response.operation, "application-flow-v3.create");
        assert_eq!(created.response.status, "created");
        let application_id = created
            .response
            .data
            .as_ref()
            .and_then(|data| data["stored"]["snapshot"]["application"]["id"].as_str())
            .expect("created Application ID")
            .to_owned();

        let listed = command_ok(execute(Cli {
            workspace: Some(root.clone()),
            command: Command::Application {
                command: ApplicationCommand::V3List(OutputArgs { json: true }),
            },
        }));
        assert_eq!(
            listed
                .response
                .data
                .as_ref()
                .and_then(serde_json::Value::as_array)
                .map(Vec::len),
            Some(1)
        );

        let shown = command_ok(execute(Cli {
            workspace: Some(root.clone()),
            command: Command::Application {
                command: ApplicationCommand::V3Show(ApplicationV3IdArgs {
                    application: application_id.clone(),
                    output: OutputArgs { json: true },
                }),
            },
        }));
        assert_eq!(shown.response.operation, "application-v3.show");

        let planned_deliverables = serde_json::json!([{
            "kind": "primary-document",
            "disposition": "required",
            "rationale": "Required by the reviewed source",
            "constraints": ["Use confirmed local evidence only"],
            "execution_mode": "manual-import"
        }]);
        fs::write(
            &stale_plan_candidate,
            serde_json::to_vec(&serde_json::json!({
                "expected_revision": 2,
                "decision": "proceed",
                "deliverables": planned_deliverables
            }))
            .expect("stale Plan JSON"),
        )
        .expect("write stale Plan candidate");
        assert!(
            execute(Cli {
                workspace: Some(root.clone()),
                command: Command::Application {
                    command: ApplicationCommand::GenericPlan(ApplicationV3CandidateArgs {
                        application: application_id.clone(),
                        candidate: stale_plan_candidate.clone(),
                        output: OutputArgs { json: true },
                    }),
                },
            })
            .is_err()
        );
        assert_eq!(
            Application::application_model_v3(&root, &application_id)
                .expect("unchanged after stale Plan")
                .data
                .snapshot
                .application
                .revision
                .get(),
            1
        );

        fs::write(
            &plan_candidate,
            serde_json::to_vec(&serde_json::json!({
                "expected_revision": 1,
                "decision": "proceed",
                "deliverables": planned_deliverables
            }))
            .expect("Plan JSON"),
        )
        .expect("write Plan candidate");
        let planned = command_ok(execute(Cli {
            workspace: Some(root.clone()),
            command: Command::Application {
                command: ApplicationCommand::GenericPlan(ApplicationV3CandidateArgs {
                    application: application_id.clone(),
                    candidate: plan_candidate.clone(),
                    output: OutputArgs { json: true },
                }),
            },
        }));
        assert_eq!(planned.response.status, "confirmed");

        fs::write(
            &compose_candidate,
            serde_json::to_vec(&serde_json::json!({
                "expected_revision": 2,
                "deliverables": [{
                    "kind": "primary-document",
                    "title": "Semantic parity narrative",
                    "media_type": "text/markdown",
                    "content": "Synthetic CLI semantic parity content."
                }]
            }))
            .expect("compose JSON"),
        )
        .expect("write compose candidate");
        let composed = command_ok(execute(Cli {
            workspace: Some(root.clone()),
            command: Command::Application {
                command: ApplicationCommand::GenericCompose(ApplicationV3CandidateArgs {
                    application: application_id.clone(),
                    candidate: compose_candidate.clone(),
                    output: OutputArgs { json: true },
                }),
            },
        }));
        assert_eq!(composed.response.status, "review-required");

        assert!(
            execute(Cli {
                workspace: Some(root.clone()),
                command: Command::Application {
                    command: ApplicationCommand::GenericApprove(ApplicationV3ApproveArgs {
                        application: application_id.clone(),
                        expected_revision: 2,
                        output: OutputArgs { json: true },
                    }),
                },
            })
            .is_err()
        );
        assert_eq!(
            Application::application_model_v3(&root, &application_id)
                .expect("unchanged after stale approval")
                .data
                .snapshot
                .application
                .revision
                .get(),
            3
        );

        let approved = command_ok(execute(Cli {
            workspace: Some(root.clone()),
            command: Command::Application {
                command: ApplicationCommand::GenericApprove(ApplicationV3ApproveArgs {
                    application: application_id.clone(),
                    expected_revision: 3,
                    output: OutputArgs { json: true },
                }),
            },
        }));
        assert_eq!(approved.response.status, "approved");

        let destination = format!("applications/{application_id}/exports/cli-semantic-parity");
        assert!(
            execute(Cli {
                workspace: Some(root.clone()),
                command: Command::Application {
                    command: ApplicationCommand::GenericExport(ApplicationV3ExportArgs {
                        application: application_id.clone(),
                        expected_revision: 3,
                        destination: destination.clone(),
                        allow_private_export: true,
                        output: OutputArgs { json: true },
                    }),
                },
            })
            .is_err()
        );
        assert!(!root.join(&destination).exists());

        let exported = command_ok(execute(Cli {
            workspace: Some(root.clone()),
            command: Command::Application {
                command: ApplicationCommand::GenericExport(ApplicationV3ExportArgs {
                    application: application_id,
                    expected_revision: 4,
                    destination,
                    allow_private_export: true,
                    output: OutputArgs { json: true },
                }),
            },
        }));
        assert_eq!(exported.response.status, "exported");
        assert_eq!(
            exported
                .response
                .data
                .as_ref()
                .and_then(|data| data["render"]["submission_performed"].as_bool()),
            Some(false)
        );

        let generic_before = Application::workspace_status(&root)
            .expect("generic status before compatibility request")
            .data
            .status;
        assert!(
            execute(Cli {
                workspace: Some(root.clone()),
                command: Command::Job {
                    command: JobCommand::List(JobListArgs {
                        include_archived: false,
                        output: OutputArgs { json: true },
                    }),
                },
            })
            .is_err(),
            "academic compatibility CLI must fail closed on the generic Pack"
        );
        assert_eq!(
            Application::workspace_status(&root)
                .expect("generic status after compatibility request")
                .data
                .status,
            generic_before
        );

        let academic_root = root.with_extension("academic-workspace");
        Application::initialize_workspace(&academic_root).expect("academic Workspace");
        let academic_before = Application::workspace_status(&academic_root)
            .expect("academic status before")
            .data
            .status;
        assert!(
            execute(Cli {
                workspace: Some(academic_root.clone()),
                command: Command::Application {
                    command: ApplicationCommand::GenericCreate(ApplicationV3CreateArgs {
                        candidate: candidate.clone(),
                        output: OutputArgs { json: true },
                    }),
                },
            })
            .is_err()
        );
        assert_eq!(
            Application::workspace_status(&academic_root)
                .expect("academic status after")
                .data
                .status,
            academic_before
        );

        fs::remove_dir_all(root).expect("remove v3 Workspace fixture");
        fs::remove_dir_all(academic_root).expect("remove academic Workspace fixture");
        fs::remove_file(candidate).expect("remove candidate fixture");
        fs::remove_file(stale_plan_candidate).expect("remove stale Plan fixture");
        fs::remove_file(plan_candidate).expect("remove Plan fixture");
        fs::remove_file(compose_candidate).expect("remove compose fixture");
    }

    #[test]
    fn human_failures_include_stable_code_remediation_and_retry_hint() {
        let mut failure = CommandFailure::new(
            "task.complete",
            "stale",
            ErrorCode::TaskStale,
            "task input changed",
            true,
        );
        failure.error.remediation = Some(NextAction {
            action: "prepare the task again".to_owned(),
            description: "do not reuse the old candidate".to_owned(),
        });
        assert_eq!(
            human_failure_lines(&failure),
            [
                "canisend [task.stale]: task input changed",
                "Next: prepare the task again — do not reuse the old candidate",
                "Retryable: yes",
            ]
        );
    }

    #[test]
    fn task_cli_values_cover_the_application_registry() {
        let operations = [
            TaskOperationName::JobParse,
            TaskOperationName::EvidenceNormalize,
            TaskOperationName::EvidenceMatch,
            TaskOperationName::CoverLetterDraft,
            TaskOperationName::ResearchStatementDraft,
            TaskOperationName::TeachingStatementDraft,
            TaskOperationName::CvDraft,
            TaskOperationName::DocumentReview,
        ]
        .map(TaskOperation::from);
        assert_eq!(operations, TaskOperation::ALL);
        assert_eq!(
            TaskExecutionMode::from(TaskExecutionModeName::HostAgent),
            TaskExecutionMode::HostAgent
        );
        assert_eq!(
            TaskExecutionMode::from(TaskExecutionModeName::ConfiguredProvider),
            TaskExecutionMode::ConfiguredProvider
        );
    }

    #[test]
    fn agent_facade_adapter_preserves_human_output() {
        let root =
            std::env::temp_dir().join(format!("canisend-cli-agent-human-{}", std::process::id()));
        let packs = root.with_extension("packs");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&packs);
        Application::initialize_workspace(&root).expect("workspace");

        let capabilities =
            capabilities().unwrap_or_else(|_| panic!("capabilities output must succeed"));
        let capability_lines = human_success_lines(&capabilities);
        assert_eq!(
            capability_lines.first().map(String::as_str),
            Some(concat!(
                "CanISend ",
                env!("CARGO_PKG_VERSION"),
                " capabilities"
            ))
        );
        assert!(
            capability_lines
                .iter()
                .any(|line| line == "agent.context: Available")
        );

        let context = context(Some(root.clone()), None)
            .unwrap_or_else(|_| panic!("context output must succeed"));
        let context_lines = human_success_lines(&context);
        assert_eq!(context_lines[0], "CanISend body-free agent context");
        assert!(context_lines[1].starts_with("Workspace: "));
        assert_eq!(context_lines[2], "Blockers: 1");
        assert_eq!(context_lines[3], "Privacy: public metadata only");
        assert_eq!(
            context_lines[4],
            "Next: canisend job create --title TITLE --institution INSTITUTION --json — Create a direct-intake job or import discovery leads"
        );

        let job = Application::create_job(&root, "Lecturer", "University X")
            .expect("job")
            .data;
        let assistance = assistance(Some(root.clone()), job.id.as_str())
            .unwrap_or_else(|_| panic!("assistance output must succeed"));
        let assistance_lines = human_success_lines(&assistance);
        assert_eq!(
            assistance_lines[0],
            "CanISend body-free contextual assistance"
        );
        assert_eq!(assistance_lines[1], "Application: Lecturer — University X");
        assert_eq!(
            assistance_lines[2],
            "Recommended skill: canisend-job-intake"
        );
        assert_eq!(assistance_lines[3], "Proposal targets: 5");

        fs::create_dir(&packs).expect("pack parent");
        let destination = packs.join("generic");
        let assets = agent_assets_export(AgentAssetsExportArgs {
            host: AgentHostName::Generic,
            destination: destination.clone(),
            output: OutputArgs { json: false },
        })
        .unwrap_or_else(|_| panic!("asset output must succeed"));
        assert_eq!(
            human_success_lines(&assets),
            [
                "Exported generic agent pack".to_owned(),
                format!("Directory: {}", destination.display()),
                format!(
                    "Manifest: {}",
                    destination.join("canisend-agent-pack.json").display()
                ),
                "Resources: 35".to_owned(),
            ]
        );

        fs::remove_dir_all(root).expect("remove workspace");
        fs::remove_dir_all(packs).expect("remove packs");
    }
}
