<script lang="ts">
  import {
    CheckCircle2,
    FileCheck2,
    FileUp,
    Link,
    Plus,
    RefreshCw,
    ShieldCheck,
  } from "@lucide/svelte";

  import * as Page from "$lib/components/patterns/page/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import * as Alert from "$lib/components/ui/alert/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import * as Empty from "$lib/components/ui/empty/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import * as NativeSelect from "$lib/components/ui/native-select/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";
  import {
    approveGenericApplication,
    auditDeliverablesV4,
    chooseApplicationSource,
    commandErrorMessage,
    commitDeliverableDraftV4,
    commitEvidenceAssociationV4,
    commitPlanConfirmationV4,
    commitPlanProposalV4,
    commitProfileAssociationV4,
    commitRequirementConfirmationV4,
    discardEvidenceAssociationV4,
    discardProfileAssociationV4,
    commitApplicationIntakePreview,
    discardApplicationIntakePreview,
    exportGenericApplication,
    listGenericApplications,
    listEvidenceAssociationsV4,
    listProfileAssociationsV4,
    previewDeliverableDraftV4,
    previewLocalApplicationIntake,
    previewPlanConfirmationV4,
    previewPlanProposalV4,
    previewEvidenceAssociationV4,
    previewPastedApplicationIntake,
    previewProfileAssociationV4,
    previewRequirementConfirmationV4,
    previewUrlApplicationIntake,
    showGenericApplication,
    type ApplicationIntakeBaseRequestV4,
    type ApplicationIntakePreviewTokenReadModelV4,
    type ApplicationFieldValueV3,
    type ContentRevisionReferenceV3,
    type EvidenceAssociationListReadModelV4,
    type EvidenceAssociationPreviewRequestV4,
    type ApplicationFlowDeliverableDraftV3,
    type ApplicationFlowComposeRequestV3,
    type ApplicationFlowReviewReadModelV3,
    type ApplicationFlowStageV3,
    type BuiltInWorkflowPackId,
    type ApplicationMutationApprovalPreviewV4,
    type ApplicationPlanConfirmRequestV4,
    type ApplicationPlanProposeRequestV4,
    type ApplicationRequirementConfirmRequestV4,
    type ProfileAssociationListReadModelV4,
    type ProfileAssociationPreviewRequestV4,
    type StoredApplicationModelV3,
    type WorkflowPackPresentationField,
    type WorkflowPackPresentationReadModel,
    type WorkspaceReadModel,
  } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";

  type Props = {
    copy: Messages;
    desktopRuntime: boolean;
    activeWorkspace: WorkspaceReadModel;
    packId: BuiltInWorkflowPackId;
    presentation: WorkflowPackPresentationReadModel | null;
    requestedApplicationId: string;
    onContextChange: (context: {
      workspacePath: string;
      packId: BuiltInWorkflowPackId;
      applications: StoredApplicationModelV3[];
      selected: StoredApplicationModelV3 | null;
      stages: ApplicationFlowStageV3[];
    }) => void;
  };

  let {
    copy,
    desktopRuntime,
    activeWorkspace,
    packId,
    presentation,
    requestedApplicationId,
    onContextChange,
  }: Props = $props();

  let applications = $state<StoredApplicationModelV3[]>([]);
  let selected = $state<StoredApplicationModelV3 | null>(null);
  let stages = $state<ApplicationFlowStageV3[]>([]);
  let review = $state<ApplicationFlowReviewReadModelV3 | null>(null);
  let busy = $state(false);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let loadedCollection = "";

  let title = $state("");
  let sourceText = $state("");
  let requirementCategory = $state("");
  let requirementPriority = $state<"mandatory" | "recommended" | "informational">("mandatory");
  let requirementDecisions = $state<Record<string, "confirm" | "exclude">>({});
  let intakeMode = $state<"pasted" | "local" | "url">("pasted");
  let localSource = $state("");
  let sourceUrl = $state("");
  let privateReadConfirmed = $state(false);
  let networkFetchConfirmed = $state(false);
  let intakePreview = $state<ApplicationIntakePreviewTokenReadModelV4 | null>(null);
  let opportunityValues = $state<Record<string, string>>({});
  let applicationValues = $state<Record<string, string>>({});
  let deliverableSelections = $state<Record<string, boolean>>({});
  let deliverableDrafts = $state<Record<string, { title: string; content: string }>>({});
  let privateReviewConsent = $state(false);
  let reviewConfirmed = $state(false);
  let exportDestination = $state("");
  let privateExportConsent = $state(false);
  let profileAssociations = $state<ProfileAssociationListReadModelV4 | null>(null);
  let evidenceAssociations = $state<EvidenceAssociationListReadModelV4 | null>(null);
  let profileSelections = $state<Record<string, boolean>>({});
  let evidenceSelections = $state<Record<string, boolean>>({});
  let associationPrivateConsent = $state(false);
  let pendingAssociationChanges = $state<PendingAssociationChange[]>([]);
  let pendingLifecycleMutation = $state<PendingLifecycleMutation | null>(null);

  type PendingLifecycleMutation = {
    kind: "requirements" | "plan-proposal" | "plan-confirmation" | "deliverable-draft";
    workspace: string;
    applicationId: string;
    previewToken: string;
    previewSha256: string;
    changes: string[];
    summary: string;
  };

  type PendingAssociationChange =
    | {
        resource: "profile";
        label: string;
        request: ProfileAssociationPreviewRequestV4;
        previewToken: string;
        previewSha256: string;
        requiresPrivateRead: boolean;
      }
    | {
        resource: "evidence";
        label: string;
        request: EvidenceAssociationPreviewRequestV4;
        previewToken: string;
        previewSha256: string;
        requiresPrivateRead: boolean;
      };

  const selectedRevision = $derived(selected?.snapshot.application.revision ?? 0);
  const completedStages = $derived(stages.filter((stage) => stage.state === "complete").length);
  const stageProgress = $derived(stages.length ? (completedStages / stages.length) * 100 : 0);
  const plannedDeliverables = $derived(
    selected?.snapshot.plan?.deliverables.filter(
      (deliverable) => deliverable.disposition !== "omitted",
    ) ?? [],
  );
  const pendingAssociationNeedsPrivateRead = $derived(
    pendingAssociationChanges.some((item) => item.requiresPrivateRead),
  );

  $effect(() => {
    const workspace = activeWorkspace.path;
    const collection = `${workspace}\u0000${packId}`;
    if (!workspace || collection === loadedCollection) return;
    const pendingPreview = intakePreview;
    if (pendingPreview) {
      const pendingWorkspace = loadedCollection.split("\u0000", 1)[0] || workspace;
      void discardApplicationIntakePreview(
        pendingWorkspace,
        pendingPreview.preview.data.pack_id,
        pendingPreview.preview_token,
      );
    }
    loadedCollection = collection;
    intakePreview = null;
    selected = null;
    stages = [];
    review = null;
    profileAssociations = null;
    evidenceAssociations = null;
    pendingAssociationChanges = [];
    void discardPendingLifecycleMutation();
    requirementCategory = presentation?.requirement_categories[0]?.id ?? "";
    opportunityValues = {};
    applicationValues = {};
    deliverableSelections = {};
    deliverableDrafts = {};
    void refresh();
  });

  $effect(() => {
    onContextChange({
      workspacePath: activeWorkspace.path,
      packId,
      applications,
      selected,
      stages,
    });
  });

  $effect(() => {
    const requested = requestedApplicationId;
    if (!requested || selected?.snapshot.application.id === requested) return;
    const application = applications.find((item) => item.snapshot.application.id === requested);
    if (application) void selectApplication(application);
  });

  $effect(() => {
    if (!requirementCategory && presentation?.requirement_categories[0]) {
      requirementCategory = presentation.requirement_categories[0].id;
    }
    for (const deliverable of presentation?.deliverables ?? []) {
      if (!(deliverable.id in deliverableSelections)) {
        deliverableSelections[deliverable.id] = deliverable.minimum > 0;
      }
    }
  });

  function localId(value: string): string {
    return value.includes(":") ? (value.split(":").at(-1) ?? value) : value;
  }

  function deliverableLabel(kind: string): string {
    const id = localId(kind);
    return presentation?.deliverables.find((item) => item.id === id)?.label.value ?? id;
  }

  function fieldInputType(field: WorkflowPackPresentationField): string {
    if (field.field_type === "date") return "date";
    if (field.field_type === "url") return "url";
    if (field.field_type === "integer") return "number";
    return "text";
  }

  function metadata(
    fields: WorkflowPackPresentationField[],
    values: Record<string, string>,
  ): Record<string, ApplicationFieldValueV3> {
    const result: Record<string, ApplicationFieldValueV3> = {};
    for (const field of fields) {
      const raw = values[field.id]?.trim() ?? "";
      if (!raw && !field.required) continue;
      if (field.field_type === "integer") {
        result[field.id] = { type: "integer", value: Number(raw) };
      } else if (field.field_type === "boolean") {
        result[field.id] = { type: "boolean", value: raw === "true" };
      } else if (field.field_type === "string-list") {
        result[field.id] = {
          type: "string-list",
          value: raw
            .split("\n")
            .map((item) => item.trim())
            .filter(Boolean),
        };
      } else {
        result[field.id] = { type: field.field_type, value: raw };
      }
    }
    return result;
  }

  function captureError(value: unknown): void {
    error = commandErrorMessage(value);
    notice = null;
  }

  async function refresh(preferredId = selected?.snapshot.application.id): Promise<void> {
    loading = true;
    error = null;
    try {
      const receipt = await listGenericApplications(activeWorkspace.path);
      applications = receipt.data.filter((application) => application.snapshot.pack.id === packId);
      const next =
        applications.find((item) => item.snapshot.application.id === preferredId) ??
        applications[0] ??
        null;
      if (next) await selectApplication(next);
      else {
        selected = null;
        stages = [];
      }
    } catch (value) {
      captureError(value);
    } finally {
      loading = false;
    }
  }

  async function selectApplication(application: StoredApplicationModelV3): Promise<void> {
    await discardPendingLifecycleMutation();
    error = null;
    review = null;
    reviewConfirmed = false;
    try {
      const receipt = await showGenericApplication(
        activeWorkspace.path,
        application.snapshot.application.id,
      );
      selected = receipt.data.stored;
      stages = receipt.data.stages;
      requirementDecisions = Object.fromEntries(
        receipt.data.stored.snapshot.requirements.map((requirement) => [
          requirement.id,
          requirement.confirmation === "excluded" ? "exclude" : "confirm",
        ]),
      );
      await loadAssociationContext(receipt.data.stored.snapshot.application.id);
      exportDestination = `applications/${receipt.data.stored.snapshot.application.id}/exports/revision-${receipt.data.stored.snapshot.application.revision}`;
      prepareDrafts();
    } catch (value) {
      captureError(value);
    }
  }

  function profileReference(
    source: ProfileAssociationListReadModelV4["profile_sources"][number],
  ): ContentRevisionReferenceV3 {
    return {
      id: source.id,
      revision: source.revision,
      sha256: String(source.original.sha256),
    };
  }

  function linkedProfile(id: string) {
    return profileAssociations?.associations.find((item) => item.profile_source.id === id);
  }

  function linkedEvidence(id: string) {
    return evidenceAssociations?.associations.find((item) => item.evidence.id === id);
  }

  async function loadAssociationContext(applicationId: string): Promise<void> {
    const [profiles, evidence] = await Promise.all([
      listProfileAssociationsV4(activeWorkspace.path, applicationId),
      listEvidenceAssociationsV4(activeWorkspace.path, applicationId),
    ]);
    profileAssociations = profiles.data;
    evidenceAssociations = evidence.data;
    profileSelections = Object.fromEntries([
      ...profiles.data.profile_sources.map((source) => [source.id, false] as const),
      ...profiles.data.associations.map(
        (association) => [association.profile_source.id, true] as const,
      ),
    ]);
    evidenceSelections = Object.fromEntries([
      ...evidence.data.evidence.map((item) => [item.evidence.id, false] as const),
      ...evidence.data.associations.map((association) => [association.evidence.id, true] as const),
    ]);
    associationPrivateConsent = false;
    pendingAssociationChanges = [];
  }

  async function previewAssociationChanges(): Promise<void> {
    if (!selected || !profileAssociations || !evidenceAssociations) return;
    await run(async () => {
      await discardPendingAssociationChanges();
      const pending: PendingAssociationChange[] = [];
      try {
        for (const source of profileAssociations!.profile_sources) {
          const association = linkedProfile(source.id);
          const selectedNow = profileSelections[source.id] ?? false;
          if (selectedNow === Boolean(association)) continue;
          const request: ProfileAssociationPreviewRequestV4 = {
            application_id: selected!.snapshot.application.id,
            profile_source: association?.profile_source ?? profileReference(source),
            change: selectedNow ? "associate" : "unlink",
          };
          const preview = await previewProfileAssociationV4(activeWorkspace.path, request);
          pending.push({
            resource: "profile",
            label: `${source.kind} · ${source.id}`,
            request,
            previewToken: preview.preview_token,
            previewSha256: preview.preview.data.preview_sha256,
            requiresPrivateRead: preview.preview.data.requires_private_read,
          });
        }
        for (const association of profileAssociations!.associations) {
          if (
            profileAssociations!.profile_sources.some(
              (source) => source.id === association.profile_source.id,
            ) ||
            profileSelections[association.profile_source.id] !== false
          ) {
            continue;
          }
          const request: ProfileAssociationPreviewRequestV4 = {
            application_id: selected!.snapshot.application.id,
            profile_source: association.profile_source,
            change: "unlink",
          };
          const preview = await previewProfileAssociationV4(activeWorkspace.path, request);
          pending.push({
            resource: "profile",
            label: `${copy.staleAssociation} · ${association.profile_source.id}`,
            request,
            previewToken: preview.preview_token,
            previewSha256: preview.preview.data.preview_sha256,
            requiresPrivateRead: false,
          });
        }
        for (const item of evidenceAssociations!.evidence) {
          const association = linkedEvidence(item.evidence.id);
          const selectedNow = evidenceSelections[item.evidence.id] ?? false;
          if (selectedNow === Boolean(association)) continue;
          const request: EvidenceAssociationPreviewRequestV4 = {
            application_id: selected!.snapshot.application.id,
            evidence: association?.evidence ?? item.evidence,
            change: selectedNow ? "associate" : "unlink",
          };
          const preview = await previewEvidenceAssociationV4(activeWorkspace.path, request);
          pending.push({
            resource: "evidence",
            label: `${item.kind} · ${item.evidence.id}`,
            request,
            previewToken: preview.preview_token,
            previewSha256: preview.preview.data.preview_sha256,
            requiresPrivateRead: preview.preview.data.requires_private_read,
          });
        }
        for (const association of evidenceAssociations!.associations) {
          if (
            evidenceAssociations!.evidence.some(
              (item) => item.evidence.id === association.evidence.id,
            ) ||
            evidenceSelections[association.evidence.id] !== false
          ) {
            continue;
          }
          const request: EvidenceAssociationPreviewRequestV4 = {
            application_id: selected!.snapshot.application.id,
            evidence: association.evidence,
            change: "unlink",
          };
          const preview = await previewEvidenceAssociationV4(activeWorkspace.path, request);
          pending.push({
            resource: "evidence",
            label: `${copy.staleAssociation} · ${association.evidence.id}`,
            request,
            previewToken: preview.preview_token,
            previewSha256: preview.preview.data.preview_sha256,
            requiresPrivateRead: false,
          });
        }
      } catch (value) {
        pendingAssociationChanges = pending;
        try {
          await discardPendingAssociationChanges();
        } catch {
          // The broker expires any token that cannot be discarded during error recovery.
        }
        throw value;
      }
      pendingAssociationChanges = pending;
      notice = pending.length ? copy.associationPreviewReady : copy.noAssociationChanges;
    });
  }

  async function commitAssociationChanges(): Promise<void> {
    if (!selected || !pendingAssociationChanges.length) return;
    if (
      pendingAssociationChanges.some((item) => item.requiresPrivateRead) &&
      !associationPrivateConsent
    ) {
      error = copy.associationPrivateConsent;
      return;
    }
    await run(async () => {
      const applicationId = selected!.snapshot.application.id;
      let committed = false;
      try {
        for (const item of pendingAssociationChanges) {
          if (item.resource === "profile") {
            await commitProfileAssociationV4({
              workspace: activeWorkspace.path,
              applicationId,
              previewToken: item.previewToken,
              previewSha256: item.previewSha256,
              approved: true,
              confirmedPrivateRead: associationPrivateConsent,
            });
          } else {
            await commitEvidenceAssociationV4({
              workspace: activeWorkspace.path,
              applicationId,
              previewToken: item.previewToken,
              previewSha256: item.previewSha256,
              approved: true,
              confirmedPrivateRead: associationPrivateConsent,
            });
          }
        }
        committed = true;
      } finally {
        // Each reviewed change is an independent canonical mutation. Always reconcile the UI in
        // case a later stale preview fails after an earlier change has already committed.
        await discardPendingAssociationChanges();
        await loadAssociationContext(applicationId);
        const receipt = await showGenericApplication(activeWorkspace.path, applicationId);
        selected = receipt.data.stored;
        stages = receipt.data.stages;
      }
      if (committed) notice = copy.associationChangesCommitted;
    });
  }

  async function discardPendingAssociationChanges(): Promise<void> {
    const pending = pendingAssociationChanges;
    pendingAssociationChanges = [];
    for (const item of pending) {
      if (item.resource === "profile") {
        await discardProfileAssociationV4(
          activeWorkspace.path,
          item.request.application_id,
          item.previewToken,
        );
      } else {
        await discardEvidenceAssociationV4(
          activeWorkspace.path,
          item.request.application_id,
          item.previewToken,
        );
      }
    }
  }

  function prepareDrafts(): void {
    for (const planned of plannedDeliverables) {
      const id = localId(planned.kind);
      if (!deliverableDrafts[id]) {
        deliverableDrafts[id] = { title: deliverableLabel(id), content: "" };
      }
    }
  }

  async function run(action: () => Promise<void>): Promise<void> {
    busy = true;
    error = null;
    notice = null;
    try {
      await action();
    } catch (value) {
      captureError(value);
    } finally {
      busy = false;
    }
  }

  function intakeBase(): ApplicationIntakeBaseRequestV4 | null {
    if (!title.trim() || !requirementCategory) {
      error = copy.requirementMustMatchSource;
      return null;
    }
    return {
      pack_id: packId,
      title: title.trim(),
      opportunity_metadata: metadata(presentation?.opportunity_fields ?? [], opportunityValues),
      application_metadata: metadata(presentation?.application_fields ?? [], applicationValues),
      requirement_category: requirementCategory,
      requirement_priority: requirementPriority,
    };
  }

  async function chooseLocalSource(): Promise<void> {
    localSource = (await chooseApplicationSource()) ?? localSource;
  }

  async function submitIntakePreview(): Promise<void> {
    const base = intakeBase();
    if (!base) return;
    if (intakeMode === "pasted" && !sourceText.trim()) {
      error = copy.requirementMustMatchSource;
      return;
    }
    if (intakeMode === "local" && (!localSource || !privateReadConfirmed)) {
      error = privateReadConfirmed ? copy.chooseFile : copy.privateReadConsent;
      return;
    }
    if (intakeMode === "url" && (!sourceUrl.trim() || !networkFetchConfirmed)) {
      error = networkFetchConfirmed ? copy.sourceUrl : copy.networkFetchConsent;
      return;
    }
    await run(async () => {
      if (intakeMode === "pasted") {
        intakePreview = await previewPastedApplicationIntake(activeWorkspace.path, {
          ...base,
          source_text: sourceText,
        });
      } else if (intakeMode === "local") {
        intakePreview = await previewLocalApplicationIntake(
          activeWorkspace.path,
          { ...base, path: localSource },
          privateReadConfirmed,
        );
      } else {
        intakePreview = await previewUrlApplicationIntake(
          activeWorkspace.path,
          { ...base, url: sourceUrl.trim() },
          networkFetchConfirmed,
        );
      }
      notice = intakePreview.preview.summary;
    });
  }

  async function commitIntakePreview(): Promise<void> {
    if (!intakePreview) return;
    await run(async () => {
      const receipt = await commitApplicationIntakePreview(
        activeWorkspace.path,
        intakePreview!.preview.data.pack_id,
        intakePreview!.preview_token,
      );
      intakePreview = null;
      selected = receipt.data.stored;
      stages = receipt.data.stages;
      notice = receipt.summary;
      title = "";
      sourceText = "";
      localSource = "";
      sourceUrl = "";
      privateReadConfirmed = false;
      networkFetchConfirmed = false;
      opportunityValues = {};
      applicationValues = {};
      await refresh(selected.snapshot.application.id);
    });
  }

  async function discardIntakePreview(): Promise<void> {
    if (!intakePreview) return;
    await run(async () => {
      await discardApplicationIntakePreview(
        activeWorkspace.path,
        intakePreview!.preview.data.pack_id,
        intakePreview!.preview_token,
      );
      intakePreview = null;
      notice = copy.discardPreview;
    });
  }

  function rememberLifecyclePreview<T>(
    kind: PendingLifecycleMutation["kind"],
    preview: ApplicationMutationApprovalPreviewV4<T>,
  ): void {
    if (!selected) return;
    pendingLifecycleMutation = {
      kind,
      workspace: activeWorkspace.path,
      applicationId: selected.snapshot.application.id,
      previewToken: preview.preview_token,
      previewSha256: preview.preview.data.preview_sha256,
      changes: preview.preview.data.changes,
      summary: preview.preview.summary,
    };
    notice = preview.preview.summary;
  }

  async function previewRequirementDecisions(): Promise<void> {
    if (!selected) return;
    const decisions = Object.fromEntries(
      selected.snapshot.requirements.map((requirement) => [
        requirement.id,
        requirementDecisions[requirement.id] ?? "confirm",
      ]),
    ) as ApplicationRequirementConfirmRequestV4["decisions"];
    if (!Object.values(decisions).includes("confirm")) {
      error = copy.confirmAtLeastOneRequirement;
      return;
    }
    await run(async () => {
      const preview = await previewRequirementConfirmationV4(
        activeWorkspace.path,
        selected!.snapshot.application.id,
        { expected_revision: selected!.snapshot.application.revision, decisions },
      );
      rememberLifecyclePreview("requirements", preview);
    });
  }

  function planProposalRequest(): ApplicationPlanProposeRequestV4 | null {
    if (!selected || !presentation) return null;
    return {
      expected_revision: selected.snapshot.application.revision,
      decision: "proceed",
      deliverables: presentation.deliverables.map((item) => ({
        kind: item.id,
        disposition: deliverableSelections[item.id]
          ? item.minimum > 0
            ? "required"
            : "optional"
          : "omitted",
        rationale: "User reviewed this Pack Deliverable in the desktop plan.",
        constraints: ["Use only reviewed local source material and confirmed evidence."],
        execution_mode: "manual-import",
      })),
    };
  }

  async function previewPlanProposal(): Promise<void> {
    const mutation = planProposalRequest();
    if (!selected || !mutation) return;
    await run(async () => {
      const preview = await previewPlanProposalV4(
        activeWorkspace.path,
        selected!.snapshot.application.id,
        mutation,
      );
      rememberLifecyclePreview("plan-proposal", preview);
    });
  }

  async function previewPlanConfirmation(): Promise<void> {
    if (!selected) return;
    const mutation: ApplicationPlanConfirmRequestV4 = {
      expected_revision: selected.snapshot.application.revision,
    };
    await run(async () => {
      const preview = await previewPlanConfirmationV4(
        activeWorkspace.path,
        selected!.snapshot.application.id,
        mutation,
      );
      rememberLifecyclePreview("plan-confirmation", preview);
    });
  }

  async function previewCompose(): Promise<void> {
    if (!selected) return;
    const drafts: ApplicationFlowDeliverableDraftV3[] = plannedDeliverables.map((planned) => {
      const id = localId(planned.kind);
      const draft = deliverableDrafts[id] ?? { title: deliverableLabel(id), content: "" };
      return {
        kind: id,
        title: draft.title.trim(),
        media_type: "text/markdown",
        content: draft.content.trim(),
      };
    });
    if (drafts.some((draft) => !draft.title || !draft.content)) {
      error = copy.deliverableContent;
      return;
    }
    await run(async () => {
      const mutation: ApplicationFlowComposeRequestV3 = {
        expected_revision: selected!.snapshot.application.revision,
        deliverables: drafts,
      };
      const preview = await previewDeliverableDraftV4(
        activeWorkspace.path,
        selected!.snapshot.application.id,
        mutation,
      );
      rememberLifecyclePreview("deliverable-draft", preview);
    });
  }

  async function invokePendingLifecycleMutation(
    pending: PendingLifecycleMutation,
    approved: boolean,
  ) {
    const options = {
      workspace: pending.workspace,
      applicationId: pending.applicationId,
      previewToken: pending.previewToken,
      previewSha256: pending.previewSha256,
      approved,
    };
    if (pending.kind === "requirements") return commitRequirementConfirmationV4(options);
    if (pending.kind === "plan-proposal") return commitPlanProposalV4(options);
    if (pending.kind === "plan-confirmation") return commitPlanConfirmationV4(options);
    return commitDeliverableDraftV4(options);
  }

  async function commitPendingLifecycleMutation(): Promise<void> {
    if (!pendingLifecycleMutation) return;
    await run(async () => {
      const pending = pendingLifecycleMutation!;
      try {
        const receipt = await invokePendingLifecycleMutation(pending, true);
        notice = receipt.summary;
        review = null;
        reviewConfirmed = false;
      } finally {
        pendingLifecycleMutation = null;
        await refresh(pending.applicationId);
      }
    });
  }

  async function discardPendingLifecycleMutation(): Promise<void> {
    const pending = pendingLifecycleMutation;
    pendingLifecycleMutation = null;
    if (!pending) return;
    try {
      await invokePendingLifecycleMutation(pending, false);
    } catch {
      // Denial is the expected broker result and consumes the exact single-use preview token.
    }
  }

  async function loadReview(): Promise<void> {
    if (!selected || !privateReviewConsent) return;
    await run(async () => {
      const receipt = await auditDeliverablesV4(
        activeWorkspace.path,
        selected!.snapshot.application.id,
        privateReviewConsent,
      );
      review = receipt.data;
      stages = receipt.data.stages;
      notice = receipt.summary;
    });
  }

  async function submitApproval(): Promise<void> {
    if (!selected || !review || !reviewConfirmed) return;
    await run(async () => {
      const receipt = await approveGenericApplication(
        activeWorkspace.path,
        selected!.snapshot.application.id,
        selected!.snapshot.application.revision,
      );
      selected = receipt.data.commit.stored;
      stages = receipt.data.stages;
      notice = receipt.summary;
      await refresh(selected.snapshot.application.id);
    });
  }

  async function submitExport(): Promise<void> {
    if (!selected || !exportDestination.trim() || !privateExportConsent) return;
    await run(async () => {
      const receipt = await exportGenericApplication({
        workspace: activeWorkspace.path,
        applicationId: selected!.snapshot.application.id,
        expectedRevision: selected!.snapshot.application.revision,
        destination: exportDestination.trim(),
        confirmedPrivateExport: privateExportConsent,
      });
      stages = receipt.data.stages;
      notice = `${receipt.summary} ${copy.submissionBoundary}`;
    });
  }
