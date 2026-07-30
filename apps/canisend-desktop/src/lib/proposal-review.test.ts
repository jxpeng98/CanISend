import { describe, expect, it } from "vitest";

import { buildJsonDiff, collectRevisionReferences } from "./proposal-review";

describe("proposal review helpers", () => {
  it("builds a bounded stable JSON-pointer diff", () => {
    const diff = buildJsonDiff(
      { decision: "hold", strategy: { risks: ["gap"] } },
      { decision: "apply", strategy: { risks: [] } },
      1,
    );

    expect(diff.totalChanges).toBe(2);
    expect(diff.changes).toEqual([
      { path: "/decision", before: '"hold"', after: '"apply"' },
    ]);
    expect(diff.truncated).toBe(true);
    expect(diff.comparisonLimited).toBe(false);
  });

  it("collects only explicit artifact-shaped revision references", () => {
    const references = collectRevisionReferences({
      matches_artifact: {
        kind: "evidence-matches",
        id: "018f2498-7b2a-7f62-8a5c-5e1e7dfb4e11",
        revision: 3,
        sha256: "a".repeat(64),
      },
      unrelated: { id: "not-an-artifact", revision: 1 },
    });

    expect(references).toHaveLength(1);
    expect(references[0]).toMatchObject({
      path: "/matches_artifact",
      kind: "evidence-matches",
      revision: 3,
    });
  });
});
