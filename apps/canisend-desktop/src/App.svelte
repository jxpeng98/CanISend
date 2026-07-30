<script lang="ts">
  import {
    Activity,
    Bot,
    BriefcaseBusiness,
    Database,
    FileUp,
    GitBranch,
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
  import { onMount, tick } from "svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import WorkspaceContextBar from "$lib/components/WorkspaceContextBar.svelte";
  import {
    archiveJob,
    backupWorkspace,
    beginWorkflowStage,
    buildRender,
    cancelAgentTurn,
    cancelTask,
    checkForUpdates,
    checkPackage,
    checkWorkspace,
    commandErrorCode,
    commandErrorMessage,
    commandErrorRetryable,
    commitJobSourcePreview,
    commitTaskCompletion,
    commitDiscoveryPreview,
    commitWorkflowRerun,
    completeWorkflowStage,
    confirmCriteria,
    confirmPlan,
    confirmProfileEvidence,
    confirmReview,
    connectWorkspace,
    configureCliPath,
    copyAgentHandoff,
    copyAgentMcpConfiguration,
    copyPackageProjection,
    createJob,
    createWorkspace,
    discardDiscoveryPreview,
    discardJobSourcePreview,
    discardWorkflowPreview,
    exportAgentPack,
    exportPackage,
    exportRender,
    exportResourceCatalog,
    exportTaskInputs,
    getAgentCapabilities,
    getAgentContext,
    getAgentRuntimeCatalog,
    getApplicationDossier,
    getCliInstallStatus,
    getContentCatalog,
    getCriteriaTemplate,
    getCurrentPackage,
    getCurrentPackageExport,
    getCurrentMatches,
    getCurrentPlan,
    getCurrentRender,
    getDesktopCliDefaults,
    getDocumentWorkspace,
    getInspectionCatalog,
    getLatestTask,
    getPlanTemplate,
    getProductSummary,
    getDiscoveryAdapters,
    getProfileEvidenceTemplate,
    getResourceDetail,
    getReviewWorkspace,
    getSchemaDetail,
    getWorkflowControls,
    importProfileSource,
    initializeProfile,
    installAgentSkills,
    installCli,
    isDesktopRuntime,
    listApplicationDossiers,
    listJobs,
    listProfileSources,
    listDiscoveryLeads,
    listDiscoverySources,
    listWorkspaces,
    previewDiscoveryFile,
    previewDiscoveryNetwork,
    previewLocalJobSource,
    previewTaskCompletion,
    previewUrlJobSource,
    previewWorkflowRerun,
    prepareAgentHandoff,
    prepareAgentMcpConfiguration,
    prepareTask,
    prepareTaskAgain,
    promoteDiscoveryLead,
    removeWorkspace,
    repairWorkspace,
    reconcilePackage,
    replacePackageProjection,
    restoreWorkspace,
    runDoctor,
    runAgentTurn,
    searchContent,
    selectWorkspace,
    showDiscoveryLead,
    showJob,
    startWorkflow,
    suggestDiscoveryDuplicates,
    uninstallCli,
    type ActionReceipt,
    type AgentCapabilitiesReadModel,
    type AgentContextReadModel,
    type AgentHandoffReadModel,
    type AgentMcpConfigurationReadModel,
    type AgentPackExportReadModel,
    type AgentRuntimeCatalog,
    type AgentRuntimeKind,
    type AgentSkillsInstallReadModel,
    type AgentTurnResult,
    type ApplicationDossierReadModel,
    type CliInstallStatus,
    type ContentCatalogEntryReadModel,
    type ContentCatalogFilter,
    type ContentCatalogReadModel,
    type ContentSearchReadModel,
    type DoctorSummary,
    type DiscoveryAdapterCapabilities,
    type DiscoveryLeadRecord,
    type DiscoveryNetworkAdapter,
    type DiscoveryPreviewReadModel,
    type DiscoverySourceRecord,
    type DiscoverySuggestionReadModel,
    type DesktopCliDefaults,
    type DocumentWorkspaceReadModel,
    type EvidenceCatalogRecord,
    type ExecutionMode,
    type JobDetailReadModel,
    type JobIntakePreviewReadModel,
    type JobRecord,
    type InspectionCatalogReadModel,
    type PackageExportManifestRecord,
    type PackageManifestRecord,
    type PrivacyClassification,
    type ProfileSourceRecord,
    type ProjectionReconcileRecord,
    type ProductSummary,
    type RegisteredAction,
    type RegistrySnapshot,
    type TaskCompletionPreviewReadModel,
    type TaskExecutionMode,
    type TaskOperation,
    type TaskStateData,
    type RenderManifestRecord,
    type ReviewWorkspaceReadModel,
    type UpdateCheckReadModel,
    type WorkflowControlReadModel,
    type WorkflowRerunPreviewReadModel,
    type WorkflowStage,
    type WorkspaceHealthReadModel,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import { upcomingDeadlineApplications } from "$lib/application-dossier";
  import { messages, type Language } from "$lib/i18n";
  import ApplicationsView from "$lib/views/ApplicationsView.svelte";
  import DeliveryView from "$lib/views/DeliveryView.svelte";
  import OpportunitiesView from "$lib/views/OpportunitiesView.svelte";
  import ProfileView from "$lib/views/ProfileView.svelte";
  import SettingsView from "$lib/views/SettingsView.svelte";
  import WorkflowView from "$lib/views/WorkflowView.svelte";
  import WorkspacesView from "$lib/views/WorkspacesView.svelte";
  import {
    defaultNavigationMemory,
    parseNavigationMemory,
    recommendWorkflowRoute,
    rememberedJob,
    routeForContentEntry,
    routeForTaskOperation,
    routeForWorkflowStage,
    type LastSuccessfulAction,
    type NavigationId,
    type WorkflowDetail,
    type WorkflowRoute,
  } from "$lib/workflow-navigation";

  type DecisionKind = "evidence" | "criteria" | "matches" | "plan";
  type AppearancePreferences = {
    language: Language;
    darkMode: boolean;
    compact: boolean;
    reducedMotion: boolean;
    textScale: number;
  };
  type AgentViewComponent = typeof import("$lib/views/AgentView.svelte").default;

  let language = $state<Language>("en");
  let darkMode = $state(false);
  let compact = $state(false);
  let reducedMotion = $state(false);
  let textScale = $state(100);
  let preferencesReady = $state(false);
  let activeView = $state<NavigationId>("today");
  let activeDetail = $state<WorkflowDetail | null>(null);
  let navigationMemory = $state(defaultNavigationMemory());
  let navigationReady = $state(false);
  let lastSuccessfulAction = $state<LastSuccessfulAction | null>(null);
  let AgentView = $state<AgentViewComponent | null>(null);
  let agentViewLoading = $state(false);
  let agentTurnRunning = $state(false);
  let product = $state<ProductSummary | null>(null);
  let doctor = $state<ActionReceipt<DoctorSummary> | null>(null);
  let bridgeError = $state<string | null>(null);
  let bridgeErrorCanRetry = $state(false);
  let notice = $state<string | null>(null);
  let doctorRunning = $state(false);
  let busy = $state(false);
  let workspaceLoading = $state(true);
  let jobsLoading = $state(false);
  let contentLoading = $state(false);
  let discoveryLoading = $state(false);
  let profileLoading = $state(false);
  let registrySnapshot = $state<RegistrySnapshot | null>(null);
  let activeWorkspace = $state<WorkspaceReadModel | null>(null);
  let workspaceHealth = $state<WorkspaceHealthReadModel | null>(null);
  let jobs = $state<JobRecord[]>([]);
  let selectedJob = $state<JobDetailReadModel | null>(null);
  let applicationDossiers = $state<ApplicationDossierReadModel[]>([]);
  let contentCatalog = $state<ContentCatalogReadModel | null>(null);
  let contentSearchResult = $state<ContentSearchReadModel | null>(null);
  let jobIntakePreview = $state<JobIntakePreviewReadModel | null>(null);
  let discoveryAdapters = $state<DiscoveryAdapterCapabilities[]>([]);
  let discoverySources = $state<DiscoverySourceRecord[]>([]);
  let discoveryLeads = $state<DiscoveryLeadRecord[]>([]);
  let selectedDiscoveryLead = $state<DiscoveryLeadRecord | null>(null);
  let discoverySuggestions = $state<DiscoverySuggestionReadModel | null>(null);
  let discoveryPreview = $state<DiscoveryPreviewReadModel | null>(null);
  let profileSources = $state<ProfileSourceRecord[]>([]);
  let profileRevision = $state(0);
  let profileEvidence = $state<EvidenceCatalogRecord | null>(null);
  const desktopRuntime = isDesktopRuntime();
  const appearancePreferenceKey = "canisend.desktop.appearance.v1";
  const navigationPreferenceKey = "canisend.desktop.navigation.v1";
  const supportedTextScales = [100, 125, 150, 200] as const;

  const copy = $derived(messages[language]);
  const selectedJobId = $derived(selectedJob?.job.id ?? "");
  const selectedDossier = $derived(
    applicationDossiers.find((dossier) => dossier.job.id === selectedJobId) ?? null,
  );
  const upcomingDeadlineItems = $derived(
    upcomingDeadlineApplications(applicationDossiers),
  );
  const nearestDeadlineItem = $derived(upcomingDeadlineItems[0] ?? null);
  const recommendation = $derived(
    recommendWorkflowRoute({
      workspacePath: activeWorkspace?.path ?? null,
      jobs,
      selectedJob,
      dossier: selectedDossier,
      profileSourceCount: profileSources.length,
    }),
  );
  const journeyNavigation = $derived([
    {
      id: "opportunities" as const,
      label: copy.opportunities,
      icon: Search,
      enabled: true,
      stage: 1,
    },
    {
      id: "applications" as const,
      label: copy.applications,
      icon: BriefcaseBusiness,
      enabled: true,
      stage: 2,
    },
    {
      id: "profile" as const,
      label: copy.profile,
      icon: UserRound,
      enabled: true,
      stage: 3,
    },
    {
      id: "workflow" as const,
      label: copy.workflow,
      icon: GitBranch,
      enabled: true,
      stage: 4,
    },
    {
      id: "agent" as const,
      label: copy.agent,
      icon: Bot,
      enabled: true,
      stage: 5,
    },
    {
      id: "delivery" as const,
      label: copy.delivery,
      icon: FileUp,
      enabled: true,
      stage: 6,
    },
  ]);
  const utilityNavigation = $derived([
    {
      id: "workspaces" as const,
      label: copy.workspaces,
      icon: Database,
      enabled: true,
    },
    { id: "settings" as const, label: copy.settings, icon: Settings2, enabled: true },
  ]);
  const navigation = $derived([
    {
      id: "today" as const,
      label: copy.today,
      icon: LayoutDashboard,
      enabled: true,
    },
    ...journeyNavigation,
    ...utilityNavigation,
  ]);
  const journeyStages = $derived(
    journeyNavigation.map((item) => ({
      number: item.stage,
      label: item.label,
      view: item.id,
      recommended: recommendation.route.view === item.id,
    })),
  );

  $effect(() => {
    document.documentElement.classList.toggle("dark", darkMode);
    document.documentElement.classList.toggle("reduce-motion", reducedMotion);
    document.documentElement.lang = language;
    document.documentElement.style.fontSize = `${textScale}%`;
    if (preferencesReady) {
      const preferences: AppearancePreferences = {
        language,
        darkMode,
        compact,
        reducedMotion,
        textScale,
      };
      try {
        localStorage.setItem(appearancePreferenceKey, JSON.stringify(preferences));
      } catch {
        // Hardened WebViews may disable storage. In-memory preferences remain usable.
      }
    }
  });

  $effect(() => {
    if (!navigationReady) return;
    const selectedJobs = { ...navigationMemory.selectedJobs };
    if (activeWorkspace && selectedJob) {
      selectedJobs[activeWorkspace.path] = selectedJob.job.id;
    }
    const snapshot = {
      version: 1 as const,
      activeView,
      activeDetail,
      workspacePath: activeWorkspace?.path ?? navigationMemory.workspacePath,
      selectedJobs,
      lastAction: lastSuccessfulAction,
    };
    try {
      localStorage.setItem(navigationPreferenceKey, JSON.stringify(snapshot));
    } catch {
      // Navigation memory is a convenience; the workspace remains authoritative.
    }
  });

  $effect(() => {
    const detail = activeDetail;
    const view = activeView;
    if (!detail) return;
    void tick().then(() => {
      if (activeView !== view || activeDetail !== detail) return;
      document.getElementById(detail)?.scrollIntoView({ block: "start" });
    });
  });

  $effect(() => {
    if (activeView !== "agent" || AgentView || agentViewLoading) return;
    agentViewLoading = true;
    void import("$lib/views/AgentView.svelte")
      .then((module) => {
        AgentView = module.default;
      })
      .catch((error: unknown) => {
        captureBridgeError(error);
      })
      .finally(() => {
        agentViewLoading = false;
      });
  });

  onMount(async () => {
    try {
      navigationMemory = parseNavigationMemory(
        localStorage.getItem(navigationPreferenceKey),
      );
      activeView = navigationMemory.activeView;
      activeDetail = navigationMemory.activeDetail;
      lastSuccessfulAction = navigationMemory.lastAction;
      const stored = localStorage.getItem(appearancePreferenceKey);
      if (stored) {
        const candidate = JSON.parse(stored) as Partial<AppearancePreferences>;
        if (candidate.language === "en" || candidate.language === "zh-CN") {
          language = candidate.language;
        }
        if (typeof candidate.darkMode === "boolean") darkMode = candidate.darkMode;
        if (typeof candidate.compact === "boolean") compact = candidate.compact;
        if (typeof candidate.reducedMotion === "boolean") {
          reducedMotion = candidate.reducedMotion;
        }
        if (
          typeof candidate.textScale === "number" &&
          supportedTextScales.includes(
            candidate.textScale as (typeof supportedTextScales)[number],
          )
        ) {
          textScale = candidate.textScale;
        }
      } else {
        darkMode = window.matchMedia("(prefers-color-scheme: dark)").matches;
        reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      }
    } catch {
      // Storage can be unavailable in hardened WebViews. Defaults remain usable.
    }
    preferencesReady = true;
    navigationReady = true;
    window.addEventListener("keydown", handleAppearanceShortcut);

    if (!desktopRuntime) {
      workspaceLoading = false;
      return;
    }
    try {
      product = await getProductSummary();
      await refreshWorkspaces(true);
    } catch (error) {
      captureBridgeError(error);
      workspaceLoading = false;
    }
  });

  function handleAppearanceShortcut(event: KeyboardEvent): void {
    if (!event.metaKey || event.altKey || event.ctrlKey) return;
    if (event.key === "0") {
      event.preventDefault();
      textScale = 100;
      return;
    }
    if (event.key !== "+" && event.key !== "=" && event.key !== "-") return;
    event.preventDefault();
    const current = supportedTextScales.indexOf(
      textScale as (typeof supportedTextScales)[number],
    );
    const next =
      event.key === "-"
        ? Math.max(0, current - 1)
        : Math.min(supportedTextScales.length - 1, current + 1);
    textScale = supportedTextScales[next] ?? 100;
  }

  function captureBridgeError(error: unknown): void {
    bridgeError = commandErrorMessage(error);
    bridgeErrorCanRetry = commandErrorRetryable(error);
  }

  type SuccessfulActionContext = {
    operation: string;
    route?: WorkflowRoute;
    jobId?: string | null;
    fallbackSummary?: string;
  };

  function extractActionSummary(value: unknown, fallback: string): string {
    if (!value || typeof value !== "object") return fallback;
    const candidate = value as Record<string, unknown>;
    if (typeof candidate.summary === "string" && candidate.summary.trim()) {
      return candidate.summary.trim().slice(0, 240);
    }
    for (const nestedKey of ["action", "started", "committed", "prepared"]) {
      const nested = candidate[nestedKey];
      if (nested && typeof nested === "object") {
        const summary = (nested as Record<string, unknown>).summary;
        if (typeof summary === "string" && summary.trim()) {
          return summary.trim().slice(0, 240);
        }
      }
    }
    return fallback;
  }

  function recordSuccessfulAction(
    context: SuccessfulActionContext,
    result: unknown,
  ): void {
    const jobId = context.jobId === undefined ? selectedJobId || null : context.jobId;
    const route = {
      ...(context.route ?? { view: activeView, detail: activeDetail ?? undefined }),
      jobId: context.route?.jobId ?? jobId ?? undefined,
    };
    lastSuccessfulAction = {
      operation: context.operation,
      summary: extractActionSummary(
        result,
        context.fallbackSummary ?? context.operation,
      ),
      route,
      workspacePath: activeWorkspace?.path ?? null,
      jobId,
      occurredAt: new Date().toISOString(),
    };
  }

  async function navigateTo(route: WorkflowRoute): Promise<void> {
    if (
      route.jobId &&
      activeWorkspace &&
      route.jobId !== selectedJob?.job.id &&
      jobs.some((job) => job.id === route.jobId)
    ) {
      const selected = await handleSelectJob(route.jobId);
      if (!selected) return;
    }
    activeDetail = route.detail ?? null;
    activeView = route.view;
  }

  async function runAction<T>(
    operation: () => Promise<T>,
    success?: SuccessfulActionContext,
  ): Promise<T | null> {
    busy = true;
    bridgeError = null;
    bridgeErrorCanRetry = false;
    notice = null;
    try {
      const result = await operation();
      if (success) recordSuccessfulAction(success, result);
      return result;
    } catch (error) {
      captureBridgeError(error);
      return null;
    } finally {
      busy = false;
    }
  }

  function applyWorkspaceSession(session: RegisteredAction<WorkspaceReadModel>): void {
    registrySnapshot = session.registry;
    const canonicalPath =
      session.registry.registry.default_path ?? session.action.data.path;
    activeWorkspace = { ...session.action.data, path: canonicalPath };
    navigationMemory = {
      ...navigationMemory,
      workspacePath: canonicalPath,
    };
    workspaceHealth = null;
    jobIntakePreview = null;
    contentCatalog = null;
    contentSearchResult = null;
    notice = session.action.summary;
  }

  async function loadJobsForActive(): Promise<void> {
    if (!activeWorkspace) {
      jobs = [];
      selectedJob = null;
      applicationDossiers = [];
      contentCatalog = null;
      contentSearchResult = null;
      jobIntakePreview = null;
      return;
    }
    jobsLoading = true;
    contentLoading = true;
    try {
      const [receipt, dossierReceipt, catalogReceipt] = await Promise.all([
        listJobs(activeWorkspace.path, false),
        listApplicationDossiers(activeWorkspace.path, false),
        getContentCatalog(activeWorkspace.path),
      ]);
      jobs = receipt.data.jobs;
      applicationDossiers = dossierReceipt.data.applications;
      contentCatalog = catalogReceipt.data;
      contentSearchResult = null;
      const currentBelongsToWorkspace =
        selectedJob?.workspace === activeWorkspace.path &&
        jobs.some((job) => job.id === selectedJob?.job.id);
      const nextId = currentBelongsToWorkspace
        ? selectedJob?.job.id
        : rememberedJob(navigationMemory, activeWorkspace.path, jobs);
      selectedJob = nextId
        ? (await showJob(activeWorkspace.path, nextId)).data
        : null;
      if (selectedJob) {
        navigationMemory = {
          ...navigationMemory,
          selectedJobs: {
            ...navigationMemory.selectedJobs,
            [activeWorkspace.path]: selectedJob.job.id,
          },
        };
      }
    } catch (error) {
      captureBridgeError(error);
    } finally {
      jobsLoading = false;
      contentLoading = false;
    }
  }

  async function refreshSelectedJobSnapshot(jobId: string): Promise<void> {
    if (!activeWorkspace || selectedJob?.job.id !== jobId) return;
    try {
      const [jobReceipt, dossierReceipt, catalogReceipt] = await Promise.all([
        showJob(activeWorkspace.path, jobId),
        getApplicationDossier(activeWorkspace.path, jobId),
        getContentCatalog(activeWorkspace.path),
      ]);
      selectedJob = jobReceipt.data;
      applicationDossiers = applicationDossiers.map((dossier) =>
        dossier.job.id === jobId ? dossierReceipt.data : dossier,
      );
      contentCatalog = catalogReceipt.data;
      contentSearchResult = null;
    } catch (error) {
      captureBridgeError(error);
    }
  }

  function clearDiscoverySession(): void {
    discoverySources = [];
    discoveryLeads = [];
    selectedDiscoveryLead = null;
    discoverySuggestions = null;
    discoveryPreview = null;
  }

  async function loadDiscoveryForActive(): Promise<void> {
    if (!activeWorkspace) {
      clearDiscoverySession();
      return;
    }
    discoveryLoading = true;
    try {
      const workspace = activeWorkspace.path;
      const adaptersPromise = discoveryAdapters.length
        ? Promise.resolve(null)
        : getDiscoveryAdapters();
      const [adaptersReceipt, sourcesReceipt, leadsReceipt] = await Promise.all([
        adaptersPromise,
        listDiscoverySources(workspace),
        listDiscoveryLeads(workspace, false),
      ]);
      if (adaptersReceipt) {
        discoveryAdapters = adaptersReceipt.data.adapters;
      }
      discoverySources = sourcesReceipt.data.sources;
      discoveryLeads = leadsReceipt.data.leads;
      const nextId =
        selectedDiscoveryLead &&
        discoveryLeads.some((lead) => lead.id === selectedDiscoveryLead?.id)
          ? selectedDiscoveryLead.id
          : discoveryLeads[0]?.id;
      if (nextId) {
        const [leadReceipt, suggestionsReceipt] = await Promise.all([
          showDiscoveryLead(workspace, nextId),
          suggestDiscoveryDuplicates(workspace, nextId, 5),
        ]);
        selectedDiscoveryLead = leadReceipt.data;
        discoverySuggestions = suggestionsReceipt.data;
      } else {
        selectedDiscoveryLead = null;
        discoverySuggestions = null;
      }
    } catch (error) {
      captureBridgeError(error);
    } finally {
      discoveryLoading = false;
    }
  }

  function clearProfileSession(): void {
    profileSources = [];
    profileRevision = 0;
    profileEvidence = null;
  }

  async function loadProfileForActive(): Promise<void> {
    if (!activeWorkspace) {
      clearProfileSession();
      return;
    }
    profileLoading = true;
    try {
      const receipt = await listProfileSources(activeWorkspace.path);
      profileSources = receipt.data.sources;
      profileRevision = receipt.data.profile_revision;
    } catch (error) {
      captureBridgeError(error);
    } finally {
      profileLoading = false;
    }
  }

  async function loadWorkspaceCollections(): Promise<void> {
    await Promise.all([
      loadJobsForActive(),
      loadDiscoveryForActive(),
      loadProfileForActive(),
    ]);
  }

  async function openWorkspace(path: string): Promise<void> {
    const session = await selectWorkspace(path);
    applyWorkspaceSession(session);
    await loadWorkspaceCollections();
  }

  async function refreshWorkspaces(autoSelect = false): Promise<boolean> {
    if (!desktopRuntime) return false;
    workspaceLoading = true;
    bridgeError = null;
    bridgeErrorCanRetry = false;
    try {
      registrySnapshot = await listWorkspaces();
      const defaultPath = registrySnapshot.registry.default_path;
      const rememberedPath = navigationMemory.workspacePath;
      const initialPath =
        rememberedPath &&
        registrySnapshot.registry.entries.some(
          (entry) => entry.path === rememberedPath,
        )
          ? rememberedPath
          : defaultPath;
      if (initialPath && (autoSelect || !activeWorkspace)) {
        await openWorkspace(initialPath);
      }
      return true;
    } catch (error) {
      captureBridgeError(error);
      return false;
    } finally {
      workspaceLoading = false;
    }
  }

  async function handleSelectWorkspace(path: string): Promise<boolean> {
    const result = await runAction(() => selectWorkspace(path));
    if (!result) return false;
    applyWorkspaceSession(result);
    await loadWorkspaceCollections();
    return true;
  }

  async function handleCreateWorkspace(alias: string, path: string): Promise<boolean> {
    const result = await runAction(() => createWorkspace(alias, path), {
      operation: "workspace.create",
      route: { view: "opportunities", detail: "lead-list" },
      jobId: null,
    });
    if (!result) return false;
    applyWorkspaceSession(result);
    await loadWorkspaceCollections();
    return true;
  }

  async function handleConnectWorkspace(alias: string, path: string): Promise<boolean> {
    const result = await runAction(() => connectWorkspace(alias, path), {
      operation: "workspace.connect",
      route: { view: "today" },
      jobId: null,
    });
    if (!result) return false;
    applyWorkspaceSession(result);
    await loadWorkspaceCollections();
    return true;
  }

  async function handleRemoveWorkspace(path: string): Promise<boolean> {
    const result = await runAction(() => removeWorkspace(path), {
      operation: "workspace.remove",
      route: { view: "workspaces" },
      jobId: null,
    });
    if (!result) return false;
    registrySnapshot = result;
    if (activeWorkspace?.path === path) {
      activeWorkspace = null;
      navigationMemory = {
        ...navigationMemory,
        workspacePath: result.registry.default_path,
      };
      workspaceHealth = null;
      jobs = [];
      selectedJob = null;
      applicationDossiers = [];
      contentCatalog = null;
      contentSearchResult = null;
      jobIntakePreview = null;
      clearDiscoverySession();
      clearProfileSession();
      const fallback = result.registry.default_path;
      if (fallback) await openWorkspace(fallback);
    }
    notice = copy.removeWorkspace;
    return true;
  }

  async function handleCheckWorkspace(): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() => checkWorkspace(activeWorkspace!.path));
    if (!result) return false;
    workspaceHealth = result.data;
    notice = result.summary;
    return true;
  }

  async function handleBackupWorkspace(destination: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(
      () => backupWorkspace(activeWorkspace!.path, destination),
      {
        operation: "workspace.backup",
        route: { view: "workspaces" },
        jobId: null,
      },
    );
    if (!result) return false;
    notice = result.summary;
    return true;
  }

  async function handleRepairWorkspace(): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() => repairWorkspace(activeWorkspace!.path), {
      operation: "workspace.repair",
      route: { view: "workspaces" },
      jobId: null,
    });
    if (!result) return false;
    workspaceHealth = {
      path: result.data.workspace,
      check: result.data.check,
    };
    notice = result.summary;
    return true;
  }

  async function handleRestoreWorkspace(
    alias: string,
    backup: string,
    destination: string,
  ): Promise<boolean> {
    const result = await runAction(
      () => restoreWorkspace(alias, backup, destination),
      {
        operation: "workspace.restore",
        route: { view: "workspaces" },
        jobId: null,
      },
    );
    if (!result) return false;
    registrySnapshot = result.registry;
    activeWorkspace = {
      path:
        result.registry.registry.default_path ?? result.action.data.destination,
      status: result.action.data.workspace,
    };
    workspaceHealth = null;
    await loadWorkspaceCollections();
    notice = result.action.summary;
    return true;
  }

  async function handleRefreshDiscovery(): Promise<boolean> {
    bridgeError = null;
    await loadDiscoveryForActive();
    return bridgeError === null;
  }

  async function handleSelectDiscoveryLead(leadId: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    const workspace = activeWorkspace.path;
    const result = await runAction(() =>
      Promise.all([
        showDiscoveryLead(workspace, leadId),
        suggestDiscoveryDuplicates(workspace, leadId, 5),
      ]),
    );
    if (!result) return false;
    selectedDiscoveryLead = result[0].data;
    discoverySuggestions = result[1].data;
    return true;
  }

  async function handlePreviewDiscoveryFile(options: {
    path: string;
    sourceName?: string;
    sourceUrl?: string;
    hostAgent?: boolean;
    confirmedPrivateRead: boolean;
  }): Promise<boolean> {
    const result = await runAction(() => previewDiscoveryFile(options));
    if (!result) return false;
    discoveryPreview = result;
    notice = result.preview.summary;
    return true;
  }

  async function handlePreviewDiscoveryNetwork(options: {
    adapter: DiscoveryNetworkAdapter;
    endpoint: string;
    sourceName: string;
    organization?: string;
    confirmedNetworkFetch: boolean;
  }): Promise<boolean> {
    const result = await runAction(() => previewDiscoveryNetwork(options));
    if (!result) return false;
    discoveryPreview = result;
    notice = result.preview.summary;
    return true;
  }

  async function handleCommitDiscoveryPreview(): Promise<boolean> {
    if (!activeWorkspace || !discoveryPreview) return false;
    const result = await runAction(
      () =>
        commitDiscoveryPreview(
          activeWorkspace!.path,
          discoveryPreview!.preview_token,
        ),
      {
        operation: "discovery.commit",
        route: { view: "opportunities", detail: "lead-list" },
        jobId: null,
      },
    );
    if (!result) return false;
    discoveryPreview = null;
    await loadDiscoveryForActive();
    notice = result.summary;
    return true;
  }

  async function handleDiscardDiscoveryPreview(): Promise<boolean> {
    if (!discoveryPreview) return false;
    const result = await runAction(() =>
      discardDiscoveryPreview(discoveryPreview!.preview_token),
    );
    if (result === null) return false;
    discoveryPreview = null;
    return true;
  }

  async function handlePromoteDiscoveryLead(leadId: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(
      () => promoteDiscoveryLead(activeWorkspace!.path, leadId),
      {
        operation: "discovery.promote",
        route: { view: "applications", detail: "source-intake" },
        jobId: null,
      },
    );
    if (!result) return false;
    await loadWorkspaceCollections();
    notice = result.summary;
    return true;
  }

  async function handleRefreshProfile(): Promise<boolean> {
    bridgeError = null;
    await loadProfileForActive();
    return bridgeError === null;
  }

  async function handleImportProfileSource(options: {
    source: string;
    sensitivity: PrivacyClassification;
    confirmedPrivateRead: boolean;
  }): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(
      () =>
        importProfileSource({
          workspace: activeWorkspace!.path,
          ...options,
        }),
      {
        operation: "profile.source.import",
        route: { view: "profile", detail: "profile-sources" },
        jobId: null,
      },
    );
    if (!result) return false;
    await loadProfileForActive();
    await loadJobsForActive();
    notice = result.summary;
    return true;
  }

  async function handleInitializeProfile(options: {
    markdown: string;
    sensitivity: PrivacyClassification;
    confirmedPrivateRead: boolean;
  }): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(
      () =>
        initializeProfile({
          workspace: activeWorkspace!.path,
          ...options,
        }),
      {
        operation: "profile.initialize",
        route: { view: "profile", detail: "profile-sources" },
        jobId: null,
      },
    );
    if (!result) return false;
    await loadProfileForActive();
    await loadJobsForActive();
    notice = result.summary;
    return true;
  }

  async function handleLoadProfileEvidence(
    jobId: string,
    confirmedPrivateRead: boolean,
  ): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() =>
      getProfileEvidenceTemplate(
        activeWorkspace!.path,
        jobId,
        confirmedPrivateRead,
      ),
    );
    if (!result) return false;
    profileEvidence = result.data;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.summary;
    return true;
  }

  async function handleConfirmProfileEvidence(
    jobId: string,
    candidate: unknown,
    confirmedPrivateRead: boolean,
  ): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(
      () =>
        confirmProfileEvidence(
          activeWorkspace!.path,
          jobId,
          candidate,
          confirmedPrivateRead,
        ),
      {
        operation: "profile.evidence.confirm",
        route: { view: "workflow", detail: "decision-matches", jobId },
        jobId,
      },
    );
    if (!result) return false;
    profileEvidence = result.data;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.summary;
    return true;
  }

  async function handleLoadWorkflow(
    jobId: string,
  ): Promise<WorkflowControlReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      getWorkflowControls(activeWorkspace!.path, jobId),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleStartWorkflow(
    jobId: string,
  ): Promise<WorkflowControlReadModel | null> {
    if (!activeWorkspace) return null;
    const workspace = activeWorkspace.path;
    const result = await runAction(
      async () => {
        const started = await startWorkflow(workspace, jobId);
        const controls = await getWorkflowControls(workspace, jobId);
        return { started, controls };
      },
      {
        operation: "workflow.start",
        route: { view: "workflow", detail: "workflow-stages", jobId },
        jobId,
      },
    );
    if (!result) return null;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.started.summary;
    return result.controls.data;
  }

  async function handleBeginWorkflowStage(
    jobId: string,
    stage: WorkflowStage,
    mode: ExecutionMode,
  ): Promise<WorkflowControlReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(
      () => beginWorkflowStage(activeWorkspace!.path, jobId, stage, mode),
      {
        operation: "workflow.begin",
        route: { ...routeForWorkflowStage(stage), jobId },
        jobId,
      },
    );
    if (!result) return null;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.summary;
    return result.data;
  }

  async function handleCompleteWorkflowStage(
    jobId: string,
    stage: WorkflowStage,
    artifactId: string,
  ): Promise<WorkflowControlReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(
      () =>
        completeWorkflowStage(
          activeWorkspace!.path,
          jobId,
          stage,
          artifactId,
        ),
      {
        operation: "workflow.complete",
        route: { view: "workflow", detail: "workflow-stages", jobId },
        jobId,
      },
    );
    if (!result) return null;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.summary;
    return result.data;
  }

  async function handlePreviewWorkflowRerun(
    jobId: string,
    stage: WorkflowStage,
  ): Promise<WorkflowRerunPreviewReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      previewWorkflowRerun(activeWorkspace!.path, jobId, stage),
    );
    if (!result) return null;
    notice = result.preview.summary;
    return result;
  }

  async function handleCommitWorkflowRerun(
    previewToken: string,
  ): Promise<WorkflowControlReadModel | null> {
    const result = await runAction(() => commitWorkflowRerun(previewToken), {
      operation: "workflow.rerun",
      route: {
        view: "workflow",
        detail: "workflow-stages",
        jobId: selectedJobId || undefined,
      },
      jobId: selectedJobId || null,
    });
    if (!result) return null;
    if (selectedJobId) await refreshSelectedJobSnapshot(selectedJobId);
    notice = result.summary;
    return result.data;
  }

  async function handleDiscardWorkflowPreview(
    previewToken: string,
  ): Promise<boolean> {
    const result = await runAction(() => discardWorkflowPreview(previewToken));
    return result !== null;
  }

  async function handleLoadDecision(
    jobId: string,
    kind: DecisionKind,
    current: boolean,
    confirmedPrivateRead: boolean,
  ): Promise<unknown | null> {
    if (!activeWorkspace) return null;
    const workspace = activeWorkspace.path;
    if (kind === "evidence") {
      return (
        await runAction(() =>
          getProfileEvidenceTemplate(
            workspace,
            jobId,
            confirmedPrivateRead,
          ),
        )
      )?.data ?? null;
    }
    if (kind === "criteria") {
      return (
        await runAction(() =>
          getCriteriaTemplate(workspace, jobId, confirmedPrivateRead),
        )
      )?.data ?? null;
    }
    if (kind === "matches") {
      return (
        await runAction(() =>
          getCurrentMatches(workspace, jobId, confirmedPrivateRead),
        )
      )?.data ?? null;
    }
    return (
      await runAction(() =>
        current
          ? getCurrentPlan(workspace, jobId, confirmedPrivateRead)
          : getPlanTemplate(workspace, jobId, confirmedPrivateRead),
      )
    )?.data ?? null;
  }

  async function handleConfirmDecision(
    jobId: string,
    kind: Exclude<DecisionKind, "matches">,
    candidate: unknown,
    confirmedPrivateRead: boolean,
  ): Promise<unknown | null> {
    if (!activeWorkspace) return null;
    const workspace = activeWorkspace.path;
    if (kind === "evidence") {
      const result = await runAction(
        () =>
          confirmProfileEvidence(
            workspace,
            jobId,
            candidate,
            confirmedPrivateRead,
          ),
        {
          operation: "profile.evidence.confirm",
          route: { view: "workflow", detail: "decision-matches", jobId },
          jobId,
        },
      );
      if (!result) return null;
      profileEvidence = result.data;
      await refreshSelectedJobSnapshot(jobId);
      notice = result.summary;
      return result.data;
    }
    if (kind === "criteria") {
      const result = await runAction(
        () =>
          confirmCriteria(
            workspace,
            jobId,
            candidate,
            confirmedPrivateRead,
          ),
        {
          operation: "criteria.confirm",
          route: { view: "profile", detail: "profile-evidence", jobId },
          jobId,
        },
      );
      if (!result) return null;
      await refreshSelectedJobSnapshot(jobId);
      notice = result.summary;
      return result.data;
    }
    const result = await runAction(
      () =>
        confirmPlan(
          workspace,
          jobId,
          candidate,
          confirmedPrivateRead,
        ),
      {
        operation: "plan.confirm",
        route: { view: "agent", detail: "agent-task", jobId },
        jobId,
      },
    );
    if (!result) return null;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.summary;
    return result.data;
  }

  async function handleLoadLatestTask(
    jobId: string,
  ): Promise<TaskStateData | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      getLatestTask(activeWorkspace!.path, jobId),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handlePrepareTask(
    jobId: string,
    operation: TaskOperation,
    mode: TaskExecutionMode,
  ): Promise<TaskStateData | null> {
    if (!activeWorkspace) return null;
    const workspace = activeWorkspace.path;
    const result = await runAction(
      async () => {
        const prepared = await prepareTask(workspace, jobId, operation, mode);
        const latest = await getLatestTask(workspace, jobId);
        return { prepared, latest };
      },
      {
        operation: "task.prepare",
        route: { view: "agent", detail: "agent-task", jobId },
        jobId,
      },
    );
    if (!result) return null;
    notice = result.prepared.summary;
    return result.latest.data;
  }

  async function handleExportTaskInputs(options: {
    taskId: string;
    destination: string;
    confirmedPrivateRead: boolean;
    confirmedProviderSend: boolean;
  }): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() =>
      exportTaskInputs({
        workspace: activeWorkspace!.path,
        ...options,
      }),
    );
    if (!result) return false;
    notice = result.summary;
    return true;
  }

  async function handlePreviewTaskCompletion(options: {
    file: string;
    confirmedPrivateRead: boolean;
  }): Promise<TaskCompletionPreviewReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      previewTaskCompletion({
        workspace: activeWorkspace!.path,
        ...options,
      }),
    );
    if (!result) return null;
    notice = result.preview.summary;
    return result;
  }

  async function handleCommitTaskCompletion(
    previewToken: string,
    jobId: string,
  ): Promise<TaskStateData | null> {
    if (!activeWorkspace) return null;
    const workspace = activeWorkspace.path;
    const result = await runAction(
      async () => {
        const committed = await commitTaskCompletion(previewToken);
        const latest = await getLatestTask(workspace, jobId);
        return { committed, latest };
      },
      {
        operation: "task.complete",
        route: { view: "workflow", detail: "agent-task", jobId },
        jobId,
      },
    );
    if (!result) return null;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.committed.summary;
    return result.latest.data;
  }

  async function handleCancelTask(
    taskId: string,
  ): Promise<TaskStateData | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(
      () => cancelTask(activeWorkspace!.path, taskId),
      {
        operation: "task.cancel",
        route: {
          view: "workflow",
          detail: "agent-task",
          jobId: selectedJobId || undefined,
        },
        jobId: selectedJobId || null,
      },
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handlePrepareTaskAgain(
    taskId: string,
    jobId: string,
  ): Promise<TaskStateData | null> {
    if (!activeWorkspace) return null;
    const workspace = activeWorkspace.path;
    const result = await runAction(async () => {
      const prepared = await prepareTaskAgain(workspace, taskId);
      const latest = await getLatestTask(workspace, jobId);
      return { prepared, latest };
    });
    if (!result) return null;
    notice = result.prepared.summary;
    return result.latest.data;
  }

  async function handleLoadAgentCapabilities(): Promise<AgentCapabilitiesReadModel | null> {
    const result = await runAction(getAgentCapabilities);
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleLoadAgentContext(
    jobId?: string,
  ): Promise<AgentContextReadModel | null> {
    const result = await runAction(() =>
      getAgentContext(activeWorkspace?.path, jobId),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleExportAgentPack(
    host: "codex" | "claude" | "generic",
    destination: string,
  ): Promise<AgentPackExportReadModel | null> {
    const result = await runAction(() => exportAgentPack(host, destination));
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handlePrepareAgentHandoff(
    host: "codex" | "claude" | "generic",
    jobId?: string,
  ): Promise<AgentHandoffReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(
      () => prepareAgentHandoff(host, activeWorkspace!.path, jobId),
      {
        operation: "agent.handoff.prepare",
        route: {
          view: "agent",
          detail: "agent-handoff",
          jobId,
        },
        jobId: jobId ?? null,
      },
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleInstallAgentSkills(
    host: "codex" | "claude" | "generic",
  ): Promise<AgentSkillsInstallReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      installAgentSkills(host, activeWorkspace!.path),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleCopyAgentHandoff(
    host: "codex" | "claude" | "generic",
    jobId: string | undefined,
    field: "launch-command" | "start-command" | "bootstrap-prompt",
  ): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(async () => {
      await copyAgentHandoff(host, activeWorkspace!.path, jobId, field);
      return true;
    });
    return result === true;
  }

  async function handlePrepareAgentMcpConfiguration(
    host: "codex" | "claude" | "generic",
  ): Promise<AgentMcpConfigurationReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      prepareAgentMcpConfiguration(host, activeWorkspace!.path),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleCopyAgentMcpConfiguration(
    host: "codex" | "claude" | "generic",
    field: "registration-command" | "configuration-snippet",
  ): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(async () => {
      await copyAgentMcpConfiguration(host, activeWorkspace!.path, field);
      return true;
    });
    return result === true;
  }

  async function handleLoadAgentRuntimes(
    jobId?: string,
  ): Promise<AgentRuntimeCatalog | null> {
    const result = await runAction(() =>
      getAgentRuntimeCatalog(activeWorkspace?.path, jobId),
    );
    if (!result) return null;
    return result;
  }

  async function handleRunAgentTurn(options: {
    jobId?: string;
    runtime: AgentRuntimeKind;
    prompt: string;
    startNew: boolean;
    confirmedProviderSend: boolean;
  }): Promise<AgentTurnResult | null> {
    if (!activeWorkspace) return null;
    busy = true;
    agentTurnRunning = true;
    bridgeError = null;
    bridgeErrorCanRetry = false;
    notice = null;
    try {
      const result = await runAgentTurn({
        workspace: activeWorkspace.path,
        selectedJobId: options.jobId,
        runtime: options.runtime,
        prompt: options.prompt,
        startNew: options.startNew,
        confirmedProviderSend: options.confirmedProviderSend,
      });
      recordSuccessfulAction(
        {
        operation: "agent.turn",
        route: {
          view: "agent",
          detail: "agent-task",
          jobId: options.jobId,
        },
        jobId: options.jobId ?? null,
        },
        result,
      );
      notice = result.resumed
        ? copy.agentSessionResumed
        : copy.agentSessionStarted;
      return result;
    } catch (error) {
      if (commandErrorCode(error) === "agent-runtime-cancelled") {
        notice = copy.agentTurnCancelled;
        return null;
      }
      captureBridgeError(error);
      return null;
    } finally {
      agentTurnRunning = false;
      busy = false;
    }
  }

  async function handleCancelAgentTurn(options: {
    jobId?: string;
    runtime: AgentRuntimeKind;
  }): Promise<boolean> {
    if (!activeWorkspace) return false;
    try {
      const result = await cancelAgentTurn({
        workspace: activeWorkspace.path,
        selectedJobId: options.jobId,
        runtime: options.runtime,
      });
      notice = result.cancellation_requested
        ? copy.agentTurnCancelled
        : copy.noActiveAgentTurn;
      return result.cancellation_requested;
    } catch (error) {
      captureBridgeError(error);
      return false;
    }
  }

  async function handleOpenTaskResult(operation: TaskOperation | string): Promise<void> {
    await navigateTo({
      ...routeForTaskOperation(operation),
      jobId: selectedJobId || undefined,
    });
  }

  async function handleLoadDocuments(
    jobId: string,
    confirmedPrivateRead: boolean,
  ): Promise<DocumentWorkspaceReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      getDocumentWorkspace(
        activeWorkspace!.path,
        jobId,
        confirmedPrivateRead,
      ),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleLoadReview(
    jobId: string,
    confirmedPrivateRead: boolean,
  ): Promise<ReviewWorkspaceReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      getReviewWorkspace(
        activeWorkspace!.path,
        jobId,
        confirmedPrivateRead,
      ),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleConfirmReview(
    jobId: string,
    candidate: unknown,
    confirmedPrivateRead: boolean,
  ): Promise<ReviewWorkspaceReadModel | null> {
    if (!activeWorkspace) return null;
    const workspace = activeWorkspace.path;
    const result = await runAction(
      async () => {
        const confirmed = await confirmReview(
          workspace,
          jobId,
          candidate,
          confirmedPrivateRead,
        );
        const refreshed = await getReviewWorkspace(
          workspace,
          jobId,
          confirmedPrivateRead,
        );
        return { confirmed, refreshed };
      },
      {
        operation: "review.confirm",
        route: { view: "delivery", detail: "delivery-package", jobId },
        jobId,
      },
    );
    if (!result) return null;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.confirmed.summary;
    return result.refreshed.data;
  }

  async function handleCheckPackage(
    jobId: string,
  ): Promise<PackageManifestRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      checkPackage(activeWorkspace!.path, jobId),
    );
    if (!result) return null;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.summary;
    return result.data;
  }

  async function handleLoadPackage(
    jobId: string,
  ): Promise<PackageManifestRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      getCurrentPackage(activeWorkspace!.path, jobId),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleExportPackage(
    jobId: string,
    destination: string,
    confirmedPrivateExport: boolean,
  ): Promise<PackageExportManifestRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(
      () =>
        exportPackage(
          activeWorkspace!.path,
          jobId,
          destination,
          confirmedPrivateExport,
        ),
      {
        operation: "package.export",
        route: { view: "delivery", detail: "delivery-package", jobId },
        jobId,
      },
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleLoadPackageExport(
    jobId: string,
  ): Promise<PackageExportManifestRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      getCurrentPackageExport(activeWorkspace!.path, jobId),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleReconcilePackage(
    jobId: string,
  ): Promise<ProjectionReconcileRecord[] | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      reconcilePackage(activeWorkspace!.path, jobId),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleReplaceProjection(
    jobId: string,
    path: string,
  ): Promise<ProjectionReconcileRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      replacePackageProjection(activeWorkspace!.path, jobId, path),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleCopyProjection(
    jobId: string,
    path: string,
    destination: string,
  ): Promise<ProjectionReconcileRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      copyPackageProjection(
        activeWorkspace!.path,
        jobId,
        path,
        destination,
      ),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleBuildRender(
    jobId: string,
  ): Promise<RenderManifestRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(
      () => buildRender(activeWorkspace!.path, jobId),
      {
        operation: "render.build",
        route: { view: "delivery", detail: "delivery-render", jobId },
        jobId,
      },
    );
    if (!result) return null;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.summary;
    return result.data;
  }

  async function handleLoadRender(
    jobId: string,
  ): Promise<RenderManifestRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      getCurrentRender(activeWorkspace!.path, jobId),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleExportRender(
    jobId: string,
    destination: string,
    confirmedPrivateExport: boolean,
  ): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(
      () =>
        exportRender(
          activeWorkspace!.path,
          jobId,
          destination,
          confirmedPrivateExport,
        ),
      {
        operation: "render.export",
        route: { view: "delivery", detail: "delivery-render", jobId },
        jobId,
      },
    );
    if (!result) return false;
    notice = result.summary;
    return true;
  }

  async function handleLoadCliDefaults(): Promise<DesktopCliDefaults | null> {
    const result = await runAction(getDesktopCliDefaults);
    return result;
  }

  async function handleCheckCli(
    destination?: string,
  ): Promise<CliInstallStatus | null> {
    const result = await runAction(() => getCliInstallStatus(destination));
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleInstallCli(options: {
    destination?: string;
    replaceExisting: boolean;
    confirmedTerminalInstall: boolean;
  }): Promise<CliInstallStatus | null> {
    const result = await runAction(() => installCli(options), {
      operation: "cli.install",
      route: { view: "settings" },
      jobId: null,
    });
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleUninstallCli(options: {
    destination?: string;
    confirmedTerminalInstall: boolean;
  }): Promise<CliInstallStatus | null> {
    const result = await runAction(() => uninstallCli(options), {
      operation: "cli.uninstall",
      route: { view: "settings" },
      jobId: null,
    });
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleConfigureCliPath(options: {
    destination?: string;
    confirmedTerminalInstall: boolean;
  }): Promise<CliInstallStatus | null> {
    const result = await runAction(() => configureCliPath(options), {
      operation: "cli.path.configure",
      route: { view: "settings" },
      jobId: null,
    });
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleCheckUpdates(
    confirmedNetworkFetch: boolean,
  ): Promise<UpdateCheckReadModel | null> {
    const result = await runAction(() =>
      checkForUpdates(confirmedNetworkFetch),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleLoadCatalog(): Promise<InspectionCatalogReadModel | null> {
    const result = await runAction(getInspectionCatalog);
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleLoadSchema(query: string): Promise<unknown | null> {
    const result = await runAction(() => getSchemaDetail(query));
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleLoadResource(query: string): Promise<unknown | null> {
    const result = await runAction(() => getResourceDetail(query));
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleExportCatalog(destination: string): Promise<boolean> {
    const result = await runAction(() => exportResourceCatalog(destination));
    if (!result) return false;
    notice = result.summary;
    return true;
  }

  async function handleRefreshContent(): Promise<boolean> {
    if (!activeWorkspace) return false;
    contentLoading = true;
    bridgeError = null;
    bridgeErrorCanRetry = false;
    try {
      const receipt = await getContentCatalog(activeWorkspace.path);
      contentCatalog = receipt.data;
      contentSearchResult = null;
      return true;
    } catch (error) {
      captureBridgeError(error);
      return false;
    } finally {
      contentLoading = false;
    }
  }

  async function handleSearchContent(options: {
    query: string;
    filter: ContentCatalogFilter;
    includePrivateBodies: boolean;
    confirmedPrivateRead: boolean;
  }): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() =>
      searchContent({
        workspace: activeWorkspace!.path,
        ...options,
      }),
    );
    if (!result) return false;
    contentSearchResult = result.data;
    notice = result.summary;
    return true;
  }

  async function handleOpenContent(
    entry: ContentCatalogEntryReadModel,
  ): Promise<void> {
    await navigateTo(routeForContentEntry(entry, selectedJobId || undefined));
  }

  async function handleRefreshJobs(): Promise<boolean> {
    bridgeError = null;
    await loadJobsForActive();
    return bridgeError === null;
  }

  async function handleCreateJob(title: string, institution: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(
      () => createJob(activeWorkspace!.path, title, institution),
      {
        operation: "job.create",
        route: { view: "applications", detail: "source-intake" },
        jobId: null,
      },
    );
    if (!result) return false;
    await loadJobsForActive();
    await handleSelectJob(result.data.id);
    notice = result.summary;
    return true;
  }

  async function handleSelectJob(jobId: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    if (
      jobIntakePreview &&
      jobIntakePreview.preview.data.job.id !== jobId
    ) {
      await discardJobSourcePreview(jobIntakePreview.preview_token).catch(() => undefined);
      jobIntakePreview = null;
    }
    const result = await runAction(() =>
      Promise.all([
        showJob(activeWorkspace!.path, jobId),
        getApplicationDossier(activeWorkspace!.path, jobId),
      ]),
    );
    if (!result) return false;
    selectedJob = result[0].data;
    if (
      contentSearchResult?.filter.job_id &&
      contentSearchResult.filter.job_id !== jobId
    ) {
      contentSearchResult = null;
    }
    applicationDossiers = applicationDossiers.map((dossier) =>
      dossier.job.id === jobId ? result[1].data : dossier,
    );
    navigationMemory = {
      ...navigationMemory,
      selectedJobs: {
        ...navigationMemory.selectedJobs,
        [activeWorkspace.path]: jobId,
      },
    };
    return true;
  }

  async function handleArchiveJob(jobId: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(
      () => archiveJob(activeWorkspace!.path, jobId),
      {
        operation: "job.archive",
        route: { view: "applications" },
        jobId: null,
      },
    );
    if (!result) return false;
    selectedJob = null;
    if (activeWorkspace) {
      const selectedJobs = { ...navigationMemory.selectedJobs };
      delete selectedJobs[activeWorkspace.path];
      navigationMemory = { ...navigationMemory, selectedJobs };
    }
    jobIntakePreview = null;
    await loadJobsForActive();
    notice = result.summary;
    return true;
  }

  async function handlePreviewLocalSource(
    source: string,
    confirmed: boolean,
  ): Promise<boolean> {
    if (!activeWorkspace || !selectedJob) return false;
    const jobId = selectedJob.job.id;
    const result = await runAction(() =>
      previewLocalJobSource(activeWorkspace!.path, jobId, source, confirmed),
    );
    if (!result) return false;
    jobIntakePreview = result;
    notice = result.preview.summary;
    return true;
  }

  async function handlePreviewUrlSource(
    url: string,
    confirmed: boolean,
  ): Promise<boolean> {
    if (!activeWorkspace || !selectedJob) return false;
    const jobId = selectedJob.job.id;
    const result = await runAction(() =>
      previewUrlJobSource(activeWorkspace!.path, jobId, url, confirmed),
    );
    if (!result) return false;
    jobIntakePreview = result;
    notice = result.preview.summary;
    return true;
  }

  async function handleCommitJobSourcePreview(): Promise<boolean> {
    if (!activeWorkspace || !jobIntakePreview) return false;
    const jobId = jobIntakePreview.preview.data.job.id;
    const result = await runAction(
      () => commitJobSourcePreview(jobIntakePreview!.preview_token),
      {
        operation: "job.source.import",
        route: { view: "profile", detail: "profile-sources", jobId },
        jobId,
      },
    );
    if (!result) return false;
    jobIntakePreview = null;
    await loadJobsForActive();
    await handleSelectJob(jobId);
    notice = result.summary;
    return true;
  }

  async function handleDiscardJobSourcePreview(): Promise<boolean> {
    if (!jobIntakePreview) return false;
    const result = await runAction(() =>
      discardJobSourcePreview(jobIntakePreview!.preview_token),
    );
    if (result === null) return false;
    jobIntakePreview = null;
    notice = copy.discardPreview;
    return true;
  }

  async function handleDoctor(): Promise<void> {
    doctorRunning = true;
    bridgeError = null;
    bridgeErrorCanRetry = false;
    try {
      doctor = await runDoctor();
    } catch (error) {
      captureBridgeError(error);
    } finally {
      doctorRunning = false;
    }
  }

  async function retryCurrentView(): Promise<void> {
    bridgeError = null;
    bridgeErrorCanRetry = false;
    if (activeView === "applications") {
      await handleRefreshJobs();
    } else if (activeView === "opportunities") {
      await handleRefreshDiscovery();
    } else if (activeView === "profile") {
      await handleRefreshProfile();
    } else {
      await refreshWorkspaces(false);
    }
  }
</script>

<svelte:head>
  <title>{copy.appName} — {navigation.find((item) => item.id === activeView)?.label}</title>
</svelte:head>

<div
  class="desktop-shell min-h-screen bg-background text-foreground"
  data-density={compact ? "compact" : "comfortable"}
>
  <a
    href="#main-content"
    class="fixed left-3 top-3 z-50 -translate-y-20 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-transform focus:translate-y-0"
  >
    {copy.skipToContent}
  </a>
  <aside
    class="fixed inset-y-0 left-0 z-20 flex w-64 flex-col overflow-y-auto border-r border-sidebar-border bg-sidebar px-3 py-4 text-sidebar-foreground"
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

    <nav class="space-y-1" aria-label={copy.primaryNavigation}>
      <Button
        variant={activeView === "today" ? "secondary" : "ghost"}
        class="min-h-11 w-full justify-start gap-3 px-3 text-sm"
        aria-current={activeView === "today" ? "page" : undefined}
        onclick={() => void navigateTo({ view: "today" })}
      >
        <LayoutDashboard size={18} strokeWidth={1.8} aria-hidden="true" />
        <span>{copy.today}</span>
      </Button>

      <p class="px-3 pb-1 pt-4 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
        {copy.applicationJourney}
      </p>
      {#each journeyNavigation as item}
        {@const Icon = item.icon}
        <Button
          variant={activeView === item.id ? "secondary" : "ghost"}
          class={[
            "min-h-11 w-full justify-start gap-3 px-3 text-sm",
            recommendation.route.view === item.id && activeView !== item.id
              ? "ring-1 ring-primary/25"
              : "",
          ]}
          aria-current={activeView === item.id ? "page" : undefined}
          disabled={!item.enabled}
          onclick={() => {
            if (item.enabled) void navigateTo({ view: item.id });
          }}
        >
          <span
            class={[
              "grid size-6 place-items-center rounded-md border text-[10px] font-semibold",
              activeView === item.id
                ? "border-primary/25 bg-primary/10 text-primary"
                : "border-sidebar-border text-muted-foreground",
            ]}
            aria-hidden="true"
          >
            {item.stage}
          </span>
          <Icon size={16} strokeWidth={1.8} aria-hidden="true" />
          <span>{item.label}</span>
          {#if recommendation.route.view === item.id}
            <span
              class="ml-auto size-2 rounded-full bg-primary"
              title={copy.nextRecommended}
              aria-label={copy.nextRecommended}
            ></span>
          {/if}
        </Button>
      {/each}

      <p class="px-3 pb-1 pt-4 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
        {copy.system}
      </p>
      {#each utilityNavigation as item}
        {@const Icon = item.icon}
        <Button
          variant={activeView === item.id ? "secondary" : "ghost"}
          class="min-h-11 w-full justify-start gap-3 px-3 text-sm"
          aria-current={activeView === item.id ? "page" : undefined}
          onclick={() => void navigateTo({ view: item.id })}
        >
          <Icon size={18} strokeWidth={1.8} aria-hidden="true" />
          <span>{item.label}</span>
        </Button>
      {/each}
    </nav>

    <div class="mt-auto space-y-3">
      {#if activeWorkspace}
        <button
          type="button"
          class="w-full rounded-xl border border-sidebar-border bg-background/55 p-3 text-left transition-colors hover:bg-background"
          onclick={() => void navigateTo({ view: "workspaces" })}
        >
          <div class="mb-1 flex items-center gap-2 text-xs font-medium">
            <Database size={15} strokeWidth={1.8} aria-hidden="true" />
            <span>{copy.activeWorkspace}</span>
          </div>
          <p class="truncate text-xs text-muted-foreground">
            {registrySnapshot?.registry.entries.find(
              (entry) => entry.path === activeWorkspace?.path,
            )?.alias ?? activeWorkspace.path}
          </p>
        </button>
      {:else}
        <div class="rounded-xl border border-sidebar-border bg-background/55 p-3">
          <div class="mb-2 flex items-center gap-2 text-xs font-medium">
            <ShieldCheck size={15} strokeWidth={1.8} aria-hidden="true" />
            <span>{copy.localFirst}</span>
          </div>
          <p class="text-xs leading-5 text-muted-foreground">{copy.localDescription}</p>
        </div>
      {/if}
      <div class="flex items-center justify-between px-1 text-[11px] text-muted-foreground">
        <span>{product?.version ?? "1.0.0-alpha.5"}</span>
        <Badge variant="outline" class="text-[10px]">Svelte</Badge>
      </div>
    </div>
  </aside>

  <main
    id="main-content"
    class="ml-64 min-h-screen"
    aria-label={copy.mainContent}
    data-testid="canisend-svelte-shell"
  >
    <div class="sticky top-0 z-10 bg-background/94 backdrop-blur">
      <header
        class="flex min-h-14 items-center justify-between border-b px-5 lg:px-8"
        data-tauri-drag-region
      >
        <p class="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
          {navigation.find((item) => item.id === activeView)?.label}
        </p>
        <div class="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon-lg"
            class="min-h-10 min-w-10"
            aria-label={language === "en" ? copy.switchChinese : copy.switchEnglish}
            title={language === "en" ? copy.switchChinese : copy.switchEnglish}
            onclick={() => (language = language === "en" ? "zh-CN" : "en")}
          >
            <Languages size={18} strokeWidth={1.8} aria-hidden="true" />
          </Button>
          <Button
            variant="ghost"
            size="icon-lg"
            class="min-h-10 min-w-10"
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
          <Button variant="outline" class="min-h-10" onclick={() => (compact = !compact)}>
            {compact ? copy.comfortable : copy.compact}
          </Button>
        </div>
      </header>
      <WorkspaceContextBar
        {copy}
        snapshot={registrySnapshot}
        {activeWorkspace}
        {jobs}
        {selectedJob}
        currentViewLabel={navigation.find((item) => item.id === activeView)?.label ?? copy.today}
        stages={journeyStages}
        lastAction={lastSuccessfulAction?.workspacePath ===
          (activeWorkspace?.path ?? null)
          ? lastSuccessfulAction
          : null}
        {busy}
        onSelectWorkspace={handleSelectWorkspace}
        onSelectJob={handleSelectJob}
        onNavigate={navigateTo}
      />
    </div>

    <div class="mx-auto max-w-[1480px] px-5 py-6 lg:px-8 lg:py-8">
      {#if bridgeError}
        <div
          class="mb-6 flex items-center justify-between gap-4 rounded-xl border border-destructive/35 bg-destructive/8 px-4 py-3 text-sm text-destructive"
          role="alert"
        >
          <span>{bridgeError}</span>
          {#if bridgeErrorCanRetry}
            <Button
              variant="outline"
              class="min-h-10 shrink-0"
              disabled={busy}
              onclick={retryCurrentView}
            >
              {copy.retry}
            </Button>
          {/if}
        </div>
      {:else if notice}
        <div
          class="mb-6 rounded-xl border border-[var(--success)]/40 bg-[var(--success)]/10 px-4 py-3 text-sm"
          role="status"
        >
          {notice}
        </div>
      {/if}

      {#if activeView === "workspaces"}
        <WorkspacesView
          {copy}
          {desktopRuntime}
          snapshot={registrySnapshot}
          {activeWorkspace}
          health={workspaceHealth}
          loading={workspaceLoading}
          {busy}
          onRefresh={() => refreshWorkspaces(false)}
          onSelect={handleSelectWorkspace}
          onCreate={handleCreateWorkspace}
          onConnect={handleConnectWorkspace}
          onRemove={handleRemoveWorkspace}
          onCheck={handleCheckWorkspace}
          onBackup={handleBackupWorkspace}
          onRestore={handleRestoreWorkspace}
          onRepair={handleRepairWorkspace}
        />
      {:else if activeView === "opportunities"}
        <OpportunitiesView
          {copy}
          {desktopRuntime}
          {activeWorkspace}
          adapters={discoveryAdapters}
          sources={discoverySources}
          leads={discoveryLeads}
          selectedLead={selectedDiscoveryLead}
          suggestions={discoverySuggestions}
          preview={discoveryPreview}
          loading={discoveryLoading}
          {busy}
          onRefresh={handleRefreshDiscovery}
          onSelect={handleSelectDiscoveryLead}
          onPreviewFile={handlePreviewDiscoveryFile}
          onPreviewNetwork={handlePreviewDiscoveryNetwork}
          onCommitPreview={handleCommitDiscoveryPreview}
          onDiscardPreview={handleDiscardDiscoveryPreview}
          onPromote={handlePromoteDiscoveryLead}
        />
      {:else if activeView === "applications"}
        <ApplicationsView
          {copy}
          {desktopRuntime}
          {activeWorkspace}
          {jobs}
          {selectedJob}
          dossiers={applicationDossiers}
          dossier={selectedDossier}
          {contentCatalog}
          {contentSearchResult}
          focus={activeView === "applications" ? activeDetail : null}
          preview={jobIntakePreview}
          loading={jobsLoading}
          {contentLoading}
          {busy}
          onRefresh={handleRefreshJobs}
          onCreate={handleCreateJob}
          onSelect={handleSelectJob}
          onArchive={handleArchiveJob}
          onPreviewLocal={handlePreviewLocalSource}
          onPreviewUrl={handlePreviewUrlSource}
          onCommitPreview={handleCommitJobSourcePreview}
          onDiscardPreview={handleDiscardJobSourcePreview}
          onRefreshContent={handleRefreshContent}
          onSearchContent={handleSearchContent}
          onOpenContent={handleOpenContent}
          onContinue={() => navigateTo(recommendation.route)}
        />
      {:else if activeView === "workflow"}
        <WorkflowView
          {copy}
          {desktopRuntime}
          {activeWorkspace}
          {selectedJobId}
          focus={activeView === "workflow" ? activeDetail : null}
          {busy}
          onNavigate={navigateTo}
          onOpenTaskResult={handleOpenTaskResult}
          onLoadWorkflow={handleLoadWorkflow}
          onStartWorkflow={handleStartWorkflow}
          onBeginStage={handleBeginWorkflowStage}
          onCompleteStage={handleCompleteWorkflowStage}
          onPreviewRerun={handlePreviewWorkflowRerun}
          onCommitRerun={handleCommitWorkflowRerun}
          onDiscardPreview={handleDiscardWorkflowPreview}
          onLoadDecision={handleLoadDecision}
          onConfirmDecision={handleConfirmDecision}
          onLoadLatestTask={handleLoadLatestTask}
          onPrepareTask={handlePrepareTask}
          onExportTaskInputs={handleExportTaskInputs}
          onPreviewTaskCompletion={handlePreviewTaskCompletion}
          onCommitTaskCompletion={handleCommitTaskCompletion}
          onCancelTask={handleCancelTask}
          onPrepareTaskAgain={handlePrepareTaskAgain}
        />
      {:else if activeView === "delivery"}
        <DeliveryView
          {copy}
          {desktopRuntime}
          {activeWorkspace}
          {selectedJobId}
          focus={activeView === "delivery" ? activeDetail : null}
          {busy}
          onLoadDocuments={handleLoadDocuments}
          onLoadReview={handleLoadReview}
          onConfirmReview={handleConfirmReview}
          onCheckPackage={handleCheckPackage}
          onLoadPackage={handleLoadPackage}
          onExportPackage={handleExportPackage}
          onLoadPackageExport={handleLoadPackageExport}
          onReconcilePackage={handleReconcilePackage}
          onReplaceProjection={handleReplaceProjection}
          onCopyProjection={handleCopyProjection}
          onBuildRender={handleBuildRender}
          onLoadRender={handleLoadRender}
          onExportRender={handleExportRender}
        />
      {:else if activeView === "profile"}
        <ProfileView
          {copy}
          {desktopRuntime}
          {activeWorkspace}
          {selectedJobId}
          focus={activeView === "profile" ? activeDetail : null}
          sources={profileSources}
          {profileRevision}
          evidence={profileEvidence}
          loading={profileLoading}
          {busy}
          onRefresh={handleRefreshProfile}
          onImport={handleImportProfileSource}
          onInitialize={handleInitializeProfile}
          onLoadEvidence={handleLoadProfileEvidence}
          onConfirmEvidence={handleConfirmProfileEvidence}
        />
      {:else if activeView === "agent"}
        {#if AgentView}
          <AgentView
            {copy}
            {desktopRuntime}
            {activeWorkspace}
            {jobs}
            {selectedJobId}
            focus={activeView === "agent" ? activeDetail : null}
            {busy}
            turnRunning={agentTurnRunning}
            onSelectJob={handleSelectJob}
            onNavigate={navigateTo}
            onLoadCapabilities={handleLoadAgentCapabilities}
            onLoadContext={handleLoadAgentContext}
            onPrepareHandoff={handlePrepareAgentHandoff}
            onInstallSkills={handleInstallAgentSkills}
            onCopyHandoff={handleCopyAgentHandoff}
            onPrepareMcpConfiguration={handlePrepareAgentMcpConfiguration}
            onCopyMcpConfiguration={handleCopyAgentMcpConfiguration}
            onLoadRuntimes={handleLoadAgentRuntimes}
            onRunTurn={handleRunAgentTurn}
            onCancelTurn={handleCancelAgentTurn}
            onExport={handleExportAgentPack}
          />
        {:else}
          <div class="flex min-h-72 items-center justify-center" aria-live="polite">
            <p class="text-sm text-muted-foreground">{copy.loading}</p>
          </div>
        {/if}
      {:else if activeView === "settings"}
        <SettingsView
          {copy}
          {desktopRuntime}
          {busy}
          {language}
          {darkMode}
          {compact}
          {reducedMotion}
          {textScale}
          onLanguageChange={(value) => (language = value)}
          onDarkModeChange={(value) => (darkMode = value)}
          onCompactChange={(value) => (compact = value)}
          onReducedMotionChange={(value) => (reducedMotion = value)}
          onTextScaleChange={(value) => (textScale = value)}
          onLoadCliDefaults={handleLoadCliDefaults}
          onCheckCli={handleCheckCli}
          onInstallCli={handleInstallCli}
          onUninstallCli={handleUninstallCli}
          onConfigureCliPath={handleConfigureCliPath}
          onCheckUpdates={handleCheckUpdates}
          onLoadCatalog={handleLoadCatalog}
          onLoadSchema={handleLoadSchema}
          onLoadResource={handleLoadResource}
          onExportCatalog={handleExportCatalog}
        />
      {:else}
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
            <Button
              variant="outline"
              class="min-h-11"
              disabled={!activeWorkspace}
              onclick={() => void navigateTo({ view: "applications", detail: "source-intake" })}
            >
              <FileUp size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
              {copy.importSource}
            </Button>
            <Button
              class="min-h-11"
              disabled={!activeWorkspace}
              onclick={() => void navigateTo({ view: "applications" })}
            >
              <Plus size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
              {copy.newApplication}
            </Button>
          </div>
        </section>

        <section
          class="mt-8 grid gap-[var(--shell-block-gap)] md:grid-cols-2 xl:grid-cols-4"
          aria-label={copy.today}
        >
          <Card.Root class="shadow-none">
            <Card.Header class="p-[var(--shell-card-padding)] pb-2">
              <Card.Description>{copy.activeApplications}</Card.Description>
              <Card.Title class="text-3xl">{jobs.length}</Card.Title>
            </Card.Header>
            <Card.Content class="p-[var(--shell-card-padding)] pt-0 text-sm text-muted-foreground">
              {activeWorkspace ? copy.applicationsDescription : copy.activeDescription}
            </Card.Content>
          </Card.Root>
          <Card.Root class="shadow-none">
            <Card.Header class="p-[var(--shell-card-padding)] pb-2">
              <Card.Description>{copy.upcomingDeadlines}</Card.Description>
              <Card.Title class="text-3xl">{upcomingDeadlineItems.length}</Card.Title>
            </Card.Header>
            <Card.Content class="p-[var(--shell-card-padding)] pt-0 text-sm text-muted-foreground">
              {nearestDeadlineItem
                ? `${copy.nextDeadline}: ${nearestDeadlineItem.metadata.deadline} — ${nearestDeadlineItem.job.title}`
                : copy.noUpcomingDeadlines}
            </Card.Content>
          </Card.Root>
          <Card.Root class="shadow-none">
            <Card.Header class="p-[var(--shell-card-padding)] pb-2">
              <Card.Description>{copy.workflowHealth}</Card.Description>
              <Card.Title class="flex items-center gap-2 text-base">
                <span class="size-2 rounded-full bg-[var(--success)]"></span>
                {workspaceHealth?.check.ok === false ? copy.integrityIssues : copy.healthy}
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
              <Card.Description>
                {selectedDossier
                  ? `${selectedDossier.job.title} — ${selectedDossier.job.institution}`
                  : copy.chooseWorkspaceDescription}
              </Card.Description>
            </Card.Header>
            <Card.Content>
              <div class="flex min-h-48 flex-col items-center justify-center rounded-xl border border-dashed bg-muted/25 px-8 text-center">
                <div class="grid size-11 place-items-center rounded-xl bg-accent text-accent-foreground">
                  <Database size={20} strokeWidth={1.8} aria-hidden="true" />
                </div>
                <h2 class="mt-4 text-base font-semibold">
                  {copy.recommendationTitle[recommendation.reason]}
                </h2>
                <p class="mt-2 max-w-md text-sm leading-6 text-muted-foreground">
                  {selectedDossier?.next_actions[0]?.description ??
                    copy.recommendationDescription[recommendation.reason]}
                </p>
                <Button
                  class="mt-5 min-h-11"
                  onclick={() => void navigateTo(recommendation.route)}
                >
                  {copy.continueNextAction}
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
                  {doctor?.summary ?? copy.diagnosticsReady}
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
      {/if}
    </div>
  </main>
</div>
