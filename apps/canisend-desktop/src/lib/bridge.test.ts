import { describe, expect, it } from "vitest";

import { commandErrorMessage } from "./bridge";

describe("commandErrorMessage", () => {
  it("uses a structured desktop error message", () => {
    expect(
      commandErrorMessage({
        code: "fixture",
        message: "Action could not be completed",
      }),
    ).toBe("Action could not be completed");
  });

  it("preserves string rejections and bounds unknown values", () => {
    expect(commandErrorMessage("offline")).toBe("offline");
    expect(commandErrorMessage({ unexpected: true })).toBe(
      "The desktop command failed without a structured error.",
    );
  });
});
