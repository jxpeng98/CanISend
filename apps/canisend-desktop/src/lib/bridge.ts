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

export interface WorkflowPackBinding {
  id: string;
  version: string;
  content_digest: string;
}

export const ACADEMIC_JOB_WORKFLOW_PACK_ID = "org.canisend.academic-job" as const;
export const GENERIC_APPLICATION_WORKFLOW_PACK_ID = "org.canisend.generic-application" as const;
export type BuiltInWorkflowPackId =
  typeof ACADEMIC_JOB_WORKFLOW_PACK_ID | typeof GENERIC_APPLICATION_WORKFLOW_PACK_ID;

export interface WorkflowPackPresentationLabel {
  value: string;
  locale: string;
  used_default_fallback: boolean;
}

export interface WorkflowPackPresentationFieldOption {
  id: string;
  label: WorkflowPackPresentationLabel;
}

export interface WorkflowPackPresentationField {
  id: string;
  label: WorkflowPackPresentationLabel;
  field_type:
    "short-text" | "long-text" | "integer" | "boolean" | "date" | "url" | "string-list" | "choice";
  required: boolean;
  options: WorkflowPackPresentationFieldOption[];
}

export interface WorkflowPackPresentationCategory {
  id: string;
  label: WorkflowPackPresentationLabel;
  fields: WorkflowPackPresentationField[];
}

export interface WorkflowPackPresentationStage {
  id: string;
  qualified_id: string;
  label: WorkflowPackPresentationLabel;
  depends_on: string[];
  output: string;
  execution_modes: ExecutionMode[];
}

export interface WorkflowPackPresentationDeliverable {
  id: string;
  qualified_id: string;
  label: WorkflowPackPresentationLabel;
  minimum: number;
  maximum: number;
  legacy_task_operation: string | null;
}

export interface WorkflowPackPresentationReadModel {
  pack: WorkflowPackBinding;
  requested_locale: string;
  selected_locale: string;
  locale_match: "exact" | "compatible" | "pack-default";
  vocabulary: {
    application_singular: string;
    application_plural: string;
    opportunity_singular: string;
    opportunity_plural: string;
    requirement_plural: string;
    evidence_plural: string;
    deliverable_plural: string;
  };
  opportunity_fields: WorkflowPackPresentationField[];
  application_fields: WorkflowPackPresentationField[];
  requirement_categories: WorkflowPackPresentationCategory[];
  evidence_categories: WorkflowPackPresentationCategory[];
  stages: WorkflowPackPresentationStage[];
  deliverables: WorkflowPackPresentationDeliverable[];
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
  application_count: number;
  artifact_count: number;
  referenced_blob_count: number;
}

export interface WorkspaceReadModel {
  path: string;
  status: WorkspaceStatus;
}

export type AgentHost = "codex" | "claude" | "generic";

export interface WorkspaceBootstrapHostReadModel {
  host: AgentHost;
  skills: AgentSkillsInstallReadModel;
  mcp: AgentMcpConfigurationReadModel;
  configuration_path: string;
}

export interface WorkspaceBootstrapBoundaryReadModel {
  workspace_alias: string;
  application_count: number;
  profile_initialized: false;
  private_bodies_written: false;
  workspace_modes_enabled: false;
}

export interface WorkspaceBootstrapReadModel {
  action: ActionReceipt<WorkspaceReadModel>;
  registry: RegistrySnapshot;
  validated_packs: WorkflowPackBinding[];
  hosts: WorkspaceBootstrapHostReadModel[];
  boundary: WorkspaceBootstrapBoundaryReadModel;
}

export type ApplicationFieldValueV3 = {
  type:
    "short-text" | "long-text" | "integer" | "boolean" | "date" | "url" | "string-list" | "choice";
  value: string | number | boolean | string[];
};

export interface ApplicationFlowRequirementDraftV3 {
  category: string;
  statement: string;
  priority: "mandatory" | "recommended" | "informational";
  start_byte: number;
  end_byte: number;
}

export interface ApplicationFlowCreateRequestV3 {
  title: string;
  opportunity_metadata: Record<string, ApplicationFieldValueV3>;
  application_metadata: Record<string, ApplicationFieldValueV3>;
  source_text: string;
  requirements: ApplicationFlowRequirementDraftV3[];
}

export interface ApplicationIntakeBaseRequestV4 {
  pack_id: BuiltInWorkflowPackId;
  title: string;
  opportunity_metadata: Record<string, ApplicationFieldValueV3>;
  application_metadata: Record<string, ApplicationFieldValueV3>;
  requirement_category: string;
  requirement_priority: "mandatory" | "recommended" | "informational";
}

export interface PastedApplicationIntakeRequestV4 extends ApplicationIntakeBaseRequestV4 {
  source_text: string;
}

export interface LocalApplicationIntakeRequestV4 extends ApplicationIntakeBaseRequestV4 {
  path: string;
}

export interface UrlApplicationIntakeRequestV4 extends ApplicationIntakeBaseRequestV4 {
  url: string;
}

