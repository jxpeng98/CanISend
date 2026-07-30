<script lang="ts">
  import {
    Boxes,
    Download,
    FolderOpen,
    PackageCheck,
    RefreshCw,
    Settings2,
    ShieldCheck,
    Terminal,
    Trash2,
  } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";
  import {
    chooseExportDirectory,
    type CliInstallStatus,
    type DesktopCliDefaults,
    type InspectionCatalogReadModel,
    type UpdateCheckReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    busy: boolean;
    language: "en" | "zh-CN";
    darkMode: boolean;
    compact: boolean;
    reducedMotion: boolean;
    textScale: number;
    onLanguageChange: (value: "en" | "zh-CN") => void;
    onDarkModeChange: (value: boolean) => void;
    onCompactChange: (value: boolean) => void;
    onReducedMotionChange: (value: boolean) => void;
    onTextScaleChange: (value: number) => void;
    onLoadCliDefaults: () => Promise<DesktopCliDefaults | null>;
    onCheckCli: (destination?: string) => Promise<CliInstallStatus | null>;
    onInstallCli: (options: {
      destination?: string;
      replaceExisting: boolean;
      confirmedTerminalInstall: boolean;
    }) => Promise<CliInstallStatus | null>;
    onUninstallCli: (options: {
      destination?: string;
      confirmedTerminalInstall: boolean;
    }) => Promise<CliInstallStatus | null>;
    onConfigureCliPath: (options: {
      destination?: string;
      confirmedTerminalInstall: boolean;
    }) => Promise<CliInstallStatus | null>;
    onCheckUpdates: (
      confirmedNetworkFetch: boolean,
    ) => Promise<UpdateCheckReadModel | null>;
    onLoadCatalog: () => Promise<InspectionCatalogReadModel | null>;
    onLoadSchema: (query: string) => Promise<unknown | null>;
    onLoadResource: (query: string) => Promise<unknown | null>;
    onExportCatalog: (destination: string) => Promise<boolean>;
  };

  let {
    copy,
    desktopRuntime,
    busy,
    language,
    darkMode,
    compact,
    reducedMotion,
    textScale,
    onLanguageChange,
    onDarkModeChange,
    onCompactChange,
    onReducedMotionChange,
    onTextScaleChange,
    onLoadCliDefaults,
    onCheckCli,
    onInstallCli,
    onUninstallCli,
    onConfigureCliPath,
    onCheckUpdates,
    onLoadCatalog,
    onLoadSchema,
    onLoadResource,
    onExportCatalog,
  }: Props = $props();

  let section = $state("cli");
  let cliDefaults = $state<DesktopCliDefaults | null>(null);
  let cliStatus = $state<CliInstallStatus | null>(null);
  let cliDestination = $state("");
  let replaceExisting = $state(false);
  let terminalConsent = $state(false);
  let uninstallOpen = $state(false);
  let updateConsent = $state(false);
  let update = $state<UpdateCheckReadModel | null>(null);
  let catalog = $state<InspectionCatalogReadModel | null>(null);
  let selectedDetail = $state("");
  let catalogDestination = $state("");
  let formError = $state<string | null>(null);

  async function loadCli(): Promise<void> {
    formError = null;
    cliDefaults = await onLoadCliDefaults();
    if (cliDefaults && !cliDestination) {
      cliDestination = cliDefaults.destination;
    }
    cliStatus = await onCheckCli(cliDestination || cliDefaults?.destination);
  }

  async function installCli(): Promise<void> {
    formError = null;
    if (!terminalConsent) {
      formError = copy.terminalInstallConsent;
      return;
    }
    cliStatus = await onInstallCli({
      destination: cliDestination || undefined,
      replaceExisting,
      confirmedTerminalInstall: terminalConsent,
    });
  }

  async function uninstallCli(): Promise<void> {
    if (!terminalConsent) {
      formError = copy.terminalInstallConsent;
      uninstallOpen = false;
      return;
    }
    cliStatus = await onUninstallCli({
      destination: cliDestination || undefined,
      confirmedTerminalInstall: terminalConsent,
    });
    uninstallOpen = false;
  }

  async function configurePath(): Promise<void> {
    formError = null;
    if (!terminalConsent) {
      formError = copy.terminalInstallConsent;
      return;
    }
    cliStatus = await onConfigureCliPath({
      destination: cliDestination || undefined,
      confirmedTerminalInstall: terminalConsent,
    });
  }

  async function checkUpdates(): Promise<void> {
    formError = null;
    if (!updateConsent) {
      formError = copy.updateConsent;
      return;
    }
    update = await onCheckUpdates(updateConsent);
  }

  async function loadCatalog(): Promise<void> {
    catalog = await onLoadCatalog();
    selectedDetail = "";
  }

  async function loadSchema(query: string): Promise<void> {
    const detail = await onLoadSchema(query);
    if (detail !== null) selectedDetail = JSON.stringify(detail, null, 2);
  }

  async function loadResource(query: string): Promise<void> {
    const detail = await onLoadResource(query);
    if (detail !== null) selectedDetail = JSON.stringify(detail, null, 2);
  }

  async function chooseCatalogDestination(): Promise<void> {
    catalogDestination =
      (await chooseExportDirectory()) ?? catalogDestination;
  }

  async function exportCatalog(): Promise<void> {
    formError = null;
    if (!catalogDestination) {
      formError = copy.catalogDestination;
      return;
    }
    await onExportCatalog(catalogDestination);
  }

  onMount(() => {
    if (desktopRuntime) void loadCli();
  });
