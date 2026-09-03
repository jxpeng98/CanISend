<script lang="ts">
  import { Activity, Database, FileUp } from "@lucide/svelte";

  import * as Page from "$lib/components/patterns/page/index.js";
  import * as Accordion from "$lib/components/ui/accordion/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import * as Empty from "$lib/components/ui/empty/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import type {
    ActionReceipt,
    ApplicationDossierReadModel,
    DoctorSummary,
    ProductSummary,
    WorkspaceHealthReadModel,
    WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";
  import type { WorkflowRecommendation, WorkflowRoute } from "$lib/workflow-navigation";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    activeWorkspace: WorkspaceReadModel | null;
    jobCount: number;
    upcomingDeadlineCount: number;
    nearestDeadlineItem: ApplicationDossierReadModel | null;
    workspaceHealth: WorkspaceHealthReadModel | null;
    selectedDossier: ApplicationDossierReadModel | null;
    recommendation: WorkflowRecommendation;
    product: ProductSummary | null;
    doctor: ActionReceipt<DoctorSummary> | null;
    doctorRunning: boolean;
    onNavigate: (route: WorkflowRoute) => Promise<void>;
    onDoctor: () => Promise<void>;
  };

  let {
    copy,
    desktopRuntime,
    activeWorkspace,
    jobCount,
    upcomingDeadlineCount,
    nearestDeadlineItem,
    workspaceHealth,
    selectedDossier,
    recommendation,
    product,
    doctor,
    doctorRunning,
    onNavigate,
    onDoctor,
  }: Props = $props();
</script>

{#snippet headerActions()}
  <Button
    class="page-action"
    onclick={() => void onNavigate({ view: "applications", detail: "source-intake" })}
  >
    <FileUp size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
    {copy.importSource}
  </Button>
{/snippet}

<Page.Root>
  <Page.Header
    eyebrow={copy.today}
    title={copy.pageTitle}
    description={copy.pageDescription}
    actions={activeWorkspace ? headerActions : undefined}
  />

  <Page.Grid class="gap-[var(--shell-block-gap)] md:grid-cols-3" aria-label={copy.today}>
    <Card.Root class="min-h-32">
      <Card.Header class="p-[var(--shell-card-padding)] pb-2">
        <Card.Description>{copy.activeApplications}</Card.Description>
        <Card.Title class="text-2xl">{jobCount}</Card.Title>
      </Card.Header>
      <Card.Content class="p-[var(--shell-card-padding)] pt-0 text-sm text-muted-foreground">
        {activeWorkspace ? copy.applicationsDescription : copy.activeDescription}
      </Card.Content>
    </Card.Root>
    <Card.Root class="min-h-32">
      <Card.Header class="p-[var(--shell-card-padding)] pb-2">
        <Card.Description>{copy.upcomingDeadlines}</Card.Description>
        <Card.Title class="text-2xl">{upcomingDeadlineCount}</Card.Title>
      </Card.Header>
      <Card.Content class="p-[var(--shell-card-padding)] pt-0 text-sm text-muted-foreground">
        {nearestDeadlineItem
          ? `${copy.nextDeadline}: ${nearestDeadlineItem.metadata.deadline} — ${nearestDeadlineItem.job.title}`
          : copy.noUpcomingDeadlines}
      </Card.Content>
    </Card.Root>
    <Card.Root class="min-h-32">
      <Card.Header class="p-[var(--shell-card-padding)] pb-2">
        <Card.Description>{copy.workflowHealth}</Card.Description>
        <Card.Title class="flex items-center gap-2 text-base">
          <span class="size-2 rounded-full bg-success"></span>
          {workspaceHealth?.check.ok === false ? copy.integrityIssues : copy.healthy}
        </Card.Title>
      </Card.Header>
      <Card.Content class="p-[var(--shell-card-padding)] pt-0 text-sm text-muted-foreground">
        {copy.healthDescription}
      </Card.Content>
    </Card.Root>
  </Page.Grid>

  <Page.Grid class="xl:grid-cols-[1.3fr_0.9fr]">
    <Card.Root>
      <Card.Header>
        <Card.Title>{copy.nextActions}</Card.Title>
        {#if selectedDossier}
          <Card.Description>
            {selectedDossier.job.title} — {selectedDossier.job.institution}
          </Card.Description>
        {/if}
      </Card.Header>
      <Card.Content>
        <Empty.Root class="min-h-32 border-0 bg-muted/55">
          <Empty.Header>
            <Empty.Media
              variant="icon"
              class="size-11 rounded-lg bg-background text-foreground ring-1 ring-border"
            >
              <Database size={20} strokeWidth={1.8} aria-hidden="true" />
            </Empty.Media>
            <Empty.Title class="text-base">
              {copy.recommendationTitle[recommendation.reason]}
            </Empty.Title>
            <Empty.Description>
              {selectedDossier?.next_actions[0]?.description ??
                copy.recommendationDescription[recommendation.reason]}
            </Empty.Description>
          </Empty.Header>
          <Empty.Content>
            <Button onclick={() => void onNavigate(recommendation.route)}>
              {copy.continueNextAction}
            </Button>
          </Empty.Content>
        </Empty.Root>
      </Card.Content>
    </Card.Root>

    <Card.Root class="self-start gap-0 py-0">
      <Accordion.Root type="single">
        <Accordion.Item value="system-diagnostics" class="border-0">
          <Card.Header class="p-0">
            <Accordion.Trigger
              level={2}
              class="rounded-xl px-[var(--card-spacing)] py-[var(--card-spacing)] text-sm font-semibold"
            >
              <span class="flex items-center gap-2">
                <Activity size={17} strokeWidth={1.8} aria-hidden="true" />
                {copy.diagnostics}
              </span>
            </Accordion.Trigger>
          </Card.Header>
          <Accordion.Content class="pb-0">
            <Card.Content class="space-y-[var(--density-section-gap)] pb-[var(--card-spacing)]">
              <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
                <dt class="text-muted-foreground">{copy.protocol}</dt>
                <dd class="truncate text-right font-medium">{product?.protocol ?? "—"}</dd>
                <dt class="text-muted-foreground">{copy.platform}</dt>
                <dd class="truncate text-right font-medium">
                  {product ? `${product.target_os} / ${product.target_arch}` : "—"}
                </dd>
              </dl>
              <Separator />
              <div
                class="flex flex-wrap items-center justify-between gap-[var(--density-section-gap)]"
              >
                <p class="min-w-0 flex-1 text-sm text-muted-foreground" aria-live="polite">
                  {doctor?.summary ?? copy.diagnosticsReady}
                </p>
                <Button
                  variant="outline"
                  class="min-h-9 shrink-0"
                  disabled={doctorRunning || !desktopRuntime}
                  onclick={onDoctor}
                >
                  {doctorRunning ? copy.runningDiagnostics : copy.runDiagnostics}
                </Button>
              </div>
            </Card.Content>
          </Accordion.Content>
        </Accordion.Item>
      </Accordion.Root>
    </Card.Root>
  </Page.Grid>
</Page.Root>