export type ApplicationIntakeSourceKindV4 =
  "pasted-text" | "local-file" | "text-pdf" | "url-html" | "url-plain-text" | "url-pdf";

export interface ApplicationIntakePreviewReadModelV4 {
  pack_id: BuiltInWorkflowPackId;
  title: string;
  source_kind: ApplicationIntakeSourceKindV4;
  requested_locator: string | null;
  final_locator: string | null;
  redirect_chain: string[];
  content_type: string;
  preview_sha256: string;
  original_sha256: string | null;
  normalized_sha256: string;
  original_bytes: number | null;
  normalized_text_bytes: number;
  normalized_lines: number;
  pdf_page_count: number | null;
  requirement_count: number;
  duplicate_count: number;
  required_consent: "read-private-inputs" | "fetch-user-supplied-url" | null;
  submission_performed: false;
}

export interface ApplicationIntakePreviewTokenReadModelV4 {
  preview_token: string;
  expires_at_unix_ms: number;
  remaining_ttl_seconds: number;
  preview: ActionReceipt<ApplicationIntakePreviewReadModelV4>;
}

export interface ApplicationFlowPlannedDeliverableV3 {
  kind: string;
  disposition: "required" | "optional" | "omitted";
  rationale: string;
  constraints: string[];
  execution_mode: ExecutionMode | null;
}

export interface ApplicationFlowPlanRequestV3 {
  expected_revision: number;
  decision: string;
  deliverables: ApplicationFlowPlannedDeliverableV3[];
}

export interface ApplicationFlowDeliverableDraftV3 {
  kind: string;
  title: string;
  media_type: "text/plain" | "text/markdown";
  content: string;
}

export interface ApplicationFlowComposeRequestV3 {
  expected_revision: number;
  deliverables: ApplicationFlowDeliverableDraftV3[];
}

export interface ApplicationFlowStageV3 {
  id: string;
  state: "pending" | "ready" | "complete";
}

export interface StoredApplicationModelV3 {
  snapshot: {
    format: string;
    pack: WorkflowPackBinding;
    opportunity: {
      id: string;
      title: string;
      metadata: Record<string, ApplicationFieldValueV3>;
      revision: number;
      archived: boolean;
    };
    application: {
      id: string;
      opportunity_id: string;
      metadata: Record<string, ApplicationFieldValueV3>;
      lifecycle: "draft" | "active" | "paused" | "completed" | "archived";
      revision: number;
    };
    requirements: Array<{
      id: string;
      category: string;
      statement: string;
      priority: "mandatory" | "recommended" | "informational";
      confirmation: "proposed" | "confirmed" | "excluded";
      revision: number;
    }>;
    plan: null | {
      id: string;
      state: "draft" | "confirmed" | "stale";
      decision: string | null;
      deliverables: ApplicationFlowPlannedDeliverableV3[];
      revision: number;
    };
    deliverables: Array<{
      id: string;
      kind: string;
      title: string;
      state: "planned" | "draft" | "review-required" | "approved" | "stale";
      media_type: string | null;
      revision: number;
    }>;
  };
  snapshot_sha256: string;
  committed_at: string;
}

export interface ApplicationFlowReadModelV3 {
  stored: StoredApplicationModelV3;
  stages: ApplicationFlowStageV3[];
  submission_performed: false;
}

export interface ApplicationFlowCommitReadModelV3 {
  commit: {
    stored: StoredApplicationModelV3;
    stale_plan_ids: string[];
    stale_deliverable_ids: string[];
  };
  stages: ApplicationFlowStageV3[];
  submission_performed: false;
}

export interface ApplicationFlowReviewReadModelV3 {
  stored: StoredApplicationModelV3;
  deliverables: Array<{
    deliverable: StoredApplicationModelV3["snapshot"]["deliverables"][number];
    content: string;
  }>;
  stages: ApplicationFlowStageV3[];
  submission_performed: false;
}

