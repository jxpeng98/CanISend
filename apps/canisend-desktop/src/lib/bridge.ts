import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface ProductSummary {
  product: string;
  version: string;
  protocol: string;
  workspace_format: string;
  resource_format: string;
  public_schema_version: string;
  target_os: string;
  target_arch: string;
}

export interface DoctorSummary {
  healthy: boolean;
  embedded_resources: number;
  embedded_renderer: boolean;
  rendered_pages: number;
  render_warning_count: number;
  rendered_pdf_bytes: number;
  render_elapsed_millis: number;
  schema_count: number;
  binary_size_bytes: number;
  release_binary_budget_bytes: number;
  system_font_scan: boolean;
  runtime_package_downloads: boolean;
  python_required: boolean;
}

export interface ActionReceipt<T> {
  operation: string;
  status: string;
  summary: string;
  data: T;
  warnings?: string[];
  next_actions?: Array<{ action: string; description: string }>;
}

export interface DesktopCommandError {
  code: string;
  message: string;
  retryable: boolean;
}

export interface WorkspaceEntry {
  alias: string;
  path: string;
  pinned: boolean;
  last_opened_unix: number;
}

export interface WorkspaceRegistry {
  format: string;
  default_path: string | null;
  entries: WorkspaceEntry[];
}

export interface RegistrySnapshot {
  registry_path: string;
  registry: WorkspaceRegistry;
}

export interface WorkspaceStatus {
  workspace_id: string;
  workspace_format: string;
  created_at: string;
  database_schema_version: number;
  sqlite_version: string;
  journal_mode: string;
  job_count: number;
  artifact_count: number;
  referenced_blob_count: number;
}

export interface WorkspaceReadModel {
  path: string;
  status: WorkspaceStatus;
}

export interface WorkspaceCheckIssue {
  code: string;
  severity: "warning" | "error";
  subject: string;
  message: string;
}

export interface WorkspaceCheck {
  workspace_id: string;
  ok: boolean;
  database_integrity: string;
  checked_referenced_blobs: number;
  unreferenced_blobs: string[];
  stale_artifact_ids: string[];
  projection_repairs_required: string[];
  issues: WorkspaceCheckIssue[];
}

export interface WorkspaceHealthReadModel {
  path: string;
  check: WorkspaceCheck;
}

export interface WorkspaceRepairReadModel {
  workspace: string;
  repaired_projections: number;
  check: WorkspaceCheck;
}

export interface WorkspaceRestoreReadModel {
  backup: string;
  destination: string;
  workspace: WorkspaceStatus;
}

export interface BackupReadModel {
  destination: string;
  format: string;
  blob_count: number;
}

export interface RegisteredAction<T> {
  action: ActionReceipt<T>;
  registry: RegistrySnapshot;
}

export interface JobRecord {
  id: string;
  title: string;
  institution: string;
  source_ids: string[];
  created_at: string;
  revision: number;
  archived: boolean;
}

export interface JobListReadModel {
  workspace: string;
  include_archived: boolean;
  jobs: JobRecord[];
}

export interface SourceRecord {
  id: string;
  job_id: string;
  kind: "local-file" | "user-url" | "discovery-lead" | "manual-text";
  source_url: string | null;
  final_url: string | null;
  content_type: string;
  redirect_chain: string[];
  retrieved_at: string;
  privacy: string;
}

export interface JobDetailReadModel {
  workspace: string;
  job: JobRecord;
  sources: SourceRecord[];
  workflow: unknown | null;
}

export interface SourceImportReadModel {
  job: JobRecord;
  source: SourceRecord;
}

export type DiscoverySourceKind =
  | "csv"
  | "json"
  | "host-agent"
  | "rss-atom"
  | "jobs-ac-uk"
  | "greenhouse"
  | "lever";

export type DiscoveryNetworkAdapter =
  | "rss-atom"
  | "jobs-ac-uk"
  | "greenhouse"
  | "lever";

export interface DiscoveryAdapterCapabilities {
  kind: DiscoverySourceKind;
  network: boolean;
  supports_cursor: boolean;
  preserves_removed: boolean;
  max_items_per_refresh: number;
}

export interface DiscoveryAdapterCatalogReadModel {
  adapters: DiscoveryAdapterCapabilities[];
}

export interface DiscoveryImportDiagnostic {
  row: number;
  code: string;
  message: string;
}

export interface DiscoveryLeadCandidate {
  external_id: string | null;
  title: string;
  organization: string;
  location: string | null;
  deadline: string | null;
  url: string;
  summary: string | null;
}

