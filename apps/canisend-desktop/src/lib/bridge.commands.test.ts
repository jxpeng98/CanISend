import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  open: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.invoke,
  isTauri: () => true,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: mocks.open,
}));

import {
  beginWorkflowStage,
  confirmPlan,
  exportPackage,
  installCli,
  previewDiscoveryFile,
} from "./bridge";

describe("typed Tauri command requests", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.invoke.mockResolvedValue({ status: "ok" });
  });

  it("preserves explicit private-read consent for discovery previews", async () => {
    await previewDiscoveryFile({
      path: "/tmp/leads.csv",
      sourceName: "Reviewed",
      hostAgent: false,
      confirmedPrivateRead: true,
    });

    expect(mocks.invoke).toHaveBeenCalledWith("preview_discovery_file", {
      request: {
        path: "/tmp/leads.csv",
        source_name: "Reviewed",
        source_url: null,
        host_agent: false,
        confirmed_private_read: true,
      },
    });
  });

  it("sends workflow enums through the shared kebab-case contract", async () => {
    await beginWorkflowStage(
      "/tmp/workspace",
      "job-id",
      "criteria",
      "host-agent",
    );

    expect(mocks.invoke).toHaveBeenCalledWith("begin_workflow_stage", {
      request: {
        workspace: "/tmp/workspace",
        job_id: "job-id",
        stage: "criteria",
        mode: "host-agent",
      },
    });
  });

  it("keeps reviewed plan candidates inside the bounded command envelope", async () => {
    const candidate = { decision: "apply" };
    await confirmPlan("/tmp/workspace", "job-id", candidate, true);

    expect(mocks.invoke).toHaveBeenCalledWith("confirm_plan", {
      request: {
        workspace: "/tmp/workspace",
        job_id: "job-id",
        candidate,
        confirmed_private_read: true,
      },
    });
  });

  it("separates private export consent from the job-scoped destination", async () => {
    await exportPackage(
      "/tmp/workspace",
      "job-id",
      "jobs/job-id/application",
      true,
    );

    expect(mocks.invoke).toHaveBeenCalledWith("export_package", {
      request: {
        workspace: "/tmp/workspace",
        job_id: "job-id",
        destination: "jobs/job-id/application",
        confirmed_private_export: true,
      },
    });
  });

  it("requires an explicit terminal mutation signal for CLI installation", async () => {
    await installCli({
      destination: "/Users/example/.local/bin/canisend",
      replaceExisting: true,
      confirmedTerminalInstall: true,
    });

    expect(mocks.invoke).toHaveBeenCalledWith("install_cli", {
      request: {
        destination: "/Users/example/.local/bin/canisend",
        replace_existing: true,
        confirmed_terminal_install: true,
      },
    });
  });
});
