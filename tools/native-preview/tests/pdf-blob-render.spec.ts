import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { browser } from "@wdio/globals";
import { PNG } from "pngjs";

import policy from "../policy.json" with { type: "json" };

const testDirectory = dirname(fileURLToPath(import.meta.url));
const fixturePath = resolve(testDirectory, "../fixtures/native-preview-probe.pdf");
const evidenceDirectory = resolve(
  process.env.CANISEND_NATIVE_PREVIEW_EVIDENCE ??
    "./native-preview-evidence",
);

type PixelEvidence = {
  blueRatio: number;
  darkRatio: number;
  height: number;
  lightRatio: number;
  opaquePixels: number;
  screenshotSha256: string;
  width: number;
};

function inspectScreenshot(screenshot: Buffer): PixelEvidence {
  const png = PNG.sync.read(screenshot);
  let bluePixels = 0;
  let darkPixels = 0;
  let lightPixels = 0;
  let opaquePixels = 0;

  for (let offset = 0; offset < png.data.length; offset += 4) {
    const red = png.data[offset] ?? 0;
    const green = png.data[offset + 1] ?? 0;
    const blue = png.data[offset + 2] ?? 0;
    const alpha = png.data[offset + 3] ?? 0;
    if (alpha < 200) {
      continue;
    }
    opaquePixels += 1;
    if (red < 55 && green < 55 && blue < 55) {
      darkPixels += 1;
    }
    if (red > 235 && green > 235 && blue > 235) {
      lightPixels += 1;
    }
    if (blue > 145 && blue > red * 1.45 && blue > green * 1.15) {
      bluePixels += 1;
    }
  }

  assert.ok(opaquePixels > 0, "native screenshot contained no opaque pixels");
  return {
    blueRatio: bluePixels / opaquePixels,
    darkRatio: darkPixels / opaquePixels,
    height: png.height,
    lightRatio: lightPixels / opaquePixels,
    opaquePixels,
    screenshotSha256: createHash("sha256").update(screenshot).digest("hex"),
    width: png.width,
  };
}

describe("native PDF preview", () => {
  it("renders the deterministic PDF blob inside the platform WebView", async () => {
    const fixture = readFileSync(fixturePath);
    const fixtureSha256 = createHash("sha256").update(fixture).digest("hex");
    assert.equal(
      fixtureSha256,
      policy.fixture.sha256,
      "native preview fixture changed without updating its render contract",
    );
    assert.equal(fixture.byteLength, policy.fixture.bytes);

    await browser.setWindowSize(1_200, 900);
    const frame = await browser.execute((pdfBase64: string) => {
      const probeWindow = window as typeof window & {
        __canisendPdfProbeUrl?: string;
      };
      if (probeWindow.__canisendPdfProbeUrl) {
        URL.revokeObjectURL(probeWindow.__canisendPdfProbeUrl);
      }

      const bytes = Uint8Array.from(atob(pdfBase64), (character) =>
        character.charCodeAt(0),
      );
      const blobUrl = URL.createObjectURL(
        new Blob([bytes], { type: "application/pdf" }),
      );
      probeWindow.__canisendPdfProbeUrl = blobUrl;

      document.documentElement.style.cssText =
        "background:#d4d4d4;color:#171717;height:100%;margin:0;overflow:hidden";
      document.body.style.cssText =
        "background:#d4d4d4;height:100%;margin:0;overflow:hidden;padding:0";
      const label = document.createElement("div");
      label.textContent = "CanISend native PDF preview qualification";
      label.style.cssText =
        "box-sizing:border-box;height:40px;padding:10px 16px;background:#e5e5e5;color:#171717;font:600 14px system-ui";
      const iframe = document.createElement("iframe");
      iframe.id = "canisend-native-pdf-probe";
      iframe.title = "CanISend native PDF probe";
      iframe.src = blobUrl;
      iframe.style.cssText =
        "display:block;width:100%;height:calc(100% - 40px);border:0;background:#a3a3a3";
      document.body.replaceChildren(label, iframe);
      return {
        devicePixelRatio: window.devicePixelRatio,
        height: iframe.getBoundingClientRect().height,
        source: iframe.src.slice(0, 5),
        userAgent: navigator.userAgent,
        width: iframe.getBoundingClientRect().width,
      };
    }, fixture.toString("base64"));

    assert.equal(frame.source, "blob:");
    assert.ok(
      frame.width >= policy.thresholds.minimumFrameWidth,
      `native PDF frame is unexpectedly narrow: ${JSON.stringify(frame)}`,
    );
    assert.ok(
      frame.height >= policy.thresholds.minimumFrameHeight,
      `native PDF frame is unexpectedly short: ${JSON.stringify(frame)}`,
    );

    await browser.pause(5_000);
    mkdirSync(evidenceDirectory, { recursive: true });
    const screenshotPath = resolve(evidenceDirectory, "native-pdf-preview.png");
    await browser.saveScreenshot(screenshotPath);
    const screenshot = readFileSync(screenshotPath);
    const pixels = inspectScreenshot(screenshot);
    const rendererDetected =
      pixels.darkRatio >= policy.thresholds.minimumDarkRatio &&
      pixels.lightRatio >= policy.thresholds.minimumLightRatio &&
      pixels.blueRatio >= policy.thresholds.minimumBlueRatio;
    const evidence = {
      schema: "canisend.native-pdf-preview-qualification/v1",
      status: rendererDetected ? "passed" : "failed",
      platform: `${process.platform}-${process.arch}`,
      runner: process.env.CANISEND_NATIVE_PREVIEW_RUNNER ?? "local",
      target: process.env.CANISEND_NATIVE_PREVIEW_TARGET ?? "local",
      fixture: {
        bytes: fixture.byteLength,
        sha256: fixtureSha256,
      },
      frame,
      pixels,
    };
    writeFileSync(
      resolve(evidenceDirectory, "native-pdf-preview.json"),
      `${JSON.stringify(evidence, null, 2)}\n`,
      "utf8",
    );

    assert.ok(
      rendererDetected,
      `native PDF renderer contract failed: ${JSON.stringify(pixels)}`,
    );
  });
});
