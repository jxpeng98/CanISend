import { describe, expect, it } from "vitest";

import { messages } from "./i18n";

describe("desktop translations", () => {
  it("keeps English and Simplified Chinese keys in parity", () => {
    expect(Object.keys(messages.en).sort()).toEqual(
      Object.keys(messages["zh-CN"]).sort(),
    );
  });

  it("keeps visible navigation labels non-empty", () => {
    for (const locale of Object.values(messages)) {
      expect(locale.today).not.toHaveLength(0);
      expect(locale.opportunities).not.toHaveLength(0);
      expect(locale.applications).not.toHaveLength(0);
      expect(locale.workspaces).not.toHaveLength(0);
    }
  });
});
