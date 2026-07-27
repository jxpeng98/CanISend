<script lang="ts">
  import {
    Activity,
    Bot,
    BriefcaseBusiness,
    Database,
    FileUp,
    Languages,
    LayoutDashboard,
    Moon,
    Plus,
    Search,
    Settings2,
    ShieldCheck,
    Sun,
    UserRound,
  } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import {
    commandErrorMessage,
    getProductSummary,
    isDesktopRuntime,
    runDoctor,
    type ActionReceipt,
    type DoctorSummary,
    type ProductSummary,
  } from "$lib/bridge";
  import { messages, type Language } from "$lib/i18n";

  type NavigationId =
    | "today"
    | "opportunities"
    | "applications"
    | "profile"
    | "agent"
    | "workspaces"
    | "settings";

  let language = $state<Language>("en");
  let darkMode = $state(false);
  let compact = $state(false);
  let activeView = $state<NavigationId>("today");
  let product = $state<ProductSummary | null>(null);
  let doctor = $state<ActionReceipt<DoctorSummary> | null>(null);
  let bridgeError = $state<string | null>(null);
  let doctorRunning = $state(false);
  const desktopRuntime = isDesktopRuntime();

  const copy = $derived(messages[language]);
  const navigation = $derived([
    { id: "today" as const, label: copy.today, icon: LayoutDashboard, enabled: true },
    {
      id: "opportunities" as const,
      label: copy.opportunities,
      icon: Search,
      enabled: false,
    },
    {
      id: "applications" as const,
      label: copy.applications,
      icon: BriefcaseBusiness,
      enabled: false,
    },
    { id: "profile" as const, label: copy.profile, icon: UserRound, enabled: false },
    { id: "agent" as const, label: copy.agent, icon: Bot, enabled: false },
    {
      id: "workspaces" as const,
      label: copy.workspaces,
      icon: Database,
      enabled: false,
    },
    { id: "settings" as const, label: copy.settings, icon: Settings2, enabled: false },
  ]);

  $effect(() => {
    document.documentElement.classList.toggle("dark", darkMode);
    document.documentElement.lang = language;
  });

  onMount(async () => {
    if (!desktopRuntime) return;
    try {
      product = await getProductSummary();
    } catch (error) {
      bridgeError = commandErrorMessage(error);
    }
  });

  async function handleDoctor(): Promise<void> {
    doctorRunning = true;
    bridgeError = null;
    try {
      doctor = await runDoctor();
    } catch (error) {
      bridgeError = commandErrorMessage(error);
    } finally {
      doctorRunning = false;
    }
  }
</script>

<svelte:head>
  <title>{copy.appName} — {copy.today}</title>
</svelte:head>

<div
  class="desktop-shell min-h-screen bg-background text-foreground"
  data-density={compact ? "compact" : "comfortable"}
