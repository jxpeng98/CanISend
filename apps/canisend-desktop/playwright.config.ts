import { defineConfig } from "@playwright/test";

const previewPort = Number(process.env.CANISEND_VISUAL_PORT ?? 14_320);
const previewUrl = `http://127.0.0.1:${previewPort}`;

export default defineConfig({
  testDir: "./tests/visual",
  outputDir: "./test-results",
  fullyParallel: false,
  reporter: "line",
  expect: {
    toHaveScreenshot: {
      animations: "disabled",
      maxDiffPixelRatio: 0.01,
    },
  },
  use: {
    baseURL: previewUrl,
    channel: "chrome",
    colorScheme: "light",
    locale: "en-GB",
    reducedMotion: "reduce",
    viewport: { width: 1280, height: 820 },
  },
  webServer: {
    command: `vite --host 127.0.0.1 --port ${previewPort} --strictPort`,
    url: `${previewUrl}/?ui-system=1`,
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
