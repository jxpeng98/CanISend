import { describe, expect, it } from "vitest";

import type { ApplicationDossierReadModel, JobDetailReadModel, JobRecord } from "./bridge";
import {
  applicationSectionForRoute,
  defaultNavigationMemory,
  isApplicationWorkspaceRoute,
  isNavigationAvailableForPack,
  parseNavigationMemory,
  recommendWorkflowRoute,
  rememberedJob,
  routeForAgentAction,
  routeForApplicationSection,
  routeForContentEntry,
  routeForTaskOperation,
  routeForWorkspacePack,
  routeForWorkflowStage,
} from "./workflow-navigation";

const academicPack = "org.canisend.academic-job";
const genericPack = "org.canisend.generic-application";

const job: JobRecord = {
  id: "job-1",
  title: "Lecturer in Economics",
  institution: "University X",
  source_ids: [],
  created_at: "2026-07-30T10:00:00Z",
  revision: 1,
  archived: false,
};

function detail(overrides: Partial<JobDetailReadModel> = {}): JobDetailReadModel {
  return {
    workspace: "/tmp/canisend",
    job,
    sources: [],
    workflow: null,
    ...overrides,
  };
}

describe("navigation memory", () => {
  it("rejects malformed or unbounded persisted state", () => {
    expect(parseNavigationMemory("{broken")).toEqual(defaultNavigationMemory());
    expect(parseNavigationMemory("x".repeat(32_769))).toEqual(defaultNavigationMemory());
    expect(
      parseNavigationMemory(
        JSON.stringify({
          version: 1,
          activeView: "unknown",
          activeDetail: "not-a-detail",
          workspacePath: 42,
          selectedJobs: { "/tmp/canisend": "job-1" },
          lastAction: null,
        }),
      ),
    ).toMatchObject({
      activeView: "today",
      activeDetail: null,
      workspacePath: null,
      selectedJobs: { "/tmp/canisend": "job-1" },
    });
  });

  it("drops an invalid last action without losing valid workspace continuity", () => {
    const restored = parseNavigationMemory(
      JSON.stringify({
        version: 1,
        activeView: "applications",
        activeDetail: null,
        workspacePath: "/tmp/canisend",
        selectedJobs: { "/tmp/canisend": "job-1" },
        lastAction: {
          operation: "package.export",
          summary: "Exported package",
          route: { view: "not-a-view", jobId: "job-1" },
          workspacePath: "/tmp/canisend",
          jobId: "job-1",
          occurredAt: "not-a-date",
        },
      }),
    );

    expect(restored).toMatchObject({
      activeView: "applications",
      workspacePath: "/tmp/canisend",
      selectedJobs: { "/tmp/canisend": "job-1" },
      lastAction: null,
    });
  });

  it("restores only a job that still belongs to the workspace", () => {
    const memory = {
      ...defaultNavigationMemory(),
      selectedJobs: { "/tmp/canisend": "job-1" },
    };
    expect(rememberedJob(memory, "/tmp/canisend", [job])).toBe("job-1");
    expect(
      rememberedJob({ ...memory, selectedJobs: { "/tmp/canisend": "missing" } }, "/tmp/canisend", [
        job,
      ]),
    ).toBe("job-1");
  });

  it("restores the exact application section and successful receipt route", () => {
    const restored = parseNavigationMemory(
      JSON.stringify({
        version: 1,
        activeView: "delivery",
        activeDetail: "delivery-package",
        workspacePath: "/tmp/canisend",
        selectedJobs: { "/tmp/canisend": "job-1" },
        lastAction: {
          operation: "package.export",
          summary: "Exported the current package",
          route: {
            view: "delivery",
            detail: "delivery-package",
            jobId: "job-1",
          },
          workspacePath: "/tmp/canisend",
          jobId: "job-1",
          occurredAt: "2026-07-30T10:30:00Z",
        },
      }),
    );

    expect(restored).toMatchObject({
      activeView: "delivery",
      activeDetail: "delivery-package",
      selectedJobs: { "/tmp/canisend": "job-1" },
      lastAction: {
        operation: "package.export",
        route: {
          view: "delivery",
          detail: "delivery-package",
          jobId: "job-1",
        },
      },
    });
    expect(
      applicationSectionForRoute({
        view: restored.activeView,
        detail: restored.activeDetail ?? undefined,
      }),
    ).toBe("review-export");
  });
});

describe("Pack-aware navigation", () => {
  it("keeps legacy academic destinations out of Generic Workspaces", () => {
    expect(isNavigationAvailableForPack("applications", genericPack)).toBe(true);
    expect(isNavigationAvailableForPack("agent", genericPack)).toBe(true);
    expect(isNavigationAvailableForPack("opportunities", genericPack)).toBe(false);
    expect(isNavigationAvailableForPack("profile", genericPack)).toBe(false);
    expect(isNavigationAvailableForPack("workflow", genericPack)).toBe(false);
    expect(isNavigationAvailableForPack("delivery", genericPack)).toBe(false);
    expect(
      routeForWorkspacePack({ view: "profile", detail: "profile-sources" }, genericPack),
    ).toEqual({ view: "applications" });
  });

  it("preserves every academic compatibility destination", () => {
    expect(isNavigationAvailableForPack("profile", academicPack)).toBe(true);
    expect(
      routeForWorkspacePack(
        { view: "delivery", detail: "delivery-review", jobId: "job-1" },
        academicPack,
      ),
    ).toEqual({ view: "delivery", detail: "delivery-review", jobId: "job-1" });
  });
});

