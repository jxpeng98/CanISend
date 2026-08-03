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
  approveGenericApplication,
  cancelAgentTurn,
  commitJobSourcePreview,
  confirmPlan,
  configureCliPath,
  createGenericApplication,
  copyAgentHandoff,
  copyAgentMcpConfiguration,
  exportPackage,
  exportRenderAndOpen,
  getAgentAssistance,
  getAgentSkillsStatus,
  getApplicationDossier,
  getContentCatalog,
  getWorkflowPackPresentation,
  GENERIC_APPLICATION_WORKFLOW_PACK_ID,
  installAgentSkills,
  installCli,
  listApplicationDossiers,
  migrateWorkspaceV3,
  reviewGenericApplication,
  prepareAgentHandoff,
  prepareAgentMcpConfiguration,
  previewDiscoveryFile,
  previewLocalJobSource,
  previewWorkspaceV3Migration,
  previewRender,
  previewUrlJobSource,
  runAgentTurn,
  searchContent,
  uninstallAgentSkills,
} from "./bridge";

describe("typed Tauri command requests", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.invoke.mockResolvedValue({ status: "ok" });
  });

  it("preserves explicit private-read consent for discovery previews", async () => {
    await previewDiscoveryFile({
      workspace: "/tmp/workspace",
      path: "/tmp/leads.csv",
      sourceName: "Reviewed",
      hostAgent: false,
      confirmedPrivateRead: true,
    });

    expect(mocks.invoke).toHaveBeenCalledWith("preview_discovery_file", {
      request: {
        workspace: "/tmp/workspace",
        path: "/tmp/leads.csv",
        source_name: "Reviewed",
        source_url: null,
        host_agent: false,
        confirmed_private_read: true,
      },
    });
  });

  it("previews local and URL job sources without committing either source", async () => {
    await previewLocalJobSource(
      "/tmp/workspace",
      "job-id",
      "/tmp/advert.pdf",
      true,
    );
    await previewUrlJobSource(
      "/tmp/workspace",
      "job-id",
      "https://example.edu/advert.pdf",
      true,
    );

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "preview_local_job_source", {
      request: {
        workspace: "/tmp/workspace",
        job_id: "job-id",
        source: "/tmp/advert.pdf",
        confirmed_private_read: true,
      },
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "preview_url_job_source", {
      request: {
        workspace: "/tmp/workspace",
        job_id: "job-id",
        url: "https://example.edu/advert.pdf",
        confirmed_network_fetch: true,
      },
    });
  });

  it("commits only an opaque reviewed job-intake preview token", async () => {
    await commitJobSourcePreview("/tmp/workspace", "job-intake-preview-123");

    expect(mocks.invoke).toHaveBeenCalledWith("commit_job_source_preview", {
      request: {
        workspace: "/tmp/workspace",
        preview_token: "job-intake-preview-123",
      },
    });
  });

  it("loads unified application dossiers through body-free desktop commands", async () => {
    await listApplicationDossiers("/tmp/workspace", false);
    await getApplicationDossier("/tmp/workspace", "job-id");

    expect(mocks.invoke).toHaveBeenNthCalledWith(
      1,
      "list_application_dossiers",
      {
        request: {
          workspace: "/tmp/workspace",
          include_archived: false,
        },
      },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "application_dossier", {
      request: {
        workspace: "/tmp/workspace",
        job_id: "job-id",
      },
    });
  });

  it("keeps the canonical v3 request and review consent boundaries explicit", async () => {
    const candidate = {
      title: "Community grant",
      opportunity_metadata: {},
      application_metadata: {},
      source_text: "A narrative is required.",
      requirements: [
        {
          category: "format",
          statement: "A narrative is required.",
          priority: "mandatory" as const,
          start_byte: 0,
          end_byte: 24,
        },
      ],
    };
    await createGenericApplication("/tmp/workspace", candidate);
    await reviewGenericApplication("/tmp/workspace", "application-id", true);
    await approveGenericApplication("/tmp/workspace", "application-id", 3);

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "create_generic_application", {
      request: { workspace: "/tmp/workspace", request: candidate },
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "review_generic_application", {
      request: {
        workspace: "/tmp/workspace",
        application_id: "application-id",
        confirmed_private_read: true,
      },
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "approve_generic_application", {
      request: {
        workspace: "/tmp/workspace",
        application_id: "application-id",
        expected_revision: 3,
      },
    });
  });

  it("commits only the exact reviewed migration digest with a separate backup", async () => {
    await previewWorkspaceV3Migration("/tmp/workspace");
    await migrateWorkspaceV3({
      workspace: "/tmp/workspace",
      expectedPlanSha256: "a".repeat(64),
      backupDestination: "/tmp/workspace-v2-backup",
    });

    expect(mocks.invoke).toHaveBeenNthCalledWith(
      1,
      "preview_workspace_v3_migration",
      { request: { path: "/tmp/workspace" } },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "migrate_workspace_v3", {
      request: {
        workspace: "/tmp/workspace",
        expected_plan_sha256: "a".repeat(64),
        backup_destination: "/tmp/workspace-v2-backup",
      },
    });
  });

  it("keeps content filters typed and private body consent explicit", async () => {
    await getContentCatalog("/tmp/workspace", {
      job_id: "job-id",
      category: "materials",
    });
    await searchContent({
      workspace: "/tmp/workspace",
      query: "teaching portfolio",
      filter: { status: "confirmed" },
      includePrivateBodies: true,
      confirmedPrivateRead: true,
      limit: 20,
    });

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "content_catalog", {
      request: {
        workspace: "/tmp/workspace",
        filter: {
          job_id: "job-id",
          category: "materials",
          stage: null,
          status: null,
          privacy: null,
          created_after: null,
          created_before: null,
        },
      },
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "search_content", {
      request: {
        workspace: "/tmp/workspace",
        query: "teaching portfolio",
        filter: {
          job_id: null,
          category: null,
          stage: null,
          status: "confirmed",
          privacy: null,
          created_after: null,
          created_before: null,
        },
        include_private_bodies: true,
        confirmed_private_read: true,
        limit: 20,
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

  it("requests Pack presentation labels with the selected host locale", async () => {
    await getWorkflowPackPresentation(
      "zh-CN",
      GENERIC_APPLICATION_WORKFLOW_PACK_ID,
    );

    expect(mocks.invoke).toHaveBeenCalledWith("workflow_pack_presentation", {
      request: {
        locale: "zh-CN",
        pack_id: "org.canisend.generic-application",
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

  it("reads a current render preview as raw bytes behind private-read consent", async () => {
    mocks.invoke.mockResolvedValueOnce(new Uint8Array([37, 80, 68, 70]).buffer);

    const bytes = await previewRender(
      "/tmp/workspace",
      "job-id",
      "cover-letter",
      true,
    );

    expect([...bytes]).toEqual([37, 80, 68, 70]);
    expect(mocks.invoke).toHaveBeenCalledWith("preview_render", {
      request: {
        workspace: "/tmp/workspace",
        job_id: "job-id",
        kind: "cover-letter",
        confirmed_private_read: true,
      },
    });
  });

  it("opens only a selected job-scoped render behind private-export consent", async () => {
    await exportRenderAndOpen(
      "/tmp/workspace",
      "job-id",
      "jobs/job-id/rendered",
      "cv",
      true,
    );

    expect(mocks.invoke).toHaveBeenCalledWith("export_render_and_open", {
      request: {
        workspace: "/tmp/workspace",
        job_id: "job-id",
        destination: "jobs/job-id/rendered",
        kind: "cv",
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

  it("keeps shell PATH configuration behind terminal consent", async () => {
    await configureCliPath({
      destination: "/Users/example/.local/bin/canisend",
      confirmedTerminalInstall: true,
    });

    expect(mocks.invoke).toHaveBeenCalledWith("configure_cli_path", {
      request: {
        destination: "/Users/example/.local/bin/canisend",
        confirmed_terminal_install: true,
      },
    });
  });

  it("keeps agent runtime scope, continuity, and provider consent explicit", async () => {
    await runAgentTurn({
      workspace: "/tmp/workspace",
      selectedJobId: "job-id",
      runtime: "codex",
      prompt: "Review the next action.",
      startNew: false,
      confirmedProviderSend: true,
    });

    expect(mocks.invoke).toHaveBeenCalledWith("run_agent_turn", {
      request: {
        workspace: "/tmp/workspace",
        selected_job_id: "job-id",
        runtime: "codex",
        prompt: "Review the next action.",
        start_new: false,
        confirmed_provider_send: true,
      },
    });
  });

  it("cancels only the exact active agent runtime scope", async () => {
    await cancelAgentTurn({
      workspace: "/tmp/workspace",
      selectedJobId: "job-id",
      runtime: "claude",
    });

    expect(mocks.invoke).toHaveBeenCalledWith("cancel_agent_turn", {
      request: {
        workspace: "/tmp/workspace",
        selected_job_id: "job-id",
        runtime: "claude",
      },
    });
  });

  it("prepares a body-free external-host handoff for the selected job", async () => {
    await prepareAgentHandoff("claude", "/tmp/workspace", "job-id");

    expect(mocks.invoke).toHaveBeenCalledWith("prepare_agent_handoff", {
      request: {
        host: "claude",
        workspace: "/tmp/workspace",
        selected_job_id: "job-id",
      },
    });
  });

  it("loads body-free contextual assistance for an explicit application", async () => {
    await getAgentAssistance("/tmp/workspace", "job-id");

    expect(mocks.invoke).toHaveBeenCalledWith("agent_assistance", {
      request: {
        workspace: "/tmp/workspace",
        job_id: "job-id",
      },
    });
  });

  it("copies only a regenerated handoff field through the native adapter", async () => {
    await copyAgentHandoff(
      "codex",
      "/tmp/workspace",
      "job-id",
      "bootstrap-prompt",
    );

    expect(mocks.invoke).toHaveBeenCalledWith("copy_agent_handoff", {
      request: {
        host: "codex",
        workspace: "/tmp/workspace",
        selected_job_id: "job-id",
        field: "bootstrap-prompt",
      },
    });
  });

  it("installs host-discoverable CanISend skills into the workspace", async () => {
    await installAgentSkills("codex", "/tmp/workspace");

    expect(mocks.invoke).toHaveBeenCalledWith("install_agent_skills", {
      request: {
        host: "codex",
        workspace: "/tmp/workspace",
      },
    });
  });

  it("inspects and safely removes host-discoverable CanISend skills", async () => {
    await getAgentSkillsStatus("claude", "/tmp/workspace");
    expect(mocks.invoke).toHaveBeenLastCalledWith("agent_skills_status", {
      request: {
        host: "claude",
        workspace: "/tmp/workspace",
      },
    });

    await uninstallAgentSkills("claude", "/tmp/workspace");
    expect(mocks.invoke).toHaveBeenLastCalledWith("uninstall_agent_skills", {
      request: {
        host: "claude",
        workspace: "/tmp/workspace",
      },
    });
  });

  it("prepares host-specific MCP configuration for the selected workspace", async () => {
    await prepareAgentMcpConfiguration("codex", "/tmp/workspace");

    expect(mocks.invoke).toHaveBeenCalledWith(
      "prepare_agent_mcp_configuration",
      {
        request: {
          host: "codex",
          workspace: "/tmp/workspace",
        },
      },
    );
  });

  it("copies only a regenerated MCP configuration field", async () => {
    await copyAgentMcpConfiguration(
      "claude",
      "/tmp/workspace",
      "configuration-snippet",
    );

    expect(mocks.invoke).toHaveBeenCalledWith("copy_agent_mcp_configuration", {
      request: {
        host: "claude",
        workspace: "/tmp/workspace",
        field: "configuration-snippet",
      },
    });
  });
});
