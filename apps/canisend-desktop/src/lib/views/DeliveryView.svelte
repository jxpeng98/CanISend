<script lang="ts">
  import {
    Archive,
    CheckCircle2,
    FileCheck2,
    FileOutput,
    FileText,
    RefreshCw,
    RotateCcw,
    ShieldCheck,
  } from "@lucide/svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";
  import type {
    DocumentWorkspaceReadModel,
    PackageExportManifestRecord,
    PackageManifestRecord,
    ProjectionReconcileRecord,
    RenderManifestRecord,
    ReviewWorkspaceReadModel,
    WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";
  import type {
    WorkflowDetail,
    WorkflowRoute,
  } from "$lib/workflow-navigation";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    activeWorkspace: WorkspaceReadModel | null;
    selectedJobId: string;
    focus: WorkflowDetail | null;
    busy: boolean;
    onNavigate: (route: WorkflowRoute) => Promise<void>;
    onLoadDocuments: (
      jobId: string,
      confirmedPrivateRead: boolean,
    ) => Promise<DocumentWorkspaceReadModel | null>;
    onLoadReview: (
      jobId: string,
      confirmedPrivateRead: boolean,
    ) => Promise<ReviewWorkspaceReadModel | null>;
    onConfirmReview: (
      jobId: string,
      candidate: unknown,
      confirmedPrivateRead: boolean,
    ) => Promise<ReviewWorkspaceReadModel | null>;
    onCheckPackage: (jobId: string) => Promise<PackageManifestRecord | null>;
    onLoadPackage: (jobId: string) => Promise<PackageManifestRecord | null>;
    onExportPackage: (
      jobId: string,
      destination: string,
      confirmedPrivateExport: boolean,
    ) => Promise<PackageExportManifestRecord | null>;
    onLoadPackageExport: (
      jobId: string,
    ) => Promise<PackageExportManifestRecord | null>;
    onReconcilePackage: (
      jobId: string,
    ) => Promise<ProjectionReconcileRecord[] | null>;
    onReplaceProjection: (
      jobId: string,
      path: string,
    ) => Promise<ProjectionReconcileRecord | null>;
    onCopyProjection: (
      jobId: string,
      path: string,
      destination: string,
    ) => Promise<ProjectionReconcileRecord | null>;
    onBuildRender: (jobId: string) => Promise<RenderManifestRecord | null>;
    onLoadRender: (jobId: string) => Promise<RenderManifestRecord | null>;
    onExportRender: (
      jobId: string,
      destination: string,
      confirmedPrivateExport: boolean,
    ) => Promise<boolean>;
  };

  let {
    copy,
    desktopRuntime,
    activeWorkspace,
    selectedJobId,
    focus,
    busy,
    onNavigate,
    onLoadDocuments,
    onLoadReview,
    onConfirmReview,
    onCheckPackage,
    onLoadPackage,
    onExportPackage,
    onLoadPackageExport,
    onReconcilePackage,
    onReplaceProjection,
    onCopyProjection,
    onBuildRender,
    onLoadRender,
    onExportRender,
  }: Props = $props();

  let section = $state("documents");
  let loadedKey = $state("");
  let privateReadConsent = $state(false);
  let privateExportConsent = $state(false);
  let documents = $state<DocumentWorkspaceReadModel | null>(null);
  let review = $state<ReviewWorkspaceReadModel | null>(null);
  let reviewJson = $state("");
  let packageManifest = $state<PackageManifestRecord | null>(null);
  let packageExport = $state<PackageExportManifestRecord | null>(null);
  let reconciliations = $state<ProjectionReconcileRecord[]>([]);
  let packageDestination = $state("");
  let selectedProjectionPath = $state("");
  let preservedDestination = $state("");
  let renderManifest = $state<RenderManifestRecord | null>(null);
  let renderDestination = $state("");
  let formError = $state<string | null>(null);

  $effect(() => {
    const nextKey = `${activeWorkspace?.path ?? ""}:${selectedJobId}`;
    if (nextKey !== loadedKey) {
      loadedKey = nextKey;
      documents = null;
      review = null;
      reviewJson = "";
      packageManifest = null;
      packageExport = null;
      reconciliations = [];
      renderManifest = null;
      privateReadConsent = false;
      privateExportConsent = false;
      packageDestination = selectedJobId
        ? `jobs/${selectedJobId}/application`
        : "";
      renderDestination = selectedJobId
        ? `jobs/${selectedJobId}/rendered`
        : "";
      selectedProjectionPath = "";
      preservedDestination = "";
    }
  });

  $effect(() => {
    if (focus === "delivery-review") section = "review";
    if (focus === "delivery-package") section = "package";
    if (focus === "delivery-render") section = "render";
    if (focus === "delivery-documents") section = "documents";
  });

  async function loadDocuments(): Promise<void> {
    formError = null;
    if (!privateReadConsent) {
      formError = copy.privateWorkspaceConsent;
      return;
    }
    documents = await onLoadDocuments(selectedJobId, privateReadConsent);
  }

  async function loadReview(): Promise<void> {
    formError = null;
    if (!privateReadConsent) {
      formError = copy.privateWorkspaceConsent;
      return;
    }
    review = await onLoadReview(selectedJobId, privateReadConsent);
    if (review) {
      reviewJson = JSON.stringify(review.disposition_candidate, null, 2);
    }
  }

  async function confirmReview(): Promise<void> {
    formError = null;
    if (!privateReadConsent) {
      formError = copy.privateWorkspaceConsent;
      return;
    }
    try {
      const candidate: unknown = JSON.parse(reviewJson);
      review = await onConfirmReview(
        selectedJobId,
        candidate,
        privateReadConsent,
      );
      if (review) {
        reviewJson = JSON.stringify(review.disposition_candidate, null, 2);
      }
    } catch {
      formError = copy.invalidJson;
    }
  }

  async function exportPackage(): Promise<void> {
    formError = null;
    if (!privateExportConsent) {
      formError = copy.privateExportConsent;
      return;
    }
    packageExport = await onExportPackage(
      selectedJobId,
      packageDestination,
      privateExportConsent,
    );
  }

  async function reconcile(): Promise<void> {
    reconciliations = (await onReconcilePackage(selectedJobId)) ?? [];
    if (reconciliations.length && !selectedProjectionPath) {
      selectProjection(reconciliations[0].projection.relative_path);
    }
  }

  function selectProjection(path: string): void {
    selectedProjectionPath = path;
    const suffix = path.includes(".")
      ? path.replace(/(\.[^./]+)$/, "-edited$1")
      : `${path}-edited`;
    preservedDestination = suffix;
  }

  async function replaceProjection(): Promise<void> {
    if (!selectedProjectionPath) return;
    const result = await onReplaceProjection(
      selectedJobId,
      selectedProjectionPath,
    );
    if (result) await reconcile();
  }

  async function copyProjection(): Promise<void> {
    if (!selectedProjectionPath || !preservedDestination) return;
    const result = await onCopyProjection(
      selectedJobId,
      selectedProjectionPath,
      preservedDestination,
    );
    if (result) await reconcile();
  }

  async function exportRender(): Promise<void> {
    formError = null;
    if (!privateExportConsent) {
      formError = copy.privateExportConsent;
      return;
    }
    await onExportRender(
      selectedJobId,
      renderDestination,
      privateExportConsent,
    );
  }

  function navigateWithinDelivery(detail: WorkflowDetail): void {
    void onNavigate({
      view: "delivery",
      detail,
      jobId: selectedJobId || undefined,
    });
  }

  const workspaceTitle = $derived(
    section === "documents"
      ? copy.materialsWorkspaceTitle
      : copy.reviewExportTitle,
  );
  const workspaceDescription = $derived(
    section === "documents"
      ? copy.materialsWorkspaceDescription
      : copy.reviewExportDescription,
  );
