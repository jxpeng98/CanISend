import { describe, expect, it } from "vitest";

import type {
  ApplicationDossierReadModel,
  JobDetailReadModel,
  JobRecord,
} from "./bridge";
import {
  defaultNavigationMemory,
  parseNavigationMemory,
  recommendWorkflowRoute,
  rememberedJob,
  routeForAgentAction,
  routeForContentEntry,
  routeForTaskOperation,
  routeForWorkflowStage,
} from "./workflow-navigation";

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

  it("restores only a job that still belongs to the workspace", () => {
    const memory = {
      ...defaultNavigationMemory(),
      selectedJobs: { "/tmp/canisend": "job-1" },
    };
    expect(rememberedJob(memory, "/tmp/canisend", [job])).toBe("job-1");
    expect(
      rememberedJob(
        { ...memory, selectedJobs: { "/tmp/canisend": "missing" } },
        "/tmp/canisend",
        [job],
      ),
    ).toBe("job-1");
  });
});

describe("connected workflow routing", () => {
  it("maps exact workflow and agent proposal destinations", () => {
    expect(routeForWorkflowStage("criteria")).toEqual({
      view: "workflow",
      detail: "decision-criteria",
    });
    expect(routeForTaskOperation("document-review")).toEqual({
      view: "delivery",
      detail: "delivery-review",
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
