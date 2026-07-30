<script lang="ts">
  import {
    BriefcaseBusiness,
    ChevronRight,
    Clock3,
    Database,
  } from "@lucide/svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import type {
    JobDetailReadModel,
    JobRecord,
    RegistrySnapshot,
    WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";
  import type {
    LastSuccessfulAction,
    NavigationId,
    WorkflowRoute,
  } from "$lib/workflow-navigation";

  type JourneyStage = {
    number: number;
    label: string;
    view: NavigationId;
    recommended: boolean;
  };

  type Props = {
    copy: Messages;
    snapshot: RegistrySnapshot | null;
    activeWorkspace: WorkspaceReadModel | null;
    jobs: JobRecord[];
    selectedJob: JobDetailReadModel | null;
    currentViewLabel: string;
    stages: JourneyStage[];
    lastAction: LastSuccessfulAction | null;
    busy: boolean;
    onSelectWorkspace: (path: string) => Promise<boolean>;
    onSelectJob: (jobId: string) => Promise<boolean>;
    onNavigate: (route: WorkflowRoute) => Promise<void>;
  };

  let {
    copy,
    snapshot,
    activeWorkspace,
    jobs,
    selectedJob,
    currentViewLabel,
    stages,
    lastAction,
    busy,
    onSelectWorkspace,
    onSelectJob,
    onNavigate,
  }: Props = $props();

  const workspaceAlias = $derived(
    snapshot?.registry.entries.find((entry) => entry.path === activeWorkspace?.path)
      ?.alias ??
      activeWorkspace?.path ??
      copy.noWorkspace,
  );
</script>

<section
  class="border-b bg-background px-5 py-3 lg:px-8"
  aria-label={copy.workspaceContext}
  data-testid="workspace-context-bar"
>
  <div class="mx-auto flex max-w-[1480px] flex-col gap-3">
    <div class="grid gap-3 min-[860px]:grid-cols-[minmax(160px,0.82fr)_minmax(220px,1.18fr)_auto] min-[860px]:items-end">
      <div class="space-y-1.5">
        <label
          for="global-workspace"
          class="flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground"
        >
          <Database size={13} strokeWidth={1.8} aria-hidden="true" />
          {copy.currentWorkspace}
        </label>
        {#if (snapshot?.registry.entries.length ?? 0) > 0}
          <select
            id="global-workspace"
            class="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm font-medium"
            value={activeWorkspace?.path ?? ""}
            disabled={busy}
            onchange={(event) => void onSelectWorkspace(event.currentTarget.value)}
          >
            {#each snapshot?.registry.entries ?? [] as entry (entry.path)}
              <option value={entry.path}>{entry.alias}</option>
            {/each}
          </select>
        {:else}
          <button
            type="button"
            class="flex h-9 w-full items-center rounded-lg border border-dashed px-3 text-left text-sm text-muted-foreground hover:bg-muted/30"
            onclick={() => void onNavigate({ view: "workspaces" })}
          >
            {copy.chooseWorkspace}
          </button>
        {/if}
      </div>

      <div class="space-y-1.5">
        <label
          for="global-application"
          class="flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground"
        >
          <BriefcaseBusiness size={13} strokeWidth={1.8} aria-hidden="true" />
          {copy.currentApplication}
        </label>
        {#if activeWorkspace && jobs.length > 0}
          <select
            id="global-application"
            class="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm font-medium"
            value={selectedJob?.job.id ?? ""}
            disabled={busy}
            onchange={(event) => void onSelectJob(event.currentTarget.value)}
          >
            {#each jobs as job (job.id)}
              <option value={job.id}>{job.title} — {job.institution}</option>
            {/each}
          </select>
        {:else}
          <button
            type="button"
            class="flex h-9 w-full items-center rounded-lg border border-dashed px-3 text-left text-sm text-muted-foreground hover:bg-muted/30"
            disabled={!activeWorkspace}
            onclick={() => void onNavigate({ view: "applications" })}
          >
            {activeWorkspace ? copy.noApplications : copy.noApplicationSelected}
          </button>
        {/if}
      </div>

      <div class="flex min-h-9 items-center justify-between gap-3 rounded-lg border bg-muted/20 px-3 min-[860px]:min-w-48">
        <div class="min-w-0">
          <p class="text-[10px] font-medium uppercase tracking-[0.12em] text-muted-foreground">
            {copy.currentStage}
          </p>
          <p class="truncate text-xs font-semibold">{currentViewLabel}</p>
        </div>
        <Badge variant="outline" class="shrink-0 text-[10px]">
          {workspaceAlias}
        </Badge>
      </div>
    </div>

    <div class="flex flex-col justify-between gap-3 xl:flex-row xl:items-center">
      <ol class="flex min-w-0 flex-wrap items-center gap-1" aria-label={copy.applicationJourney}>
        {#each stages as stage (stage.view)}
          <li class="flex items-center">
            <button
              type="button"
              class={[
                "group flex min-h-8 items-center gap-2 rounded-lg px-2.5 text-xs transition-colors hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                stage.recommended
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground",
              ]}
              aria-current={stage.recommended ? "step" : undefined}
              title={stage.recommended ? copy.nextRecommended : stage.label}
              onclick={() => void onNavigate({ view: stage.view })}
            >
              <span
                class={[
                  "grid size-5 place-items-center rounded-full border text-[10px] font-semibold",
                  stage.recommended
                    ? "border-primary-foreground/45"
                    : "border-border bg-background group-hover:border-primary/40",
                ]}
              >
                {stage.number}
              </span>
              <span>{stage.label}</span>
            </button>
            {#if stage.number < stages.length}
              <ChevronRight
                size={13}
                strokeWidth={1.6}
                class="mx-0.5 text-muted-foreground/55"
                aria-hidden="true"
              />
            {/if}
          </li>
        {/each}
      </ol>

      <div class="flex min-w-0 items-center gap-2">
        <Clock3
          size={14}
          strokeWidth={1.8}
          class="shrink-0 text-muted-foreground"
          aria-hidden="true"
        />
        {#if lastAction}
          <p class="max-w-md truncate text-xs text-muted-foreground">
            <span class="font-medium text-foreground">{copy.lastSuccessfulAction}:</span>
            {lastAction.summary}
          </p>
          <Button
            variant="ghost"
            size="sm"
            class="min-h-8 shrink-0"
            onclick={() => void onNavigate(lastAction.route)}
          >
            {copy.resumeLastAction}
          </Button>
        {:else}
          <p class="text-xs text-muted-foreground">{copy.noRecentAction}</p>
        {/if}
      </div>
    </div>
  </div>
</section>
