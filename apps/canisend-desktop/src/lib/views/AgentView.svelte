<script lang="ts">
  import {
    ArrowRight,
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
    Sparkles,
    Trash2,
  } from "@lucide/svelte";
  import { onMount } from "svelte";

  import {
    agentUiState,
    appendAgentMessage,
    beginNewAgentConversation,
    scopeAgentUiState,
    switchAgentConversationScope,
  } from "$lib/agent-state.svelte";
  import ActionMenu from "$lib/components/patterns/ActionMenu.svelte";
  import ContextHelp from "$lib/components/patterns/ContextHelp.svelte";
  import * as Page from "$lib/components/patterns/page/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import * as Accordion from "$lib/components/ui/accordion/index.js";
  import * as Alert from "$lib/components/ui/alert/index.js";
  import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import * as Empty from "$lib/components/ui/empty/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import * as Item from "$lib/components/ui/item/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import * as NativeSelect from "$lib/components/ui/native-select/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import LoadingPanel from "$lib/components/patterns/LoadingPanel.svelte";
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";
  import {
    chooseExportDirectory,
    type AgentAssistanceReadModel,
    type AgentCapabilitiesReadModel,
    type AgentContextReadModel,
    type AgentHandoffReadModel,
    type AgentMcpConfigurationReadModel,
    type AgentPackExportReadModel,
    type AgentRuntimeCatalog,
    type AgentRuntimeKind,
    type AgentSkillsInstallReadModel,
    type AgentSkillsStatusReadModel,
    type AgentSkillsUninstallReadModel,
    type AgentTurnResult,
    type JobRecord,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";
  import {
    routeForApplicationSection,
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
    onLoadAssistance: (
      jobId: string,
    ) => Promise<AgentAssistanceReadModel | null>;
    onPrepareHandoff: (
      host: AgentHost,
      jobId?: string,
    ) => Promise<AgentHandoffReadModel | null>;
    onInstallSkills: (
      host: AgentHost,
    ) => Promise<AgentSkillsInstallReadModel | null>;
    onLoadSkills: (
      host: AgentHost,
    ) => Promise<AgentSkillsStatusReadModel | null>;
    onUninstallSkills: (
      host: AgentHost,
    ) => Promise<AgentSkillsUninstallReadModel | null>;
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
    onLoadAssistance,
    onPrepareHandoff,
    onLoadSkills,
    onInstallSkills,
    onUninstallSkills,
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
  let assistanceLoading = $state(false);
  let skillsLoading = $state(false);
  let uninstallSkillsOpen = $state(false);
  let observedGlobalScope = $state("");
  let observedAssistanceScope = $state("");
  let observedSkillsScope = $state("");

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
  const skillManagementBlocked = $derived(
    agentUiState.skillsStatus?.state === "user-modified" ||
      agentUiState.skillsStatus?.state === "unmanaged",
  );
  const skillsCanRemove = $derived(
    agentUiState.skillsStatus !== null &&
      agentUiState.skillsStatus.state !== "not-installed" &&
      !skillManagementBlocked,
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
    const assistanceScope = `${activeWorkspace?.path ?? ""}:${agentUiState.selectedJobId}`;
    if (assistanceScope !== observedAssistanceScope) {
      observedAssistanceScope = assistanceScope;
      if (activeWorkspace && agentUiState.selectedJobId) {
        void loadAssistance();
      }
    }
    const skillsScope = `${activeWorkspace?.path ?? ""}:${agentUiState.host}`;
    if (skillsScope !== observedSkillsScope) {
      observedSkillsScope = skillsScope;
      if (activeWorkspace) void loadSkills();
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
    agentUiState.assistance = null;
    agentUiState.handoff = null;
    agentUiState.skillsInstallation = null;
    agentUiState.mcpConfiguration = null;
    agentUiState.runtimeCatalog = await onLoadRuntimes(jobId || undefined);
    if (jobId) await loadAssistance();
  }

  function changeHost(host: AgentHost): void {
    agentUiState.host = host;
    agentUiState.handoff = null;
    agentUiState.skillsInstallation = null;
    agentUiState.skillsStatus = null;
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
    agentUiState.skillsStatus = await onLoadSkills(agentUiState.host);
    const handoff = await onPrepareHandoff(
      agentUiState.host,
      agentUiState.selectedJobId || undefined,
    );
    if (!handoff) return;
    agentUiState.handoff = handoff;
    agentUiState.context = handoff.context;
    agentUiState.assistance = handoff.assistance;
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

  async function loadSkills(): Promise<void> {
    if (!activeWorkspace || skillsLoading) return;
    const host = agentUiState.host;
    skillsLoading = true;
    try {
      const status = await onLoadSkills(host);
      if (agentUiState.host === host) agentUiState.skillsStatus = status;
    } finally {
      skillsLoading = false;
    }
  }

  async function installOrUpdateSkills(): Promise<void> {
    if (!activeWorkspace) return;
    agentUiState.formError = null;
    const installation = await onInstallSkills(agentUiState.host);
    if (!installation) return;
    agentUiState.skillsInstallation = installation;
    await loadSkills();
  }

  function skillsPrimaryLabel(): string {
    if (skillsLoading) return copy.loading;
    const state = agentUiState.skillsStatus?.state;
    if (!state || state === "up-to-date") return copy.checkSkills;
    if (state === "not-installed") return copy.installSkills;
    return copy.updateOrRepairSkills;
  }

  async function runSkillsPrimaryAction(): Promise<void> {
    const state = agentUiState.skillsStatus?.state;
    if (!state || state === "up-to-date") {
      await loadSkills();
      return;
    }
    await installOrUpdateSkills();
  }

  async function uninstallSkills(): Promise<void> {
    if (!activeWorkspace) return;
    agentUiState.formError = null;
    const removed = await onUninstallSkills(agentUiState.host);
    if (!removed) return;
    uninstallSkillsOpen = false;
    agentUiState.skillsInstallation = null;
    await loadSkills();
  }

  async function loadContext(): Promise<void> {
    agentUiState.context = await onLoadContext(
      agentUiState.selectedJobId || undefined,
    );
  }

  async function refreshAgentContext(): Promise<void> {
    await loadContext();
    await loadCapabilities();
  }

  async function loadAssistance(): Promise<void> {
    const jobId = agentUiState.selectedJobId;
    if (!activeWorkspace || !jobId || assistanceLoading) return;
    assistanceLoading = true;
    try {
      const assistance = await onLoadAssistance(jobId);
      if (agentUiState.selectedJobId !== jobId) return;
      agentUiState.assistance = assistance;
      if (assistance) agentUiState.context = assistance.context;
    } finally {
      assistanceLoading = false;
    }
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

  function skillsStatusLabel(
    value: AgentSkillsStatusReadModel["state"],
  ): string {
    if (value === "not-installed") return copy.skillsNotInstalled;
    if (value === "up-to-date") return copy.skillsUpToDate;
    if (value === "update-available") return copy.skillsUpdateAvailable;
    if (value === "incomplete") return copy.skillsIncomplete;
    if (value === "user-modified") return copy.skillsUserModified;
    return copy.skillsUnmanaged;
  }

  function skillTitle(id: string): string {
    if (id === "canisend-application") return copy.skillApplication;
    if (id === "canisend-job-intake") return copy.skillJobIntake;
    if (id === "canisend-application-materials")
      return copy.skillApplicationMaterials;
    if (id === "canisend-application-review") return copy.skillApplicationReview;
    return id;
  }

  function skillDescription(id: string): string {
    if (id === "canisend-application") return copy.skillApplicationDescription;
    if (id === "canisend-job-intake") return copy.skillJobIntakeDescription;
    if (id === "canisend-application-materials")
      return copy.skillApplicationMaterialsDescription;
    if (id === "canisend-application-review")
      return copy.skillApplicationReviewDescription;
    return copy.skillsManagerDescription;
  }

  function shortSessionId(value: string): string {
    return value.length > 18
      ? `${value.slice(0, 8)}…${value.slice(-6)}`
      : value;
  }

  function proposalLabel(
    value: AgentAssistanceReadModel["proposal_targets"][number]["kind"],
  ): string {
    if (value === "criteria") return copy.criteria;
    if (value === "evidence") return copy.evidence;
    if (value === "matches") return copy.matches;
    if (value === "plan") return copy.plan;
    return copy.applicationWorkspaceSectionLabel.materials;
  }

  function proposalStateLabel(
    value: AgentAssistanceReadModel["proposal_targets"][number]["state"],
  ): string {
    if (value === "blocked") return copy.proposalBlocked;
    if (value === "ready") return copy.proposalReady;
    if (value === "proposed") return copy.proposalProposed;
    if (value === "current") return copy.proposalCurrent;
    return copy.proposalStale;
  }
</script>

<Page.Root>
  <Page.Header
    eyebrow={copy.agent}
    title={copy.agentTitle}
    description={copy.agentDescription}
  />

  <Item.Group class="grid gap-3 md:grid-cols-3">
    <Item.Root variant="muted" class="items-start p-[var(--density-panel-padding)]">
      <Item.Content>
        <Item.Title class="text-xs text-muted-foreground">
          1 · {copy.controlPlane}
        </Item.Title>
        <Item.Description
          class="line-clamp-1 text-sm font-semibold text-foreground"
          title={activeWorkspace?.path ?? copy.noWorkspace}
        >
          {activeWorkspace?.path ?? copy.noWorkspace}
        </Item.Description>
      </Item.Content>
    </Item.Root>
    <Item.Root variant="muted" class="items-start p-[var(--density-panel-padding)]">
      <Item.Content>
        <Item.Title class="text-xs text-muted-foreground">
          2 · {copy.reasoningPlane}
        </Item.Title>
        <Item.Description class="text-sm font-semibold text-foreground">
          {hostLabel(agentUiState.host)}
        </Item.Description>
      </Item.Content>
    </Item.Root>
    <Item.Root variant="muted" class="items-start p-[var(--density-panel-padding)]">
      <Item.Content>
        <Item.Title class="text-xs text-muted-foreground">
          3 · {copy.workspaceScope}
        </Item.Title>
        <Item.Description
          class="line-clamp-1 text-sm font-semibold text-foreground"
          title={selectedJob
            ? `${selectedJob.title} — ${selectedJob.institution}`
            : copy.wholeWorkspace}
        >
          {selectedJob
            ? `${selectedJob.title} — ${selectedJob.institution}`
            : copy.wholeWorkspace}
        </Item.Description>
      </Item.Content>
    </Item.Root>
  </Item.Group>

  <Card.Root class="border-primary/25">
    <Card.Header>
      <div class="flex flex-wrap items-start justify-between gap-[var(--density-section-gap)]">
        <div class="max-w-3xl">
          <div class="mb-2 flex flex-wrap items-center gap-2">
            <Badge variant="secondary">{copy.contextualAssistanceLabel}</Badge>
            <Badge variant="outline">{copy.bodyFree}</Badge>
          </div>
          <div class="flex min-w-0 items-center gap-1.5">
            <Card.Title>{copy.contextualAssistance}</Card.Title>
            <ContextHelp content={copy.contextualAssistanceDescription} />
          </div>
        </div>
        {#if activeWorkspace && agentUiState.selectedJobId}
          <Button
            variant="outline"
            class="min-h-9"
            disabled={!desktopRuntime || busy || assistanceLoading}
            onclick={loadAssistance}
          >
            <RefreshCw
              size={16}
              strokeWidth={1.8}
              class={assistanceLoading ? "animate-spin motion-reduce:animate-none" : ""}
              data-icon="inline-start"
              aria-hidden="true"
            />
            {assistanceLoading ? copy.loading : copy.refreshGuidance}
          </Button>
        {/if}
      </div>
    </Card.Header>
    <Card.Content class="space-y-[var(--density-section-gap)]">
      {#if !agentUiState.selectedJobId}
        <Empty.Root class="min-h-20 border bg-muted/10">
          <Empty.Header><Empty.Description>{copy.selectApplicationForGuidance}</Empty.Description></Empty.Header>
        </Empty.Root>
      {:else if assistanceLoading && !agentUiState.assistance}
        <LoadingPanel label={copy.loadingGuidance} class="min-h-20" />
      {:else if agentUiState.assistance}
        <div class="grid gap-[var(--density-section-gap)] xl:grid-cols-[minmax(0,1.15fr)_minmax(300px,0.85fr)]">
          <div class="rounded-lg border bg-primary/5 p-5">
            <div class="flex items-start gap-3">
              <Sparkles
                size={19}
                strokeWidth={1.8}
                class="mt-0.5 shrink-0 text-primary"
                aria-hidden="true"
              />
              <div class="min-w-0">
                <p class="text-xs font-medium text-muted-foreground">
                  {copy.smallestApplicableSkill}
                </p>
                <p class="mt-1 break-all font-mono text-sm font-semibold">
                  {agentUiState.assistance.recommendation.skill_id}
                </p>
                <p class="mt-2 text-xs leading-5 text-muted-foreground">
                  {agentUiState.assistance.recommendation.reason}
                </p>
                <div class="mt-3 flex flex-wrap gap-2">
                  <Badge variant="outline">
                    {copy.applicationWorkspaceSectionLabel[
                      agentUiState.assistance.recommendation.section
                    ]}
                  </Badge>
                  <Badge variant="outline">
                    {copy.stateInCanisend}
                  </Badge>
                </div>
              </div>
            </div>
          </div>

          <div class="rounded-lg border p-5">
            <p class="text-xs font-medium text-muted-foreground">
              {copy.contentRelationshipGraph}
            </p>
            <p class="mt-2 text-2xl font-semibold tracking-tight">
              {agentUiState.assistance.content.entries.length}
              <span class="text-sm font-normal text-muted-foreground">
                / {agentUiState.assistance.content.total_entries}
              </span>
            </p>
            <p class="mt-2 text-xs leading-5 text-muted-foreground">
              {copy.contentRelationshipGraphDescription}
            </p>
            {#if agentUiState.assistance.content.truncated}
              <Badge variant="outline" class="mt-3">{copy.truncatedMetadata}</Badge>
            {/if}
          </div>
        </div>

        {#if agentUiState.assistance.recommendation.next_action}
          <div class="rounded-lg border p-[var(--density-panel-padding)]">
            <p class="text-xs font-medium text-muted-foreground">
              {copy.exactRecommendedAction}
            </p>
            <p class="mt-2 text-sm font-semibold">
              {agentUiState.assistance.recommendation.next_action.description}
            </p>
            <p class="mt-2 overflow-x-auto font-mono text-xs leading-5 text-muted-foreground">
              {agentUiState.assistance.recommendation.next_action.action}
            </p>
            <Button
              variant="outline"
              class="mt-[var(--density-section-gap)] min-h-9"
              onclick={() =>
                void onNavigate({
                  ...routeForAgentAction(
                    agentUiState.assistance?.recommendation.next_action?.action ?? "",
                  ),
                  jobId: agentUiState.selectedJobId,
                })}
            >
              {copy.openRelatedStep}
              <ArrowRight size={16} strokeWidth={1.8} data-icon="inline-end" aria-hidden="true" />
            </Button>
          </div>
        {/if}

        <div>
          <div class="mb-3">
            <h2 class="text-sm font-semibold">{copy.revisionBoundProposals}</h2>
            <p class="mt-1 text-xs leading-5 text-muted-foreground">
              {copy.revisionBoundProposalsDescription}
            </p>
          </div>
          <div class="grid gap-3 lg:grid-cols-2 xl:grid-cols-5">
            {#each agentUiState.assistance.proposal_targets as target (target.kind)}
              <div class="flex min-w-0 flex-col rounded-lg border p-[var(--density-panel-padding)]">
                <div class="flex flex-wrap items-start justify-between gap-2">
                  <p class="text-sm font-semibold">{proposalLabel(target.kind)}</p>
                  <Badge variant={target.state === "current" ? "secondary" : "outline"}>
                    {proposalStateLabel(target.state)}
                  </Badge>
                </div>
                <p class="mt-3 text-xs leading-5 text-muted-foreground">
                  {target.intended_mutation}
                </p>
                <div class="mt-3 flex flex-wrap gap-2 text-[11px] text-muted-foreground">
                  <span>{target.current_artifacts.length} {copy.currentArtifacts}</span>
                  <span>·</span>
                  <span>{target.upstream_artifacts.length} {copy.upstreamArtifacts}</span>
                </div>
                <Accordion.Root type="single" class="mt-3 text-xs">
                  <Accordion.Item value="boundary">
                    <Accordion.Trigger level={2} class="py-2 text-xs text-primary">
                      {copy.validationAndBoundary}
                    </Accordion.Trigger>
                    <Accordion.Content class="pb-2">
                      <ul class="list-disc space-y-1.5 pl-4 leading-5 text-muted-foreground">
                        {#each target.validation_rules as rule (rule)}
                          <li>{rule}</li>
                        {/each}
                      </ul>
                      <p class="mt-2 font-mono text-[11px] text-muted-foreground">
                        {target.commit_boundary}
                      </p>
                    </Accordion.Content>
                  </Accordion.Item>
                </Accordion.Root>
                <Button
                  variant="ghost"
                  class="mt-auto min-h-9 justify-start px-0 pt-[var(--density-section-gap)]"
                  onclick={() =>
                    void onNavigate(
                      routeForApplicationSection(
                        target.section,
                        agentUiState.selectedJobId,
                      ),
                    )}
                >
                  {copy.openRelatedStep}
                  <ArrowRight size={15} strokeWidth={1.8} data-icon="inline-end" aria-hidden="true" />
                </Button>
              </div>
            {/each}
          </div>
        </div>

        {#if agentUiState.assistance.content.entries.length}
          <Accordion.Root type="single">
            <Accordion.Item value="content-identities" class="rounded-lg border px-4">
              <Accordion.Trigger level={2}>{copy.inspectContentIdentities}</Accordion.Trigger>
              <Accordion.Content class="pb-[var(--density-section-gap)]">
                <div class="grid gap-2 md:grid-cols-2 xl:grid-cols-3">
              {#each agentUiState.assistance.content.entries.slice(0, 6) as entry (entry.artifact.id)}
                <div class="rounded-lg border bg-muted/15 p-3">
                  <div class="flex items-start justify-between gap-2">
                    <p class="text-xs font-semibold">{entry.title}</p>
                    <Badge variant="outline">{entry.status}</Badge>
                  </div>
                  <p class="mt-2 break-all font-mono text-[10px] text-muted-foreground">
                    {entry.artifact.kind} · {entry.artifact.id} · r{entry.artifact.revision}
                  </p>
                  <p class="mt-2 text-[11px] leading-5 text-muted-foreground">
                    {entry.provenance.actor} · {entry.provenance.reason}
                  </p>
                  <p class="mt-1 text-[11px] text-muted-foreground">
                    {entry.relationships.length} {copy.relationships}
                  </p>
                </div>
              {/each}
                </div>
              </Accordion.Content>
            </Accordion.Item>
          </Accordion.Root>
        {/if}
      {:else}
        <Empty.Root class="min-h-20 border bg-muted/10">
          <Empty.Header><Empty.Description>{copy.guidanceUnavailable}</Empty.Description></Empty.Header>
        </Empty.Root>
      {/if}
    </Card.Content>
  </Card.Root>

  <Tabs.Root bind:value={agentUiState.integrationMode}>
    <Tabs.List class="responsive-tabs max-w-xl" data-columns="2">
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
        "scroll-mt-44 space-y-[var(--density-section-gap)] pt-[var(--density-section-gap)]",
        focus === "agent-handoff" || focus === "agent-task"
          ? "rounded-lg ring-2 ring-primary/25"
          : "",
      ]}
    >
      <div class="grid gap-[var(--density-section-gap)] xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)]">
        <Card.Root>
          <Card.Header>
            <div class="flex flex-wrap items-start justify-between gap-[var(--density-section-gap)]">
              <div class="flex min-w-0 items-center gap-1.5">
                <Card.Title>{copy.externalHost}</Card.Title>
                <ContextHelp content={copy.externalHostDescription} />
              </div>
              <Badge>{copy.recommended}</Badge>
            </div>
          </Card.Header>
          <Card.Content class="space-y-[var(--density-section-gap)]">
            <div class="grid gap-3 sm:grid-cols-2">
              {#each ["codex", "claude"] as host}
                <Button
                  variant="outline"
                  class={[
                    "h-auto min-h-9 w-full flex-col items-stretch gap-2 whitespace-normal p-[var(--density-panel-padding)] text-left",
                    agentUiState.host === host ? "border-primary bg-primary/5" : "",
                  ]}
                  aria-pressed={agentUiState.host === host}
                  onclick={() => changeHost(host as AgentHost)}
                >
                  <div class="flex flex-wrap items-center justify-between gap-3">
                    <p class="text-sm font-semibold">
                      {host === "codex" ? copy.codex : copy.claude}
                    </p>
                    {#if agentUiState.runtimeCatalog?.runtimes.find((item) => item.runtime === host)?.available}
                      <span class="inline-flex items-center gap-1.5 text-xs font-medium text-success">
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
                </Button>
              {/each}
            </div>

            <div class="space-y-2">
              <Label for="handoff-job">{copy.selectApplication}</Label>
              <NativeSelect.Root
                id="handoff-job"
                size="desktop"
                class="w-full"
                value={agentUiState.selectedJobId}
                disabled={!activeWorkspace || busy}
                onchange={(event) => void changeScope(event.currentTarget.value)}
              >
                <NativeSelect.Option value="">{copy.wholeWorkspace}</NativeSelect.Option>
                {#each jobs as job (job.id)}
                  <NativeSelect.Option value={job.id}>{job.title} — {job.institution}</NativeSelect.Option>
                {/each}
              </NativeSelect.Root>
            </div>

            <Alert.Root variant="success">
              <ShieldCheck
                size={18}
                strokeWidth={1.8}
                aria-hidden="true"
              />
              <Alert.Description>{copy.handoffPrivacy}</Alert.Description>
            </Alert.Root>

            {#if activeWorkspace}
              <Button
                class="min-h-9 w-full sm:w-auto"
                disabled={!desktopRuntime || busy}
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
            {/if}
          </Card.Content>
        </Card.Root>

        <Card.Root>
          <Card.Header>
            <div class="flex min-w-0 items-center gap-1.5">
              <Card.Title>{copy.stateInCanisend}</Card.Title>
              <ContextHelp content={copy.sessionInHost} />
            </div>
          </Card.Header>
          <Card.Content class="space-y-[var(--density-section-gap)]">
            <Alert.Root variant="success">
              <Database size={18} strokeWidth={1.8} aria-hidden="true" />
              <Alert.Title>{copy.controlPlane}</Alert.Title>
              <Alert.Description>{copy.controlPlaneDescription}</Alert.Description>
            </Alert.Root>
            <div class="rounded-lg border p-[var(--density-panel-padding)]">
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

      <Card.Root>
        <Card.Header>
          <div class="flex flex-wrap items-start justify-between gap-[var(--density-section-gap)]">
            <div class="max-w-3xl">
              <div class="mb-2 flex flex-wrap items-center gap-2">
                <Badge variant="secondary">{hostLabel(agentUiState.host)}</Badge>
                {#if agentUiState.skillsStatus}
                  <Badge
                    variant={skillManagementBlocked ? "destructive" : "outline"}
                    aria-live="polite"
                  >
                    {skillsStatusLabel(agentUiState.skillsStatus.state)}
                  </Badge>
                {/if}
              </div>
              <div class="flex min-w-0 items-center gap-1.5">
                <Card.Title>{copy.skillsManager}</Card.Title>
                <ContextHelp content={copy.skillsManagerDescription} />
              </div>
            </div>
            {#if activeWorkspace}
              <div class="flex flex-wrap items-center gap-2">
                <Button
                  class="min-h-9"
                  disabled={!desktopRuntime ||
                    busy ||
                    skillsLoading ||
                    skillManagementBlocked}
                  onclick={runSkillsPrimaryAction}
                >
                  <ShieldCheck
                    size={16}
                    strokeWidth={1.8}
                    data-icon="inline-start"
                    aria-hidden="true"
                  />
                  {skillsPrimaryLabel()}
                </Button>
                <ActionMenu label={copy.moreActions} disabled={busy || skillsLoading}>
                  <DropdownMenu.Item disabled={!desktopRuntime} onclick={loadSkills}>
                    <RefreshCw size={16} strokeWidth={1.8} aria-hidden="true" />
                    {copy.checkSkills}
                  </DropdownMenu.Item>
                  <DropdownMenu.Separator />
                  <DropdownMenu.Item
                    variant="destructive"
                    disabled={!desktopRuntime || !skillsCanRemove}
                    onclick={() => (uninstallSkillsOpen = true)}
                  >
                    <Trash2 size={16} strokeWidth={1.8} aria-hidden="true" />
                    {copy.removeSkills}
                  </DropdownMenu.Item>
                </ActionMenu>
              </div>
            {/if}
          </div>
        </Card.Header>
        <Card.Content class="space-y-[var(--density-section-gap)]">
          {#if skillsLoading && !agentUiState.skillsStatus}
            <LoadingPanel label={copy.loadingSkills} class="min-h-20" />
          {:else if agentUiState.skillsStatus}
            {#if skillManagementBlocked}
              <Alert.Root variant="destructive">
                <Alert.Title>
                  {agentUiState.skillsStatus.state === "user-modified"
                    ? copy.skillsModifiedWarning
                    : copy.skillsUnmanagedWarning}
                </Alert.Title>
                <Alert.Description>{copy.skillsPreservedDescription}</Alert.Description>
              </Alert.Root>
            {/if}

            <div class="grid gap-3 md:grid-cols-2">
              {#each agentUiState.skillsStatus.skills as skill (skill.id)}
                <article class="rounded-lg border bg-muted/10 p-[var(--density-panel-padding)]">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <h3 class="text-sm font-semibold">{skillTitle(skill.id)}</h3>
                      <p class="mt-1 break-all font-mono text-[10px] text-muted-foreground">
                        {skill.id}
                      </p>
                    </div>
                    <Badge
                      variant={skill.state === "user-modified" ||
                      skill.state === "unmanaged"
                        ? "destructive"
                        : "outline"}
                    >
                      {skillsStatusLabel(skill.state)}
                    </Badge>
                  </div>
                  <p class="mt-3 text-xs leading-5 text-muted-foreground">
                    {skillDescription(skill.id)}
                  </p>
                  <div class="mt-3 flex flex-wrap gap-2 text-[11px] text-muted-foreground">
                    <span>
                      {skill.installed_file_count}/{skill.file_count} {copy.managedFiles}
                    </span>
                    <span aria-hidden="true">·</span>
                    <span>{skill.resource_version}</span>
                  </div>
                </article>
              {/each}
            </div>

            <div class="grid gap-3 rounded-lg border bg-muted/15 p-[var(--density-panel-padding)] text-xs sm:grid-cols-2">
              <div>
                <p class="font-medium text-muted-foreground">{copy.skillsInstallLocation}</p>
                <p class="mt-1 break-all font-mono">{agentUiState.skillsStatus.directory}</p>
              </div>
              <div>
                <p class="font-medium text-muted-foreground">{copy.managedManifest}</p>
                <p class="mt-1 break-all font-mono">{agentUiState.skillsStatus.manifest_path}</p>
              </div>
            </div>
          {:else}
            <Empty.Root class="min-h-20 border bg-muted/10">
              <Empty.Header><Empty.Description>{copy.skillsStatusUnavailable}</Empty.Description></Empty.Header>
            </Empty.Root>
          {/if}
        </Card.Content>
      </Card.Root>

      {#if agentUiState.handoff}
        <Card.Root class="border-primary/30">
          <Card.Header>
            <div class="flex flex-wrap items-start justify-between gap-[var(--density-section-gap)]">
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
          <Card.Content class="space-y-[var(--density-section-gap)]">
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
                class="min-h-20 overflow-x-auto rounded-lg border bg-muted/30 p-[var(--density-panel-padding)] font-mono text-xs leading-5"
              >
                {agentUiState.handoff.start_command}
              </div>
            </div>

            {#if agentUiState.handoff.context.next_actions[0]}
              <div class="rounded-lg border bg-primary/5 p-[var(--density-panel-padding)]">
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
            <div class="grid gap-[var(--density-section-gap)] xl:grid-cols-2">
              <div class="space-y-3">
                <Label>
                  {agentUiState.handoff.assistance_command
                    ? copy.assistanceCommand
                    : copy.contextCommand}
                </Label>
                <div class="overflow-x-auto rounded-lg border bg-muted/30 p-[var(--density-panel-padding)] font-mono text-xs leading-5">
                  {agentUiState.handoff.assistance_command ??
                    agentUiState.handoff.context_command}
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

      <Card.Root>
        <Card.Header>
          <div class="flex flex-wrap items-start justify-between gap-[var(--density-section-gap)]">
            <div class="flex min-w-0 items-center gap-1.5">
              <Card.Title>{copy.mcpIntegration}</Card.Title>
              <ContextHelp content={copy.mcpIntegrationDescription} />
            </div>
            <Badge variant="outline">{copy.guardedToolSurface}</Badge>
          </div>
        </Card.Header>
        <Card.Content class="space-y-[var(--density-section-gap)]">
          <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-[var(--density-panel-padding)]">
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

          {#if activeWorkspace}
            <Button
              variant="outline"
              class="min-h-9"
              disabled={!desktopRuntime || busy}
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
          {/if}

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
                  class="overflow-x-auto rounded-lg border bg-muted/30 p-[var(--density-panel-padding)] font-mono text-xs leading-5"
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
                class="min-h-32 resize-y font-mono text-xs leading-5"
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

    <Tabs.Content value="in-app" class="space-y-[var(--density-section-gap)] pt-[var(--density-section-gap)]">
      <Alert.Root variant="info">
        <PlugZap size={18} strokeWidth={1.8} aria-hidden="true" />
        <Alert.Title>{copy.optionalRuntimeBridge}</Alert.Title>
        <Alert.Description>{copy.optionalRuntimeBridgeDescription}</Alert.Description>
      </Alert.Root>

      <div class="grid gap-[var(--density-section-gap)] xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)]">
        <Card.Root>
          <Card.Header>
            <div class="flex flex-wrap items-start justify-between gap-[var(--density-section-gap)]">
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
          <Card.Content class="space-y-[var(--density-section-gap)]">
            <div
              class="min-h-40 max-h-80 space-y-3 overflow-y-auto rounded-lg border bg-muted/10 p-3"
              aria-live="polite"
            >
              {#each agentUiState.messages as message (message.id)}
                <div
                  class={[
                    "max-w-[88%] rounded-lg px-4 py-3 text-sm leading-6",
                    message.role === "user"
                      ? "ml-auto bg-primary text-primary-foreground"
                      : "border bg-background",
                  ]}
                >
                  <p class="whitespace-pre-wrap">{message.text}</p>
                </div>
              {:else}
                <Empty.Root class="min-h-32 border-0">
                  <Empty.Header>
                    <Empty.Media variant="icon"><Bot size={22} strokeWidth={1.8} aria-hidden="true" /></Empty.Media>
                    <Empty.Title>{copy.noConversation}</Empty.Title>
                  </Empty.Header>
                </Empty.Root>
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
                class="min-h-20 resize-y"
                placeholder={copy.messagePlaceholder}
                bind:value={agentUiState.prompt}
                disabled={!activeWorkspace || busy}
              />
            </div>
            <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
              <Checkbox
                id="agent-provider-consent"
                aria-label={copy.providerConsent}
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
                class="min-h-9"
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
                  class="min-h-9"
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
                  class="min-h-9"
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

        <Card.Root>
          <Card.Header>
            <div class="flex min-w-0 items-center gap-1.5">
              <Card.Title>{copy.localAgentRuntime}</Card.Title>
              <ContextHelp content={copy.localAgentRuntimeDescription} />
            </div>
          </Card.Header>
          <Card.Content class="space-y-3">
            {#each agentUiState.runtimeCatalog?.runtimes ?? [] as runtime (runtime.runtime)}
              <Button
                variant="outline"
                class={[
                  "h-auto min-h-9 w-full flex-col items-stretch gap-2 p-[var(--density-panel-padding)] text-left",
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
              </Button>
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

  {#if activeWorkspace}
    <Accordion.Root type="single" class="rounded-lg border px-[var(--density-panel-padding)]">
      <Accordion.Item value="advanced-agent-tools" class="border-0">
        <Accordion.Trigger level={2} class="py-3 text-sm font-semibold">
          {copy.advancedAgentTools}
        </Accordion.Trigger>
        <Accordion.Content class="pb-[var(--density-section-gap)]">
          <div class="grid gap-[var(--density-section-gap)] xl:grid-cols-2">
            <Card.Root>
              <Card.Header>
                <div class="flex min-w-0 items-center gap-1.5">
                  <Card.Title>{copy.bodyFreeContext}</Card.Title>
                  <ContextHelp content={copy.workspaceScopeDescription} />
                </div>
              </Card.Header>
              <Card.Content class="space-y-[var(--density-section-gap)]">
                <div class="flex flex-wrap gap-2">
                  <Button
                    variant="outline"
                    class="min-h-10"
                    disabled={!desktopRuntime || busy}
                    onclick={refreshAgentContext}
                  >
                    <RefreshCw
                      size={16}
                      strokeWidth={1.8}
                      data-icon="inline-start"
                      aria-hidden="true"
                    />
                    {copy.refreshAgentContext}
                  </Button>
                </div>
                {#if agentUiState.context}
                  <div class="space-y-2">
                    {#each agentUiState.context.blockers as blocker (blocker.code)}
                      <div class="rounded-lg border p-3">
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
                        <Button
                          variant="outline"
                          class="h-auto min-h-9 w-full flex-col items-start gap-1 p-3 text-left"
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
                        </Button>
                      {/each}
                    </div>
                  {/if}
                {/if}
                {#if agentUiState.capabilities}
                  <div class="rounded-lg border bg-muted/20 p-3">
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

            <Card.Root>
              <Card.Header>
                <div class="flex items-start justify-between gap-[var(--density-section-gap)]">
                  <div>
                    <Card.Title>{copy.workspaceBridge}</Card.Title>
                    <Card.Description class="mt-1.5">{copy.exportAgentPack}</Card.Description>
                  </div>
                  <FileOutput size={18} strokeWidth={1.8} aria-hidden="true" />
                </div>
              </Card.Header>
              <Card.Content class="space-y-[var(--density-section-gap)]">
                <div class="grid gap-[var(--density-section-gap)] sm:grid-cols-[160px_minmax(0,1fr)]">
                  <div class="space-y-2">
                    <Label for="agent-host-pack">{copy.agentHost}</Label>
                    <NativeSelect.Root
                      id="agent-host-pack"
                      size="desktop"
                      class="w-full"
                      value={agentUiState.host}
                      onchange={(event) =>
                        changeHost(event.currentTarget.value as AgentHost)}
                    >
                      <NativeSelect.Option value="codex">{copy.codex}</NativeSelect.Option>
                      <NativeSelect.Option value="claude">{copy.claude}</NativeSelect.Option>
                      <NativeSelect.Option value="generic">{copy.generic}</NativeSelect.Option>
                    </NativeSelect.Root>
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
                        size="icon-desktop"
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
                  class="min-h-9"
                  disabled={!desktopRuntime || busy || !agentUiState.destination}
                  onclick={exportPack}
                >
                  {copy.exportAgentPack}
                </Button>
                {#if agentUiState.exported}
                  <Alert.Root variant="success">
                    <Alert.Title>{hostLabel(agentUiState.exported.manifest.host)}</Alert.Title>
                    <Alert.Description class="truncate font-mono">
                      {agentUiState.exported.manifest_path}
                    </Alert.Description>
                    <Alert.Action>
                      <Badge variant="outline">
                        {agentUiState.exported.manifest.files.length} {copy.exportedFiles}
                      </Badge>
                    </Alert.Action>
                  </Alert.Root>
                {/if}
              </Card.Content>
            </Card.Root>
          </div>
        </Accordion.Content>
      </Accordion.Item>
    </Accordion.Root>
  {/if}

  {#if agentUiState.formError}
    <Alert.Root variant="destructive">
      <Alert.Description>{agentUiState.formError}</Alert.Description>
    </Alert.Root>
  {/if}
</Page.Root>

<AlertDialog.Root bind:open={uninstallSkillsOpen}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>{copy.removeSkills}</AlertDialog.Title>
      <AlertDialog.Description>{copy.removeSkillsDescription}</AlertDialog.Description>
    </AlertDialog.Header>
    <div class="rounded-lg border bg-muted/20 p-3">
      <p class="text-xs font-medium text-muted-foreground">
        {hostLabel(agentUiState.host)}
      </p>
      <p class="mt-1 break-all font-mono text-xs">
        {agentUiState.skillsStatus?.directory ?? activeWorkspace?.path ?? ""}
      </p>
    </div>
    <AlertDialog.Footer>
      <AlertDialog.Cancel onclick={() => (uninstallSkillsOpen = false)}>
        {copy.cancel}
      </AlertDialog.Cancel>
      <AlertDialog.Action variant="destructive" disabled={busy} onclick={uninstallSkills}>
        {copy.removeSkills}
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
