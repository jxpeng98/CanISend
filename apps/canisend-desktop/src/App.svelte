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
  import { onMount } from "svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import {
    archiveJob,
    backupWorkspace,
    beginWorkflowStage,
    buildRender,
    cancelTask,
    checkForUpdates,
    checkPackage,
    checkWorkspace,
    commandErrorMessage,
    commandErrorRetryable,
    commitTaskCompletion,
    commitDiscoveryPreview,
    commitWorkflowRerun,
    completeWorkflowStage,
    confirmCriteria,
    confirmPlan,
    confirmProfileEvidence,
    confirmReview,
    connectWorkspace,
    copyPackageProjection,
    createJob,
    createWorkspace,
    discardDiscoveryPreview,
    discardWorkflowPreview,
    exportAgentPack,
    exportPackage,
    exportRender,
    exportResourceCatalog,
    exportTaskInputs,
    getAgentCapabilities,
    getAgentContext,
    getCliInstallStatus,
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
    installCli,
    importLocalJobSource,
    importUrlJobSource,
    isDesktopRuntime,
    listJobs,
    listProfileSources,
    listDiscoveryLeads,
    listDiscoverySources,
    listWorkspaces,
    previewDiscoveryFile,
    previewDiscoveryNetwork,
    previewTaskCompletion,
    previewWorkflowRerun,
    prepareTask,
    prepareTaskAgain,
    promoteDiscoveryLead,
    removeWorkspace,
    repairWorkspace,
    reconcilePackage,
    replacePackageProjection,
    restoreWorkspace,
    runDoctor,
    selectWorkspace,
    showDiscoveryLead,
    showJob,
    startWorkflow,
    suggestDiscoveryDuplicates,
    uninstallCli,
    type ActionReceipt,
    type AgentCapabilitiesReadModel,
    type AgentContextReadModel,
    type AgentPackExportReadModel,
    type CliInstallStatus,
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
  import { messages, type Language } from "$lib/i18n";
  import AgentView from "$lib/views/AgentView.svelte";
  import ApplicationsView from "$lib/views/ApplicationsView.svelte";
  import DeliveryView from "$lib/views/DeliveryView.svelte";
  import OpportunitiesView from "$lib/views/OpportunitiesView.svelte";
  import ProfileView from "$lib/views/ProfileView.svelte";
  import SettingsView from "$lib/views/SettingsView.svelte";
  import WorkflowView from "$lib/views/WorkflowView.svelte";
  import WorkspacesView from "$lib/views/WorkspacesView.svelte";

  type NavigationId =
    | "today"
    | "opportunities"
    | "applications"
    | "workflow"
    | "delivery"
    | "profile"
    | "agent"
    | "workspaces"
    | "settings";

  type DecisionKind = "evidence" | "criteria" | "matches" | "plan";
  type AppearancePreferences = {
    language: Language;
    darkMode: boolean;
    compact: boolean;
    reducedMotion: boolean;
    textScale: number;
  };

  let language = $state<Language>("en");
  let darkMode = $state(false);
  let compact = $state(false);
  let reducedMotion = $state(false);
  let textScale = $state(100);
  let preferencesReady = $state(false);
  let activeView = $state<NavigationId>("today");
  let product = $state<ProductSummary | null>(null);
  let doctor = $state<ActionReceipt<DoctorSummary> | null>(null);
  let bridgeError = $state<string | null>(null);
  let bridgeErrorCanRetry = $state(false);
  let notice = $state<string | null>(null);
  let doctorRunning = $state(false);
  let busy = $state(false);
  let workspaceLoading = $state(true);
  let jobsLoading = $state(false);
  let discoveryLoading = $state(false);
  let profileLoading = $state(false);
  let registrySnapshot = $state<RegistrySnapshot | null>(null);
  let activeWorkspace = $state<WorkspaceReadModel | null>(null);
  let workspaceHealth = $state<WorkspaceHealthReadModel | null>(null);
  let jobs = $state<JobRecord[]>([]);
  let selectedJob = $state<JobDetailReadModel | null>(null);
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
  const supportedTextScales = [100, 125, 150, 200] as const;

  const copy = $derived(messages[language]);
  const navigation = $derived([
    { id: "today" as const, label: copy.today, icon: LayoutDashboard, enabled: true, stage: "" },
    {
      id: "opportunities" as const,
      label: copy.opportunities,
      icon: Search,
      enabled: true,
      stage: "",
    },
    {
      id: "applications" as const,
      label: copy.applications,
      icon: BriefcaseBusiness,
      enabled: true,
      stage: "",
    },
    {
      id: "workflow" as const,
      label: copy.workflow,
      icon: GitBranch,
      enabled: true,
      stage: "",
    },
    {
      id: "delivery" as const,
      label: copy.delivery,
      icon: FileUp,
      enabled: true,
      stage: "",
    },
    { id: "profile" as const, label: copy.profile, icon: UserRound, enabled: true, stage: "" },
    { id: "agent" as const, label: copy.agent, icon: Bot, enabled: true, stage: "" },
    {
      id: "workspaces" as const,
      label: copy.workspaces,
      icon: Database,
      enabled: true,
      stage: "",
    },
    { id: "settings" as const, label: copy.settings, icon: Settings2, enabled: true, stage: "" },
  ]);

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
      localStorage.setItem(appearancePreferenceKey, JSON.stringify(preferences));
    }
  });

  onMount(async () => {
    try {
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

  async function runAction<T>(operation: () => Promise<T>): Promise<T | null> {
    busy = true;
    bridgeError = null;
    bridgeErrorCanRetry = false;
    notice = null;
    try {
      return await operation();
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
    workspaceHealth = null;
    notice = session.action.summary;
  }

  async function loadJobsForActive(): Promise<void> {
    if (!activeWorkspace) {
      jobs = [];
      selectedJob = null;
      return;
    }
    jobsLoading = true;
    try {
      const receipt = await listJobs(activeWorkspace.path, false);
      jobs = receipt.data.jobs;
      const nextId =
        selectedJob && jobs.some((job) => job.id === selectedJob?.job.id)
          ? selectedJob.job.id
          : jobs[0]?.id;
      selectedJob = nextId
        ? (await showJob(activeWorkspace.path, nextId)).data
        : null;
    } catch (error) {
      captureBridgeError(error);
    } finally {
      jobsLoading = false;
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
      if (defaultPath && (autoSelect || !activeWorkspace)) {
        await openWorkspace(defaultPath);
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
    const result = await runAction(() => createWorkspace(alias, path));
    if (!result) return false;
    applyWorkspaceSession(result);
    await loadWorkspaceCollections();
    return true;
  }

  async function handleConnectWorkspace(alias: string, path: string): Promise<boolean> {
    const result = await runAction(() => connectWorkspace(alias, path));
    if (!result) return false;
    applyWorkspaceSession(result);
    await loadWorkspaceCollections();
    return true;
  }

  async function handleRemoveWorkspace(path: string): Promise<boolean> {
    const result = await runAction(() => removeWorkspace(path));
    if (!result) return false;
    registrySnapshot = result;
    if (activeWorkspace?.path === path) {
      activeWorkspace = null;
      workspaceHealth = null;
      jobs = [];
      selectedJob = null;
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
    const result = await runAction(() =>
      backupWorkspace(activeWorkspace!.path, destination),
    );
    if (!result) return false;
    notice = result.summary;
    return true;
  }

  async function handleRepairWorkspace(): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() => repairWorkspace(activeWorkspace!.path));
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
    const result = await runAction(() =>
      restoreWorkspace(alias, backup, destination),
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
    const result = await runAction(() =>
      commitDiscoveryPreview(
        activeWorkspace!.path,
        discoveryPreview!.preview_token,
      ),
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
    const result = await runAction(() =>
      promoteDiscoveryLead(activeWorkspace!.path, leadId),
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
    const result = await runAction(() =>
      importProfileSource({
        workspace: activeWorkspace!.path,
        ...options,
      }),
    );
    if (!result) return false;
    await loadProfileForActive();
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
    notice = result.summary;
    return true;
  }

  async function handleConfirmProfileEvidence(
    jobId: string,
    candidate: unknown,
    confirmedPrivateRead: boolean,
  ): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() =>
      confirmProfileEvidence(
        activeWorkspace!.path,
        jobId,
        candidate,
        confirmedPrivateRead,
      ),
    );
    if (!result) return false;
    profileEvidence = result.data;
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
    const result = await runAction(async () => {
      const started = await startWorkflow(workspace, jobId);
      const controls = await getWorkflowControls(workspace, jobId);
      return { started, controls };
    });
    if (!result) return null;
    notice = result.started.summary;
    return result.controls.data;
  }

  async function handleBeginWorkflowStage(
    jobId: string,
    stage: WorkflowStage,
    mode: ExecutionMode,
  ): Promise<WorkflowControlReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      beginWorkflowStage(activeWorkspace!.path, jobId, stage, mode),
    );
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleCompleteWorkflowStage(
    jobId: string,
    stage: WorkflowStage,
    artifactId: string,
  ): Promise<WorkflowControlReadModel | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      completeWorkflowStage(
        activeWorkspace!.path,
        jobId,
        stage,
        artifactId,
      ),
    );
    if (!result) return null;
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
    const result = await runAction(() => commitWorkflowRerun(previewToken));
    if (!result) return null;
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
      const result = await runAction(() =>
        confirmProfileEvidence(
          workspace,
          jobId,
          candidate,
          confirmedPrivateRead,
        ),
      );
      if (!result) return null;
      profileEvidence = result.data;
      notice = result.summary;
      return result.data;
    }
    if (kind === "criteria") {
      const result = await runAction(() =>
        confirmCriteria(
          workspace,
          jobId,
          candidate,
          confirmedPrivateRead,
        ),
      );
      if (!result) return null;
      notice = result.summary;
      return result.data;
    }
    const result = await runAction(() =>
      confirmPlan(
        workspace,
        jobId,
        candidate,
        confirmedPrivateRead,
      ),
    );
    if (!result) return null;
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
    const result = await runAction(async () => {
      const prepared = await prepareTask(workspace, jobId, operation, mode);
      const latest = await getLatestTask(workspace, jobId);
      return { prepared, latest };
    });
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
    const result = await runAction(async () => {
      const committed = await commitTaskCompletion(previewToken);
      const latest = await getLatestTask(workspace, jobId);
      return { committed, latest };
    });
    if (!result) return null;
    notice = result.committed.summary;
    return result.latest.data;
  }

  async function handleCancelTask(
    taskId: string,
  ): Promise<TaskStateData | null> {
    if (!activeWorkspace) return null;
    const result = await runAction(() =>
      cancelTask(activeWorkspace!.path, taskId),
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
    const result = await runAction(async () => {
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
    });
    if (!result) return null;
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
    const result = await runAction(() =>
      exportPackage(
        activeWorkspace!.path,
        jobId,
        destination,
        confirmedPrivateExport,
      ),
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
    const result = await runAction(() =>
      buildRender(activeWorkspace!.path, jobId),
    );
    if (!result) return null;
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
    const result = await runAction(() =>
      exportRender(
        activeWorkspace!.path,
        jobId,
        destination,
        confirmedPrivateExport,
      ),
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
    const result = await runAction(() => installCli(options));
    if (!result) return null;
    notice = result.summary;
    return result.data;
  }

  async function handleUninstallCli(options: {
    destination?: string;
    confirmedTerminalInstall: boolean;
  }): Promise<CliInstallStatus | null> {
    const result = await runAction(() => uninstallCli(options));
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

  async function handleRefreshJobs(): Promise<boolean> {
    bridgeError = null;
    await loadJobsForActive();
    return bridgeError === null;
  }

  async function handleCreateJob(title: string, institution: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() =>
      createJob(activeWorkspace!.path, title, institution),
    );
    if (!result) return false;
    await loadJobsForActive();
    await handleSelectJob(result.data.id);
    notice = result.summary;
    return true;
  }

  async function handleSelectJob(jobId: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() => showJob(activeWorkspace!.path, jobId));
    if (!result) return false;
    selectedJob = result.data;
    return true;
  }

  async function handleArchiveJob(jobId: string): Promise<boolean> {
    if (!activeWorkspace) return false;
    const result = await runAction(() => archiveJob(activeWorkspace!.path, jobId));
    if (!result) return false;
    selectedJob = null;
    await loadJobsForActive();
    notice = result.summary;
    return true;
  }

  async function handleImportLocal(source: string, confirmed: boolean): Promise<boolean> {
    if (!activeWorkspace || !selectedJob) return false;
    const jobId = selectedJob.job.id;
    const result = await runAction(() =>
      importLocalJobSource(activeWorkspace!.path, jobId, source, confirmed),
    );
    if (!result) return false;
    await loadJobsForActive();
    await handleSelectJob(jobId);
    notice = result.summary;
    return true;
  }

  async function handleImportUrl(url: string, confirmed: boolean): Promise<boolean> {
    if (!activeWorkspace || !selectedJob) return false;
    const jobId = selectedJob.job.id;
    const result = await runAction(() =>
      importUrlJobSource(activeWorkspace!.path, jobId, url, confirmed),
    );
    if (!result) return false;
    await loadJobsForActive();
    await handleSelectJob(jobId);
    notice = result.summary;
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
          {#if item.stage}
            <span class="ml-auto text-[10px] font-normal text-muted-foreground">{item.stage}</span>
          {/if}
        </Button>
      {/each}
    </nav>

    <div class="mt-auto space-y-3">
      {#if activeWorkspace}
        <button
          type="button"
          class="w-full rounded-xl border border-sidebar-border bg-background/55 p-3 text-left transition-colors hover:bg-background"
          onclick={() => (activeView = "workspaces")}
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
        <span>{product?.version ?? "1.0.0-alpha.3"}</span>
        <Badge variant="outline" class="text-[10px]">Svelte</Badge>
      </div>
    </div>
  </aside>

  <main
    class="ml-64 min-h-screen"
    aria-label={copy.mainContent}
    data-testid="canisend-svelte-shell"
  >
    <header
      class="sticky top-0 z-10 flex min-h-16 items-center justify-between border-b bg-background/92 px-8 backdrop-blur"
      data-tauri-drag-region
    >
      <p class="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
        {navigation.find((item) => item.id === activeView)?.label}
      </p>
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
        <Button variant="outline" class="min-h-11" onclick={() => (compact = !compact)}>
          {compact ? copy.comfortable : copy.compact}
        </Button>
      </div>
    </header>

    <div class="mx-auto max-w-[1480px] px-8 py-8">
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
          loading={jobsLoading}
          {busy}
          onRefresh={handleRefreshJobs}
          onCreate={handleCreateJob}
          onSelect={handleSelectJob}
          onArchive={handleArchiveJob}
          onImportLocal={handleImportLocal}
          onImportUrl={handleImportUrl}
        />
      {:else if activeView === "workflow"}
        <WorkflowView
          {copy}
          {desktopRuntime}
          {activeWorkspace}
          {jobs}
          {busy}
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
          {jobs}
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
          {jobs}
          sources={profileSources}
          {profileRevision}
          evidence={profileEvidence}
          loading={profileLoading}
          {busy}
          onRefresh={handleRefreshProfile}
          onImport={handleImportProfileSource}
          onLoadEvidence={handleLoadProfileEvidence}
          onConfirmEvidence={handleConfirmProfileEvidence}
        />
      {:else if activeView === "agent"}
        <AgentView
          {copy}
          {desktopRuntime}
          {activeWorkspace}
          {jobs}
          {busy}
          onLoadCapabilities={handleLoadAgentCapabilities}
          onLoadContext={handleLoadAgentContext}
          onExport={handleExportAgentPack}
        />
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
              onclick={() => (activeView = "applications")}
            >
              <FileUp size={17} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
              {copy.importSource}
            </Button>
            <Button
              class="min-h-11"
              disabled={!activeWorkspace}
              onclick={() => (activeView = "applications")}
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
              <Card.Description>{copy.chooseWorkspaceDescription}</Card.Description>
            </Card.Header>
            <Card.Content>
              <div class="flex min-h-48 flex-col items-center justify-center rounded-xl border border-dashed bg-muted/25 px-8 text-center">
                <div class="grid size-11 place-items-center rounded-xl bg-accent text-accent-foreground">
                  <Database size={20} strokeWidth={1.8} aria-hidden="true" />
                </div>
                <h2 class="mt-4 text-base font-semibold">
                  {activeWorkspace ? copy.openApplications : copy.chooseWorkspace}
                </h2>
                <p class="mt-2 max-w-md text-sm leading-6 text-muted-foreground">
                  {activeWorkspace?.path ?? copy.chooseWorkspaceDescription}
                </p>
                <Button
                  class="mt-5 min-h-11"
                  variant="outline"
                  onclick={() =>
                    (activeView = activeWorkspace ? "applications" : "workspaces")}
                >
                  {activeWorkspace ? copy.openApplications : copy.manageWorkspaces}
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
