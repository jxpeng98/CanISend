// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import DeliveryView from "./views/DeliveryView.svelte";
import { messages } from "./i18n";
import type {
  ArtifactReference,
  RenderManifestRecord,
  WorkflowPackPresentationReadModel,
  WorkspaceReadModel,
} from "./bridge";

const JOB_ID = "019f2f55-7c00-7000-8000-000000000101";
const PDF_SHA = "d".repeat(64);

function artifact(kind: string, sha256: string): ArtifactReference {
  return {
    id: `019f2f55-7c00-7000-8000-${sha256[0]?.repeat(12)}`,
    kind,
    revision: 1,
    sha256,
  };
}

const manifest: RenderManifestRecord = {
  id: "019f2f55-7c00-7000-8000-000000000800",
  job_id: JOB_ID,
  documents: [
    {
      kind: "cv",
      document_artifact: artifact("cv", "b".repeat(64)),
      typst_artifact: artifact("typst-source", "c".repeat(64)),
      pdf_artifact: artifact("pdf", PDF_SHA),
      page_count: 1,
      byte_count: 16,
      warning_count: 0,
      elapsed_millis: 25,
    },
  ],
  rendered_at: "2026-08-01T12:00:00Z",
  submission_performed: false,
  revision: 1,
};

const workspace = {
  path: "/tmp/canisend-preview-workspace",
  status: {},
} as WorkspaceReadModel;

const presentation = {
  deliverables: [
    {
      id: "cv",
      label: { value: "Academic CV", locale: "en", used_default_fallback: false },
    },
  ],
} as WorkflowPackPresentationReadModel;

beforeEach(() => {
  Object.defineProperty(URL, "createObjectURL", {
    configurable: true,
    value: vi.fn(() => "blob:canisend-render-preview"),
  });
  Object.defineProperty(URL, "revokeObjectURL", {
    configurable: true,
    value: vi.fn(),
  });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("delivery final PDF preview", () => {
  it("previews the current validated PDF only after private-read consent", async () => {
    const user = userEvent.setup();
    const onPreviewRender = vi.fn(async () => new Uint8Array([37, 80, 68, 70]));
    const onOpenRender = vi.fn(async () => true);

    render(DeliveryView, {
      copy: messages.en,
      desktopRuntime: true,
      activeWorkspace: workspace,
      selectedJobId: JOB_ID,
      presentation,
      focus: "delivery-render",
      busy: false,
      onNavigate: vi.fn(async () => undefined),
      onLoadDocuments: vi.fn(async () => null),
      onLoadReview: vi.fn(async () => null),
      onConfirmReview: vi.fn(async () => null),
      onCheckPackage: vi.fn(async () => null),
      onLoadPackage: vi.fn(async () => null),
      onExportPackage: vi.fn(async () => null),
      onLoadPackageExport: vi.fn(async () => null),
      onReconcilePackage: vi.fn(async () => null),
      onReplaceProjection: vi.fn(async () => null),
      onCopyProjection: vi.fn(async () => null),
      onBuildRender: vi.fn(async () => manifest),
      onLoadRender: vi.fn(async () => manifest),
      onPreviewRender,
      onExportRender: vi.fn(async () => true),
      onOpenRender,
    });

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: messages.en.buildRender }),
      ).toBeTruthy(),
    );
    await user.click(
      screen.getByLabelText(messages.en.privateWorkspaceConsent),
    );
    await user.click(
      screen.getByRole("button", { name: messages.en.buildRender }),
    );

    const previewButton = await screen.findByRole("button", {
      name: /CV.*Preview PDF/i,
    });
    await user.click(previewButton);

    expect(onPreviewRender).toHaveBeenCalledWith(JOB_ID, "cv", true);
    expect(URL.createObjectURL).toHaveBeenCalledOnce();
    expect(
      (
        await screen.findByTitle(`${messages.en.exactPdfPreview}: Academic CV`)
      ).getAttribute("src"),
    ).toBe("blob:canisend-render-preview");
    expect(screen.getByText(PDF_SHA)).toBeTruthy();

    await user.click(screen.getByLabelText(messages.en.privateExportConsent));
    await user.click(
      screen.getByRole("button", { name: messages.en.openSystemViewer }),
    );
    expect(onOpenRender).toHaveBeenCalledWith(
      JOB_ID,
      `jobs/${JOB_ID}/rendered`,
      "cv",
      true,
    );
  });
});