describe("connected workflow routing", () => {
  it("projects legacy detail routes into five application workspace sections", () => {
    expect(routeForApplicationSection("overview", "job-1")).toEqual({
      view: "applications",
      jobId: "job-1",
    });
    expect(routeForApplicationSection("job-criteria", "job-1")).toEqual({
      view: "workflow",
      detail: "decision-criteria",
      jobId: "job-1",
    });
    expect(routeForApplicationSection("evidence-fit", "job-1")).toEqual({
      view: "workflow",
      detail: "decision-matches",
      jobId: "job-1",
    });
    expect(routeForApplicationSection("materials", "job-1")).toEqual({
      view: "delivery",
      detail: "delivery-documents",
      jobId: "job-1",
    });
    expect(routeForApplicationSection("review-export", "job-1")).toEqual({
      view: "delivery",
      detail: "delivery-review",
      jobId: "job-1",
    });

    expect(
      applicationSectionForRoute({
        view: "applications",
        detail: "source-intake",
      }),
    ).toBe("job-criteria");
    expect(
      applicationSectionForRoute({
        view: "profile",
        detail: "profile-evidence",
      }),
    ).toBe("evidence-fit");
    expect(
      applicationSectionForRoute({
        view: "workflow",
        detail: "decision-plan",
      }),
    ).toBe("evidence-fit");
    expect(
      applicationSectionForRoute({
        view: "delivery",
        detail: "delivery-render",
      }),
    ).toBe("review-export");
    expect(isApplicationWorkspaceRoute({ view: "opportunities" })).toBe(false);
  });

  it("maps exact workflow and agent proposal destinations", () => {
    expect(routeForWorkflowStage("criteria")).toEqual({
      view: "workflow",
      detail: "decision-criteria",
    });
    expect(routeForTaskOperation("document-review")).toEqual({
      view: "delivery",
      detail: "delivery-review",
    });
    expect(routeForTaskOperation("portfolio-draft")).toEqual({
      view: "delivery",
      detail: "delivery-documents",
    });
    expect(routeForAgentAction("profile.evidence.confirm")).toEqual({
      view: "profile",
      detail: "profile-evidence",
    });
    expect(routeForAgentAction("document.review")).toEqual({
      view: "delivery",
      detail: "delivery-review",
    });
    expect(
      routeForContentEntry({
        category: "materials",
        stage: "draft",
        subject_jobs: [
          {
            id: "job-1",
            title: "Lecturer",
            institution: "University X",
            archived: false,
          },
        ],
      }),
    ).toEqual({
      view: "delivery",
      detail: "delivery-documents",
      jobId: "job-1",
    });
    expect(
      routeForContentEntry({
        category: "profile",
        stage: "evidence",
        subject_jobs: [],
      }),
    ).toEqual({
      view: "profile",
      detail: "profile-sources",
    });
  });

  it("recommends the first unmet durable workflow requirement", () => {
    expect(
      recommendWorkflowRoute({
        workspacePath: null,
        jobs: [],
        selectedJob: null,
        profileSourceCount: 0,
      }).reason,
    ).toBe("choose-workspace");

    expect(
      recommendWorkflowRoute({
        workspacePath: "/tmp/canisend",
        jobs: [job],
        selectedJob: detail(),
        profileSourceCount: 1,
      }),
    ).toMatchObject({
      reason: "attach-source",
      route: { view: "applications", detail: "source-intake", jobId: "job-1" },
    });

    const sourcedJob = { ...job, source_ids: ["source-1"] };
    const workflowDetail = detail({
      job: sourcedJob,
      workflow: {
        run_id: "run-1",
        job_id: "job-1",
        status: "active",
        stages: [
          {
            stage: "intake",
            status: "complete",
            execution_mode: "deterministic",
            output: null,
            updated_at: "2026-07-30T10:00:00Z",
          },
          {
            stage: "criteria",
            status: "awaiting-user",
            execution_mode: "user-decision",
            output: null,
            updated_at: "2026-07-30T10:01:00Z",
          },
        ],
        blockers: [],
        next_actions: [],
      },
    });
    expect(
      recommendWorkflowRoute({
        workspacePath: "/tmp/canisend",
        jobs: [sourcedJob],
        selectedJob: workflowDetail,
        profileSourceCount: 1,
      }),
    ).toMatchObject({
      reason: "continue-workflow",
      route: { view: "workflow", detail: "decision-criteria", jobId: "job-1" },
    });

    const dossier: ApplicationDossierReadModel = {
      workspace: "/tmp/canisend",
      job: sourcedJob,
      metadata: {
        origin: "direct",
        discovery_lead_id: null,
        discovery_source_id: null,
        location: null,
        deadline: null,
        source_url: null,
        freshness: null,
        last_seen_at: null,
      },
      source_count: 1,
      profile_source_count: 1,
      state: "in-progress",
      current_stage: "review",
      completed_stages: 8,
      total_stages: 11,
      workflow: workflowDetail.workflow,
      blockers: [],
      next_actions: [],
    };
    expect(
      recommendWorkflowRoute({
        workspacePath: "/tmp/canisend",
        jobs: [sourcedJob],
        selectedJob: workflowDetail,
        dossier,
        profileSourceCount: 0,
      }),
    ).toMatchObject({
      reason: "continue-workflow",
      route: { view: "delivery", detail: "delivery-review", jobId: "job-1" },
    });
  });
});
