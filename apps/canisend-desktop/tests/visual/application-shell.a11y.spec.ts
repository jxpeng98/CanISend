import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

type AppearancePreferences = {
  language: "en" | "zh-CN";
  darkMode: boolean;
  compact: boolean;
  reducedMotion: boolean;
  textScale: number;
};

async function openApplication(page: Page, preferences: AppearancePreferences): Promise<void> {
  await page.addInitScript((candidate) => {
    localStorage.setItem("canisend.desktop.appearance.v1", JSON.stringify(candidate));
  }, preferences);
  await page.goto("/");
  await expect(page.locator("#main-content")).toBeVisible();
  await expect(page.locator("#main-content h1")).toBeVisible();
}

async function expectNoAccessibilityViolations(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations).toEqual([]);
}

async function expectNoLayoutOverflow(page: Page): Promise<void> {
  const report = await page.evaluate(() => {
    const viewportWidth = document.documentElement.clientWidth;
    const offenders = Array.from(document.querySelectorAll("#main-content *"))
      .filter((element) => {
        if (!(element instanceof HTMLElement)) return false;
        const style = getComputedStyle(element);
        const bounds = element.getBoundingClientRect();
        if (
          style.display === "none" ||
          style.visibility === "hidden" ||
          bounds.width <= 0 ||
          bounds.height <= 0
        ) {
          return false;
        }

        const overflowAllowed = ["auto", "scroll", "hidden", "clip"].includes(style.overflowX);
        const hasVisibleText = (element.textContent ?? "").trim().length > 0;
        const exceedsOwnWidth = element.scrollWidth - element.clientWidth > 2;
        const exceedsViewport = bounds.right > viewportWidth + 1;
        const emptyCheckbox = element.getAttribute("role") === "checkbox" && !hasVisibleText;

        return (
          !emptyCheckbox &&
          !overflowAllowed &&
          (exceedsViewport || (exceedsOwnWidth && style.display !== "inline" && hasVisibleText))
        );
      })
      .slice(0, 20)
      .map((element) => ({
        tag: element.tagName.toLowerCase(),
        text: (element.textContent ?? "").trim().replace(/\s+/gu, " ").slice(0, 100),
        overflow: element.scrollWidth - element.clientWidth,
      }));

    const controlOffenders = Array.from(
      document.querySelectorAll<HTMLElement>(
        "#main-content button:not([data-slot='checkbox']):not([data-slot='switch']), #main-content a[data-slot='button'], #main-content [role='tab']",
      ),
    )
      .filter((element) => {
        const bounds = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        if (
          style.display === "none" ||
          style.visibility === "hidden" ||
          bounds.width <= 0 ||
          bounds.height <= 0
        ) {
          return false;
        }
        return (
          element.scrollWidth - element.clientWidth > 2 ||
          element.scrollHeight - element.clientHeight > 2
        );
      })
      .slice(0, 20)
      .map((element) => ({
        tag: element.tagName.toLowerCase(),
        text: (element.textContent ?? "").trim().replace(/\s+/gu, " ").slice(0, 100),
        horizontalOverflow: element.scrollWidth - element.clientWidth,
        verticalOverflow: element.scrollHeight - element.clientHeight,
      }));

    return {
      pageOverflow: document.documentElement.scrollWidth > viewportWidth + 1,
      offenders,
      controlOffenders,
    };
  });

  expect(report).toEqual({
    pageOverflow: false,
    offenders: [],
    controlOffenders: [],
  });
}

test("English light application shell meets automated accessibility rules", async ({ page }) => {
  await openApplication(page, {
    language: "en",
    darkMode: false,
    compact: false,
    reducedMotion: false,
    textScale: 100,
  });
  await expectNoAccessibilityViolations(page);
  await expectNoLayoutOverflow(page);

  const pageHelp = page.locator('[data-slot="page-header"] [data-context-help]');
  await expect(pageHelp).toHaveCount(1);
  await pageHelp.hover();
  await expect(page.locator('[data-slot="tooltip-content"]')).toContainText(
    "Track the evidence, decisions, documents, and next action",
  );
});

test("Chinese dark compact shell at 200 percent meets automated accessibility rules", async ({
  page,
}) => {
  await page.setViewportSize({ width: 960, height: 680 });
  await openApplication(page, {
    language: "zh-CN",
    darkMode: true,
    compact: true,
    reducedMotion: true,
    textScale: 200,
  });
  await expectNoAccessibilityViolations(page);
  await expectNoLayoutOverflow(page);
});

