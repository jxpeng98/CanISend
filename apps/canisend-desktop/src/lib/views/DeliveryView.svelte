<script lang="ts">
  import {
    Archive,
    CheckCircle2,
    ExternalLink,
    FileCheck2,
    FileOutput,
    FileText,
    Eye,
    RefreshCw,
    RotateCcw,
    ShieldCheck,
  } from "@lucide/svelte";
  import { onDestroy } from "svelte";

  import ActionMenu from "$lib/components/patterns/ActionMenu.svelte";
  import * as Page from "$lib/components/patterns/page/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import * as Alert from "$lib/components/ui/alert/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import * as Empty from "$lib/components/ui/empty/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";
  import type {
    DocumentWorkspaceReadModel,
    DocumentKind,
    PackageExportManifestRecord,
    PackageManifestRecord,
    ProjectionReconcileRecord,
    RenderManifestRecord,
    RenderedDocumentRecord,
    ReviewWorkspaceReadModel,
    WorkflowPackPresentationReadModel,
    WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";
  import type { WorkflowDetail, WorkflowRoute } from "$lib/workflow-navigation";
  import { deliverablePresentationLabel } from "$lib/workflow-pack-presentation";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    activeWorkspace: WorkspaceReadModel | null;
    selectedJobId: string;
    presentation: WorkflowPackPresentationReadModel | null;
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
    onLoadPackageExport: (jobId: string) => Promise<PackageExportManifestRecord | null>;
    onReconcilePackage: (jobId: string) => Promise<ProjectionReconcileRecord[] | null>;
    onReplaceProjection: (jobId: string, path: string) => Promise<ProjectionReconcileRecord | null>;
    onCopyProjection: (
      jobId: string,
      path: string,
      destination: string,
    ) => Promise<ProjectionReconcileRecord | null>;
    onBuildRender: (jobId: string) => Promise<RenderManifestRecord | null>;
    onLoadRender: (jobId: string) => Promise<RenderManifestRecord | null>;
    onPreviewRender: (
      jobId: string,
      kind: DocumentKind,
      confirmedPrivateRead: boolean,
    ) => Promise<Uint8Array | null>;
    onExportRender: (
      jobId: string,
      destination: string,
      confirmedPrivateExport: boolean,
    ) => Promise<boolean>;
    onOpenRender: (
      jobId: string,
      destination: string,
      kind: DocumentKind,
      confirmedPrivateExport: boolean,
    ) => Promise<boolean>;
  };

  let {
    copy,
    desktopRuntime,
    activeWorkspace,
    selectedJobId,
    presentation,
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
    onPreviewRender,
    onExportRender,
    onOpenRender,
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
  let previewUrl = $state<string | null>(null);
  let previewKind = $state<DocumentKind | null>(null);
  let previewSha256 = $state<string | null>(null);
  let formError = $state<string | null>(null);

  const previewDocument = $derived(
    renderManifest?.documents.find(
      (document) => document.kind === previewKind && document.pdf_artifact.sha256 === previewSha256,
    ) ?? null,
  );

  function clearRenderPreview(): void {
    if (previewUrl) URL.revokeObjectURL(previewUrl);
    previewUrl = null;
    previewKind = null;
    previewSha256 = null;
  }

  onDestroy(clearRenderPreview);

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
      clearRenderPreview();
      privateReadConsent = false;
      privateExportConsent = false;
      packageDestination = selectedJobId ? `jobs/${selectedJobId}/application` : "";
      renderDestination = selectedJobId ? `jobs/${selectedJobId}/rendered` : "";
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
      review = await onConfirmReview(selectedJobId, candidate, privateReadConsent);
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
    packageExport = await onExportPackage(selectedJobId, packageDestination, privateExportConsent);
  }

  async function reconcile(): Promise<void> {
    reconciliations = (await onReconcilePackage(selectedJobId)) ?? [];
    if (reconciliations.length && !selectedProjectionPath) {
      selectProjection(reconciliations[0].projection.relative_path);
    }
  }

  function selectProjection(path: string): void {
    selectedProjectionPath = path;
    const suffix = path.includes(".") ? path.replace(/(\.[^./]+)$/, "-edited$1") : `${path}-edited`;
    preservedDestination = suffix;
  }

  async function replaceProjection(): Promise<void> {
    if (!selectedProjectionPath) return;
    const result = await onReplaceProjection(selectedJobId, selectedProjectionPath);
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
    await onExportRender(selectedJobId, renderDestination, privateExportConsent);
  }

  async function openRenderedPdf(): Promise<void> {
    formError = null;
    if (!previewDocument) return;
    if (!privateExportConsent) {
      formError = copy.privateExportConsent;
      return;
    }
    await onOpenRender(
      selectedJobId,
      renderDestination,
      previewDocument.kind,
      privateExportConsent,
    );
  }

  async function buildRenderedPdfs(): Promise<void> {
    clearRenderPreview();
    renderManifest = await onBuildRender(selectedJobId);
  }

  async function loadRenderedPdfs(): Promise<void> {
    clearRenderPreview();
    renderManifest = await onLoadRender(selectedJobId);
  }

  async function showRenderPreview(document: RenderedDocumentRecord): Promise<void> {
    formError = null;
    if (!privateReadConsent) {
      formError = copy.privateWorkspaceConsent;
      return;
    }
    const bytes = await onPreviewRender(selectedJobId, document.kind, privateReadConsent);
    if (!bytes) return;
    clearRenderPreview();
    const buffer = bytes.buffer.slice(
      bytes.byteOffset,
      bytes.byteOffset + bytes.byteLength,
    ) as ArrayBuffer;
    previewUrl = URL.createObjectURL(new Blob([buffer], { type: "application/pdf" }));
    previewKind = document.kind;
    previewSha256 = document.pdf_artifact.sha256;
  }

  function navigateWithinDelivery(detail: WorkflowDetail): void {
    void onNavigate({
      view: "delivery",
      detail,
      jobId: selectedJobId || undefined,
    });
  }

  const workspaceTitle = $derived(
    section === "documents" ? copy.materialsWorkspaceTitle : copy.reviewExportTitle,
  );
  const workspaceDescription = $derived(
    section === "documents" ? copy.materialsWorkspaceDescription : copy.reviewExportDescription,
  );