export interface DiscoveryImportReport {
  dry_run: boolean;
  accepted: number;
  rejected: number;
  diagnostics: DiscoveryImportDiagnostic[];
  batch: {
    source_kind: DiscoverySourceKind;
    source_name: string;
    source_url: string | null;
    cursor: string | null;
    observed_at: string;
    leads: DiscoveryLeadCandidate[];
  } | null;
  receipt: unknown | null;
}

export interface DiscoveryPreviewReadModel {
  preview_token: string;
  kind: "import" | "refresh";
  preview: ActionReceipt<DiscoveryImportReport>;
}

export interface DiscoverySourceRecord {
  id: string;
  kind: DiscoverySourceKind;
  name: string;
  endpoint: string | null;
  enabled: boolean;
  cursor: string | null;
  last_refreshed_at: string | null;
  created_at: string;
}

export interface DiscoverySourceListReadModel {
  workspace: string;
  sources: DiscoverySourceRecord[];
}

export interface DiscoveryLeadRecord {
  id: string;
  source_id: string;
  external_id: string | null;
  canonical_key: string;
  title: string;
  organization: string;
  location: string | null;
  deadline: string | null;
  url: string;
  summary: string | null;
  status: "active" | "removed" | "expired" | "promoted";
  freshness: "current" | "stale" | "unknown";
  first_seen_at: string;
  last_seen_at: string;
  revision: number;
  promoted_job_id: string | null;
}

export interface DiscoveryLeadListReadModel {
  workspace: string;
  include_history: boolean;
  leads: DiscoveryLeadRecord[];
}

export interface DiscoverySuggestionReadModel {
  suggestions: Array<{
    lead: DiscoveryLeadRecord;
    similarity_percent: number;
  }>;
  automatic_merge: boolean;
}

export interface DiscoveryPromotionReadModel {
  job: JobRecord;
  lead_id: string;
}

export type PrivacyClassification = "public" | "private-local";

export interface ProfileSourceRecord {
  id: string;
  kind: "markdown" | "plain-text" | "json";
  original: Record<string, unknown>;
  normalized_text: Record<string, unknown>;
  content_type: string;
  sensitivity: PrivacyClassification;
  created_at: string;
  revision: number;
}

export interface ProfileSourceListReadModel {
  workspace: string;
  profile_revision: number;
  sources: ProfileSourceRecord[];
}

export interface ProfileSourceImportReadModel {
  profile_revision: number;
  source: ProfileSourceRecord;
}

export interface EvidenceCatalogRecord {
  id: string;
  profile_revision: number;
  items: Array<{
    id: string;
    kind: string;
    summary: string;
    confirmed: boolean;
    excluded: boolean;
    revision: number;
  }>;
  revision: number;
}

export interface CriteriaSetRecord {
  id: string;
  job_id: string;
  criteria: Array<{
    id: string;
    kind: string;
    requirement: string;
    importance: string;
    confidence_milli: number;
    confirmed: boolean;
    revision: number;
  }>;
  revision: number;
}

export interface EvidenceMatchSetRecord {
  id: string;
  job_id: string;
  matches: Array<{
    id: string;
    strength: string;
    rationale: string;
    gap: string | null;
    prohibited_claims: string[];
    revision: number;
  }>;
  revision: number;
}

export interface ApplicationPlanRecord {
  id?: string;
  job_id: string;
  decision: "apply" | "hold" | "skip";
  strategy: {
    positioning: string;
    priorities: string[];
    risks: string[];
  };
  documents: Array<{
    id?: string;
    kind: string;
    requirement: string;
    rationale: string;
    constraints: string[];
    executor: string | null;
    revision?: number;
  }>;
  blockers: Array<{
    code: string;
    severity: string;
    description: string;
  }>;
  revision?: number;
}

export type WorkflowStage =
  | "intake"
  | "parse"
  | "criteria"
  | "evidence"
  | "match"
  | "plan"
  | "draft"
  | "review"
  | "package"
  | "render";

export type ExecutionMode =
  | "deterministic"
  | "host-agent"
  | "configured-provider"
  | "user-decision"
  | "manual-import";

export interface ArtifactReference {
  id: string;
  kind: string;
  revision: number;
  sha256: string;
}

