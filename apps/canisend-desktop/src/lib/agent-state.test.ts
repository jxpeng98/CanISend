import { beforeEach, describe, expect, it } from "vitest";

import {
  agentUiState,
  scopeAgentUiState,
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
    agentUiState.messages = [];
    agentUiState.lastTurn = null;
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
});
