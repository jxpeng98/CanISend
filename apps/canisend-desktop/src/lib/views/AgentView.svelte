<script lang="ts">
  import {
    Bot,
    Check,
    CircleOff,
    Copy,
    Database,
    FileOutput,
    FolderOpen,
    MessageSquarePlus,
    MessagesSquare,
    PlugZap,
    RefreshCw,
    Send,
    ShieldCheck,
  } from "@lucide/svelte";
  import { onMount } from "svelte";

  import {
    agentUiState,
    appendAgentMessage,
    beginNewAgentConversation,
    scopeAgentUiState,
    switchAgentConversationScope,
  } from "$lib/agent-state.svelte";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";
  import {
    chooseExportDirectory,
    type AgentCapabilitiesReadModel,
    type AgentContextReadModel,
    type AgentHandoffReadModel,
    type AgentMcpConfigurationReadModel,
    type AgentPackExportReadModel,
    type AgentRuntimeCatalog,
    type AgentRuntimeKind,
    type AgentSkillsInstallReadModel,
    type AgentTurnResult,
    type JobRecord,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";
  import {
    routeForAgentAction,
    type WorkflowDetail,
    type WorkflowRoute,
  } from "$lib/workflow-navigation";

  type AgentHost = "codex" | "claude" | "generic";
  type HandoffCopyTarget = "start" | "prompt";
  type CopyTarget = "start" | "prompt" | "mcp-command" | "mcp-config";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    activeWorkspace: WorkspaceReadModel | null;
    jobs: JobRecord[];
    selectedJobId: string;
    focus: WorkflowDetail | null;
    busy: boolean;
    turnRunning: boolean;
    onSelectJob: (jobId: string) => Promise<boolean>;
    onNavigate: (route: WorkflowRoute) => Promise<void>;
    onLoadCapabilities: () => Promise<AgentCapabilitiesReadModel | null>;
    onLoadContext: (jobId?: string) => Promise<AgentContextReadModel | null>;
    onPrepareHandoff: (
      host: AgentHost,
      jobId?: string,
    ) => Promise<AgentHandoffReadModel | null>;
    onInstallSkills: (
      host: AgentHost,
    ) => Promise<AgentSkillsInstallReadModel | null>;
    onCopyHandoff: (
      host: AgentHost,
      jobId: string | undefined,
      field: "launch-command" | "start-command" | "bootstrap-prompt",
    ) => Promise<boolean>;
    onPrepareMcpConfiguration: (
      host: AgentHost,
    ) => Promise<AgentMcpConfigurationReadModel | null>;
    onCopyMcpConfiguration: (
      host: AgentHost,
      field: "registration-command" | "configuration-snippet",
    ) => Promise<boolean>;
    onLoadRuntimes: (jobId?: string) => Promise<AgentRuntimeCatalog | null>;
    onRunTurn: (options: {
      jobId?: string;
      runtime: AgentRuntimeKind;
      prompt: string;
      startNew: boolean;
      confirmedProviderSend: boolean;
    }) => Promise<AgentTurnResult | null>;
    onCancelTurn: (options: {
      jobId?: string;
      runtime: AgentRuntimeKind;
    }) => Promise<boolean>;
    onExport: (
      host: AgentHost,
      destination: string,
    ) => Promise<AgentPackExportReadModel | null>;
  };

  let {
    copy,
    desktopRuntime,
    activeWorkspace,
    jobs,
    selectedJobId,
    focus,
    busy,
    turnRunning,
    onSelectJob,
    onNavigate,
    onLoadCapabilities,
    onLoadContext,
    onPrepareHandoff,
    onInstallSkills,
    onCopyHandoff,
    onPrepareMcpConfiguration,
    onCopyMcpConfiguration,
    onLoadRuntimes,
    onRunTurn,
    onCancelTurn,
    onExport,
  }: Props = $props();

  let copied = $state<CopyTarget | null>(null);
  let cancellingTurn = $state(false);
  let observedGlobalScope = $state("");

  const runtimeProbe = $derived(
    agentUiState.runtimeCatalog?.runtimes.find(
      (runtime) => runtime.runtime === agentUiState.runtime,
    ) ?? null,
  );
  const currentSession = $derived(
    agentUiState.runtimeCatalog?.sessions.find(
      (session) => session.runtime === agentUiState.runtime,
    ) ?? null,
  );
  const selectedJob = $derived(
    jobs.find((job) => job.id === agentUiState.selectedJobId) ?? null,
  );

  $effect(() => {
    scopeAgentUiState(activeWorkspace?.path ?? null);
    const globalScope = `${activeWorkspace?.path ?? ""}:${selectedJobId}`;
    if (globalScope !== observedGlobalScope) {
      observedGlobalScope = globalScope;
      if (selectedJobId) {
        switchAgentConversationScope(agentUiState.runtime, selectedJobId);
      }
    }
    if (focus === "agent-handoff" || focus === "agent-task") {
      agentUiState.integrationMode = "handoff";
    }
  });

  onMount(() => {
    if (desktopRuntime) void refreshRuntimes();
  });

  async function refreshRuntimes(): Promise<void> {
    agentUiState.formError = null;
    agentUiState.runtimeCatalog = await onLoadRuntimes(
      agentUiState.selectedJobId || undefined,
    );
  }

  async function changeScope(jobId: string): Promise<void> {
    if (jobId && jobId !== selectedJobId) {
      const selected = await onSelectJob(jobId);
      if (!selected) return;
    }
    switchAgentConversationScope(agentUiState.runtime, jobId);
    agentUiState.context = null;
    agentUiState.handoff = null;
    agentUiState.skillsInstallation = null;
    agentUiState.mcpConfiguration = null;
    agentUiState.runtimeCatalog = await onLoadRuntimes(jobId || undefined);
  }

  function changeHost(host: AgentHost): void {
    agentUiState.host = host;
    agentUiState.handoff = null;
    agentUiState.skillsInstallation = null;
    agentUiState.mcpConfiguration = null;
    if (host !== "generic") {
      switchAgentConversationScope(host, agentUiState.selectedJobId);
    }
  }

  function changeRuntime(runtime: AgentRuntimeKind): void {
    agentUiState.host = runtime;
    switchAgentConversationScope(runtime, agentUiState.selectedJobId);
  }

  async function prepareHandoff(): Promise<void> {
    agentUiState.formError = null;
    if (!activeWorkspace) {
      agentUiState.formError = copy.noWorkspace;
      return;
    }
    const installation = await onInstallSkills(agentUiState.host);
    if (!installation) return;
    agentUiState.skillsInstallation = installation;
    const handoff = await onPrepareHandoff(
      agentUiState.host,
      agentUiState.selectedJobId || undefined,
    );
    if (!handoff) return;
    agentUiState.handoff = handoff;
    agentUiState.context = handoff.context;
  }

  async function copyHandoff(target: HandoffCopyTarget): Promise<void> {
    if (!activeWorkspace) return;
    const field =
      target === "start" ? "start-command" : "bootstrap-prompt";
    const copiedSuccessfully = await onCopyHandoff(
      agentUiState.host,
      agentUiState.selectedJobId || undefined,
      field,
    );
    if (!copiedSuccessfully) {
      agentUiState.formError = copy.copyFailed;
      return;
    }
    copied = target;
    window.setTimeout(() => {
      if (copied === target) copied = null;
    }, 1_800);
  }

  async function prepareMcpConfiguration(): Promise<void> {
    agentUiState.formError = null;
    if (!activeWorkspace) {
      agentUiState.formError = copy.noWorkspace;
      return;
    }
    agentUiState.mcpConfiguration =
      await onPrepareMcpConfiguration(agentUiState.host);
  }

  async function copyMcpConfiguration(
    target: "mcp-command" | "mcp-config",
  ): Promise<void> {
    const field =
      target === "mcp-command"
        ? "registration-command"
        : "configuration-snippet";
    const copiedSuccessfully = await onCopyMcpConfiguration(
      agentUiState.host,
      field,
    );
    if (!copiedSuccessfully) {
      agentUiState.formError = copy.copyFailed;
      return;
    }
    copied = target;
    window.setTimeout(() => {
      if (copied === target) copied = null;
    }, 1_800);
  }

  async function loadCapabilities(): Promise<void> {
    agentUiState.capabilities = await onLoadCapabilities();
  }

  async function loadContext(): Promise<void> {
    agentUiState.context = await onLoadContext(
      agentUiState.selectedJobId || undefined,
    );
  }

  async function sendMessage(): Promise<void> {
    agentUiState.formError = null;
    const prompt = agentUiState.prompt.trim();
    if (!activeWorkspace) {
      agentUiState.formError = copy.noWorkspace;
      return;
    }
    if (!runtimeProbe?.available) {
      agentUiState.formError = copy.noRuntimeFound;
      return;
    }
    if (!prompt) {
      agentUiState.formError = copy.messagePlaceholder;
      return;
    }
    if (!agentUiState.confirmedProviderSend) {
      agentUiState.formError = copy.providerConsent;
      return;
    }
    const result = await onRunTurn({
      jobId: agentUiState.selectedJobId || undefined,
      runtime: agentUiState.runtime,
      prompt,
      startNew: agentUiState.startNew,
      confirmedProviderSend: agentUiState.confirmedProviderSend,
    });
    if (!result) return;
    appendAgentMessage("user", prompt);
    appendAgentMessage("assistant", result.response);
    agentUiState.lastTurn = result;
    agentUiState.prompt = "";
    agentUiState.startNew = false;
    agentUiState.runtimeCatalog = await onLoadRuntimes(
      agentUiState.selectedJobId || undefined,
    );
  }

  async function cancelTurn(): Promise<void> {
    cancellingTurn = true;
    try {
      await onCancelTurn({
        jobId: agentUiState.selectedJobId || undefined,
        runtime: agentUiState.runtime,
      });
    } finally {
      cancellingTurn = false;
    }
  }

  async function chooseDestination(): Promise<void> {
    agentUiState.destination =
      (await chooseExportDirectory()) ?? agentUiState.destination;
  }

  async function exportPack(): Promise<void> {
    agentUiState.formError = null;
    if (!agentUiState.destination) {
      agentUiState.formError = copy.chooseDirectory;
      return;
    }
    agentUiState.exported = await onExport(
      agentUiState.host,
      agentUiState.destination,
    );
  }

  function hostLabel(value: AgentHost): string {
    if (value === "codex") return copy.codex;
    if (value === "claude") return copy.claude;
    return copy.generic;
  }

  function skillStateLabel(
    value: AgentSkillsInstallReadModel["state"],
  ): string {
    if (value === "installed") return copy.skillsInstalled;
    if (value === "updated") return copy.skillsUpdated;
    return copy.skillsUpToDate;
  }

  function shortSessionId(value: string): string {
    return value.length > 18
      ? `${value.slice(0, 8)}…${value.slice(-6)}`
      : value;
  }
