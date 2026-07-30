export interface JsonDiffEntry {
  path: string;
  before: string;
  after: string;
}

export interface JsonDiffSummary {
  changes: JsonDiffEntry[];
  totalChanges: number;
  truncated: boolean;
  comparisonLimited: boolean;
}

export interface RevisionReferenceSummary {
  path: string;
  kind: string;
  id: string;
  revision: number;
  sha256: string | null;
}

const MAX_COMPARISON_NODES = 10_000;
const MAX_SUMMARY_CHARS = 160;

export function buildJsonDiff(
  before: unknown,
  after: unknown,
  visibleLimit = 24,
): JsonDiffSummary {
  const changes: JsonDiffEntry[] = [];
  const stack: Array<{ before: unknown; after: unknown; path: string; depth: number }> = [
    { before, after, path: "/", depth: 0 },
  ];
  let totalChanges = 0;
  let visited = 0;

  while (stack.length > 0 && visited < MAX_COMPARISON_NODES) {
    const current = stack.pop();
    if (!current) break;
    visited += 1;
    if (Object.is(current.before, current.after)) continue;

    const beforeRecord = asRecord(current.before);
    const afterRecord = asRecord(current.after);
    if (current.depth < 32 && beforeRecord && afterRecord) {
      const keys = [...new Set([...Object.keys(beforeRecord), ...Object.keys(afterRecord)])]
        .sort()
        .reverse();
      for (const key of keys) {
        stack.push({
          before: beforeRecord[key],
          after: afterRecord[key],
          path: joinPointer(current.path, key),
          depth: current.depth + 1,
        });
      }
      continue;
    }

    const beforeArray = Array.isArray(current.before) ? current.before : null;
    const afterArray = Array.isArray(current.after) ? current.after : null;
    if (current.depth < 32 && beforeArray && afterArray) {
      const length = Math.max(beforeArray.length, afterArray.length);
      for (let index = length - 1; index >= 0; index -= 1) {
        stack.push({
          before: beforeArray[index],
          after: afterArray[index],
          path: joinPointer(current.path, String(index)),
          depth: current.depth + 1,
        });
      }
      continue;
    }

    totalChanges += 1;
    if (changes.length < Math.max(1, visibleLimit)) {
      changes.push({
        path: current.path,
        before: summarize(current.before),
        after: summarize(current.after),
      });
    }
  }

  return {
    changes,
    totalChanges,
    truncated: totalChanges > changes.length,
    comparisonLimited: stack.length > 0,
  };
}

export function collectRevisionReferences(
  value: unknown,
  limit = 12,
): RevisionReferenceSummary[] {
  const references: RevisionReferenceSummary[] = [];
  const stack: Array<{ value: unknown; path: string; depth: number }> = [
    { value, path: "/", depth: 0 },
  ];
  let visited = 0;

  while (
    stack.length > 0 &&
    references.length < Math.max(1, limit) &&
    visited < MAX_COMPARISON_NODES
  ) {
    const current = stack.pop();
    if (!current) break;
    visited += 1;
    const record = asRecord(current.value);
    if (record) {
      if (
        typeof record.id === "string" &&
        typeof record.kind === "string" &&
        typeof record.revision === "number" &&
        Number.isSafeInteger(record.revision) &&
        record.revision > 0
      ) {
        references.push({
          path: current.path,
          kind: record.kind,
          id: record.id,
          revision: record.revision,
          sha256: typeof record.sha256 === "string" ? record.sha256 : null,
        });
      }
      if (current.depth < 32) {
        for (const key of Object.keys(record).sort().reverse()) {
          stack.push({
            value: record[key],
            path: joinPointer(current.path, key),
            depth: current.depth + 1,
          });
        }
      }
      continue;
    }
    if (Array.isArray(current.value) && current.depth < 32) {
      for (let index = current.value.length - 1; index >= 0; index -= 1) {
        stack.push({
          value: current.value[index],
          path: joinPointer(current.path, String(index)),
          depth: current.depth + 1,
        });
      }
    }
  }

  return references;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function joinPointer(parent: string, segment: string): string {
  const escaped = segment.replaceAll("~", "~0").replaceAll("/", "~1");
  return parent === "/" ? `/${escaped}` : `${parent}/${escaped}`;
}

function summarize(value: unknown): string {
  if (value === undefined) return "∅";
  let serialized: string;
  try {
    serialized = JSON.stringify(value);
  } catch {
    serialized = String(value);
  }
  if (serialized.length <= MAX_SUMMARY_CHARS) return serialized;
  return `${serialized.slice(0, MAX_SUMMARY_CHARS - 1)}…`;
}
