<script lang="ts">
  import {
    ArrowRight,
    BriefcaseBusiness,
    CalendarDays,
    ClipboardList,
    Clock3,
    Database,
    FileCheck2,
    Files,
    Gauge,
    LayoutDashboard,
    Scale,
    TriangleAlert,
  } from "@lucide/svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import type {
    ApplicationDossierReadModel,
    JobDetailReadModel,
    JobRecord,
    RegistrySnapshot,
    WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";
  import {
    applicationSectionForRoute,
    routeForApplicationSection,
    type ApplicationWorkspaceSection,
    type LastSuccessfulAction,
    type NavigationId,
    type WorkflowDetail,
    type WorkflowRecommendation,
    type WorkflowRoute,
  } from "$lib/workflow-navigation";

  type Props = {
    copy: Messages;
    snapshot: RegistrySnapshot | null;
    activeWorkspace: WorkspaceReadModel | null;
    jobs: JobRecord[];
    selectedJob: JobDetailReadModel | null;
    dossier: ApplicationDossierReadModel | null;
    activeView: NavigationId;
    activeDetail: WorkflowDetail | null;
    recommendation: WorkflowRecommendation;
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
    dossier,
    activeView,
    activeDetail,
    recommendation,
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
  const activeSection = $derived(
    applicationSectionForRoute({
      view: activeView,
      detail: activeDetail ?? undefined,
    }),
  );
  const progressPercent = $derived(
    dossier?.total_stages
      ? Math.round((dossier.completed_stages / dossier.total_stages) * 100)
      : 0,
  );
  const currentStageLabel = $derived(
    dossier?.current_stage
      ? copy.workflowStageLabel[dossier.current_stage]
      : dossier?.state === "complete"
        ? copy.allStagesComplete
        : copy.notApplicable,
  );
  const nextActionDescription = $derived(
    dossier?.next_actions[0]?.description ??
      copy.recommendationDescription[recommendation.reason],
  );
  const sections = $derived([
    {
      id: "overview" as const,
      label: copy.applicationWorkspaceSectionLabel.overview,
      icon: LayoutDashboard,
    },
    {
      id: "job-criteria" as const,
      label: copy.applicationWorkspaceSectionLabel["job-criteria"],
      icon: ClipboardList,
    },
    {
      id: "evidence-fit" as const,
      label: copy.applicationWorkspaceSectionLabel["evidence-fit"],
      icon: Scale,
    },
    {
      id: "materials" as const,
      label: copy.applicationWorkspaceSectionLabel.materials,
      icon: Files,
    },
    {
      id: "review-export" as const,
      label: copy.applicationWorkspaceSectionLabel["review-export"],
      icon: FileCheck2,
    },
  ]);

  function openSection(section: ApplicationWorkspaceSection): void {
    void onNavigate(
      routeForApplicationSection(section, selectedJob?.job.id),
    );
  }
</script>

<section
  class="border-b bg-background px-5 py-3 lg:px-8"
  aria-label={copy.workspaceContext}
  data-testid="workspace-context-bar"
>
  <div class="mx-auto flex max-w-[1480px] flex-col gap-3">
    <div
      class="grid gap-3 sm:grid-cols-2 xl:grid-cols-[minmax(160px,0.75fr)_minmax(240px,1.25fr)_minmax(420px,1.45fr)] xl:items-end"
    >
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
            class="h-10 w-full rounded-lg border border-input bg-background px-3 text-sm font-medium"
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
            class="flex h-10 w-full items-center rounded-lg border border-dashed px-3 text-left text-sm text-muted-foreground transition-colors hover:bg-muted/30"
            onclick={() => void onNavigate({ view: "workspaces" })}
          >
            {copy.chooseWorkspace}
          </button>
        {/if}
      </div>

      <div class="space-y-1.5">
        <div class="flex items-center justify-between gap-2">
          <label
            for="global-application"
            class="flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground"
          >
            <BriefcaseBusiness size={13} strokeWidth={1.8} aria-hidden="true" />
            {copy.currentApplication}
          </label>
          {#if dossier}
            <Badge variant="outline" class="px-1.5 py-0 text-[9px]">
              {copy.applicationDossierState[dossier.state]}
            </Badge>
          {/if}
        </div>
        {#if activeWorkspace && jobs.length > 0}
          <select
            id="global-application"
            class="h-10 w-full rounded-lg border border-input bg-background px-3 text-sm font-medium"
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
            class="flex h-10 w-full items-center rounded-lg border border-dashed px-3 text-left text-sm text-muted-foreground transition-colors hover:bg-muted/30 disabled:cursor-not-allowed disabled:opacity-60"
            disabled={!activeWorkspace}
            onclick={() => void onNavigate({ view: "applications" })}
          >
            {activeWorkspace ? copy.noApplications : copy.noApplicationSelected}
          </button>
        {/if}
      </div>

      <dl
        class="grid grid-cols-3 gap-2 sm:col-span-2 xl:col-span-1"
        aria-label={copy.applicationSnapshot}
      >
        <div class="min-w-0 rounded-lg border bg-muted/20 px-3 py-2">
          <dt class="flex items-center gap-1.5 text-[10px] font-medium text-muted-foreground">
            <CalendarDays size={12} strokeWidth={1.8} aria-hidden="true" />
            {copy.deadline}
          </dt>
          <dd class="mt-1 truncate text-xs font-semibold">
            {dossier?.metadata.deadline ?? copy.noDeadlineRecorded}
          </dd>
        </div>
        <div class="min-w-0 rounded-lg border bg-muted/20 px-3 py-2">
          <dt class="flex items-center gap-1.5 text-[10px] font-medium text-muted-foreground">
            <Gauge size={12} strokeWidth={1.8} aria-hidden="true" />
            {copy.currentStage}
          </dt>
          <dd class="mt-1 truncate text-xs font-semibold">{currentStageLabel}</dd>
        </div>
        <div class="min-w-0 rounded-lg border bg-muted/20 px-3 py-2">
          <dt class="text-[10px] font-medium text-muted-foreground">
            {copy.workflowProgress}
          </dt>
          <dd class="mt-1 flex items-center gap-2 text-xs font-semibold">
            <span>{progressPercent}%</span>
            <span
              class="h-1.5 min-w-8 flex-1 overflow-hidden rounded-full bg-muted"
              role="progressbar"
              aria-label={copy.workflowProgress}
              aria-valuemin="0"
              aria-valuemax="100"
              aria-valuenow={progressPercent}
            >
              <span
                class="block h-full rounded-full bg-primary transition-[width] duration-200 motion-reduce:transition-none"
                style={`width: ${progressPercent}%`}
              ></span>
            </span>
          </dd>
        </div>
      </dl>
    </div>

    <div class="flex flex-col justify-between gap-3 xl:flex-row xl:items-center">
      <nav
        class="min-w-0 overflow-x-auto"
        aria-label={copy.applicationWorkspaceNavigation}
      >
        <ol class="flex min-w-max items-center gap-1">
          {#each sections as section, index (section.id)}
            {@const Icon = section.icon}
            <li class="flex items-center">
              <button
                type="button"
                class={[
                  "flex min-h-10 items-center gap-2 rounded-lg px-3 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                  activeSection === section.id
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                ]}
                aria-current={activeSection === section.id ? "page" : undefined}
                disabled={!selectedJob && section.id !== "overview"}
                onclick={() => openSection(section.id)}
              >
                <Icon size={15} strokeWidth={1.8} aria-hidden="true" />
                <span>{index + 1}. {section.label}</span>
              </button>
            </li>
          {/each}
        </ol>
      </nav>

      <div class="flex min-w-0 items-center gap-3">
        <div class="min-w-0">
          <p class="text-[10px] font-medium uppercase tracking-[0.12em] text-muted-foreground">
            {copy.nextAction}
          </p>
          <p class="max-w-md truncate text-xs font-medium">{nextActionDescription}</p>
        </div>
        <Button
          size="sm"
          class="min-h-10 shrink-0"
          disabled={busy}
          onclick={() => void onNavigate(recommendation.route)}
        >
          {copy.continueNextAction}
          <ArrowRight
            size={15}
            strokeWidth={1.8}
            data-icon="inline-end"
            aria-hidden="true"
          />
        </Button>
      </div>
    </div>

    {#if dossier?.blockers[0] || lastAction}
      <div class="flex flex-col justify-between gap-2 border-t pt-2 lg:flex-row lg:items-center">
        <div class="flex min-w-0 items-center gap-2">
          {#if dossier?.blockers[0]}
            <TriangleAlert
              size={14}
              strokeWidth={1.8}
              class="shrink-0 text-amber-600 dark:text-amber-400"
              aria-hidden="true"
            />
            <p class="truncate text-xs text-muted-foreground">
              <span class="font-medium text-foreground">{copy.currentBlocker}:</span>
              {dossier.blockers[0].description}
            </p>
          {/if}
        </div>

        {#if lastAction}
          <div class="flex min-w-0 items-center gap-2">
            <Clock3
              size={14}
              strokeWidth={1.8}
              class="shrink-0 text-muted-foreground"
              aria-hidden="true"
            />
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
          </div>
        {/if}
      </div>
    {/if}

    <p class="sr-only">{workspaceAlias}</p>
  </div>
</section>
