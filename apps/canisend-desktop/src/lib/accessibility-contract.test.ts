import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const app = readFileSync(new URL("../App.svelte", import.meta.url), "utf8");
const contextBar = readFileSync(
  new URL("./components/WorkspaceContextBar.svelte", import.meta.url),
  "utf8",
);
const agentView = readFileSync(
  new URL("./views/AgentView.svelte", import.meta.url),
  "utf8",
);
const primarySurfaces = [app, contextBar, agentView].join("\n");

describe("desktop accessibility contract", () => {
  it("keeps keyboard landmarks, current state, and explicit focus treatment", () => {
    expect(app).toContain('href="#main-content"');
    expect(app).toContain('id="main-content"');
    expect(app).toContain("focus-visible:ring-2");
    expect(contextBar).toContain('aria-current=');
    expect(contextBar).toContain('role="progressbar"');
    expect(contextBar).toContain("focus-visible:ring-2");
  });

  it("announces asynchronous, recovery, and conversation changes", () => {
    expect(app).toContain('role="alert"');
    expect(app).toContain('role="status"');
    expect(agentView).toContain('role="status"');
    expect(agentView).toContain('role="alert"');
    expect(agentView).toContain('aria-live="polite"');
  });

  it("uses semantic controls with bounded motion and 44-pixel critical targets", () => {
    expect(primarySurfaces).toContain("<button");
    expect(primarySurfaces).toContain("<select");
    expect(agentView).toContain("<details");
    expect(primarySurfaces).toContain("motion-reduce:");
    expect(contextBar).toContain("min-h-11");
    expect(agentView).toContain("min-h-11");

    const nonSemanticHandlers =
      /<(?:div|span|p|section|article)\b[^>]*\b(?:onclick|onkeydown)=/giu;
    expect(primarySurfaces.match(nonSemanticHandlers) ?? []).toEqual([]);
  });
});