test("keyboard traversal reaches the skip link and primary navigation", async ({ page }) => {
  await openApplication(page, {
    language: "en",
    darkMode: false,
    compact: false,
    reducedMotion: false,
    textScale: 100,
  });

  const skipLink = page.getByRole("link", { name: "Skip to main content", exact: true });
  await page.keyboard.press("Tab");
  await expect(skipLink).toBeFocused();
  await expect(skipLink).toBeVisible();

  const workspace = page.getByRole("button", { name: "Choose a workspace to begin", exact: true });
  await page.keyboard.press("Tab");
  await expect(workspace).toBeFocused();

  const today = page.getByRole("button", { name: "Today", exact: true });
  await page.keyboard.press("Tab");
  await expect(today).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(workspace).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(skipLink).toBeFocused();

  await page.keyboard.press("Enter");
  await expect.poll(() => page.evaluate(() => window.location.hash)).toBe("#main-content");
  await expect(page.locator("#main-content")).toBeVisible();

  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: "简体中文", exact: true })).toBeFocused();
});

test("density toggle changes the full application rhythm", async ({ page }) => {
  await openApplication(page, {
    language: "en",
    darkMode: false,
    compact: false,
    reducedMotion: false,
    textScale: 100,
  });

  const readDensity = () =>
    page.evaluate(() => {
      const shell = document.querySelector<HTMLElement>(".desktop-shell");
      const header = document.querySelector<HTMLElement>("#main-content > div > header");
      const sidebarItem = document.querySelector<HTMLElement>('[data-sidebar="menu-button"]');
      const card = document.querySelector<HTMLElement>('[data-slot="card"]');
      if (!shell || !header || !sidebarItem || !card) {
        throw new Error("density fixtures are not rendered");
      }
      const style = getComputedStyle(shell);
      return {
        density: shell.dataset.density,
        controlHeight: style.getPropertyValue("--control-height").trim(),
        sectionGap: style.getPropertyValue("--density-section-gap").trim(),
        panelPadding: style.getPropertyValue("--density-panel-padding").trim(),
        pagePadding: style.getPropertyValue("--page-padding-block").trim(),
        cardSpacing: getComputedStyle(card).getPropertyValue("--card-spacing").trim(),
        headerHeight: header.getBoundingClientRect().height,
        sidebarItemHeight: sidebarItem.getBoundingClientRect().height,
      };
    });

  const comfortable = await readDensity();
  expect(comfortable).toMatchObject({
    density: "comfortable",
    controlHeight: "2.25rem",
    sectionGap: "1rem",
    panelPadding: "1rem",
    pagePadding: "1.5rem",
    cardSpacing: "1rem",
    headerHeight: 56,
    sidebarItemHeight: 36,
  });

  await page.getByRole("button", { name: "Compact density" }).click();
  await expect(page.getByRole("button", { name: "Comfortable density" })).toBeVisible();
  await page.waitForTimeout(250);

  const compact = await readDensity();
  expect(compact).toMatchObject({
    density: "compact",
    controlHeight: "2rem",
    sectionGap: "0.625rem",
    panelPadding: "0.75rem",
    pagePadding: "1rem",
    cardSpacing: "0.75rem",
    headerHeight: 48,
    sidebarItemHeight: 32,
  });
});

test("toolbar appearance and language buttons update the application state", async ({ page }) => {
  await openApplication(page, {
    language: "en",
    darkMode: false,
    compact: false,
    reducedMotion: false,
    textScale: 100,
  });

  await page.getByRole("button", { name: "Dark mode", exact: true }).click();
  await expect(page.locator("html")).toHaveClass(/\bdark\b/u);
  await expect(page.getByRole("button", { name: "Light mode", exact: true })).toBeVisible();

  await page.getByRole("button", { name: "简体中文", exact: true }).click();
  await expect(page.getByRole("button", { name: "English", exact: true })).toBeVisible();
  await expect(
    page.getByRole("heading", {
      name: "用更清晰的流程，准备更有说服力的申请。",
      level: 1,
    }),
  ).toBeVisible();
  await expectNoLayoutOverflow(page);
});

