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
