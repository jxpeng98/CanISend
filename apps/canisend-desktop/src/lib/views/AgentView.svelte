<script lang="ts">
  import {
    Bot,
    Boxes,
    FileOutput,
    FolderOpen,
    RefreshCw,
    ShieldCheck,
  } from "@lucide/svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import {
    chooseExportDirectory,
    type AgentCapabilitiesReadModel,
    type AgentContextReadModel,
    type AgentPackExportReadModel,
    type JobRecord,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";

  type AgentHost = "codex" | "claude" | "generic";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    activeWorkspace: WorkspaceReadModel | null;
    jobs: JobRecord[];
    busy: boolean;
    onLoadCapabilities: () => Promise<AgentCapabilitiesReadModel | null>;
    onLoadContext: (jobId?: string) => Promise<AgentContextReadModel | null>;
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
    busy,
    onLoadCapabilities,
    onLoadContext,
    onExport,
  }: Props = $props();

  let capabilities = $state<AgentCapabilitiesReadModel | null>(null);
  let context = $state<AgentContextReadModel | null>(null);
  let selectedJobId = $state("");
  let host = $state<AgentHost>("codex");
  let destination = $state("");
  let exported = $state<AgentPackExportReadModel | null>(null);
  let formError = $state<string | null>(null);

  async function loadCapabilities(): Promise<void> {
    capabilities = await onLoadCapabilities();
  }

  async function loadContext(): Promise<void> {
    context = await onLoadContext(selectedJobId || undefined);
  }

  async function chooseDestination(): Promise<void> {
    destination = (await chooseExportDirectory()) ?? destination;
  }

  async function exportPack(): Promise<void> {
    formError = null;
    if (!destination) {
      formError = copy.chooseDirectory;
      return;
    }
    exported = await onExport(host, destination);
  }

  function hostLabel(value: AgentHost): string {
    if (value === "codex") return copy.codex;
    if (value === "claude") return copy.claude;
    return copy.generic;
  }
</script>

