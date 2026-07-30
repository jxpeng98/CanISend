<script lang="ts">
  import {
    FileCheck2,
    FileText,
    FolderOpen,
    RefreshCw,
    ShieldCheck,
    UserRound,
  } from "@lucide/svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import { Skeleton } from "$lib/components/ui/skeleton/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";
  import {
    chooseProfileSource,
    type EvidenceCatalogRecord,
    type PrivacyClassification,
    type ProfileSourceRecord,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";
  import {
    buildJsonDiff,
    collectRevisionReferences,
    type JsonDiffSummary,
    type RevisionReferenceSummary,
  } from "$lib/proposal-review";
  import type { WorkflowDetail } from "$lib/workflow-navigation";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    activeWorkspace: WorkspaceReadModel | null;
    selectedJobId: string;
    focus: WorkflowDetail | null;
    sources: ProfileSourceRecord[];
    profileRevision: number;
    evidence: EvidenceCatalogRecord | null;
    loading: boolean;
    busy: boolean;
    onRefresh: () => Promise<boolean>;
    onImport: (options: {
      source: string;
      sensitivity: PrivacyClassification;
      confirmedPrivateRead: boolean;
    }) => Promise<boolean>;
    onInitialize: (options: {
      markdown: string;
      sensitivity: PrivacyClassification;
      confirmedPrivateRead: boolean;
    }) => Promise<boolean>;
    onLoadEvidence: (
      jobId: string,
      confirmedPrivateRead: boolean,
    ) => Promise<boolean>;
    onConfirmEvidence: (
      jobId: string,
      candidate: unknown,
      confirmedPrivateRead: boolean,
    ) => Promise<boolean>;
  };

  let {
    copy,
    desktopRuntime,
    activeWorkspace,
    selectedJobId,
    focus,
    sources,
    profileRevision,
    evidence,
    loading,
    busy,
    onRefresh,
    onImport,
    onInitialize,
    onLoadEvidence,
    onConfirmEvidence,
  }: Props = $props();

  let sourcePath = $state("");
  let sensitivity = $state<PrivacyClassification>("private-local");
  let importConsent = $state(false);
  let privateSessionConsent = $state(false);
  let evidenceJson = $state("");
  let evidenceKey = $state("");
  let evidenceBaseline = $state<unknown>(null);
  let evidencePreview = $state<{
    candidate: unknown;
    diff: JsonDiffSummary;
    references: RevisionReferenceSummary[];
  } | null>(null);
  let formError = $state<string | null>(null);
  let initializationMarkdown = $state("");
  let previousInitializationTemplate = $state("");
  let initializationConsent = $state(false);

  $effect(() => {
    const nextTemplate = copy.profileInitializationTemplate;
    if (
      !previousInitializationTemplate ||
      initializationMarkdown === previousInitializationTemplate
    ) {
      initializationMarkdown = nextTemplate;
    }
    previousInitializationTemplate = nextTemplate;
  });

  $effect(() => {
    const nextKey = evidence ? `${evidence.id}:${evidence.revision}` : "";
    if (nextKey !== evidenceKey) {
      evidenceKey = nextKey;
      evidenceJson = evidence ? JSON.stringify(evidence, null, 2) : "";
      evidenceBaseline = evidence;
      evidencePreview = null;
    }
  });

  async function chooseSource(): Promise<void> {
    sourcePath = (await chooseProfileSource()) ?? sourcePath;
  }

  async function submitImport(): Promise<void> {
    formError = null;
    if (!sourcePath) {
      formError = copy.chooseFile;
      return;
    }
    if (!importConsent) {
      formError = copy.profileImportConsent;
      return;
    }
    if (
      await onImport({
        source: sourcePath,
        sensitivity,
        confirmedPrivateRead: importConsent,
      })
    ) {
      sourcePath = "";
      importConsent = false;
    }
  }

  async function initializeProfile(): Promise<void> {
    formError = null;
    if (!initializationConsent) {
      formError = copy.profileInitializationConsent;
      return;
    }
    if (
      await onInitialize({
        markdown: initializationMarkdown,
        sensitivity: "private-local",
        confirmedPrivateRead: initializationConsent,
      })
    ) {
      initializationConsent = false;
    }
  }

  async function loadEvidence(): Promise<void> {
    formError = null;
    if (!selectedJobId) {
      formError = copy.selectApplication;
      return;
    }
    if (!privateSessionConsent) {
      formError = copy.privateWorkspaceConsent;
      return;
    }
    await onLoadEvidence(selectedJobId, privateSessionConsent);
  }

  function previewEvidence(): void {
    formError = null;
    if (!selectedJobId || !privateSessionConsent) {
      formError = copy.privateWorkspaceConsent;
      return;
    }
    try {
      const candidate: unknown = JSON.parse(evidenceJson);
      evidencePreview = {
        candidate,
        diff: buildJsonDiff(evidenceBaseline, candidate),
        references: collectRevisionReferences(candidate),
      };
    } catch {
      formError = copy.invalidJson;
    }
  }

  function editEvidence(): void {
    evidencePreview = null;
  }

  async function confirmEvidence(): Promise<void> {
    formError = null;
    if (!selectedJobId || !privateSessionConsent || !evidencePreview) {
      formError = copy.privateWorkspaceConsent;
      return;
    }
    if (
      await onConfirmEvidence(
        selectedJobId,
        evidencePreview.candidate,
        privateSessionConsent,
      )
    ) {
      evidencePreview = null;
    }
  }
