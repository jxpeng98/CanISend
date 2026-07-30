import type {
  JobDetailReadModel,
  JobRecord,
  TaskOperation,
  WorkflowStage,
} from "$lib/bridge";

export type NavigationId =
  | "today"
  | "opportunities"
  | "applications"
  | "profile"
  | "workflow"
  | "agent"
  | "delivery"
  | "workspaces"
  | "settings";

export type WorkflowDetail =
  | "lead-list"
  | "source-intake"
  | "profile-sources"
  | "profile-evidence"
  | "workflow-stages"
  | "decision-criteria"
  | "decision-evidence"
  | "decision-matches"
  | "decision-plan"
  | "agent-handoff"
  | "agent-task"
  | "delivery-documents"
  | "delivery-review"
  | "delivery-package"
  | "delivery-render";

export interface WorkflowRoute {
  view: NavigationId;
  detail?: WorkflowDetail;
  jobId?: string;
}

export interface LastSuccessfulAction {
  operation: string;
  summary: string;
  route: WorkflowRoute;
  workspacePath: string | null;
  jobId: string | null;
  occurredAt: string;
}

export interface NavigationMemory {
  version: 1;
  activeView: NavigationId;
  activeDetail: WorkflowDetail | null;
  workspacePath: string | null;
  selectedJobs: Record<string, string>;
  lastAction: LastSuccessfulAction | null;
}

export type RecommendationReason =
  | "choose-workspace"
  | "discover-opportunity"
  | "choose-application"
  | "attach-source"
  | "build-profile"
  | "start-workflow"
  | "continue-workflow"
  | "review-delivery"
  | "complete";

export interface WorkflowRecommendation {
  route: WorkflowRoute;
  reason: RecommendationReason;
}

const navigationIds = new Set<NavigationId>([
  "today",
  "opportunities",
  "applications",
  "profile",
  "workflow",
  "agent",
  "delivery",
  "workspaces",
  "settings",
]);

const workflowDetails = new Set<WorkflowDetail>([
  "lead-list",
  "source-intake",
  "profile-sources",
  "profile-evidence",
  "workflow-stages",
  "decision-criteria",
  "decision-evidence",
  "decision-matches",
  "decision-plan",
  "agent-handoff",
  "agent-task",
  "delivery-documents",
  "delivery-review",
  "delivery-package",
  "delivery-render",
]);

const MAX_WORKSPACE_MEMORY = 32;
const MAX_MEMORY_STRING = 512;

export function defaultNavigationMemory(): NavigationMemory {
  return {
    version: 1,
    activeView: "today",
    activeDetail: null,
    workspacePath: null,
    selectedJobs: {},
    lastAction: null,
  };
}

function boundedString(value: unknown, max = MAX_MEMORY_STRING): string | null {
  if (typeof value !== "string") return null;
  const normalized = value.trim();
  if (!normalized || normalized.length > max) return null;
  return normalized;
}

function parseRoute(value: unknown): WorkflowRoute | null {
  if (!value || typeof value !== "object") return null;
  const candidate = value as Record<string, unknown>;
  if (
    typeof candidate.view !== "string" ||
    !navigationIds.has(candidate.view as NavigationId)
  ) {
    return null;
  }
  const detail =
    typeof candidate.detail === "string" &&
    workflowDetails.has(candidate.detail as WorkflowDetail)
      ? (candidate.detail as WorkflowDetail)
      : undefined;
  const jobId = boundedString(candidate.jobId, 128) ?? undefined;
  return { view: candidate.view as NavigationId, detail, jobId };
}

function parseLastAction(value: unknown): LastSuccessfulAction | null {
  if (!value || typeof value !== "object") return null;
  const candidate = value as Record<string, unknown>;
  const operation = boundedString(candidate.operation, 128);
  const summary = boundedString(candidate.summary, 240);
  const route = parseRoute(candidate.route);
  const occurredAt = boundedString(candidate.occurredAt, 64);
  const workspacePath =
    candidate.workspacePath === null
      ? null
      : boundedString(candidate.workspacePath, 4_096);
  const jobId =
    candidate.jobId === null ? null : boundedString(candidate.jobId, 128);
  if (
    !operation ||
    !summary ||
    !route ||
    !occurredAt ||
    workspacePath === undefined ||
    jobId === undefined
  ) {
    return null;
  }
  if (Number.isNaN(Date.parse(occurredAt))) return null;
  return { operation, summary, route, workspacePath, jobId, occurredAt };
}

export function parseNavigationMemory(serialized: string | null): NavigationMemory {
  if (!serialized || serialized.length > 32_768) return defaultNavigationMemory();
  try {
    const value: unknown = JSON.parse(serialized);
    if (!value || typeof value !== "object") return defaultNavigationMemory();
    const candidate = value as Record<string, unknown>;
    if (candidate.version !== 1) return defaultNavigationMemory();

    const activeView =
      typeof candidate.activeView === "string" &&
      navigationIds.has(candidate.activeView as NavigationId)
        ? (candidate.activeView as NavigationId)
        : "today";
    const activeDetail =
      typeof candidate.activeDetail === "string" &&
      workflowDetails.has(candidate.activeDetail as WorkflowDetail)
        ? (candidate.activeDetail as WorkflowDetail)
        : null;
    const workspacePath = boundedString(candidate.workspacePath, 4_096);
    const selectedJobs: Record<string, string> = {};
    if (candidate.selectedJobs && typeof candidate.selectedJobs === "object") {
      for (const [workspace, job] of Object.entries(
        candidate.selectedJobs as Record<string, unknown>,
      ).slice(0, MAX_WORKSPACE_MEMORY)) {
        const safeWorkspace = boundedString(workspace, 4_096);
        const safeJob = boundedString(job, 128);
        if (safeWorkspace && safeJob) selectedJobs[safeWorkspace] = safeJob;
      }
    }

    return {
      version: 1,
      activeView,
      activeDetail,
      workspacePath,
      selectedJobs,
      lastAction: parseLastAction(candidate.lastAction),
    };
  } catch {
    return defaultNavigationMemory();
  }
}