</script>

<section class="space-y-6">
  <div class="flex flex-col justify-between gap-4 xl:flex-row xl:items-end">
    <div>
      <Badge variant="secondary" class="mb-3">{copy.applicationWorkspace}</Badge>
      <h1 class="text-3xl font-semibold tracking-[-0.03em]">{workspaceTitle}</h1>
      <p class="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">
        {workspaceDescription}
      </p>
    </div>
  </div>

  {#if !activeWorkspace || !selectedJobId}
    <Card.Root class="shadow-none">
      <Card.Content class="flex min-h-80 flex-col items-center justify-center px-8 text-center">
        <Archive size={24} strokeWidth={1.8} class="text-muted-foreground" aria-hidden="true" />
        <h2 class="mt-4 text-base font-semibold">
          {activeWorkspace ? copy.noApplications : copy.noWorkspace}
        </h2>
      </Card.Content>
    </Card.Root>
  {:else}
    <div class="flex flex-wrap gap-3 rounded-xl border bg-muted/20 p-3">
      <div class="flex min-w-[300px] flex-1 items-start gap-3">
        <Checkbox id="delivery-private-read" bind:checked={privateReadConsent} class="mt-0.5" />
        <Label for="delivery-private-read" class="text-xs leading-5 font-normal">
          <span class="flex items-center gap-2">
            <ShieldCheck size={14} strokeWidth={1.8} aria-hidden="true" />
            {copy.privateWorkspaceConsent}
          </span>
        </Label>
      </div>
      <div class="flex min-w-[300px] flex-1 items-start gap-3">
        <Checkbox id="delivery-private-export" bind:checked={privateExportConsent} class="mt-0.5" />
        <Label for="delivery-private-export" class="text-xs leading-5 font-normal">
          {copy.privateExportConsent}
        </Label>
      </div>
    </div>

    <Tabs.Root bind:value={section}>
      <Tabs.List class="grid w-full max-w-3xl grid-cols-4">
        <Tabs.Trigger
          value="documents"
          onclick={() => navigateWithinDelivery("delivery-documents")}
        >
          {copy.documents}
        </Tabs.Trigger>
        <Tabs.Trigger
          value="review"
          onclick={() => navigateWithinDelivery("delivery-review")}
        >
          {copy.review}
        </Tabs.Trigger>
        <Tabs.Trigger
          value="package"
          onclick={() => navigateWithinDelivery("delivery-package")}
        >
          {copy.package}
        </Tabs.Trigger>
        <Tabs.Trigger
          value="render"
          onclick={() => navigateWithinDelivery("delivery-render")}
        >
          {copy.render}
        </Tabs.Trigger>
      </Tabs.List>

      <Tabs.Content
        id="delivery-documents"
        value="documents"
        class={[
          "scroll-mt-64 pt-4",
          focus === "delivery-documents" ? "rounded-xl ring-2 ring-primary/25" : "",
        ]}
      >
        <Card.Root class="shadow-none">
          <Card.Header>
            <div class="flex items-start justify-between gap-4">
              <div>
                <Card.Title>{copy.documentWorkspace}</Card.Title>
                <Card.Description class="mt-1.5">
                  {documents?.acceptance_blocker ?? copy.acceptedDocumentSet}
                </Card.Description>
              </div>
              <Button
                variant="outline"
                class="min-h-11"
                disabled={busy || !privateReadConsent}
                onclick={loadDocuments}
              >
                <RefreshCw size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                {copy.loadDocuments}
              </Button>
            </div>
          </Card.Header>
          <Card.Content>
            <div class="grid gap-3 lg:grid-cols-2">
              {#each documents?.documents ?? [] as document (document.id)}
                <div class="rounded-xl border p-4">
                  <div class="flex items-start justify-between gap-3">
                    <div>
                      <h2 class="text-sm font-semibold">{document.title}</h2>
                      <p class="mt-1 text-xs text-muted-foreground">{document.kind}</p>
                    </div>
                    <Badge variant="outline">r{document.revision}</Badge>
                  </div>
                  <p class="mt-4 text-xs text-muted-foreground">
                    {document.sections.length} {copy.sections} · {document.placeholders.length} {copy.placeholders}
                  </p>
                </div>
              {:else}
                <div class="col-span-full flex min-h-64 flex-col items-center justify-center rounded-xl border border-dashed text-center">
                  <FileText size={22} strokeWidth={1.8} class="text-muted-foreground" aria-hidden="true" />
                  <p class="mt-3 text-sm text-muted-foreground">{copy.noDocuments}</p>
                </div>
              {/each}
            </div>
          </Card.Content>
        </Card.Root>
      </Tabs.Content>

      <Tabs.Content
        id="delivery-review"
        value="review"
        class={[
          "scroll-mt-64 pt-4",
          focus === "delivery-review" ? "rounded-xl ring-2 ring-primary/25" : "",
        ]}
      >
        <div class="grid gap-6 xl:grid-cols-[minmax(320px,0.75fr)_minmax(0,1.25fr)]">
          <Card.Root class="shadow-none">
            <Card.Header>
              <Card.Title>{copy.reviewFindings}</Card.Title>
              <Card.Description>{review?.current.findings.length ?? 0}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-3">
              <Button
                variant="outline"
                class="min-h-11"
                disabled={busy || !privateReadConsent}
                onclick={loadReview}
              >
                {copy.loadReview}
              </Button>
              {#each review?.current.findings ?? [] as finding (finding.id)}
                <div class="rounded-xl border p-4">
                  <div class="flex items-center justify-between gap-3">
                    <p class="text-xs font-semibold">{finding.code}</p>
                    <Badge variant={finding.severity === "blocker" ? "destructive" : "outline"}>
                      {finding.severity}
                    </Badge>
                  </div>
                  <p class="mt-2 text-sm leading-6">{finding.message}</p>
                  <p class="mt-2 text-xs text-muted-foreground">
                    {finding.authority} · {finding.status}
                  </p>
                </div>
              {:else}
                <p class="rounded-xl border border-dashed p-5 text-center text-sm text-muted-foreground">
                  {copy.noFindings}
                </p>
              {/each}
            </Card.Content>
          </Card.Root>

          <Card.Root class="shadow-none">
            <Card.Header>
              <Card.Title>{copy.dispositionCandidate}</Card.Title>
              <Card.Description>{copy.reviewBeforeCommitDescription}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              <Textarea
                class="min-h-[460px] resize-y font-mono text-xs leading-5"
                bind:value={reviewJson}
                spellcheck={false}
                disabled={!review}
              />
              <Button
                class="min-h-11"
                disabled={busy || !review || !reviewJson || !privateReadConsent}
                onclick={confirmReview}
              >
                {copy.confirmDispositions}
              </Button>
            </Card.Content>
          </Card.Root>
        </div>
      </Tabs.Content>

      <Tabs.Content
        id="delivery-package"
        value="package"
        class={[
          "scroll-mt-64 space-y-6 pt-4",
          focus === "delivery-package" ? "rounded-xl ring-2 ring-primary/25" : "",
        ]}
      >
        <Card.Root class="shadow-none">
          <Card.Header>
            <Card.Title>{copy.readiness}</Card.Title>
            <Card.Description>
              {packageManifest?.readiness.state ?? copy.checkPackage}
            </Card.Description>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div class="flex flex-wrap gap-2">
              <Button disabled={busy} onclick={async () => (packageManifest = await onCheckPackage(selectedJobId))}>
                <CheckCircle2 size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                {copy.checkPackage}
              </Button>
              <Button variant="outline" disabled={busy} onclick={async () => (packageManifest = await onLoadPackage(selectedJobId))}>
                {copy.loadPackage}
              </Button>
            </div>
            {#if packageManifest}
              <div class="grid gap-3 md:grid-cols-3">
                <div class="rounded-xl border p-4">
                  <p class="text-xs text-muted-foreground">{copy.readiness}</p>
                  <p class="mt-2 text-lg font-semibold">{packageManifest.readiness.state}</p>
                </div>
                <div class="rounded-xl border p-4">
                  <p class="text-xs text-muted-foreground">{copy.documents}</p>
                  <p class="mt-2 text-lg font-semibold">{packageManifest.documents.length}</p>
                </div>
                <div class="rounded-xl border p-4">
                  <p class="text-xs text-muted-foreground">{copy.readinessReasons}</p>
                  <p class="mt-2 text-lg font-semibold">{packageManifest.readiness.reasons.length}</p>
                </div>
              </div>
            {/if}
            <Separator />
            <div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
              <div class="space-y-2">
                <Label for="package-destination">{copy.relativeDestination}</Label>
                <Input id="package-destination" bind:value={packageDestination} />
              </div>
              <Button
                class="min-h-11"
                disabled={!desktopRuntime || busy || !privateExportConsent || !packageDestination}
                onclick={exportPackage}
              >
                <FileOutput size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                {copy.exportPackage}
              </Button>
            </div>
          </Card.Content>
        </Card.Root>

        <Card.Root class="shadow-none">
          <Card.Header>
            <div class="flex items-start justify-between gap-4">
              <div>
                <Card.Title>{copy.projections}</Card.Title>
                <Card.Description>{packageExport?.projections.length ?? reconciliations.length}</Card.Description>
              </div>
              <div class="flex flex-wrap gap-2">
                <Button variant="outline" disabled={busy} onclick={async () => (packageExport = await onLoadPackageExport(selectedJobId))}>
                  {copy.loadExports}
                </Button>
                <Button variant="outline" disabled={busy} onclick={reconcile}>
                  <RotateCcw size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                  {copy.reconcileProjections}
                </Button>
              </div>
            </div>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div class="grid gap-2 lg:grid-cols-2">
              {#each reconciliations as record (record.projection.relative_path)}
                <button
                  type="button"
                  class={`rounded-xl border p-4 text-left ${
                    selectedProjectionPath === record.projection.relative_path ? "border-primary bg-accent/45" : ""
                  }`}
                  onclick={() => selectProjection(record.projection.relative_path)}
                >
                  <div class="flex items-center justify-between gap-3">
                    <p class="truncate font-mono text-xs">{record.projection.relative_path}</p>
                    <Badge variant={record.projection.edit_status === "current" ? "outline" : "destructive"}>
                      {record.projection.edit_status}
                    </Badge>
                  </div>
                </button>
              {/each}
            </div>
            {#if selectedProjectionPath}
              <div class="rounded-xl border bg-muted/20 p-4">
                <p class="break-all font-mono text-xs">{selectedProjectionPath}</p>
                <div class="mt-4 space-y-2">
                  <Label for="preserved-destination">{copy.preservedDestination}</Label>
                  <Input id="preserved-destination" bind:value={preservedDestination} />
                </div>
                <div class="mt-4 flex flex-wrap gap-2">
                  <Button variant="outline" disabled={busy} onclick={replaceProjection}>
                    {copy.replaceProjection}
                  </Button>
                  <Button
                    variant="outline"
                    disabled={busy || !preservedDestination}
                    onclick={copyProjection}
                  >
                    {copy.copyAsNew}
                  </Button>
                </div>
              </div>
            {/if}
          </Card.Content>
        </Card.Root>
      </Tabs.Content>

      <Tabs.Content
        id="delivery-render"
        value="render"
        class={[
          "scroll-mt-64 pt-4",
          focus === "delivery-render" ? "rounded-xl ring-2 ring-primary/25" : "",
        ]}
      >
        <Card.Root class="shadow-none">
          <Card.Header>
            <Card.Title>{copy.render}</Card.Title>
            <Card.Description>{renderManifest?.rendered_at ?? copy.buildRender}</Card.Description>
          </Card.Header>
          <Card.Content class="space-y-5">
            <div class="flex flex-wrap gap-2">
              <Button disabled={busy} onclick={async () => (renderManifest = await onBuildRender(selectedJobId))}>
                <FileCheck2 size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                {copy.buildRender}
              </Button>
              <Button variant="outline" disabled={busy} onclick={async () => (renderManifest = await onLoadRender(selectedJobId))}>
                {copy.loadRender}
              </Button>
            </div>
            <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              {#each renderManifest?.documents ?? [] as document (document.kind)}
                <div class="rounded-xl border p-4">
                  <p class="text-sm font-semibold">{document.kind}</p>
                  <p class="mt-2 text-xs text-muted-foreground">
                    {document.page_count} {copy.pages} · {document.warning_count} {copy.warnings}
                  </p>
                </div>
              {/each}
            </div>
            <Separator />
            <div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
              <div class="space-y-2">
                <Label for="render-destination">{copy.relativeDestination}</Label>
                <Input id="render-destination" bind:value={renderDestination} />
              </div>
              <Button
                class="min-h-11"
                disabled={!desktopRuntime || busy || !privateExportConsent || !renderDestination}
                onclick={exportRender}
              >
                {copy.exportRender}
              </Button>
            </div>
          </Card.Content>
        </Card.Root>
      </Tabs.Content>
    </Tabs.Root>
  {/if}

  {#if formError}
    <p class="text-sm text-destructive" role="alert">{formError}</p>
  {/if}
</section>
