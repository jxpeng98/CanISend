// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";

import UiInteractionHarness from "./test-fixtures/UiInteractionHarness.svelte";

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

describe("shadcn-svelte interaction contract", () => {
  it("exposes optional guidance through a keyboard-accessible help trigger", () => {
    render(UiInteractionHarness);

    const trigger = screen.getByRole("button", { name: "Workflow help" });
    expect(trigger.hasAttribute("data-context-help")).toBe(true);
    expect(trigger.getAttribute("aria-label")).toBe("Workflow help");
  });

  it("propagates native select values and supports keyboard tab navigation", async () => {
    const user = userEvent.setup();
    render(UiInteractionHarness);

    await user.selectOptions(screen.getByLabelText("Status"), "blocked");
    expect(screen.getByLabelText("Selected status").textContent).toContain("blocked");

    const overview = screen.getByRole("tab", { name: "Overview" });
    overview.focus();
    await user.keyboard("{ArrowRight}");

    expect(screen.getByRole("tab", { name: "Review" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.getByText("Review panel")).toBeTruthy();
  });

  it("opens and closes disclosures through their semantic trigger", async () => {
    const user = userEvent.setup();
    render(UiInteractionHarness);

    const trigger = screen.getByRole("button", { name: "Revision provenance" });
    expect(trigger.getAttribute("aria-expanded")).toBe("false");

    await user.click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByText("Artifact r7 was verified locally.")).toBeTruthy();

    await user.click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
  });

  it("exposes secondary actions through a semantic shared menu trigger", () => {
    render(UiInteractionHarness);

    const trigger = screen.getByRole("button", { name: "Harness actions" });
    expect(trigger.getAttribute("aria-haspopup")).toBe("menu");
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(trigger.getAttribute("data-slot")).toBe("dropdown-menu-trigger");
  });

  it("traps destructive confirmation and returns focus after cancel or confirm", async () => {
    const user = userEvent.setup();
    render(UiInteractionHarness);

    const opener = screen.getByRole("button", { name: "Remove managed files" });
    await user.click(opener);
    expect(screen.getByRole("alertdialog")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(document.activeElement).toBe(opener));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());

    cleanup();
    document.body.style.pointerEvents = "";
    render(UiInteractionHarness);
    const confirmUser = userEvent.setup();
    const confirmOpener = screen.getByRole("button", { name: "Remove managed files" });
    await confirmUser.click(confirmOpener);
    await confirmUser.click(screen.getByRole("button", { name: "Confirm removal" }));
    expect(screen.getByLabelText("Confirm count").textContent).toContain("1");
    await waitFor(() => expect(document.activeElement).toBe(confirmOpener));
  });
});