test("deferred product views remain reachable from primary navigation", async ({ page }) => {
  const runtimeErrors: string[] = [];
  page.on("pageerror", (error) => runtimeErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") runtimeErrors.push(message.text());
  });

  await openApplication(page, {
    language: "en",
    darkMode: false,
    compact: false,
    reducedMotion: false,
    textScale: 100,
  });

  for (const [navigationName, headingName] of [
    ["Opportunities", "Opportunity discovery"],
    ["Application workspace", "Application workspace"],
    ["Profile", "Reusable profile evidence"],
    ["Workspaces", "Workspaces"],
    ["Settings", "Settings and diagnostics"],
  ] as const) {
    await page.getByRole("button", { name: navigationName, exact: true }).click();
    await expect(page.getByRole("heading", { name: headingName, level: 1 })).toBeVisible();
    await expectNoLayoutOverflow(page);
  }

  expect(runtimeErrors).toEqual([]);
});

test("secondary workspace and Agent actions use progressive disclosure", async ({ page }) => {
  await openApplication(page, {
    language: "en",
    darkMode: false,
    compact: true,
    reducedMotion: false,
    textScale: 100,
  });

  await page.getByRole("button", { name: "Today", exact: true }).click();
  await expect(page.getByRole("button", { name: "Import source" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "New application" })).toHaveCount(0);
  const diagnostics = page.getByRole("button", {
    name: "System diagnostics",
    exact: true,
  });
  await diagnostics.click();
  await expect(page.getByRole("button", { name: "Run diagnostics" })).toBeVisible();
  await diagnostics.click();

  for (const [navigationName, hiddenActions] of [
    ["Opportunities", ["Refresh"]],
    ["Application workspace", ["Refresh", "Create application"]],
    ["Profile", ["Refresh"]],
  ] as const) {
    await page.getByRole("button", { name: navigationName, exact: true }).click();
    for (const hiddenAction of hiddenActions) {
      await expect(page.getByRole("button", { name: hiddenAction, exact: true })).toHaveCount(0);
    }
  }

  await page.getByRole("button", { name: "Workspaces", exact: true }).click();
  await expect(page.getByRole("button", { name: "Create workspace" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Connect existing" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Restore backup" })).toHaveCount(0);

  const workspaceActions = page.getByRole("button", {
    name: "Workspace actions",
    exact: true,
  });
  await workspaceActions.click();
  await expect(page.getByRole("menuitem", { name: "Refresh", exact: true })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Connect existing" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Restore backup" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(workspaceActions).toBeFocused();

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("tab", { name: "Terminal CLI", exact: true }).click();
  const cliActions = page.getByRole("button", { name: "More actions", exact: true });
  await expect(cliActions).toHaveCount(1);
  await cliActions.click();
  await expect(page.getByRole("menuitem", { name: "Check CLI", exact: true })).toBeVisible();
  await expect(
    page.getByRole("menuitem", { name: "Uninstall managed CLI", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(cliActions).toBeFocused();

  await page.getByRole("button", { name: "Agent integration", exact: true }).click();
  for (const hiddenAction of [
    "Refresh runtimes",
    "Refresh guidance",
    "Prepare AI workspace",
    "Check Skills",
    "Prepare MCP configuration",
  ]) {
    await expect(page.getByRole("button", { name: hiddenAction, exact: true })).toHaveCount(0);
  }
  await expect(page.getByRole("button", { name: "Advanced Agent tools" })).toHaveCount(0);
  await expectNoLayoutOverflow(page);
});

test("restored workflow routes render the correct application section", async ({ page }) => {
  await openApplication(page, {
    language: "en",
    darkMode: false,
    compact: true,
    reducedMotion: false,
    textScale: 100,
  });

  for (const [view, detail, heading] of [
    ["workflow", "decision-criteria", "Job and selection criteria"],
    ["workflow", "decision-matches", "Evidence and fit"],
    ["delivery", "delivery-documents", "Application materials"],
    ["delivery", "delivery-review", "Review and export"],
  ] as const) {
    await page.evaluate(
      ([activeView, activeDetail]) => {
        localStorage.setItem(
          "canisend.desktop.navigation.v1",
          JSON.stringify({
            version: 1,
            activeView,
            activeDetail,
            workspacePath: null,
            selectedJobs: {},
            lastAction: null,
          }),
        );
      },
      [view, detail],
    );
    await page.reload();

    await expect(page.getByRole("heading", { name: heading, level: 1 })).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Application workspace", exact: true }),
    ).toHaveAttribute("aria-current", "page");
    await expect(page.getByText("No workspace selected", { exact: true }).last()).toBeVisible();
    await expectNoLayoutOverflow(page);
  }
});

test("settings and Agent tabs bind selected triggers to their exact panels", async ({ page }) => {
  await openApplication(page, {
    language: "en",
    darkMode: false,
    compact: true,
    reducedMotion: false,
    textScale: 100,
  });

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  for (const [tabName, panelHeading] of [
    ["Appearance", "Accessibility & appearance"],
    ["Terminal CLI", "Terminal CLI"],
    ["Check for updates", "Check for updates"],
    ["Schema and resource inspection", "Schema and resource inspection"],
  ] as const) {
    const trigger = page.getByRole("tab", { name: tabName, exact: true });
    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-selected", "true");
    await expect(page.getByRole("tabpanel", { name: tabName, exact: true })).toBeVisible();
    await expect(page.getByText(panelHeading, { exact: true }).first()).toBeVisible();

    const activeBackground = await trigger.evaluate(
      (element) => getComputedStyle(element).backgroundColor,
    );
    const inactiveBackground = await page
      .getByRole("tab", { name: tabName === "Appearance" ? "Terminal CLI" : "Appearance" })
      .evaluate((element) => getComputedStyle(element).backgroundColor);
    expect(activeBackground).not.toBe(inactiveBackground);
  }

  await page.getByRole("button", { name: "Agent integration", exact: true }).click();
  for (const [tabName, panelText] of [
    ["Agent host", "Continue in agent host"],
    ["In-App read-only", "Optional runtime bridge"],
  ] as const) {
    const trigger = page.getByRole("tab", { name: tabName, exact: true });
    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-selected", "true");
    await expect(page.getByRole("tabpanel", { name: tabName, exact: true })).toBeVisible();
    await expect(page.getByText(panelText, { exact: true }).first()).toBeVisible();
  }

  await expectNoLayoutOverflow(page);
});

test("sidebar and workspace context keep one clear interactive state", async ({ page }) => {
  await openApplication(page, {
    language: "en",
    darkMode: false,
    compact: false,
    reducedMotion: false,
    textScale: 100,
  });

  const activeNavigation = page.locator('[data-sidebar="menu-button"][data-active="true"]');
  await expect(activeNavigation).toHaveCount(1);
  await expect(activeNavigation).toHaveAttribute("aria-current", "page");
  await expect(page.locator('[data-sidebar="menu-button"][data-active="false"]')).toHaveCount(0);

  await page.getByRole("button", { name: "Opportunities", exact: true }).click();
  await expect(activeNavigation).toHaveCount(1);
  await expect(activeNavigation).toHaveText("Opportunities");

  const contextToggle = page.getByRole("button", {
    name: "Current application snapshot",
    exact: true,
  });
  await expect(contextToggle).toHaveAttribute("aria-expanded", "false");
  await contextToggle.click();
  await expect(contextToggle).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByRole("progressbar", { name: "Workflow progress" })).toBeVisible();
  await contextToggle.click();
  await expect(contextToggle).toHaveAttribute("aria-expanded", "false");

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("tab", { name: "Appearance", exact: true }).click();
  await expect(page.getByRole("tabpanel", { name: "Appearance" })).toBeVisible();
  await expectNoLayoutOverflow(page);
});

test("primary navigation starts each page at its visible header", async ({ page }) => {
  await openApplication(page, {
    language: "en",
    darkMode: false,
    compact: false,
    reducedMotion: false,
    textScale: 100,
  });

  await page.getByRole("button", { name: "Agent integration", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Connected agent workspace", level: 1 }),
  ).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(0);

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBe(0);
  await expect(
    page.getByRole("heading", { name: "Settings and diagnostics", level: 1 }),
  ).toBeVisible();
});

test("all primary views reflow in Chinese at 200 percent text", async ({ page }) => {
  const runtimeErrors: string[] = [];
  page.on("pageerror", (error) => runtimeErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") runtimeErrors.push(message.text());
  });

  await page.setViewportSize({ width: 960, height: 680 });
  await openApplication(page, {
    language: "zh-CN",
    darkMode: true,
    compact: true,
    reducedMotion: true,
    textScale: 200,
  });

  await expectNoLayoutOverflow(page);

  for (const [navigationName, headingName] of [
    ["职位机会", "职位机会发现"],
    ["申请工作台", "申请工作台"],
    ["个人资料", "可复用的个人证据"],
    ["Agent 集成", "Agent 协作工作区"],
    ["工作区", "工作区"],
    ["设置", "设置与诊断"],
  ] as const) {
    await page.getByRole("button", { name: navigationName, exact: true }).click();
    await expect(page.getByRole("heading", { name: headingName, level: 1 })).toBeVisible();
    await expectNoLayoutOverflow(page);
  }

  expect(runtimeErrors).toEqual([]);
});