export interface WorkflowStatusData {
  run_id: string;
  job_id: string;
  status: "active" | "complete";
  stages: Array<{
    stage: WorkflowStage;
    status:
      | "blocked"
      | "ready"
      | "running"
      | "awaiting-user"
      | "complete"
      | "stale";
    execution_mode: ExecutionMode | null;
    output: ArtifactReference | null;
    updated_at: string;
  }>;
  blockers: Array<{
    code: string;
    stage: WorkflowStage;
    description: string;
  }>;
  next_actions: Array<{ action: string; description: string }>;
}

export interface WorkflowControlReadModel {
  status: WorkflowStatusData;
  stage_descriptors: Array<{
    stage: WorkflowStage;
    depends_on: WorkflowStage[];
    output_kind: string;
    execution_modes: ExecutionMode[];
  }>;
}

export interface WorkflowRerunPreviewReadModel {
  preview_token: string;
  preview: ActionReceipt<{
    job_id: string;
    target: WorkflowStage;
    affected_stages: WorkflowStage[];
    affected_outputs: ArtifactReference[];
  }>;
}

export type TaskOperation =
  | "job-parse"
  | "evidence-normalize"
  | "evidence-match"
  | "cover-letter-draft"
  | "research-statement-draft"
  | "teaching-statement-draft"
  | "cv-draft"
  | "document-review";

export type TaskExecutionMode = "host-agent" | "configured-provider";

export interface TaskDescriptor {
  id: string;
  operation: string;
  job_id: string;
  job_revision: number;
  profile_revision: number | null;
  actor: string;
  execution_mode: ExecutionMode;
  input_artifacts: ArtifactReference[];
  allowed_output_kind: string;
  candidate_schema: Record<string, unknown>;
  required_consents: Array<Record<string, unknown>>;
  private_read_scope: ArtifactReference[];
  lease: Record<string, unknown>;
}

export interface TaskStateData {
  descriptor: TaskDescriptor;
  status: "prepared" | "committed" | "cancelled" | "stale";
  result: ArtifactReference | null;
}

export interface TaskCompletionPreviewReadModel {
  preview_token: string;
  preview: ActionReceipt<{
    request: Record<string, unknown>;
    state: TaskStateData;
  }>;
}

export interface AgentCapabilitiesReadModel {
  product_version: string;
  protocol: string;
  workspace_format: string;
  resource_format: string;
  capabilities: Array<Record<string, unknown>>;
  stages: Array<Record<string, unknown>>;
  discovery_adapters: Array<Record<string, unknown>>;
  error_codes: string[];
}

export interface AgentContextReadModel {
  product_version: string;
  protocol: string;
  workspace_id: string | null;
  active_job_id: string | null;
  workspace: Record<string, unknown> | null;
  selected_job: Record<string, unknown> | null;
  blockers: Array<{
    code: string;
    description: string;
    subject_id: string | null;
  }>;
  next_actions: Array<{ action: string; description: string }>;
}

export interface AgentPackExportReadModel {
  directory: string;
  manifest_path: string;
  manifest: {
    host: "codex" | "claude" | "generic";
    files: Array<{
      resource_id: string;
      path: string;
      size: number;
      sha256: string;
    }>;
  };
}

export interface DocumentWorkspaceReadModel {
  documents: Array<{
    id: string;
    job_id: string;
    kind: string;
    title: string;
    sections: Array<Record<string, unknown>>;
    placeholders: Array<Record<string, unknown>>;
    revision: number;
  }>;
  accepted_set: Record<string, unknown> | null;
  acceptance_blocker: string | null;
}

export interface ReviewWorkspaceReadModel {
  current: {
    id: string;
    job_id: string;
    findings: Array<{
      id: string;
      code: string;
      category: string;
      severity: "info" | "warning" | "blocker";
      authority: "deterministic" | "human-review";
      message: string;
      status: "open" | "accepted-risk" | "resolved" | "dismissed";
      revision: number;
    }>;
    revision: number;
  };
  disposition_candidate: Record<string, unknown>;
}

export interface PackageManifestRecord {
  id: string;
  job_id: string;
  documents: ArtifactReference[];
  readiness: {
    job_id: string;
    state: "blocked" | "needs-review" | "ready-to-export" | "exported";
    reasons: Array<{
      code: string;
      document_kind: string | null;
      finding_id: string | null;
    }>;
    checked_at: string;
  };
  submission_performed: false;
  revision: number;
}

export interface ProjectionRecord {
  source_artifact: ArtifactReference;
  relative_path: string;
  kind: string;
  generated_sha256: string;
  observed_sha256: string | null;
  edit_status: "current" | "edited" | "missing" | "repair-required";
  updated_at: string;
}