<section class="space-y-6">
  <div>
    <Badge variant="secondary" class="mb-3">{copy.agent}</Badge>
    <h1 class="text-3xl font-semibold tracking-[-0.03em]">{copy.agentTitle}</h1>
    <p class="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">
      {copy.agentDescription}
    </p>
  </div>

  <div class="grid gap-6 xl:grid-cols-2">
    <Card.Root class="shadow-none">
      <Card.Header>
        <div class="flex items-start justify-between gap-4">
          <div>
            <Card.Title>{copy.capabilities}</Card.Title>
            <Card.Description class="mt-1.5">
              {capabilities?.protocol ?? copy.protocol}
            </Card.Description>
          </div>
          <div class="grid size-10 place-items-center rounded-xl bg-accent text-accent-foreground">
            <Boxes size={18} strokeWidth={1.8} aria-hidden="true" />
          </div>
        </div>
      </Card.Header>
      <Card.Content class="space-y-4">
        {#if capabilities}
          <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-3 text-sm">
            <dt class="text-muted-foreground">{copy.version}</dt>
            <dd class="text-right font-medium">{capabilities.product_version}</dd>
            <dt class="text-muted-foreground">{copy.protocol}</dt>
            <dd class="text-right font-medium">{capabilities.protocol}</dd>
            <dt class="text-muted-foreground">{copy.capabilities}</dt>
            <dd class="text-right font-medium">{capabilities.capabilities.length}</dd>
            <dt class="text-muted-foreground">{copy.workflowStages}</dt>
            <dd class="text-right font-medium">{capabilities.stages.length}</dd>
          </dl>
          <Separator />
          <div class="flex flex-wrap gap-2">
            {#each capabilities.error_codes.slice(0, 12) as code}
              <Badge variant="outline">{code}</Badge>
            {/each}
          </div>
        {:else}
          <div class="flex min-h-48 flex-col items-center justify-center rounded-xl border border-dashed text-center">
            <Bot size={22} strokeWidth={1.8} class="text-muted-foreground" aria-hidden="true" />
            <p class="mt-3 text-sm text-muted-foreground">{copy.agentDescription}</p>
          </div>
        {/if}
        <Button
          variant="outline"
          class="min-h-11"
          disabled={!desktopRuntime || busy}
          onclick={loadCapabilities}
        >
          <RefreshCw size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
          {copy.refresh}
        </Button>
      </Card.Content>
    </Card.Root>

    <Card.Root class="shadow-none">
      <Card.Header>
        <div class="flex items-start justify-between gap-4">
          <div>
            <Card.Title>{copy.context}</Card.Title>
            <Card.Description class="mt-1.5">
              {context?.workspace_id ?? activeWorkspace?.path ?? copy.noWorkspace}
            </Card.Description>
          </div>
          <div class="grid size-10 place-items-center rounded-xl bg-accent text-accent-foreground">
            <ShieldCheck size={18} strokeWidth={1.8} aria-hidden="true" />
          </div>
        </div>
      </Card.Header>
      <Card.Content class="space-y-4">
        <div class="space-y-2">
          <Label for="agent-job">{copy.selectApplication}</Label>
          <select
            id="agent-job"
            class="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm"
            bind:value={selectedJobId}
          >
            <option value="">—</option>
            {#each jobs as job (job.id)}
              <option value={job.id}>{job.title} — {job.institution}</option>
            {/each}
          </select>
        </div>
        <Button
          variant="outline"
          class="min-h-11"
          disabled={!desktopRuntime || busy}
          onclick={loadContext}
        >
          {copy.loadContext}
        </Button>
        <Separator />
        <div>
          <h2 class="text-sm font-semibold">{copy.agentBlockers}</h2>
          <div class="mt-3 space-y-2">
            {#each context?.blockers ?? [] as blocker (blocker.code)}
              <div class="rounded-xl border p-3">
                <p class="text-xs font-semibold">{blocker.code}</p>
                <p class="mt-1 text-xs leading-5 text-muted-foreground">
                  {blocker.description}
                </p>
              </div>
            {:else}
              <p class="text-sm text-muted-foreground">{copy.noAgentBlockers}</p>
            {/each}
          </div>
        </div>
        {#if context?.next_actions.length}
          <div>
            <h2 class="text-sm font-semibold">{copy.nextActions}</h2>
            <div class="mt-3 space-y-2">
              {#each context.next_actions as action}
                <div class="rounded-xl border bg-muted/20 p-3">
                  <p class="break-all font-mono text-[11px]">{action.action}</p>
                  <p class="mt-1 text-xs leading-5 text-muted-foreground">
                    {action.description}
                  </p>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </Card.Content>
    </Card.Root>
  </div>

  <Card.Root class="shadow-none">
    <Card.Header>
      <div class="flex items-start justify-between gap-4">
        <div>
          <Card.Title>{copy.exportAgentPack}</Card.Title>
          <Card.Description class="mt-1.5">{copy.agentDescription}</Card.Description>
        </div>
        <div class="grid size-10 place-items-center rounded-xl bg-accent text-accent-foreground">
          <FileOutput size={18} strokeWidth={1.8} aria-hidden="true" />
        </div>
      </div>
    </Card.Header>
    <Card.Content class="space-y-4">
      <div class="grid gap-4 lg:grid-cols-[220px_minmax(0,1fr)_auto] lg:items-end">
        <div class="space-y-2">
          <Label for="agent-host">{copy.agentHost}</Label>
          <select
            id="agent-host"
            class="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm"
            bind:value={host}
          >
            <option value="codex">{copy.codex}</option>
            <option value="claude">{copy.claude}</option>
            <option value="generic">{copy.generic}</option>
          </select>
        </div>
        <div class="space-y-2">
          <Label for="agent-export-directory">{copy.exportDestination}</Label>
          <div class="flex gap-2">
            <Input id="agent-export-directory" bind:value={destination} readonly />
            <Button variant="outline" class="shrink-0" onclick={chooseDestination}>
              <FolderOpen size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
              {copy.chooseDirectory}
            </Button>
          </div>
        </div>
        <Button
          class="min-h-11"
          disabled={!desktopRuntime || busy || !destination}
          onclick={exportPack}
        >
          {copy.exportAgentPack}
        </Button>
      </div>
      {#if exported}
        <div class="rounded-xl border border-[var(--success)]/35 bg-[var(--success)]/8 p-4">
          <div class="flex items-center justify-between gap-4">
            <p class="text-sm font-semibold">{hostLabel(exported.manifest.host)}</p>
            <Badge variant="outline">
              {exported.manifest.files.length} {copy.exportedFiles}
            </Badge>
          </div>
          <p class="mt-2 truncate font-mono text-xs text-muted-foreground">
            {exported.manifest_path}
          </p>
        </div>
      {/if}
      {#if formError}
        <p class="text-sm text-destructive" role="alert">{formError}</p>
      {/if}
    </Card.Content>
  </Card.Root>
</section>
