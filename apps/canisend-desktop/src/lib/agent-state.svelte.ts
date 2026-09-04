import type {
  AgentAssistanceReadModel,
  AgentCapabilitiesReadModel,
  AgentContextReadModel,
  AgentHandoffReadModel,
  AgentMcpConfigurationReadModel,
  AgentPackExportReadModel,
  AgentRuntimeCatalog,
  AgentRuntimeKind,
  AgentEmbeddedSessionState,
  AgentStreamEvent,
  AgentSkillsInstallReadModel,
  AgentSkillsStatusReadModel,
  AgentTurnResult,
} from "$lib/bridge";

export type AgentChatMessage = {
  id: number;
  role: "user" | "assistant";
  text: string;
};

type AgentConversationSnapshot = {
  prompt: string;
  confirmedProviderSend: boolean;
  startNew: boolean;
  messages: AgentChatMessage[];
  lastTurn: AgentTurnResult | null;
};

type AgentUiState = {
  workspacePath: string | null;
  selectedJobId: string;
  runtime: AgentRuntimeKind;
  prompt: string;
  confirmedProviderSend: boolean;
  startNew: boolean;
  capabilities: AgentCapabilitiesReadModel | null;
  context: AgentContextReadModel | null;
  assistance: AgentAssistanceReadModel | null;
  runtimeCatalog: AgentRuntimeCatalog | null;
  handoff: AgentHandoffReadModel | null;
  skillsInstallation: AgentSkillsInstallReadModel | null;
  skillsStatus: AgentSkillsStatusReadModel | null;
  mcpConfiguration: AgentMcpConfigurationReadModel | null;
  integrationMode: "handoff" | "in-app";
  messages: AgentChatMessage[];
  lastTurn: AgentTurnResult | null;
  host: "codex" | "claude" | "generic";
  destination: string;
  exported: AgentPackExportReadModel | null;
  formError: string | null;
  embeddedSessionState: AgentEmbeddedSessionState;
  streamSessionId: string | null;
  lastStreamSequence: number;
  nextMessageId: number;
  activeConversationKey: string;
  conversationCache: Record<string, AgentConversationSnapshot>;
};

export const agentUiState = $state<AgentUiState>({
  workspacePath: null,
  selectedJobId: "",
  runtime: "codex",
  prompt: "",
  confirmedProviderSend: false,
  startNew: false,
  capabilities: null,
  context: null,
  assistance: null,
  runtimeCatalog: null,
  handoff: null,
  skillsInstallation: null,
  skillsStatus: null,
  mcpConfiguration: null,
  integrationMode: "handoff",
  messages: [],
  lastTurn: null,
  host: "codex",
  destination: "",
  exported: null,
  formError: null,
  embeddedSessionState: "not-configured",
  streamSessionId: null,
  lastStreamSequence: 0,
  nextMessageId: 1,
  activeConversationKey: "codex:workspace",
  conversationCache: {},
});

export function scopeAgentUiState(workspacePath: string | null): void {
  if (agentUiState.workspacePath === workspacePath) return;
  agentUiState.workspacePath = workspacePath;
  agentUiState.selectedJobId = "";
  agentUiState.context = null;
  agentUiState.assistance = null;
  agentUiState.runtimeCatalog = null;
  agentUiState.handoff = null;
  agentUiState.skillsInstallation = null;
  agentUiState.skillsStatus = null;
  agentUiState.mcpConfiguration = null;
  agentUiState.integrationMode = "handoff";
  agentUiState.messages = [];
  agentUiState.lastTurn = null;
  agentUiState.prompt = "";
  agentUiState.confirmedProviderSend = false;
  agentUiState.startNew = false;
  agentUiState.formError = null;
  agentUiState.embeddedSessionState = "not-configured";
  agentUiState.streamSessionId = null;
  agentUiState.lastStreamSequence = 0;
  agentUiState.activeConversationKey = `${agentUiState.runtime}:workspace`;
  agentUiState.conversationCache = {};
}

export function switchAgentConversationScope(runtime: AgentRuntimeKind, jobId: string): void {
  const jobScopeChanged = agentUiState.selectedJobId !== jobId;
  agentUiState.conversationCache[agentUiState.activeConversationKey] = {
    prompt: agentUiState.prompt,
    confirmedProviderSend: agentUiState.confirmedProviderSend,
    startNew: agentUiState.startNew,
    messages: [...agentUiState.messages],
    lastTurn: agentUiState.lastTurn,
  };

  const targetKey = `${runtime}:${jobId || "workspace"}`;
  const target = agentUiState.conversationCache[targetKey];
  agentUiState.runtime = runtime;
  agentUiState.selectedJobId = jobId;
  agentUiState.prompt = target?.prompt ?? "";
  agentUiState.confirmedProviderSend = target?.confirmedProviderSend ?? false;
  agentUiState.startNew = target?.startNew ?? false;
  agentUiState.messages = target ? [...target.messages] : [];
  agentUiState.lastTurn = target?.lastTurn ?? null;
  agentUiState.formError = null;
  agentUiState.embeddedSessionState = "not-configured";
  agentUiState.streamSessionId = null;
  agentUiState.lastStreamSequence = 0;
  agentUiState.handoff = null;
  if (jobScopeChanged) agentUiState.assistance = null;
  agentUiState.skillsInstallation = null;
  agentUiState.mcpConfiguration = null;
  agentUiState.activeConversationKey = targetKey;
}

export function appendAgentMessage(role: AgentChatMessage["role"], text: string): number {
  const id = agentUiState.nextMessageId;
  agentUiState.messages.push({
    id,
    role,
    text,
  });
  agentUiState.nextMessageId += 1;
  return id;
}

export function applyAgentStreamEvent(
  event: AgentStreamEvent,
  assistantMessageId: number,
): boolean {
  if (agentUiState.streamSessionId !== event.desktop_session_id) {
    agentUiState.streamSessionId = event.desktop_session_id;
    agentUiState.lastStreamSequence = 0;
  }
  if (event.sequence <= agentUiState.lastStreamSequence) return false;
  agentUiState.lastStreamSequence = event.sequence;
  if (event.state) agentUiState.embeddedSessionState = event.state;
  switch (event.kind) {
    case "assistant-delta": {
      if (!event.text) return true;
      const message = agentUiState.messages.find((item) => item.id === assistantMessageId);
      if (message) message.text += event.text;
      return true;
    }
    case "status":
    case "server-request":
    case "completed":
    case "interrupted":
    case "failed":
      return true;
  }
}

export function reconcileAgentMessage(messageId: number, text: string): void {
  const message = agentUiState.messages.find((item) => item.id === messageId);
  if (message) message.text = text;
}

export function removeEmptyAgentMessage(messageId: number): void {
  const message = agentUiState.messages.find((item) => item.id === messageId);
  if (message?.text) return;
  agentUiState.messages = agentUiState.messages.filter((item) => item.id !== messageId);
}

export function beginNewAgentConversation(): void {
  agentUiState.messages = [];
  agentUiState.lastTurn = null;
  agentUiState.prompt = "";
  agentUiState.startNew = true;
  agentUiState.formError = null;
  agentUiState.embeddedSessionState = "not-configured";
  agentUiState.streamSessionId = null;
  agentUiState.lastStreamSequence = 0;
}