export interface PackageExportManifestRecord {
  id: string;
  job_id: string;
  projections: ProjectionRecord[];
  exported_at: string;
  submission_performed: false;
  revision: number;
}

export interface ProjectionReconcileRecord {
  job_id: string;
  projection: ProjectionRecord;
  action: "inspect" | "replace" | "copy-as-new";
  preserved_copy_path: string | null;
  authoritative_changed: false;
  reconciled_at: string;
}

export interface RenderManifestRecord {
  id: string;
  job_id: string;
  documents: Array<{
    kind: string;
    page_count: number;
    byte_count: number;
    warning_count: number;
    elapsed_millis: number;
  }>;
  rendered_at: string;
  submission_performed: false;
  revision: number;
}

export interface DesktopCliDefaults {
  bundled_source: string | null;
  destination: string;
}

export interface CliInstallStatus {
  state:
    | "not-installed"
    | "current"
    | "update-available"
    | "migration-available"
    | "newer-installed"
    | "modified"
    | "source-unavailable";
  bundled_version: string;
  installed_version: string | null;
  version_relation: "unknown" | "older" | "same" | "newer";
  source_path: string | null;
  destination: string;
  manifest_path: string;
  installed: boolean;
  managed: boolean;
  path_configured: boolean;
  active_command: string | null;
  active_is_managed: boolean;
  previous_installation_preserved: boolean;
}

export interface UpdateCheckReadModel {
  current_version: string;
  latest_version: string;
  latest_tag: string;
  release_name: string;
  release_url: string;
  published_at: string | null;
  prerelease: boolean;
  channel: string;
  update_available: boolean;
}

export interface InspectionCatalogReadModel {
  schemas: {
    schemas: Array<{
      id: string;
      version: string;
      uri: string;
      resource_id: string;
      size: number;
      sha256: string;
    }>;
  };
  resources: Array<{
    entry: {
      id: string;
      kind: string;
      version: string;
      size: number;
      sha256: string;
    };
    path: string;
  }>;
}

export async function getProductSummary(): Promise<ProductSummary> {
  return invoke<ProductSummary>("product_summary");
}

export async function runDoctor(): Promise<ActionReceipt<DoctorSummary>> {
  return invoke<ActionReceipt<DoctorSummary>>("run_doctor");
}

export async function listWorkspaces(): Promise<RegistrySnapshot> {
  return invoke<RegistrySnapshot>("list_workspaces");
}

export async function createWorkspace(
  alias: string,
  path: string,
): Promise<RegisteredAction<WorkspaceReadModel>> {
  return invoke("create_workspace", { request: { alias, path } });
}

export async function connectWorkspace(
  alias: string,
  path: string,
): Promise<RegisteredAction<WorkspaceReadModel>> {
  return invoke("connect_workspace", { request: { alias, path } });
}

export async function selectWorkspace(
  path: string,
): Promise<RegisteredAction<WorkspaceReadModel>> {
  return invoke("select_workspace", { request: { path } });
}

export async function removeWorkspace(path: string): Promise<RegistrySnapshot> {
  return invoke("remove_workspace", { request: { path } });
}

export async function checkWorkspace(
  path: string,
): Promise<ActionReceipt<WorkspaceHealthReadModel>> {
  return invoke("check_workspace", { request: { path } });
}

export async function backupWorkspace(
  workspace: string,
  destination: string,
): Promise<ActionReceipt<BackupReadModel>> {
  return invoke("backup_workspace", {
    request: { workspace, destination },
  });
}

export async function restoreWorkspace(
  alias: string,
  backup: string,
  destination: string,
): Promise<RegisteredAction<WorkspaceRestoreReadModel>> {
  return invoke("restore_workspace", {
    request: { alias, backup, destination },
  });
}

export async function repairWorkspace(
  path: string,
): Promise<ActionReceipt<WorkspaceRepairReadModel>> {
  return invoke("repair_workspace", { request: { path } });
}

export async function listJobs(
  workspace: string,
  includeArchived = false,
): Promise<ActionReceipt<JobListReadModel>> {
  return invoke("list_jobs", {
    request: { workspace, include_archived: includeArchived },
  });
}

export async function createJob(
  workspace: string,
  title: string,
  institution: string,
): Promise<ActionReceipt<JobRecord>> {
  return invoke("create_job", {
    request: { workspace, title, institution },
  });
}

