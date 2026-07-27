<script lang="ts">
  import {
    Archive,
    Database,
    FolderOpen,
    HeartPulse,
    Link,
    Plus,
    RefreshCw,
    ShieldCheck,
    Trash2,
    Wrench,
  } from "@lucide/svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import { Skeleton } from "$lib/components/ui/skeleton/index.js";
  import {
    chooseWorkspaceDirectory,
    type RegistrySnapshot,
    type WorkspaceHealthReadModel,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    snapshot: RegistrySnapshot | null;
    activeWorkspace: WorkspaceReadModel | null;
    health: WorkspaceHealthReadModel | null;
    loading: boolean;
    busy: boolean;
    onRefresh: () => Promise<boolean>;
    onSelect: (path: string) => Promise<boolean>;
    onCreate: (alias: string, path: string) => Promise<boolean>;
    onConnect: (alias: string, path: string) => Promise<boolean>;
    onRemove: (path: string) => Promise<boolean>;
    onCheck: () => Promise<boolean>;
    onBackup: (destination: string) => Promise<boolean>;
    onRestore: (alias: string, backup: string, destination: string) => Promise<boolean>;
    onRepair: () => Promise<boolean>;
  };

  let {
    copy,
    desktopRuntime,
    snapshot,
    activeWorkspace,
    health,
    loading,
    busy,
    onRefresh,
    onSelect,
    onCreate,
    onConnect,
    onRemove,
    onCheck,
    onBackup,
    onRestore,
    onRepair,
  }: Props = $props();

  let createOpen = $state(false);
  let connectOpen = $state(false);
  let removeOpen = $state(false);
  let restoreOpen = $state(false);
  let createAlias = $state("");
  let createPath = $state("");
  let connectAlias = $state("");
  let connectPath = $state("");
  let pendingRemove = $state<string | null>(null);
  let restoreAlias = $state("");
  let restoreBackup = $state("");
  let restoreDestination = $state("");
  let formError = $state<string | null>(null);

  async function chooseCreatePath(): Promise<void> {
    createPath = (await chooseWorkspaceDirectory()) ?? createPath;
  }

  async function chooseConnectPath(): Promise<void> {
    connectPath = (await chooseWorkspaceDirectory()) ?? connectPath;
  }

  async function submitCreate(): Promise<void> {
    formError = null;
    if (!createAlias.trim()) {
      formError = copy.nameRequired;
      return;
    }
    if (!createPath) {
      formError = copy.pathRequired;
      return;
    }
    if (await onCreate(createAlias.trim(), createPath)) {
      createOpen = false;
      createAlias = "";
      createPath = "";
    }
  }

  async function submitConnect(): Promise<void> {
    formError = null;
    if (!connectAlias.trim()) {
      formError = copy.nameRequired;
      return;
    }
    if (!connectPath) {
      formError = copy.pathRequired;
      return;
    }
    if (await onConnect(connectAlias.trim(), connectPath)) {
      connectOpen = false;
      connectAlias = "";
      connectPath = "";
    }
  }

  async function chooseBackup(): Promise<void> {
    const destination = await chooseWorkspaceDirectory();
    if (destination) await onBackup(destination);
  }

  async function chooseRestoreBackup(): Promise<void> {
    restoreBackup = (await chooseWorkspaceDirectory()) ?? restoreBackup;
  }

  async function chooseRestoreDestination(): Promise<void> {
    restoreDestination =
      (await chooseWorkspaceDirectory()) ?? restoreDestination;
  }

  async function submitRestore(): Promise<void> {
    formError = null;
    if (!restoreAlias.trim()) {
      formError = copy.nameRequired;
      return;
    }
    if (!restoreBackup || !restoreDestination) {
      formError = copy.pathRequired;
      return;
    }
    if (
      await onRestore(
        restoreAlias.trim(),
        restoreBackup,
        restoreDestination,
      )
    ) {
      restoreOpen = false;
      restoreAlias = "";
      restoreBackup = "";
      restoreDestination = "";
    }
  }

  function reviewRemove(path: string): void {
    pendingRemove = path;
    removeOpen = true;
  }

  async function confirmRemove(): Promise<void> {
    if (pendingRemove && (await onRemove(pendingRemove))) {
      removeOpen = false;
      pendingRemove = null;
    }
  }