</script>

<section class="space-y-6">
  <div class="flex flex-col justify-between gap-4 xl:flex-row xl:items-end">
    <div>
      <Badge variant="secondary" class="mb-3">{copy.agent}</Badge>
      <h1 class="text-3xl font-semibold tracking-[-0.03em]">{copy.agentTitle}</h1>
      <p class="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">
        {copy.agentDescription}
      </p>
    </div>
    <Button
      variant="outline"
      class="min-h-11 shrink-0"
      disabled={!desktopRuntime || busy}
      onclick={refreshRuntimes}
    >
      <RefreshCw size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
      {copy.refreshRuntimes}
    </Button>
  </div>

  <div class="grid gap-3 md:grid-cols-3">
    <div class="rounded-xl border bg-muted/20 p-4">
      <p class="text-xs font-medium text-muted-foreground">
        1 · {copy.controlPlane}
      </p>
      <p class="mt-2 truncate text-sm font-semibold">
        {activeWorkspace?.path ?? copy.noWorkspace}
      </p>
    </div>
    <div class="rounded-xl border bg-muted/20 p-4">
      <p class="text-xs font-medium text-muted-foreground">
        2 · {copy.reasoningPlane}
      </p>
      <p class="mt-2 text-sm font-semibold">{hostLabel(agentUiState.host)}</p>
    </div>
    <div class="rounded-xl border bg-muted/20 p-4">
      <p class="text-xs font-medium text-muted-foreground">
        3 · {copy.workspaceScope}
      </p>
      <p class="mt-2 truncate text-sm font-semibold">
        {selectedJob
          ? `${selectedJob.title} — ${selectedJob.institution}`
          : copy.wholeWorkspace}
      </p>
    </div>
  </div>

  <Tabs.Root bind:value={agentUiState.integrationMode}>
    <Tabs.List class="grid w-full max-w-xl grid-cols-2">
      <Tabs.Trigger value="handoff">
        <MessagesSquare data-icon="inline-start" aria-hidden="true" />
        {copy.externalHostTab}
      </Tabs.Trigger>
      <Tabs.Trigger value="in-app">
        <Bot data-icon="inline-start" aria-hidden="true" />
        {copy.inAppAdvancedTab}
      </Tabs.Trigger>
    </Tabs.List>

    <Tabs.Content
      id={focus === "agent-task" ? "agent-task" : "agent-handoff"}
      value="handoff"
      class={[
        "scroll-mt-44 space-y-6 pt-4",
        focus === "agent-handoff" || focus === "agent-task"
          ? "rounded-xl ring-2 ring-primary/25"
          : "",
      ]}
    >
      <div class="grid gap-6 xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)]">
        <Card.Root class="shadow-none">
          <Card.Header>
            <div class="flex flex-wrap items-start justify-between gap-4">
              <div>
                <Card.Title>{copy.externalHost}</Card.Title>
                <Card.Description class="mt-1.5">
                  {copy.externalHostDescription}
                </Card.Description>
              </div>
              <Badge>{copy.recommended}</Badge>
            </div>
          </Card.Header>
          <Card.Content class="space-y-5">
            <div class="grid gap-3 sm:grid-cols-2">
              {#each ["codex", "claude"] as host}
                <button
                  type="button"
                  class={[
                    "rounded-xl border p-4 text-left transition-colors hover:bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    agentUiState.host === host ? "border-primary bg-primary/5" : "",
                  ]}
                  aria-pressed={agentUiState.host === host}
                  onclick={() => changeHost(host as AgentHost)}
                >
                  <div class="flex items-center justify-between gap-3">
                    <p class="text-sm font-semibold">
                      {host === "codex" ? copy.codex : copy.claude}
                    </p>
                    {#if agentUiState.runtimeCatalog?.runtimes.find((item) => item.runtime === host)?.available}
                      <span class="inline-flex items-center gap-1.5 text-xs font-medium text-[var(--success)]">
                        <Check size={14} strokeWidth={2} aria-hidden="true" />
                        {copy.runtimeAvailable}
                      </span>
                    {:else}
                      <span class="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
                        <CircleOff size={14} strokeWidth={1.8} aria-hidden="true" />
                        {copy.runtimeUnavailable}
                      </span>
                    {/if}
                  </div>
                  <p class="mt-2 text-xs leading-5 text-muted-foreground">
                    {host === "codex" ? "Codex CLI" : "Claude Code"}
                  </p>
                </button>
              {/each}
            </div>

            <div class="space-y-2">
              <Label for="handoff-job">{copy.selectApplication}</Label>
              <select
                id="handoff-job"
                class="min-h-11 w-full rounded-lg border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                value={agentUiState.selectedJobId}
                disabled={!activeWorkspace || busy}
                onchange={(event) => void changeScope(event.currentTarget.value)}
              >
                <option value="">{copy.wholeWorkspace}</option>
                {#each jobs as job (job.id)}
                  <option value={job.id}>{job.title} — {job.institution}</option>
                {/each}
              </select>
            </div>

            <div class="flex items-start gap-3 rounded-xl border bg-muted/20 p-4">
              <ShieldCheck
                size={18}
                strokeWidth={1.8}
                class="mt-0.5 shrink-0 text-[var(--success)]"
                aria-hidden="true"
              />
              <p class="text-xs leading-5 text-muted-foreground">
                {copy.handoffPrivacy}
              </p>
            </div>

            <Button
              class="min-h-11 w-full sm:w-auto"
              disabled={!desktopRuntime || !activeWorkspace || busy}
              onclick={prepareHandoff}
            >
              <MessagesSquare
                size={16}
                strokeWidth={1.8}
                data-icon="inline-start"
                aria-hidden="true"
              />
              {busy ? copy.working : copy.prepareAiWorkspace}
            </Button>
          </Card.Content>
        </Card.Root>

        <Card.Root class="shadow-none">
          <Card.Header>
            <Card.Title>{copy.stateInCanisend}</Card.Title>
            <Card.Description>{copy.sessionInHost}</Card.Description>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div class="rounded-xl border border-[var(--success)]/30 bg-[var(--success)]/8 p-4">
              <div class="flex items-start gap-3">
                <Database
                  size={18}
                  strokeWidth={1.8}
                  class="mt-0.5 shrink-0"
                  aria-hidden="true"
                />
                <div>
                  <p class="text-sm font-semibold">{copy.controlPlane}</p>
                  <p class="mt-1 text-xs leading-5 text-muted-foreground">
                    {copy.controlPlaneDescription}
                  </p>
                </div>
              </div>
            </div>
            <div class="rounded-xl border p-4">
              <div class="flex items-start gap-3">
                <MessagesSquare
                  size={18}
                  strokeWidth={1.8}
                  class="mt-0.5 shrink-0"
                  aria-hidden="true"
                />
                <div>
                  <p class="text-sm font-semibold">{copy.reasoningPlane}</p>
                  <p class="mt-1 text-xs leading-5 text-muted-foreground">
                    {hostLabel(agentUiState.host)} · {copy.reasoningPlaneDescription}
                  </p>
                </div>
              </div>
            </div>
            <Separator />
            <div class="space-y-3 text-xs leading-5 text-muted-foreground">
              <p>1. {copy.handoffStepOne}</p>
              <p>2. {copy.handoffStepTwo}</p>
              <p>3. {copy.handoffStepThree}</p>
            </div>
          </Card.Content>
        </Card.Root>
      </div>

      {#if agentUiState.handoff}
        <Card.Root class="border-primary/30 shadow-none">
          <Card.Header>
            <div class="flex flex-wrap items-start justify-between gap-4">
              <div>
                <Card.Title>{copy.handoffReady}</Card.Title>
                <Card.Description class="mt-1.5">
                  {agentUiState.handoff.workspace}
                </Card.Description>
              </div>
              <div class="flex flex-wrap gap-2">
                <Badge>
                  {agentUiState.handoff.recommended_skill}
                </Badge>
                {#if agentUiState.skillsInstallation}
                  <Badge variant="secondary">
                    {copy.skillsReady} ·
                    {skillStateLabel(agentUiState.skillsInstallation.state)}
                  </Badge>
                {/if}
                <Badge variant="outline">
                  {copy.stateInCanisend}
                </Badge>
                <Badge variant="outline">
                  {copy.sessionInHost}
                </Badge>
              </div>
            </div>
          </Card.Header>
          <Card.Content class="space-y-6">
            <div class="space-y-3">
              <div class="flex items-center justify-between gap-3">
                <div>
                  <Label for="handoff-command">{copy.oneStepStart}</Label>
                  <p class="mt-1 text-xs text-muted-foreground">
                    {copy.oneStepStartDescription}
                  </p>
                </div>
                <Button
                  size="sm"
                  onclick={() => void copyHandoff("start")}
                >
                  <Copy size={14} strokeWidth={1.8} aria-hidden="true" />
                  {copied === "start" ? copy.copied : copy.copyStartCommand}
                </Button>
              </div>
              <div
                id="handoff-command"
                class="min-h-20 overflow-x-auto rounded-xl border bg-muted/30 p-4 font-mono text-xs leading-5"
              >
                {agentUiState.handoff.start_command}
              </div>
            </div>

            {#if agentUiState.handoff.context.next_actions[0]}
              <div class="rounded-xl border bg-primary/5 p-4">
                <p class="text-xs font-medium text-muted-foreground">
                  {copy.currentNextAction}
                </p>
                <p class="mt-2 text-sm font-semibold">
                  {agentUiState.handoff.context.next_actions[0].description}
                </p>
                <p class="mt-2 overflow-x-auto font-mono text-xs text-muted-foreground">
                  {agentUiState.handoff.context.next_actions[0].action}
                </p>
              </div>
            {/if}

            <Separator />
            <div class="grid gap-6 xl:grid-cols-2">
              <div class="space-y-3">
                <Label>{copy.contextCommand}</Label>
                <div class="overflow-x-auto rounded-xl border bg-muted/30 p-4 font-mono text-xs leading-5">
                  {agentUiState.handoff.context_command}
                </div>
              </div>
              <div class="space-y-3">
                <div class="flex items-center justify-between gap-3">
                  <Label for="handoff-prompt">{copy.bootstrapPrompt}</Label>
                  <Button
                    variant="outline"
                    size="sm"
                    onclick={() => void copyHandoff("prompt")}
                  >
                    <Copy size={14} strokeWidth={1.8} aria-hidden="true" />
                    {copied === "prompt" ? copy.copied : copy.copyPrompt}
                  </Button>
                </div>
                <Textarea
                  id="handoff-prompt"
                  class="min-h-36 resize-y font-mono text-xs leading-5"
                  value={agentUiState.handoff.bootstrap_prompt}
                  readonly
                />
              </div>
            </div>
          </Card.Content>
        </Card.Root>
      {/if}

      <Card.Root class="shadow-none">
        <Card.Header>
          <div class="flex flex-wrap items-start justify-between gap-4">
            <div>
              <Card.Title>{copy.mcpIntegration}</Card.Title>
              <Card.Description class="mt-1.5">
                {copy.mcpIntegrationDescription}
              </Card.Description>
            </div>
            <Badge variant="outline">{copy.guardedToolSurface}</Badge>
          </div>
        </Card.Header>
        <Card.Content class="space-y-5">
          <div class="flex items-start gap-3 rounded-xl border bg-muted/20 p-4">
            <PlugZap
              size={18}
              strokeWidth={1.8}
              class="mt-0.5 shrink-0"
              aria-hidden="true"
            />
            <p class="text-xs leading-5 text-muted-foreground">
              {copy.mcpPrivacy}
            </p>
          </div>

          <Button
            variant="outline"
            class="min-h-11"
            disabled={!desktopRuntime || !activeWorkspace || busy}
            onclick={prepareMcpConfiguration}
          >
            <PlugZap
              size={16}
              strokeWidth={1.8}
              data-icon="inline-start"
              aria-hidden="true"
            />
            {busy ? copy.working : copy.prepareMcpConfiguration}
          </Button>

          {#if agentUiState.mcpConfiguration}
            <Separator />
            <div class="flex flex-wrap gap-2">
              <Badge variant="secondary">
                {agentUiState.mcpConfiguration.transport}
              </Badge>
              <Badge variant="secondary">
                MCP {agentUiState.mcpConfiguration.protocol_version}
              </Badge>
              <Badge variant="secondary">
                {agentUiState.mcpConfiguration.read_only_tools.length}
                {copy.readOnlyToolCount}
              </Badge>
              <Badge variant="secondary">
                {agentUiState.mcpConfiguration.guarded_write_tools.length}
                {copy.guardedWriteToolCount}
              </Badge>
            </div>

            {#if agentUiState.mcpConfiguration.registration_command}
              <div class="space-y-3">
                <div class="flex flex-wrap items-center justify-between gap-3">
                  <Label for="mcp-registration-command">
                    {copy.registrationCommand}
                  </Label>
                  <Button
                    variant="outline"
                    size="sm"
                    onclick={() => void copyMcpConfiguration("mcp-command")}
                  >
                    <Copy size={14} strokeWidth={1.8} aria-hidden="true" />
                    {copied === "mcp-command" ? copy.copied : copy.copyCommand}
                  </Button>
                </div>
                <div
                  id="mcp-registration-command"
                  class="overflow-x-auto rounded-xl border bg-muted/30 p-4 font-mono text-xs leading-5"
                >
                  {agentUiState.mcpConfiguration.registration_command}
                </div>
              </div>
            {/if}

            <div class="space-y-3">
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div>
                  <Label for="mcp-configuration-snippet">
                    {copy.configurationSnippet}
                  </Label>
                  <p class="mt-1 text-xs text-muted-foreground">
                    {copy.configurationTarget} · {agentUiState.mcpConfiguration.configuration_target}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onclick={() => void copyMcpConfiguration("mcp-config")}
                >
                  <Copy size={14} strokeWidth={1.8} aria-hidden="true" />
                  {copied === "mcp-config" ? copy.copied : copy.copyConfiguration}
                </Button>
              </div>
              <Textarea
                id="mcp-configuration-snippet"
                class="min-h-44 resize-y font-mono text-xs leading-5"
                value={agentUiState.mcpConfiguration.configuration_snippet}
                readonly
              />
              <p class="text-xs leading-5 text-muted-foreground">
                {copy.verifyWith}:
                <span class="font-mono">
                  {agentUiState.mcpConfiguration.verification_command}
                </span>
              </p>
            </div>
          {/if}
        </Card.Content>
      </Card.Root>
    </Tabs.Content>

    <Tabs.Content value="in-app" class="space-y-6 pt-4">
      <div class="rounded-xl border border-dashed bg-muted/10 p-4">
        <div class="flex items-start gap-3">
          <PlugZap size={18} strokeWidth={1.8} class="mt-0.5 shrink-0" aria-hidden="true" />
          <div>
            <p class="text-sm font-semibold">{copy.optionalRuntimeBridge}</p>
            <p class="mt-1 text-xs leading-5 text-muted-foreground">
              {copy.optionalRuntimeBridgeDescription}
            </p>
          </div>
        </div>
      </div>

      <div class="grid gap-6 xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)]">
        <Card.Root class="shadow-none">
          <Card.Header>
            <div class="flex flex-wrap items-start justify-between gap-4">
              <div>
                <Card.Title>{copy.conversation}</Card.Title>
                <Card.Description class="mt-1.5">
                  {currentSession ? copy.sessionContinues : copy.noConversationDescription}
                </Card.Description>
              </div>
              <div class="flex flex-wrap gap-2">
                <Badge variant="outline">{copy.readOnlyMode}</Badge>
                {#if currentSession}
                  <Badge variant="secondary">
                    {shortSessionId(currentSession.external_session_id)}
                  </Badge>
                {/if}
              </div>
            </div>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div
              class="min-h-56 max-h-96 space-y-4 overflow-y-auto rounded-xl border bg-muted/10 p-4"
              aria-live="polite"
            >
              {#each agentUiState.messages as message (message.id)}
                <div
                  class={[
                    "max-w-[88%] rounded-xl px-4 py-3 text-sm leading-6",
                    message.role === "user"
                      ? "ml-auto bg-primary text-primary-foreground"
                      : "border bg-background",
                  ]}
                >
                  <p class="whitespace-pre-wrap">{message.text}</p>
                </div>
              {:else}
                <div class="grid min-h-48 place-items-center text-center">
                  <div>
                    <Bot
                      size={22}
                      strokeWidth={1.8}
                      class="mx-auto"
                      aria-hidden="true"
                    />
                    <p class="mt-3 text-sm font-semibold">{copy.noConversation}</p>
                  </div>
                </div>
              {/each}
            </div>

            {#if agentUiState.lastTurn}
              <div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <span>{copy.agentResponseMetadata}</span>
                <Badge variant="outline">
                  {agentUiState.lastTurn.event_count} {copy.events}
                </Badge>
                {#each agentUiState.lastTurn.tool_activity as activity (activity)}
                  <Badge variant="outline">{activity}</Badge>
                {/each}
              </div>
            {/if}

            <div class="space-y-2">
              <Label for="agent-message">{copy.conversation}</Label>
              <Textarea
                id="agent-message"
                class="min-h-24 resize-y"
                placeholder={copy.messagePlaceholder}
                bind:value={agentUiState.prompt}
                disabled={!activeWorkspace || busy}
              />
            </div>
            <div class="flex items-start gap-3 rounded-xl border bg-muted/20 p-3">
              <Checkbox
                id="agent-provider-consent"
                bind:checked={agentUiState.confirmedProviderSend}
                class="mt-0.5"
              />
              <Label for="agent-provider-consent" class="text-xs leading-5 font-normal">
                {copy.providerConsent}
              </Label>
            </div>
            <div class="flex flex-wrap justify-between gap-2">
              <Button
                variant="outline"
                class="min-h-11"
                disabled={busy || turnRunning}
                onclick={beginNewAgentConversation}
              >
                <MessageSquarePlus
                  size={16}
                  strokeWidth={1.8}
                  data-icon="inline-start"
                  aria-hidden="true"
                />
                {copy.startNewConversation}
              </Button>
              {#if turnRunning}
                <Button
                  variant="destructive"
                  class="min-h-11"
                  disabled={cancellingTurn}
                  onclick={cancelTurn}
                >
                  <CircleOff
                    size={16}
                    strokeWidth={1.8}
                    data-icon="inline-start"
                    aria-hidden="true"
                  />
                  {cancellingTurn ? copy.cancellingAgentTurn : copy.cancelAgentTurn}
                </Button>
              {:else}
                <Button
                  class="min-h-11"
                  disabled={!desktopRuntime ||
                    !activeWorkspace ||
                    busy ||
                    !runtimeProbe?.available ||
                    !agentUiState.prompt.trim() ||
                    !agentUiState.confirmedProviderSend}
                  onclick={sendMessage}
                >
                  <Send size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                  {busy ? copy.working : copy.sendMessage}
                </Button>
              {/if}
            </div>
          </Card.Content>
        </Card.Root>

        <Card.Root class="shadow-none">
          <Card.Header>
            <Card.Title>{copy.localAgentRuntime}</Card.Title>
            <Card.Description>{copy.localAgentRuntimeDescription}</Card.Description>
          </Card.Header>
          <Card.Content class="space-y-3">
            {#each agentUiState.runtimeCatalog?.runtimes ?? [] as runtime (runtime.runtime)}
              <button
                type="button"
                class={[
                  "w-full rounded-xl border p-4 text-left transition-colors hover:bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                  agentUiState.runtime === runtime.runtime ? "border-primary bg-primary/5" : "",
                ]}
                aria-pressed={agentUiState.runtime === runtime.runtime}
                onclick={() => changeRuntime(runtime.runtime)}
              >
                <div class="flex items-center justify-between gap-3">
                  <p class="text-sm font-semibold">
                    {runtime.runtime === "codex" ? copy.codex : copy.claude}
                  </p>
                  <Badge variant={runtime.available ? "secondary" : "outline"}>
                    {runtime.available ? copy.runtimeAvailable : copy.runtimeUnavailable}
                  </Badge>
                </div>
                <p class="mt-2 truncate font-mono text-[11px] text-muted-foreground">
                  {runtime.version ?? runtime.executable ?? "—"}
                </p>
              </button>
            {/each}
            {#if runtimeProbe?.available}
              <Separator />
              <div class="flex flex-wrap gap-2">
                <Badge variant="outline">{copy.runtimeDetected}</Badge>
                <Badge variant="outline">{copy.sessionIdBinding}</Badge>
                <Badge variant="outline">{copy.hostManagedCapabilities}</Badge>
              </div>
              <p class="text-xs leading-5 text-muted-foreground">
                {copy.runtimeProbeEvidence}
              </p>
              <p class="break-all font-mono text-[10px] text-muted-foreground">
                {runtimeProbe.executable ?? "—"}
              </p>
            {:else}
              <Separator />
              <p class="text-xs leading-5 text-muted-foreground">
                {copy.noRuntimeFound}
              </p>
            {/if}
          </Card.Content>
        </Card.Root>
      </div>
    </Tabs.Content>
  </Tabs.Root>

  <div class="grid gap-6 xl:grid-cols-2">
    <Card.Root class="shadow-none">
      <Card.Header>
        <Card.Title>{copy.bodyFreeContext}</Card.Title>
        <Card.Description>{copy.workspaceScopeDescription}</Card.Description>
      </Card.Header>
      <Card.Content class="space-y-4">
        <div class="flex flex-wrap gap-2">
          <Button
            variant="outline"
            class="min-h-10"
            disabled={!desktopRuntime || busy}
            onclick={loadContext}
          >
            {copy.loadContext}
          </Button>
          <Button
            variant="outline"
            class="min-h-10"
            disabled={!desktopRuntime || busy}
            onclick={loadCapabilities}
          >
            {copy.loadContract}
          </Button>
        </div>
        {#if agentUiState.context}
          <div class="space-y-2">
            {#each agentUiState.context.blockers as blocker (blocker.code)}
              <div class="rounded-xl border p-3">
                <p class="text-xs font-semibold">{blocker.code}</p>
                <p class="mt-1 text-xs leading-5 text-muted-foreground">
                  {blocker.description}
                </p>
              </div>
            {:else}
              <p class="text-xs text-muted-foreground">{copy.noAgentBlockers}</p>
            {/each}
          </div>
          {#if agentUiState.context.next_actions.length}
            <Separator />
            <div class="space-y-2">
              <p class="text-xs font-semibold">{copy.agentNextActions}</p>
              {#each agentUiState.context.next_actions as action, index (`${action.action}-${index}`)}
                <button
                  type="button"
                  class="w-full rounded-xl border p-3 text-left transition-colors hover:border-primary/35 hover:bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  onclick={() =>
                    void onNavigate({
                      ...routeForAgentAction(action.action),
                      jobId: selectedJobId || undefined,
                    })}
                >
                  <span class="text-xs font-semibold">{action.action}</span>
                  <span class="mt-1 block text-xs leading-5 text-muted-foreground">
                    {action.description}
                  </span>
                  <span class="mt-2 block text-[11px] font-medium text-primary">
                    {copy.openRelatedStep}
                  </span>
                </button>
              {/each}
            </div>
          {/if}
        {/if}
        {#if agentUiState.capabilities}
          <div class="rounded-xl border bg-muted/20 p-3">
            <div class="flex items-center justify-between gap-3 text-xs">
              <span class="text-muted-foreground">{copy.protocol}</span>
              <span class="font-semibold">{agentUiState.capabilities.protocol}</span>
            </div>
            <div class="mt-2 flex items-center justify-between gap-3 text-xs">
              <span class="text-muted-foreground">{copy.capabilities}</span>
              <span class="font-semibold">
                {agentUiState.capabilities.capabilities.length}
              </span>
            </div>
          </div>
        {/if}
      </Card.Content>
    </Card.Root>

    <Card.Root class="shadow-none">
      <Card.Header>
        <div class="flex items-start justify-between gap-4">
          <div>
            <Card.Title>{copy.workspaceBridge}</Card.Title>
            <Card.Description class="mt-1.5">{copy.exportAgentPack}</Card.Description>
          </div>
          <FileOutput size={18} strokeWidth={1.8} aria-hidden="true" />
        </div>
      </Card.Header>
      <Card.Content class="space-y-4">
        <div class="grid gap-4 sm:grid-cols-[160px_minmax(0,1fr)]">
          <div class="space-y-2">
            <Label for="agent-host-pack">{copy.agentHost}</Label>
            <select
              id="agent-host-pack"
              class="min-h-11 w-full rounded-lg border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
              bind:value={agentUiState.host}
              onchange={() => (agentUiState.handoff = null)}
            >
              <option value="codex">{copy.codex}</option>
              <option value="claude">{copy.claude}</option>
              <option value="generic">{copy.generic}</option>
            </select>
          </div>
          <div class="space-y-2">
            <Label for="agent-destination">{copy.exportDestination}</Label>
            <div class="flex gap-2">
              <Input
                id="agent-destination"
                bind:value={agentUiState.destination}
                placeholder="/Users/me/canisend-agent-pack"
              />
              <Button
                variant="outline"
                size="icon"
                aria-label={copy.chooseDirectory}
                disabled={!desktopRuntime || busy}
                onclick={chooseDestination}
              >
                <FolderOpen size={16} strokeWidth={1.8} aria-hidden="true" />
              </Button>
            </div>
          </div>
        </div>
        <Button
          class="min-h-11"
          disabled={!desktopRuntime || busy || !agentUiState.destination}
          onclick={exportPack}
        >
          {copy.exportAgentPack}
        </Button>
        {#if agentUiState.exported}
          <div class="rounded-xl border border-[var(--success)]/35 bg-[var(--success)]/8 p-4">
            <div class="flex items-center justify-between gap-4">
              <p class="text-sm font-semibold">
                {hostLabel(agentUiState.exported.manifest.host)}
              </p>
              <Badge variant="outline">
                {agentUiState.exported.manifest.files.length} {copy.exportedFiles}
              </Badge>
            </div>
            <p class="mt-2 truncate font-mono text-xs text-muted-foreground">
              {agentUiState.exported.manifest_path}
            </p>
          </div>
        {/if}
      </Card.Content>
    </Card.Root>
  </div>

  {#if agentUiState.formError}
    <p class="text-sm text-destructive" role="alert">{agentUiState.formError}</p>
  {/if}
</section>
