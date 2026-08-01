<script lang="ts">
  import { Activity, Database, FileUp, Plus, ShieldCheck } from "@lucide/svelte";

  import * as Page from "$lib/components/patterns/page/index.js";
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
    variant="outline"
    class="page-action"
    disabled={!activeWorkspace}
    onclick={() => void onNavigate({ view: "applications", detail: "source-intake" })}
  >
    <FileUp size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
    {copy.importSource}
  </Button>
  <Button
    class="page-action"
    disabled={!activeWorkspace}
    onclick={() => void onNavigate({ view: "applications" })}
  >
    <Plus size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
    {copy.newApplication}
  </Button>
{/snippet}

<Page.Root>
  <Page.Header
    eyebrow={copy.today}
    title={copy.pageTitle}
    description={copy.pageDescription}
    actions={headerActions}
  />

  <Page.Grid class="gap-[var(--shell-block-gap)] md:grid-cols-2 xl:grid-cols-4" aria-label={copy.today}>
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
  <Card.Root class="min-h-32">
    <Card.Header class="p-[var(--shell-card-padding)] pb-2">
      <Card.Description>{copy.localFirst}</Card.Description>
      <Card.Title class="flex items-center gap-2 text-base">
        <ShieldCheck size={18} strokeWidth={1.8} aria-hidden="true" />
        {copy.healthy}
      </Card.Title>
    </Card.Header>
    <Card.Content class="p-[var(--shell-card-padding)] pt-0 text-sm text-muted-foreground">
      {copy.localDescription}
    </Card.Content>
  </Card.Root>
  </Page.Grid>

  <Page.Grid class="xl:grid-cols-[1.3fr_0.9fr]">
  <Card.Root>
    <Card.Header>
      <Card.Title>{copy.nextActions}</Card.Title>
      <Card.Description>
        {selectedDossier
          ? `${selectedDossier.job.title} — ${selectedDossier.job.institution}`
          : copy.chooseWorkspaceDescription}
      </Card.Description>
    </Card.Header>
    <Card.Content>
      <Empty.Root class="min-h-32 border-0 bg-muted/55">
        <Empty.Header>
          <Empty.Media variant="icon" class="size-11 rounded-lg bg-background text-foreground ring-1 ring-border">
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

  <Card.Root>
    <Card.Header>
      <div class="flex items-center justify-between gap-[var(--density-section-gap)]">
        <div>
          <Card.Title>{copy.diagnostics}</Card.Title>
          <Card.Description class="mt-1.5">{copy.diagnosticsDescription}</Card.Description>
        </div>
        <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-muted text-foreground">
          <Activity size={19} strokeWidth={1.8} aria-hidden="true" />
        </div>
      </div>
    </Card.Header>
    <Card.Content class="space-y-[var(--density-section-gap)]">
      <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
        <dt class="text-muted-foreground">{copy.version}</dt>
        <dd class="truncate text-right font-medium">{product?.version ?? "—"}</dd>
        <dt class="text-muted-foreground">{copy.protocol}</dt>
        <dd class="truncate text-right font-medium">{product?.protocol ?? "—"}</dd>
        <dt class="text-muted-foreground">{copy.platform}</dt>
        <dd class="truncate text-right font-medium">
          {product ? `${product.target_os} / ${product.target_arch}` : "—"}
        </dd>
      </dl>
      <Separator />
      <div class="flex items-center justify-between gap-[var(--density-section-gap)]">
        <p class="text-sm text-muted-foreground" aria-live="polite">
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
  </Card.Root>
  </Page.Grid>
</Page.Root>
