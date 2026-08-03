<script lang="ts">
  import {
    ArrowUpRight,
    DatabaseZap,
    FileSearch,
    FileUp,
    Link,
    MapPin,
    Network,
    RefreshCw,
    Search,
  } from "@lucide/svelte";

  import * as Page from "$lib/components/patterns/page/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import * as Alert from "$lib/components/ui/alert/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import * as Empty from "$lib/components/ui/empty/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import * as NativeSelect from "$lib/components/ui/native-select/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import { Skeleton } from "$lib/components/ui/skeleton/index.js";
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import IntakeReviewSummary from "$lib/components/IntakeReviewSummary.svelte";
  import {
    chooseDiscoverySource,
    type DiscoveryAdapterCapabilities,
    type DiscoveryLeadRecord,
    type DiscoveryNetworkAdapter,
    type DiscoveryPreviewReadModel,
    type DiscoverySourceRecord,
    type DiscoverySuggestionReadModel,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    activeWorkspace: WorkspaceReadModel | null;
    adapters: DiscoveryAdapterCapabilities[];
    sources: DiscoverySourceRecord[];
    leads: DiscoveryLeadRecord[];
    selectedLead: DiscoveryLeadRecord | null;
    suggestions: DiscoverySuggestionReadModel | null;
    preview: DiscoveryPreviewReadModel | null;
    loading: boolean;
    busy: boolean;
    onRefresh: () => Promise<boolean>;
    onSelect: (leadId: string) => Promise<boolean>;
    onPreviewFile: (options: {
      path: string;
      sourceName?: string;
      sourceUrl?: string;
      hostAgent?: boolean;
      confirmedPrivateRead: boolean;
    }) => Promise<boolean>;
    onPreviewNetwork: (options: {
      adapter: DiscoveryNetworkAdapter;
      endpoint: string;
      sourceName: string;
      organization?: string;
      confirmedNetworkFetch: boolean;
    }) => Promise<boolean>;
    onCommitPreview: () => Promise<boolean>;
    onDiscardPreview: () => Promise<boolean>;
    onPromote: (leadId: string) => Promise<boolean>;
  };

  let {
    copy,
    desktopRuntime,
    activeWorkspace,
    adapters,
    sources,
    leads,
    selectedLead,
    suggestions,
    preview,
    loading,
    busy,
    onRefresh,
    onSelect,
    onPreviewFile,
    onPreviewNetwork,
    onCommitPreview,
    onDiscardPreview,
    onPromote,
  }: Props = $props();

  let intakeTab = $state("batch");
  let batchPath = $state("");
  let batchName = $state("");
  let batchUrl = $state("");
  let hostAgent = $state(false);
  let privateReadConfirmed = $state(false);
  let adapter = $state<DiscoveryNetworkAdapter>("rss-atom");
  let endpoint = $state("");
  let sourceName = $state("");
  let organization = $state("");
  let networkFetchConfirmed = $state(false);
  let formError = $state<string | null>(null);
  let promoteOpen = $state(false);

  const networkAdapters = $derived(adapters.filter((item) => item.network));

  async function chooseBatch(): Promise<void> {
    batchPath = (await chooseDiscoverySource()) ?? batchPath;
  }

  async function submitBatchPreview(): Promise<void> {
    formError = null;
    if (!batchPath) {
      formError = copy.chooseFile;
      return;
    }
    if (!privateReadConfirmed) {
      formError = copy.discoveryPrivateConsent;
      return;
    }
    await onPreviewFile({
      path: batchPath,
      sourceName: batchName.trim() || undefined,
      sourceUrl: batchUrl.trim() || undefined,
      hostAgent,
      confirmedPrivateRead: privateReadConfirmed,
    });
  }

  async function submitNetworkPreview(): Promise<void> {
    formError = null;
    if (!endpoint.trim() || !sourceName.trim()) {
      formError = `${copy.endpoint} / ${copy.sourceName}`;
      return;
    }
    if (!networkFetchConfirmed) {
      formError = copy.discoveryNetworkConsent;
      return;
    }
    await onPreviewNetwork({
      adapter,
      endpoint: endpoint.trim(),
      sourceName: sourceName.trim(),
      organization: organization.trim() || undefined,
      confirmedNetworkFetch: networkFetchConfirmed,
    });
  }

  async function confirmPromote(): Promise<void> {
    if (selectedLead && (await onPromote(selectedLead.id))) {
      promoteOpen = false;
    }
  }

  function localizedFreshness(value: DiscoveryLeadRecord["freshness"]): string {
    if (value === "current") return copy.current;
    if (value === "stale") return copy.stale;
    return copy.unknown;
  }

  function localizedStatus(value: DiscoveryLeadRecord["status"]): string {
    if (value === "active") return copy.active;
    if (value === "removed") return copy.removed;
    if (value === "expired") return copy.expired;
    return copy.promoted;
  }

  function adapterLabel(value: DiscoveryNetworkAdapter): string {
    if (value === "rss-atom") return copy.rssAtom;
    if (value === "jobs-ac-uk") return copy.jobsAcUk;
    if (value === "greenhouse") return copy.greenhouse;
    return copy.lever;
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
{/snippet}

<Page.Root>
  <Page.Header
    eyebrow={copy.opportunities}
    title={copy.opportunitiesTitle}
    description={copy.opportunitiesDescription}
    actions={activeWorkspace ? headerActions : undefined}
  />

  {#if !activeWorkspace}
    <Card.Root>
      <Card.Content>
        <Empty.Root class="min-h-32">
          <Empty.Header>
            <Empty.Media variant="icon" class="size-12 rounded-lg bg-accent text-accent-foreground">
              <Search size={21} strokeWidth={1.8} aria-hidden="true" />
            </Empty.Media>
            <Empty.Title class="text-base">{copy.noWorkspace}</Empty.Title>
            <Empty.Description>{copy.chooseWorkspaceDescription}</Empty.Description>
          </Empty.Header>
        </Empty.Root>
      </Card.Content>
    </Card.Root>
  {:else}
    <Page.Grid class="2xl:grid-cols-[minmax(320px,0.8fr)_minmax(0,1.2fr)]">
      <Page.Stack>
        <Card.Root id="lead-list" class="scroll-mt-44">
          <Card.Header>
            <Card.Title>{copy.discoveryLeads}</Card.Title>
            <Card.Description>{activeWorkspace.path}</Card.Description>
          </Card.Header>
          <Card.Content class="space-y-2">
            {#if loading}
              {#each [1, 2, 3] as row}
                <div class="space-y-2 rounded-lg border p-[var(--density-panel-padding)]">
                  <Skeleton class="h-4 w-2/3" />
                  <Skeleton class="h-3 w-1/2" />
                </div>
              {/each}
            {:else if !leads.length}
              <Empty.Root class="min-h-32 border bg-muted/20">
                <Empty.Header>
                  <Empty.Media variant="icon">
                    <FileSearch size={22} strokeWidth={1.8} aria-hidden="true" />
                  </Empty.Media>
                  <Empty.Title>{copy.noLeads}</Empty.Title>
                  <Empty.Description>{copy.noLeadsDescription}</Empty.Description>
                </Empty.Header>
              </Empty.Root>
            {:else}
              {#each leads as lead (lead.id)}
                <Button
                  variant="outline"
                  class={[
                    "h-auto min-h-9 w-full flex-col items-stretch gap-3 p-[var(--density-panel-padding)] text-left",
                    selectedLead?.id === lead.id ? "border-primary bg-accent/45" : "",
                  ]}
                  aria-current={selectedLead?.id === lead.id ? "true" : undefined}
                  onclick={() => onSelect(lead.id)}
                >
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <h2 class="truncate text-sm font-semibold">{lead.title}</h2>
                      <p class="mt-1 truncate text-xs text-muted-foreground">{lead.organization}</p>
                    </div>
                    <Badge variant={lead.freshness === "stale" ? "destructive" : "outline"}>
                      {localizedFreshness(lead.freshness)}
                    </Badge>
                  </div>
                  {#if lead.location || lead.deadline}
                    <p class="mt-3 truncate text-xs text-muted-foreground">
                      {[lead.location, lead.deadline].filter(Boolean).join(" · ")}
                    </p>
                  {/if}
                </Button>
              {/each}
            {/if}
          </Card.Content>
        </Card.Root>

        <Card.Root>
          <Card.Header>
            <Card.Title>{copy.discoverySources}</Card.Title>
            <Card.Description>{sources.length}</Card.Description>
          </Card.Header>
          <Card.Content class="space-y-2">
            {#if sources.length}
              {#each sources as source (source.id)}
                <div class="flex items-center gap-3 rounded-lg border p-3">
                  <div
                    class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent text-accent-foreground"
                  >
                    <DatabaseZap size={16} strokeWidth={1.8} aria-hidden="true" />
                  </div>
                  <div class="min-w-0 flex-1">
                    <p class="truncate text-sm font-medium">{source.name}</p>
                    <p class="mt-1 truncate text-xs text-muted-foreground">
                      {source.kind} · {source.endpoint ?? copy.localFile}
                    </p>
                  </div>
                </div>
              {/each}
            {:else}
              <Empty.Root class="min-h-20 border">
                <Empty.Header>
                  <Empty.Title>{copy.noDiscoverySources}</Empty.Title>
                </Empty.Header>
              </Empty.Root>
            {/if}
          </Card.Content>
        </Card.Root>
      </Page.Stack>

      <Page.Stack>
        <Card.Root>
          <Card.Header>
            <div class="flex items-start justify-between gap-[var(--density-section-gap)]">
              <div>
                <Card.Title>{selectedLead?.title ?? copy.leadDetails}</Card.Title>
                <Card.Description class="mt-1.5">
                  {selectedLead?.organization ?? copy.chooseLead}
                </Card.Description>
              </div>
              {#if selectedLead}
                <Button
                  class="min-h-10 shrink-0"
                  disabled={busy || selectedLead.status === "promoted"}
                  onclick={() => (promoteOpen = true)}
                >
                  <ArrowUpRight
                    size={16}
                    strokeWidth={1.8}
                    data-icon="inline-start"
                    aria-hidden="true"
                  />
                  {selectedLead.status === "promoted" ? copy.promotedLead : copy.promoteLead}
                </Button>
              {/if}
            </div>
          </Card.Header>
          <Card.Content>
            {#if selectedLead}
              <dl class="grid grid-cols-[auto_1fr] gap-x-5 gap-y-3 text-sm">
                <dt class="text-muted-foreground">{copy.organization}</dt>
                <dd class="text-right font-medium">{selectedLead.organization}</dd>
                <dt class="text-muted-foreground">{copy.location}</dt>
                <dd class="text-right font-medium">{selectedLead.location ?? "—"}</dd>
                <dt class="text-muted-foreground">{copy.deadline}</dt>
                <dd class="text-right font-medium">{selectedLead.deadline ?? "—"}</dd>
                <dt class="text-muted-foreground">{copy.freshness}</dt>
                <dd class="text-right font-medium">{localizedFreshness(selectedLead.freshness)}</dd>
                <dt class="text-muted-foreground">{copy.status}</dt>
                <dd class="text-right font-medium">{localizedStatus(selectedLead.status)}</dd>
                <dt class="text-muted-foreground">{copy.publicUrl}</dt>
                <dd class="truncate text-right font-medium" title={selectedLead.url}>
                  {selectedLead.url}
                </dd>
              </dl>
              {#if selectedLead.summary}
                <Separator class="my-5" />
                <p class="text-sm leading-6 text-muted-foreground">{selectedLead.summary}</p>
              {/if}
              <Separator class="my-5" />
              <div>
                <h3 class="text-sm font-semibold">{copy.possibleDuplicates}</h3>
                {#if suggestions?.suggestions.length}
                  <div class="mt-3 space-y-2">
                    {#each suggestions.suggestions as suggestion (suggestion.lead.id)}
                      <div
                        class="flex items-center justify-between gap-[var(--density-section-gap)] rounded-lg border p-3"
                      >
                        <div class="min-w-0">
                          <p class="truncate text-sm font-medium">{suggestion.lead.title}</p>
                          <p class="mt-1 truncate text-xs text-muted-foreground">
                            {suggestion.lead.organization}
                          </p>
                        </div>
                        <Badge variant="outline">
                          {suggestion.similarity_percent}% {copy.similarity}
                        </Badge>
                      </div>
                    {/each}
                  </div>
                {:else}
                  <p class="mt-3 text-sm text-muted-foreground">{copy.noDuplicates}</p>
                {/if}
              </div>
            {:else}
              <Empty.Root class="min-h-40 border">
                <Empty.Header>
                  <Empty.Media variant="icon">
                    <MapPin size={21} strokeWidth={1.8} aria-hidden="true" />
                  </Empty.Media>
                  <Empty.Description>{copy.chooseLead}</Empty.Description>
                </Empty.Header>
              </Empty.Root>
            {/if}
          </Card.Content>
        </Card.Root>

        <Card.Root>
          <Card.Header>
            <Card.Title>{copy.sourceIntake}</Card.Title>
            <Card.Description>{copy.opportunitiesDescription}</Card.Description>
          </Card.Header>
          <Card.Content>
            {#if preview}
              <Page.Panel tone="accent" class="space-y-[var(--density-section-gap)]">
                <div>
                  <Badge variant="secondary">{copy.reviewBeforeCommit}</Badge>
                  <p class="mt-3 text-sm leading-6 text-muted-foreground">
                    {copy.reviewBeforeCommitDescription}
                  </p>
                </div>
                <IntakeReviewSummary {copy} review={preview.intake} />
                {#if preview.preview.data.diagnostics.length}
                  <div>
                    <p class="text-xs font-semibold">{copy.importDiagnostics}</p>
                    <ul class="mt-2 space-y-1 text-xs text-muted-foreground">
                      {#each preview.preview.data.diagnostics.slice(0, 5) as diagnostic}
                        <li>{diagnostic.row}: {diagnostic.message}</li>
                      {/each}
                    </ul>
                  </div>
                {/if}
                <div class="flex flex-wrap gap-2">
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
              </Page.Panel>
            {:else}
              <Tabs.Root bind:value={intakeTab}>
                <Tabs.List class="responsive-tabs" data-columns="2">
                  <Tabs.Trigger value="batch">
                    <FileUp
                      size={16}
                      strokeWidth={1.8}
                      data-icon="inline-start"
                      aria-hidden="true"
                    />
                    {copy.discoveryImport}
                  </Tabs.Trigger>
                  <Tabs.Trigger value="network">
                    <Network
                      size={16}
                      strokeWidth={1.8}
                      data-icon="inline-start"
                      aria-hidden="true"
                    />
                    {copy.discoveryRefresh}
                  </Tabs.Trigger>
                </Tabs.List>
                <Tabs.Content
                  value="batch"
                  class="space-y-[var(--density-section-gap)] pt-[var(--density-section-gap)]"
                >
                  <div class="space-y-2">
                    <Label for="discovery-batch">{copy.discoveryBatch}</Label>
                    <div class="flex gap-2">
                      <Input id="discovery-batch" bind:value={batchPath} readonly />
                      <Button
                        type="button"
                        variant="outline"
                        class="shrink-0"
                        onclick={chooseBatch}
                      >
                        {copy.chooseFile}
                      </Button>
                    </div>
                  </div>
                  <div class="grid gap-[var(--density-section-gap)] sm:grid-cols-2">
                    <div class="space-y-2">
                      <Label for="discovery-batch-name">{copy.discoveryBatchName}</Label>
                      <Input id="discovery-batch-name" bind:value={batchName} />
                    </div>
                    <div class="space-y-2">
                      <Label for="discovery-batch-url">{copy.discoveryBatchUrl}</Label>
                      <Input id="discovery-batch-url" type="url" bind:value={batchUrl} />
                    </div>
                  </div>
                  <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
                    <Checkbox id="host-agent-batch" bind:checked={hostAgent} class="mt-0.5" />
                    <Label for="host-agent-batch" class="text-xs leading-5 font-normal">
                      {copy.hostAgentBatch}
                    </Label>
                  </div>
                  <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
                    <Checkbox
                      id="discovery-private-consent"
                      bind:checked={privateReadConfirmed}
                      class="mt-0.5"
                    />
                    <Label for="discovery-private-consent" class="text-xs leading-5 font-normal">
                      {copy.discoveryPrivateConsent}
                    </Label>
                  </div>
                  {#if formError && intakeTab === "batch"}
                    <Alert.Root variant="destructive">
                      <Alert.Description>{formError}</Alert.Description>
                    </Alert.Root>
                  {/if}
                  <Button
                    class="min-h-9"
                    disabled={!desktopRuntime || busy || !batchPath || !privateReadConfirmed}
                    onclick={submitBatchPreview}
                  >
                    {busy ? copy.working : copy.previewBatch}
                  </Button>
                </Tabs.Content>
                <Tabs.Content
                  value="network"
                  class="space-y-[var(--density-section-gap)] pt-[var(--density-section-gap)]"
                >
                  <div class="grid gap-[var(--density-section-gap)] sm:grid-cols-2">
                    <div class="space-y-2">
                      <Label for="discovery-adapter">{copy.discoveryAdapter}</Label>
                      <NativeSelect.Root
                        id="discovery-adapter"
                        size="desktop"
                        class="w-full"
                        bind:value={adapter}
                      >
                        {#each networkAdapters as option (option.kind)}
                          <NativeSelect.Option value={option.kind}
                            >{adapterLabel(
                              option.kind as DiscoveryNetworkAdapter,
                            )}</NativeSelect.Option
                          >
                        {/each}
                      </NativeSelect.Root>
                    </div>
                    <div class="space-y-2">
                      <Label for="discovery-source-name">{copy.sourceName}</Label>
                      <Input id="discovery-source-name" bind:value={sourceName} />
                    </div>
                  </div>
                  <div class="space-y-2">
                    <Label for="discovery-endpoint">{copy.endpoint}</Label>
                    <Input id="discovery-endpoint" type="url" bind:value={endpoint} />
                  </div>
                  <div class="space-y-2">
                    <Label for="discovery-organization">{copy.optionalOrganization}</Label>
                    <Input id="discovery-organization" bind:value={organization} />
                  </div>
                  <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
                    <Checkbox
                      id="discovery-network-consent"
                      bind:checked={networkFetchConfirmed}
                      class="mt-0.5"
                    />
                    <Label for="discovery-network-consent" class="text-xs leading-5 font-normal">
                      {copy.discoveryNetworkConsent}
                    </Label>
                  </div>
                  {#if formError && intakeTab === "network"}
                    <Alert.Root variant="destructive">
                      <Alert.Description>{formError}</Alert.Description>
                    </Alert.Root>
                  {/if}
                  <Button
                    class="min-h-9"
                    disabled={!desktopRuntime ||
                      busy ||
                      !endpoint.trim() ||
                      !sourceName.trim() ||
                      !networkFetchConfirmed}
                    onclick={submitNetworkPreview}
                  >
                    {busy ? copy.working : copy.previewRefresh}
                  </Button>
                </Tabs.Content>
              </Tabs.Root>
            {/if}
          </Card.Content>
        </Card.Root>
      </Page.Stack>
    </Page.Grid>
  {/if}
</Page.Root>

<Dialog.Root bind:open={promoteOpen}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>{copy.promoteLead}</Dialog.Title>
      <Dialog.Description>{copy.promoteLeadDescription}</Dialog.Description>
    </Dialog.Header>
    <div class="rounded-lg border bg-muted/20 p-3">
      <p class="text-sm font-medium">{selectedLead?.title}</p>
      <p class="mt-1 text-xs text-muted-foreground">{selectedLead?.organization}</p>
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (promoteOpen = false)}>{copy.cancel}</Button>
      <Button disabled={busy} onclick={confirmPromote}>
        <Link size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
        {copy.promoteLead}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
