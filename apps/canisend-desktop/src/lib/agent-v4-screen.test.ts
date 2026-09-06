// @vitest-environment jsdom

import { cleanup, fireEvent, render, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import { agentUiState, applyAgentStreamEvent, scopeAgentUiState } from "./agent-state.svelte";
import type { WorkspaceReadModel } from "./bridge";
import { messages } from "./i18n";
import AgentView from "./views/AgentView.svelte";

afterEach(cleanup);

describe("Agent Workspace v4 screen", () => {
  it.each(["", "generic-application", "academic-application"])(
    "loads the runtime catalog for Application scope %s",
    async (applicationId) => {
      const workspace = {
        path: "/tmp/canisend-v4-workspace",
        status: { workspace_format: "canisend.workspace/v4" },
      } as WorkspaceReadModel;
      agentUiState.workspacePath = null;
      scopeAgentUiState(workspace.path);
      agentUiState.selectedApplicationId = "legacy-job";
      agentUiState.activeConversationKey = "codex:legacy-job";
      const onCodexSignIn = vi.fn(async () => true);
      const onLoadRuntimes = vi.fn(async () => null);
      const onLoadAssistance = vi.fn(async () => null);

      const copy = applicationId === "generic-application" ? messages["zh-CN"] : messages.en;
      const rendered = render(AgentView, {
        copy,
        desktopRuntime: true,
        activeWorkspace: workspace,
        jobs: [],
        selectedApplicationId: applicationId,
        focus: null,
        busy: false,
        turnRunning: false,
        onSelectApplication: vi.fn(async () => true),
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
        onCodexSignIn,
        onRunTurn: vi.fn(async () => null),
        onCancelTurn: vi.fn(async () => false),
        onExport: vi.fn(async () => null),
      });

      await waitFor(() => expect(onLoadRuntimes).toHaveBeenCalledOnce());
      expect(onLoadRuntimes.mock.calls[0]).toEqual([applicationId || undefined]);
      if (applicationId) expect(onLoadAssistance).toHaveBeenCalledWith(applicationId);
      else expect(onLoadAssistance).not.toHaveBeenCalled();
      expect(agentUiState.selectedApplicationId).toBe(applicationId);
      agentUiState.integrationMode = "in-app";
      const loginButton = await rendered.findByRole("button", { name: copy.codexSignIn });
      agentUiState.confirmedProviderSend = true;
      onCodexSignIn.mockResolvedValueOnce(false);
      await fireEvent.click(loginButton);
      await waitFor(() => expect(onCodexSignIn).toHaveBeenCalledOnce());
      await waitFor(() => expect(loginButton.hasAttribute("disabled")).toBe(false));
      expect(agentUiState.confirmedProviderSend).toBe(false);
      expect(agentUiState.startNew).toBe(true);
      await fireEvent.click(loginButton);
      await waitFor(() => expect(onCodexSignIn).toHaveBeenCalledTimes(2));
      await waitFor(() => expect(rendered.getByText(copy.codexSignInComplete)).toBeTruthy());
      expect(agentUiState.startNew).toBe(true);
      expect(agentUiState.confirmedProviderSend).toBe(false);
      applyAgentStreamEvent(
        {
          sequence: 1,
          desktop_session_id: "session",
          desktop_turn_id: "turn",
          kind: "server-request",
          state: null,
          text: null,
          method: "item/permissions/requestApproval",
          provider_event_id: "request",
        },
        1,
      );
      await waitFor(() =>
        expect(
          rendered.getByText(copy.agentHostAccessDenied).closest('[role="alert"]'),
        ).not.toBeNull(),
      );
    },
  );
});