</script>

{#snippet headerActions()}
  <Button variant="outline" disabled={busy || loading} onclick={() => refresh()}>
    <RefreshCw size={17} strokeWidth={1.8} aria-hidden="true" />
    {copy.refresh}
  </Button>
{/snippet}

<Page.Root>
  <Page.Header
    eyebrow={presentation?.vocabulary.application_plural ?? copy.applications}
    title={copy.applicationsTitle}
    description={copy.genericApplicationsDescription}
    actions={headerActions}
  />

  {#if error}
    <Alert.Root variant="destructive" role="alert" aria-live="assertive">
      <Alert.Description>{error}</Alert.Description>
    </Alert.Root>
  {/if}
  {#if notice}
    <Alert.Root variant="success" aria-live="polite">
      <CheckCircle2 size={17} strokeWidth={1.8} aria-hidden="true" />
      <Alert.Description>{notice}</Alert.Description>
    </Alert.Root>
  {/if}
  <Alert.Root aria-live="polite">
    <ShieldCheck size={17} strokeWidth={1.8} aria-hidden="true" />
    <Alert.Description>{copy.submissionBoundary}</Alert.Description>
  </Alert.Root>

  <Page.Grid class="xl:grid-cols-[minmax(280px,0.7fr)_minmax(0,1.3fr)]">
    <div class="space-y-[var(--density-section-gap)]">
      <Card.Root>
        <Card.Header>
          <Card.Title>{presentation?.vocabulary.application_plural ?? copy.applications}</Card.Title
          >
          <Card.Description class="truncate" title={activeWorkspace.path}>
            {activeWorkspace.path}
          </Card.Description>
        </Card.Header>
        <Card.Content class="space-y-2">
          {#if loading}
            <p class="text-sm text-muted-foreground" aria-live="polite">{copy.loading}</p>
          {:else if !applications.length}
            <Empty.Root class="min-h-28 border bg-muted/20">
              <Empty.Header>
                <Empty.Media variant="icon"><FileCheck2 size={20} aria-hidden="true" /></Empty.Media
                >
                <Empty.Title>{copy.noGenericApplications}</Empty.Title>
              </Empty.Header>
            </Empty.Root>
          {:else}
            {#each applications as application (application.snapshot.application.id)}
              <Button
                variant={selected?.snapshot.application.id === application.snapshot.application.id
                  ? "secondary"
                  : "ghost"}
                class="h-auto w-full justify-start px-3 py-2 text-left"
                aria-pressed={selected?.snapshot.application.id ===
                  application.snapshot.application.id}
                onclick={() => selectApplication(application)}
              >
                <span class="min-w-0">
                  <span class="block truncate font-medium"
                    >{application.snapshot.opportunity.title}</span
                  >
                  <span class="block text-xs text-muted-foreground">
                    {copy.revision}
                    {application.snapshot.application.revision} · {application.snapshot.application
                      .lifecycle}
                  </span>
                </span>
              </Button>
            {/each}
          {/if}
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header>
          <Card.Title>{copy.createApplication}</Card.Title>
          <Card.Description>{presentation?.pack.id}</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-4">
          {#if intakePreview}
            <div class="space-y-4" aria-live="polite" aria-atomic="true">
              <div class="flex flex-col justify-between gap-3 sm:flex-row sm:items-start">
                <div class="flex items-start gap-3">
                  <div
                    class="grid size-10 shrink-0 place-items-center rounded-lg bg-accent text-accent-foreground"
                  >
                    <ShieldCheck size={18} strokeWidth={1.8} aria-hidden="true" />
                  </div>
                  <div>
                    <p class="text-sm font-semibold">{copy.sourcePreviewTitle}</p>
                    <p class="mt-1 text-xs leading-5 text-muted-foreground">
                      {intakePreview.preview.summary}
                    </p>
                  </div>
                </div>
                <Badge variant="secondary">{intakePreview.preview.data.source_kind}</Badge>
              </div>

              <dl class="grid gap-3 rounded-lg border bg-muted/20 p-4 text-sm sm:grid-cols-2">
                <div class="min-w-0 space-y-1">
                  <dt class="text-xs font-medium text-muted-foreground">
                    {presentation?.vocabulary.requirement_plural}
                  </dt>
                  <dd>{intakePreview.preview.data.requirement_count}</dd>
                </div>
                <div class="min-w-0 space-y-1">
                  <dt class="text-xs font-medium text-muted-foreground">
                    {copy.intakeDuplicateSignal}
                  </dt>
                  <dd>{intakePreview.preview.data.duplicate_count}</dd>
                </div>
                <div class="min-w-0 space-y-1">
                  <dt class="text-xs font-medium text-muted-foreground">
                    {copy.intakeNormalizedText}
                  </dt>
                  <dd>
                    {intakePreview.preview.data.normalized_text_bytes} B ·
                    {intakePreview.preview.data.normalized_lines}
                  </dd>
                </div>
                <div class="min-w-0 space-y-1">
                  <dt class="text-xs font-medium text-muted-foreground">
                    {copy.intakeDetectedType}
                  </dt>
                  <dd class="truncate" title={intakePreview.preview.data.content_type}>
                    {intakePreview.preview.data.content_type}
                  </dd>
                </div>
                {#if intakePreview.preview.data.requested_locator}
                  <div class="min-w-0 space-y-1 sm:col-span-2">
                    <dt class="text-xs font-medium text-muted-foreground">
                      {copy.intakeSourceIdentity}
                    </dt>
                    <dd
                      class="truncate font-mono text-xs"
                      title={intakePreview.preview.data.final_locator ??
                        intakePreview.preview.data.requested_locator}
                    >
                      {intakePreview.preview.data.final_locator ??
                        intakePreview.preview.data.requested_locator}
                    </dd>
                  </div>
                {/if}
                <div class="min-w-0 space-y-1 sm:col-span-2">
                  <dt class="text-xs font-medium text-muted-foreground">SHA-256</dt>
                  <dd
                    class="truncate font-mono text-xs"
                    title={intakePreview.preview.data.preview_sha256}
                  >
                    {intakePreview.preview.data.preview_sha256}
                  </dd>
                </div>
              </dl>

              <Alert.Root>
                <ShieldCheck size={16} strokeWidth={1.8} aria-hidden="true" />
                <Alert.Description>
                  {copy.reviewBeforeCommit} · {copy.submissionBoundary}
                </Alert.Description>
              </Alert.Root>

              <div class="flex flex-col gap-2 sm:flex-row">
                <Button disabled={busy} onclick={commitIntakePreview}>
                  {busy ? copy.working : copy.commitPreview}
                </Button>
                <Button variant="outline" disabled={busy} onclick={discardIntakePreview}>
                  {copy.discardPreview}
                </Button>
              </div>
            </div>
          {:else}
            <form
              class="space-y-4"
              onsubmit={(event) => {
                event.preventDefault();
                submitIntakePreview();
              }}
            >
              <div class="space-y-2">
                <Label for="generic-title">{copy.genericApplicationTitle}</Label>
                <Input id="generic-title" bind:value={title} required />
              </div>
              {#each presentation?.opportunity_fields ?? [] as field (field.id)}
                <div class="space-y-2">
                  <Label for={`generic-opportunity-${field.id}`}>{field.label.value}</Label>
                  <Input
                    id={`generic-opportunity-${field.id}`}
                    type={fieldInputType(field)}
                    value={opportunityValues[field.id] ?? ""}
                    required={field.required}
                    oninput={(event) => (opportunityValues[field.id] = event.currentTarget.value)}
                  />
                </div>
              {/each}
              {#each presentation?.application_fields ?? [] as field (field.id)}
                <div class="space-y-2">
                  <Label for={`generic-application-${field.id}`}>{field.label.value}</Label>
                  {#if field.field_type === "choice"}
                    <NativeSelect.Root
                      id={`generic-application-${field.id}`}
                      value={applicationValues[field.id] ?? ""}
                      onchange={(event) =>
                        (applicationValues[field.id] = event.currentTarget.value)}
                    >
                      <NativeSelect.Option value="">—</NativeSelect.Option>
                      {#each field.options as option (option.id)}
                        <NativeSelect.Option value={option.id}
                          >{option.label.value}</NativeSelect.Option
                        >
                      {/each}
                    </NativeSelect.Root>
                  {:else if field.field_type === "long-text"}
                    <Textarea
                      id={`generic-application-${field.id}`}
                      value={applicationValues[field.id] ?? ""}
                      oninput={(event) => (applicationValues[field.id] = event.currentTarget.value)}
                    />
                  {:else}
                    <Input
                      id={`generic-application-${field.id}`}
                      type={fieldInputType(field)}
                      value={applicationValues[field.id] ?? ""}
                      oninput={(event) => (applicationValues[field.id] = event.currentTarget.value)}
                    />
                  {/if}
                </div>
              {/each}
              <Tabs.Root bind:value={intakeMode}>
                <Tabs.List class="responsive-tabs" data-columns="3" aria-label={copy.sourceIntake}>
                  <Tabs.Trigger value="pasted">{copy.sourceText}</Tabs.Trigger>
                  <Tabs.Trigger value="local">
                    <FileUp size={16} strokeWidth={1.8} aria-hidden="true" />
                    {copy.localFile}
                  </Tabs.Trigger>
                  <Tabs.Trigger value="url">
                    <Link size={16} strokeWidth={1.8} aria-hidden="true" />
                    {copy.sourceUrl}
                  </Tabs.Trigger>
                </Tabs.List>
                <Tabs.Content value="pasted" class="space-y-2 pt-4">
                  <Label for="generic-source">{copy.sourceText}</Label>
                  <Textarea
                    id="generic-source"
                    bind:value={sourceText}
                    rows={8}
                    aria-describedby="generic-source-help"
                    required={intakeMode === "pasted"}
                  />
                  <p id="generic-source-help" class="text-xs text-muted-foreground">
                    {copy.applicationIntakeSourceHelp}
                  </p>
                </Tabs.Content>
                <Tabs.Content value="local" class="space-y-4 pt-4">
                  <div class="space-y-2">
                    <Label for="generic-local-source">{copy.sourceFile}</Label>
                    <div class="flex flex-col gap-2 sm:flex-row">
                      <Input id="generic-local-source" bind:value={localSource} readonly />
                      <Button type="button" variant="outline" onclick={chooseLocalSource}>
                        {copy.chooseFile}
                      </Button>
                    </div>
                  </div>
                  <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
                    <Checkbox
                      id="generic-private-read-consent"
                      bind:checked={privateReadConfirmed}
                      class="mt-0.5"
                    />
                    <Label for="generic-private-read-consent" class="text-xs leading-5 font-normal">
                      {copy.privateReadConsent}
                    </Label>
                  </div>
                </Tabs.Content>
                <Tabs.Content value="url" class="space-y-4 pt-4">
                  <div class="space-y-2">
                    <Label for="generic-source-url">{copy.sourceUrl}</Label>
                    <Input
                      id="generic-source-url"
                      type="url"
                      bind:value={sourceUrl}
                      placeholder={copy.sourceUrlPlaceholder}
                      autocomplete="url"
                    />
                  </div>
                  <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
                    <Checkbox
                      id="generic-network-fetch-consent"
                      bind:checked={networkFetchConfirmed}
                      class="mt-0.5"
                    />
                    <Label
                      for="generic-network-fetch-consent"
                      class="text-xs leading-5 font-normal"
                    >
                      {copy.networkFetchConsent}
                    </Label>
                  </div>
                </Tabs.Content>
              </Tabs.Root>
              <div class="grid gap-4 sm:grid-cols-2">
                <div class="space-y-2">
                  <Label for="generic-category">{copy.requirementCategory}</Label>
                  <NativeSelect.Root id="generic-category" bind:value={requirementCategory}>
                    {#each presentation?.requirement_categories ?? [] as category (category.id)}
                      <NativeSelect.Option value={category.id}
                        >{category.label.value}</NativeSelect.Option
                      >
                    {/each}
                  </NativeSelect.Root>
                </div>
                <div class="space-y-2">
                  <Label for="generic-priority">{copy.priority}</Label>
                  <NativeSelect.Root id="generic-priority" bind:value={requirementPriority}>
                    <NativeSelect.Option value="mandatory">{copy.mandatory}</NativeSelect.Option>
                    <NativeSelect.Option value="recommended">{copy.recommended}</NativeSelect.Option
                    >
                    <NativeSelect.Option value="informational"
                      >{copy.informational}</NativeSelect.Option
                    >
                  </NativeSelect.Root>
                </div>
              </div>
              <Button type="submit" disabled={!desktopRuntime || busy || !presentation}>
                <Plus size={17} strokeWidth={1.8} aria-hidden="true" />
                {busy ? copy.working : copy.reviewBeforeCommit}
              </Button>
            </form>
          {/if}
        </Card.Content>
      </Card.Root>
    </div>

    <div class="space-y-[var(--density-section-gap)]">
      {#if selected}
        <Card.Root>
          <Card.Header>
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div>
                <Card.Title>{selected.snapshot.opportunity.title}</Card.Title>
                <Card.Description>{selected.snapshot.application.id}</Card.Description>
              </div>
              <Badge variant="secondary">{copy.revision} {selectedRevision}</Badge>
            </div>
          </Card.Header>
          <Card.Content class="space-y-3">
            <Progress value={stageProgress} aria-label={`${completedStages}/${stages.length}`} />
            <div class="flex flex-wrap gap-2" aria-label={copy.applicationJourney}>
              {#each presentation?.stages ?? [] as stage (stage.id)}
                {@const state =
                  stages.find((item) => localId(item.id) === stage.id)?.state ?? "pending"}
                <Badge
                  variant={state === "complete"
                    ? "default"
                    : state === "ready"
                      ? "secondary"
                      : "outline"}
                >
                  {stage.label.value}
                </Badge>
              {/each}
            </div>
          </Card.Content>
        </Card.Root>

        <Card.Root>
          <Card.Header>
            <Card.Title>{copy.applicationEvidenceSelection}</Card.Title>
            <Card.Description>{copy.applicationEvidenceSelectionDescription}</Card.Description>
          </Card.Header>
          <Card.Content class="space-y-5">
            <fieldset class="space-y-3">
              <legend class="text-sm font-semibold">{copy.profileSources}</legend>
              {#if !profileAssociations}
                <p class="text-sm text-muted-foreground" aria-live="polite">{copy.loading}</p>
              {:else if !profileAssociations.profile_sources.length && !profileAssociations.associations.length}
                <Empty.Root class="min-h-24 border bg-muted/20">
                  <Empty.Header>
                    <Empty.Title>{copy.noProfileSources}</Empty.Title>
                    <Empty.Description>{copy.profileSourceHelp}</Empty.Description>
                  </Empty.Header>
                </Empty.Root>
              {:else}
                {#each profileAssociations.profile_sources as source (source.id)}
                  {@const association = linkedProfile(source.id)}
                  <div class="flex items-start gap-3 rounded-md border p-3">
                    <Checkbox
                      id={`application-profile-${source.id}`}
                      bind:checked={profileSelections[source.id]}
                      disabled={busy || pendingAssociationChanges.length > 0}
                      class="mt-0.5"
                    />
                    <Label
                      for={`application-profile-${source.id}`}
                      class="min-w-0 flex-1 font-normal"
                    >
                      <span class="flex flex-wrap items-center gap-2 font-medium">
                        {source.kind}
                        {#if association?.stale}
                          <Badge variant="destructive">{copy.staleAssociation}</Badge>
                        {:else if association}
                          <Badge variant="secondary">{copy.associated}</Badge>
                        {/if}
                      </span>
                      <span class="mt-1 block truncate font-mono text-xs text-muted-foreground">
                        {source.id} · {copy.revision}
                        {source.revision} · {source.sensitivity}
                      </span>
                    </Label>
                  </div>
                {/each}
                {#each profileAssociations.associations.filter((association) => !profileAssociations?.profile_sources.some((source) => source.id === association.profile_source.id)) as association (association.profile_source.id)}
                  <div class="flex items-start gap-3 rounded-md border border-destructive/40 p-3">
                    <Checkbox
                      id={`application-profile-orphan-${association.profile_source.id}`}
                      bind:checked={profileSelections[association.profile_source.id]}
                      disabled={busy || pendingAssociationChanges.length > 0}
                      class="mt-0.5"
                    />
                    <Label
                      for={`application-profile-orphan-${association.profile_source.id}`}
                      class="min-w-0 flex-1 font-normal"
                    >
                      <span class="flex flex-wrap items-center gap-2 font-medium">
                        {copy.staleAssociation}
                        <Badge variant="destructive">{copy.reviewRequired}</Badge>
                      </span>
                      <span class="mt-1 block truncate font-mono text-xs text-muted-foreground">
                        {association.profile_source.id} · {copy.revision}
                        {association.profile_source.revision}
                      </span>
                    </Label>
                  </div>
                {/each}
              {/if}
            </fieldset>

            <fieldset class="space-y-3">
              <legend class="text-sm font-semibold">
                {presentation?.vocabulary.evidence_plural ?? copy.evidence}
              </legend>
              {#if !evidenceAssociations}
                <p class="text-sm text-muted-foreground" aria-live="polite">{copy.loading}</p>
              {:else if !evidenceAssociations.evidence.length && !evidenceAssociations.associations.length}
                <Empty.Root class="min-h-24 border bg-muted/20">
                  <Empty.Header>
                    <Empty.Title>{copy.noConfirmedEvidence}</Empty.Title>
                    <Empty.Description>{copy.confirmedEvidenceHelp}</Empty.Description>
                  </Empty.Header>
                </Empty.Root>
              {:else}
                {#each evidenceAssociations.evidence as item (item.evidence.id)}
                  {@const association = linkedEvidence(item.evidence.id)}
                  <div class="flex items-start gap-3 rounded-md border p-3">
                    <Checkbox
                      id={`application-evidence-${item.evidence.id}`}
                      bind:checked={evidenceSelections[item.evidence.id]}
                      disabled={busy || pendingAssociationChanges.length > 0}
                      class="mt-0.5"
                    />
                    <Label
                      for={`application-evidence-${item.evidence.id}`}
                      class="min-w-0 flex-1 font-normal"
                    >
                      <span class="flex flex-wrap items-center gap-2 font-medium">
                        {item.kind}
                        {#if association?.stale}
                          <Badge variant="destructive">{copy.staleAssociation}</Badge>
                        {:else if association}
                          <Badge variant="secondary">{copy.associated}</Badge>
                        {/if}
                      </span>
                      <span class="mt-1 block truncate font-mono text-xs text-muted-foreground">
                        {item.evidence.id} · {copy.revision}
                        {item.evidence.revision} ·
                        {item.sensitivity}
                      </span>
                    </Label>
                  </div>
                {/each}
                {#each evidenceAssociations.associations.filter((association) => !evidenceAssociations?.evidence.some((item) => item.evidence.id === association.evidence.id)) as association (association.evidence.id)}
                  <div class="flex items-start gap-3 rounded-md border border-destructive/40 p-3">
                    <Checkbox
                      id={`application-evidence-orphan-${association.evidence.id}`}
                      bind:checked={evidenceSelections[association.evidence.id]}
                      disabled={busy || pendingAssociationChanges.length > 0}
                      class="mt-0.5"
                    />
                    <Label
                      for={`application-evidence-orphan-${association.evidence.id}`}
                      class="min-w-0 flex-1 font-normal"
                    >
                      <span class="flex flex-wrap items-center gap-2 font-medium">
                        {copy.staleAssociation}
                        <Badge variant="destructive">{copy.reviewRequired}</Badge>
                      </span>
                      <span class="mt-1 block truncate font-mono text-xs text-muted-foreground">
                        {association.evidence.id} · {copy.revision}
                        {association.evidence.revision}
                      </span>
                    </Label>
                  </div>
                {/each}
              {/if}
            </fieldset>

            {#if pendingAssociationChanges.length}
              <Alert.Root aria-live="polite" aria-atomic="true">
                <ShieldCheck size={17} strokeWidth={1.8} aria-hidden="true" />
                <Alert.Description>
                  <span class="block font-medium">
                    {copy.associationPreviewCount}: {pendingAssociationChanges.length}
                  </span>
                  <ul class="mt-2 list-disc space-y-1 pl-5 text-xs">
                    {#each pendingAssociationChanges as item (`${item.resource}-${item.request.change}-${item.label}`)}
                      <li>{item.request.change} · {item.label}</li>
                    {/each}
                  </ul>
                </Alert.Description>
              </Alert.Root>
              {#if pendingAssociationNeedsPrivateRead}
                <div class="flex items-start gap-3 rounded-lg border bg-muted/20 p-3">
                  <Checkbox
                    id="application-association-private-consent"
                    bind:checked={associationPrivateConsent}
                    class="mt-0.5"
                  />
                  <Label
                    for="application-association-private-consent"
                    class="text-xs leading-5 font-normal"
                  >
                    {copy.associationPrivateConsent}
                  </Label>
                </div>
              {/if}
              <div class="flex flex-col gap-2 sm:flex-row">
                <Button
                  disabled={busy ||
                    (pendingAssociationNeedsPrivateRead && !associationPrivateConsent)}
                  onclick={commitAssociationChanges}
                >
                  {copy.commitAssociationChanges}
                </Button>
                <Button
                  variant="outline"
                  disabled={busy}
                  onclick={() => loadAssociationContext(selected!.snapshot.application.id)}
                >
                  {copy.discardPreview}
                </Button>
              </div>
            {:else}
              <Button
                variant="outline"
                disabled={busy || !profileAssociations || !evidenceAssociations}
                onclick={previewAssociationChanges}
              >
                {copy.previewAssociationChanges}
              </Button>
            {/if}
          </Card.Content>
        </Card.Root>

        {#if pendingLifecycleMutation}
          <Alert.Root aria-live="polite" aria-atomic="true">
            <ShieldCheck size={17} strokeWidth={1.8} aria-hidden="true" />
            <Alert.Title>{copy.reviewedLifecycleChange}</Alert.Title>
            <Alert.Description>
              <p>{pendingLifecycleMutation.summary}</p>
              <ul class="mt-2 list-disc space-y-1 pl-5 text-xs">
                {#each pendingLifecycleMutation.changes as change, index (`${index}-${change}`)}
                  <li>{change}</li>
                {/each}
              </ul>
              <div class="mt-4 flex flex-col gap-2 sm:flex-row">
                <Button disabled={busy} onclick={commitPendingLifecycleMutation}>
                  {copy.commitReviewedLifecycleChange}
                </Button>
                <Button variant="outline" disabled={busy} onclick={discardPendingLifecycleMutation}>
                  {copy.discardPreview}
                </Button>
              </div>
            </Alert.Description>
          </Alert.Root>
        {/if}

        {#if selected.snapshot.requirements.some((item) => item.confirmation === "proposed")}
          <Card.Root>
            <Card.Header>
              <Card.Title>{copy.reviewRequirements}</Card.Title>
              <Card.Description>{copy.reviewRequirementsDescription}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              <fieldset class="space-y-3">
                <legend class="sr-only">{copy.reviewRequirements}</legend>
                {#each selected.snapshot.requirements as requirement (requirement.id)}
                  <div class="grid gap-3 rounded-md border p-3 sm:grid-cols-[1fr_10rem]">
                    <div class="min-w-0">
                      <p class="text-sm font-medium">{requirement.statement}</p>
                      <p class="mt-1 text-xs text-muted-foreground">
                        {localId(requirement.category)} · {requirement.priority}
                      </p>
                    </div>
                    <div class="space-y-2">
                      <Label for={`requirement-decision-${requirement.id}`}
                        >{copy.requirementDecision}</Label
                      >
                      <NativeSelect.Root
                        id={`requirement-decision-${requirement.id}`}
                        bind:value={requirementDecisions[requirement.id]}
                        disabled={busy || Boolean(pendingLifecycleMutation)}
                      >
                        <NativeSelect.Option value="confirm">{copy.confirm}</NativeSelect.Option>
                        <NativeSelect.Option value="exclude"
                          >{copy.excludeRequirement}</NativeSelect.Option
                        >
                      </NativeSelect.Root>
                    </div>
                  </div>
                {/each}
              </fieldset>
              <Button
                disabled={busy || Boolean(pendingLifecycleMutation)}
                onclick={previewRequirementDecisions}
              >
                {copy.reviewBeforeCommit}
              </Button>
            </Card.Content>
          </Card.Root>
        {:else if !selected.snapshot.plan}
          <Card.Root>
            <Card.Header>
              <Card.Title>{copy.confirmApplicationPlan}</Card.Title>
              <Card.Description>{presentation?.vocabulary.requirement_plural}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              <ul class="space-y-2 text-sm">
                {#each selected.snapshot.requirements as requirement (requirement.id)}
                  <li class="rounded-md border p-3">{requirement.statement}</li>
                {/each}
              </ul>
              <fieldset class="space-y-3">
                <legend class="text-sm font-medium"
                  >{presentation?.vocabulary.deliverable_plural}</legend
                >
                {#each presentation?.deliverables ?? [] as deliverable (deliverable.id)}
                  <div class="flex items-start gap-3 rounded-md border p-3">
                    <Checkbox
                      id={`plan-${deliverable.id}`}
                      bind:checked={deliverableSelections[deliverable.id]}
                      disabled={deliverable.minimum > 0}
                    />
                    <Label for={`plan-${deliverable.id}`} class="min-w-0 font-normal">
                      <span class="block font-medium">{deliverable.label.value}</span>
                      <span class="block text-xs text-muted-foreground">
                        {deliverable.minimum}–{deliverable.maximum}
                      </span>
                    </Label>
                  </div>
                {/each}
              </fieldset>
              <Button
                disabled={busy || Boolean(pendingLifecycleMutation)}
                onclick={previewPlanProposal}>{copy.reviewBeforeCommit}</Button
              >
            </Card.Content>
          </Card.Root>
        {:else if selected.snapshot.plan.state === "draft"}
          <Card.Root>
            <Card.Header>
              <Card.Title>{copy.confirmApplicationPlan}</Card.Title>
              <Card.Description>{copy.reviewPlanDescription}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              <ul class="space-y-2 text-sm">
                {#each selected.snapshot.plan.deliverables as deliverable (deliverable.kind)}
                  <li class="rounded-md border p-3">
                    <span class="font-medium">{deliverableLabel(deliverable.kind)}</span>
                    <span class="ml-2 text-xs text-muted-foreground">{deliverable.disposition}</span
                    >
                  </li>
                {/each}
              </ul>
              <Button
                disabled={busy || Boolean(pendingLifecycleMutation)}
                onclick={previewPlanConfirmation}>{copy.reviewBeforeCommit}</Button
              >
            </Card.Content>
          </Card.Root>
        {:else if selected.snapshot.plan.state === "stale"}
          <Alert.Root variant="destructive">
            <Alert.Title>{copy.reviewRequired}</Alert.Title>
            <Alert.Description>{copy.stalePlanDescription}</Alert.Description>
          </Alert.Root>
        {:else if !selected.snapshot.deliverables.length}
          <Card.Root>
            <Card.Header><Card.Title>{copy.composeDeliverables}</Card.Title></Card.Header>
            <Card.Content>
              <form
                class="space-y-5"
                onsubmit={(event) => {
                  event.preventDefault();
                  previewCompose();
                }}
              >
                {#each plannedDeliverables as planned (planned.kind)}
                  {@const id = localId(planned.kind)}
                  <fieldset class="space-y-3 rounded-md border p-4">
                    <legend class="px-1 text-sm font-semibold">{deliverableLabel(id)}</legend>
                    <div class="space-y-2">
                      <Label for={`deliverable-title-${id}`}>{copy.genericApplicationTitle}</Label>
                      <Input
                        id={`deliverable-title-${id}`}
                        bind:value={deliverableDrafts[id].title}
                        required
                      />
                    </div>
                    <div class="space-y-2">
                      <Label for={`deliverable-content-${id}`}>{copy.deliverableContent}</Label>
                      <Textarea
                        id={`deliverable-content-${id}`}
                        bind:value={deliverableDrafts[id].content}
                        rows={10}
                        required
                      />
                    </div>
                  </fieldset>
                {/each}
                <Button type="submit" disabled={busy || Boolean(pendingLifecycleMutation)}
                  >{copy.reviewBeforeCommit}</Button
                >
              </form>
            </Card.Content>
          </Card.Root>
        {:else if selected.snapshot.deliverables.every((item) => item.state === "review-required")}
          <Card.Root>
            <Card.Header>
              <Card.Title>{copy.reviewDeliverables}</Card.Title>
              <Card.Description>{copy.privateReadConsent}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              <div class="flex items-start gap-3">
                <Checkbox id="generic-review-consent" bind:checked={privateReviewConsent} />
                <Label for="generic-review-consent" class="font-normal"
                  >{copy.reviewConsentLabel}</Label
                >
              </div>
              <Button
                variant="outline"
                disabled={busy || !privateReviewConsent}
                onclick={loadReview}
              >
                {copy.loadPrivateReview}
              </Button>
              {#if review}
                {#each review.deliverables as item (item.deliverable.id)}
                  <article class="space-y-2 rounded-md border p-4">
                    <h3 class="font-semibold">{item.deliverable.title}</h3>
                    <pre
                      class="max-h-80 overflow-auto whitespace-pre-wrap break-words rounded-md bg-muted p-3 text-sm">{item.content}</pre>
                  </article>
                {/each}
                <div class="flex items-start gap-3">
                  <Checkbox id="generic-review-complete" bind:checked={reviewConfirmed} />
                  <Label for="generic-review-complete" class="font-normal"
                    >{copy.reviewedAllDeliverables}</Label
                  >
                </div>
                <Button disabled={busy || !reviewConfirmed} onclick={submitApproval}>
                  {copy.approveDeliverables}
                </Button>
              {/if}
            </Card.Content>
          </Card.Root>
        {:else if selected.snapshot.deliverables.every((item) => item.state === "approved")}
          <Card.Root>
            <Card.Header>
              <Card.Title>{copy.exportApplication}</Card.Title>
              <Card.Description>{copy.submissionBoundary}</Card.Description>
            </Card.Header>
            <Card.Content class="space-y-4">
              <div class="space-y-2">
                <Label for="generic-export-destination">{copy.exportDestination}</Label>
                <Input id="generic-export-destination" bind:value={exportDestination} />
              </div>
              <div class="flex items-start gap-3">
                <Checkbox id="generic-export-consent" bind:checked={privateExportConsent} />
                <Label for="generic-export-consent" class="font-normal"
                  >{copy.privateExportConsent}</Label
                >
              </div>
              <Button
                disabled={busy || !privateExportConsent || !exportDestination.trim()}
                onclick={submitExport}
              >
                {copy.exportApplication}
              </Button>
            </Card.Content>
          </Card.Root>
        {/if}
      {:else}
        <Card.Root>
          <Card.Content>
            <Empty.Root class="min-h-48">
              <Empty.Header>
                <Empty.Media variant="icon"><FileCheck2 size={22} aria-hidden="true" /></Empty.Media
                >
                <Empty.Title>{copy.noGenericApplications}</Empty.Title>
              </Empty.Header>
            </Empty.Root>
          </Card.Content>
        </Card.Root>
      {/if}
    </div>
  </Page.Grid>
</Page.Root>
