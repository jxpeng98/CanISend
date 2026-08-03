import type {
  AgentAssistanceReadModel,
  AgentCapabilitiesReadModel,
  AgentContextReadModel,
  AgentHandoffReadModel,
  AgentMcpConfigurationReadModel,
  AgentPackExportReadModel,
  AgentRuntimeCatalog,
  AgentRuntimeKind,
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
  agentUiState.handoff = null;
  if (jobScopeChanged) agentUiState.assistance = null;
  agentUiState.skillsInstallation = null;
  agentUiState.mcpConfiguration = null;
  agentUiState.activeConversationKey = targetKey;
}

export function appendAgentMessage(role: AgentChatMessage["role"], text: string): void {
  agentUiState.messages.push({
    id: agentUiState.nextMessageId,
    role,
    text,
  });
  agentUiState.nextMessageId += 1;
}

export function beginNewAgentConversation(): void {
  agentUiState.messages = [];
  agentUiState.lastTurn = null;
  agentUiState.prompt = "";
  agentUiState.startNew = true;
  agentUiState.formError = null;
}
