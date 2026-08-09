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
  import * as Item from "$lib/components/ui/item/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import { Skeleton } from "$lib/components/ui/skeleton/index.js";
  import {
    chooseWorkspaceDirectory,
    commandErrorMessage,
    migrateWorkspaceV3,
    previewWorkspaceV3Migration,
    type AgentHost,
    type RegistrySnapshot,
    type WorkspaceHealthReadModel,
    type WorkspaceReadModel,
    type WorkspaceV3MigrationPreview,
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
    onCreate: (alias: string, path: string, hosts: AgentHost[]) => Promise<boolean>;
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
  let createCodex = $state(true);
  let createClaude = $state(false);
  let connectAlias = $state("");
  let connectPath = $state("");
  let pendingRemove = $state<string | null>(null);
  let restoreAlias = $state("");
  let restoreBackup = $state("");
  let restoreDestination = $state("");
  let formError = $state<string | null>(null);
  let migrationPreview = $state<WorkspaceV3MigrationPreview | null>(null);
  let migrationBackup = $state("");
  let migrationConsent = $state(false);
  let migrationBusy = $state(false);
  let migrationError = $state<string | null>(null);
  let migrationNotice = $state<string | null>(null);

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
    const hosts: AgentHost[] = [];
    if (createCodex) hosts.push("codex");
    if (createClaude) hosts.push("claude");
    if (await onCreate(createAlias.trim(), createPath, hosts)) {
      createOpen = false;
      createAlias = "";
      createPath = "";
      createCodex = true;
      createClaude = false;
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
    restoreDestination = (await chooseWorkspaceDirectory()) ?? restoreDestination;
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
    if (await onRestore(restoreAlias.trim(), restoreBackup, restoreDestination)) {
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

  async function previewMigration(): Promise<void> {
    if (!activeWorkspace) return;
    migrationBusy = true;
    migrationError = null;
    migrationNotice = null;
    try {
      migrationPreview = (await previewWorkspaceV3Migration(activeWorkspace.path)).data;
      migrationConsent = false;
    } catch (error) {
      migrationError = commandErrorMessage(error);
    } finally {
      migrationBusy = false;
    }
  }

  async function chooseMigrationBackup(): Promise<void> {
    migrationBackup = (await chooseWorkspaceDirectory()) ?? migrationBackup;
  }

  async function migrateWorkspace(): Promise<void> {
    if (!activeWorkspace || !migrationPreview || !migrationBackup || !migrationConsent) return;
    migrationBusy = true;
    migrationError = null;
    migrationNotice = null;
    try {
      const receipt = await migrateWorkspaceV3({
        workspace: activeWorkspace.path,
        expectedPlanSha256: migrationPreview.migration_plan_sha256,
        backupDestination: migrationBackup,
      });
      migrationNotice = receipt.summary;
      migrationPreview = null;
      migrationConsent = false;
      migrationBackup = "";
      await onSelect(activeWorkspace.path);
    } catch (error) {
      migrationError = commandErrorMessage(error);
    } finally {
      migrationBusy = false;
    }
  }
</script>

{#snippet headerActions()}
  <ActionMenu label={copy.workspaceActions} showLabel disabled={busy}>
    <DropdownMenu.Item onclick={onRefresh}>
      <RefreshCw size={16} strokeWidth={1.8} aria-hidden="true" />
      {copy.refresh}
    </DropdownMenu.Item>
    <DropdownMenu.Item
      disabled={!desktopRuntime}
      onclick={() => {
        formError = null;
        connectOpen = true;
      }}
    >
      <Link size={16} strokeWidth={1.8} aria-hidden="true" />
      {copy.connectWorkspace}
    </DropdownMenu.Item>
    <DropdownMenu.Item
      disabled={!desktopRuntime}
      onclick={() => {
        formError = null;
        restoreOpen = true;
      }}
    >
      <Archive size={16} strokeWidth={1.8} aria-hidden="true" />
      {copy.restoreBackup}
    </DropdownMenu.Item>
  </ActionMenu>
  <Button
    class="page-action"
    disabled={!desktopRuntime || busy}
    onclick={() => {
      formError = null;
      createOpen = true;
    }}
  >
    <Plus size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
    {copy.createWorkspace}
  </Button>
{/snippet}

<Page.Root>
  <Page.Header
    eyebrow={copy.workspaces}
    title={copy.workspaces}
    description={copy.workspaceListDescription}
    actions={headerActions}
  />

  {#if !desktopRuntime}
    <Alert.Root variant="warning">
      <Alert.Description>{copy.unsupportedPreview}</Alert.Description>
    </Alert.Root>
  {/if}

  <Page.Grid class="xl:grid-cols-[minmax(0,1.25fr)_minmax(320px,0.75fr)]">
    <Card.Root>
      <Card.Header>
        <Card.Title>{copy.workspaces}</Card.Title>
        <Card.Description>{snapshot?.registry_path ?? copy.localFirst}</Card.Description>
      </Card.Header>
      <Card.Content class="space-y-3">
        {#if loading}
          <Item.Group class="gap-3" aria-label={copy.loading}>
            {#each [1, 2, 3] as row}
              <Item.Root
                variant="outline"
                class="p-[var(--density-panel-padding)]"
                aria-hidden="true"
              >
                <Item.Media>
                  <Skeleton class="size-10 rounded-lg" />
                </Item.Media>
                <Item.Content class="space-y-1">
                  <Skeleton class="h-4 w-40 max-w-full" />
                  <Skeleton class="h-3 w-3/4" />
                </Item.Content>
              </Item.Root>
            {/each}
          </Item.Group>
        {:else if !snapshot?.registry.entries.length}
          <Empty.Root class="min-h-32 border bg-muted/20">
            <Empty.Header>
              <Empty.Media
                variant="icon"
                class="size-11 rounded-lg bg-accent text-accent-foreground"
              >
                <Database size={20} strokeWidth={1.8} aria-hidden="true" />
              </Empty.Media>
              <Empty.Title class="text-base">{copy.noRegisteredWorkspaces}</Empty.Title>
              <Empty.Description>{copy.noRegisteredWorkspacesDescription}</Empty.Description>
            </Empty.Header>
          </Empty.Root>
        {:else}
          <Item.Group class="gap-3">
            {#each snapshot.registry.entries as entry (entry.path)}
              <Item.Root
                variant="outline"
                class="gap-[var(--density-section-gap)] p-[var(--density-panel-padding)] hover:bg-muted/25"
              >
                <Item.Media
                  variant="icon"
                  class="size-10 rounded-lg bg-accent text-accent-foreground"
                >
                  <Database size={18} strokeWidth={1.8} aria-hidden="true" />
                </Item.Media>
                <Item.Content>
                  <Item.Title class="flex-wrap">
                    <h2 class="truncate text-sm font-semibold">{entry.alias}</h2>
                    {#if activeWorkspace?.path === entry.path}
                      <Badge variant="secondary">{copy.selected}</Badge>
                    {/if}
                  </Item.Title>
                  <Item.Description class="truncate text-xs" title={entry.path}>
                    {entry.path}
                  </Item.Description>
                </Item.Content>
                <Item.Actions class="shrink-0">
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
                    size="icon-desktop"
                    class="text-muted-foreground hover:text-destructive"
                    aria-label={`${copy.removeWorkspace}: ${entry.alias}`}
                    title={copy.removeWorkspace}
                    disabled={busy}
                    onclick={() => reviewRemove(entry.path)}
                  >
                    <Trash2 size={17} strokeWidth={1.8} aria-hidden="true" />
                  </Button>
                </Item.Actions>
              </Item.Root>
            {/each}
          </Item.Group>
        {/if}
      </Card.Content>
    </Card.Root>

    <Card.Root>
      <Card.Header>
        <Card.Title>{copy.workspaceHealth}</Card.Title>
        <Card.Description>
          {activeWorkspace?.path ?? copy.noWorkspace}
        </Card.Description>
      </Card.Header>
      <Card.Content class="space-y-[var(--density-section-gap)]">
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
            <Alert.Root variant={health.check.ok ? "success" : "destructive"} aria-live="polite">
              <ShieldCheck size={17} strokeWidth={1.8} aria-hidden="true" />
              <Alert.Title>
                {health.check.ok ? copy.integrityHealthy : copy.integrityIssues}
              </Alert.Title>
              {#if health.check.issues.length}
                <Alert.Description>
                  {health.check.issues[0].message}
                </Alert.Description>
              {/if}
            </Alert.Root>
          {/if}
          <div class="flex flex-wrap items-center gap-2">
            <Button variant="outline" class="min-h-9" disabled={busy} onclick={onCheck}>
              <HeartPulse size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
              {copy.checkIntegrity}
            </Button>
            <ActionMenu label={copy.moreActions} disabled={busy}>
              <DropdownMenu.Item onclick={chooseBackup}>
                <Archive size={16} strokeWidth={1.8} aria-hidden="true" />
                {copy.createBackup}
              </DropdownMenu.Item>
              <DropdownMenu.Item onclick={onRepair}>
                <Wrench size={16} strokeWidth={1.8} aria-hidden="true" />
                {copy.repairWorkspace}
              </DropdownMenu.Item>
            </ActionMenu>
          </div>
          {#if activeWorkspace.status.workspace_format === "canisend.workspace/v2"}
            <Separator />
            <section class="space-y-3" aria-labelledby="workspace-migration-title">
              <div>
                <h3 id="workspace-migration-title" class="text-sm font-semibold">
                  {copy.migrateWorkspace}
                </h3>
                <p class="mt-1 text-xs text-muted-foreground">{copy.migrationDescription}</p>
              </div>
              <Button variant="outline" disabled={busy || migrationBusy} onclick={previewMigration}>
                {copy.previewMigration}
              </Button>
              {#if migrationPreview}
                <dl class="grid grid-cols-[1fr_auto] gap-x-4 gap-y-2 rounded-md border p-3 text-sm">
                  <dt class="text-muted-foreground">{copy.migrationApplications}</dt>
                  <dd>{migrationPreview.application_count}</dd>
                  <dt class="text-muted-foreground">{copy.migrationConflicts}</dt>
                  <dd>{migrationPreview.projection_conflict_count}</dd>
                </dl>
                <p class="break-all font-mono text-xs" aria-label="Migration plan SHA-256">
                  {migrationPreview.migration_plan_sha256}
                </p>
                <div class="space-y-2">
                  <Label for="workspace-migration-backup">{copy.backupDirectory}</Label>
                  <div class="flex gap-2">
                    <Input id="workspace-migration-backup" bind:value={migrationBackup} readonly />
                    <Button type="button" variant="outline" onclick={chooseMigrationBackup}>
                      {copy.chooseDirectory}
                    </Button>
                  </div>
                </div>
                <div class="flex items-start gap-3">
                  <Checkbox id="workspace-migration-consent" bind:checked={migrationConsent} />
                  <Label for="workspace-migration-consent" class="font-normal">
                    {copy.migrationConsent}
                  </Label>
                </div>
                <Button
                  disabled={migrationBusy || !migrationBackup || !migrationConsent}
                  onclick={migrateWorkspace}
                >
                  {copy.migrateWorkspace}
                </Button>
              {/if}
              {#if migrationError}
                <Alert.Root variant="destructive" role="alert" aria-live="assertive">
                  <Alert.Description>{migrationError}</Alert.Description>
                </Alert.Root>
              {/if}
              {#if migrationNotice}
                <Alert.Root variant="success" aria-live="polite">
                  <Alert.Description>{migrationNotice}</Alert.Description>
                </Alert.Root>
              {/if}
            </section>
          {/if}
        {:else}
          <Empty.Root class="min-h-32 border">
            <Empty.Header>
              <Empty.Media variant="icon">
                <FolderOpen size={22} strokeWidth={1.8} aria-hidden="true" />
              </Empty.Media>
              <Empty.Title>{copy.noWorkspace}</Empty.Title>
            </Empty.Header>
          </Empty.Root>
        {/if}
      </Card.Content>
    </Card.Root>
  </Page.Grid>
</Page.Root>

<Dialog.Root bind:open={createOpen}>
  <Dialog.Content class="sm:max-w-lg">
    <Dialog.Header>
      <Dialog.Title>{copy.createWorkspace}</Dialog.Title>
      <Dialog.Description>{copy.createWorkspaceDescription}</Dialog.Description>
    </Dialog.Header>
    <form
      class="space-y-[var(--density-section-gap)]"
      onsubmit={(event) => {
        event.preventDefault();
        submitCreate();
      }}
    >
      <div class="space-y-2">
        <Label for="create-workspace-alias">{copy.workspaceName}</Label>
        <Input id="create-workspace-alias" bind:value={createAlias} autocomplete="off" />
      </div>
      <Alert.Root>
        <Alert.Description>{copy.workflowPackDescription}</Alert.Description>
      </Alert.Root>
      <fieldset class="space-y-3 rounded-lg border p-4">
        <legend class="px-1 text-sm font-semibold">{copy.agentSetup}</legend>
        <p class="text-sm text-muted-foreground">{copy.agentSetupDescription}</p>
        <div class="flex items-start gap-3">
          <Checkbox id="create-workspace-codex" bind:checked={createCodex} class="mt-0.5" />
          <div class="space-y-1">
            <Label for="create-workspace-codex">{copy.codex}</Label>
            <p class="text-xs text-muted-foreground">{copy.codexSetupDescription}</p>
          </div>
        </div>
        <div class="flex items-start gap-3">
          <Checkbox id="create-workspace-claude" bind:checked={createClaude} class="mt-0.5" />
          <div class="space-y-1">
            <Label for="create-workspace-claude">{copy.claude}</Label>
            <p class="text-xs text-muted-foreground">{copy.claudeSetupDescription}</p>
          </div>
        </div>
        <p class="text-xs text-muted-foreground">{copy.agentSetupOptional}</p>
      </fieldset>
      <Alert.Root>
        <ShieldCheck size={16} aria-hidden="true" />
        <Alert.Description>{copy.workspaceBootstrapBoundary}</Alert.Description>
      </Alert.Root>
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
        <Alert.Root variant="destructive" role="alert" aria-live="assertive">
          <Alert.Description>{formError}</Alert.Description>
        </Alert.Root>
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
      class="space-y-[var(--density-section-gap)]"
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
        <Alert.Root variant="destructive">
          <Alert.Description>{formError}</Alert.Description>
        </Alert.Root>
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
      class="space-y-[var(--density-section-gap)]"
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
        <Alert.Root variant="destructive">
          <Alert.Description>{formError}</Alert.Description>
        </Alert.Root>
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

<AlertDialog.Root bind:open={removeOpen}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>{copy.removeWorkspace}</AlertDialog.Title>
      <AlertDialog.Description>{copy.workspaceListDescription}</AlertDialog.Description>
    </AlertDialog.Header>
    <p class="break-all rounded-lg bg-muted p-3 text-xs text-muted-foreground">{pendingRemove}</p>
    <AlertDialog.Footer>
      <AlertDialog.Cancel onclick={() => (removeOpen = false)}>{copy.cancel}</AlertDialog.Cancel>
      <AlertDialog.Action variant="destructive" disabled={busy} onclick={confirmRemove}>
        {copy.removeWorkspace}
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
