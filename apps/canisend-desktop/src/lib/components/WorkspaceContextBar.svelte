<script lang="ts">
  import {
    ArrowRight,
    BriefcaseBusiness,
    CalendarDays,
    ChevronDown,
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
  import * as Collapsible from "$lib/components/ui/collapsible/index.js";
  import * as NativeSelect from "$lib/components/ui/native-select/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import type {
    ApplicationFlowStageV3,
    ApplicationDossierReadModel,
    JobRecord,
    RegistrySnapshot,
    StoredApplicationModelV3,
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
    selectedJob: ApplicationDossierReadModel | null;
    v4Applications: StoredApplicationModelV3[];
    selectedV4Application: StoredApplicationModelV3 | null;
    v4Stages: ApplicationFlowStageV3[];
    activeView: NavigationId;
    activeDetail: WorkflowDetail | null;
    recommendation: WorkflowRecommendation;
    lastAction: LastSuccessfulAction | null;
    busy: boolean;
    onSelectWorkspace: (path: string) => Promise<boolean>;
    onSelectJob: (jobId: string) => Promise<boolean>;
    onSelectV4Application: (applicationId: string) => void;
    onNavigate: (route: WorkflowRoute) => Promise<void>;
  };

  let {
    copy,
    snapshot,
    activeWorkspace,
    jobs,
    selectedJob,
    v4Applications,
    selectedV4Application,
    v4Stages,
    activeView,
    activeDetail,
    recommendation,
    lastAction,
    busy,
    onSelectWorkspace,
    onSelectJob,
    onSelectV4Application,
    onNavigate,
  }: Props = $props();

  let detailsOpen = $state(false);

  const workspaceAlias = $derived(
    snapshot?.registry.entries.find((entry) => entry.path === activeWorkspace?.path)?.alias ??
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
    selectedJob?.total_stages
      ? Math.round((selectedJob.completed_stages / selectedJob.total_stages) * 100)
      : 0,
  );
  const usesV4ApplicationContext = $derived(
    activeView === "applications" || v4Applications.length > 0,
  );
  const v4CompletedStages = $derived(v4Stages.filter((stage) => stage.state === "complete").length);
  const v4ProgressPercent = $derived(
    v4Stages.length ? Math.round((v4CompletedStages / v4Stages.length) * 100) : 0,
  );
  const v4CurrentStage = $derived(
    v4Stages
      .find((stage) => stage.state !== "complete")
      ?.id.split(":")
      .at(-1) ?? copy.allStagesComplete,
  );
  const currentStageLabel = $derived(
    selectedJob?.current_stage
      ? copy.workflowStageLabel[selectedJob.current_stage]
      : selectedJob?.state === "complete"
        ? copy.allStagesComplete
        : copy.notApplicable,
  );
  const nextActionDescription = $derived(
    selectedJob?.next_actions[0]?.description ??
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
    void onNavigate(routeForApplicationSection(section, selectedJob?.job.id));
  }
</script>

