<script lang="ts">
  import {
    Bot,
    BriefcaseBusiness,
    Database,
    Languages,
    LayoutDashboard,
    Moon,
    Search,
    Settings2,
    Sun,
    UserRound,
    X,
  } from "@lucide/svelte";
  import { onMount, tick } from "svelte";

  import * as Alert from "$lib/components/ui/alert/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import * as Sidebar from "$lib/components/ui/sidebar/index.js";
  import LoadingPanel from "$lib/components/patterns/LoadingPanel.svelte";
  import * as Page from "$lib/components/patterns/page/index.js";
  import { agentUiState } from "$lib/agent-state.svelte";
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
    exportRenderAndOpen,
    exportResourceCatalog,
    exportTaskInputs,
    getAgentAssistance,
    getAgentCapabilities,
    getAgentContext,
    getAgentRuntimeCatalog,
    getAgentSkillsStatus,
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
    getWorkflowPackPresentation,
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
    listProfileSources,
    listDiscoveryLeads,
    listDiscoverySources,
    listWorkspaces,
    previewDiscoveryFile,
    previewDiscoveryNetwork,
    previewLocalJobSource,
    previewRender,
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
    startWorkflow,
    suggestDiscoveryDuplicates,
    uninstallCli,
    uninstallAgentSkills,
    type ActionReceipt,
    ACADEMIC_JOB_WORKFLOW_PACK_ID,
    GENERIC_APPLICATION_WORKFLOW_PACK_ID,
    type AgentAssistanceReadModel,
    type BuiltInWorkflowPackId,
    type AgentCapabilitiesReadModel,
    type AgentContextReadModel,
    type AgentHandoffReadModel,
    type AgentHost,
    type AgentSkillsInstallScope,
    type AgentMcpConfigurationReadModel,
    type AgentPackExportReadModel,
    type AgentRuntimeCatalog,
    type AgentRuntimeKind,
    type AgentSkillsInstallReadModel,
    type AgentSkillsStatusReadModel,
    type AgentSkillsUninstallReadModel,
    type AgentTurnResult,
    type ApplicationDossierReadModel,
    type ApplicationFlowStageV3,
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
    type DocumentKind,
    type DocumentWorkspaceReadModel,
    type EvidenceCatalogRecord,
    type ExecutionMode,
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
    type StoredApplicationModelV3,
    type TaskCompletionPreviewReadModel,
    type TaskExecutionMode,
    type TaskOperation,
    type TaskStateData,
    type RenderManifestRecord,
    type ReviewWorkspaceReadModel,
    type UpdateCheckReadModel,
    type WorkflowControlReadModel,
    type WorkflowPackPresentationReadModel,
    type WorkflowRerunPreviewReadModel,
    type WorkflowStage,
    type WorkspaceHealthReadModel,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import { upcomingDeadlineApplications } from "$lib/application-dossier";
  import { messages, type Language } from "$lib/i18n";
  import {
    applicationSectionForRoute,
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
  type GenericApplicationsViewComponent =
    typeof import("$lib/views/GenericApplicationsView.svelte").default;
  type WorkflowViewComponent = typeof import("$lib/views/WorkflowView.svelte").default;
  type DeliveryViewComponent = typeof import("$lib/views/DeliveryView.svelte").default;
  type OpportunitiesViewComponent = typeof import("$lib/views/OpportunitiesView.svelte").default;
  type ProfileViewComponent = typeof import("$lib/views/ProfileView.svelte").default;
  type SettingsViewComponent = typeof import("$lib/views/SettingsView.svelte").default;
  type WorkspacesViewComponent = typeof import("$lib/views/WorkspacesView.svelte").default;
  type TodayViewComponent = typeof import("$lib/views/TodayView.svelte").default;

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
  let GenericApplicationsView = $state<GenericApplicationsViewComponent | null>(null);
  let WorkflowView = $state<WorkflowViewComponent | null>(null);
  let DeliveryView = $state<DeliveryViewComponent | null>(null);
  let OpportunitiesView = $state<OpportunitiesViewComponent | null>(null);
  let ProfileView = $state<ProfileViewComponent | null>(null);
  let SettingsView = $state<SettingsViewComponent | null>(null);
  let WorkspacesView = $state<WorkspacesViewComponent | null>(null);
  let TodayView = $state<TodayViewComponent | null>(null);
  let agentViewLoading = $state(false);
  let applicationsViewLoading = $state(false);
  let workflowViewLoading = $state(false);
  let deliveryViewLoading = $state(false);
  let opportunitiesViewLoading = $state(false);
  let profileViewLoading = $state(false);
  let settingsViewLoading = $state(false);
  let workspacesViewLoading = $state(false);
  let todayViewLoading = $state(false);
  let agentViewFailed = $state(false);
  let applicationsViewFailed = $state(false);
  let workflowViewFailed = $state(false);
  let deliveryViewFailed = $state(false);
  let opportunitiesViewFailed = $state(false);
  let profileViewFailed = $state(false);
  let settingsViewFailed = $state(false);
  let workspacesViewFailed = $state(false);
  let todayViewFailed = $state(false);
  let agentTurnRunning = $state(false);
  let product = $state<ProductSummary | null>(null);
  let workflowPackPresentation = $state<WorkflowPackPresentationReadModel | null>(null);
  let workflowPackPresentationRequest = 0;
  let doctor = $state<ActionReceipt<DoctorSummary> | null>(null);
  let bridgeError = $state<string | null>(null);
  let bridgeErrorCanRetry = $state(false);
  let notice = $state<string | null>(null);
  let noticeRoute = $state<WorkflowRoute | null>(null);
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
  let selectedJob = $state<ApplicationDossierReadModel | null>(null);
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
  let activePackId = $state<BuiltInWorkflowPackId>(GENERIC_APPLICATION_WORKFLOW_PACK_ID);
  let v4Applications = $state<StoredApplicationModelV3[]>([]);
  let selectedV4Application = $state<StoredApplicationModelV3 | null>(null);
  let v4Stages = $state<ApplicationFlowStageV3[]>([]);
  let requestedV4ApplicationId = $state("");
  const selectedDossier = $derived(
    applicationDossiers.find((dossier) => dossier.job.id === selectedJobId) ?? null,
  );
  const upcomingDeadlineItems = $derived(upcomingDeadlineApplications(applicationDossiers));
  const nearestDeadlineItem = $derived(upcomingDeadlineItems[0] ?? null);
  const recommendation = $derived(
    recommendWorkflowRoute({
      workspacePath: activeWorkspace?.path ?? null,
      jobs,
      selectedJob,
    }),
  );
  const workNavigation = $derived([
    {
      id: "opportunities" as const,
      label: copy.opportunities,
      icon: Search,
      enabled: true,
    },
    {
      id: "applications" as const,
      label: copy.applicationWorkspace,
      icon: BriefcaseBusiness,
      enabled: true,
    },
    {
      id: "profile" as const,
      label: copy.profile,
      icon: UserRound,
      enabled: true,
    },
    {
      id: "agent" as const,
      label: copy.agent,
      icon: Bot,
      enabled: true,
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
    ...workNavigation,
    ...utilityNavigation,
  ]);
  const currentApplicationSection = $derived(
    applicationSectionForRoute({
      view: activeView,
      detail: activeDetail ?? undefined,
    }),
  );
  const currentViewLabel = $derived(
    currentApplicationSection &&
      (activeView === "applications" || activeView === "workflow" || activeView === "delivery")
      ? `${copy.applicationWorkspace} · ${
          copy.applicationWorkspaceSectionLabel[currentApplicationSection]
        }`
      : (navigation.find((item) => item.id === activeView)?.label ?? copy.today),
  );

  function isWorkNavigationActive(id: NavigationId): boolean {
    if (id === "applications") {
      return (
        activeView === "applications" || activeView === "workflow" || activeView === "delivery"
      );
    }
    return activeView === id;
  }

  function isRecommendedNavigation(id: NavigationId): boolean {
    if (id === "applications") {
      return (
        recommendation.route.view === "applications" ||
        recommendation.route.view === "workflow" ||
        recommendation.route.view === "delivery"
      );
    }
    return recommendation.route.view === id;
  }

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
    void tick().then(() => {
      if (activeView !== view || activeDetail !== detail) return;
      if (detail) {
        document.getElementById(detail)?.scrollIntoView({ block: "start" });
        return;
      }
      window.scrollTo({ top: 0, left: 0, behavior: "auto" });
    });
  });

  $effect(() => {
    if (activeView !== "today" || TodayView || todayViewLoading || todayViewFailed) {
      return;
    }
    todayViewLoading = true;
    void import("$lib/views/TodayView.svelte")
      .then((module) => {
        TodayView = module.default;
      })
      .catch((error: unknown) => {
        todayViewFailed = true;
        captureBridgeError(error);
        bridgeErrorCanRetry = true;
      })
      .finally(() => {
        todayViewLoading = false;
      });
  });

  $effect(() => {
    if (
      activeView !== "workspaces" ||
      WorkspacesView ||
      workspacesViewLoading ||
      workspacesViewFailed
    ) {
      return;
    }
    workspacesViewLoading = true;
    void import("$lib/views/WorkspacesView.svelte")
      .then((module) => {
        WorkspacesView = module.default;
      })
      .catch((error: unknown) => {
        workspacesViewFailed = true;
        captureBridgeError(error);
        bridgeErrorCanRetry = true;
      })
      .finally(() => {
        workspacesViewLoading = false;
      });
  });

  $effect(() => {
    if (
      activeView !== "opportunities" ||
      OpportunitiesView ||
      opportunitiesViewLoading ||
      opportunitiesViewFailed
    ) {
      return;
    }
    opportunitiesViewLoading = true;
    void import("$lib/views/OpportunitiesView.svelte")
      .then((module) => {
        OpportunitiesView = module.default;
      })
      .catch((error: unknown) => {
        opportunitiesViewFailed = true;
        captureBridgeError(error);
        bridgeErrorCanRetry = true;
      })
      .finally(() => {
        opportunitiesViewLoading = false;
      });
  });

  $effect(() => {
    if (
      activeView !== "applications" ||
      GenericApplicationsView ||
      applicationsViewLoading ||
      applicationsViewFailed
    ) {
      return;
    }
    applicationsViewLoading = true;
    void import("$lib/views/GenericApplicationsView.svelte")
      .then((module) => {
        GenericApplicationsView = module.default;
      })
      .catch((error: unknown) => {
        applicationsViewFailed = true;
        captureBridgeError(error);
        bridgeErrorCanRetry = true;
      })
      .finally(() => {
        applicationsViewLoading = false;
      });
  });

  $effect(() => {
    if (activeView !== "profile" || ProfileView || profileViewLoading || profileViewFailed) {
      return;
    }
    profileViewLoading = true;
    void import("$lib/views/ProfileView.svelte")
      .then((module) => {
        ProfileView = module.default;
      })
      .catch((error: unknown) => {
        profileViewFailed = true;
        captureBridgeError(error);
        bridgeErrorCanRetry = true;
      })
      .finally(() => {
        profileViewLoading = false;
      });
  });

  $effect(() => {
    if (activeView !== "settings" || SettingsView || settingsViewLoading || settingsViewFailed) {
      return;
    }
    settingsViewLoading = true;
    void import("$lib/views/SettingsView.svelte")
      .then((module) => {
        SettingsView = module.default;
      })
      .catch((error: unknown) => {
        settingsViewFailed = true;
        captureBridgeError(error);
        bridgeErrorCanRetry = true;
      })
      .finally(() => {
        settingsViewLoading = false;
      });
  });

  $effect(() => {
    if (activeView !== "agent" || AgentView || agentViewLoading || agentViewFailed) {
      return;
    }
    agentViewLoading = true;
    void import("$lib/views/AgentView.svelte")
      .then((module) => {
        AgentView = module.default;
      })
      .catch((error: unknown) => {
        agentViewFailed = true;
        captureBridgeError(error);
        bridgeErrorCanRetry = true;
      })
      .finally(() => {
        agentViewLoading = false;
      });
  });

  $effect(() => {
    if (activeView !== "workflow" || WorkflowView || workflowViewLoading || workflowViewFailed) {
      return;
    }
    workflowViewLoading = true;
    void import("$lib/views/WorkflowView.svelte")
      .then((module) => {
        WorkflowView = module.default;
      })
      .catch((error: unknown) => {
        workflowViewFailed = true;
        captureBridgeError(error);
        bridgeErrorCanRetry = true;
      })
      .finally(() => {
        workflowViewLoading = false;
      });
  });

  $effect(() => {
    if (activeView !== "delivery" || DeliveryView || deliveryViewLoading || deliveryViewFailed) {
      return;
    }
    deliveryViewLoading = true;
    void import("$lib/views/DeliveryView.svelte")
      .then((module) => {
        DeliveryView = module.default;
      })
      .catch((error: unknown) => {
        deliveryViewFailed = true;
        captureBridgeError(error);
        bridgeErrorCanRetry = true;
      })
      .finally(() => {
        deliveryViewLoading = false;
      });
  });

  onMount(async () => {
    try {
      navigationMemory = parseNavigationMemory(localStorage.getItem(navigationPreferenceKey));
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
          supportedTextScales.includes(candidate.textScale as (typeof supportedTextScales)[number])
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
      await refreshWorkflowPackPresentation(language);
      product = await getProductSummary();
      await refreshWorkspaces(true);
    } catch (error) {
      captureBridgeError(error);
      workspaceLoading = false;
    }
  });

  async function refreshWorkflowPackPresentation(
    requestedLanguage: Language,
    packId: BuiltInWorkflowPackId = activePackId,
  ): Promise<void> {
    if (!desktopRuntime) return;
    const request = ++workflowPackPresentationRequest;
    try {
      const receipt = await getWorkflowPackPresentation(requestedLanguage, packId);
      if (request === workflowPackPresentationRequest) {
        workflowPackPresentation = receipt.data;
      }
    } catch (error) {
      if (request === workflowPackPresentationRequest) {
        workflowPackPresentation = null;
        captureBridgeError(error);
      }
    }
  }

  function selectApplicationPack(packId: BuiltInWorkflowPackId): void {
    if (packId === activePackId) return;
    activePackId = packId;
    void refreshWorkflowPackPresentation(language, packId);
  }

  function handleV4ApplicationContext(context: {
    workspacePath: string;
    packId: BuiltInWorkflowPackId;
    applications: StoredApplicationModelV3[];
    selected: StoredApplicationModelV3 | null;
    stages: ApplicationFlowStageV3[];
  }): void {
    if (context.workspacePath !== activeWorkspace?.path || context.packId !== activePackId) return;
    v4Applications = context.applications;
    selectedV4Application = context.selected;
    v4Stages = context.stages;
    requestedV4ApplicationId = context.selected?.snapshot.application.id ?? "";
  }

  function handleSelectV4Application(applicationId: string): void {
    requestedV4ApplicationId = applicationId;
  }

  function resetV4ApplicationContext(): void {
    v4Applications = [];
    selectedV4Application = null;
    v4Stages = [];
    requestedV4ApplicationId = "";
  }

  function handleLanguageChange(value: Language): void {
    language = value;
    void refreshWorkflowPackPresentation(value);
  }

  function handleAppearanceShortcut(event: KeyboardEvent): void {
    if (!event.metaKey || event.altKey || event.ctrlKey) return;
    if (event.key === "0") {
      event.preventDefault();
      textScale = 100;
      return;
    }
    if (event.key !== "+" && event.key !== "=" && event.key !== "-") return;
    event.preventDefault();
    const current = supportedTextScales.indexOf(textScale as (typeof supportedTextScales)[number]);
    const next =
      event.key === "-"
        ? Math.max(0, current - 1)
        : Math.min(supportedTextScales.length - 1, current + 1);
    textScale = supportedTextScales[next] ?? 100;
  }

  function captureBridgeError(error: unknown): void {
    notice = null;
    noticeRoute = null;
    bridgeError = commandErrorMessage(error);
    bridgeErrorCanRetry = commandErrorRetryable(error);
  }

  function dismissNotification(): void {
    bridgeError = null;
    bridgeErrorCanRetry = false;
    notice = null;
    noticeRoute = null;
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

  function recordSuccessfulAction(context: SuccessfulActionContext, result: unknown): void {
    const jobId = context.jobId === undefined ? selectedJobId || null : context.jobId;
    const route = {
      ...(context.route ?? { view: activeView, detail: activeDetail ?? undefined }),
      jobId: context.route?.jobId ?? jobId ?? undefined,
    };
    noticeRoute = route;
    lastSuccessfulAction = {
      operation: context.operation,
      summary: extractActionSummary(result, context.fallbackSummary ?? context.operation),
      route,
      workspacePath: activeWorkspace?.path ?? null,
      jobId,
      occurredAt: new Date().toISOString(),
    };
  }

  async function navigateTo(route: WorkflowRoute): Promise<void> {
    const destination = route;
    if (
      destination.view === "opportunities" ||
      destination.view === "profile" ||
      destination.view === "workflow" ||
      destination.view === "delivery"
    ) {
      selectApplicationPack(ACADEMIC_JOB_WORKFLOW_PACK_ID);
    }
    if (
      destination.jobId &&
      activeWorkspace &&
      destination.jobId !== selectedJob?.job.id &&
      jobs.some((job) => job.id === destination.jobId)
    ) {
      const selected = await handleSelectJob(destination.jobId);
      if (!selected) return;
    }
    activeDetail = destination.detail ?? null;
    activeView = destination.view;
  }

  async function runAction<T>(
    operation: () => Promise<T>,
    success?: SuccessfulActionContext,
  ): Promise<T | null> {
    busy = true;
    bridgeError = null;
    bridgeErrorCanRetry = false;
    notice = null;
    noticeRoute = null;
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
    const canonicalPath = session.registry.registry.default_path ?? session.action.data.path;
    activeWorkspace = { ...session.action.data, path: canonicalPath };
    resetV4ApplicationContext();
    navigationMemory = {
      ...navigationMemory,
      workspacePath: canonicalPath,
    };
    workspaceHealth = null;
    jobIntakePreview = null;
    contentCatalog = null;
    contentSearchResult = null;
    notice = session.action.summary;
    void refreshWorkflowPackPresentation(language, activePackId);
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
      const [dossierReceipt, catalogReceipt] = await Promise.all([
        listApplicationDossiers(activeWorkspace.path, false),
        getContentCatalog(activeWorkspace.path),
      ]);
      applicationDossiers = dossierReceipt.data.applications;
      jobs = applicationDossiers.map((dossier) => dossier.job);
      contentCatalog = catalogReceipt.data;
      contentSearchResult = null;
      const currentBelongsToWorkspace =
        selectedJob?.workspace === activeWorkspace.path &&
        jobs.some((job) => job.id === selectedJob?.job.id);
      const nextId = currentBelongsToWorkspace
        ? selectedJob?.job.id
        : rememberedJob(navigationMemory, activeWorkspace.path, jobs);
      selectedJob = applicationDossiers.find((dossier) => dossier.job.id === nextId) ?? null;
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
      const [dossierReceipt, catalogReceipt] = await Promise.all([
        getApplicationDossier(activeWorkspace.path, jobId),
        getContentCatalog(activeWorkspace.path),
      ]);
      selectedJob = dossierReceipt.data;
      applicationDossiers = applicationDossiers.map((dossier) =>
        dossier.job.id === jobId ? dossierReceipt.data : dossier,
      );
      contentCatalog = catalogReceipt.data;
      contentSearchResult = null;
      if (agentUiState.selectedJobId === jobId) {
        agentUiState.assistance = null;
        agentUiState.handoff = null;
      }
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
    await Promise.all([loadJobsForActive(), loadDiscoveryForActive(), loadProfileForActive()]);
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
        registrySnapshot.registry.entries.some((entry) => entry.path === rememberedPath)
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

  async function handleCreateWorkspace(
    alias: string,
    path: string,
    hosts: AgentHost[],
    skillsScope: AgentSkillsInstallScope,
  ): Promise<boolean> {
    const result = await runAction(() => createWorkspace(alias, path, hosts, skillsScope), {
      operation: "workspace.create",
      route: { view: "applications" },
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
      resetV4ApplicationContext();
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
    const result = await runAction(() => backupWorkspace(activeWorkspace!.path, destination), {
      operation: "workspace.backup",
      route: { view: "workspaces" },
      jobId: null,
    });
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
    const result = await runAction(() => restoreWorkspace(alias, backup, destination), {
      operation: "workspace.restore",
      route: { view: "workspaces" },
      jobId: null,
    });
    if (!result) return false;
    const restoredPath = result.registry.registry.default_path ?? result.action.data.destination;
    const session = await selectWorkspace(restoredPath);
    applyWorkspaceSession(session);
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
    if (!activeWorkspace) return false;
    const result = await runAction(() =>
      previewDiscoveryFile({ workspace: activeWorkspace!.path, ...options }),
    );
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
    if (!activeWorkspace) return false;
    const result = await runAction(() =>
      previewDiscoveryNetwork({ workspace: activeWorkspace!.path, ...options }),
    );
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
          discoveryPreview!.kind,
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
    if (!activeWorkspace || !discoveryPreview) return false;
    const result = await runAction(() =>
      discardDiscoveryPreview(
        activeWorkspace!.path,
        discoveryPreview!.preview_token,
        discoveryPreview!.kind,
      ),
    );
    if (result === null) return false;
    discoveryPreview = null;
    return true;
  }

  async function handlePromoteDiscoveryLead(leadId: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() => promoteDiscoveryLead(activeWorkspace!.path, leadId));
    if (!result) return false;
    await loadWorkspaceCollections();
    await handleSelectJob(result.data.job.id);
    recordSuccessfulAction(
      {
        operation: "discovery.promote",
        route: {
          view: "applications",
          detail: "source-intake",
          jobId: result.data.job.id,
        },
        jobId: result.data.job.id,
      },
      result,
    );
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
      getProfileEvidenceTemplate(activeWorkspace!.path, jobId, confirmedPrivateRead),
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
      () => confirmProfileEvidence(activeWorkspace!.path, jobId, candidate, confirmedPrivateRead),
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

  async function handleLoadWorkflow(jobId: string): Promise<WorkflowControlReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => getWorkflowControls(activeWorkspace!.path, jobId));
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleStartWorkflow(jobId: string): Promise<WorkflowControlReadModel | null> {
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
      () => completeWorkflowStage(activeWorkspace!.path, jobId, stage, artifactId),
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
    const result = await runAction(() => previewWorkflowRerun(activeWorkspace!.path, jobId, stage));
    if (!result) return null;
    notice = result.preview.summary;
    return result;
  }

  async function handleCommitWorkflowRerun(
    previewToken: string,
  ): Promise<WorkflowControlReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => commitWorkflowRerun(activeWorkspace!.path, previewToken), {
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

  async function handleDiscardWorkflowPreview(previewToken: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() =>
      discardWorkflowPreview(activeWorkspace!.path, previewToken),
    );
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
        (await runAction(() => getProfileEvidenceTemplate(workspace, jobId, confirmedPrivateRead)))
          ?.data ?? null
      );
    }
    if (kind === "criteria") {
      return (
        (await runAction(() => getCriteriaTemplate(workspace, jobId, confirmedPrivateRead)))
          ?.data ?? null
      );
    }
    if (kind === "matches") {
      return (
        (await runAction(() => getCurrentMatches(workspace, jobId, confirmedPrivateRead)))?.data ??
        null
      );
    }
    return (
      (
        await runAction(() =>
          current
            ? getCurrentPlan(workspace, jobId, confirmedPrivateRead)
            : getPlanTemplate(workspace, jobId, confirmedPrivateRead),
        )
      )?.data ?? null
    );
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
        () => confirmProfileEvidence(workspace, jobId, candidate, confirmedPrivateRead),
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
        () => confirmCriteria(workspace, jobId, candidate, confirmedPrivateRead),
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
      () => confirmPlan(workspace, jobId, candidate, confirmedPrivateRead),
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

  async function handleLoadLatestTask(jobId: string): Promise<TaskStateData | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => getLatestTask(activeWorkspace!.path, jobId));
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
        const committed = await commitTaskCompletion(workspace, previewToken);
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

  async function handleCancelTask(taskId: string): Promise<TaskStateData | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => cancelTask(activeWorkspace!.path, taskId), {
      operation: "task.cancel",
      route: {
        view: "workflow",
        detail: "agent-task",
        jobId: selectedJobId || undefined,
      },
      jobId: selectedJobId || null,
    });
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

  async function handleLoadAgentContext(jobId?: string): Promise<AgentContextReadModel | null> {
    const result = await runAction(() => getAgentContext(activeWorkspace?.path, jobId));
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleLoadAgentAssistance(
    jobId: string,
  ): Promise<AgentAssistanceReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => getAgentAssistance(activeWorkspace!.path, jobId), {
      operation: "agent.assistance",
      route: {
        view: "agent",
        detail: "agent-handoff",
        jobId,
      },
      jobId,
    });
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
    const result = await runAction(() => prepareAgentHandoff(host, activeWorkspace!.path, jobId), {
      operation: "agent.handoff.prepare",
      route: {
        view: "agent",
        detail: "agent-handoff",
        jobId,
      },
      jobId: jobId ?? null,
    });
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleInstallAgentSkills(
    host: "codex" | "claude" | "generic",
  ): Promise<AgentSkillsInstallReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => installAgentSkills(host, activeWorkspace!.path));
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleLoadAgentSkills(
    host: "codex" | "claude" | "generic",
  ): Promise<AgentSkillsStatusReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => getAgentSkillsStatus(host, activeWorkspace!.path));
    if (!result) return null;
    return result.data;
  }

  async function handleUninstallAgentSkills(
    host: "codex" | "claude" | "generic",
  ): Promise<AgentSkillsUninstallReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => uninstallAgentSkills(host, activeWorkspace!.path));
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
    const result = await runAction(() => prepareAgentMcpConfiguration(host, activeWorkspace!.path));
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

  async function handleLoadAgentRuntimes(jobId?: string): Promise<AgentRuntimeCatalog | null> {
    const result = await runAction(() => getAgentRuntimeCatalog(activeWorkspace?.path, jobId));
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
      notice = result.resumed ? copy.agentSessionResumed : copy.agentSessionStarted;
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
      notice = result.cancellation_requested ? copy.agentTurnCancelled : copy.noActiveAgentTurn;
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
      getDocumentWorkspace(activeWorkspace!.path, jobId, confirmedPrivateRead),
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
      getReviewWorkspace(activeWorkspace!.path, jobId, confirmedPrivateRead),
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
        const confirmed = await confirmReview(workspace, jobId, candidate, confirmedPrivateRead);
        const refreshed = await getReviewWorkspace(workspace, jobId, confirmedPrivateRead);
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

  async function handleCheckPackage(jobId: string): Promise<PackageManifestRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => checkPackage(activeWorkspace!.path, jobId));
    if (!result) return null;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.summary;
    return result.data;
  }

  async function handleLoadPackage(jobId: string): Promise<PackageManifestRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => getCurrentPackage(activeWorkspace!.path, jobId));
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
      () => exportPackage(activeWorkspace!.path, jobId, destination, confirmedPrivateExport),
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
    const result = await runAction(() => getCurrentPackageExport(activeWorkspace!.path, jobId));
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleReconcilePackage(
    jobId: string,
  ): Promise<ProjectionReconcileRecord[] | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => reconcilePackage(activeWorkspace!.path, jobId));
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
      copyPackageProjection(activeWorkspace!.path, jobId, path, destination),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleBuildRender(jobId: string): Promise<RenderManifestRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => buildRender(activeWorkspace!.path, jobId), {
      operation: "render.build",
      route: { view: "delivery", detail: "delivery-render", jobId },
      jobId,
    });
    if (!result) return null;
    await refreshSelectedJobSnapshot(jobId);
    notice = result.summary;
    return result.data;
  }

  async function handleLoadRender(jobId: string): Promise<RenderManifestRecord | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() => getCurrentRender(activeWorkspace!.path, jobId));
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handlePreviewRender(
    jobId: string,
    kind: DocumentKind,
    confirmedPrivateRead: boolean,
  ): Promise<Uint8Array | null> {
    if (!activeWorkspace) return null;
    return runAction(() => previewRender(activeWorkspace!.path, jobId, kind, confirmedPrivateRead));
  }

  async function handleExportRender(
    jobId: string,
    destination: string,
    confirmedPrivateExport: boolean,
  ): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(
      () => exportRender(activeWorkspace!.path, jobId, destination, confirmedPrivateExport),
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

  async function handleOpenRender(
    jobId: string,
    destination: string,
    kind: DocumentKind,
    confirmedPrivateExport: boolean,
  ): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(
      () =>
        exportRenderAndOpen(
          activeWorkspace!.path,
          jobId,
          destination,
          kind,
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

  async function handleCheckCli(destination?: string): Promise<CliInstallStatus | null> {
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
    const result = await runAction(() => checkForUpdates(confirmedNetworkFetch));
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

  async function handleOpenContent(entry: ContentCatalogEntryReadModel): Promise<void> {
    await navigateTo(routeForContentEntry(entry, selectedJobId || undefined));
  }

  async function handleRefreshJobs(): Promise<boolean> {
    bridgeError = null;
    await loadJobsForActive();
    return bridgeError === null;
  }

  async function handleCreateJob(title: string, institution: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() => createJob(activeWorkspace!.path, title, institution));
    if (!result) return false;
    await loadJobsForActive();
    await handleSelectJob(result.data.id);
    recordSuccessfulAction(
      {
        operation: "job.create",
        route: {
          view: "applications",
          detail: "source-intake",
          jobId: result.data.id,
        },
        jobId: result.data.id,
      },
      result,
    );
    notice = result.summary;
    return true;
  }

  async function handleSelectJob(jobId: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    if (jobIntakePreview && jobIntakePreview.preview.data.job.id !== jobId) {
      await discardJobSourcePreview(activeWorkspace.path, jobIntakePreview.preview_token).catch(
        () => undefined,
      );
      jobIntakePreview = null;
    }
    const result = await runAction(() => getApplicationDossier(activeWorkspace!.path, jobId));
    if (!result) return false;
    selectedJob = result.data;
    if (contentSearchResult?.filter.job_id && contentSearchResult.filter.job_id !== jobId) {
      contentSearchResult = null;
    }
    applicationDossiers = applicationDossiers.map((dossier) =>
      dossier.job.id === jobId ? result.data : dossier,
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
    const result = await runAction(() => archiveJob(activeWorkspace!.path, jobId), {
      operation: "job.archive",
      route: { view: "applications" },
      jobId: null,
    });
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

  async function handlePreviewLocalSource(source: string, confirmed: boolean): Promise<boolean> {
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

  async function handlePreviewUrlSource(url: string, confirmed: boolean): Promise<boolean> {
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
    const result = await runAction(() =>
      commitJobSourcePreview(activeWorkspace!.path, jobIntakePreview!.preview_token),
    );
    if (!result) return false;
    jobIntakePreview = null;
    await loadJobsForActive();
    await handleSelectJob(jobId);
    recordSuccessfulAction(
      {
        operation: "job.source.import",
        route: { view: "profile", detail: "profile-sources", jobId },
        jobId,
      },
      result,
    );
    notice = result.summary;
    return true;
  }

  async function handleDiscardJobSourcePreview(): Promise<boolean> {
    if (!activeWorkspace || !jobIntakePreview) return false;
    const result = await runAction(() =>
      discardJobSourcePreview(activeWorkspace!.path, jobIntakePreview!.preview_token),
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
    if (activeView === "today" && todayViewFailed) {
      todayViewFailed = false;
      await tick();
    } else if (activeView === "agent" && agentViewFailed) {
      agentViewFailed = false;
      await tick();
    } else if (activeView === "applications" && applicationsViewFailed) {
      applicationsViewFailed = false;
      await tick();
    } else if (activeView === "workflow" && workflowViewFailed) {
      workflowViewFailed = false;
      await tick();
    } else if (activeView === "delivery" && deliveryViewFailed) {
      deliveryViewFailed = false;
      await tick();
    } else if (activeView === "opportunities" && opportunitiesViewFailed) {
      opportunitiesViewFailed = false;
      await tick();
    } else if (activeView === "profile" && profileViewFailed) {
      profileViewFailed = false;
      await tick();
    } else if (activeView === "settings" && settingsViewFailed) {
      settingsViewFailed = false;
      await tick();
    } else if (activeView === "workspaces" && workspacesViewFailed) {
      workspacesViewFailed = false;
      await tick();
    } else if (activeView === "applications") {
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
  <title>{copy.appName} — {currentViewLabel}</title>
</svelte:head>

<Sidebar.DesktopProvider
  class="desktop-shell min-h-screen bg-background text-foreground"
  style="--sidebar-width: min(18rem, 32vw);"
  data-density={compact ? "compact" : "comfortable"}
>
  <a
    href="#main-content"
    class="fixed left-3 top-3 z-50 min-h-9 -translate-y-20 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-transform focus:translate-y-0 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
  >
    {copy.skipToContent}
  </a>
  <Sidebar.DesktopRoot
    role="complementary"
    class="sticky top-0 h-svh shrink-0 overflow-hidden border-r border-sidebar-border px-2.5 py-[var(--sidebar-padding-block)] transition-[padding] duration-200 ease-out motion-reduce:transition-none"
    aria-label={copy.appName}
  >
    <Sidebar.Header class="p-0">
      <div
        class="flex min-h-[var(--sidebar-header-height)] items-center gap-2 px-2 transition-[min-height] duration-200 ease-out motion-reduce:transition-none"
      >
        <div
          class="grid size-9 place-items-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground"
          aria-hidden="true"
        >
          <BriefcaseBusiness size={18} strokeWidth={1.8} />
        </div>
        <p class="min-w-0 truncate text-sm font-semibold tracking-tight">{copy.appName}</p>
      </div>
      <Separator class="my-1.5 bg-sidebar-border" />
    </Sidebar.Header>

    <Sidebar.Content class="pr-1">
      <WorkspaceContextBar
        {copy}
        snapshot={registrySnapshot}
        {activeWorkspace}
        {jobs}
        {selectedJob}
        {v4Applications}
        {selectedV4Application}
        {v4Stages}
        {activeView}
        {activeDetail}
        {recommendation}
        lastAction={lastSuccessfulAction?.workspacePath === (activeWorkspace?.path ?? null)
          ? lastSuccessfulAction
          : null}
        {busy}
        onSelectWorkspace={handleSelectWorkspace}
        onSelectJob={handleSelectJob}
        onSelectV4Application={handleSelectV4Application}
        onNavigate={navigateTo}
      />
      <Separator class="mb-2 bg-sidebar-border" />
      <nav class="min-w-0 space-y-1" aria-label={copy.primaryNavigation}>
        <Sidebar.Menu>
          <Sidebar.MenuItem>
            <Sidebar.DesktopMenuButton
              size="lg"
              isActive={activeView === "today"}
              aria-current={activeView === "today" ? "page" : undefined}
              onclick={() => void navigateTo({ view: "today" })}
            >
              <LayoutDashboard size={18} strokeWidth={1.8} aria-hidden="true" />
              <span title={copy.today}>{copy.today}</span>
            </Sidebar.DesktopMenuButton>
          </Sidebar.MenuItem>
        </Sidebar.Menu>

        <p
          class="px-2 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground"
        >
          {copy.work}
        </p>
        <Sidebar.Menu>
          {#each workNavigation as item}
            {@const Icon = item.icon}
            <Sidebar.MenuItem>
              <Sidebar.DesktopMenuButton
                size="lg"
                isActive={isWorkNavigationActive(item.id)}
                class={isRecommendedNavigation(item.id) && !isWorkNavigationActive(item.id)
                  ? "ring-1 ring-primary/25"
                  : ""}
                aria-current={isWorkNavigationActive(item.id) ? "page" : undefined}
                disabled={!item.enabled}
                onclick={() => {
                  if (item.enabled) void navigateTo({ view: item.id });
                }}
              >
                <Icon size={17} strokeWidth={1.8} aria-hidden="true" />
                <span class="min-w-0" title={item.label}>{item.label}</span>
                {#if isRecommendedNavigation(item.id)}
                  <span
                    class="ml-auto size-2 rounded-full bg-primary"
                    title={copy.nextRecommended}
                    aria-label={copy.nextRecommended}
                  ></span>
                {/if}
              </Sidebar.DesktopMenuButton>
            </Sidebar.MenuItem>
          {/each}
        </Sidebar.Menu>

        <p
          class="px-2 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground"
        >
          {copy.system}
        </p>
        <Sidebar.Menu>
          {#each utilityNavigation as item}
            {@const Icon = item.icon}
            <Sidebar.MenuItem>
              <Sidebar.DesktopMenuButton
                size="lg"
                isActive={activeView === item.id}
                aria-current={activeView === item.id ? "page" : undefined}
                onclick={() => void navigateTo({ view: item.id })}
              >
                <Icon size={18} strokeWidth={1.8} aria-hidden="true" />
                <span class="min-w-0" title={item.label}>{item.label}</span>
              </Sidebar.DesktopMenuButton>
            </Sidebar.MenuItem>
          {/each}
        </Sidebar.Menu>
      </nav>
    </Sidebar.Content>

    <Sidebar.Footer class="mt-auto border-t border-sidebar-border p-0 pt-2">
      <div class="grid grid-cols-3 gap-1" role="group" aria-label={copy.appearance}>
        <Button
          variant="ghost"
          size="icon-desktop"
          class="w-full"
          aria-label={language === "en" ? copy.switchChinese : copy.switchEnglish}
          title={language === "en" ? copy.switchChinese : copy.switchEnglish}
          onclick={() => handleLanguageChange(language === "en" ? "zh-CN" : "en")}
        >
          <Languages size={16} strokeWidth={1.8} aria-hidden="true" />
        </Button>
        <Button
          variant="ghost"
          size="icon-desktop"
          class="w-full"
          aria-label={darkMode ? copy.lightMode : copy.darkMode}
          title={darkMode ? copy.lightMode : copy.darkMode}
          onclick={() => (darkMode = !darkMode)}
        >
          {#if darkMode}
            <Sun size={16} strokeWidth={1.8} aria-hidden="true" />
          {:else}
            <Moon size={16} strokeWidth={1.8} aria-hidden="true" />
          {/if}
        </Button>
        <Button
          variant="ghost"
          size="icon-desktop"
          class="w-full"
          aria-label={compact ? copy.comfortable : copy.compact}
          title={compact ? copy.comfortable : copy.compact}
          onclick={() => (compact = !compact)}
        >
          <Settings2 size={16} strokeWidth={1.8} aria-hidden="true" />
        </Button>
      </div>
    </Sidebar.Footer>
  </Sidebar.DesktopRoot>

  <Sidebar.Inset
    id="main-content"
    class="min-h-screen bg-background"
    aria-label={copy.mainContent}
    data-testid="canisend-svelte-shell"
  >
    <div
      class="mx-auto w-full min-w-0 max-w-[1480px] px-4 py-[var(--page-padding-block)] transition-[padding] duration-200 ease-out motion-reduce:transition-none sm:px-5 lg:px-6"
    >
      {#if activeView === "workspaces"}
        {#if WorkspacesView}
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
        {:else if workspacesViewFailed}
          <Alert.Root variant="destructive" class="min-h-12">
            <Alert.Description>{copy.viewLoadFailed}</Alert.Description>
          </Alert.Root>
        {:else}
          <LoadingPanel label={copy.loading} class="min-h-32" />
        {/if}
      {:else if activeView === "opportunities"}
        {#if OpportunitiesView}
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
        {:else if opportunitiesViewFailed}
          <Alert.Root variant="destructive" class="min-h-12">
            <Alert.Description>{copy.viewLoadFailed}</Alert.Description>
          </Alert.Root>
        {:else}
          <LoadingPanel label={copy.loading} class="min-h-32" />
        {/if}
      {:else if activeView === "applications"}
        <div
          class="mb-[var(--density-section-gap)] flex flex-wrap items-center gap-2 rounded-lg border bg-card p-2"
          role="group"
          aria-label={copy.workflowPack}
        >
          <Button
            type="button"
            variant={activePackId === GENERIC_APPLICATION_WORKFLOW_PACK_ID ? "secondary" : "ghost"}
            aria-pressed={activePackId === GENERIC_APPLICATION_WORKFLOW_PACK_ID}
            onclick={() => selectApplicationPack(GENERIC_APPLICATION_WORKFLOW_PACK_ID)}
          >
            {copy.genericApplicationPack}
          </Button>
          <Button
            type="button"
            variant={activePackId === ACADEMIC_JOB_WORKFLOW_PACK_ID ? "secondary" : "ghost"}
            aria-pressed={activePackId === ACADEMIC_JOB_WORKFLOW_PACK_ID}
            onclick={() => selectApplicationPack(ACADEMIC_JOB_WORKFLOW_PACK_ID)}
          >
            {copy.academicJobPack}
          </Button>
          <span class="text-xs text-muted-foreground">{copy.workflowPackDescription}</span>
        </div>
        {#if !activeWorkspace}
          <Page.Root>
            <Page.Header
              eyebrow={copy.applications}
              title={copy.applicationsTitle}
              description={copy.genericApplicationsDescription}
            />
            <Alert.Root>
              <Alert.Description>{copy.noWorkspace}</Alert.Description>
            </Alert.Root>
          </Page.Root>
        {:else if GenericApplicationsView}
          <GenericApplicationsView
            {copy}
            {desktopRuntime}
            {activeWorkspace}
            packId={activePackId}
            presentation={workflowPackPresentation}
            requestedApplicationId={requestedV4ApplicationId}
            onContextChange={handleV4ApplicationContext}
          />
        {:else if applicationsViewFailed}
          <Alert.Root variant="destructive" class="min-h-12">
            <Alert.Description>{copy.viewLoadFailed}</Alert.Description>
          </Alert.Root>
        {:else}
          <LoadingPanel label={copy.loading} class="min-h-32" />
        {/if}
      {:else if activeView === "workflow"}
        {#if WorkflowView}
          <WorkflowView
            {copy}
            {desktopRuntime}
            {activeWorkspace}
            {selectedJobId}
            presentation={workflowPackPresentation}
            focus={activeDetail}
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
        {:else if workflowViewFailed}
          <Alert.Root variant="destructive" class="min-h-12">
            <Alert.Description>{copy.viewLoadFailed}</Alert.Description>
          </Alert.Root>
        {:else}
          <LoadingPanel label={copy.loading} class="min-h-32" />
        {/if}
      {:else if activeView === "delivery"}
        {#if DeliveryView}
          <DeliveryView
            {copy}
            {desktopRuntime}
            {activeWorkspace}
            {selectedJobId}
            presentation={workflowPackPresentation}
            focus={activeDetail}
            {busy}
            onNavigate={navigateTo}
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
            onPreviewRender={handlePreviewRender}
            onExportRender={handleExportRender}
            onOpenRender={handleOpenRender}
          />
        {:else if deliveryViewFailed}
          <Alert.Root variant="destructive" class="min-h-12">
            <Alert.Description>{copy.viewLoadFailed}</Alert.Description>
          </Alert.Root>
        {:else}
          <LoadingPanel label={copy.loading} class="min-h-32" />
        {/if}
      {:else if activeView === "profile"}
        {#if ProfileView}
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
        {:else if profileViewFailed}
          <Alert.Root variant="destructive" class="min-h-12">
            <Alert.Description>{copy.viewLoadFailed}</Alert.Description>
          </Alert.Root>
        {:else}
          <LoadingPanel label={copy.loading} class="min-h-32" />
        {/if}
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
            onLoadAssistance={handleLoadAgentAssistance}
            onPrepareHandoff={handlePrepareAgentHandoff}
            onLoadSkills={handleLoadAgentSkills}
            onInstallSkills={handleInstallAgentSkills}
            onUninstallSkills={handleUninstallAgentSkills}
            onCopyHandoff={handleCopyAgentHandoff}
            onPrepareMcpConfiguration={handlePrepareAgentMcpConfiguration}
            onCopyMcpConfiguration={handleCopyAgentMcpConfiguration}
            onLoadRuntimes={handleLoadAgentRuntimes}
            onRunTurn={handleRunAgentTurn}
            onCancelTurn={handleCancelAgentTurn}
            onExport={handleExportAgentPack}
          />
        {:else if agentViewFailed}
          <Alert.Root variant="destructive" class="min-h-12">
            <Alert.Description>{copy.viewLoadFailed}</Alert.Description>
          </Alert.Root>
        {:else}
          <LoadingPanel label={copy.loading} class="min-h-32" />
        {/if}
      {:else if activeView === "settings"}
        {#if SettingsView}
          <SettingsView
            {copy}
            {desktopRuntime}
            {busy}
            productVersion={product?.version ?? null}
            targetOs={product?.target_os ?? null}
            {language}
            {darkMode}
            {compact}
            {reducedMotion}
            {textScale}
            onLanguageChange={handleLanguageChange}
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
        {:else if settingsViewFailed}
          <Alert.Root variant="destructive" class="min-h-12">
            <Alert.Description>{copy.viewLoadFailed}</Alert.Description>
          </Alert.Root>
        {:else}
          <LoadingPanel label={copy.loading} class="min-h-32" />
        {/if}
      {:else}
        {#if TodayView}
          <TodayView
            {copy}
            {desktopRuntime}
            {activeWorkspace}
            jobCount={jobs.length}
            upcomingDeadlineCount={upcomingDeadlineItems.length}
            {nearestDeadlineItem}
            {workspaceHealth}
            {selectedDossier}
            {recommendation}
            {product}
            {doctor}
            {doctorRunning}
            onNavigate={navigateTo}
            onDoctor={handleDoctor}
          />
        {:else if todayViewFailed}
          <Alert.Root variant="destructive" class="min-h-12">
            <Alert.Description>{copy.viewLoadFailed}</Alert.Description>
          </Alert.Root>
        {:else}
          <LoadingPanel label={copy.loading} class="min-h-32" />
        {/if}
      {/if}
    </div>
  </Sidebar.Inset>

  {#if bridgeError || notice}
    <div
      data-slot="app-notification-region"
      class="pointer-events-none fixed right-4 bottom-4 z-50 w-[min(24rem,calc(100vw-2rem))]"
    >
      <Alert.Root
        variant={bridgeError ? "destructive" : "success"}
        role={bridgeError ? "alert" : "status"}
        aria-atomic="true"
        class="pointer-events-auto gap-3 p-3 shadow-lg"
      >
        <div class="flex min-w-0 items-start gap-2">
          <Alert.Description class="min-w-0 flex-1 pt-1">
            {bridgeError ?? notice}
          </Alert.Description>
          <Button
            variant="ghost"
            size="icon-desktop"
            class="-mt-1 -mr-1 shrink-0"
            aria-label={copy.dismiss}
            title={copy.dismiss}
            onclick={dismissNotification}
          >
            <X size={16} strokeWidth={1.8} aria-hidden="true" />
          </Button>
        </div>
        {#if bridgeErrorCanRetry || noticeRoute}
          <div class="flex flex-wrap justify-end gap-2">
            {#if bridgeErrorCanRetry}
              <Button variant="outline" size="desktop" disabled={busy} onclick={retryCurrentView}>
                {copy.retry}
              </Button>
            {:else if noticeRoute}
              <Button
                variant="outline"
                size="desktop"
                onclick={() => {
                  const route = noticeRoute!;
                  dismissNotification();
                  void navigateTo(route);
                }}
              >
                {copy.openAffectedContent}
              </Button>
            {/if}
          </div>
        {/if}
      </Alert.Root>
    </div>
  {/if}
</Sidebar.DesktopProvider>
