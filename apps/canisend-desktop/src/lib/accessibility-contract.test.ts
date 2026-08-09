import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const app = readFileSync(new URL("../App.svelte", import.meta.url), "utf8");
const bridge = readFileSync(new URL("./bridge.ts", import.meta.url), "utf8");
const contextBar = readFileSync(
  new URL("./components/WorkspaceContextBar.svelte", import.meta.url),
  "utf8",
);
const agentView = readFileSync(new URL("./views/AgentView.svelte", import.meta.url), "utf8");
const genericApplicationsView = readFileSync(
  new URL("./views/GenericApplicationsView.svelte", import.meta.url),
  "utf8",
);
const workspacesView = readFileSync(
  new URL("./views/WorkspacesView.svelte", import.meta.url),
  "utf8",
);
const loadingPanel = readFileSync(
  new URL("./components/patterns/LoadingPanel.svelte", import.meta.url),
  "utf8",
);
const pageHeader = readFileSync(
  new URL("./components/patterns/page/page-header.svelte", import.meta.url),
  "utf8",
);
const contextHelp = readFileSync(
  new URL("./components/patterns/ContextHelp.svelte", import.meta.url),
  "utf8",
);
const actionMenu = readFileSync(
  new URL("./components/patterns/ActionMenu.svelte", import.meta.url),
  "utf8",
);
const dropdownMenuItem = readFileSync(
  new URL("./components/ui/dropdown-menu/dropdown-menu-item.svelte", import.meta.url),
  "utf8",
);
const button = readFileSync(
  new URL("./components/ui/button/button.svelte", import.meta.url),
  "utf8",
);
const nativeSelect = readFileSync(
  new URL("./components/ui/native-select/native-select.svelte", import.meta.url),
  "utf8",
);
const tabsTrigger = readFileSync(
  new URL("./components/ui/tabs/tabs-trigger.svelte", import.meta.url),
  "utf8",
);
const alert = readFileSync(new URL("./components/ui/alert/alert.svelte", import.meta.url), "utf8");
const progress = readFileSync(
  new URL("./components/ui/progress/progress.svelte", import.meta.url),
  "utf8",
);
const accordion = readFileSync(
  new URL("./components/ui/accordion/accordion.svelte", import.meta.url),
  "utf8",
);
const dialogContent = readFileSync(
  new URL("./components/ui/dialog/dialog-content.svelte", import.meta.url),
  "utf8",
);
const primarySurfaces = [app, contextBar, agentView].join("\n");

