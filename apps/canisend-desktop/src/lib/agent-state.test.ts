import { beforeEach, describe, expect, it } from "vitest";

import type {
  AgentAssistanceReadModel,
  AgentSkillsStatusReadModel,
} from "./bridge";
import {
  agentUiState,
  scopeAgentUiState,
  switchAgentConversationScope,
} from "./agent-state.svelte";

describe("Agent UI architecture boundary", () => {
  beforeEach(() => {
    agentUiState.workspacePath = null;
    agentUiState.selectedJobId = "";
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
    agentUiState.selectedJobId = "019f4876-016d-7b41-b959-f4f2543ffd9f";
    agentUiState.prompt = "Private draft prompt";
    agentUiState.confirmedProviderSend = true;
    agentUiState.messages = [
      { id: 1, role: "assistant", text: "Private rendered response" },
    ];
    agentUiState.activeConversationKey =
      "claude:019f4876-016d-7b41-b959-f4f2543ffd9f";
    agentUiState.conversationCache = {
      "claude:workspace": {
        prompt: "Cached prompt",
        confirmedProviderSend: true,
        startNew: false,
        messages: [{ id: 2, role: "user", text: "Cached message" }],
        lastTurn: null,
      },
    };

    scopeAgentUiState("/tmp/workspace-b");

    expect(agentUiState.integrationMode).toBe("handoff");
    expect(agentUiState.runtime).toBe("claude");
    expect(agentUiState.activeConversationKey).toBe("claude:workspace");
    expect(agentUiState.selectedJobId).toBe("");
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
    agentUiState.selectedJobId = "job-a";
    agentUiState.activeConversationKey = "codex:job-a";
    agentUiState.assistance = assistance;
    agentUiState.messages = [{ id: 1, role: "assistant", text: "Codex state" }];

    switchAgentConversationScope("claude", "job-a");

    expect(agentUiState.assistance).toBe(assistance);
    expect(agentUiState.runtime).toBe("claude");
    expect(agentUiState.messages).toEqual([]);

    switchAgentConversationScope("codex", "job-a");

    expect(agentUiState.assistance).toBe(assistance);
    expect(agentUiState.messages).toEqual([
      { id: 1, role: "assistant", text: "Codex state" },
    ]);
  });

  it("preserves workspace-scoped Skill status while changing application scope", () => {
    const skillsStatus = {
      host: "codex",
      state: "up-to-date",
    } as AgentSkillsStatusReadModel;
    agentUiState.workspacePath = "/tmp/workspace-a";
    agentUiState.selectedJobId = "job-a";
    agentUiState.activeConversationKey = "codex:job-a";
    agentUiState.skillsStatus = skillsStatus;

    switchAgentConversationScope("codex", "job-b");

    expect(agentUiState.skillsStatus).toBe(skillsStatus);
  });

  it("isolates application conversations and restores their local rendered state", () => {
    agentUiState.workspacePath = "/tmp/workspace-a";
    agentUiState.selectedJobId = "job-a";
    agentUiState.activeConversationKey = "codex:job-a";
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
    expect(agentUiState.messages).toEqual([
      { id: 1, role: "user", text: "Application A" },
    ]);
  });
});
