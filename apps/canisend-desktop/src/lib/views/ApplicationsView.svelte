<script lang="ts">
  import {
    Archive,
    ArrowRight,
    BriefcaseBusiness,
    CalendarDays,
    CheckCircle2,
    CircleDot,
    FileText,
    FileUp,
    Link,
    MapPin,
    Plus,
    RefreshCw,
    ShieldCheck,
    TriangleAlert,
  } from "@lucide/svelte";

  import ActionMenu from "$lib/components/patterns/ActionMenu.svelte";
  import * as Page from "$lib/components/patterns/page/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import * as Alert from "$lib/components/ui/alert/index.js";
  import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import * as Empty from "$lib/components/ui/empty/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import { Skeleton } from "$lib/components/ui/skeleton/index.js";
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import IntakeReviewSummary from "$lib/components/IntakeReviewSummary.svelte";
  import LoadingPanel from "$lib/components/patterns/LoadingPanel.svelte";
  import {
    chooseJobSource,
    type ApplicationDossierReadModel,
    type ApplicationDossierState,
    type ContentCatalogEntryReadModel,
    type ContentCatalogFilter,
    type ContentCatalogReadModel,
    type ContentSearchReadModel,
    type JobDetailReadModel,
    type JobIntakePreviewReadModel,
    type JobRecord,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";
  import type { WorkflowDetail } from "$lib/workflow-navigation";

  type ContentLibraryPanelComponent =
    typeof import("$lib/components/ContentLibraryPanel.svelte").default;

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    activeWorkspace: WorkspaceReadModel | null;
    jobs: JobRecord[];
    selectedJob: JobDetailReadModel | null;
    dossiers: ApplicationDossierReadModel[];
    dossier: ApplicationDossierReadModel | null;
    contentCatalog: ContentCatalogReadModel | null;
    contentSearchResult: ContentSearchReadModel | null;
    focus: WorkflowDetail | null;
    preview: JobIntakePreviewReadModel | null;
    loading: boolean;
    contentLoading: boolean;
    busy: boolean;
    onRefresh: () => Promise<boolean>;
    onCreate: (title: string, institution: string) => Promise<boolean>;
    onSelect: (jobId: string) => Promise<boolean>;
    onArchive: (jobId: string) => Promise<boolean>;
    onPreviewLocal: (source: string, confirmed: boolean) => Promise<boolean>;
    onPreviewUrl: (url: string, confirmed: boolean) => Promise<boolean>;
    onCommitPreview: () => Promise<boolean>;
    onDiscardPreview: () => Promise<boolean>;
    onRefreshContent: () => Promise<boolean>;
    onSearchContent: (options: {
      query: string;
      filter: ContentCatalogFilter;
      includePrivateBodies: boolean;
      confirmedPrivateRead: boolean;
    }) => Promise<boolean>;
    onOpenContent: (entry: ContentCatalogEntryReadModel) => Promise<void>;
    onContinue: () => Promise<void>;
  };

  let {
    copy,
    desktopRuntime,
    activeWorkspace,
    jobs,
    selectedJob,
    dossiers,
    dossier,
    contentCatalog,
    contentSearchResult,
    focus,
    preview,
    loading,
    contentLoading,
    busy,
    onRefresh,
    onCreate,
    onSelect,
    onArchive,
    onPreviewLocal,
    onPreviewUrl,
    onCommitPreview,
    onDiscardPreview,
    onRefreshContent,
    onSearchContent,
    onOpenContent,
    onContinue,
  }: Props = $props();

  let createOpen = $state(false);
  let archiveOpen = $state(false);
  let title = $state("");
  let institution = $state("");
  let formError = $state<string | null>(null);
  let intakeTab = $state("local");
  let localSource = $state("");
  let sourceUrl = $state("");
  let privateReadConfirmed = $state(false);
  let networkFetchConfirmed = $state(false);
  let ContentLibraryPanel = $state<ContentLibraryPanelComponent | null>(null);
  let contentPanelLoading = $state(false);
  let contentPanelFailed = $state(false);

  $effect(() => {
    if (ContentLibraryPanel || contentPanelLoading || contentPanelFailed) return;
    contentPanelLoading = true;
    void import("$lib/components/ContentLibraryPanel.svelte")
      .then((module) => {
        ContentLibraryPanel = module.default;
      })
      .catch(() => {
        contentPanelFailed = true;
      })
      .finally(() => {
        contentPanelLoading = false;
      });
  });

  async function submitCreate(): Promise<void> {
    formError = null;
    if (!title.trim() || !institution.trim()) {
      formError = `${copy.applicationTitle} / ${copy.institution}`;
      return;
    }
    if (await onCreate(title.trim(), institution.trim())) {
      createOpen = false;
      title = "";
      institution = "";
    }
  }

  async function chooseLocalSource(): Promise<void> {
    localSource = (await chooseJobSource()) ?? localSource;
  }

  async function submitLocalSource(): Promise<void> {
    formError = null;
    if (!localSource) {
      formError = copy.chooseFile;
      return;
    }
    if (!privateReadConfirmed) {
      formError = copy.privateReadConsent;
      return;
    }
    if (await onPreviewLocal(localSource, privateReadConfirmed)) {
      localSource = "";
      privateReadConfirmed = false;
    }
  }

  async function submitUrlSource(): Promise<void> {
    formError = null;
    if (!sourceUrl.trim()) {
      formError = copy.sourceUrl;
      return;
    }
    if (!networkFetchConfirmed) {
      formError = copy.networkFetchConsent;
      return;
    }
    if (await onPreviewUrl(sourceUrl.trim(), networkFetchConfirmed)) {
      sourceUrl = "";
      networkFetchConfirmed = false;
    }
  }

  async function confirmArchive(): Promise<void> {
    if (selectedJob && (await onArchive(selectedJob.job.id))) {
      archiveOpen = false;
    }
  }

  function dossierStateLabel(state: ApplicationDossierState): string {
    return copy.applicationDossierState[state];
  }
