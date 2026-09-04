// @vitest-environment jsdom

import { cleanup, render, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import { agentUiState, scopeAgentUiState } from "./agent-state.svelte";
import type { WorkspaceReadModel } from "./bridge";
import { messages } from "./i18n";
import AgentView from "./views/AgentView.svelte";

afterEach(cleanup);

describe("Agent Workspace v4 screen", () => {
  it("clears a stale legacy Job before loading the runtime catalog", async () => {
    const workspace = {
      path: "/tmp/canisend-v4-workspace",
      status: { workspace_format: "canisend.workspace/v4" },
    } as WorkspaceReadModel;
    agentUiState.workspacePath = null;
    scopeAgentUiState(workspace.path);
    agentUiState.selectedJobId = "legacy-job";
    agentUiState.activeConversationKey = "codex:legacy-job";
    const onLoadRuntimes = vi.fn(async () => null);
    const onLoadAssistance = vi.fn(async () => null);

    render(AgentView, {
      copy: messages.en,
      desktopRuntime: true,
      activeWorkspace: workspace,
      jobs: [],
      selectedJobId: "",
      focus: "agent-handoff",
      busy: false,
      turnRunning: false,
      onSelectJob: vi.fn(async () => true),
      onNavigate: vi.fn(async () => undefined),
      onLoadCapabilities: vi.fn(async () => null),
      onLoadContext: vi.fn(async () => null),
      onLoadAssistance,
      onPrepareHandoff: vi.fn(async () => null),
      onInstallSkills: vi.fn(async () => null),
      onLoadSkills: vi.fn(async () => null),
      onUninstallSkills: vi.fn(async () => null),
      onCopyHandoff: vi.fn(async () => true),
      onPrepareMcpConfiguration: vi.fn(async () => null),
      onCopyMcpConfiguration: vi.fn(async () => true),
      onLoadRuntimes,
      onRunTurn: vi.fn(async () => null),
      onCancelTurn: vi.fn(async () => false),
      onExport: vi.fn(async () => null),
    });

    await waitFor(() => expect(onLoadRuntimes).toHaveBeenCalledOnce());
    expect(onLoadRuntimes.mock.calls[0]).toEqual([undefined]);
    expect(onLoadAssistance).not.toHaveBeenCalled();
    expect(agentUiState.selectedJobId).toBe("");
  });
});