</script>

<Page.Root>
  <Page.Header
    eyebrow={copy.applicationWorkspace}
    title={workspaceTitle}
    description={workspaceDescription}
  />

  {#if !activeWorkspace || !selectedJobId}
    <Card.Root>
      <Card.Content>
        <Empty.Root class="min-h-32">
          <Empty.Header>
            <Empty.Media variant="icon">
              <Archive size={24} strokeWidth={1.8} aria-hidden="true" />
            </Empty.Media>
            <Empty.Title class="text-base">
              {activeWorkspace ? copy.noApplications : copy.noWorkspace}
            </Empty.Title>
          </Empty.Header>
        </Empty.Root>
      </Card.Content>
    </Card.Root>
  {:else}
    <div class="flex flex-wrap gap-3 rounded-lg border bg-muted/20 p-3">
      <div class="flex min-w-0 flex-[1_1_20rem] items-start gap-3">
        <Checkbox id="delivery-private-read" bind:checked={privateReadConsent} class="mt-0.5" />
        <Label for="delivery-private-read" class="text-xs leading-5 font-normal">
          <span class="flex items-center gap-2">
            <ShieldCheck size={14} strokeWidth={1.8} aria-hidden="true" />
            {copy.privateWorkspaceConsent}
          </span>
        </Label>
      </div>
      <div class="flex min-w-0 flex-[1_1_20rem] items-start gap-3">
        <Checkbox id="delivery-private-export" bind:checked={privateExportConsent} class="mt-0.5" />
        <Label for="delivery-private-export" class="text-xs leading-5 font-normal">
          {copy.privateExportConsent}
        </Label>
      </div>
    </div>

    <Tabs.Root bind:value={section}>
      <Tabs.List class="responsive-tabs max-w-3xl" data-columns="4">
        <Tabs.Trigger
          value="documents"
          onclick={() => navigateWithinDelivery("delivery-documents")}
        >
          {copy.documents}
        </Tabs.Trigger>
        <Tabs.Trigger value="review" onclick={() => navigateWithinDelivery("delivery-review")}>
          {copy.review}
        </Tabs.Trigger>
        <Tabs.Trigger value="package" onclick={() => navigateWithinDelivery("delivery-package")}>
          {copy.package}
        </Tabs.Trigger>
        <Tabs.Trigger value="render" onclick={() => navigateWithinDelivery("delivery-render")}>
          {copy.render}
        </Tabs.Trigger>
      </Tabs.List>

      <Tabs.Content
        id="delivery-documents"
        value="documents"
        class={[
          "scroll-mt-64 pt-[var(--density-section-gap)]",
          focus === "delivery-documents" ? "rounded-lg ring-2 ring-primary/25" : "",
        ]}
      >
        <Card.Root>
          <Card.Header>
            <div class="flex items-start justify-between gap-[var(--density-section-gap)]">
              <div>
                <Card.Title>{copy.documentWorkspace}</Card.Title>
                <Card.Description class="mt-1.5">
                  {documents?.acceptance_blocker ?? copy.acceptedDocumentSet}
                </Card.Description>
              </div>
              <Button
                variant="outline"
                class="min-h-9"
                disabled={busy || !privateReadConsent}
                onclick={loadDocuments}
              >
                <RefreshCw
                  size={16}
                  strokeWidth={1.8}
                  data-icon="inline-start"
                  aria-hidden="true"
                />
                {copy.loadDocuments}
              </Button>
            </div>
          </Card.Header>
          <Card.Content>
            <div class="grid gap-3 lg:grid-cols-2">
              {#each documents?.documents ?? [] as document (document.id)}
                <div class="rounded-lg border p-[var(--density-panel-padding)]">
                  <div class="flex items-start justify-between gap-3">
                    <div>
                      <h2 class="text-sm font-semibold">{document.title}</h2>
                      <p class="mt-1 text-xs text-muted-foreground">
                        {deliverablePresentationLabel(presentation, document.kind)}
                      </p>
                    </div>
                    <Badge variant="outline">r{document.revision}</Badge>
                  </div>
                  <p class="mt-[var(--density-section-gap)] text-xs text-muted-foreground">
                    {document.sections.length}
                    {copy.sections} · {document.placeholders.length}
                    {copy.placeholders}
                  </p>
                </div>
              {:else}
                <Empty.Root class="col-span-full min-h-32 border">
                  <Empty.Header>
                    <Empty.Media variant="icon">
                      <FileText size={22} strokeWidth={1.8} aria-hidden="true" />
                    </Empty.Media>
                    <Empty.Description>{copy.noDocuments}</Empty.Description>
                  </Empty.Header>
                </Empty.Root>
              {/each}
            </div>
          </Card.Content>
        </Card.Root>
      </Tabs.Content>

      <Tabs.Content
        id="delivery-review"
        value="review"
        class={[
          "scroll-mt-64 pt-[var(--density-section-gap)]",
          focus === "delivery-review" ? "rounded-lg ring-2 ring-primary/25" : "",
        ]}
      >
        <div
          class="grid gap-[var(--density-section-gap)] xl:grid-cols-[minmax(320px,0.75fr)_minmax(0,1.25fr)]"
        >
          <Card.Root>
            <Card.Header>
              <Card.Title>{copy.reviewFindings}</Card.Title>
              <Card.Description>{review?.current.findings.length ?? 0}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-3">
              <Button
                variant="outline"
                class="min-h-9"
                disabled={busy || !privateReadConsent}
                onclick={loadReview}
              >
                {copy.loadReview}
              </Button>
              {#each review?.current.findings ?? [] as finding (finding.id)}
                <div class="rounded-lg border p-[var(--density-panel-padding)]">
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
                <Empty.Root class="min-h-20 border">
                  <Empty.Header><Empty.Title>{copy.noFindings}</Empty.Title></Empty.Header>
                </Empty.Root>
              {/each}
            </Card.Content>
          </Card.Root>

          <Card.Root>
            <Card.Header>
              <Card.Title>{copy.dispositionCandidate}</Card.Title>
              <Card.Description>{copy.reviewBeforeCommitDescription}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-[var(--density-section-gap)]">
              <Textarea
                class="min-h-[320px] resize-y font-mono text-xs leading-5"
                bind:value={reviewJson}
                spellcheck={false}
                disabled={!review}
              />
              <Button
                class="min-h-9"
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
          "scroll-mt-64 space-y-[var(--density-section-gap)] pt-[var(--density-section-gap)]",
          focus === "delivery-package" ? "rounded-lg ring-2 ring-primary/25" : "",
        ]}
      >
        <Card.Root>
          <Card.Header>
            <Card.Title>{copy.readiness}</Card.Title>
            <Card.Description>
              {packageManifest?.readiness.state ?? copy.checkPackage}
            </Card.Description>
          </Card.Header>
          <Card.Content class="space-y-[var(--density-section-gap)]">
            <div class="flex flex-wrap gap-2">
              <Button
                disabled={busy}
                onclick={async () => (packageManifest = await onCheckPackage(selectedJobId))}
              >
                <CheckCircle2
                  size={16}
                  strokeWidth={1.8}
                  data-icon="inline-start"
                  aria-hidden="true"
                />
                {copy.checkPackage}
              </Button>
              <ActionMenu label={copy.moreActions} disabled={busy}>
                <DropdownMenu.Item
                  onclick={async () => (packageManifest = await onLoadPackage(selectedJobId))}
                >
                  <RefreshCw size={16} strokeWidth={1.8} aria-hidden="true" />
                  {copy.loadPackage}
                </DropdownMenu.Item>
              </ActionMenu>
            </div>
            {#if packageManifest}
              <div class="grid gap-3 md:grid-cols-3">
                <div class="rounded-lg border p-[var(--density-panel-padding)]">
                  <p class="text-xs text-muted-foreground">{copy.readiness}</p>
                  <p class="mt-2 text-lg font-semibold">{packageManifest.readiness.state}</p>
                </div>
                <div class="rounded-lg border p-[var(--density-panel-padding)]">
                  <p class="text-xs text-muted-foreground">{copy.documents}</p>
                  <p class="mt-2 text-lg font-semibold">{packageManifest.documents.length}</p>
                </div>
                <div class="rounded-lg border p-[var(--density-panel-padding)]">
                  <p class="text-xs text-muted-foreground">{copy.readinessReasons}</p>
                  <p class="mt-2 text-lg font-semibold">
                    {packageManifest.readiness.reasons.length}
                  </p>
                </div>
              </div>
            {/if}
            <Separator />
            <div
              class="grid gap-[var(--density-section-gap)] lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end"
            >
              <div class="space-y-2">
                <Label for="package-destination">{copy.relativeDestination}</Label>
                <Input id="package-destination" bind:value={packageDestination} />
              </div>
              <Button
                class="min-h-9"
                disabled={!desktopRuntime || busy || !privateExportConsent || !packageDestination}
                onclick={exportPackage}
              >
                <FileOutput
                  size={16}
                  strokeWidth={1.8}
                  data-icon="inline-start"
                  aria-hidden="true"
                />
                {copy.exportPackage}
              </Button>
            </div>
          </Card.Content>
        </Card.Root>

        <Card.Root>
          <Card.Header>
            <div class="flex items-start justify-between gap-[var(--density-section-gap)]">
              <div>
                <Card.Title>{copy.projections}</Card.Title>
                <Card.Description
                  >{packageExport?.projections.length ?? reconciliations.length}</Card.Description
                >
              </div>
              <div class="flex flex-wrap items-center gap-2">
                <Button variant="outline" disabled={busy} onclick={reconcile}>
                  <RotateCcw
                    size={16}
                    strokeWidth={1.8}
                    data-icon="inline-start"
                    aria-hidden="true"
                  />
                  {copy.reconcileProjections}
                </Button>
                <ActionMenu label={copy.moreActions} disabled={busy}>
                  <DropdownMenu.Item
                    onclick={async () => (packageExport = await onLoadPackageExport(selectedJobId))}
                  >
                    <RefreshCw size={16} strokeWidth={1.8} aria-hidden="true" />
                    {copy.loadExports}
                  </DropdownMenu.Item>
                </ActionMenu>
              </div>
            </div>
          </Card.Header>
          <Card.Content class="space-y-[var(--density-section-gap)]">
            <div class="grid gap-2 lg:grid-cols-2">
              {#each reconciliations as record (record.projection.relative_path)}
                <Button
                  variant="outline"
                  class={[
                    "h-auto min-h-9 w-full justify-between p-[var(--density-panel-padding)] text-left",
                    selectedProjectionPath === record.projection.relative_path
                      ? "border-primary bg-accent/45"
                      : "",
                  ]}
                  onclick={() => selectProjection(record.projection.relative_path)}
                >
                  <div class="flex items-center justify-between gap-3">
                    <p class="truncate font-mono text-xs">{record.projection.relative_path}</p>
                    <Badge
                      variant={record.projection.edit_status === "current"
                        ? "outline"
                        : "destructive"}
                    >
                      {record.projection.edit_status}
                    </Badge>
                  </div>
                </Button>
              {/each}
            </div>
            {#if selectedProjectionPath}
              <div class="rounded-lg border bg-muted/20 p-[var(--density-panel-padding)]">
                <p class="break-all font-mono text-xs">{selectedProjectionPath}</p>
                <div class="mt-[var(--density-section-gap)] space-y-2">
                  <Label for="preserved-destination">{copy.preservedDestination}</Label>
                  <Input id="preserved-destination" bind:value={preservedDestination} />
                </div>
                <div class="mt-[var(--density-section-gap)] flex flex-wrap gap-2">
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
          "scroll-mt-64 pt-[var(--density-section-gap)]",
          focus === "delivery-render" ? "rounded-lg ring-2 ring-primary/25" : "",
        ]}
      >
        <Card.Root>
          <Card.Header>
            <Card.Title>{copy.render}</Card.Title>
            <Card.Description>{renderManifest?.rendered_at ?? copy.buildRender}</Card.Description>
          </Card.Header>
          <Card.Content class="space-y-[var(--density-section-gap)]">
            <div class="flex flex-wrap gap-2">
              <Button disabled={busy} onclick={buildRenderedPdfs}>
                <FileCheck2
                  size={16}
                  strokeWidth={1.8}
                  data-icon="inline-start"
                  aria-hidden="true"
                />
                {copy.buildRender}
              </Button>
              <ActionMenu label={copy.moreActions} disabled={busy}>
                <DropdownMenu.Item onclick={loadRenderedPdfs}>
                  <RefreshCw size={16} strokeWidth={1.8} aria-hidden="true" />
                  {copy.loadRender}
                </DropdownMenu.Item>
              </ActionMenu>
            </div>
            <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              {#each renderManifest?.documents ?? [] as document (document.kind)}
                <Button
                  variant={previewDocument?.kind === document.kind ? "secondary" : "outline"}
                  class="h-auto min-w-0 justify-between p-[var(--density-panel-padding)] text-left"
                  disabled={busy || !privateReadConsent}
                  aria-pressed={previewDocument?.kind === document.kind}
                  title={!privateReadConsent ? copy.privateWorkspaceConsent : copy.previewPdf}
                  onclick={() => showRenderPreview(document)}
                >
                  <span class="min-w-0">
                    <span class="block truncate text-sm font-semibold">
                      {deliverablePresentationLabel(presentation, document.kind)}
                    </span>
                    <span class="mt-1 block text-xs text-muted-foreground">
                      {document.page_count}
                      {copy.pages} · {document.warning_count}
                      {copy.warnings}
                    </span>
                  </span>
                  <Eye size={16} strokeWidth={1.8} aria-hidden="true" />
                  <span class="sr-only">{copy.previewPdf}</span>
                </Button>
              {/each}
            </div>
            {#if previewUrl && previewDocument}
              <section
                class="overflow-hidden rounded-lg border bg-muted/20"
                aria-label={copy.exactPdfPreview}
              >
                <div class="flex min-w-0 flex-wrap items-center gap-2 border-b px-3 py-2">
                  <Badge variant="secondary">
                    {deliverablePresentationLabel(presentation, previewDocument.kind)}
                  </Badge>
                  <span class="text-xs font-medium">{copy.exactPdfPreview}</span>
                  <span
                    class="min-w-0 flex-1 truncate text-right font-mono text-[10px] text-muted-foreground"
                    title={previewDocument.pdf_artifact.sha256}
                  >
                    {previewDocument.pdf_artifact.sha256}
                  </span>
                  <Button
                    variant="outline"
                    size="icon-sm"
                    disabled={!desktopRuntime ||
                      busy ||
                      !privateExportConsent ||
                      !renderDestination}
                    aria-label={copy.openSystemViewer}
                    title={copy.previewUnavailable}
                    onclick={openRenderedPdf}
                  >
                    <ExternalLink size={15} strokeWidth={1.8} aria-hidden="true" />
                  </Button>
                </div>
                <iframe
                  src={previewUrl}
                  title={`${copy.exactPdfPreview}: ${deliverablePresentationLabel(presentation, previewDocument.kind)}`}
                  class="h-[min(70vh,48rem)] min-h-[28rem] w-full bg-white"
                ></iframe>
              </section>
              <output class="sr-only" aria-live="polite">
                {copy.previewReady}: {deliverablePresentationLabel(
                  presentation,
                  previewDocument.kind,
                )}
              </output>
            {/if}
            <Separator />
            <div
              class="grid gap-[var(--density-section-gap)] lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end"
            >
              <div class="space-y-2">
                <Label for="render-destination">{copy.relativeDestination}</Label>
                <Input id="render-destination" bind:value={renderDestination} />
              </div>
              <Button
                class="min-h-9"
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
    <Alert.Root variant="destructive">
      <Alert.Description>{formError}</Alert.Description>
    </Alert.Root>
  {/if}
</Page.Root>
