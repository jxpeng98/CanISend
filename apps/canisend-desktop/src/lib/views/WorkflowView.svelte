<script lang="ts">
  import {
    Bot,
    CheckCircle2,
    CircleDot,
    FileJson,
    FolderOpen,
    GitBranch,
    Play,
    RefreshCw,
    RotateCcw,
    ShieldCheck,
  } from "@lucide/svelte";

  import * as Page from "$lib/components/patterns/page/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import * as Accordion from "$lib/components/ui/accordion/index.js";
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
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";
  import {
    chooseExportDirectory,
    chooseTaskCompletion,
    type ExecutionMode,
    type TaskCompletionPreviewReadModel,
    type TaskExecutionMode,
    type TaskOperation,
    type TaskStateData,
    type WorkflowControlReadModel,
    type WorkflowPackPresentationReadModel,
    type WorkflowRerunPreviewReadModel,
    type WorkflowStage,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";
  import {
    buildJsonDiff,
    collectRevisionReferences,
    type JsonDiffSummary,
    type RevisionReferenceSummary,
  } from "$lib/proposal-review";
  import type {
    WorkflowDetail,
    WorkflowRoute,
  } from "$lib/workflow-navigation";
  import {
    packTaskOperationOptions,
    workflowStageLabel,
  } from "$lib/workflow-pack-presentation";

  type DecisionKind = "evidence" | "criteria" | "matches" | "plan";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    activeWorkspace: WorkspaceReadModel | null;
    selectedJobId: string;
    presentation: WorkflowPackPresentationReadModel | null;
    focus: WorkflowDetail | null;
    busy: boolean;
    onNavigate: (route: WorkflowRoute) => Promise<void>;
    onOpenTaskResult: (operation: TaskOperation | string) => Promise<void>;
    onLoadWorkflow: (jobId: string) => Promise<WorkflowControlReadModel | null>;
    onStartWorkflow: (jobId: string) => Promise<WorkflowControlReadModel | null>;
    onBeginStage: (
      jobId: string,
      stage: WorkflowStage,
      mode: ExecutionMode,
    ) => Promise<WorkflowControlReadModel | null>;
    onCompleteStage: (
      jobId: string,
      stage: WorkflowStage,
      artifactId: string,
    ) => Promise<WorkflowControlReadModel | null>;
    onPreviewRerun: (
      jobId: string,
      stage: WorkflowStage,
    ) => Promise<WorkflowRerunPreviewReadModel | null>;
    onCommitRerun: (
      previewToken: string,
    ) => Promise<WorkflowControlReadModel | null>;
    onDiscardPreview: (previewToken: string) => Promise<boolean>;
    onLoadDecision: (
      jobId: string,
      kind: DecisionKind,
      current: boolean,
      confirmedPrivateRead: boolean,
    ) => Promise<unknown | null>;
    onConfirmDecision: (
      jobId: string,
      kind: Exclude<DecisionKind, "matches">,
      candidate: unknown,
      confirmedPrivateRead: boolean,
    ) => Promise<unknown | null>;
    onLoadLatestTask: (jobId: string) => Promise<TaskStateData | null>;
    onPrepareTask: (
      jobId: string,
      operation: TaskOperation,
      mode: TaskExecutionMode,
    ) => Promise<TaskStateData | null>;
    onExportTaskInputs: (options: {
      taskId: string;
      destination: string;
      confirmedPrivateRead: boolean;
      confirmedProviderSend: boolean;
    }) => Promise<boolean>;
    onPreviewTaskCompletion: (options: {
      file: string;
      confirmedPrivateRead: boolean;
    }) => Promise<TaskCompletionPreviewReadModel | null>;
    onCommitTaskCompletion: (
      previewToken: string,
      jobId: string,
    ) => Promise<TaskStateData | null>;
    onCancelTask: (taskId: string) => Promise<TaskStateData | null>;
    onPrepareTaskAgain: (
      taskId: string,
      jobId: string,
    ) => Promise<TaskStateData | null>;
  };

  let {
    copy,
    desktopRuntime,
    activeWorkspace,
    selectedJobId,
    presentation,
    focus,
    busy,
    onNavigate,
    onOpenTaskResult,
    onLoadWorkflow,
    onStartWorkflow,
    onBeginStage,
    onCompleteStage,
    onPreviewRerun,
    onCommitRerun,
    onDiscardPreview,
    onLoadDecision,
    onConfirmDecision,
    onLoadLatestTask,
    onPrepareTask,
    onExportTaskInputs,
    onPreviewTaskCompletion,
    onCommitTaskCompletion,
    onCancelTask,
    onPrepareTaskAgain,
  }: Props = $props();

  let section = $state("workflow");
  let loadedJobId = $state("");
  let workflow = $state<WorkflowControlReadModel | null>(null);
  let selectedStage = $state<WorkflowStage>("parse");
  let executionMode = $state<ExecutionMode>("host-agent");
  let artifactId = $state("");
  let rerunPreview = $state<WorkflowRerunPreviewReadModel | null>(null);
  let rerunOpen = $state(false);
  let privateSessionConsent = $state(false);
  let decisionKind = $state<DecisionKind>("evidence");
  let decisionEditable = $state(false);
  let decisionJson = $state("");
  let decisionBaseline = $state<unknown>(null);
  let decisionPreview = $state<{
    candidate: unknown;
    diff: JsonDiffSummary;
    references: RevisionReferenceSummary[];
  } | null>(null);
  let task = $state<TaskStateData | null>(null);
  let taskOperation = $state<TaskOperation>("job-parse");
  let taskMode = $state<TaskExecutionMode>("host-agent");
  let taskExportDestination = $state("");
  let taskCompletionFile = $state("");
  let taskPrivateConsent = $state(false);
  let providerSendConsent = $state(false);
  let taskCompletionPreview = $state<TaskCompletionPreviewReadModel | null>(null);
  let formError = $state<string | null>(null);

  $effect(() => {
    const nextKey = `${activeWorkspace?.path ?? ""}:${selectedJobId}`;
    if (nextKey !== loadedJobId) {
      loadedJobId = nextKey;
      workflow = null;
      decisionJson = "";
      decisionBaseline = null;
      decisionPreview = null;
      decisionEditable = false;
      task = null;
      rerunPreview = null;
      taskCompletionPreview = null;
    }
  });

  $effect(() => {
    if (focus === "agent-task") {
      section = "task";
    } else if (focus?.startsWith("decision-")) {
      section = "decisions";
      if (focus === "decision-criteria") decisionKind = "criteria";
      if (focus === "decision-evidence") decisionKind = "evidence";
      if (focus === "decision-matches") decisionKind = "matches";
      if (focus === "decision-plan") decisionKind = "plan";
    } else if (focus === "workflow-stages") {
      section = "workflow";
    }
  });

  const selectedDescriptor = $derived(
    workflow?.stage_descriptors.find((descriptor) => descriptor.stage === selectedStage) ?? null,
  );
  const taskOperationOptions = $derived(packTaskOperationOptions(presentation));
  const selectedStageState = $derived(
    workflow?.status.stages.find((stage) => stage.stage === selectedStage) ?? null,
  );

  $effect(() => {
    if (
      selectedDescriptor &&
      !selectedDescriptor.execution_modes.includes(executionMode)
    ) {
      executionMode = selectedDescriptor.execution_modes[0] ?? "host-agent";
    }
  });

  async function refreshWorkflow(): Promise<void> {
    formError = null;
    if (!selectedJobId) return;
    workflow = await onLoadWorkflow(selectedJobId);
  }

  async function startWorkflow(): Promise<void> {
    formError = null;
    if (!selectedJobId) return;
    workflow = await onStartWorkflow(selectedJobId);
  }

  async function beginStage(): Promise<void> {
    formError = null;
    if (!selectedJobId) return;
    workflow = await onBeginStage(selectedJobId, selectedStage, executionMode);
  }

  async function completeStage(): Promise<void> {
    formError = null;
    if (!selectedJobId || !artifactId.trim()) {
      formError = copy.artifactId;
      return;
    }
    workflow = await onCompleteStage(
      selectedJobId,
      selectedStage,
      artifactId.trim(),
    );
    if (workflow) artifactId = "";
  }

  async function previewRerun(): Promise<void> {
    formError = null;
    if (!selectedJobId || selectedStage === "intake") return;
    rerunPreview = await onPreviewRerun(selectedJobId, selectedStage);
    rerunOpen = rerunPreview !== null;
  }

  async function commitRerun(): Promise<void> {
    if (!rerunPreview) return;
    const next = await onCommitRerun(rerunPreview.preview_token);
    if (next) {
      workflow = next;
      rerunPreview = null;
      rerunOpen = false;
    }
  }

  async function closeRerun(): Promise<void> {
    if (rerunPreview) {
      await onDiscardPreview(rerunPreview.preview_token);
    }
    rerunPreview = null;
    rerunOpen = false;
  }

  async function loadDecision(current = false): Promise<void> {
    formError = null;
    if (!selectedJobId || !privateSessionConsent) {
      formError = copy.privateWorkspaceConsent;
      return;
    }
    const data = await onLoadDecision(
      selectedJobId,
      decisionKind,
      current,
      privateSessionConsent,
    );
    if (data !== null) {
      decisionJson = JSON.stringify(data, null, 2);
      decisionBaseline = data;
      decisionPreview = null;
      decisionEditable = decisionKind !== "matches";
    }
  }

  function previewDecision(): void {
    formError = null;
    if (
      !selectedJobId ||
      !privateSessionConsent ||
      decisionKind === "matches" ||
      !decisionEditable
    ) {
      return;
    }
    try {
      const candidate: unknown = JSON.parse(decisionJson);
      decisionPreview = {
        candidate,
        diff: buildJsonDiff(decisionBaseline, candidate),
        references: collectRevisionReferences(candidate),
      };
      decisionEditable = false;
    } catch {
      formError = copy.invalidJson;
    }
  }

  function editDecision(): void {
    decisionPreview = null;
    decisionEditable = true;
  }

  async function confirmDecision(): Promise<void> {
    formError = null;
    if (
      !selectedJobId ||
      !privateSessionConsent ||
      decisionKind === "matches" ||
      !decisionPreview
    ) {
      return;
    }
    try {
      const data = await onConfirmDecision(
        selectedJobId,
        decisionKind,
        decisionPreview.candidate,
        privateSessionConsent,
      );
      if (data !== null) {
        decisionJson = JSON.stringify(data, null, 2);
        decisionBaseline = data;
        decisionPreview = null;
        decisionEditable = false;
      }
    } catch {
      formError = copy.invalidJson;
    }
  }

  async function refreshTask(): Promise<void> {
    formError = null;
    if (!selectedJobId) return;
    task = await onLoadLatestTask(selectedJobId);
  }

  async function prepareTask(): Promise<void> {
    formError = null;
    if (!selectedJobId) return;
    task = await onPrepareTask(selectedJobId, taskOperation, taskMode);
  }

  async function chooseTaskDestination(): Promise<void> {
    taskExportDestination =
      (await chooseExportDirectory()) ?? taskExportDestination;
  }

  async function chooseCompletion(): Promise<void> {
    taskCompletionFile =
      (await chooseTaskCompletion()) ?? taskCompletionFile;
  }

  async function exportInputs(): Promise<void> {
    formError = null;
    if (!task || !taskExportDestination || !taskPrivateConsent) {
      formError = copy.privateWorkspaceConsent;
      return;
    }
    await onExportTaskInputs({
      taskId: task.descriptor.id,
      destination: taskExportDestination,
      confirmedPrivateRead: taskPrivateConsent,
      confirmedProviderSend: providerSendConsent,
    });
  }

  async function previewCompletion(): Promise<void> {
    formError = null;
    if (!taskCompletionFile || !taskPrivateConsent) {
      formError = copy.privateWorkspaceConsent;
      return;
    }
    taskCompletionPreview = await onPreviewTaskCompletion({
      file: taskCompletionFile,
      confirmedPrivateRead: taskPrivateConsent,
    });
  }

  async function commitCompletion(): Promise<void> {
    if (!taskCompletionPreview) return;
    const next = await onCommitTaskCompletion(
      taskCompletionPreview.preview_token,
      selectedJobId,
    );
    if (next) {
      task = next;
      taskCompletionPreview = null;
      taskCompletionFile = "";
    }
  }

  async function cancelCurrentTask(): Promise<void> {
    if (!task) return;
    task = await onCancelTask(task.descriptor.id);
  }

  async function prepareCurrentTaskAgain(): Promise<void> {
    if (!task) return;
    task = await onPrepareTaskAgain(task.descriptor.id, selectedJobId);
  }

  function decisionLabel(kind: DecisionKind): string {
    if (kind === "evidence") return copy.evidence;
    if (kind === "criteria") return copy.criteria;
    if (kind === "matches") return copy.matches;
    return copy.plan;
  }

  function decisionDetail(kind: DecisionKind): WorkflowDetail {
    if (kind === "criteria") return "decision-criteria";
    if (kind === "matches") return "decision-matches";
    if (kind === "plan") return "decision-plan";
    return "decision-evidence";
  }

  function navigateWithinWorkflow(detail: WorkflowDetail): void {
    void onNavigate({
      view: "workflow",
      detail,
      jobId: selectedJobId || undefined,
    });
  }

  const workspaceTitle = $derived(
    section === "decisions" && decisionKind !== "criteria"
      ? copy.evidenceFitTitle
      : copy.jobCriteriaTitle,
  );
  const workspaceDescription = $derived(
    section === "decisions" && decisionKind !== "criteria"
      ? copy.evidenceFitDescription
      : copy.jobCriteriaDescription,
  );
