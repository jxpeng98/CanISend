<script lang="ts">
  import {
    ArrowRight,
    FileSearch,
    LibraryBig,
    Link2,
    LockKeyhole,
    RefreshCw,
    Search,
    ShieldCheck,
  } from "@lucide/svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import * as Alert from "$lib/components/ui/alert/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import * as Empty from "$lib/components/ui/empty/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import * as NativeSelect from "$lib/components/ui/native-select/index.js";
  import LoadingPanel from "$lib/components/patterns/LoadingPanel.svelte";
  import type {
    ContentCatalogEntryReadModel,
    ContentCatalogFilter,
    ContentCatalogReadModel,
    ContentCatalogStatus,
    ContentCategory,
    ContentPrivacyClassification,
    ContentSearchReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";

  type Scope = "application" | "workspace";
  type OptionalCategory = ContentCategory | "all";
  type OptionalStatus = ContentCatalogStatus | "all";
  type OptionalPrivacy = ContentPrivacyClassification | "all";
  type DisplayItem = {
    entry: ContentCatalogEntryReadModel;
    score: number | null;
    matchedFields: Array<"metadata" | "private-body">;
    snippet: string | null;
  };

  type SearchOptions = {
    query: string;
    filter: ContentCatalogFilter;
    includePrivateBodies: boolean;
    confirmedPrivateRead: boolean;
  };

  type Props = {
    copy: Messages;
    catalog: ContentCatalogReadModel | null;
    searchResult: ContentSearchReadModel | null;
    selectedJobId: string;
    loading: boolean;
    busy: boolean;
    onRefresh: () => Promise<boolean>;
    onSearch: (options: SearchOptions) => Promise<boolean>;
    onOpen: (entry: ContentCatalogEntryReadModel) => Promise<void>;
  };

  let {
    copy,
    catalog,
    searchResult,
    selectedJobId,
    loading,
    busy,
    onRefresh,
    onSearch,
    onOpen,
  }: Props = $props();

  let scope = $state<Scope>("workspace");
  let panelInitialized = $state(false);
  let query = $state("");
  let category = $state<OptionalCategory>("all");
  let status = $state<OptionalStatus>("all");
  let privacy = $state<OptionalPrivacy>("all");
  let createdAfter = $state("");
  let createdBefore = $state("");
  let includePrivateBodies = $state(false);
  let confirmedPrivateRead = $state(false);
  let showSearchResults = $state(false);
  let formError = $state<string | null>(null);

  $effect(() => {
    if (!panelInitialized) {
      if (searchResult) {
        query = searchResult.query;
        scope = searchResult.filter.job_id ? "application" : "workspace";
        category = searchResult.filter.category ?? "all";
        status = searchResult.filter.status ?? "all";
        privacy = searchResult.filter.privacy ?? "all";
        createdAfter = searchResult.filter.created_after?.slice(0, 10) ?? "";
        createdBefore = searchResult.filter.created_before?.slice(0, 10) ?? "";
        includePrivateBodies = searchResult.include_private_bodies;
        showSearchResults = true;
      } else {
        scope = selectedJobId ? "application" : "workspace";
      }
      panelInitialized = true;
    }
    if (!selectedJobId && scope === "application") {
      scope = "workspace";
      showSearchResults = false;
    }
    if (panelInitialized && showSearchResults && !searchResult) {
      showSearchResults = false;
    }
  });

  const filteredCatalogEntries = $derived(
    (catalog?.entries ?? []).filter((entry) => {
      const jobMatches =
        scope === "workspace" ||
        (!!selectedJobId &&
          entry.subject_jobs.some((job) => job.id === selectedJobId));
      return (
        jobMatches &&
        (category === "all" || entry.category === category) &&
        (status === "all" || entry.status === status) &&
        (privacy === "all" || entry.privacy === privacy) &&
        (!createdAfter ||
          entry.created_at >= `${createdAfter}T00:00:00Z`) &&
        (!createdBefore ||
          entry.created_at <= `${createdBefore}T23:59:59.999999999Z`)
      );
    }),
  );

  const displayItems = $derived<DisplayItem[]>(
    showSearchResults && searchResult
      ? searchResult.results.map((result) => ({
          entry: result.entry,
          score: result.score,
          matchedFields: result.matched_fields,
          snippet: result.snippet,
        }))
      : filteredCatalogEntries.map((entry) => ({
          entry,
          score: null,
          matchedFields: [],
          snippet: null,
        })),
  );

  function currentFilter(): ContentCatalogFilter {
    return {
      job_id:
        scope === "application" && selectedJobId ? selectedJobId : null,
      category: category === "all" ? null : category,
      status: status === "all" ? null : status,
      privacy: privacy === "all" ? null : privacy,
      stage: null,
      created_after: createdAfter
        ? `${createdAfter}T00:00:00Z`
        : null,
      created_before: createdBefore
        ? `${createdBefore}T23:59:59.999999999Z`
        : null,
    };
  }

  function invalidateSearch(): void {
    showSearchResults = false;
    formError = null;
  }

  function togglePrivateBodies(checked: boolean): void {
    includePrivateBodies = checked;
    if (!checked) confirmedPrivateRead = false;
    invalidateSearch();
  }

  async function submitSearch(): Promise<void> {
    formError = null;
    if (includePrivateBodies && !confirmedPrivateRead) {
      formError = copy.contentPrivateConsentRequired;
      return;
    }
    if (includePrivateBodies && !query.trim()) {
      formError = copy.contentPrivateQueryRequired;
      return;
    }
    const completed = await onSearch({
      query: query.trim(),
      filter: currentFilter(),
      includePrivateBodies,
      confirmedPrivateRead,
    });
    showSearchResults = completed;
  }

  async function refreshCatalog(): Promise<void> {
    if (await onRefresh()) showSearchResults = false;
  }

  function formatBytes(value: number): string {
    if (value < 1024) return `${value} B`;
    if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
    return `${(value / (1024 * 1024)).toFixed(1)} MB`;
  }

  function provenanceLabel(entry: ContentCatalogEntryReadModel): string {
    return (
      entry.provenance.locator ??
      entry.provenance.source_kind ??
      entry.provenance.reason
    );
  }
</script>

<Card.Root id="content-library" class="scroll-mt-44">
  <Card.Header>
    <div class="flex flex-col justify-between gap-3 sm:flex-row sm:items-start">
      <div class="flex items-start gap-3">
        <div class="grid size-10 shrink-0 place-items-center rounded-lg bg-accent text-accent-foreground">
          <LibraryBig size={18} strokeWidth={1.8} aria-hidden="true" />
        </div>
        <div>
          <Card.Title>{copy.contentLibrary}</Card.Title>
          <Card.Description class="mt-1.5 max-w-2xl">
            {copy.contentLibraryDescription}
          </Card.Description>
        </div>
      </div>
      <Button
        variant="outline"
        class="min-h-9 shrink-0"
        disabled={loading || busy}
        onclick={refreshCatalog}
      >
        <RefreshCw
          size={16}
          strokeWidth={1.8}
          class={loading ? "animate-spin motion-reduce:animate-none" : undefined}
          data-icon="inline-start"
          aria-hidden="true"
        />
        {copy.refresh}
      </Button>
    </div>
  </Card.Header>

  <Card.Content class="space-y-[var(--density-section-gap)]">
    <form class="space-y-[var(--density-section-gap)]" onsubmit={(event) => {
      event.preventDefault();
      void submitSearch();
    }}>
      <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <div class="space-y-2">
          <Label for="content-scope">{copy.contentScope}</Label>
          <NativeSelect.Root
            id="content-scope"
            bind:value={scope}
            onchange={invalidateSearch}
            size="desktop"
            class="w-full"
          >
            <NativeSelect.Option value="application" disabled={!selectedJobId}>
              {copy.contentCurrentApplication}
            </NativeSelect.Option>
            <NativeSelect.Option value="workspace">{copy.contentEntireWorkspace}</NativeSelect.Option>
          </NativeSelect.Root>
        </div>

        <div class="space-y-2">
          <Label for="content-category">{copy.contentCategory}</Label>
          <NativeSelect.Root
            id="content-category"
            bind:value={category}
            onchange={invalidateSearch}
            size="desktop"
            class="w-full"
          >
            <NativeSelect.Option value="all">{copy.contentAllCategories}</NativeSelect.Option>
            {#each Object.entries(copy.contentCategoryLabel) as [value, label]}
              <NativeSelect.Option value={value}>{label}</NativeSelect.Option>
            {/each}
          </NativeSelect.Root>
        </div>

        <div class="space-y-2">
          <Label for="content-status">{copy.contentLifecycle}</Label>
          <NativeSelect.Root
            id="content-status"
            bind:value={status}
            onchange={invalidateSearch}
            size="desktop"
            class="w-full"
          >
            <NativeSelect.Option value="all">{copy.contentAllStatuses}</NativeSelect.Option>
            {#each Object.entries(copy.contentStatusLabel) as [value, label]}
              <NativeSelect.Option value={value}>{label}</NativeSelect.Option>
            {/each}
          </NativeSelect.Root>
        </div>

        <div class="space-y-2">
          <Label for="content-privacy">{copy.contentPrivacy}</Label>
          <NativeSelect.Root
            id="content-privacy"
            bind:value={privacy}
            onchange={invalidateSearch}
            size="desktop"
            class="w-full"
          >
            <NativeSelect.Option value="all">{copy.contentAllPrivacy}</NativeSelect.Option>
            {#each Object.entries(copy.contentPrivacyLabel) as [value, label]}
              <NativeSelect.Option value={value}>{label}</NativeSelect.Option>
            {/each}
          </NativeSelect.Root>
        </div>
      </div>

      <div class="grid gap-3 sm:grid-cols-2">
        <div class="space-y-2">
          <Label for="content-created-after">{copy.contentCreatedAfter}</Label>
          <Input
            id="content-created-after"
            type="date"
            class="min-h-9"
            bind:value={createdAfter}
            oninput={invalidateSearch}
          />
        </div>
        <div class="space-y-2">
          <Label for="content-created-before">{copy.contentCreatedBefore}</Label>
          <Input
            id="content-created-before"
            type="date"
            class="min-h-9"
            bind:value={createdBefore}
            oninput={invalidateSearch}
          />
        </div>
      </div>

      <div class="flex flex-col gap-2 sm:flex-row">
        <div class="relative min-w-0 flex-1">
          <Search
            size={16}
            strokeWidth={1.8}
            class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
            aria-hidden="true"
          />
          <Input
            aria-label={copy.contentSearch}
            class="min-h-9 pl-9"
            maxlength={200}
            placeholder={copy.contentSearchPlaceholder}
            bind:value={query}
            oninput={invalidateSearch}
          />
        </div>
        <Button
          type="submit"
          class="min-h-9 shrink-0"
          disabled={loading || busy || (includePrivateBodies && !confirmedPrivateRead)}
        >
          <FileSearch size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
          {busy ? copy.working : copy.contentSearch}
        </Button>
      </div>

      <div class="rounded-lg border bg-muted/20 p-[var(--density-panel-padding)]">
        <div class="flex items-start gap-3">
          <Checkbox
            id="content-private-bodies"
            checked={includePrivateBodies}
            onCheckedChange={(value) => togglePrivateBodies(value === true)}
            class="mt-0.5"
          />
          <div class="min-w-0">
            <Label for="content-private-bodies" class="text-sm font-medium">
              {copy.contentPrivateSearch}
            </Label>
            <p class="mt-1 text-xs leading-5 text-muted-foreground">
              {copy.contentPrivateSearchDescription}
            </p>
          </div>
        </div>
        {#if includePrivateBodies}
          <div class="mt-3 flex items-start gap-3 border-t pt-3">
            <Checkbox
              id="content-private-consent"
              bind:checked={confirmedPrivateRead}
              class="mt-0.5"
            />
            <Label for="content-private-consent" class="text-xs leading-5 font-normal">
              {copy.contentPrivateConsent}
            </Label>
          </div>
        {/if}
      </div>

      {#if formError}
        <Alert.Root variant="destructive">
          <Alert.Description>{formError}</Alert.Description>
        </Alert.Root>
      {/if}
    </form>

    <div class="flex flex-col justify-between gap-3 border-t pt-5 sm:flex-row sm:items-center">
      <div>
        <h3 class="text-sm font-semibold">
          {showSearchResults ? copy.contentSearchResults : copy.contentCatalogItems}
        </h3>
        <p class="mt-1 text-xs text-muted-foreground" aria-live="polite">
          {displayItems.length} {copy.items}
          {#if showSearchResults && searchResult}
            · {searchResult.index.metadata_entries} {copy.contentMetadataEntries}
            · {searchResult.index.private_body_entries} {copy.contentPrivateEntries}
          {/if}
        </p>
      </div>
      <div class="flex flex-wrap gap-2">
        <Badge variant="outline">
          <ShieldCheck size={13} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
          {showSearchResults && searchResult?.include_private_bodies
            ? copy.contentEphemeralIndex
            : copy.contentMetadataOnly}
        </Badge>
      </div>
    </div>

    {#if loading}
      <LoadingPanel label={copy.loading} class="min-h-32 border" />
    {:else if !displayItems.length}
      <Empty.Root class="min-h-32 border">
        <Empty.Header>
          <Empty.Media variant="icon">
            <LibraryBig size={22} strokeWidth={1.8} aria-hidden="true" />
          </Empty.Media>
          <Empty.Title>{copy.contentNoResults}</Empty.Title>
          <Empty.Description>{copy.contentNoResultsDescription}</Empty.Description>
        </Empty.Header>
      </Empty.Root>
    {:else}
      <div class="space-y-2">
        {#each displayItems as item (item.entry.artifact.id)}
          <article class="rounded-lg border p-[var(--density-panel-padding)]">
            <div class="flex flex-col justify-between gap-3 sm:flex-row sm:items-start">
              <div class="flex min-w-0 items-start gap-3">
                <div class="grid size-9 shrink-0 place-items-center rounded-lg bg-accent text-accent-foreground">
                  {#if item.entry.provenance.locator}
                    <Link2 size={16} strokeWidth={1.8} aria-hidden="true" />
                  {:else if item.entry.private_body_searchable}
                    <FileSearch size={16} strokeWidth={1.8} aria-hidden="true" />
                  {:else}
                    <LockKeyhole size={16} strokeWidth={1.8} aria-hidden="true" />
                  {/if}
                </div>
                <div class="min-w-0">
                  <div class="flex flex-wrap items-center gap-2">
                    <h4 class="text-sm font-semibold">{item.entry.title}</h4>
                    <Badge variant="secondary">
                      {copy.contentCategoryLabel[item.entry.category]}
                    </Badge>
                    <Badge variant="outline">
                      {copy.contentStatusLabel[item.entry.status]}
                    </Badge>
                  </div>
                  <p class="mt-1 truncate text-xs text-muted-foreground" title={provenanceLabel(item.entry)}>
                    {provenanceLabel(item.entry)}
                  </p>
                </div>
              </div>
              <Button
                variant="outline"
                class="min-h-10 shrink-0"
                disabled={busy}
                onclick={() => onOpen(item.entry)}
              >
                {copy.contentOpenStep}
                <ArrowRight size={15} strokeWidth={1.8} data-icon="inline-end" aria-hidden="true" />
              </Button>
            </div>

            <div class="mt-3 flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-muted-foreground">
              <span>{copy.workflowStageLabel[item.entry.stage]}</span>
              <span>{copy.contentPrivacyLabel[item.entry.privacy]}</span>
              <span>{formatBytes(item.entry.size)}</span>
              <span>{item.entry.subject_jobs[0]?.title ?? copy.contentReusableProfile}</span>
              {#if item.score !== null}
                <span>{copy.contentRelevance} {item.score}</span>
              {/if}
            </div>

            {#if item.matchedFields.length}
              <div class="mt-3 flex flex-wrap gap-2">
                {#each item.matchedFields as field}
                  <Badge variant="outline">
                    {field === "private-body" ? copy.contentBodyMatch : copy.contentMetadataMatch}
                  </Badge>
                {/each}
              </div>
            {/if}
            {#if item.snippet}
              <p class="mt-3 rounded-lg border bg-muted/20 p-3 text-xs leading-5">
                {item.snippet}
              </p>
            {/if}
          </article>
        {/each}
      </div>
    {/if}
  </Card.Content>
</Card.Root>
