import { beforeEach, describe, expect, it } from "vitest";

import type {
  AgentAssistanceReadModel,
  AgentSkillsStatusReadModel,
  AgentStreamEvent,
} from "./bridge";
import {
  agentUiState,
  applyAgentStreamEvent,
  appendAgentMessage,
  scopeAgentUiState,
  switchAgentConversationScope,
} from "./agent-state.svelte";

describe("Agent UI architecture boundary", () => {
  beforeEach(() => {
    agentUiState.workspacePath = null;
    agentUiState.selectedApplicationId = "";
    agentUiState.runtime = "codex";
    agentUiState.integrationMode = "handoff";
    agentUiState.prompt = "";
    agentUiState.confirmedProviderSend = false;
    agentUiState.startNew = false;
    agentUiState.capabilities = null;
    agentUiState.context = null;
    agentUiState.assistance = null;
    agentUiState.runtimeCatalog = null;
    agentUiState.handoff = null;
    agentUiState.skillsInstallation = null;
    agentUiState.skillsStatus = null;
    agentUiState.mcpConfiguration = null;
    agentUiState.messages = [];
    agentUiState.lastTurn = null;
    agentUiState.formError = null;
    agentUiState.embeddedSessionState = "not-configured";
    agentUiState.streamSessionId = null;
    agentUiState.lastStreamSequence = 0;
    agentUiState.hostAccessDenied = false;
    agentUiState.nextMessageId = 1;
    agentUiState.activeConversationKey = "codex:workspace";
    agentUiState.conversationCache = {};
  });

  it("uses external-host handoff as the default integration mode", () => {
    expect(agentUiState.integrationMode).toBe("handoff");
  });

  it("returns a new workspace to handoff without carrying rendered conversation state", () => {
    agentUiState.workspacePath = "/tmp/workspace-a";
    agentUiState.runtime = "claude";
    agentUiState.integrationMode = "in-app";
    agentUiState.selectedApplicationId = "019f4876-016d-7b41-b959-f4f2543ffd9f";
    agentUiState.prompt = "Private draft prompt";
    agentUiState.confirmedProviderSend = true;
    agentUiState.messages = [{ id: 1, role: "assistant", text: "Private rendered response" }];
    agentUiState.activeConversationKey = "claude:application:019f4876-016d-7b41-b959-f4f2543ffd9f";
    agentUiState.conversationCache = {
      "claude:workspace": {
        prompt: "Cached prompt",
        startNew: false,
        messages: [{ id: 2, role: "user", text: "Cached message" }],
        lastTurn: null,
      },
    };

    scopeAgentUiState("/tmp/workspace-b");

    expect(agentUiState.integrationMode).toBe("handoff");
    expect(agentUiState.runtime).toBe("claude");
    expect(agentUiState.activeConversationKey).toBe("claude:workspace");
    expect(agentUiState.selectedApplicationId).toBe("");
    expect(agentUiState.prompt).toBe("");
    expect(agentUiState.confirmedProviderSend).toBe(false);
    expect(agentUiState.messages).toEqual([]);
    expect(agentUiState.conversationCache).toEqual({});
  });

  it("preserves current guidance while changing runtime for the same application", () => {
    const assistance = {
      selected_job_id: "job-a",
    } as AgentAssistanceReadModel;
    agentUiState.workspacePath = "/tmp/workspace-a";
    agentUiState.selectedApplicationId = "job-a";
    agentUiState.activeConversationKey = "codex:application:job-a";
    agentUiState.assistance = assistance;
    agentUiState.messages = [{ id: 1, role: "assistant", text: "Codex state" }];

    switchAgentConversationScope("claude", "job-a");

    expect(agentUiState.assistance).toBe(assistance);
    expect(agentUiState.runtime).toBe("claude");
    expect(agentUiState.messages).toEqual([]);

    switchAgentConversationScope("codex", "job-a");

    expect(agentUiState.assistance).toBe(assistance);
    expect(agentUiState.messages).toEqual([{ id: 1, role: "assistant", text: "Codex state" }]);
  });

  it("preserves workspace-scoped Skill status while changing application scope", () => {
    const skillsStatus = {
      host: "codex",
      state: "up-to-date",
    } as AgentSkillsStatusReadModel;
    agentUiState.workspacePath = "/tmp/workspace-a";
    agentUiState.selectedApplicationId = "job-a";
    agentUiState.activeConversationKey = "codex:application:job-a";
    agentUiState.skillsStatus = skillsStatus;

    switchAgentConversationScope("codex", "job-b");

    expect(agentUiState.skillsStatus).toBe(skillsStatus);
  });

  it("isolates application conversations and restores their local rendered state", () => {
    agentUiState.workspacePath = "/tmp/workspace-a";
    agentUiState.selectedApplicationId = "job-a";
    agentUiState.activeConversationKey = "codex:application:job-a";
    agentUiState.assistance = {
      selected_job_id: "job-a",
    } as AgentAssistanceReadModel;
    agentUiState.prompt = "Continue application A";
    agentUiState.messages = [{ id: 1, role: "user", text: "Application A" }];

    switchAgentConversationScope("codex", "job-b");

    expect(agentUiState.assistance).toBeNull();
    expect(agentUiState.prompt).toBe("");
    expect(agentUiState.messages).toEqual([]);
    agentUiState.messages = [{ id: 2, role: "user", text: "Application B" }];

    switchAgentConversationScope("codex", "job-a");

    expect(agentUiState.assistance).toBeNull();
    expect(agentUiState.prompt).toBe("Continue application A");
    expect(agentUiState.messages).toEqual([{ id: 1, role: "user", text: "Application A" }]);
  });

  it("revokes send consent and rejects late events after leaving and returning to an Application", () => {
    switchAgentConversationScope("codex", "application-a");
    const messageId = appendAgentMessage("assistant", "");
    const epoch = agentUiState.conversationEpoch;
    agentUiState.confirmedProviderSend = true;
    switchAgentConversationScope("codex", "application-b");
    switchAgentConversationScope("codex", "application-a");
    expect(agentUiState.confirmedProviderSend).toBe(false);
    expect(
      applyAgentStreamEvent(
        {
          sequence: 1,
          desktop_session_id: "stale-session",
          desktop_turn_id: "stale-turn",
          kind: "assistant-delta",
          state: "running",
          text: "Late private reply",
          method: null,
          provider_event_id: null,
        },
        messageId,
        epoch,
      ),
    ).toBe(false);
    expect(agentUiState.messages[0]?.text).toBe("");
    expect(agentUiState.streamSessionId).toBeNull();
    expect(agentUiState.embeddedSessionState).toBe("not-configured");
  });

  it("shows only denied host permissions and clears them on a scope switch", () => {
    const request = (sequence: number, method: string): AgentStreamEvent => ({
      sequence,
      desktop_session_id: "session",
      desktop_turn_id: "turn",
      kind: "server-request",
      state: null,
      text: null,
      method,
      provider_event_id: "request",
    });
    applyAgentStreamEvent(request(1, "mcpServer/elicitation/request"), 1);
    expect(agentUiState.hostAccessDenied).toBe(false);
    for (const [index, method] of [
      "item/commandExecution/requestApproval",
      "item/fileChange/requestApproval",
      "item/permissions/requestApproval",
    ].entries()) {
      agentUiState.hostAccessDenied = false;
      applyAgentStreamEvent(request(index + 2, method), 1);
      expect(agentUiState.hostAccessDenied).toBe(true);
    }
    switchAgentConversationScope("codex", "another-application");
    expect(agentUiState.hostAccessDenied).toBe(false);
  });

  it("reduces ordered stream events into one assistant message", () => {
    const messageId = appendAgentMessage("assistant", "");
    const event = (sequence: number, text: string): AgentStreamEvent => ({
      sequence,
      desktop_session_id: "desktop-session",
      desktop_turn_id: "desktop-turn",
      kind: "assistant-delta",
      state: null,
      text,
      method: null,
      provider_event_id: "item-1",
    });

    expect(applyAgentStreamEvent(event(1, "Hello"), messageId)).toBe(true);
    expect(applyAgentStreamEvent(event(1, " duplicate"), messageId)).toBe(false);
    expect(applyAgentStreamEvent(event(2, " world"), messageId)).toBe(true);
    applyAgentStreamEvent(
      {
        ...event(3, ""),
        kind: "completed",
        state: "ready",
        text: null,
      },
      messageId,
    );

    expect(agentUiState.messages).toEqual([
      { id: messageId, role: "assistant", text: "Hello world" },
    ]);
    expect(agentUiState.embeddedSessionState).toBe("ready");
  });
});