</script>

{#snippet headerActions()}
  <Button
    variant="outline"
    class="page-action"
    disabled={!activeWorkspace || busy}
    onclick={onRefresh}
  >
    <RefreshCw size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
    {copy.refresh}
  </Button>
  <Button
    class="page-action"
    disabled={!desktopRuntime || !activeWorkspace || busy}
    onclick={() => {
      formError = null;
      createOpen = true;
    }}
  >
    <Plus size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
    {copy.createApplication}
  </Button>
{/snippet}

<Page.Root>
  <Page.Header
    eyebrow={copy.applicationWorkspace}
    title={copy.applicationsTitle}
    description={copy.applicationsDescription}
    actions={activeWorkspace ? headerActions : undefined}
  />

  {#if !activeWorkspace}
    <Card.Root>
      <Card.Content>
        <Empty.Root class="min-h-32">
          <Empty.Header>
            <Empty.Media variant="icon" class="size-12 rounded-lg bg-accent text-accent-foreground">
              <BriefcaseBusiness size={21} strokeWidth={1.8} aria-hidden="true" />
            </Empty.Media>
            <Empty.Title class="text-base">{copy.noWorkspace}</Empty.Title>
            <Empty.Description>{copy.chooseWorkspaceDescription}</Empty.Description>
          </Empty.Header>
        </Empty.Root>
      </Card.Content>
    </Card.Root>
  {:else}
    <Page.Grid class="xl:grid-cols-[minmax(300px,0.75fr)_minmax(0,1.25fr)]">
      <Card.Root>
        <Card.Header>
          <Card.Title>{copy.applications}</Card.Title>
          <Card.Description class="truncate" title={activeWorkspace.path}>
            {activeWorkspace.path}
          </Card.Description>
        </Card.Header>
        <Card.Content class="space-y-2">
          {#if loading}
            {#each [1, 2, 3] as row}
              <div class="space-y-2 rounded-lg border p-[var(--density-panel-padding)]">
                <Skeleton class="h-4 w-2/3" />
                <Skeleton class="h-3 w-1/2" />
              </div>
            {/each}
          {:else if !jobs.length}
            <Empty.Root class="min-h-32 border bg-muted/20">
              <Empty.Header>
                <Empty.Media variant="icon">
                  <BriefcaseBusiness size={22} strokeWidth={1.8} aria-hidden="true" />
                </Empty.Media>
                <Empty.Title>{copy.noApplications}</Empty.Title>
                <Empty.Description>{copy.noApplicationsDescription}</Empty.Description>
              </Empty.Header>
            </Empty.Root>
          {:else}
            {#each jobs as job (job.id)}
              {@const application = dossiers.find((dossier) => dossier.job.id === job.id)}
              <Button
                variant="outline"
                class={[
                  "h-auto min-h-9 w-full items-start justify-between p-[var(--density-panel-padding)] text-left",
                  selectedJob?.job.id === job.id ? "border-primary bg-accent/45" : "",
                ]}
                aria-current={selectedJob?.job.id === job.id ? "true" : undefined}
                onclick={() => onSelect(job.id)}
              >
                <div class="flex items-start justify-between gap-3">
                  <div class="min-w-0">
                    <h2 class="truncate text-sm font-semibold">{job.title}</h2>
                    <p class="mt-1 truncate text-xs text-muted-foreground">{job.institution}</p>
                  </div>
                  <div class="flex shrink-0 flex-col items-end gap-1.5">
                    {#if application}
                      <Badge variant="outline">
                        {dossierStateLabel(application.state)}
                      </Badge>
                    {/if}
                    <span class="text-[11px] text-muted-foreground">
                      {application?.metadata.deadline ?? `${job.source_ids.length} ${copy.sourceCount}`}
                    </span>
                  </div>
                </div>
              </Button>
            {/each}
          {/if}
        </Card.Content>
      </Card.Root>

      <Page.Stack>
        {#if dossier}
          <Card.Root>
            <Card.Header>
              <div class="flex flex-col justify-between gap-3 sm:flex-row sm:items-start">
                <div>
                  <Card.Title>{copy.applicationOverview}</Card.Title>
                  <Card.Description class="mt-1.5">
                    {copy.applicationOverviewDescription}
                  </Card.Description>
                </div>
                <Badge variant="secondary">{dossierStateLabel(dossier.state)}</Badge>
              </div>
            </Card.Header>
            <Card.Content class="space-y-[var(--density-section-gap)]">
              <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
                <div class="rounded-lg border bg-muted/20 p-3">
                  <p class="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <CircleDot size={14} strokeWidth={1.8} aria-hidden="true" />
                    {copy.workflowProgress}
                  </p>
                  <p class="mt-2 text-sm font-semibold">
                    {dossier.completed_stages} / {dossier.total_stages}
                  </p>
                </div>
                <div class="rounded-lg border bg-muted/20 p-3">
                  <p class="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <CircleDot size={14} strokeWidth={1.8} aria-hidden="true" />
                    {copy.currentStage}
                  </p>
                  <p class="mt-2 text-sm font-semibold">
                    {dossier.current_stage
                      ? copy.workflowStageLabel[dossier.current_stage]
                      : copy.allStagesComplete}
                  </p>
                </div>
                <div class="rounded-lg border bg-muted/20 p-3">
                  <p class="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <CalendarDays size={14} strokeWidth={1.8} aria-hidden="true" />
                    {copy.deadline}
                  </p>
                  <p class="mt-2 text-sm font-semibold">
                    {dossier.metadata.deadline ?? copy.noDeadlineRecorded}
                  </p>
                </div>
                <div class="rounded-lg border bg-muted/20 p-3">
                  <p class="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <MapPin size={14} strokeWidth={1.8} aria-hidden="true" />
                    {copy.location}
                  </p>
                  <p class="mt-2 truncate text-sm font-semibold">
                    {dossier.metadata.location ?? copy.notApplicable}
                  </p>
                </div>
              </div>

              <Progress
                class="h-2"
                value={dossier.completed_stages}
                max={dossier.total_stages || 1}
                aria-label={copy.workflowProgress}
              />

              {#if dossier.blockers[0]}
                <Alert.Root variant="warning">
                  <TriangleAlert
                    size={17}
                    strokeWidth={1.8}
                    aria-hidden="true"
                  />
                  <Alert.Title>{copy.attention}</Alert.Title>
                  <Alert.Description>{dossier.blockers[0].description}</Alert.Description>
                </Alert.Root>
              {/if}

              <div class="flex flex-col justify-between gap-[var(--density-section-gap)] rounded-lg border bg-accent/30 p-[var(--density-panel-padding)] sm:flex-row sm:items-center">
                <div>
                  <p class="text-xs font-medium text-muted-foreground">{copy.nextAction}</p>
                  <p class="mt-1 max-w-2xl text-sm font-semibold">
                    {dossier.next_actions[0]?.description ?? copy.noNextAction}
                  </p>
                </div>
                <Button
                  class="min-h-9 shrink-0"
                  disabled={busy || !dossier.next_actions.length}
                  onclick={onContinue}
                >
                  {copy.continueApplication}
                  <ArrowRight
                    size={16}
                    strokeWidth={1.8}
                    data-icon="inline-end"
                    aria-hidden="true"
                  />
                </Button>
              </div>
            </Card.Content>
          </Card.Root>
        {/if}

        {#if ContentLibraryPanel}
          <ContentLibraryPanel
            {copy}
            catalog={contentCatalog}
            searchResult={contentSearchResult}
            selectedJobId={selectedJob?.job.id ?? ""}
            loading={contentLoading}
            {busy}
            onRefresh={onRefreshContent}
            onSearch={onSearchContent}
            onOpen={onOpenContent}
          />
        {:else}
          <Card.Root>
            <Card.Content class="grid min-h-32 place-items-center p-[var(--density-panel-padding)]">
              {#if contentPanelFailed}
                <Alert.Root variant="destructive">
                  <Alert.Description>{copy.contentLibraryLoadFailed}</Alert.Description>
                </Alert.Root>
              {:else}
                <LoadingPanel label={copy.loading} class="w-full" />
              {/if}
            </Card.Content>
          </Card.Root>
        {/if}

        <Card.Root>
          <Card.Header>
            <div class="flex items-start justify-between gap-[var(--density-section-gap)]">
              <div>
                <Card.Title>{selectedJob?.job.title ?? copy.applicationDetails}</Card.Title>
                <Card.Description class="mt-1.5">
                  {selectedJob?.job.institution ?? copy.chooseApplication}
                </Card.Description>
              </div>
              {#if selectedJob}
                <ActionMenu label={copy.moreActions} disabled={busy}>
                  <DropdownMenu.Item
                    variant="destructive"
                    onclick={() => (archiveOpen = true)}
                  >
                    <Archive size={16} strokeWidth={1.8} aria-hidden="true" />
                    {copy.archiveApplication}
                  </DropdownMenu.Item>
                </ActionMenu>
              {/if}
            </div>
          </Card.Header>
          <Card.Content>
            {#if selectedJob}
              <div>
                <div class="mb-3 flex items-center justify-between">
                  <h3 class="text-sm font-semibold">{copy.attachedSources}</h3>
                  <Badge variant="secondary">{selectedJob.sources.length}</Badge>
                </div>
                {#if selectedJob.sources.length}
                  <div class="space-y-2">
                    {#each selectedJob.sources as source (source.id)}
                      <div class="flex items-start gap-3 rounded-lg border p-3">
                        <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent text-accent-foreground">
                          {#if source.kind === "user-url"}
                            <Link size={16} strokeWidth={1.8} aria-hidden="true" />
                          {:else}
                            <FileText size={16} strokeWidth={1.8} aria-hidden="true" />
                          {/if}
                        </div>
                        <div class="min-w-0 flex-1">
                          <p class="text-sm font-medium">{source.content_type}</p>
                          <p class="mt-1 truncate text-xs text-muted-foreground">
                            {source.final_url ?? source.source_url ?? source.kind}
                          </p>
                        </div>
                      </div>
                    {/each}
                  </div>
                {:else}
                  <Empty.Root class="min-h-20 border">
                    <Empty.Header><Empty.Title>{copy.noSources}</Empty.Title></Empty.Header>
                  </Empty.Root>
                {/if}
              </div>
            {:else}
              <Empty.Root class="min-h-32 border">
                <Empty.Header><Empty.Description>{copy.chooseApplication}</Empty.Description></Empty.Header>
              </Empty.Root>
            {/if}
          </Card.Content>
        </Card.Root>

        {#if selectedJob}
          <Card.Root
            id="source-intake"
            class={[
              "scroll-mt-64  transition-colors",
              focus === "source-intake" ? "ring-2 ring-primary/35" : "",
            ]}
          >
            <Card.Header>
              <Card.Title>{copy.sourceIntake}</Card.Title>
              <Card.Description>{copy.sourceIntakeDescription}</Card.Description>
            </Card.Header>
            <Card.Content>
              {#if preview}
                <div class="space-y-[var(--density-section-gap)]" aria-live="polite">
                  <div class="flex flex-col justify-between gap-3 sm:flex-row sm:items-start">
                    <div class="flex items-start gap-3">
                      <div class="grid size-10 shrink-0 place-items-center rounded-lg bg-accent text-accent-foreground">
                        <ShieldCheck size={18} strokeWidth={1.8} aria-hidden="true" />
                      </div>
                      <div>
                        <div class="flex flex-wrap items-center gap-2">
                          <h3 class="text-sm font-semibold">{copy.sourcePreviewTitle}</h3>
                          <Badge variant="secondary">{copy.reviewBeforeCommit}</Badge>
                        </div>
                        <p class="mt-1 text-xs leading-5 text-muted-foreground">
                          {copy.sourcePreviewDescription}
                        </p>
                      </div>
                    </div>
                    <Badge variant="outline">
                      {preview.preview.data.provenance.source_kind === "url"
                        ? copy.sourceUrl
                        : copy.localFile}
                    </Badge>
                  </div>

                  <IntakeReviewSummary {copy} review={preview.intake} />

                  <div class="space-y-2">
                    <p class="text-xs font-medium text-muted-foreground">
                      {copy.validationIssues}
                    </p>
                    {#each preview.preview.data.validation_issues as issue (issue.code)}
                      <Alert.Root variant={issue.severity === "warning" ? "warning" : "success"}>
                        {#if issue.severity === "warning"}
                          <TriangleAlert
                            size={16}
                            strokeWidth={1.8}
                            aria-hidden="true"
                          />
                        {:else}
                          <CheckCircle2
                            size={16}
                            strokeWidth={1.8}
                            aria-hidden="true"
                          />
                        {/if}
                        <Alert.Description>{issue.message}</Alert.Description>
                      </Alert.Root>
                    {/each}
                  </div>

                  <Separator />
                  <div class="flex flex-col gap-2 sm:flex-row">
                    <Button class="min-h-9" disabled={busy} onclick={onCommitPreview}>
                      {busy ? copy.working : copy.commitPreview}
                    </Button>
                    <Button
                      variant="outline"
                      class="min-h-9"
                      disabled={busy}
                      onclick={onDiscardPreview}
                    >
                      {copy.discardPreview}
                    </Button>
                  </div>
                </div>
              {:else}
                <Tabs.Root bind:value={intakeTab}>
                <Tabs.List class="responsive-tabs" data-columns="2">
                  <Tabs.Trigger value="local">
                    <FileUp size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                    {copy.localFile}
                  </Tabs.Trigger>
                  <Tabs.Trigger value="url">
                    <Link size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                    {copy.sourceUrl}
                  </Tabs.Trigger>
                </Tabs.List>
                <Tabs.Content value="local" class="space-y-[var(--density-section-gap)] pt-[var(--density-section-gap)]">
                  <div class="space-y-2">
                    <Label for="local-source">{copy.sourceFile}</Label>
                    <div class="flex gap-2">
                      <Input id="local-source" bind:value={localSource} readonly />
                      <Button type="button" variant="outline" class="shrink-0" onclick={chooseLocalSource}>
                        {copy.chooseFile}
                      </Button>
                    </div>
                  </div>
                  <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
                    <Checkbox
                      id="private-read-consent"
                      bind:checked={privateReadConfirmed}
                      class="mt-0.5"
                    />
                    <Label for="private-read-consent" class="text-xs leading-5 font-normal">
                      {copy.privateReadConsent}
                    </Label>
                  </div>
                  {#if formError && intakeTab === "local"}
                    <Alert.Root variant="destructive">
                      <Alert.Description>{formError}</Alert.Description>
                    </Alert.Root>
                  {/if}
                  <Button
                    class="min-h-9"
                    disabled={busy || !localSource || !privateReadConfirmed}
                    onclick={submitLocalSource}
                  >
                    {busy ? copy.working : copy.previewLocalSource}
                  </Button>
                </Tabs.Content>
                <Tabs.Content value="url" class="space-y-[var(--density-section-gap)] pt-[var(--density-section-gap)]">
                  <div class="space-y-2">
                    <Label for="source-url">{copy.sourceUrl}</Label>
                    <Input
                      id="source-url"
                      type="url"
                      bind:value={sourceUrl}
                      placeholder={copy.sourceUrlPlaceholder}
                      autocomplete="url"
                    />
                  </div>
                  <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
                    <Checkbox
                      id="network-fetch-consent"
                      bind:checked={networkFetchConfirmed}
                      class="mt-0.5"
                    />
                    <Label for="network-fetch-consent" class="text-xs leading-5 font-normal">
                      {copy.networkFetchConsent}
                    </Label>
                  </div>
                  {#if formError && intakeTab === "url"}
                    <Alert.Root variant="destructive">
                      <Alert.Description>{formError}</Alert.Description>
                    </Alert.Root>
                  {/if}
                  <Button
                    class="min-h-9"
                    disabled={busy || !sourceUrl.trim() || !networkFetchConfirmed}
                    onclick={submitUrlSource}
                  >
                    {busy ? copy.working : copy.previewUrlSource}
                  </Button>
                </Tabs.Content>
              </Tabs.Root>
              {/if}
            </Card.Content>
          </Card.Root>
        {/if}
      </Page.Stack>
    </Page.Grid>
  {/if}
</Page.Root>

<Dialog.Root bind:open={createOpen}>
  <Dialog.Content class="sm:max-w-lg">
    <Dialog.Header>
      <Dialog.Title>{copy.createApplication}</Dialog.Title>
      <Dialog.Description>{copy.createApplicationDescription}</Dialog.Description>
    </Dialog.Header>
    <form
      class="space-y-[var(--density-section-gap)]"
      onsubmit={(event) => {
        event.preventDefault();
        submitCreate();
      }}
    >
      <div class="space-y-2">
        <Label for="job-title">{copy.applicationTitle}</Label>
        <Input id="job-title" bind:value={title} autocomplete="off" />
      </div>
      <div class="space-y-2">
        <Label for="job-institution">{copy.institution}</Label>
        <Input id="job-institution" bind:value={institution} autocomplete="organization" />
      </div>
      {#if formError}
        <Alert.Root variant="destructive">
          <Alert.Description>{formError}</Alert.Description>
        </Alert.Root>
      {/if}
      <Dialog.Footer>
        <Button type="button" variant="outline" onclick={() => (createOpen = false)}>
          {copy.cancel}
        </Button>
        <Button type="submit" disabled={busy}>{busy ? copy.working : copy.createApplication}</Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>

<AlertDialog.Root bind:open={archiveOpen}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>{copy.archiveApplication}</AlertDialog.Title>
      <AlertDialog.Description>{copy.archiveApplicationDescription}</AlertDialog.Description>
    </AlertDialog.Header>
    <div class="rounded-lg border bg-muted/20 p-3">
      <p class="text-sm font-medium">{selectedJob?.job.title}</p>
      <p class="mt-1 text-xs text-muted-foreground">{selectedJob?.job.institution}</p>
    </div>
    <AlertDialog.Footer>
      <AlertDialog.Cancel onclick={() => (archiveOpen = false)}>{copy.cancel}</AlertDialog.Cancel>
      <AlertDialog.Action variant="destructive" disabled={busy} onclick={confirmArchive}>
        {copy.archiveApplication}
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
