<script lang="ts">
  import { CheckCircle2, FileCheck2, Plus, RefreshCw, ShieldCheck } from "@lucide/svelte";

  import * as Page from "$lib/components/patterns/page/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import * as Alert from "$lib/components/ui/alert/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import * as Empty from "$lib/components/ui/empty/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import * as NativeSelect from "$lib/components/ui/native-select/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";
  import {
    approveGenericApplication,
    commandErrorMessage,
    composeGenericApplication,
    createGenericApplication,
    exportGenericApplication,
    listGenericApplications,
    planGenericApplication,
    reviewGenericApplication,
    showGenericApplication,
    type ApplicationFieldValueV3,
    type ApplicationFlowDeliverableDraftV3,
    type ApplicationFlowReviewReadModelV3,
    type ApplicationFlowStageV3,
    type StoredApplicationModelV3,
    type WorkflowPackPresentationField,
    type WorkflowPackPresentationReadModel,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";
  import { exactUtf8Span } from "$lib/generic-application-form";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    activeWorkspace: WorkspaceReadModel;
    presentation: WorkflowPackPresentationReadModel | null;
  };

  let { copy, desktopRuntime, activeWorkspace, presentation }: Props = $props();

  let applications = $state<StoredApplicationModelV3[]>([]);
  let selected = $state<StoredApplicationModelV3 | null>(null);
  let stages = $state<ApplicationFlowStageV3[]>([]);
  let review = $state<ApplicationFlowReviewReadModelV3 | null>(null);
  let busy = $state(false);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let loadedWorkspace = "";

  let title = $state("");
  let sourceText = $state("");
  let requirementStatement = $state("");
  let requirementCategory = $state("");
  let requirementPriority = $state<"mandatory" | "recommended" | "informational">(
    "mandatory",
  );
  let opportunityValues = $state<Record<string, string>>({});
  let applicationValues = $state<Record<string, string>>({});
  let deliverableSelections = $state<Record<string, boolean>>({});
  let deliverableDrafts = $state<Record<string, { title: string; content: string }>>({});
  let privateReviewConsent = $state(false);
  let reviewConfirmed = $state(false);
  let exportDestination = $state("");
  let privateExportConsent = $state(false);

  const selectedRevision = $derived(selected?.snapshot.application.revision ?? 0);
  const completedStages = $derived(stages.filter((stage) => stage.state === "complete").length);
  const stageProgress = $derived(stages.length ? (completedStages / stages.length) * 100 : 0);
  const plannedDeliverables = $derived(
    selected?.snapshot.plan?.deliverables.filter(
      (deliverable) => deliverable.disposition !== "omitted",
    ) ?? [],
  );

  $effect(() => {
    const workspace = activeWorkspace.path;
    if (!workspace || workspace === loadedWorkspace) return;
    loadedWorkspace = workspace;
    void refresh();
  });

  $effect(() => {
    if (!requirementCategory && presentation?.requirement_categories[0]) {
      requirementCategory = presentation.requirement_categories[0].id;
    }
    for (const deliverable of presentation?.deliverables ?? []) {
      if (!(deliverable.id in deliverableSelections)) {
        deliverableSelections[deliverable.id] = deliverable.minimum > 0;
      }
    }
  });

  function localId(value: string): string {
    return value.includes(":") ? (value.split(":").at(-1) ?? value) : value;
  }

  function deliverableLabel(kind: string): string {
    const id = localId(kind);
    return presentation?.deliverables.find((item) => item.id === id)?.label.value ?? id;
  }

  function fieldInputType(field: WorkflowPackPresentationField): string {
    if (field.field_type === "date") return "date";
    if (field.field_type === "url") return "url";
    if (field.field_type === "integer") return "number";
    return "text";
  }

  function metadata(
    fields: WorkflowPackPresentationField[],
    values: Record<string, string>,
  ): Record<string, ApplicationFieldValueV3> {
    const result: Record<string, ApplicationFieldValueV3> = {};
    for (const field of fields) {
      const raw = values[field.id]?.trim() ?? "";
      if (!raw && !field.required) continue;
      if (field.field_type === "integer") {
        result[field.id] = { type: "integer", value: Number(raw) };
      } else if (field.field_type === "boolean") {
        result[field.id] = { type: "boolean", value: raw === "true" };
      } else if (field.field_type === "string-list") {
        result[field.id] = {
          type: "string-list",
          value: raw.split("\n").map((item) => item.trim()).filter(Boolean),
        };
      } else {
        result[field.id] = { type: field.field_type, value: raw };
      }
    }
    return result;
  }

  function captureError(value: unknown): void {
    error = commandErrorMessage(value);
    notice = null;
  }

  async function refresh(preferredId = selected?.snapshot.application.id): Promise<void> {
    loading = true;
    error = null;
    try {
      const receipt = await listGenericApplications(activeWorkspace.path);
      applications = receipt.data;
      const next = applications.find((item) => item.snapshot.application.id === preferredId)
        ?? applications[0]
        ?? null;
      if (next) await selectApplication(next);
      else {
        selected = null;
        stages = [];
      }
    } catch (value) {
      captureError(value);
    } finally {
      loading = false;
    }
  }

  async function selectApplication(application: StoredApplicationModelV3): Promise<void> {
    error = null;
    review = null;
    reviewConfirmed = false;
    try {
      const receipt = await showGenericApplication(
        activeWorkspace.path,
        application.snapshot.application.id,
      );
      selected = receipt.data.stored;
      stages = receipt.data.stages;
      exportDestination = `applications/${receipt.data.stored.snapshot.application.id}/exports/revision-${receipt.data.stored.snapshot.application.revision}`;
      prepareDrafts();
    } catch (value) {
      captureError(value);
    }
  }

  function prepareDrafts(): void {
    for (const planned of plannedDeliverables) {
      const id = localId(planned.kind);
      if (!deliverableDrafts[id]) {
        deliverableDrafts[id] = { title: deliverableLabel(id), content: "" };
      }
    }
  }

  async function run(action: () => Promise<void>): Promise<void> {
    busy = true;
    error = null;
    notice = null;
    try {
      await action();
    } catch (value) {
      captureError(value);
    } finally {
      busy = false;
    }
  }

  async function submitCreate(): Promise<void> {
    const statement = requirementStatement.trim();
    const source = sourceText.trim();
    const span = exactUtf8Span(source, statement);
    if (!title.trim() || !source || !statement || !requirementCategory || !span) {
      error = copy.requirementMustMatchSource;
      return;
    }
    await run(async () => {
      const receipt = await createGenericApplication(activeWorkspace.path, {
        title: title.trim(),
        opportunity_metadata: metadata(
          presentation?.opportunity_fields ?? [],
          opportunityValues,
        ),
        application_metadata: metadata(
          presentation?.application_fields ?? [],
          applicationValues,
        ),
        source_text: source,
        requirements: [
          {
            category: requirementCategory,
            statement,
            priority: requirementPriority,
            start_byte: span[0],
            end_byte: span[1],
          },
        ],
      });
      selected = receipt.data.stored;
      stages = receipt.data.stages;
      applications = [receipt.data.stored, ...applications];
      notice = receipt.summary;
      title = "";
      sourceText = "";
      requirementStatement = "";
      opportunityValues = {};
      applicationValues = {};
    });
  }

  async function submitPlan(): Promise<void> {
    if (!selected || !presentation) return;
    await run(async () => {
      const receipt = await planGenericApplication(
        activeWorkspace.path,
        selected!.snapshot.application.id,
        {
          expected_revision: selected!.snapshot.application.revision,
          decision: "proceed",
          deliverables: presentation!.deliverables.map((item) => ({
            kind: item.id,
            disposition: deliverableSelections[item.id]
              ? item.minimum > 0 ? "required" : "optional"
              : "omitted",
            rationale: "User confirmed this Pack Deliverable in the desktop plan.",
            constraints: ["Use only reviewed local source material and confirmed evidence."],
            execution_mode: "manual-import",
          })),
        },
      );
      selected = receipt.data.commit.stored;
      stages = receipt.data.stages;
      prepareDrafts();
      notice = receipt.summary;
      await refresh(selected.snapshot.application.id);
    });
  }

  async function submitCompose(): Promise<void> {
    if (!selected) return;
    const drafts: ApplicationFlowDeliverableDraftV3[] = plannedDeliverables.map((planned) => {
      const id = localId(planned.kind);
      const draft = deliverableDrafts[id] ?? { title: deliverableLabel(id), content: "" };
      return {
        kind: id,
        title: draft.title.trim(),
        media_type: "text/markdown",
        content: draft.content.trim(),
      };
    });
    if (drafts.some((draft) => !draft.title || !draft.content)) {
      error = copy.deliverableContent;
      return;
    }
    await run(async () => {
      const receipt = await composeGenericApplication(
        activeWorkspace.path,
        selected!.snapshot.application.id,
        { expected_revision: selected!.snapshot.application.revision, deliverables: drafts },
      );
      selected = receipt.data.commit.stored;
      stages = receipt.data.stages;
      notice = receipt.summary;
      review = null;
      reviewConfirmed = false;
      await refresh(selected.snapshot.application.id);
    });
  }

  async function loadReview(): Promise<void> {
    if (!selected || !privateReviewConsent) return;
    await run(async () => {
      const receipt = await reviewGenericApplication(
        activeWorkspace.path,
        selected!.snapshot.application.id,
        privateReviewConsent,
      );
      review = receipt.data;
      stages = receipt.data.stages;
      notice = receipt.summary;
    });
  }

  async function submitApproval(): Promise<void> {
    if (!selected || !review || !reviewConfirmed) return;
    await run(async () => {
      const receipt = await approveGenericApplication(
        activeWorkspace.path,
        selected!.snapshot.application.id,
        selected!.snapshot.application.revision,
      );
      selected = receipt.data.commit.stored;
      stages = receipt.data.stages;
      notice = receipt.summary;
      await refresh(selected.snapshot.application.id);
    });
  }

  async function submitExport(): Promise<void> {
    if (!selected || !exportDestination.trim() || !privateExportConsent) return;
    await run(async () => {
      const receipt = await exportGenericApplication({
        workspace: activeWorkspace.path,
        applicationId: selected!.snapshot.application.id,
        expectedRevision: selected!.snapshot.application.revision,
        destination: exportDestination.trim(),
        confirmedPrivateExport: privateExportConsent,
      });
      stages = receipt.data.stages;
      notice = `${receipt.summary} ${copy.submissionBoundary}`;
    });
  }
