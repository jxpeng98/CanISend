import { existsSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";

// Avoid inheriting a host application's custom global Undici dispatcher. WebdriverIO documents
// this native-fetch path for its own smoke tests, and it is stable across the three CI runners.
process.env.WDIO_USE_NATIVE_FETCH = "1";

const binary = process.env.CANISEND_NATIVE_PREVIEW_BINARY;

if (!binary) {
  throw new Error(
    "CANISEND_NATIVE_PREVIEW_BINARY must point to a preview-qualification build",
  );
}

const appBinaryPath = resolve(binary);
if (!existsSync(appBinaryPath)) {
  throw new Error(`native preview host does not exist: ${appBinaryPath}`);
}

const evidenceDirectory = resolve(
  process.env.CANISEND_NATIVE_PREVIEW_EVIDENCE ??
    "./native-preview-evidence",
);
mkdirSync(evidenceDirectory, { recursive: true });
const embeddedPort = Number(process.env.TAURI_WEBDRIVER_PORT ?? "4445");

export const config = {
  tsConfigPath: "./tsconfig.json",
  runner: "local",
  hostname: "127.0.0.1",
  port: embeddedPort,
  runnerEnv: {
    WDIO_USE_NATIVE_FETCH: "1",
  },
  specs: ["./tests/**/*.spec.ts"],
  maxInstances: 1,
  capabilities: [
    {
      browserName: "tauri",
      "wdio:enforceWebDriverClassic": true,
      "tauri:options": {
        application: appBinaryPath,
      },
    },
  ],
  services: [
    [
      "@wdio/tauri-service",
      {
        appBinaryPath,
        autoDownloadEdgeDriver: false,
        autoInstallTauriDriver: false,
        captureBackendLogs: false,
        captureFrontendLogs: false,
        driverProvider: "embedded",
        embeddedPort,
        startTimeout: 90_000,
        statusPollTimeout: 10_000,
      },
    ],
  ],
  logLevel: "info",
  bail: 1,
  waitforTimeout: 20_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 1,
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 120_000,
  },
  outputDir: evidenceDirectory,
};