</script>

<section class="space-y-6">
  <div>
    <Badge variant="secondary" class="mb-3">{copy.settings}</Badge>
    <h1 class="text-3xl font-semibold tracking-[-0.03em]">{copy.settingsTitle}</h1>
    <p class="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">
      {copy.settingsDescription}
    </p>
  </div>

  <Tabs.Root bind:value={section}>
    <Tabs.List class="grid w-full max-w-3xl grid-cols-4">
      <Tabs.Trigger value="appearance">{copy.appearance}</Tabs.Trigger>
      <Tabs.Trigger value="cli">{copy.cliLifecycle}</Tabs.Trigger>
      <Tabs.Trigger value="updates">{copy.checkUpdates}</Tabs.Trigger>
      <Tabs.Trigger value="inspection">{copy.inspection}</Tabs.Trigger>
    </Tabs.List>

    <Tabs.Content value="appearance" class="pt-4">
      <Card.Root class="shadow-none">
        <Card.Header>
          <Card.Title>{copy.accessibilityAppearance}</Card.Title>
          <Card.Description>{copy.accessibilityAppearanceDescription}</Card.Description>
        </Card.Header>
        <Card.Content class="grid gap-5 md:grid-cols-2">
          <div class="space-y-2">
            <Label for="appearance-language">{copy.language}</Label>
            <select
              id="appearance-language"
              class="flex min-h-11 w-full rounded-md border bg-transparent px-3 py-2 text-sm shadow-xs outline-none focus-visible:ring-2 focus-visible:ring-ring"
              value={language}
              onchange={(event) =>
                onLanguageChange(event.currentTarget.value as "en" | "zh-CN")}
            >
              <option value="en">{copy.english}</option>
              <option value="zh-CN">{copy.simplifiedChinese}</option>
            </select>
          </div>
          <div class="space-y-2">
            <Label for="appearance-text-size">{copy.textSize}</Label>
            <select
              id="appearance-text-size"
              class="flex min-h-11 w-full rounded-md border bg-transparent px-3 py-2 text-sm shadow-xs outline-none focus-visible:ring-2 focus-visible:ring-ring"
              value={textScale}
              onchange={(event) => onTextScaleChange(Number(event.currentTarget.value))}
            >
              {#each [100, 125, 150, 200] as scale}
                <option value={scale}>{scale}%</option>
              {/each}
            </select>
          </div>
          <div class="flex items-start gap-3 rounded-xl border bg-muted/20 p-4">
            <Checkbox
              id="appearance-dark"
              checked={darkMode}
              onCheckedChange={(value) => onDarkModeChange(value === true)}
              class="mt-0.5"
            />
            <Label for="appearance-dark" class="font-normal">{copy.darkMode}</Label>
          </div>
          <div class="flex items-start gap-3 rounded-xl border bg-muted/20 p-4">
            <Checkbox
              id="appearance-compact"
              checked={compact}
              onCheckedChange={(value) => onCompactChange(value === true)}
              class="mt-0.5"
            />
            <Label for="appearance-compact" class="font-normal">{copy.compact}</Label>
          </div>
          <div class="flex items-start gap-3 rounded-xl border bg-muted/20 p-4 md:col-span-2">
            <Checkbox
              id="appearance-motion"
              checked={reducedMotion}
              onCheckedChange={(value) => onReducedMotionChange(value === true)}
              class="mt-0.5"
            />
            <Label for="appearance-motion" class="font-normal">{copy.reduceMotion}</Label>
          </div>
        </Card.Content>
      </Card.Root>
    </Tabs.Content>

    <Tabs.Content value="cli" class="pt-4">
      <Card.Root class="shadow-none">
        <Card.Header>
          <div class="flex items-start justify-between gap-4">
            <div>
              <Card.Title>{copy.cliLifecycle}</Card.Title>
              <Card.Description class="mt-1.5">
                {cliStatus?.state ?? copy.checkCli}
              </Card.Description>
            </div>
            <div class="grid size-10 place-items-center rounded-xl bg-accent text-accent-foreground">
              <Terminal size={18} strokeWidth={1.8} aria-hidden="true" />
            </div>
          </div>
        </Card.Header>
        <Card.Content class="space-y-5">
          <div class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_auto] xl:items-end">
            <div class="space-y-2">
              <Label for="cli-destination">{copy.cliDestination}</Label>
              <Input id="cli-destination" bind:value={cliDestination} />
            </div>
            <Button
              variant="outline"
              class="min-h-11"
              disabled={!desktopRuntime || busy}
              onclick={loadCli}
            >
              <RefreshCw size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
              {copy.checkCli}
            </Button>
          </div>

          {#if cliStatus}
            <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              <div class="rounded-xl border p-4">
                <p class="text-xs text-muted-foreground">{copy.status}</p>
                <p class="mt-2 text-sm font-semibold">{cliStatus.state}</p>
              </div>
              <div class="rounded-xl border p-4">
                <p class="text-xs text-muted-foreground">{copy.bundledCli}</p>
                <p class="mt-2 text-sm font-semibold">{cliStatus.bundled_version}</p>
              </div>
              <div class="rounded-xl border p-4">
                <p class="text-xs text-muted-foreground">{copy.installedCli}</p>
                <p class="mt-2 text-sm font-semibold">{cliStatus.installed_version ?? "—"}</p>
              </div>
              <div class="rounded-xl border p-4">
                <p class="text-xs text-muted-foreground">{copy.pathConfigured}</p>
                <p class="mt-2 text-sm font-semibold">
                  {cliStatus.path_active
                    ? copy.pathActive
                    : cliStatus.path_configured
                      ? copy.pathPending
                      : copy.pathNotConfigured}
                </p>
              </div>
            </div>
            {#if !cliStatus.path_configured}
              <div class="flex flex-col justify-between gap-3 rounded-xl border border-primary/25 bg-primary/5 p-4 md:flex-row md:items-center">
                <div>
                  <p class="text-sm font-semibold">{copy.addToPath}</p>
                  <p class="mt-1 text-xs leading-5 text-muted-foreground">
                    {copy.addToPathDescription}
                  </p>
                </div>
                <Button
                  variant="outline"
                  class="min-h-11 shrink-0"
                  disabled={!desktopRuntime || busy || !terminalConsent}
                  onclick={configurePath}
                >
                  <Terminal
                    size={16}
                    strokeWidth={1.8}
                    data-icon="inline-start"
                    aria-hidden="true"
                  />
                  {copy.addToPath}
                </Button>
              </div>
            {:else if cliStatus.path_configuration_file}
              <div class="rounded-xl border bg-muted/20 p-4">
                <p class="text-xs text-muted-foreground">{copy.pathConfigurationFile}</p>
                <p class="mt-2 break-all font-mono text-xs">
                  {cliStatus.path_configuration_file}
                </p>
              </div>
            {/if}
          {/if}

          <div class="rounded-xl border bg-muted/20 p-4">
            <p class="text-xs text-muted-foreground">{copy.bundledCli}</p>
            <p class="mt-2 break-all font-mono text-xs">
              {cliDefaults?.bundled_source ?? cliStatus?.source_path ?? "—"}
            </p>
          </div>

          <div class="flex items-start gap-3 rounded-xl border bg-muted/20 p-3">
            <Checkbox id="cli-replace" bind:checked={replaceExisting} class="mt-0.5" />
            <Label for="cli-replace" class="text-xs leading-5 font-normal">
              {copy.replaceExistingCli}
            </Label>
          </div>
          <div class="flex items-start gap-3 rounded-xl border bg-muted/20 p-3">
            <Checkbox id="cli-consent" bind:checked={terminalConsent} class="mt-0.5" />
            <Label for="cli-consent" class="text-xs leading-5 font-normal">
              <span class="flex items-center gap-2">
                <ShieldCheck size={14} strokeWidth={1.8} aria-hidden="true" />
                {copy.terminalInstallConsent}
              </span>
            </Label>
          </div>
          <div class="flex flex-wrap gap-2">
            <Button
              class="min-h-11"
              disabled={!desktopRuntime || busy || !terminalConsent}
              onclick={installCli}
            >
              <Download size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
              {copy.installCli}
            </Button>
            <Button
              variant="destructive"
              class="min-h-11"
              disabled={!desktopRuntime || busy || !cliStatus?.managed || !terminalConsent}
              onclick={() => (uninstallOpen = true)}
            >
              <Trash2 size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
              {copy.uninstallCli}
            </Button>
          </div>
        </Card.Content>
      </Card.Root>
    </Tabs.Content>

    <Tabs.Content value="updates" class="pt-4">
      <Card.Root class="shadow-none">
        <Card.Header>
          <div class="flex items-start justify-between gap-4">
            <div>
              <Card.Title>{copy.checkUpdates}</Card.Title>
              <Card.Description class="mt-1.5">
                {update?.channel ?? copy.updateConsent}
              </Card.Description>
            </div>
            <div class="grid size-10 place-items-center rounded-xl bg-accent text-accent-foreground">
              <PackageCheck size={18} strokeWidth={1.8} aria-hidden="true" />
            </div>
          </div>
        </Card.Header>
        <Card.Content class="space-y-5">
          <div class="flex items-start gap-3 rounded-xl border bg-muted/20 p-3">
            <Checkbox id="update-consent" bind:checked={updateConsent} class="mt-0.5" />
            <Label for="update-consent" class="text-xs leading-5 font-normal">
              {copy.updateConsent}
            </Label>
          </div>
          <Button
            class="min-h-11"
            disabled={!desktopRuntime || busy || !updateConsent}
            onclick={checkUpdates}
          >
            {copy.checkUpdates}
          </Button>
          {#if update}
            <div class="grid gap-3 md:grid-cols-3">
              <div class="rounded-xl border p-4">
                <p class="text-xs text-muted-foreground">{copy.version}</p>
                <p class="mt-2 text-lg font-semibold">{update.current_version}</p>
              </div>
              <div class="rounded-xl border p-4">
                <p class="text-xs text-muted-foreground">{copy.latestVersion}</p>
                <p class="mt-2 text-lg font-semibold">{update.latest_version}</p>
              </div>
              <div class="rounded-xl border p-4">
                <p class="text-xs text-muted-foreground">{copy.status}</p>
                <p class="mt-2 text-lg font-semibold">
                  {update.update_available ? copy.updateAvailable : copy.upToDate}
                </p>
              </div>
            </div>
            <div class="rounded-xl border bg-muted/20 p-4">
              <p class="text-sm font-medium">{update.release_name}</p>
              <p class="mt-2 break-all font-mono text-xs text-muted-foreground">
                {update.release_url}
              </p>
            </div>
          {/if}
        </Card.Content>
      </Card.Root>
    </Tabs.Content>

    <Tabs.Content value="inspection" class="pt-4">
      <div class="grid gap-6 xl:grid-cols-[minmax(340px,0.85fr)_minmax(0,1.15fr)]">
        <Card.Root class="shadow-none">
          <Card.Header>
            <div class="flex items-start justify-between gap-4">
              <div>
                <Card.Title>{copy.inspection}</Card.Title>
                <Card.Description class="mt-1.5">
                  {(catalog?.schemas.schemas.length ?? 0) + (catalog?.resources.length ?? 0)}
                </Card.Description>
              </div>
              <div class="grid size-10 place-items-center rounded-xl bg-accent text-accent-foreground">
                <Boxes size={18} strokeWidth={1.8} aria-hidden="true" />
              </div>
            </div>
          </Card.Header>
          <Card.Content class="space-y-4">
            <Button
              variant="outline"
              class="min-h-11"
              disabled={!desktopRuntime || busy}
              onclick={loadCatalog}
            >
              {copy.loadCatalog}
            </Button>
            <div>
              <h2 class="text-sm font-semibold">{copy.schemas}</h2>
              <div class="mt-2 max-h-64 space-y-2 overflow-y-auto">
                {#each catalog?.schemas.schemas ?? [] as schema (schema.id)}
                  <button
                    type="button"
                    class="w-full rounded-xl border p-3 text-left hover:bg-muted/30"
                    onclick={() => loadSchema(schema.id)}
                  >
                    <p class="truncate text-xs font-semibold">{schema.id}</p>
                    <p class="mt-1 truncate font-mono text-[10px] text-muted-foreground">
                      {schema.sha256}
                    </p>
                  </button>
                {/each}
              </div>
            </div>
            <Separator />
            <div>
              <h2 class="text-sm font-semibold">{copy.resources}</h2>
              <div class="mt-2 max-h-64 space-y-2 overflow-y-auto">
                {#each catalog?.resources ?? [] as resource (resource.entry.id)}
                  <button
                    type="button"
                    class="w-full rounded-xl border p-3 text-left hover:bg-muted/30"
                    onclick={() => loadResource(resource.entry.id)}
                  >
                    <div class="flex items-center justify-between gap-3">
                      <p class="truncate text-xs font-semibold">{resource.entry.id}</p>
                      <Badge variant="outline">{resource.entry.kind}</Badge>
                    </div>
                  </button>
                {/each}
              </div>
            </div>
          </Card.Content>
        </Card.Root>

        <div class="space-y-6">
          <Card.Root class="shadow-none">
            <Card.Header>
              <Card.Title>{copy.inspection}</Card.Title>
              <Card.Description>{copy.integrityDigest}</Card.Description>
            </Card.Header>
            <Card.Content>
              <Textarea
                class="min-h-[390px] resize-y font-mono text-xs leading-5"
                value={selectedDetail}
                readonly
                spellcheck={false}
              />
            </Card.Content>
          </Card.Root>
          <Card.Root class="shadow-none">
            <Card.Header>
              <Card.Title>{copy.exportCatalog}</Card.Title>
              <Card.Description>{copy.catalogDestination}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              <div class="flex gap-2">
                <Input bind:value={catalogDestination} />
                <Button variant="outline" class="shrink-0" onclick={chooseCatalogDestination}>
                  <FolderOpen size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                  {copy.chooseDirectory}
                </Button>
              </div>
              <Button
                class="min-h-11"
                disabled={!desktopRuntime || busy || !catalog || !catalogDestination}
                onclick={exportCatalog}
              >
                {copy.exportCatalog}
              </Button>
            </Card.Content>
          </Card.Root>
        </div>
      </div>
    </Tabs.Content>
  </Tabs.Root>

  {#if formError}
    <p class="text-sm text-destructive" role="alert">{formError}</p>
  {/if}
</section>

<Dialog.Root bind:open={uninstallOpen}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>{copy.uninstallCli}</Dialog.Title>
      <Dialog.Description>{copy.terminalInstallConsent}</Dialog.Description>
    </Dialog.Header>
    <div class="rounded-xl border bg-muted/20 p-3">
      <p class="break-all font-mono text-xs">{cliDestination}</p>
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (uninstallOpen = false)}>{copy.cancel}</Button>
      <Button variant="destructive" disabled={busy} onclick={uninstallCli}>
        {copy.uninstallCli}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