</script>

<section class="space-y-6">
  <div class="flex flex-col justify-between gap-4 xl:flex-row xl:items-end">
    <div>
      <Badge variant="secondary" class="mb-3">{copy.workspaces}</Badge>
      <h1 class="text-3xl font-semibold tracking-[-0.03em]">{copy.workspaces}</h1>
      <p class="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">
        {copy.workspaceListDescription}
      </p>
    </div>
    <div class="flex flex-wrap gap-2">
      <Button variant="outline" class="min-h-11" disabled={busy} onclick={onRefresh}>
        <RefreshCw size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
        {copy.refresh}
      </Button>
      <Button
        variant="outline"
        class="min-h-11"
        disabled={!desktopRuntime || busy}
        onclick={() => {
          formError = null;
          connectOpen = true;
        }}
      >
        <Link size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
        {copy.connectWorkspace}
      </Button>
      <Button
        variant="outline"
        class="min-h-11"
        disabled={!desktopRuntime || busy}
        onclick={() => {
          formError = null;
          restoreOpen = true;
        }}
      >
        <Archive size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
        {copy.restoreBackup}
      </Button>
      <Button
        class="min-h-11"
        disabled={!desktopRuntime || busy}
        onclick={() => {
          formError = null;
          createOpen = true;
        }}
      >
        <Plus size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
        {copy.createWorkspace}
      </Button>
    </div>
  </div>

  {#if !desktopRuntime}
    <div class="rounded-xl border border-warning/40 bg-warning/10 px-4 py-3 text-sm">
      {copy.unsupportedPreview}
    </div>
  {/if}

  <div class="grid gap-6 xl:grid-cols-[minmax(0,1.25fr)_minmax(320px,0.75fr)]">
    <Card.Root class="shadow-none">
      <Card.Header>
        <Card.Title>{copy.workspaces}</Card.Title>
        <Card.Description>{snapshot?.registry_path ?? copy.localFirst}</Card.Description>
      </Card.Header>
      <Card.Content class="space-y-3">
        {#if loading}
          {#each [1, 2, 3] as row}
            <div class="flex items-center gap-3 rounded-xl border p-4">
              <Skeleton class="size-10 rounded-xl" />
              <div class="flex-1 space-y-2">
                <Skeleton class="h-4 w-40" />
                <Skeleton class="h-3 w-3/4" />
              </div>
            </div>
          {/each}
        {:else if !snapshot?.registry.entries.length}
          <div class="flex min-h-72 flex-col items-center justify-center rounded-xl border border-dashed bg-muted/20 px-8 text-center">
            <div class="grid size-11 place-items-center rounded-xl bg-accent text-accent-foreground">
              <Database size={20} strokeWidth={1.8} aria-hidden="true" />
            </div>
            <h2 class="mt-4 text-base font-semibold">{copy.noRegisteredWorkspaces}</h2>
            <p class="mt-2 max-w-md text-sm leading-6 text-muted-foreground">
              {copy.noRegisteredWorkspacesDescription}
            </p>
          </div>
        {:else}
          {#each snapshot.registry.entries as entry (entry.path)}
            <article
              class="flex flex-col gap-4 rounded-xl border p-4 transition-colors hover:bg-muted/25 sm:flex-row sm:items-center"
            >
              <div class="grid size-10 shrink-0 place-items-center rounded-xl bg-accent text-accent-foreground">
                <Database size={18} strokeWidth={1.8} aria-hidden="true" />
              </div>
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                  <h2 class="truncate text-sm font-semibold">{entry.alias}</h2>
                  {#if activeWorkspace?.path === entry.path}
                    <Badge variant="secondary">{copy.selected}</Badge>
                  {/if}
                </div>
                <p class="mt-1 truncate text-xs text-muted-foreground" title={entry.path}>
                  {entry.path}
                </p>
              </div>
              <div class="flex shrink-0 gap-2">
                <Button
                  variant="outline"
                  class="min-h-10"
                  disabled={busy || activeWorkspace?.path === entry.path}
                  onclick={() => onSelect(entry.path)}
                >
                  {copy.selectWorkspace}
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  class="min-h-10 min-w-10 text-muted-foreground hover:text-destructive"
                  aria-label={`${copy.removeWorkspace}: ${entry.alias}`}
                  title={copy.removeWorkspace}
                  disabled={busy}
                  onclick={() => reviewRemove(entry.path)}
                >
                  <Trash2 size={17} strokeWidth={1.8} aria-hidden="true" />
                </Button>
              </div>
            </article>
          {/each}
        {/if}
      </Card.Content>
    </Card.Root>

    <Card.Root class="shadow-none">
      <Card.Header>
        <Card.Title>{copy.workspaceHealth}</Card.Title>
        <Card.Description>
          {activeWorkspace?.path ?? copy.noWorkspace}
        </Card.Description>
      </Card.Header>
      <Card.Content class="space-y-5">
        {#if activeWorkspace}
          <dl class="grid grid-cols-[1fr_auto] gap-x-4 gap-y-3 text-sm">
            <dt class="text-muted-foreground">{copy.workspaceJobs}</dt>
            <dd class="font-semibold">{activeWorkspace.status.job_count}</dd>
            <dt class="text-muted-foreground">{copy.workspaceArtifacts}</dt>
            <dd class="font-semibold">{activeWorkspace.status.artifact_count}</dd>
            <dt class="text-muted-foreground">{copy.workspaceBlobs}</dt>
            <dd class="font-semibold">{activeWorkspace.status.referenced_blob_count}</dd>
            <dt class="text-muted-foreground">{copy.databaseSchema}</dt>
            <dd class="font-semibold">v{activeWorkspace.status.database_schema_version}</dd>
          </dl>
          <Separator />
          {#if health}
            <div
              class={`rounded-xl border p-3 ${
                health.check.ok ? "border-[var(--success)]" : "border-destructive"
              }`}
              aria-live="polite"
            >
              <div class="flex items-center gap-2 text-sm font-medium">
                <ShieldCheck size={17} strokeWidth={1.8} aria-hidden="true" />
                {health.check.ok ? copy.integrityHealthy : copy.integrityIssues}
              </div>
              {#if health.check.issues.length}
                <p class="mt-2 text-xs leading-5 text-muted-foreground">
                  {health.check.issues[0].message}
                </p>
              {/if}
            </div>
          {/if}
          <div class="grid gap-2 sm:grid-cols-2">
            <Button variant="outline" class="min-h-11" disabled={busy} onclick={onCheck}>
              <HeartPulse size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
              {copy.checkIntegrity}
            </Button>
            <Button variant="outline" class="min-h-11" disabled={busy} onclick={chooseBackup}>
              <Archive size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
              {copy.createBackup}
            </Button>
            <Button
              variant="outline"
              class="min-h-11 sm:col-span-2"
              disabled={busy}
              onclick={onRepair}
            >
              <Wrench size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
              {copy.repairWorkspace}
            </Button>
          </div>
        {:else}
          <div class="flex min-h-72 flex-col items-center justify-center rounded-xl border border-dashed px-6 text-center">
            <FolderOpen size={22} strokeWidth={1.8} class="text-muted-foreground" aria-hidden="true" />
            <p class="mt-3 text-sm font-medium">{copy.noWorkspace}</p>
          </div>
        {/if}
      </Card.Content>
    </Card.Root>
  </div>
</section>

<Dialog.Root bind:open={createOpen}>
  <Dialog.Content class="sm:max-w-lg">
    <Dialog.Header>
      <Dialog.Title>{copy.createWorkspace}</Dialog.Title>
      <Dialog.Description>{copy.createWorkspaceDescription}</Dialog.Description>
    </Dialog.Header>
    <form
      class="space-y-4"
      onsubmit={(event) => {
        event.preventDefault();
        submitCreate();
      }}
    >
      <div class="space-y-2">
        <Label for="create-workspace-alias">{copy.workspaceName}</Label>
        <Input id="create-workspace-alias" bind:value={createAlias} autocomplete="off" />
      </div>
      <div class="space-y-2">
        <Label for="create-workspace-path">{copy.workspacePath}</Label>
        <div class="flex gap-2">
          <Input id="create-workspace-path" bind:value={createPath} readonly />
          <Button type="button" variant="outline" class="shrink-0" onclick={chooseCreatePath}>
            {copy.chooseDirectory}
          </Button>
        </div>
      </div>
      {#if formError}
        <p class="text-sm text-destructive" role="alert">{formError}</p>
      {/if}
      <Dialog.Footer>
        <Button type="button" variant="outline" onclick={() => (createOpen = false)}>
          {copy.cancel}
        </Button>
        <Button type="submit" disabled={busy}>{busy ? copy.working : copy.createWorkspace}</Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root bind:open={connectOpen}>
  <Dialog.Content class="sm:max-w-lg">
    <Dialog.Header>
      <Dialog.Title>{copy.connectWorkspace}</Dialog.Title>
      <Dialog.Description>{copy.connectWorkspaceDescription}</Dialog.Description>
    </Dialog.Header>
    <form
      class="space-y-4"
      onsubmit={(event) => {
        event.preventDefault();
        submitConnect();
      }}
    >
      <div class="space-y-2">
        <Label for="connect-workspace-alias">{copy.workspaceName}</Label>
        <Input id="connect-workspace-alias" bind:value={connectAlias} autocomplete="off" />
      </div>
      <div class="space-y-2">
        <Label for="connect-workspace-path">{copy.workspacePath}</Label>
        <div class="flex gap-2">
          <Input id="connect-workspace-path" bind:value={connectPath} readonly />
          <Button type="button" variant="outline" class="shrink-0" onclick={chooseConnectPath}>
            {copy.chooseDirectory}
          </Button>
        </div>
      </div>
      {#if formError}
        <p class="text-sm text-destructive" role="alert">{formError}</p>
      {/if}
      <Dialog.Footer>
        <Button type="button" variant="outline" onclick={() => (connectOpen = false)}>
          {copy.cancel}
        </Button>
        <Button type="submit" disabled={busy}>{busy ? copy.working : copy.connectWorkspace}</Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root bind:open={restoreOpen}>
  <Dialog.Content class="sm:max-w-lg">
    <Dialog.Header>
      <Dialog.Title>{copy.restoreBackup}</Dialog.Title>
      <Dialog.Description>{copy.restoreBackupDescription}</Dialog.Description>
    </Dialog.Header>
    <form
      class="space-y-4"
      onsubmit={(event) => {
        event.preventDefault();
        submitRestore();
      }}
    >
      <div class="space-y-2">
        <Label for="restore-workspace-alias">{copy.workspaceName}</Label>
        <Input id="restore-workspace-alias" bind:value={restoreAlias} autocomplete="off" />
      </div>
      <div class="space-y-2">
        <Label for="restore-backup-path">{copy.backupDirectory}</Label>
        <div class="flex gap-2">
          <Input id="restore-backup-path" bind:value={restoreBackup} readonly />
          <Button type="button" variant="outline" class="shrink-0" onclick={chooseRestoreBackup}>
            {copy.chooseDirectory}
          </Button>
        </div>
      </div>
      <div class="space-y-2">
        <Label for="restore-destination-path">{copy.restoreDestination}</Label>
        <div class="flex gap-2">
          <Input id="restore-destination-path" bind:value={restoreDestination} readonly />
          <Button
            type="button"
            variant="outline"
            class="shrink-0"
            onclick={chooseRestoreDestination}
          >
            {copy.chooseDirectory}
          </Button>
        </div>
      </div>
      {#if formError}
        <p class="text-sm text-destructive" role="alert">{formError}</p>
      {/if}
      <Dialog.Footer>
        <Button type="button" variant="outline" onclick={() => (restoreOpen = false)}>
          {copy.cancel}
        </Button>
        <Button type="submit" disabled={busy}>{busy ? copy.working : copy.restoreBackup}</Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root bind:open={removeOpen}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>{copy.removeWorkspace}</Dialog.Title>
      <Dialog.Description>{copy.workspaceListDescription}</Dialog.Description>
    </Dialog.Header>
    <p class="break-all rounded-lg bg-muted p-3 text-xs text-muted-foreground">{pendingRemove}</p>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (removeOpen = false)}>{copy.cancel}</Button>
      <Button variant="destructive" disabled={busy} onclick={confirmRemove}>
        {copy.removeWorkspace}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
