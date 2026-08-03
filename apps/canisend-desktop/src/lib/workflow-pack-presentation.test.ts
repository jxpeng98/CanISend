import { describe, expect, it } from "vitest";

import type { WorkflowPackPresentationReadModel } from "./bridge";
import {
  deliverablePresentationLabel,
  packTaskOperationOptions,
  workflowStageLabel,
} from "./workflow-pack-presentation";

const presentation = {
  stages: [
    {
      id: "compose",
      label: { value: "撰写", locale: "zh-Hans", used_default_fallback: false },
    },
  ],
  deliverables: [
    {
      id: "portfolio",
      label: {
        value: "作品集",
        locale: "zh-Hans",
        used_default_fallback: false,
      },
      legacy_task_operation: "portfolio-draft",
    },
    {
      id: "reference-list",
      label: {
        value: "参考资料清单",
        locale: "zh-Hans",
        used_default_fallback: false,
      },
      legacy_task_operation: null,
    },
  ],
} as WorkflowPackPresentationReadModel;

describe("Pack-backed desktop presentation", () => {
  it("resolves custom stage and Deliverable labels with safe identifier fallback", () => {
    expect(workflowStageLabel(presentation, "compose")).toBe("撰写");
    expect(workflowStageLabel(presentation, "unknown-stage")).toBe("unknown-stage");
    expect(deliverablePresentationLabel(presentation, "portfolio")).toBe("作品集");
    expect(deliverablePresentationLabel(presentation, "unknown-output")).toBe("unknown-output");
  });

  it("builds draft operations from Pack metadata without a fixed Deliverable set", () => {
    const operations = packTaskOperationOptions(presentation);

    expect(operations).toContainEqual({
      id: "portfolio-draft",
      label: "作品集 · portfolio-draft",
    });
    expect(operations.some((operation) => operation.id === "reference-list-draft")).toBe(false);
  });
});