export async function showJob(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<JobDetailReadModel>> {
  return invoke("show_job", {
    request: { workspace, job_id: jobId },
  });
}

export async function archiveJob(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<JobRecord>> {
  return invoke("archive_job", {
    request: { workspace, job_id: jobId },
  });
}

export async function importLocalJobSource(
  workspace: string,
  jobId: string,
  source: string,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<SourceImportReadModel>> {
  return invoke("import_local_job_source", {
    request: {
      workspace,
      job_id: jobId,
      source,
      confirmed_private_read: confirmedPrivateRead,
    },
  });
}

export async function importUrlJobSource(
  workspace: string,
  jobId: string,
  url: string,
  confirmedNetworkFetch: boolean,
): Promise<ActionReceipt<SourceImportReadModel>> {
  return invoke("import_url_job_source", {
    request: {
      workspace,
      job_id: jobId,
      url,
      confirmed_network_fetch: confirmedNetworkFetch,
    },
  });
}

export async function getDiscoveryAdapters(): Promise<
  ActionReceipt<DiscoveryAdapterCatalogReadModel>
> {
  return invoke("discovery_adapters");
}

export async function previewDiscoveryFile(options: {
  path: string;
  sourceName?: string;
  sourceUrl?: string;
  hostAgent?: boolean;
  confirmedPrivateRead: boolean;
}): Promise<DiscoveryPreviewReadModel> {
  return invoke("preview_discovery_file", {
    request: {
      path: options.path,
      source_name: options.sourceName || null,
      source_url: options.sourceUrl || null,
      host_agent: options.hostAgent ?? false,
      confirmed_private_read: options.confirmedPrivateRead,
    },
  });
}

export async function previewDiscoveryNetwork(options: {
  adapter: DiscoveryNetworkAdapter;
  endpoint: string;
  sourceName: string;
  organization?: string;
  confirmedNetworkFetch: boolean;
}): Promise<DiscoveryPreviewReadModel> {
  return invoke("preview_discovery_network", {
    request: {
      adapter: options.adapter,
      endpoint: options.endpoint,
      source_name: options.sourceName,
      organization: options.organization || null,
      confirmed_network_fetch: options.confirmedNetworkFetch,
    },
  });
}

export async function commitDiscoveryPreview(
  workspace: string,
  previewToken: string,
): Promise<ActionReceipt<DiscoveryImportReport>> {
  return invoke("commit_discovery_preview", {
    request: { workspace, preview_token: previewToken },
  });
}

export async function discardDiscoveryPreview(
  previewToken: string,
): Promise<void> {
  return invoke("discard_discovery_preview", {
    request: { preview_token: previewToken },
  });
}

export async function listDiscoverySources(
  workspace: string,
): Promise<ActionReceipt<DiscoverySourceListReadModel>> {
  return invoke("list_discovery_sources", { request: { workspace } });
}

export async function listDiscoveryLeads(
  workspace: string,
  includeHistory = false,
): Promise<ActionReceipt<DiscoveryLeadListReadModel>> {
  return invoke("list_discovery_leads", {
    request: { workspace, include_history: includeHistory },
  });
}

export async function showDiscoveryLead(
  workspace: string,
  leadId: string,
): Promise<ActionReceipt<DiscoveryLeadRecord>> {
  return invoke("show_discovery_lead", {
    request: { workspace, lead_id: leadId },
  });
}

export async function suggestDiscoveryDuplicates(
  workspace: string,
  leadId: string,
  limit = 5,
): Promise<ActionReceipt<DiscoverySuggestionReadModel>> {
  return invoke("suggest_discovery_duplicates", {
    request: { workspace, lead_id: leadId, limit },
  });
}

export async function promoteDiscoveryLead(
  workspace: string,
  leadId: string,
): Promise<ActionReceipt<DiscoveryPromotionReadModel>> {
  return invoke("promote_discovery_lead", {
    request: { workspace, lead_id: leadId },
  });
}

export async function listProfileSources(
  workspace: string,
): Promise<ActionReceipt<ProfileSourceListReadModel>> {
  return invoke("list_profile_sources", { request: { workspace } });
}

export async function importProfileSource(options: {
  workspace: string;
  source: string;
  sensitivity: PrivacyClassification;
  confirmedPrivateRead: boolean;
}): Promise<ActionReceipt<ProfileSourceImportReadModel>> {
  return invoke("import_profile_source", {
    request: {
      workspace: options.workspace,
      source: options.source,
      sensitivity: options.sensitivity,
      confirmed_private_read: options.confirmedPrivateRead,
    },
  });
}

function privateJobRequest(
  workspace: string,
  jobId: string,
  confirmedPrivateRead: boolean,
) {
  return {
    request: {
      workspace,
      job_id: jobId,
      confirmed_private_read: confirmedPrivateRead,
    },
  };
}

function candidateRequest(
  workspace: string,
  jobId: string,
  candidate: unknown,
  confirmedPrivateRead: boolean,
) {
  return {
    request: {
      workspace,
      job_id: jobId,
      candidate,
      confirmed_private_read: confirmedPrivateRead,
    },
  };
}

export async function getProfileEvidenceTemplate(
  workspace: string,
  jobId: string,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<EvidenceCatalogRecord>> {
  return invoke(
    "profile_evidence_template",
    privateJobRequest(workspace, jobId, confirmedPrivateRead),
  );
}

export async function confirmProfileEvidence(
  workspace: string,
  jobId: string,
  candidate: unknown,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<EvidenceCatalogRecord>> {
  return invoke(
    "confirm_profile_evidence",
    candidateRequest(workspace, jobId, candidate, confirmedPrivateRead),
  );
}

export async function getCriteriaTemplate(
  workspace: string,
  jobId: string,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<CriteriaSetRecord>> {
  return invoke(
    "criteria_template",
    privateJobRequest(workspace, jobId, confirmedPrivateRead),
  );
}

export async function confirmCriteria(
  workspace: string,
  jobId: string,
  candidate: unknown,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<CriteriaSetRecord>> {
  return invoke(
    "confirm_criteria",
    candidateRequest(workspace, jobId, candidate, confirmedPrivateRead),
  );
}

export async function getCurrentMatches(
  workspace: string,
  jobId: string,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<EvidenceMatchSetRecord>> {
  return invoke(
    "current_matches",
    privateJobRequest(workspace, jobId, confirmedPrivateRead),
  );
}

export async function getPlanTemplate(
  workspace: string,
  jobId: string,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<ApplicationPlanRecord>> {
  return invoke(
    "plan_template",
    privateJobRequest(workspace, jobId, confirmedPrivateRead),
  );
}

export async function getCurrentPlan(
  workspace: string,
  jobId: string,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<ApplicationPlanRecord>> {
  return invoke(
    "current_plan",
    privateJobRequest(workspace, jobId, confirmedPrivateRead),
  );
}

export async function confirmPlan(
  workspace: string,
  jobId: string,
  candidate: unknown,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<ApplicationPlanRecord>> {
  return invoke(
    "confirm_plan",
    candidateRequest(workspace, jobId, candidate, confirmedPrivateRead),
  );
}

export async function startWorkflow(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<WorkflowStatusData>> {
  return invoke("start_workflow", {
    request: { workspace, job_id: jobId },
  });
}

export async function getWorkflowControls(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<WorkflowControlReadModel>> {
  return invoke("workflow_controls", {
    request: { workspace, job_id: jobId },
  });
}

export async function beginWorkflowStage(
  workspace: string,
  jobId: string,
  stage: WorkflowStage,
  mode: ExecutionMode,
): Promise<ActionReceipt<WorkflowControlReadModel>> {
  return invoke("begin_workflow_stage", {
    request: { workspace, job_id: jobId, stage, mode },
  });
}

export async function completeWorkflowStage(
  workspace: string,
  jobId: string,
  stage: WorkflowStage,
  artifactId: string,
): Promise<ActionReceipt<WorkflowControlReadModel>> {
  return invoke("complete_workflow_stage", {
    request: {
      workspace,
      job_id: jobId,
      stage,
      artifact_id: artifactId,
    },
  });
}

export async function previewWorkflowRerun(
  workspace: string,
  jobId: string,
  stage: WorkflowStage,
): Promise<WorkflowRerunPreviewReadModel> {
  return invoke("preview_workflow_rerun", {
    request: { workspace, job_id: jobId, stage },
  });
}

export async function commitWorkflowRerun(
  previewToken: string,
): Promise<ActionReceipt<WorkflowControlReadModel>> {
  return invoke("commit_workflow_rerun", {
    request: { preview_token: previewToken },
  });
}

export async function discardWorkflowPreview(
  previewToken: string,
): Promise<void> {
  return invoke("discard_workflow_preview", {
    request: { preview_token: previewToken },
  });
}

export async function getLatestTask(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<TaskStateData | null>> {
  return invoke("latest_task", {
    request: { workspace, job_id: jobId },
  });
}

export async function prepareTask(
  workspace: string,
  jobId: string,
  operation: TaskOperation,
  mode: TaskExecutionMode,
): Promise<ActionReceipt<TaskDescriptor>> {
  return invoke("prepare_task", {
    request: { workspace, job_id: jobId, operation, mode },
  });
}

export async function exportTaskInputs(options: {
  workspace: string;
  taskId: string;
  destination: string;
  confirmedPrivateRead: boolean;
  confirmedProviderSend: boolean;
}): Promise<ActionReceipt<Record<string, unknown>>> {
  return invoke("export_task_inputs", {
    request: {
      workspace: options.workspace,
      task_id: options.taskId,
      destination: options.destination,
      confirmed_private_read: options.confirmedPrivateRead,
      confirmed_provider_send: options.confirmedProviderSend,
    },
  });
}

export async function previewTaskCompletion(options: {
  workspace: string;
  file: string;
  confirmedPrivateRead: boolean;
}): Promise<TaskCompletionPreviewReadModel> {
  return invoke("preview_task_completion", {
    request: {
      workspace: options.workspace,
      file: options.file,
      confirmed_private_read: options.confirmedPrivateRead,
    },
  });
}

export async function commitTaskCompletion(
  previewToken: string,
): Promise<ActionReceipt<Record<string, unknown>>> {
  return invoke("commit_task_completion_preview", {
    request: { preview_token: previewToken },
  });
}

export async function cancelTask(
  workspace: string,
  taskId: string,
): Promise<ActionReceipt<TaskStateData>> {
  return invoke("cancel_task", {
    request: { workspace, task_id: taskId },
  });
}

export async function prepareTaskAgain(
  workspace: string,
  taskId: string,
): Promise<ActionReceipt<Record<string, unknown>>> {
  return invoke("prepare_task_again", {
    request: { workspace, task_id: taskId },
  });
}

export async function getAgentCapabilities(): Promise<
  ActionReceipt<AgentCapabilitiesReadModel>
> {
  return invoke("agent_capabilities");
}

export async function getAgentContext(
  workspace?: string,
  selectedJobId?: string,
): Promise<ActionReceipt<AgentContextReadModel>> {
  return invoke("agent_context", {
    request: {
      workspace: workspace || null,
      selected_job_id: selectedJobId || null,
    },
  });
}

export async function exportAgentPack(
  host: "codex" | "claude" | "generic",
  destination: string,
): Promise<ActionReceipt<AgentPackExportReadModel>> {
  return invoke("export_agent_pack", {
    request: { host, destination },
  });
}

export async function getDocumentWorkspace(
  workspace: string,
  jobId: string,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<DocumentWorkspaceReadModel>> {
  return invoke("document_workspace", {
    request: {
      workspace,
      job_id: jobId,
      confirmed_private_read: confirmedPrivateRead,
    },
  });
}

export async function getReviewWorkspace(
  workspace: string,
  jobId: string,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<ReviewWorkspaceReadModel>> {
  return invoke("review_workspace", {
    request: {
      workspace,
      job_id: jobId,
      confirmed_private_read: confirmedPrivateRead,
    },
  });
}

export async function confirmReview(
  workspace: string,
  jobId: string,
  candidate: unknown,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<Record<string, unknown>>> {
  return invoke("confirm_review", {
    request: {
      workspace,
      job_id: jobId,
      candidate,
      confirmed_private_read: confirmedPrivateRead,
    },
  });
}

function deliveryJobRequest(workspace: string, jobId: string) {
  return { request: { workspace, job_id: jobId } };
}

export async function checkPackage(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<PackageManifestRecord>> {
  return invoke("check_package", deliveryJobRequest(workspace, jobId));
}

export async function getCurrentPackage(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<PackageManifestRecord>> {
  return invoke("current_package", deliveryJobRequest(workspace, jobId));
}

export async function exportPackage(
  workspace: string,
  jobId: string,
  destination: string,
  confirmedPrivateExport: boolean,
): Promise<ActionReceipt<PackageExportManifestRecord>> {
  return invoke("export_package", {
    request: {
      workspace,
      job_id: jobId,
      destination,
      confirmed_private_export: confirmedPrivateExport,
    },
  });
}

export async function getCurrentPackageExport(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<PackageExportManifestRecord>> {
  return invoke("current_package_export", deliveryJobRequest(workspace, jobId));
}

export async function reconcilePackage(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<ProjectionReconcileRecord[]>> {
  return invoke("reconcile_package", deliveryJobRequest(workspace, jobId));
}

export async function replacePackageProjection(
  workspace: string,
  jobId: string,
  path: string,
): Promise<ActionReceipt<ProjectionReconcileRecord>> {
  return invoke("replace_package_projection", {
    request: { workspace, job_id: jobId, path },
  });
}

export async function copyPackageProjection(
  workspace: string,
  jobId: string,
  path: string,
  destination: string,
): Promise<ActionReceipt<ProjectionReconcileRecord>> {
  return invoke("copy_package_projection", {
    request: { workspace, job_id: jobId, path, destination },
  });
}

export async function buildRender(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<RenderManifestRecord>> {
  return invoke("build_render", deliveryJobRequest(workspace, jobId));
}

export async function getCurrentRender(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<RenderManifestRecord>> {
  return invoke("current_render", deliveryJobRequest(workspace, jobId));
}

export async function exportRender(
  workspace: string,
  jobId: string,
  destination: string,
  confirmedPrivateExport: boolean,
): Promise<ActionReceipt<Record<string, unknown>>> {
  return invoke("export_render", {
    request: {
      workspace,
      job_id: jobId,
      destination,
      confirmed_private_export: confirmedPrivateExport,
    },
  });
}

export async function getDesktopCliDefaults(): Promise<DesktopCliDefaults> {
  return invoke("desktop_cli_defaults");
}

export async function getCliInstallStatus(
  destination?: string,
): Promise<ActionReceipt<CliInstallStatus>> {
  return invoke("cli_install_status", {
    request: { destination: destination || null },
  });
}

export async function installCli(options: {
  destination?: string;
  replaceExisting: boolean;
  confirmedTerminalInstall: boolean;
}): Promise<ActionReceipt<CliInstallStatus>> {
  return invoke("install_cli", {
    request: {
      destination: options.destination || null,
      replace_existing: options.replaceExisting,
      confirmed_terminal_install: options.confirmedTerminalInstall,
    },
  });
}

export async function uninstallCli(options: {
  destination?: string;
  confirmedTerminalInstall: boolean;
}): Promise<ActionReceipt<CliInstallStatus>> {
  return invoke("uninstall_cli", {
    request: {
      destination: options.destination || null,
      confirmed_terminal_install: options.confirmedTerminalInstall,
    },
  });
}

export async function checkForUpdates(
  confirmedNetworkFetch: boolean,
): Promise<ActionReceipt<UpdateCheckReadModel>> {
  return invoke("check_for_updates", {
    request: { confirmed_network_fetch: confirmedNetworkFetch },
  });
}

export async function getInspectionCatalog(): Promise<
  ActionReceipt<InspectionCatalogReadModel>
> {
  return invoke("inspection_catalog");
}

export async function getSchemaDetail(
  query: string,
): Promise<ActionReceipt<Record<string, unknown>>> {
  return invoke("schema_detail", { request: { query } });
}

export async function getResourceDetail(
  query: string,
): Promise<ActionReceipt<Record<string, unknown>>> {
  return invoke("resource_detail", { request: { query } });
}

export async function exportResourceCatalog(
  destination: string,
): Promise<ActionReceipt<Record<string, unknown>>> {
  return invoke("export_resource_catalog", {
    request: { destination },
  });
}

export async function chooseWorkspaceDirectory(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}

export async function chooseJobSource(): Promise<string | null> {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [
      {
        name: "Job advert",
        extensions: ["pdf", "txt", "md", "markdown", "json"],
      },
    ],
  });
  return typeof selected === "string" ? selected : null;
}

export async function chooseDiscoverySource(): Promise<string | null> {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [
      {
        name: "Discovery batch",
        extensions: ["csv", "json"],
      },
    ],
  });
  return typeof selected === "string" ? selected : null;
}

export async function chooseProfileSource(): Promise<string | null> {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [
      {
        name: "Profile source",
        extensions: ["md", "markdown", "txt", "json"],
      },
    ],
  });
  return typeof selected === "string" ? selected : null;
}

export async function chooseTaskCompletion(): Promise<string | null> {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [{ name: "Task completion", extensions: ["json"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function chooseExportDirectory(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}

export function isDesktopRuntime(): boolean {
  return isTauri();
}

export function commandErrorMessage(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  return "The desktop command failed without a structured error.";
}

export function commandErrorRetryable(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "retryable" in error &&
    error.retryable === true
  );
}
