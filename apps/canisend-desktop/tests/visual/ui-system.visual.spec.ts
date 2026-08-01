import { expect, test, type Page } from "@playwright/test";

async function openGallery(page: Page): Promise<void> {
  await page.goto("/?ui-system=1");
  await expect(page.getByTestId("ui-system-gallery")).toBeVisible();
  await page.addStyleTag({
    content: "*, *::before, *::after { caret-color: transparent !important; }",
  });
}

test("light comfortable gallery", async ({ page }) => {
  await openGallery(page);
  await expect(page).toHaveScreenshot("gallery-light-comfortable-1280.png", {
    fullPage: true,
  });
});

test("dark compact reduced-motion gallery", async ({ page }) => {
  await openGallery(page);
  await page.getByRole("button", { name: "Dark" }).click();
  await page.getByRole("button", { name: "Compact" }).click();
  await page.getByRole("button", { name: "Reduce motion" }).click();
  await expect(page.getByRole("button", { name: "Light" })).toBeVisible();
  await expect(page).toHaveScreenshot("gallery-dark-compact-reduced-1280.png", {
    fullPage: true,
  });
});

test("minimum window at 200 percent text", async ({ page }) => {
  await page.setViewportSize({ width: 960, height: 680 });
  await openGallery(page);
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "200%";
  });
  await expect(page).toHaveScreenshot("gallery-light-comfortable-960-text-200.png", {
    fullPage: true,
  });
});