</script>

{#snippet headerActions()}
  <Button variant="outline" disabled={busy || loading} onclick={() => refresh()}>
    <RefreshCw size={17} strokeWidth={1.8} aria-hidden="true" />
    {copy.refresh}
  </Button>
{/snippet}

<Page.Root>
  <Page.Header
    eyebrow={presentation?.vocabulary.application_plural ?? copy.applications}
    title={copy.applicationsTitle}
    description={copy.genericApplicationsDescription}
    actions={headerActions}
  />

  {#if error}
    <Alert.Root variant="destructive" role="alert" aria-live="assertive">
      <Alert.Description>{error}</Alert.Description>
    </Alert.Root>
  {/if}
  {#if notice}
    <Alert.Root variant="success" aria-live="polite">
      <CheckCircle2 size={17} strokeWidth={1.8} aria-hidden="true" />
      <Alert.Description>{notice}</Alert.Description>
    </Alert.Root>
  {/if}
  <Alert.Root aria-live="polite">
    <ShieldCheck size={17} strokeWidth={1.8} aria-hidden="true" />
    <Alert.Description>{copy.submissionBoundary}</Alert.Description>
  </Alert.Root>

  <Page.Grid class="xl:grid-cols-[minmax(280px,0.7fr)_minmax(0,1.3fr)]">
    <div class="space-y-[var(--density-section-gap)]">
      <Card.Root>
        <Card.Header>
          <Card.Title>{presentation?.vocabulary.application_plural ?? copy.applications}</Card.Title>
          <Card.Description class="truncate" title={activeWorkspace.path}>
            {activeWorkspace.path}
          </Card.Description>
        </Card.Header>
        <Card.Content class="space-y-2">
          {#if loading}
            <p class="text-sm text-muted-foreground" aria-live="polite">{copy.loading}</p>
          {:else if !applications.length}
            <Empty.Root class="min-h-28 border bg-muted/20">
              <Empty.Header>
                <Empty.Media variant="icon"><FileCheck2 size={20} aria-hidden="true" /></Empty.Media>
                <Empty.Title>{copy.noGenericApplications}</Empty.Title>
              </Empty.Header>
            </Empty.Root>
          {:else}
            {#each applications as application (application.snapshot.application.id)}
              <Button
                variant={selected?.snapshot.application.id === application.snapshot.application.id
                  ? "secondary"
                  : "ghost"}
                class="h-auto w-full justify-start px-3 py-2 text-left"
                aria-pressed={selected?.snapshot.application.id === application.snapshot.application.id}
                onclick={() => selectApplication(application)}
              >
                <span class="min-w-0">
                  <span class="block truncate font-medium">{application.snapshot.opportunity.title}</span>
                  <span class="block text-xs text-muted-foreground">
                    {copy.revision} {application.snapshot.application.revision} · {application.snapshot.application.lifecycle}
                  </span>
                </span>
              </Button>
            {/each}
          {/if}
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header>
          <Card.Title>{copy.createApplication}</Card.Title>
          <Card.Description>{presentation?.pack.id}</Card.Description>
        </Card.Header>
        <Card.Content>
          <form class="space-y-4" onsubmit={(event) => { event.preventDefault(); submitCreate(); }}>
            <div class="space-y-2">
              <Label for="generic-title">{copy.genericApplicationTitle}</Label>
              <Input id="generic-title" bind:value={title} required />
            </div>
            {#each presentation?.opportunity_fields ?? [] as field (field.id)}
              <div class="space-y-2">
                <Label for={`generic-opportunity-${field.id}`}>{field.label.value}</Label>
                <Input
                  id={`generic-opportunity-${field.id}`}
                  type={fieldInputType(field)}
                  value={opportunityValues[field.id] ?? ""}
                  required={field.required}
                  oninput={(event) => (opportunityValues[field.id] = event.currentTarget.value)}
                />
              </div>
            {/each}
            {#each presentation?.application_fields ?? [] as field (field.id)}
              <div class="space-y-2">
                <Label for={`generic-application-${field.id}`}>{field.label.value}</Label>
                {#if field.field_type === "choice"}
                  <NativeSelect.Root
                    id={`generic-application-${field.id}`}
                    value={applicationValues[field.id] ?? ""}
                    onchange={(event) => (applicationValues[field.id] = event.currentTarget.value)}
                  >
                    <NativeSelect.Option value="">—</NativeSelect.Option>
                    {#each field.options as option (option.id)}
                      <NativeSelect.Option value={option.id}>{option.label.value}</NativeSelect.Option>
                    {/each}
                  </NativeSelect.Root>
                {:else if field.field_type === "long-text"}
                  <Textarea
                    id={`generic-application-${field.id}`}
                    value={applicationValues[field.id] ?? ""}
                    oninput={(event) => (applicationValues[field.id] = event.currentTarget.value)}
                  />
                {:else}
                  <Input
                    id={`generic-application-${field.id}`}
                    type={fieldInputType(field)}
                    value={applicationValues[field.id] ?? ""}
                    oninput={(event) => (applicationValues[field.id] = event.currentTarget.value)}
                  />
                {/if}
              </div>
            {/each}
            <div class="space-y-2">
              <Label for="generic-source">{copy.sourceText}</Label>
              <Textarea id="generic-source" bind:value={sourceText} rows={6} required />
            </div>
            <div class="space-y-2">
              <Label for="generic-requirement">{copy.requirementStatement}</Label>
              <Textarea
                id="generic-requirement"
                bind:value={requirementStatement}
                aria-describedby="generic-requirement-help"
                required
              />
              <p id="generic-requirement-help" class="text-xs text-muted-foreground">
                {copy.requirementMustMatchSource}
              </p>
            </div>
            <div class="grid gap-4 sm:grid-cols-2">
              <div class="space-y-2">
                <Label for="generic-category">{copy.requirementCategory}</Label>
                <NativeSelect.Root id="generic-category" bind:value={requirementCategory}>
                  {#each presentation?.requirement_categories ?? [] as category (category.id)}
                    <NativeSelect.Option value={category.id}>{category.label.value}</NativeSelect.Option>
                  {/each}
                </NativeSelect.Root>
              </div>
              <div class="space-y-2">
                <Label for="generic-priority">{copy.priority}</Label>
                <NativeSelect.Root id="generic-priority" bind:value={requirementPriority}>
                  <NativeSelect.Option value="mandatory">{copy.mandatory}</NativeSelect.Option>
                  <NativeSelect.Option value="recommended">{copy.recommended}</NativeSelect.Option>
                  <NativeSelect.Option value="informational">{copy.informational}</NativeSelect.Option>
                </NativeSelect.Root>
              </div>
            </div>
            <Button type="submit" disabled={!desktopRuntime || busy || !presentation}>
              <Plus size={17} strokeWidth={1.8} aria-hidden="true" />
              {busy ? copy.working : copy.createApplication}
            </Button>
          </form>
        </Card.Content>
      </Card.Root>
    </div>

    <div class="space-y-[var(--density-section-gap)]">
      {#if selected}
        <Card.Root>
          <Card.Header>
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div>
                <Card.Title>{selected.snapshot.opportunity.title}</Card.Title>
                <Card.Description>{selected.snapshot.application.id}</Card.Description>
              </div>
              <Badge variant="secondary">{copy.revision} {selectedRevision}</Badge>
            </div>
          </Card.Header>
          <Card.Content class="space-y-3">
            <Progress value={stageProgress} aria-label={`${completedStages}/${stages.length}`} />
            <div class="flex flex-wrap gap-2" aria-label={copy.applicationJourney}>
              {#each presentation?.stages ?? [] as stage (stage.id)}
                {@const state = stages.find((item) => localId(item.id) === stage.id)?.state ?? "pending"}
                <Badge variant={state === "complete" ? "default" : state === "ready" ? "secondary" : "outline"}>
                  {stage.label.value}
                </Badge>
              {/each}
            </div>
          </Card.Content>
        </Card.Root>

        {#if !selected.snapshot.plan}
          <Card.Root>
            <Card.Header>
              <Card.Title>{copy.confirmApplicationPlan}</Card.Title>
              <Card.Description>{presentation?.vocabulary.requirement_plural}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              <ul class="space-y-2 text-sm">
                {#each selected.snapshot.requirements as requirement (requirement.id)}
                  <li class="rounded-md border p-3">{requirement.statement}</li>
                {/each}
              </ul>
              <fieldset class="space-y-3">
                <legend class="text-sm font-medium">{presentation?.vocabulary.deliverable_plural}</legend>
                {#each presentation?.deliverables ?? [] as deliverable (deliverable.id)}
                  <div class="flex items-start gap-3 rounded-md border p-3">
                    <Checkbox
                      id={`plan-${deliverable.id}`}
                      bind:checked={deliverableSelections[deliverable.id]}
                      disabled={deliverable.minimum > 0}
                    />
                    <Label for={`plan-${deliverable.id}`} class="min-w-0 font-normal">
                      <span class="block font-medium">{deliverable.label.value}</span>
                      <span class="block text-xs text-muted-foreground">
                        {deliverable.minimum}–{deliverable.maximum}
                      </span>
                    </Label>
                  </div>
                {/each}
              </fieldset>
              <Button disabled={busy} onclick={submitPlan}>{copy.confirmApplicationPlan}</Button>
            </Card.Content>
          </Card.Root>
        {:else if !selected.snapshot.deliverables.length}
          <Card.Root>
            <Card.Header><Card.Title>{copy.composeDeliverables}</Card.Title></Card.Header>
            <Card.Content>
              <form class="space-y-5" onsubmit={(event) => { event.preventDefault(); submitCompose(); }}>
                {#each plannedDeliverables as planned (planned.kind)}
                  {@const id = localId(planned.kind)}
                  <fieldset class="space-y-3 rounded-md border p-4">
                    <legend class="px-1 text-sm font-semibold">{deliverableLabel(id)}</legend>
                    <div class="space-y-2">
                      <Label for={`deliverable-title-${id}`}>{copy.genericApplicationTitle}</Label>
                      <Input id={`deliverable-title-${id}`} bind:value={deliverableDrafts[id].title} required />
                    </div>
                    <div class="space-y-2">
                      <Label for={`deliverable-content-${id}`}>{copy.deliverableContent}</Label>
                      <Textarea id={`deliverable-content-${id}`} bind:value={deliverableDrafts[id].content} rows={10} required />
                    </div>
                  </fieldset>
                {/each}
                <Button type="submit" disabled={busy}>{copy.composeDeliverables}</Button>
              </form>
            </Card.Content>
          </Card.Root>
        {:else if selected.snapshot.deliverables.every((item) => item.state === "review-required")}
          <Card.Root>
            <Card.Header>
              <Card.Title>{copy.reviewDeliverables}</Card.Title>
              <Card.Description>{copy.privateReadConsent}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              <div class="flex items-start gap-3">
                <Checkbox id="generic-review-consent" bind:checked={privateReviewConsent} />
                <Label for="generic-review-consent" class="font-normal">{copy.reviewConsentLabel}</Label>
              </div>
              <Button variant="outline" disabled={busy || !privateReviewConsent} onclick={loadReview}>
                {copy.loadPrivateReview}
              </Button>
              {#if review}
                {#each review.deliverables as item (item.deliverable.id)}
                  <article class="space-y-2 rounded-md border p-4">
                    <h3 class="font-semibold">{item.deliverable.title}</h3>
                    <pre class="max-h-80 overflow-auto whitespace-pre-wrap break-words rounded-md bg-muted p-3 text-sm">{item.content}</pre>
                  </article>
                {/each}
                <div class="flex items-start gap-3">
                  <Checkbox id="generic-review-complete" bind:checked={reviewConfirmed} />
                  <Label for="generic-review-complete" class="font-normal">{copy.reviewedAllDeliverables}</Label>
                </div>
                <Button disabled={busy || !reviewConfirmed} onclick={submitApproval}>
                  {copy.approveDeliverables}
                </Button>
              {/if}
            </Card.Content>
          </Card.Root>
        {:else if selected.snapshot.deliverables.every((item) => item.state === "approved")}
          <Card.Root>
            <Card.Header>
              <Card.Title>{copy.exportApplication}</Card.Title>
              <Card.Description>{copy.submissionBoundary}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              <div class="space-y-2">
                <Label for="generic-export-destination">{copy.exportDestination}</Label>
                <Input id="generic-export-destination" bind:value={exportDestination} />
              </div>
              <div class="flex items-start gap-3">
                <Checkbox id="generic-export-consent" bind:checked={privateExportConsent} />
                <Label for="generic-export-consent" class="font-normal">{copy.privateExportConsent}</Label>
              </div>
              <Button disabled={busy || !privateExportConsent || !exportDestination.trim()} onclick={submitExport}>
                {copy.exportApplication}
              </Button>
            </Card.Content>
          </Card.Root>
        {/if}
      {:else}
        <Card.Root>
          <Card.Content>
            <Empty.Root class="min-h-48">
              <Empty.Header>
                <Empty.Media variant="icon"><FileCheck2 size={22} aria-hidden="true" /></Empty.Media>
                <Empty.Title>{copy.noGenericApplications}</Empty.Title>
              </Empty.Header>
            </Empty.Root>
          </Card.Content>
        </Card.Root>
      {/if}
    </div>
  </Page.Grid>
</Page.Root>