describe("desktop accessibility contract", () => {
  it("keeps keyboard landmarks, current state, and explicit focus treatment", () => {
    expect(app).toContain('href="#main-content"');
    expect(app).toContain('id="main-content"');
    expect(app).toContain("focus-visible:ring-2");
    expect(contextBar).toContain("aria-current=");
    expect(contextBar).toContain("<Progress");
    expect(progress).toContain("ProgressPrimitive.Root");
    expect(button).toContain("focus-visible:ring");
    expect(nativeSelect).toContain("focus-visible:ring");
  });

  it("announces asynchronous, recovery, and conversation changes", () => {
    expect(app).toContain('<Alert.Root variant="destructive"');
    expect(alert).toContain('role="alert"');
    expect(app).toContain('role="status"');
    expect(agentView).toContain("<LoadingPanel");
    expect(loadingPanel).toContain('role="status"');
    expect(agentView).toContain('<Alert.Root variant="destructive"');
    expect(agentView).toContain('aria-live="polite"');
    expect(genericApplicationsView).toContain('role="alert"');
    expect(genericApplicationsView).toContain('aria-live="assertive"');
    expect(genericApplicationsView).toContain('aria-live="polite"');
  });

  it("keeps generic Pack forms labeled and consent actions keyboard-native", () => {
    expect(genericApplicationsView).toContain('<Label for="generic-title"');
    expect(genericApplicationsView).toContain('aria-describedby="generic-source-help"');
    expect(genericApplicationsView).toContain('<Checkbox id="generic-review-consent"');
    expect(genericApplicationsView).toContain('<Checkbox id="generic-export-consent"');
    expect(genericApplicationsView).toContain("<fieldset");
    expect(genericApplicationsView).toContain("<legend");
    expect(genericApplicationsView).toContain('id="application-association-private-consent"');
    expect(genericApplicationsView).not.toMatch(
      /<(?:div|span|p|section|article)\b[^>]*\b(?:onclick|onkeydown)=/giu,
    );
  });

  it("routes the connected Application lifecycle through reviewed v4 mutations", () => {
    for (const operation of [
      "previewRequirementConfirmationV4",
      "commitRequirementConfirmationV4",
      "previewPlanProposalV4",
      "commitPlanProposalV4",
      "previewPlanConfirmationV4",
      "commitPlanConfirmationV4",
      "previewDeliverableDraftV4",
      "commitDeliverableDraftV4",
      "auditDeliverablesV4",
    ]) {
      expect(genericApplicationsView).toContain(operation);
    }
    for (const directMutation of [
      "planGenericApplication",
      "composeGenericApplication",
      "reviewGenericApplication",
    ]) {
      expect(genericApplicationsView).not.toContain(directMutation);
    }
    expect(genericApplicationsView).toContain("requirement-decision-${requirement.id}");
    expect(genericApplicationsView).toContain("commitPendingLifecycleMutation");
    expect(genericApplicationsView).toContain("discardPendingLifecycleMutation");
  });

  it("keeps mixed Application selection keyboard-native and Workspace creation neutral", () => {
    expect(app).toContain('role="group"');
    expect(app).toContain("aria-label={copy.workflowPack}");
    expect(app).toContain("aria-pressed={activePackId === GENERIC_APPLICATION_WORKFLOW_PACK_ID}");
    expect(app).toContain("aria-pressed={activePackId === ACADEMIC_JOB_WORKFLOW_PACK_ID}");
    expect(app).toContain("onContextChange={handleV4ApplicationContext}");
    expect(app).toContain("onSelectV4Application={handleSelectV4Application}");
    expect(contextBar).toContain("usesV4ApplicationContext");
    expect(contextBar).toContain("selectedV4Application?.snapshot.application.id");
    expect(contextBar).toContain("application.snapshot.opportunity.title");
    expect(workspacesView).not.toContain('id="create-workspace-pack"');
    expect(workspacesView).toContain("{copy.workflowPackDescription}");
  });

  it("rejects retired read projections before invoking the desktop host", () => {
    for (const operation of [
      "agent.capabilities",
      "agent.context",
      "job.list",
      "job.show",
      "task.latest",
      "workflow.status",
      "workspace.v3.migration.preview",
      "workspace.v3.migration.commit",
    ]) {
      expect(bridge).toContain(`unsupportedLegacyDesktopOperation("${operation}"`);
    }
    for (const handler of [
      "agent_capabilities",
      "agent_context",
      "list_jobs",
      "show_job",
      "latest_task",
      "workflow_controls",
      "preview_workspace_v3_migration",
      "migrate_workspace_v3",
    ]) {
      expect(bridge).not.toContain(`invoke("${handler}"`);
    }
  });

  it("centralizes semantic controls and compact desktop target sizes", () => {
    expect(button).toContain("<button");
    expect(button).toContain('data-slot="button"');
    expect(button).toContain('"min-h-(--control-height) gap-1.5');
    expect(button).toContain("max-w-full min-w-0");
    expect(button).toContain("whitespace-normal");
    expect(nativeSelect).toContain("<select");
    expect(nativeSelect).toContain('data-slot="native-select"');
    expect(nativeSelect).toContain("data-[size=desktop]:h-(--control-height)");
    expect(tabsTrigger).toContain(
      "group-data-[variant=default]/tabs-list:data-[state=active]:bg-primary",
    );
    expect(tabsTrigger).toContain("motion-reduce:transition-none");
    expect(accordion).toContain('from "bits-ui"');
    expect(dialogContent).toContain('data-slot="dialog-content"');
    expect(primarySurfaces).toContain("motion-reduce:");

    const nonSemanticHandlers = /<(?:div|span|p|section|article)\b[^>]*\b(?:onclick|onkeydown)=/giu;
    expect(primarySurfaces.match(nonSemanticHandlers) ?? []).toEqual([]);
  });

  it("keeps optional page guidance concise and keyboard discoverable", () => {
    expect(pageHeader).toContain("<ContextHelp");
    expect(pageHeader).not.toContain('<p class="mt-2');
    expect(contextHelp).toContain('"data-context-help": ""');
    expect(contextHelp).toContain('"aria-label": label');
    expect(contextHelp).toContain("<Tooltip.Content");
  });

  it("keeps progressive disclosure on shared semantic menu controls", () => {
    expect(actionMenu).toContain("<DropdownMenu.Root>");
    expect(actionMenu).toContain("<DropdownMenu.Trigger>");
    expect(actionMenu).toContain("buttonVariants({");
    expect(actionMenu).toContain('"aria-label": label');
    expect(dropdownMenuItem).toContain("DropdownMenuPrimitive.Item");
    expect(dropdownMenuItem).toContain('data-slot="dropdown-menu-item"');
  });
});