>
  <aside
    class="fixed inset-y-0 left-0 z-20 flex w-64 flex-col border-r border-sidebar-border bg-sidebar px-3 py-4 text-sidebar-foreground"
    aria-label={copy.appName}
  >
    <div class="flex min-h-14 items-center gap-3 px-2">
      <div
        class="grid size-10 place-items-center rounded-xl bg-sidebar-primary text-sidebar-primary-foreground"
        aria-hidden="true"
      >
        <BriefcaseBusiness size={20} strokeWidth={1.8} />
      </div>
      <div class="min-w-0">
        <p class="truncate text-sm font-semibold tracking-tight">{copy.appName}</p>
        <p class="truncate text-xs text-muted-foreground">{copy.appTagline}</p>
      </div>
    </div>

    <Separator class="my-4 bg-sidebar-border" />

    <nav class="space-y-1" aria-label={copy.appTagline}>
      {#each navigation as item}
        {@const Icon = item.icon}
        <Button
          variant={activeView === item.id ? "secondary" : "ghost"}
          class="min-h-11 w-full justify-start gap-3 px-3 text-sm"
          aria-current={activeView === item.id ? "page" : undefined}
          disabled={!item.enabled}
          onclick={() => {
            if (item.enabled) activeView = item.id;
          }}
        >
          <Icon size={18} strokeWidth={1.8} aria-hidden="true" />
          <span>{item.label}</span>
          {#if !item.enabled}
            <span class="ml-auto text-[10px] font-normal text-muted-foreground">TS2+</span>
          {/if}
        </Button>
      {/each}
    </nav>

    <div class="mt-auto space-y-3">
      <div class="rounded-xl border border-sidebar-border bg-background/55 p-3">
        <div class="mb-2 flex items-center gap-2 text-xs font-medium">
          <ShieldCheck size={15} strokeWidth={1.8} aria-hidden="true" />
          <span>{copy.localFirst}</span>
        </div>
        <p class="text-xs leading-5 text-muted-foreground">{copy.localDescription}</p>
      </div>
      <div class="flex items-center justify-between px-1 text-[11px] text-muted-foreground">
        <span>{product?.version ?? "1.0.0-alpha.3"}</span>
        <Badge variant="outline" class="text-[10px]">{copy.preview}</Badge>
      </div>
    </div>
  </aside>

  <main class="ml-64 min-h-screen">
    <header
      class="sticky top-0 z-10 flex min-h-16 items-center justify-between border-b bg-background/92 px-8 backdrop-blur"
      data-tauri-drag-region
    >
      <div>
        <p class="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
          {copy.pageEyebrow}
        </p>
      </div>
      <div class="flex items-center gap-2">
        <Button
          variant="ghost"
          size="icon-lg"
          class="min-h-11 min-w-11"
          aria-label={language === "en" ? copy.switchChinese : copy.switchEnglish}
          title={language === "en" ? copy.switchChinese : copy.switchEnglish}
          onclick={() => (language = language === "en" ? "zh-CN" : "en")}
        >
          <Languages size={18} strokeWidth={1.8} aria-hidden="true" />
        </Button>
        <Button
          variant="ghost"
          size="icon-lg"
          class="min-h-11 min-w-11"
          aria-label={darkMode ? copy.lightMode : copy.darkMode}
          title={darkMode ? copy.lightMode : copy.darkMode}
          onclick={() => (darkMode = !darkMode)}
        >
          {#if darkMode}
            <Sun size={18} strokeWidth={1.8} aria-hidden="true" />
          {:else}
            <Moon size={18} strokeWidth={1.8} aria-hidden="true" />
          {/if}
        </Button>
        <Button
          variant="outline"
          class="min-h-11"
          onclick={() => (compact = !compact)}
        >
          {compact ? copy.comfortable : copy.compact}
        </Button>
      </div>
    </header>

    <div class="mx-auto max-w-[1480px] px-8 py-8">
      <section class="flex flex-col items-start justify-between gap-6 2xl:flex-row 2xl:gap-8">
        <div class="max-w-3xl">
          <Badge variant="secondary" class="mb-4">{copy.today}</Badge>
          <h1 class="text-balance text-4xl font-semibold tracking-[-0.035em]">
            {copy.pageTitle}
          </h1>
          <p class="mt-4 max-w-2xl text-pretty text-base leading-7 text-muted-foreground">
            {copy.pageDescription}
          </p>
        </div>
        <div class="flex shrink-0 flex-wrap items-center gap-2">
          <Button variant="outline" class="min-h-11" disabled>
            <FileUp size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
            {copy.importSource}
          </Button>
          <Button class="min-h-11" disabled>
            <Plus size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
            {copy.newApplication}
          </Button>
        </div>
      </section>

      {#if bridgeError}
        <div
          class="mt-6 rounded-xl border border-destructive/35 bg-destructive/8 px-4 py-3 text-sm text-destructive"
          role="alert"
        >
          {copy.bridgeUnavailable}
          <span class="mt-1 block text-xs opacity-80">{bridgeError}</span>
        </div>
      {/if}

      <section
        class="mt-8 grid gap-[var(--shell-block-gap)] md:grid-cols-2 xl:grid-cols-4"
        aria-label={copy.today}
      >
        <Card.Root class="shadow-none">
          <Card.Header class="p-[var(--shell-card-padding)] pb-2">
            <Card.Description>{copy.activeApplications}</Card.Description>
            <Card.Title class="text-3xl">0</Card.Title>
          </Card.Header>
          <Card.Content class="p-[var(--shell-card-padding)] pt-0 text-sm text-muted-foreground">
            {copy.activeDescription}
          </Card.Content>
        </Card.Root>

        <Card.Root class="shadow-none">
          <Card.Header class="p-[var(--shell-card-padding)] pb-2">
            <Card.Description>{copy.upcomingDeadlines}</Card.Description>
            <Card.Title class="text-3xl">—</Card.Title>
          </Card.Header>
          <Card.Content class="p-[var(--shell-card-padding)] pt-0 text-sm text-muted-foreground">
            {copy.deadlineDescription}
          </Card.Content>
        </Card.Root>

        <Card.Root class="shadow-none">
          <Card.Header class="p-[var(--shell-card-padding)] pb-2">
            <Card.Description>{copy.workflowHealth}</Card.Description>
            <Card.Title class="flex items-center gap-2 text-base">
              <span class="size-2 rounded-full bg-[var(--success)]"></span>
              {copy.healthy}
            </Card.Title>
          </Card.Header>
          <Card.Content class="p-[var(--shell-card-padding)] pt-0 text-sm text-muted-foreground">
            {copy.healthDescription}
          </Card.Content>
        </Card.Root>

        <Card.Root class="shadow-none">
          <Card.Header class="p-[var(--shell-card-padding)] pb-2">
            <Card.Description>{copy.localFirst}</Card.Description>
            <Card.Title class="flex items-center gap-2 text-base">
              <ShieldCheck size={18} strokeWidth={1.8} aria-hidden="true" />
              {copy.healthy}
            </Card.Title>
          </Card.Header>
          <Card.Content class="p-[var(--shell-card-padding)] pt-0 text-sm text-muted-foreground">
            {copy.localDescription}
          </Card.Content>
        </Card.Root>
      </section>

      <section class="mt-8 grid gap-6 xl:grid-cols-[1.3fr_0.9fr]">
        <Card.Root class="shadow-none">
          <Card.Header>
            <Card.Title>{copy.nextActions}</Card.Title>
            <Card.Description>{copy.chooseWorkspaceDescription}</Card.Description>
          </Card.Header>
          <Card.Content>
            <div class="flex min-h-48 flex-col items-center justify-center rounded-xl border border-dashed bg-muted/25 px-8 text-center">
              <div class="grid size-11 place-items-center rounded-xl bg-accent text-accent-foreground">
                <Database size={20} strokeWidth={1.8} aria-hidden="true" />
              </div>
              <h2 class="mt-4 text-base font-semibold">{copy.chooseWorkspace}</h2>
              <p class="mt-2 max-w-md text-sm leading-6 text-muted-foreground">
                {copy.chooseWorkspaceDescription}
              </p>
              <Button class="mt-5 min-h-11" variant="outline" disabled>
                {copy.openWorkspaces}
              </Button>
            </div>
          </Card.Content>
        </Card.Root>

        <Card.Root class="shadow-none">
          <Card.Header>
            <div class="flex items-center justify-between gap-4">
              <div>
                <Card.Title>{copy.diagnostics}</Card.Title>
                <Card.Description class="mt-1.5">{copy.diagnosticsDescription}</Card.Description>
              </div>
              <div class="grid size-10 shrink-0 place-items-center rounded-xl bg-accent text-accent-foreground">
                <Activity size={19} strokeWidth={1.8} aria-hidden="true" />
              </div>
            </div>
          </Card.Header>
          <Card.Content class="space-y-4">
            <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
              <dt class="text-muted-foreground">{copy.version}</dt>
              <dd class="truncate text-right font-medium">{product?.version ?? "—"}</dd>
              <dt class="text-muted-foreground">{copy.protocol}</dt>
              <dd class="truncate text-right font-medium">{product?.protocol ?? "—"}</dd>
              <dt class="text-muted-foreground">{copy.platform}</dt>
              <dd class="truncate text-right font-medium">
                {product ? `${product.target_os} / ${product.target_arch}` : "—"}
              </dd>
            </dl>
            <Separator />
            <div class="flex items-center justify-between gap-4">
              <p class="text-sm text-muted-foreground" aria-live="polite">
                {#if doctor}
                  {doctor.summary}
                {:else}
                  {copy.diagnosticsReady}
                {/if}
              </p>
              <Button
                variant="outline"
                class="min-h-11 shrink-0"
                disabled={doctorRunning || !desktopRuntime}
                onclick={handleDoctor}
              >
                {doctorRunning ? copy.runningDiagnostics : copy.runDiagnostics}
              </Button>
            </div>
          </Card.Content>
        </Card.Root>
      </section>
    </div>
  </main>
</div>
