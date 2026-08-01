import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

async function openGallery(page: Page): Promise<void> {
  await page.goto("/?ui-system=1");
  await expect(page.getByTestId("ui-system-gallery")).toBeVisible();
}

async function expectNoAccessibilityViolations(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations).toEqual([]);
}

test("light gallery meets automated accessibility rules", async ({ page }) => {
  await openGallery(page);
  await expectNoAccessibilityViolations(page);
});

test("dark compact gallery meets automated accessibility rules", async ({ page }) => {
  await openGallery(page);
  await page.getByRole("button", { name: "Dark" }).click();
  await page.getByRole("button", { name: "Compact" }).click();
  await expect(page.getByRole("button", { name: "Light" })).toBeVisible();
  await expectNoAccessibilityViolations(page);
});