</script>

<section class="space-y-6">
  <div class="flex flex-col justify-between gap-4 xl:flex-row xl:items-end">
    <div>
      <Badge variant="secondary" class="mb-3">{copy.profile}</Badge>
      <h1 class="text-3xl font-semibold tracking-[-0.03em]">{copy.profileTitle}</h1>
      <p class="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">
        {copy.profileDescription}
      </p>
    </div>
    <Button
      variant="outline"
      class="min-h-11"
      disabled={!activeWorkspace || busy}
      onclick={onRefresh}
    >
      <RefreshCw size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
      {copy.refresh}
    </Button>
  </div>

  {#if !activeWorkspace}
    <Card.Root class="shadow-none">
      <Card.Content class="flex min-h-80 flex-col items-center justify-center px-8 text-center">
        <UserRound size={24} strokeWidth={1.8} class="text-muted-foreground" aria-hidden="true" />
        <h2 class="mt-4 text-base font-semibold">{copy.noWorkspace}</h2>
        <p class="mt-2 max-w-md text-sm leading-6 text-muted-foreground">
          {copy.chooseWorkspaceDescription}
        </p>
      </Card.Content>
    </Card.Root>
  {:else}
    <div class="grid gap-6 2xl:grid-cols-[minmax(320px,0.78fr)_minmax(0,1.22fr)]">
      <div class="space-y-6">
        {#if !loading && sources.length === 0}
          <Card.Root class="border-primary/25 bg-primary/5 shadow-none">
            <Card.Header>
              <Card.Title>{copy.initializeProfile}</Card.Title>
              <Card.Description>{copy.initializeProfileDescription}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              <div class="space-y-2">
                <Label for="profile-initialization">{copy.profileMarkdown}</Label>
                <Textarea
                  id="profile-initialization"
                  class="min-h-72 resize-y text-sm leading-6"
                  bind:value={initializationMarkdown}
                />
              </div>
              <div class="rounded-xl border bg-background/70 p-3 text-xs leading-5 text-muted-foreground">
                {copy.profileStorageDescription}
              </div>
              <div class="flex items-start gap-3 rounded-xl border bg-background/70 p-3">
                <Checkbox
                  id="profile-initialization-consent"
                  bind:checked={initializationConsent}
                  class="mt-0.5"
                />
                <Label
                  for="profile-initialization-consent"
                  class="text-xs leading-5 font-normal"
                >
                  {copy.profileInitializationConsent}
                </Label>
              </div>
              <Button
                class="min-h-11"
                disabled={busy || !initializationConsent || !initializationMarkdown.trim()}
                onclick={initializeProfile}
              >
                {busy ? copy.working : copy.initializeProfile}
              </Button>
            </Card.Content>
          </Card.Root>
        {/if}

        <Card.Root
          id="profile-sources"
          class={[
            "scroll-mt-64 shadow-none transition-colors",
            focus === "profile-sources" ? "ring-2 ring-primary/35" : "",
          ]}
        >
          <Card.Header>
            <div class="flex items-start justify-between gap-4">
              <div>
                <Card.Title>{copy.profileSources}</Card.Title>
                <Card.Description class="mt-1.5">
                  {copy.profileRevision} {profileRevision}
                </Card.Description>
              </div>
              <div class="grid size-10 place-items-center rounded-xl bg-accent text-accent-foreground">
                <FileText size={18} strokeWidth={1.8} aria-hidden="true" />
              </div>
            </div>
          </Card.Header>
          <Card.Content class="space-y-2">
            {#if loading}
              {#each [1, 2, 3] as row}
                <div class="space-y-2 rounded-xl border p-4">
                  <Skeleton class="h-4 w-2/3" />
                  <Skeleton class="h-3 w-1/2" />
                </div>
              {/each}
            {:else if sources.length}
              {#each sources as source (source.id)}
                <div class="rounded-xl border p-4">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <p class="truncate text-sm font-semibold">{source.kind}</p>
                      <p class="mt-1 truncate font-mono text-[11px] text-muted-foreground">
                        {source.id}
                      </p>
                    </div>
                    <Badge variant="outline">{source.sensitivity}</Badge>
                  </div>
                  <p class="mt-3 text-xs text-muted-foreground">
                    {source.content_type} · r{source.revision}
                  </p>
                </div>
              {/each}
            {:else}
              <div class="flex min-h-52 flex-col items-center justify-center rounded-xl border border-dashed bg-muted/20 p-6 text-center">
                <FileText size={22} strokeWidth={1.8} class="text-muted-foreground" aria-hidden="true" />
                <h2 class="mt-3 text-sm font-semibold">{copy.noProfileSources}</h2>
                <p class="mt-2 text-xs leading-5 text-muted-foreground">
                  {copy.noProfileSourcesDescription}
                </p>
              </div>
            {/if}
          </Card.Content>
        </Card.Root>

        <Card.Root class="shadow-none">
          <Card.Header>
            <Card.Title>{copy.importProfileSource}</Card.Title>
            <Card.Description>{copy.noProfileSourcesDescription}</Card.Description>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div class="space-y-2">
              <Label for="profile-source-file">{copy.sourceFile}</Label>
              <div class="flex gap-2">
                <Input id="profile-source-file" bind:value={sourcePath} readonly />
                <Button type="button" variant="outline" class="shrink-0" onclick={chooseSource}>
                  <FolderOpen size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                  {copy.chooseFile}
                </Button>
              </div>
            </div>
            <div class="space-y-2">
              <Label for="profile-sensitivity">{copy.profileSensitivity}</Label>
              <select
                id="profile-sensitivity"
                class="h-9 w-full rounded-lg border border-input bg-background px-3 text-sm"
                bind:value={sensitivity}
              >
                <option value="private-local">{copy.privateProfileSource}</option>
                <option value="public">{copy.publicProfileSource}</option>
              </select>
            </div>
            <div class="flex items-start gap-3 rounded-xl border bg-muted/20 p-3">
              <Checkbox id="profile-import-consent" bind:checked={importConsent} class="mt-0.5" />
              <Label for="profile-import-consent" class="text-xs leading-5 font-normal">
                {copy.profileImportConsent}
              </Label>
            </div>
            <Button
              class="min-h-11"
              disabled={!desktopRuntime || busy || !sourcePath || !importConsent}
              onclick={submitImport}
            >
              {busy ? copy.working : copy.importProfileSource}
            </Button>
          </Card.Content>
        </Card.Root>
      </div>

      <Card.Root
        id="profile-evidence"
        class={[
          "scroll-mt-64 shadow-none transition-colors",
          focus === "profile-evidence" ? "ring-2 ring-primary/35" : "",
        ]}
      >
        <Card.Header>
          <div class="flex items-start justify-between gap-4">
            <div>
              <Card.Title>{copy.evidenceReview}</Card.Title>
              <Card.Description class="mt-1.5">
                {evidence ? `${evidence.items.length} ${copy.items} · r${evidence.revision}` : copy.loadEvidenceCandidate}
              </Card.Description>
            </div>
            <div class="grid size-10 place-items-center rounded-xl bg-accent text-accent-foreground">
              <FileCheck2 size={18} strokeWidth={1.8} aria-hidden="true" />
            </div>
          </div>
        </Card.Header>
        <Card.Content class="space-y-4">
          <div class="flex justify-end">
            <Button
              variant="outline"
              class="min-h-11"
              disabled={busy || !selectedJobId || !privateSessionConsent}
              onclick={loadEvidence}
            >
              {copy.loadEvidenceCandidate}
            </Button>
          </div>
          <div class="flex items-start gap-3 rounded-xl border bg-muted/20 p-3">
            <Checkbox
              id="profile-private-session"
              bind:checked={privateSessionConsent}
              class="mt-0.5"
            />
            <Label for="profile-private-session" class="text-xs leading-5 font-normal">
              <span class="mb-1 flex items-center gap-2 font-medium">
                <ShieldCheck size={14} strokeWidth={1.8} aria-hidden="true" />
                {copy.privateWorkspaceConsent}
              </span>
            </Label>
          </div>
          <Separator />
          <div class="space-y-2">
            <Label for="evidence-candidate">{copy.candidateJson}</Label>
            <Textarea
              id="evidence-candidate"
              class="min-h-[430px] resize-y font-mono text-xs leading-5"
              bind:value={evidenceJson}
              spellcheck={false}
              disabled={!evidence || evidencePreview !== null}
            />
          </div>
          {#if formError}
            <p class="text-sm text-destructive" role="alert">{formError}</p>
          {/if}
          <Button
            variant="outline"
            class="min-h-11"
            disabled={busy ||
              !evidence ||
              !privateSessionConsent ||
              !evidenceJson ||
              evidencePreview !== null}
            onclick={previewEvidence}
          >
            {busy ? copy.working : copy.previewProposal}
          </Button>
          {#if evidencePreview}
            <div class="space-y-4 rounded-xl border border-primary/35 bg-primary/5 p-4">
              <div class="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <Badge variant="secondary">{copy.reviewBeforeCommit}</Badge>
                  <p class="mt-2 text-sm font-semibold">{copy.proposalDiff}</p>
                  <p class="mt-1 text-xs leading-5 text-muted-foreground">
                    {copy.proposalPreviewNotCommit}
                  </p>
                </div>
                <Badge variant="outline">
                  {evidencePreview.diff.totalChanges} {copy.changedFields}
                </Badge>
              </div>

              <div class="max-h-72 overflow-auto rounded-lg border bg-background">
                {#each evidencePreview.diff.changes as change (change.path)}
                  <div class="border-b p-3 last:border-b-0">
                    <p class="break-all font-mono text-[11px] font-semibold">{change.path}</p>
                    <div class="mt-2 grid gap-2 lg:grid-cols-2">
                      <div class="rounded-md bg-muted/40 p-2">
                        <p class="text-[10px] font-medium text-muted-foreground">{copy.before}</p>
                        <p class="mt-1 break-words font-mono text-[11px]">{change.before}</p>
                      </div>
                      <div class="rounded-md bg-primary/5 p-2">
                        <p class="text-[10px] font-medium text-muted-foreground">{copy.after}</p>
                        <p class="mt-1 break-words font-mono text-[11px]">{change.after}</p>
                      </div>
                    </div>
                  </div>
                {:else}
                  <p class="p-4 text-xs text-muted-foreground">{copy.noProposalChanges}</p>
                {/each}
              </div>
              {#if evidencePreview.diff.truncated || evidencePreview.diff.comparisonLimited}
                <p class="text-xs text-muted-foreground">{copy.diffTruncated}</p>
              {/if}

              <div class="grid gap-4 lg:grid-cols-2">
                <div class="rounded-lg border bg-background p-3">
                  <p class="text-xs font-semibold">{copy.revisionProvenance}</p>
                  <div class="mt-2 space-y-2">
                    {#each evidencePreview.references as reference (`${reference.path}:${reference.id}:${reference.revision}`)}
                      <p class="break-all font-mono text-[10px] leading-4 text-muted-foreground">
                        {reference.kind} · {reference.id} · r{reference.revision}
                      </p>
                    {:else}
                      <p class="text-xs text-muted-foreground">
                        {copy.noEmbeddedRevisionReferences}
                      </p>
                    {/each}
                  </div>
                </div>
                <div class="rounded-lg border bg-background p-3">
                  <p class="text-xs font-semibold">{copy.validationAtCommit}</p>
                  <ul class="mt-2 list-disc space-y-1.5 pl-4 text-xs leading-5 text-muted-foreground">
                    <li>{copy.validateCandidateSchema}</li>
                    <li>{copy.validateCurrentRevisions}</li>
                    <li>{copy.validateSourceScope}</li>
                  </ul>
                </div>
              </div>

              <div class="rounded-lg border bg-background p-3">
                <p class="text-xs font-semibold">{copy.intendedStateChange}</p>
                <p class="mt-1 text-xs leading-5 text-muted-foreground">
                  {copy.evidenceMutationDescription}
                </p>
              </div>

              <div class="flex flex-wrap justify-end gap-2">
                <Button variant="outline" disabled={busy} onclick={editEvidence}>
                  {copy.editProposal}
                </Button>
                <Button class="min-h-11" disabled={busy} onclick={confirmEvidence}>
                  {busy ? copy.working : copy.confirmEvidence}
                </Button>
              </div>
            </div>
          {/if}
        </Card.Content>
      </Card.Root>
    </div>
  {/if}
</section>