</script>

<Page.Root>
  <Page.Header
    eyebrow={copy.applicationWorkspace}
    title={workspaceTitle}
    description={workspaceDescription}
  />

  {#if !activeWorkspace || !selectedJobId}
    <Card.Root>
      <Card.Content>
        <Empty.Root class="min-h-32">
          <Empty.Header>
            <Empty.Media variant="icon">
              <GitBranch size={24} strokeWidth={1.8} aria-hidden="true" />
            </Empty.Media>
            <Empty.Title class="text-base">
              {activeWorkspace ? copy.noApplications : copy.noWorkspace}
            </Empty.Title>
            <Empty.Description>
              {activeWorkspace ? copy.noApplicationsDescription : copy.chooseWorkspaceDescription}
            </Empty.Description>
          </Empty.Header>
        </Empty.Root>
      </Card.Content>
    </Card.Root>
  {:else}
    <Tabs.Root bind:value={section}>
      <Tabs.List class="responsive-tabs max-w-2xl" data-columns="3">
        <Tabs.Trigger
          value="workflow"
          onclick={() => navigateWithinWorkflow("workflow-stages")}
        >
          {copy.workflowStages}
        </Tabs.Trigger>
        <Tabs.Trigger
          value="decisions"
          onclick={() => navigateWithinWorkflow(decisionDetail(decisionKind))}
        >
          {copy.decisions}
        </Tabs.Trigger>
        <Tabs.Trigger
          value="task"
          onclick={() => navigateWithinWorkflow("agent-task")}
        >
          {copy.taskCenter}
        </Tabs.Trigger>
      </Tabs.List>

      <Tabs.Content
        id="workflow-stages"
        value="workflow"
        class="scroll-mt-64 space-y-[var(--density-section-gap)] pt-[var(--density-section-gap)]"
      >
        <div class="flex flex-wrap gap-2">
          <Button variant="outline" class="min-h-9" disabled={busy} onclick={refreshWorkflow}>
            <RefreshCw size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
            {copy.refresh}
          </Button>
          <Button class="min-h-9" disabled={busy} onclick={startWorkflow}>
            <Play size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
            {copy.startWorkflow}
          </Button>
        </div>

        {#if workflow}
          <div class="grid gap-[var(--density-section-gap)] xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)]">
            <Card.Root>
              <Card.Header>
                <Card.Title>{copy.workflowStages}</Card.Title>
                <Card.Description>
                  {workflow.status.status} · {workflow.status.run_id}
                </Card.Description>
              </Card.Header>
              <Card.Content>
                <div class="grid gap-2 md:grid-cols-2">
                  {#each workflow.status.stages as stage (stage.stage)}
                    <Button
                      variant="outline"
                      class={[
                        "h-auto min-h-9 w-full flex-col items-stretch gap-2 p-[var(--density-panel-padding)] text-left",
                        selectedStage === stage.stage ? "border-primary bg-accent/45" : "",
                      ]}
                      onclick={() => (selectedStage = stage.stage)}
                    >
                      <div class="flex items-center justify-between gap-3">
                        <span class="text-sm font-semibold">
                          {workflowStageLabel(presentation, stage.stage)}
                        </span>
                        <Badge variant={stage.status === "blocked" || stage.status === "stale" ? "destructive" : "outline"}>
                          {stage.status}
                        </Badge>
                      </div>
                      <p class="mt-2 truncate text-xs text-muted-foreground">
                        {stage.execution_mode ?? "—"}
                      </p>
                    </Button>
                  {/each}
                </div>
              </Card.Content>
            </Card.Root>

            <div class="space-y-[var(--density-section-gap)]">
              <Card.Root>
                <Card.Header>
                  <Card.Title>{workflowStageLabel(presentation, selectedStage)}</Card.Title>
                  <Card.Description>{selectedStageState?.status ?? "—"}</Card.Description>
                </Card.Header>
                <Card.Content class="space-y-[var(--density-section-gap)]">
                  <div class="space-y-2">
                    <Label for="workflow-mode">{copy.executionMode}</Label>
                    <NativeSelect.Root
                      id="workflow-mode"
                      size="desktop"
                      class="w-full"
                      bind:value={executionMode}
                    >
                      {#each selectedDescriptor?.execution_modes ?? [] as mode}
                        <NativeSelect.Option value={mode}>{mode}</NativeSelect.Option>
                      {/each}
                    </NativeSelect.Root>
                  </div>
                  <Button
                    class="min-h-9 w-full"
                    disabled={busy || !selectedDescriptor?.execution_modes.length}
                    onclick={beginStage}
                  >
                    <CircleDot size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                    {copy.beginStage}
                  </Button>
                  <Separator />
                  <div class="space-y-2">
                    <Label for="workflow-artifact">{copy.artifactId}</Label>
                    <Input id="workflow-artifact" bind:value={artifactId} />
                  </div>
                  <Button
                    variant="outline"
                    class="min-h-9 w-full"
                    disabled={busy || !artifactId.trim()}
                    onclick={completeStage}
                  >
                    <CheckCircle2 size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                    {copy.completeStage}
                  </Button>
                  <Button
                    variant="outline"
                    class="min-h-9 w-full"
                    disabled={busy || selectedStage === "intake"}
                    onclick={previewRerun}
                  >
                    <RotateCcw size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                    {copy.rerunStage}
                  </Button>
                </Card.Content>
              </Card.Root>

              <Card.Root>
                <Card.Header>
                  <Card.Title>{copy.blockers}</Card.Title>
                </Card.Header>
                <Card.Content class="space-y-2">
                  {#each workflow.status.blockers as blocker (blocker.code)}
                    <Alert.Root variant="destructive">
                      <Alert.Title>{blocker.code}</Alert.Title>
                      <Alert.Description>{blocker.description}</Alert.Description>
                    </Alert.Root>
                  {:else}
                    <p class="text-sm text-muted-foreground">{copy.noBlockers}</p>
                  {/each}
                </Card.Content>
              </Card.Root>
            </div>
          </div>
        {:else}
          <Card.Root>
            <Card.Content>
              <Empty.Root class="min-h-32">
                <Empty.Header>
                  <Empty.Media variant="icon"><GitBranch size={22} strokeWidth={1.8} aria-hidden="true" /></Empty.Media>
                  <Empty.Description>{copy.noWorkflow}</Empty.Description>
                </Empty.Header>
              </Empty.Root>
            </Card.Content>
          </Card.Root>
        {/if}
      </Tabs.Content>

      <Tabs.Content value="decisions" class="space-y-[var(--density-section-gap)] pt-[var(--density-section-gap)]">
        <Card.Root
          id={`decision-${decisionKind}`}
          class="scroll-mt-64 "
        >
          <Card.Header>
            <Card.Title>{copy.decisions}</Card.Title>
            <Card.Description>{copy.privateWorkspaceConsent}</Card.Description>
          </Card.Header>
          <Card.Content class="space-y-[var(--density-section-gap)]">
            <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
              <Checkbox id="workflow-private-session" bind:checked={privateSessionConsent} class="mt-0.5" />
              <Label for="workflow-private-session" class="text-xs leading-5 font-normal">
                <span class="flex items-center gap-2">
                  <ShieldCheck size={14} strokeWidth={1.8} aria-hidden="true" />
                  {copy.privateWorkspaceConsent}
                </span>
              </Label>
            </div>
            <div class="grid gap-[var(--density-section-gap)] lg:grid-cols-[220px_auto_auto_1fr] lg:items-end">
              <div class="space-y-2">
                <Label for="decision-kind">{copy.decisions}</Label>
                <NativeSelect.Root
                  id="decision-kind"
                  size="desktop"
                  class="w-full"
                  bind:value={decisionKind}
                  onchange={() => {
                    decisionJson = "";
                    decisionBaseline = null;
                    decisionPreview = null;
                    decisionEditable = false;
                    navigateWithinWorkflow(decisionDetail(decisionKind));
                  }}
                >
                  <NativeSelect.Option value="evidence">{copy.evidence}</NativeSelect.Option>
                  <NativeSelect.Option value="criteria">{copy.criteria}</NativeSelect.Option>
                  <NativeSelect.Option value="matches">{copy.matches}</NativeSelect.Option>
                  <NativeSelect.Option value="plan">{copy.plan}</NativeSelect.Option>
                </NativeSelect.Root>
              </div>
              <Button
                variant="outline"
                class="min-h-9"
                disabled={busy || !privateSessionConsent}
                onclick={() => loadDecision(false)}
              >
                {copy.loadCandidate}
              </Button>
              {#if decisionKind === "plan"}
                <Button
                  variant="outline"
                  class="min-h-9"
                  disabled={busy || !privateSessionConsent}
                  onclick={() => loadDecision(true)}
                >
                  {copy.loadCurrent}
                </Button>
              {/if}
              <p class="text-right text-xs text-muted-foreground">
                {decisionLabel(decisionKind)}
              </p>
            </div>
            <div class="space-y-2">
              <Label for="decision-json">{copy.candidateJson}</Label>
              <Textarea
                id="decision-json"
                class="min-h-[300px] resize-y font-mono text-xs leading-5"
                bind:value={decisionJson}
                spellcheck={false}
                disabled={!decisionJson || !decisionEditable}
              />
            </div>
            <Button
              variant="outline"
              class="min-h-9"
              disabled={busy || !decisionEditable || !decisionJson || !privateSessionConsent}
              onclick={previewDecision}
            >
              {copy.previewProposal}
            </Button>
            {#if decisionPreview}
              <Page.Panel tone="primary" class="space-y-[var(--density-section-gap)]">
                <div class="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <Badge variant="secondary">{copy.reviewBeforeCommit}</Badge>
                    <p class="mt-2 text-sm font-semibold">
                      {copy.proposalDiff}
                    </p>
                    <p class="mt-1 text-xs leading-5 text-muted-foreground">
                      {copy.proposalPreviewNotCommit}
                    </p>
                  </div>
                  <Badge variant="outline">
                    {decisionPreview.diff.totalChanges} {copy.changedFields}
                  </Badge>
                </div>

                <div class="max-h-72 overflow-auto rounded-lg border bg-background">
                  {#each decisionPreview.diff.changes as change (change.path)}
                    <div class="border-b p-3 last:border-b-0">
                      <p class="break-all font-mono text-[11px] font-semibold">
                        {change.path}
                      </p>
                      <div class="mt-2 grid gap-2 lg:grid-cols-2">
                        <div class="rounded-md bg-muted/40 p-2">
                          <p class="text-[10px] font-medium text-muted-foreground">
                            {copy.before}
                          </p>
                          <p class="mt-1 break-words font-mono text-[11px]">
                            {change.before}
                          </p>
                        </div>
                        <div class="rounded-md bg-primary/5 p-2">
                          <p class="text-[10px] font-medium text-muted-foreground">
                            {copy.after}
                          </p>
                          <p class="mt-1 break-words font-mono text-[11px]">
                            {change.after}
                          </p>
                        </div>
                      </div>
                    </div>
                  {:else}
                    <p class="p-[var(--density-panel-padding)] text-xs text-muted-foreground">{copy.noProposalChanges}</p>
                  {/each}
                </div>
                {#if decisionPreview.diff.truncated || decisionPreview.diff.comparisonLimited}
                  <p class="text-xs text-muted-foreground">{copy.diffTruncated}</p>
                {/if}

                <div class="grid gap-[var(--density-section-gap)] lg:grid-cols-2">
                  <div class="rounded-lg border bg-background p-3">
                    <p class="text-xs font-semibold">{copy.revisionProvenance}</p>
                    <div class="mt-2 space-y-2">
                      {#each decisionPreview.references as reference (`${reference.path}:${reference.id}:${reference.revision}`)}
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
                    {copy.decisionMutationDescription}
                  </p>
                </div>

                <div class="flex flex-wrap justify-end gap-2">
                  <Button variant="outline" disabled={busy} onclick={editDecision}>
                    {copy.editProposal}
                  </Button>
                  <Button
                    class="min-h-9"
                    disabled={busy}
                    onclick={confirmDecision}
                  >
                    {copy.confirmCandidate}
                  </Button>
                </div>
              </Page.Panel>
            {/if}
          </Card.Content>
        </Card.Root>
      </Tabs.Content>

      <Tabs.Content
        id="agent-task"
        value="task"
        class={[
          "scroll-mt-64 space-y-[var(--density-section-gap)] pt-[var(--density-section-gap)]",
          focus === "agent-task" ? "rounded-lg ring-2 ring-primary/25" : "",
        ]}
      >
        <div class="grid gap-[var(--density-section-gap)] xl:grid-cols-[minmax(320px,0.8fr)_minmax(0,1.2fr)]">
          <div class="space-y-[var(--density-section-gap)]">
            <Card.Root>
              <Card.Header>
                <Card.Title>{copy.prepareTask}</Card.Title>
                <Card.Description>{copy.workflowDescription}</Card.Description>
              </Card.Header>
              <Card.Content class="space-y-[var(--density-section-gap)]">
                <div class="space-y-2">
                  <Label for="task-operation">{copy.taskOperation}</Label>
                  <NativeSelect.Root
                    id="task-operation"
                    size="desktop"
                    class="w-full"
                    bind:value={taskOperation}
                  >
                    {#each taskOperationOptions as operation (operation.id)}
                      <NativeSelect.Option value={operation.id}>{operation.label}</NativeSelect.Option>
                    {/each}
                  </NativeSelect.Root>
                </div>
                <div class="space-y-2">
                  <Label for="task-mode">{copy.taskMode}</Label>
                  <NativeSelect.Root
                    id="task-mode"
                    size="desktop"
                    class="w-full"
                    bind:value={taskMode}
                  >
                    <NativeSelect.Option value="host-agent">host-agent</NativeSelect.Option>
                    <NativeSelect.Option value="configured-provider">configured-provider</NativeSelect.Option>
                  </NativeSelect.Root>
                </div>
                <div class="flex flex-wrap gap-2">
                  <Button class="min-h-9" disabled={busy} onclick={prepareTask}>
                    <Bot size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                    {copy.prepareTask}
                  </Button>
                  <Button variant="outline" class="min-h-9" disabled={busy} onclick={refreshTask}>
                    {copy.refresh}
                  </Button>
                </div>
              </Card.Content>
            </Card.Root>

            <Card.Root>
              <Card.Header>
                <Card.Title>{copy.taskStatus}</Card.Title>
                <Card.Description>{task?.descriptor.operation ?? copy.noTask}</Card.Description>
              </Card.Header>
              <Card.Content>
                {#if task}
                  <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-3 text-sm">
                    <dt class="text-muted-foreground">ID</dt>
                    <dd class="truncate text-right font-mono text-xs">{task.descriptor.id}</dd>
                    <dt class="text-muted-foreground">{copy.status}</dt>
                    <dd class="text-right font-medium">{task.status}</dd>
                    <dt class="text-muted-foreground">{copy.executionMode}</dt>
                    <dd class="text-right font-medium">{task.descriptor.execution_mode}</dd>
                  </dl>
                  <Separator class="my-4" />
                  <div class="flex flex-wrap gap-2">
                    {#if task.status === "prepared"}
                      <Button
                        disabled={busy}
                        onclick={() =>
                          void onNavigate({
                            view: "agent",
                            detail: "agent-task",
                            jobId: selectedJobId,
                          })}
                      >
                        {copy.continueInAgent}
                      </Button>
                    {/if}
                    {#if task.status === "committed" && task.result}
                      <Button
                        disabled={busy}
                        onclick={() =>
                          task && void onOpenTaskResult(task.descriptor.operation)}
                      >
                        {copy.openAgentResult}
                      </Button>
                    {/if}
                    <Button
                      variant="outline"
                      disabled={busy || task.status !== "prepared"}
                      onclick={cancelCurrentTask}
                    >
                      {copy.cancelTask}
                    </Button>
                    <Button
                      variant="outline"
                      disabled={busy || (task.status !== "cancelled" && task.status !== "stale")}
                      onclick={prepareCurrentTaskAgain}
                    >
                      {copy.prepareAgain}
                    </Button>
                  </div>
                {:else}
                  <p class="text-sm text-muted-foreground">{copy.noTask}</p>
                {/if}
              </Card.Content>
            </Card.Root>
          </div>

          <div class="space-y-[var(--density-section-gap)]">
            <Card.Root>
              <Card.Header>
                <Card.Title>{copy.exportInputs}</Card.Title>
                <Card.Description>{copy.privateWorkspaceConsent}</Card.Description>
              </Card.Header>
              <Card.Content class="space-y-[var(--density-section-gap)]">
                <div class="space-y-2">
                  <Label for="task-export-destination">{copy.exportDestination}</Label>
                  <div class="flex gap-2">
                    <Input id="task-export-destination" bind:value={taskExportDestination} readonly />
                    <Button variant="outline" class="shrink-0" onclick={chooseTaskDestination}>
                      <FolderOpen size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                      {copy.chooseDirectory}
                    </Button>
                  </div>
                </div>
                <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
                  <Checkbox id="task-private-consent" bind:checked={taskPrivateConsent} class="mt-0.5" />
                  <Label for="task-private-consent" class="text-xs leading-5 font-normal">
                    {copy.privateWorkspaceConsent}
                  </Label>
                </div>
                {#if taskMode === "configured-provider" || task?.descriptor.execution_mode === "configured-provider"}
                  <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
                    <Checkbox id="task-provider-consent" bind:checked={providerSendConsent} class="mt-0.5" />
                    <Label for="task-provider-consent" class="text-xs leading-5 font-normal">
                      {copy.providerSendConsent}
                    </Label>
                  </div>
                {/if}
                <Button
                  class="min-h-9"
                  disabled={!desktopRuntime || busy || !task || !taskExportDestination || !taskPrivateConsent}
                  onclick={exportInputs}
                >
                  {copy.exportInputs}
                </Button>
              </Card.Content>
            </Card.Root>

            <Card.Root>
              <Card.Header>
                <Card.Title>{copy.previewCompletion}</Card.Title>
                <Card.Description>{copy.reviewExactCompletion}</Card.Description>
              </Card.Header>
              <Card.Content class="space-y-[var(--density-section-gap)]">
                <div class="space-y-2">
                  <Label for="task-completion-file">{copy.taskCompletionFile}</Label>
                  <div class="flex gap-2">
                    <Input id="task-completion-file" bind:value={taskCompletionFile} readonly />
                    <Button variant="outline" class="shrink-0" onclick={chooseCompletion}>
                      <FileJson size={16} strokeWidth={1.8} data-icon="inline-start" aria-hidden="true" />
                      {copy.chooseFile}
                    </Button>
                  </div>
                </div>
                <Button
                  variant="outline"
                  class="min-h-9"
                  disabled={!desktopRuntime || busy || !taskCompletionFile || !taskPrivateConsent}
                  onclick={previewCompletion}
                >
                  {copy.previewCompletion}
                </Button>
                {#if taskCompletionPreview}
                  <Page.Panel tone="accent" class="space-y-[var(--density-section-gap)]">
                    <div class="flex flex-wrap items-start justify-between gap-3">
                      <div>
                        <Badge variant="secondary">{copy.validatedPreview}</Badge>
                        <p class="mt-3 text-sm text-muted-foreground">
                          {taskCompletionPreview.preview.summary}
                        </p>
                      </div>
                      <Badge variant="outline">
                        {taskCompletionPreview.preview.data.state.descriptor.operation}
                      </Badge>
                    </div>
                    <div class="grid gap-3 lg:grid-cols-3">
                      <div class="rounded-lg border bg-background p-3">
                        <p class="text-[10px] font-medium text-muted-foreground">
                          {copy.jobRevision}
                        </p>
                        <p class="mt-1 font-mono text-xs">
                          r{taskCompletionPreview.preview.data.state.descriptor.job_revision}
                        </p>
                      </div>
                      <div class="rounded-lg border bg-background p-3">
                        <p class="text-[10px] font-medium text-muted-foreground">
                          {copy.outputArtifact}
                        </p>
                        <p class="mt-1 font-mono text-xs">
                          {taskCompletionPreview.preview.data.state.descriptor.allowed_output_kind}
                        </p>
                      </div>
                      <div class="rounded-lg border bg-background p-3">
                        <p class="text-[10px] font-medium text-muted-foreground">
                          {copy.declaredInputs}
                        </p>
                        <p class="mt-1 font-mono text-xs">
                          {taskCompletionPreview.preview.data.state.descriptor.input_artifacts.length}
                        </p>
                      </div>
                    </div>
                    <Accordion.Root type="single">
                      <Accordion.Item value="revision-provenance" class="rounded-lg border bg-background px-3">
                        <Accordion.Trigger level={2} class="text-xs font-semibold">
                          {copy.revisionProvenance}
                        </Accordion.Trigger>
                        <Accordion.Content class="space-y-2 pb-3">
                        {#each taskCompletionPreview.preview.data.state.descriptor.input_artifacts as artifact (`${artifact.id}:${artifact.revision}`)}
                          <p class="break-all font-mono text-[10px] leading-4 text-muted-foreground">
                            {artifact.kind} · {artifact.id} · r{artifact.revision} · {artifact.sha256}
                          </p>
                        {:else}
                          <p class="text-xs text-muted-foreground">
                            {copy.noEmbeddedRevisionReferences}
                          </p>
                        {/each}
                        </Accordion.Content>
                      </Accordion.Item>
                    </Accordion.Root>
                    <div class="grid gap-3 lg:grid-cols-2">
                      <div class="rounded-lg border bg-background p-3">
                        <p class="text-xs font-semibold">{copy.validationAtCommit}</p>
                        <ul class="mt-2 list-disc space-y-1.5 pl-4 text-xs leading-5 text-muted-foreground">
                          <li>{copy.validateCandidateSchema}</li>
                          <li>{copy.validateCurrentRevisions}</li>
                          <li>{copy.validateTaskLease}</li>
                        </ul>
                      </div>
                      <div class="rounded-lg border bg-background p-3">
                        <p class="text-xs font-semibold">{copy.intendedStateChange}</p>
                        <p class="mt-2 text-xs leading-5 text-muted-foreground">
                          {copy.taskMutationDescription}
                        </p>
                      </div>
                    </div>
                    <p class="text-xs leading-5 text-muted-foreground">
                      {copy.reviewExactCompletion}
                    </p>
                    <Button class="mt-[var(--density-section-gap)] min-h-9" disabled={busy} onclick={commitCompletion}>
                      {copy.commitCompletion}
                    </Button>
                  </Page.Panel>
                {/if}
              </Card.Content>
            </Card.Root>
          </div>
        </div>
      </Tabs.Content>
    </Tabs.Root>
  {/if}

  {#if formError}
    <Alert.Root variant="destructive">
      <Alert.Description>{formError}</Alert.Description>
    </Alert.Root>
  {/if}
</Page.Root>

<Dialog.Root bind:open={rerunOpen}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>{copy.rerunStage}</Dialog.Title>
      <Dialog.Description>{copy.rerunDescription}</Dialog.Description>
    </Dialog.Header>
    {#if rerunPreview}
      <div class="space-y-[var(--density-section-gap)]">
        <div class="rounded-lg border bg-muted/20 p-[var(--density-panel-padding)]">
          <p class="text-sm font-medium">{rerunPreview.preview.summary}</p>
          <p class="mt-3 text-xs font-semibold">{copy.affectedStages}</p>
          <div class="mt-2 flex flex-wrap gap-2">
            {#each rerunPreview.preview.data.affected_stages as stage}
              <Badge variant="outline">{stage}</Badge>
            {/each}
          </div>
        </div>
      </div>
    {/if}
    <Dialog.Footer>
      <Button variant="outline" disabled={busy} onclick={closeRerun}>{copy.cancel}</Button>
      <Button disabled={busy || !rerunPreview} onclick={commitRerun}>
        {copy.commitRerun}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
