import { describe, expect, it } from "vitest";

import { commandErrorMessage, commandErrorRetryable } from "./bridge";

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

describe("commandErrorRetryable", () => {
  it("only enables retry for an explicit structured signal", () => {
    expect(commandErrorRetryable({ retryable: true })).toBe(true);
    expect(commandErrorRetryable({ retryable: false })).toBe(false);
    expect(commandErrorRetryable("offline")).toBe(false);
  });
});
