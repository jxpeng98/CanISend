import type {
  DocumentKind,
  WorkflowPackPresentationReadModel,
} from "./bridge";

export interface PackTaskOperationOption {
  id: string;
  label: string;
}

export function workflowStageLabel(
  presentation: WorkflowPackPresentationReadModel | null,
  stage: string,
): string {
  return (
    presentation?.stages.find((candidate) => candidate.id === stage)?.label.value ??
    stage
  );
}

export function deliverablePresentationLabel(
  presentation: WorkflowPackPresentationReadModel | null,
  kind: DocumentKind,
): string {
  return (
    presentation?.deliverables.find((deliverable) => deliverable.id === kind)?.label
      .value ?? kind
  );
}

export function packTaskOperationOptions(
  presentation: WorkflowPackPresentationReadModel | null,
): PackTaskOperationOption[] {
  return [
    { id: "job-parse", label: "job-parse" },
    { id: "evidence-normalize", label: "evidence-normalize" },
    { id: "evidence-match", label: "evidence-match" },
    ...(presentation?.deliverables.flatMap((deliverable) =>
      deliverable.legacy_task_operation
        ? [
            {
              id: deliverable.legacy_task_operation,
              label: `${deliverable.label.value} · ${deliverable.legacy_task_operation}`,
            },
          ]
        : [],
    ) ?? []),
    { id: "document-review", label: "document-review" },
  ];
}
