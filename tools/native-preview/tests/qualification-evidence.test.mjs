import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

import { recordHostSize } from "../record-host-size.mjs";
import { summarizeMatrix } from "../summarize-evidence.mjs";

const policy = JSON.parse(
  readFileSync(resolve(import.meta.dirname, "../policy.json"), "utf8"),
);
function writeTargetEvidence(root, target, status = "passed") {
  const directory = join(root, target.slug);
  mkdirSync(directory, { recursive: true });
  const screenshotPath = join(directory, "native-pdf-preview.png");
  copyFileSync(
    resolve(import.meta.dirname, "../fixtures/native-preview-probe.pdf"),
    screenshotPath,
  );
  const screenshotSha256 = createHash("sha256")
    .update(readFileSync(screenshotPath))
    .digest("hex");
  writeFileSync(
    join(directory, "native-pdf-preview.json"),
    JSON.stringify({
      schema: "canisend.native-pdf-preview-qualification/v1",
      status,
      platform: target.nodePlatform,
      runner: target.slug,
      target: target.rustTarget,
      fixture: policy.fixture,
      frame: {
        height: 600,
        source: "blob:",
        userAgent: `fixture-${target.engine}`,
        width: 900,
      },
      pixels: {
        blueRatio: 0.01,
        darkRatio: 0.3,
        lightRatio: 0.4,
        screenshotSha256,
      },
    }),
  );
  writeFileSync(
    join(directory, "native-preview-host-size.json"),
    JSON.stringify({
      schema: "canisend.native-preview-host-size/v1",
      status: "passed",
      target: target.rustTarget,
      production: {
        bytes: 1000,
        features: ["custom-protocol"],
        sha256: "c".repeat(64),
      },
      qualification: {
        bytes: 1100,
        features: ["custom-protocol", "preview-qualification"],
        sha256: "d".repeat(64),
      },
      deltaBytes: 100,
      deltaPercent: 10,
    }),
  );
}

test("matrix summary accepts complete native evidence", () => {
  const root = mkdtempSync(join(tmpdir(), "canisend-native-preview-summary-"));
  try {
    for (const target of policy.targets) writeTargetEvidence(root, target);
    const summary = summarizeMatrix(root, policy);
    assert.equal(summary.status, "passed");
    assert.deepEqual(summary.decision.fallbackRequiredFor, []);
    assert.equal(summary.decision.pdfjs, "not-required");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("matrix summary names the platform that requires fallback review", () => {
  const root = mkdtempSync(join(tmpdir(), "canisend-native-preview-summary-"));
  try {
    for (const target of policy.targets) writeTargetEvidence(root, target);
    writeTargetEvidence(root, policy.targets[2], "failed");
    const summary = summarizeMatrix(root, policy);
    assert.equal(summary.status, "failed");
    assert.deepEqual(summary.decision.fallbackRequiredFor, [
      "linux-webkitgtk",
    ]);
    assert.equal(summary.decision.pdfjs, policy.decision.pdfjs);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("matrix summary rejects duplicate platform evidence", () => {
  const root = mkdtempSync(join(tmpdir(), "canisend-native-preview-summary-"));
  try {
    for (const target of policy.targets) writeTargetEvidence(root, target);
    writeTargetEvidence(join(root, "duplicate"), policy.targets[0]);
    const summary = summarizeMatrix(root, policy);
    assert.equal(summary.status, "failed");
    assert.deepEqual(summary.decision.fallbackRequiredFor, [
      "macos-wkwebview",
    ]);
    assert.match(
      summary.targets[0].errors[0],
      /expected one native preview record, found 2/,
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("host-size recorder binds production and qualification builds", () => {
  const root = mkdtempSync(join(tmpdir(), "canisend-native-preview-size-"));
  try {
    const binary = join(root, "canisend-gui");
    const output = join(root, "native-preview-host-size.json");
    writeFileSync(binary, "production");
    recordHostSize("production", binary, "test-target", output);
    writeFileSync(binary, "qualification-is-larger");
    const evidence = recordHostSize(
      "qualification",
      binary,
      "test-target",
      output,
    );
    assert.equal(evidence.status, "passed");
    assert.ok(evidence.deltaBytes > 0);
    assert.notEqual(
      evidence.production.sha256,
      evidence.qualification.sha256,
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
