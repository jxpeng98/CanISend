import { describe, expect, it } from "vitest";

import { messages } from "./i18n";

function keyPaths(value: unknown, prefix = ""): string[] {
  if (!value || typeof value !== "object" || Array.isArray(value)) return [];
  return Object.entries(value as Record<string, unknown>).flatMap(([key, child]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    return child && typeof child === "object" && !Array.isArray(child)
      ? [path, ...keyPaths(child, path)]
      : [path];
  });
}

describe("desktop translations", () => {
  it("keeps English and Simplified Chinese keys in parity", () => {
    expect(keyPaths(messages.en).sort()).toEqual(keyPaths(messages["zh-CN"]).sort());
  });

  it("keeps visible navigation labels non-empty", () => {
    for (const locale of Object.values(messages)) {
      expect(locale.today).not.toHaveLength(0);
      expect(locale.opportunities).not.toHaveLength(0);
      expect(locale.applications).not.toHaveLength(0);
      expect(locale.workspaces).not.toHaveLength(0);
    }
  });

  it("translates critical loading, recovery, empty, and guidance states", () => {
    for (const locale of Object.values(messages)) {
      const criticalStates = [
        locale.skipToContent,
        locale.loading,
        locale.viewLoadFailed,
        locale.retry,
        locale.noApplications,
        locale.noApplicationsDescription,
        locale.contentNoResults,
        locale.contentNoResultsDescription,
        locale.loadingGuidance,
        locale.guidanceUnavailable,
        locale.noConversation,
        locale.noConversationDescription,
      ];
      for (const value of criticalStates) {
        expect(value.trim().length).toBeGreaterThan(0);
      }
    }
    expect(
      [
        messages["zh-CN"].loading,
        messages["zh-CN"].viewLoadFailed,
        messages["zh-CN"].noApplications,
        messages["zh-CN"].guidanceUnavailable,
      ].every((value) => /\p{Script=Han}/u.test(value)),
    ).toBe(true);
  });
});