export interface ApplicationFlowExportReadModelV3 {
  render: {
    application_id: string;
    application_revision: number;
    destination: string;
    documents: Array<{
      deliverable_id: string;
      relative_path: string;
      page_count: number;
      byte_count: number;
      warning_count: number;
    }>;
    submission_performed: false;
  };
  stages: ApplicationFlowStageV3[];
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

export interface WorkspaceV3MigrationPreview {
  format: string;
  source_workspace_format: string;
  target_workspace_format: string;
  pack: WorkflowPackBinding;
  application_count: number;
  legacy_inventory_count: number;
  referenced_blob_count: number;
  required_backup_bytes: number;
  projection_conflict_count: number;
  rollback_boundary: string;
  migration_plan_sha256: string;
}

export interface WorkspaceV3MigrationReadModel {
  backup_destination: string;
  migration: {
    format: string;
    migration_plan_sha256: string;
    application_ids: string[];
    legacy_binding_count: number;
    backup_manifest_sha256: string;
  };
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
  workflow: WorkflowStatusData | null;
}

export type ApplicationDossierState =
  | "needs-source"
  | "ready-to-start"
  | "in-progress"
  | "awaiting-user"
  | "blocked"
  | "complete"
  | "archived";

export interface ApplicationDossierReadModel {
  workspace: string;
  job: JobRecord;
  metadata: {
    origin: "direct" | "discovery";
    discovery_lead_id: string | null;
    discovery_source_id: string | null;
    location: string | null;
    deadline: string | null;
    source_url: string | null;
    freshness: "current" | "stale" | "unknown" | null;
    last_seen_at: string | null;
  };
  source_count: number;
  profile_source_count: number;
  state: ApplicationDossierState;
  current_stage: WorkflowStage | null;
  completed_stages: number;
  total_stages: number;
  workflow: WorkflowStatusData | null;
  blockers: Array<{
    code: string;
    description: string;
    stage: WorkflowStage | null;
  }>;
  next_actions: Array<{ action: string; description: string }>;
}

export interface ApplicationDossierListReadModel {
  workspace: string;
  include_archived: boolean;
  applications: ApplicationDossierReadModel[];
}

export interface SourceImportReadModel {
  job: JobRecord;
  source: SourceRecord;
}

export interface IntakeReviewReadModel {
  source: {
    kind: "url" | "pdf" | "local-file" | "csv" | "json" | "agent" | "network";
    locator: string;
    detected_type: string;
    sha256: string | null;
  };
  extraction: {
    original_bytes: number | null;
    normalized_text_bytes: number | null;
    normalized_lines: number | null;
    pdf_pages: number | null;
    accepted_items: number;
    rejected_items: number;
    semantic_fields_pending: boolean;
  };
  duplicate_signal: {
    state: "none-known" | "exact-match" | "review-after-commit";
    count: number;
    automatic_merge: boolean;
  };
  target: {
    kind: "application" | "opportunity-library";
    id: string | null;
    label: string;
  };
  intended_mutations: Array<{
    subject: string;
    action: string;
    description: string;
  }>;
  required_consent:
    | "read-private-inputs"
    | "send-to-configured-provider"
    | "fetch-user-supplied-url"
    | "export-private-artifacts"
    | "use-system-fonts";
  consent_confirmed: boolean;
  commit_boundary: "exact-prepared-bytes" | "exact-normalized-report";
}

export interface JobIntakePreviewReadModel {
  preview_token: string;
  expires_at_unix_ms: number;
  remaining_ttl_seconds: number;
  intake: IntakeReviewReadModel;
  preview: ActionReceipt<{
    workspace: string;
    job: JobRecord;
    expected_job_revision: number;
    provenance: {
      source_kind: "local-file" | "url";
      requested_locator: string;
      final_url: string | null;
      redirect_chain: string[];
      original_sha256: string;
    };
    extraction: {
      content_type: string;
      original_bytes: number;
      normalized_text_bytes: number;
      normalized_lines: number;
      pdf_pages: number | null;
      semantic_fields_pending: boolean;
    };
    validation_issues: Array<{
      code: string;
      severity: "information" | "warning";
      message: string;
    }>;
    intended_mutations: Array<{
      subject: string;
      action: string;
      description: string;
    }>;
  }>;
}

export type DiscoverySourceKind =
  "csv" | "json" | "host-agent" | "rss-atom" | "jobs-ac-uk" | "greenhouse" | "lever";

export type DiscoveryNetworkAdapter = "rss-atom" | "jobs-ac-uk" | "greenhouse" | "lever";

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
  expires_at_unix_ms: number;
  remaining_ttl_seconds: number;
  kind: "import" | "refresh";
  intake: IntakeReviewReadModel;
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

export type ProfileInitializationReadModel = ProfileSourceImportReadModel;

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
  "deterministic" | "host-agent" | "configured-provider" | "user-decision" | "manual-import";

export interface ArtifactReference {
  id: string;
  kind: string;
  revision: number;
  sha256: string;
}

export type DocumentKind = string;

export type ContentCategory =
  | "source"
  | "profile"
  | "job-analysis"
  | "evidence"
  | "planning"
  | "materials"
  | "review"
  | "delivery";

export type ContentCatalogStatus = "imported" | "proposed" | "confirmed" | "generated" | "stale";

export type ContentPrivacyClassification = "public" | "private-local" | "provider-bound" | "secret";

export interface ContentCatalogFilter {
  job_id?: string | null;
  category?: ContentCategory | null;
  stage?: WorkflowStage | null;
  status?: ContentCatalogStatus | null;
  privacy?: ContentPrivacyClassification | null;
  created_after?: string | null;
  created_before?: string | null;
}

export interface ContentCatalogEntryReadModel {
  artifact: ArtifactReference;
  title: string;
  category: ContentCategory;
  stage: WorkflowStage;
  status: ContentCatalogStatus;
  privacy: ContentPrivacyClassification;
  size: number;
  created_at: string;
  provenance: {
    actor: "user" | "host-agent" | "configured-provider" | "system";
    reason: string;
    source_id: string | null;
    source_scope: "job" | "profile" | null;
    source_role: "original" | "normalized" | null;
    source_kind: string | null;
    content_type: string | null;
    locator: string | null;
  };
  subject_jobs: Array<{
    id: string;
    title: string;
    institution: string;
    archived: boolean;
  }>;
  relationships: ArtifactReference[];
  private_body_searchable: boolean;
}

export interface ContentCatalogReadModel {
  workspace: string;
  total_entries: number;
  entries: ContentCatalogEntryReadModel[];
  filter: Required<ContentCatalogFilter>;
}

export interface ContentSearchReadModel {
  workspace: string;
  query: string;
  include_private_bodies: boolean;
  total_matches: number;
  results: Array<{
    entry: ContentCatalogEntryReadModel;
    score: number;
    matched_fields: Array<"metadata" | "private-body">;
    snippet: string | null;
  }>;
  index: {
    strategy: string;
    metadata_entries: number;
    private_body_entries: number;
    private_body_bytes: number;
    skipped_oversized_entries: number;
    skipped_secret_entries: number;
    truncated: boolean;
  };
  filter: Required<ContentCatalogFilter>;
}

export interface WorkflowStatusData {
  run_id: string;
  job_id: string;
  status: "active" | "complete";
  stages: Array<{
    stage: WorkflowStage;
    status: "blocked" | "ready" | "running" | "awaiting-user" | "complete" | "stale";
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
  expires_at_unix_ms: number;
  remaining_ttl_seconds: number;
  preview: ActionReceipt<{
    job_id: string;
    target: WorkflowStage;
    affected_stages: WorkflowStage[];
    affected_outputs: ArtifactReference[];
  }>;
}

export type TaskOperation = string;

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
  expires_at_unix_ms: number;
  remaining_ttl_seconds: number;
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

export type AgentWorkspaceSection =
  "overview" | "job-criteria" | "evidence-fit" | "materials" | "review-export";

export type AgentProposalKind = "criteria" | "evidence" | "matches" | "plan" | "draft";

export type AgentProposalState = "blocked" | "ready" | "proposed" | "current" | "stale";

export interface AgentAssistanceReadModel {
  workspace: string;
  selected_job_id: string;
  dossier: ApplicationDossierReadModel;
  context: AgentContextReadModel;
  content: {
    total_entries: number;
    truncated: boolean;
    entries: Array<{
      artifact: ArtifactReference;
      title: string;
      category: ContentCategory;
      stage: WorkflowStage;
      status: ContentCatalogStatus;
      privacy: ContentPrivacyClassification;
      provenance: {
        actor: "user" | "host-agent" | "configured-provider" | "system";
        reason: string;
        source_id: string | null;
        source_scope: "job" | "profile" | null;
        source_role: "original" | "normalized" | null;
        source_kind: string | null;
        content_type: string | null;
      };
      relationships: ArtifactReference[];
    }>;
  };
  recommendation: {
    skill_id: string;
    section: AgentWorkspaceSection;
    reason: string;
    next_action: { action: string; description: string } | null;
  };
  proposal_targets: Array<{
    kind: AgentProposalKind;
    stage: WorkflowStage;
    section: AgentWorkspaceSection;
    state: AgentProposalState;
    operation: string;
    current_artifacts: ArtifactReference[];
    upstream_artifacts: ArtifactReference[];
    validation_rules: string[];
    intended_mutation: string;
    commit_boundary: "user-confirmation" | "task-preview-commit";
  }>;
  execution_boundary: {
    recommended_integration: "external-host";
    session_authority: "external-agent-host";
    state_authority: "canisend";
    in_app_runtime: "optional-read-only";
    transcript_persisted_by_canisend: false;
  };
  privacy: "public";
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

export interface AgentHandoffReadModel {
  host: "codex" | "claude" | "generic";
  workspace: string;
  selected_job_id: string | null;
  launch_command: string;
  start_command: string;
  capabilities_command: string;
  context_command: string;
  assistance_command: string | null;
  bootstrap_prompt: string;
  recommended_skill: string;
  recommended_integration: "external-host";
  session_authority: string;
  state_authority: "canisend";
  context: AgentContextReadModel;
  assistance: AgentAssistanceReadModel | null;
}

export interface AgentSkillsInstallReadModel {
  workspace: string;
  directory: string;
  manifest_path: string;
  state: "installed" | "updated" | "up-to-date";
  files: Array<{
    resource_id: string;
    resource_version: string;
    path: string;
    size: number;
    sha256: string;
  }>;
}

export type AgentSkillsStatusState =
  | "not-installed"
  | "up-to-date"
  | "update-available"
  | "incomplete"
  | "user-modified"
  | "unmanaged";

export interface AgentSkillsStatusReadModel {
  workspace: string;
  directory: string;
  manifest_path: string;
  host: "codex" | "claude" | "generic";
  bundled_product_version: string;
  installed_product_version: string | null;
  state: AgentSkillsStatusState;
  skills: Array<{
    id: string;
    resource_version: string;
    state: AgentSkillsStatusState;
    file_count: number;
    installed_file_count: number;
  }>;
}

export interface AgentSkillsUninstallReadModel {
  workspace: string;
  directory: string;
  manifest_path: string;
  host: "codex" | "claude" | "generic";
  state: "not-installed" | "removed";
  removed_files: number;
}

export interface AgentMcpConfigurationReadModel {
  host: "codex" | "claude" | "generic";
  workspace: string;
  executable: string;
  server_name: "canisend";
  transport: "stdio";
  protocol_version: string;
  configuration_target: string;
  registration_command: string | null;
  configuration_snippet: string;
  verification_command: string;
  tools: string[];
  read_only_tools: string[];
  guarded_write_tools: string[];
  state_authority: string;
  session_authority: string;
}

export type AgentRuntimeKind = "codex" | "claude";

export interface AgentSessionEntry {
  workspace: string;
  runtime: AgentRuntimeKind;
  job_id: string | null;
  external_session_id: string;
  created_at_unix: number;
  updated_at_unix: number;
}

export interface AgentRuntimeProbe {
  runtime: AgentRuntimeKind;
  available: boolean;
  executable: string | null;
  version: string | null;
  resume_strategy: "external-session-id";
  authentication_state: "host-managed-unverified";
  host_configuration_state: "host-managed-unverified";
  probe_evidence: "executable-and-version-only";
  interaction_mode: "read-only";
}

export interface AgentRuntimeCatalog {
  runtimes: AgentRuntimeProbe[];
  sessions: AgentSessionEntry[];
  session_storage: string;
}

export interface AgentTurnResult {
  runtime: AgentRuntimeKind;
  session: AgentSessionEntry;
  response: string;
  resumed: boolean;
  event_count: number;
  tool_activity: string[];
}

export interface AgentTurnCancelResult {
  runtime: AgentRuntimeKind;
  workspace: string;
  selected_job_id: string | null;
  cancellation_requested: boolean;
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

export interface RenderedDocumentRecord {
  kind: DocumentKind;
  document_artifact: ArtifactReference;
  typst_artifact: ArtifactReference;
  pdf_artifact: ArtifactReference;
  page_count: number;
  byte_count: number;
  warning_count: number;
  elapsed_millis: number;
}

export interface RenderManifestRecord {
  id: string;
  job_id: string;
  documents: RenderedDocumentRecord[];
  rendered_at: string;
  submission_performed: false;
  revision: number;
}

export interface RenderExportReadModel {
  render_manifest: RenderManifestRecord;
  destination: string;
  files: string[];
  submission_performed: false;
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
  path_active: boolean;
  path_configuration_file: string | null;
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

export async function getWorkflowPackPresentation(
  locale: "en" | "zh-CN",
  packId: BuiltInWorkflowPackId = ACADEMIC_JOB_WORKFLOW_PACK_ID,
): Promise<ActionReceipt<WorkflowPackPresentationReadModel>> {
  return invoke("workflow_pack_presentation", {
    request: { locale, pack_id: packId },
  });
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
  hosts: AgentHost[],
): Promise<WorkspaceBootstrapReadModel> {
  return invoke("create_workspace", {
    request: { alias, path, hosts },
  });
}

export async function connectWorkspace(
  alias: string,
  path: string,
): Promise<RegisteredAction<WorkspaceReadModel>> {
  return invoke("connect_workspace", { request: { alias, path } });
}

export async function selectWorkspace(path: string): Promise<RegisteredAction<WorkspaceReadModel>> {
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

export async function previewWorkspaceV3Migration(
  workspace: string,
): Promise<ActionReceipt<WorkspaceV3MigrationPreview>> {
  return invoke("preview_workspace_v3_migration", {
    request: { path: workspace },
  });
}

export async function migrateWorkspaceV3(options: {
  workspace: string;
  expectedPlanSha256: string;
  backupDestination: string;
}): Promise<ActionReceipt<WorkspaceV3MigrationReadModel>> {
  return invoke("migrate_workspace_v3", {
    request: {
      workspace: options.workspace,
      expected_plan_sha256: options.expectedPlanSha256,
      backup_destination: options.backupDestination,
    },
  });
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

export async function listApplicationDossiers(
  workspace: string,
  includeArchived = false,
): Promise<ActionReceipt<ApplicationDossierListReadModel>> {
  return invoke("list_application_dossiers", {
    request: { workspace, include_archived: includeArchived },
  });
}

export async function getApplicationDossier(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<ApplicationDossierReadModel>> {
  return invoke("application_dossier", {
    request: { workspace, job_id: jobId },
  });
}

export async function listGenericApplications(
  workspace: string,
): Promise<ActionReceipt<StoredApplicationModelV3[]>> {
  return invoke("list_generic_applications", { request: { workspace } });
}

export async function showGenericApplication(
  workspace: string,
  applicationId: string,
): Promise<ActionReceipt<ApplicationFlowReadModelV3>> {
  return invoke("show_generic_application", {
    request: { workspace, application_id: applicationId },
  });
}

export async function createGenericApplication(
  workspace: string,
  packId: BuiltInWorkflowPackId,
  request: ApplicationFlowCreateRequestV3,
): Promise<ActionReceipt<ApplicationFlowReadModelV3>> {
  return invoke("create_generic_application", {
    request: { workspace, pack_id: packId, request },
  });
}

export async function previewPastedApplicationIntake(
  workspace: string,
  preview: PastedApplicationIntakeRequestV4,
): Promise<ApplicationIntakePreviewTokenReadModelV4> {
  return invoke("preview_pasted_application_intake", {
    request: { workspace, preview },
  });
}

export async function previewLocalApplicationIntake(
  workspace: string,
  preview: LocalApplicationIntakeRequestV4,
  confirmedPrivateRead: boolean,
): Promise<ApplicationIntakePreviewTokenReadModelV4> {
  return invoke("preview_local_application_intake", {
    request: {
      workspace,
      preview,
      confirmed_private_read: confirmedPrivateRead,
    },
  });
}

export async function previewUrlApplicationIntake(
  workspace: string,
  preview: UrlApplicationIntakeRequestV4,
  confirmedNetworkFetch: boolean,
): Promise<ApplicationIntakePreviewTokenReadModelV4> {
  return invoke("preview_url_application_intake", {
    request: {
      workspace,
      preview,
      confirmed_network_fetch: confirmedNetworkFetch,
    },
  });
}

export async function commitApplicationIntakePreview(
  workspace: string,
  packId: BuiltInWorkflowPackId,
  previewToken: string,
): Promise<ActionReceipt<ApplicationFlowReadModelV3>> {
  return invoke("commit_application_intake_preview", {
    request: { workspace, pack_id: packId, preview_token: previewToken },
  });
}

export async function discardApplicationIntakePreview(
  workspace: string,
  packId: BuiltInWorkflowPackId,
  previewToken: string,
): Promise<void> {
  return invoke("discard_application_intake_preview", {
    request: { workspace, pack_id: packId, preview_token: previewToken },
  });
}

export async function planGenericApplication(
  workspace: string,
  applicationId: string,
  request: ApplicationFlowPlanRequestV3,
): Promise<ActionReceipt<ApplicationFlowCommitReadModelV3>> {
  return invoke("plan_generic_application", {
    request: { workspace, application_id: applicationId, request },
  });
}

export async function composeGenericApplication(
  workspace: string,
  applicationId: string,
  request: ApplicationFlowComposeRequestV3,
): Promise<ActionReceipt<ApplicationFlowCommitReadModelV3>> {
  return invoke("compose_generic_application", {
    request: { workspace, application_id: applicationId, request },
  });
}

export async function reviewGenericApplication(
  workspace: string,
  applicationId: string,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<ApplicationFlowReviewReadModelV3>> {
  return invoke("review_generic_application", {
    request: {
      workspace,
      application_id: applicationId,
      confirmed_private_read: confirmedPrivateRead,
    },
  });
}

export async function approveGenericApplication(
  workspace: string,
  applicationId: string,
  expectedRevision: number,
): Promise<ActionReceipt<ApplicationFlowCommitReadModelV3>> {
  return invoke("approve_generic_application", {
    request: {
      workspace,
      application_id: applicationId,
      expected_revision: expectedRevision,
    },
  });
}

export async function exportGenericApplication(options: {
  workspace: string;
  applicationId: string;
  expectedRevision: number;
  destination: string;
  confirmedPrivateExport: boolean;
}): Promise<ActionReceipt<ApplicationFlowExportReadModelV3>> {
  return invoke("export_generic_application", {
    request: {
      workspace: options.workspace,
      application_id: options.applicationId,
      expected_revision: options.expectedRevision,
      destination: options.destination,
      confirmed_private_export: options.confirmedPrivateExport,
    },
  });
}

function contentFilterRequest(filter: ContentCatalogFilter = {}): Required<ContentCatalogFilter> {
  return {
    job_id: filter.job_id ?? null,
    category: filter.category ?? null,
    stage: filter.stage ?? null,
    status: filter.status ?? null,
    privacy: filter.privacy ?? null,
    created_after: filter.created_after ?? null,
    created_before: filter.created_before ?? null,
  };
}

export async function getContentCatalog(
  workspace: string,
  filter: ContentCatalogFilter = {},
): Promise<ActionReceipt<ContentCatalogReadModel>> {
  return invoke("content_catalog", {
    request: {
      workspace,
      filter: contentFilterRequest(filter),
    },
  });
}

export async function searchContent(options: {
  workspace: string;
  query: string;
  filter?: ContentCatalogFilter;
  includePrivateBodies: boolean;
  confirmedPrivateRead: boolean;
  limit?: number;
}): Promise<ActionReceipt<ContentSearchReadModel>> {
  return invoke("search_content", {
    request: {
      workspace: options.workspace,
      query: options.query,
      filter: contentFilterRequest(options.filter),
      include_private_bodies: options.includePrivateBodies,
      confirmed_private_read: options.confirmedPrivateRead,
      limit: options.limit ?? 50,
    },
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

export async function previewLocalJobSource(
  workspace: string,
  jobId: string,
  source: string,
  confirmedPrivateRead: boolean,
): Promise<JobIntakePreviewReadModel> {
  return invoke("preview_local_job_source", {
    request: {
      workspace,
      job_id: jobId,
      source,
      confirmed_private_read: confirmedPrivateRead,
    },
  });
}

export async function previewUrlJobSource(
  workspace: string,
  jobId: string,
  url: string,
  confirmedNetworkFetch: boolean,
): Promise<JobIntakePreviewReadModel> {
  return invoke("preview_url_job_source", {
    request: {
      workspace,
      job_id: jobId,
      url,
      confirmed_network_fetch: confirmedNetworkFetch,
    },
  });
}

export async function commitJobSourcePreview(
  workspace: string,
  previewToken: string,
): Promise<ActionReceipt<SourceImportReadModel>> {
  return invoke("commit_job_source_preview", {
    request: { workspace, preview_token: previewToken },
  });
}

export async function discardJobSourcePreview(
  workspace: string,
  previewToken: string,
): Promise<void> {
  return invoke("discard_job_source_preview", {
    request: { workspace, preview_token: previewToken },
  });
}

export async function getDiscoveryAdapters(): Promise<
  ActionReceipt<DiscoveryAdapterCatalogReadModel>
> {
  return invoke("discovery_adapters");
}

export async function previewDiscoveryFile(options: {
  workspace: string;
  path: string;
  sourceName?: string;
  sourceUrl?: string;
  hostAgent?: boolean;
  confirmedPrivateRead: boolean;
}): Promise<DiscoveryPreviewReadModel> {
  return invoke("preview_discovery_file", {
    request: {
      workspace: options.workspace,
      path: options.path,
      source_name: options.sourceName || null,
      source_url: options.sourceUrl || null,
      host_agent: options.hostAgent ?? false,
      confirmed_private_read: options.confirmedPrivateRead,
    },
  });
}

export async function previewDiscoveryNetwork(options: {
  workspace: string;
  adapter: DiscoveryNetworkAdapter;
  endpoint: string;
  sourceName: string;
  organization?: string;
  confirmedNetworkFetch: boolean;
}): Promise<DiscoveryPreviewReadModel> {
  return invoke("preview_discovery_network", {
    request: {
      workspace: options.workspace,
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
  kind: "import" | "refresh",
): Promise<ActionReceipt<DiscoveryImportReport>> {
  return invoke("commit_discovery_preview", {
    request: { workspace, preview_token: previewToken, kind },
  });
}

export async function discardDiscoveryPreview(
  workspace: string,
  previewToken: string,
  kind: "import" | "refresh",
): Promise<void> {
  return invoke("discard_discovery_preview", {
    request: { workspace, preview_token: previewToken, kind },
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

export async function initializeProfile(options: {
  workspace: string;
  markdown: string;
  sensitivity: PrivacyClassification;
  confirmedPrivateRead: boolean;
}): Promise<ActionReceipt<ProfileInitializationReadModel>> {
  return invoke("initialize_profile", {
    request: {
      workspace: options.workspace,
      markdown: options.markdown,
      sensitivity: options.sensitivity,
      confirmed_private_read: options.confirmedPrivateRead,
    },
  });
}

function privateJobRequest(workspace: string, jobId: string, confirmedPrivateRead: boolean) {
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
  return invoke("criteria_template", privateJobRequest(workspace, jobId, confirmedPrivateRead));
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
  return invoke("current_matches", privateJobRequest(workspace, jobId, confirmedPrivateRead));
}

export async function getPlanTemplate(
  workspace: string,
  jobId: string,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<ApplicationPlanRecord>> {
  return invoke("plan_template", privateJobRequest(workspace, jobId, confirmedPrivateRead));
}

export async function getCurrentPlan(
  workspace: string,
  jobId: string,
  confirmedPrivateRead: boolean,
): Promise<ActionReceipt<ApplicationPlanRecord>> {
  return invoke("current_plan", privateJobRequest(workspace, jobId, confirmedPrivateRead));
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
  workspace: string,
  previewToken: string,
): Promise<ActionReceipt<WorkflowControlReadModel>> {
  return invoke("commit_workflow_rerun", {
    request: { workspace, preview_token: previewToken },
  });
}

export async function discardWorkflowPreview(
  workspace: string,
  previewToken: string,
): Promise<void> {
  return invoke("discard_workflow_preview", {
    request: { workspace, preview_token: previewToken },
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
  workspace: string,
  previewToken: string,
): Promise<ActionReceipt<Record<string, unknown>>> {
  return invoke("commit_task_completion_preview", {
    request: { workspace, preview_token: previewToken },
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

export async function getAgentCapabilities(): Promise<ActionReceipt<AgentCapabilitiesReadModel>> {
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

export async function getAgentAssistance(
  workspace: string,
  jobId: string,
): Promise<ActionReceipt<AgentAssistanceReadModel>> {
  return invoke("agent_assistance", {
    request: {
      workspace,
      job_id: jobId,
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

export async function prepareAgentHandoff(
  host: "codex" | "claude" | "generic",
  workspace: string,
  selectedJobId?: string,
): Promise<ActionReceipt<AgentHandoffReadModel>> {
  return invoke("prepare_agent_handoff", {
    request: {
      host,
      workspace,
      selected_job_id: selectedJobId || null,
    },
  });
}

export async function copyAgentHandoff(
  host: "codex" | "claude" | "generic",
  workspace: string,
  selectedJobId: string | undefined,
  field: "launch-command" | "start-command" | "bootstrap-prompt",
): Promise<void> {
  return invoke("copy_agent_handoff", {
    request: {
      host,
      workspace,
      selected_job_id: selectedJobId || null,
      field,
    },
  });
}

export async function installAgentSkills(
  host: "codex" | "claude" | "generic",
  workspace: string,
): Promise<ActionReceipt<AgentSkillsInstallReadModel>> {
  return invoke("install_agent_skills", {
    request: { host, workspace },
  });
}

export async function getAgentSkillsStatus(
  host: "codex" | "claude" | "generic",
  workspace: string,
): Promise<ActionReceipt<AgentSkillsStatusReadModel>> {
  return invoke("agent_skills_status", {
    request: { host, workspace },
  });
}

export async function uninstallAgentSkills(
  host: "codex" | "claude" | "generic",
  workspace: string,
): Promise<ActionReceipt<AgentSkillsUninstallReadModel>> {
  return invoke("uninstall_agent_skills", {
    request: { host, workspace },
  });
}

export async function prepareAgentMcpConfiguration(
  host: "codex" | "claude" | "generic",
  workspace: string,
): Promise<ActionReceipt<AgentMcpConfigurationReadModel>> {
  return invoke("prepare_agent_mcp_configuration", {
    request: { host, workspace },
  });
}

export async function copyAgentMcpConfiguration(
  host: "codex" | "claude" | "generic",
  workspace: string,
  field: "registration-command" | "configuration-snippet",
): Promise<void> {
  return invoke("copy_agent_mcp_configuration", {
    request: { host, workspace, field },
  });
}

export async function getAgentRuntimeCatalog(
  workspace?: string,
  selectedJobId?: string,
): Promise<AgentRuntimeCatalog> {
  return invoke("agent_runtime_catalog", {
    request: {
      workspace: workspace || null,
      selected_job_id: selectedJobId || null,
    },
  });
}

export async function runAgentTurn(options: {
  workspace: string;
  selectedJobId?: string;
  runtime: AgentRuntimeKind;
  prompt: string;
  startNew: boolean;
  confirmedProviderSend: boolean;
}): Promise<AgentTurnResult> {
  return invoke("run_agent_turn", {
    request: {
      workspace: options.workspace,
      selected_job_id: options.selectedJobId || null,
      runtime: options.runtime,
      prompt: options.prompt,
      start_new: options.startNew,
      confirmed_provider_send: options.confirmedProviderSend,
    },
  });
}

export async function cancelAgentTurn(options: {
  workspace: string;
  selectedJobId?: string;
  runtime: AgentRuntimeKind;
}): Promise<AgentTurnCancelResult> {
  return invoke("cancel_agent_turn", {
    request: {
      workspace: options.workspace,
      selected_job_id: options.selectedJobId || null,
      runtime: options.runtime,
    },
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

export async function previewRender(
  workspace: string,
  jobId: string,
  kind: DocumentKind,
  confirmedPrivateRead: boolean,
): Promise<Uint8Array> {
  const response = await invoke<ArrayBuffer>("preview_render", {
    request: {
      workspace,
      job_id: jobId,
      kind,
      confirmed_private_read: confirmedPrivateRead,
    },
  });
  return new Uint8Array(response);
}

export async function exportRender(
  workspace: string,
  jobId: string,
  destination: string,
  confirmedPrivateExport: boolean,
): Promise<ActionReceipt<RenderExportReadModel>> {
  return invoke("export_render", {
    request: {
      workspace,
      job_id: jobId,
      destination,
      confirmed_private_export: confirmedPrivateExport,
    },
  });
}

export async function exportRenderAndOpen(
  workspace: string,
  jobId: string,
  destination: string,
  kind: DocumentKind,
  confirmedPrivateExport: boolean,
): Promise<ActionReceipt<RenderExportReadModel>> {
  return invoke("export_render_and_open", {
    request: {
      workspace,
      job_id: jobId,
      destination,
      kind,
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

export async function configureCliPath(options: {
  destination?: string;
  confirmedTerminalInstall: boolean;
}): Promise<ActionReceipt<CliInstallStatus>> {
  return invoke("configure_cli_path", {
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

export async function getInspectionCatalog(): Promise<ActionReceipt<InspectionCatalogReadModel>> {
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

export async function chooseApplicationSource(): Promise<string | null> {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [
      {
        name: "Application source",
        extensions: ["pdf", "txt", "md", "json"],
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

export function commandErrorCode(error: unknown): string | null {
  return typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string"
    ? error.code
    : null;
}

export function commandErrorRetryable(error: unknown): boolean {
  return (
    typeof error === "object" && error !== null && "retryable" in error && error.retryable === true
  );
}