<Collapsible.Root bind:open={detailsOpen}>
  <section
    class="min-w-0 px-1 pb-2"
    aria-label={copy.workspaceContext}
    data-testid="workspace-context-bar"
  >
    <div
      class="flex min-w-0 flex-col gap-[var(--workspace-context-gap)] rounded-lg border border-sidebar-border bg-card p-[var(--workspace-context-padding)] shadow-sm"
    >
      <div class="flex min-w-0 flex-col gap-2">
        <div class="workspace-context-grid min-w-0 flex-1 gap-[var(--workspace-context-gap)]">
          <div class="min-w-0 space-y-1.5">
            <label
              for="global-workspace"
              class="flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground"
            >
              <Database size={13} strokeWidth={1.8} aria-hidden="true" />
              {copy.currentWorkspace}
            </label>
            {#if (snapshot?.registry.entries.length ?? 0) > 0}
              <NativeSelect.Root
                id="global-workspace"
                size="desktop"
                class="w-full font-medium"
                value={activeWorkspace?.path ?? ""}
                disabled={busy}
                onchange={(event) => void onSelectWorkspace(event.currentTarget.value)}
              >
                {#each snapshot?.registry.entries ?? [] as entry (entry.path)}
                  <NativeSelect.Option value={entry.path}>{entry.alias}</NativeSelect.Option>
                {/each}
              </NativeSelect.Root>
            {:else}
              <Button
                variant="outline"
                size="desktop"
                class="h-auto min-h-9 w-full min-w-0 justify-start whitespace-normal border-dashed py-2 text-left leading-snug text-muted-foreground"
                onclick={() => void onNavigate({ view: "workspaces" })}
              >
                {copy.chooseWorkspace}
              </Button>
            {/if}
          </div>

          <div class="min-w-0 space-y-1.5">
            <div class="flex items-center justify-between gap-2">
              <label
                for="global-application"
                class="flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground"
              >
                <BriefcaseBusiness size={13} strokeWidth={1.8} aria-hidden="true" />
                {copy.currentApplication}
              </label>
              {#if usesV4ApplicationContext && selectedV4Application}
                <Badge variant="outline" class="px-1.5 py-0 text-[9px]">
                  {selectedV4Application.snapshot.application.lifecycle}
                </Badge>
              {:else if selectedJob}
                <Badge variant="outline" class="px-1.5 py-0 text-[9px]">
                  {copy.applicationDossierState[selectedJob.state]}
                </Badge>
              {/if}
            </div>
            {#if usesV4ApplicationContext && activeWorkspace && v4Applications.length > 0}
              <NativeSelect.Root
                id="global-application"
                size="desktop"
                class="w-full font-medium"
                value={selectedV4Application?.snapshot.application.id ?? ""}
                disabled={busy}
                onchange={(event) => onSelectV4Application(event.currentTarget.value)}
              >
                {#each v4Applications as application (application.snapshot.application.id)}
                  <NativeSelect.Option value={application.snapshot.application.id}>
                    {application.snapshot.opportunity.title}
                  </NativeSelect.Option>
                {/each}
              </NativeSelect.Root>
            {:else if !usesV4ApplicationContext && activeWorkspace && jobs.length > 0}
              <NativeSelect.Root
                id="global-application"
                size="desktop"
                class="w-full font-medium"
                value={selectedJob?.job.id ?? ""}
                disabled={busy}
                onchange={(event) => void onSelectJob(event.currentTarget.value)}
              >
                {#each jobs as job (job.id)}
                  <NativeSelect.Option value={job.id}
                    >{job.title} — {job.institution}</NativeSelect.Option
                  >
                {/each}
              </NativeSelect.Root>
            {:else}
              <Button
                variant="outline"
                size="desktop"
                class="h-auto min-h-9 w-full min-w-0 justify-start whitespace-normal border-dashed py-2 text-left leading-snug text-muted-foreground"
                disabled={!activeWorkspace}
                onclick={() => void onNavigate({ view: "applications" })}
              >
                {activeWorkspace
                  ? usesV4ApplicationContext
                    ? copy.chooseApplication
                    : copy.noApplications
                  : copy.noApplicationSelected}
              </Button>
            {/if}
          </div>
        </div>

        {#if activeView !== "today"}
          <Collapsible.Trigger>
            {#snippet child({ props })}
              <Button
                variant="ghost"
                size="desktop"
                class="w-full shrink-0 justify-between text-muted-foreground"
                {...props}
              >
                {copy.applicationSnapshot}
                <ChevronDown
                  size={16}
                  strokeWidth={1.8}
                  class={detailsOpen ? "rotate-180 transition-transform" : "transition-transform"}
                  data-icon="inline-end"
                  aria-hidden="true"
                />
              </Button>
            {/snippet}
          </Collapsible.Trigger>
        {/if}
      </div>

      {#if activeView !== "today"}
        <Collapsible.Content class="space-y-2.5 border-t pt-2.5">
          {#if usesV4ApplicationContext}
            <dl class="workspace-snapshot-grid gap-2" aria-label={copy.applicationSnapshot}>
              <div class="min-w-0 rounded-lg bg-muted/55 px-2.5 py-2">
                <dt class="text-[10px] font-medium text-muted-foreground">{copy.workflowPack}</dt>
                <dd class="mt-1 truncate text-xs font-semibold">
                  {selectedV4Application?.snapshot.pack.id ?? copy.notApplicable}
                </dd>
              </div>
              <div class="min-w-0 rounded-lg bg-muted/55 px-2.5 py-2">
                <dt class="flex items-center gap-1.5 text-[10px] font-medium text-muted-foreground">
                  <Gauge size={12} strokeWidth={1.8} aria-hidden="true" />
                  {copy.currentStage}
                </dt>
                <dd class="mt-1 truncate text-xs font-semibold">{v4CurrentStage}</dd>
              </div>
              <div class="min-w-0 rounded-lg bg-muted/55 px-2.5 py-2">
                <dt class="text-[10px] font-medium text-muted-foreground">
                  {copy.workflowProgress}
                </dt>
                <dd class="mt-1 flex items-center gap-2 text-xs font-semibold">
                  <span>{v4ProgressPercent}%</span>
                  <Progress
                    class="h-1.5 min-w-8 flex-1"
                    value={v4ProgressPercent}
                    aria-label={copy.workflowProgress}
                  />
                </dd>
              </div>
            </dl>
          {:else}
            <dl class="workspace-snapshot-grid gap-2" aria-label={copy.applicationSnapshot}>
              <div class="min-w-0 rounded-lg bg-muted/55 px-2.5 py-2">
                <dt class="flex items-center gap-1.5 text-[10px] font-medium text-muted-foreground">
                  <CalendarDays size={12} strokeWidth={1.8} aria-hidden="true" />
                  {copy.deadline}
                </dt>
                <dd class="mt-1 truncate text-xs font-semibold">
                  {selectedJob?.metadata.deadline ?? copy.noDeadlineRecorded}
                </dd>
              </div>
              <div class="min-w-0 rounded-lg bg-muted/55 px-2.5 py-2">
                <dt class="flex items-center gap-1.5 text-[10px] font-medium text-muted-foreground">
                  <Gauge size={12} strokeWidth={1.8} aria-hidden="true" />
                  {copy.currentStage}
                </dt>
                <dd class="mt-1 truncate text-xs font-semibold">{currentStageLabel}</dd>
              </div>
              <div class="min-w-0 rounded-lg bg-muted/55 px-2.5 py-2">
                <dt class="text-[10px] font-medium text-muted-foreground">
                  {copy.workflowProgress}
                </dt>
                <dd class="mt-1 flex items-center gap-2 text-xs font-semibold">
                  <span>{progressPercent}%</span>
                  <Progress
                    class="h-1.5 min-w-8 flex-1"
                    value={progressPercent}
                    aria-label={copy.workflowProgress}
                  />
                </dd>
              </div>
            </dl>

            <div class="flex flex-col justify-between gap-2">
              <nav class="min-w-0 flex-1" aria-label={copy.applicationWorkspaceNavigation}>
                <ol class="workspace-section-grid gap-1.5 rounded-lg bg-muted/55 p-1">
                  {#each sections as section, index (section.id)}
                    {@const Icon = section.icon}
                    <li class="flex min-w-0 items-stretch">
                      <Button
                        variant={activeSection === section.id ? "secondary" : "ghost"}
                        size="desktop"
                        class="h-auto min-h-9 w-full min-w-0 justify-start gap-1.5 whitespace-normal px-2.5 py-1.5 text-left text-xs leading-snug"
                        aria-current={activeSection === section.id ? "page" : undefined}
                        disabled={!selectedJob && section.id !== "overview"}
                        onclick={() => openSection(section.id)}
                      >
                        <Icon size={15} strokeWidth={1.8} aria-hidden="true" />
                        <span>{index + 1}. {section.label}</span>
                      </Button>
                    </li>
                  {/each}
                </ol>
              </nav>

              <div class="workspace-next-action gap-2">
                <div class="min-w-0">
                  <p
                    class="text-[10px] font-medium uppercase tracking-[0.1em] text-muted-foreground"
                  >
                    {copy.nextAction}
                  </p>
                  <p
                    class="max-w-md text-xs font-medium leading-5 line-clamp-2"
                    title={nextActionDescription}
                  >
                    {nextActionDescription}
                  </p>
                </div>
                <Button
                  size="desktop"
                  class="workspace-next-action-button shrink-0"
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

            {#if selectedJob?.blockers[0] || lastAction}
              <div class="flex flex-col justify-between gap-2 border-t pt-2">
                <div class="flex min-w-0 items-center gap-2">
                  {#if selectedJob?.blockers[0]}
                    <TriangleAlert
                      size={14}
                      strokeWidth={1.8}
                      class="shrink-0 text-warning"
                      aria-hidden="true"
                    />
                    <p class="truncate text-xs text-muted-foreground">
                      <span class="font-medium text-foreground">{copy.currentBlocker}:</span>
                      {selectedJob.blockers[0].description}
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
                      size="desktop"
                      class="shrink-0"
                      onclick={() => void onNavigate(lastAction.route)}
                    >
                      {copy.resumeLastAction}
                    </Button>
                  </div>
                {/if}
              </div>
            {/if}
          {/if}
        </Collapsible.Content>
      {/if}

      <p class="sr-only">{workspaceAlias}</p>
    </div>
  </section>
</Collapsible.Root>