export function rememberedJob(
  memory: NavigationMemory,
  workspacePath: string,
  jobs: JobRecord[],
): string | null {
  const remembered = memory.selectedJobs[workspacePath];
  if (remembered && jobs.some((job) => job.id === remembered)) return remembered;
  return jobs[0]?.id ?? null;
}

export function routeForWorkflowStage(stage: WorkflowStage): WorkflowRoute {
  if (stage === "intake") {
    return { view: "applications", detail: "source-intake" };
  }
  if (stage === "parse") {
    return { view: "workflow", detail: "workflow-stages" };
  }
  if (stage === "criteria") {
    return { view: "workflow", detail: "decision-criteria" };
  }
  if (stage === "evidence") {
    return { view: "profile", detail: "profile-evidence" };
  }
  if (stage === "match") {
    return { view: "workflow", detail: "decision-matches" };
  }
  if (stage === "plan") {
    return { view: "workflow", detail: "decision-plan" };
  }
  if (stage === "draft") {
    return { view: "delivery", detail: "delivery-documents" };
  }
  if (stage === "review") {
    return { view: "delivery", detail: "delivery-review" };
  }
  if (stage === "package") {
    return { view: "delivery", detail: "delivery-package" };
  }
  return { view: "delivery", detail: "delivery-render" };
}

export function routeForTaskOperation(operation: TaskOperation | string): WorkflowRoute {
  if (operation === "evidence-normalize") {
    return { view: "profile", detail: "profile-evidence" };
  }
  if (operation === "evidence-match") {
    return { view: "workflow", detail: "decision-matches" };
  }
  if (operation === "document-review") {
    return { view: "delivery", detail: "delivery-review" };
  }
  if (
    operation === "cover-letter-draft" ||
    operation === "research-statement-draft" ||
    operation === "teaching-statement-draft" ||
    operation === "cv-draft"
  ) {
    return { view: "delivery", detail: "delivery-documents" };
  }
  return { view: "workflow", detail: "workflow-stages" };
}

export function routeForAgentAction(action: string): WorkflowRoute {
  const normalized = action.toLowerCase();
  if (normalized.includes("criteria")) {
    return { view: "workflow", detail: "decision-criteria" };
  }
  if (normalized.includes("match")) {
    return { view: "workflow", detail: "decision-matches" };
  }
  if (normalized.includes("plan")) {
    return { view: "workflow", detail: "decision-plan" };
  }
  if (normalized.includes("evidence") || normalized.includes("profile")) {
    return { view: "profile", detail: "profile-evidence" };
  }
  if (normalized.includes("review")) {
    return { view: "delivery", detail: "delivery-review" };
  }
  if (normalized.includes("render")) {
    return { view: "delivery", detail: "delivery-render" };
  }
  if (normalized.includes("package") || normalized.includes("export")) {
    return { view: "delivery", detail: "delivery-package" };
  }
  if (normalized.includes("document") || normalized.includes("draft")) {
    return { view: "delivery", detail: "delivery-documents" };
  }
  if (
    normalized.includes("source") ||
    normalized.includes("intake") ||
    normalized.includes("job.")
  ) {
    return { view: "applications", detail: "source-intake" };
  }
  return { view: "workflow", detail: "workflow-stages" };
}

export function recommendWorkflowRoute(input: {
  workspacePath: string | null;
  jobs: JobRecord[];
  selectedJob: JobDetailReadModel | null;
  profileSourceCount: number;
}): WorkflowRecommendation {
  if (!input.workspacePath) {
    return { route: { view: "workspaces" }, reason: "choose-workspace" };
  }
  if (input.jobs.length === 0) {
    return {
      route: { view: "opportunities", detail: "lead-list" },
      reason: "discover-opportunity",
    };
  }
  if (!input.selectedJob) {
    return { route: { view: "applications" }, reason: "choose-application" };
  }

  const jobId = input.selectedJob.job.id;
  if (input.selectedJob.job.source_ids.length === 0) {
    return {
      route: { view: "applications", detail: "source-intake", jobId },
      reason: "attach-source",
    };
  }
  if (input.profileSourceCount === 0) {
    return {
      route: { view: "profile", detail: "profile-sources", jobId },
      reason: "build-profile",
    };
  }
  if (!input.selectedJob.workflow) {
    return {
      route: { view: "workflow", detail: "workflow-stages", jobId },
      reason: "start-workflow",
    };
  }

  const nextStage = input.selectedJob.workflow.stages.find(
    (stage) => stage.status !== "complete",
  );
  if (nextStage) {
    return {
      route: { ...routeForWorkflowStage(nextStage.stage), jobId },
      reason: "continue-workflow",
    };
  }
  if (input.selectedJob.workflow.status === "complete") {
    return {
      route: { view: "delivery", detail: "delivery-package", jobId },
      reason: "complete",
    };
  }
  return {
    route: { view: "delivery", detail: "delivery-review", jobId },
    reason: "review-delivery",
  };
}
