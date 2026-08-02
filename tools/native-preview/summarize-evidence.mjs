import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const toolDirectory = fileURLToPath(new URL(".", import.meta.url));
const defaultPolicy = JSON.parse(
  readFileSync(resolve(toolDirectory, "policy.json"), "utf8"),
);

function collectNamedFiles(root, name, found = []) {
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = resolve(root, entry.name);
    if (entry.isDirectory()) collectNamedFiles(path, name, found);
    if (entry.isFile() && entry.name === name) found.push(path);
  }
  return found;
}

function readRecords(root, name) {
  return collectNamedFiles(root, name).map((path) => ({
    directory: dirname(path),
    path,
    value: JSON.parse(readFileSync(path, "utf8")),
  }));
}

function validSha256(value) {
  return typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
}

function fileSha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function validatePreview(record, target, policy) {
  const errors = [];
  const { value } = record;
  if (value.schema !== "canisend.native-pdf-preview-qualification/v1") {
    errors.push("unexpected preview schema");
  }
  if (value.status !== "passed") errors.push("direct preview did not pass");
  if (value.platform !== target.nodePlatform) errors.push("platform mismatch");
  if (value.runner !== target.slug) errors.push("runner slug mismatch");
  if (value.target !== target.rustTarget) errors.push("Rust target mismatch");
  if (value.fixture?.bytes !== policy.fixture.bytes) {
    errors.push("fixture byte count mismatch");
  }
  if (value.fixture?.sha256 !== policy.fixture.sha256) {
    errors.push("fixture SHA-256 mismatch");
  }
  if (value.frame?.source !== "blob:") errors.push("preview source is not a Blob URL");
  if (!value.frame?.userAgent) errors.push("missing native WebView user agent");
  if (value.frame?.width < policy.thresholds.minimumFrameWidth) {
    errors.push("preview frame is too narrow");
  }
  if (value.frame?.height < policy.thresholds.minimumFrameHeight) {
    errors.push("preview frame is too short");
  }
  if (value.pixels?.darkRatio < policy.thresholds.minimumDarkRatio) {
    errors.push("dark render ratio is too low");
  }
  if (value.pixels?.lightRatio < policy.thresholds.minimumLightRatio) {
    errors.push("light render ratio is too low");
  }
  if (value.pixels?.blueRatio < policy.thresholds.minimumBlueRatio) {
    errors.push("blue render ratio is too low");
  }
  if (!validSha256(value.pixels?.screenshotSha256)) {
    errors.push("invalid screenshot SHA-256");
  }
  const screenshot = resolve(record.directory, "native-pdf-preview.png");
  if (!existsSync(screenshot)) errors.push("missing native screenshot");
  else if (fileSha256(screenshot) !== value.pixels?.screenshotSha256) {
    errors.push("screenshot SHA-256 mismatch");
  }
  return errors;
}

function validateHostSize(record, target) {
  const errors = [];
  const { value } = record;
  if (value.schema !== "canisend.native-preview-host-size/v1") {
    errors.push("unexpected host-size schema");
  }
  if (value.status !== "passed") errors.push("host-size comparison did not pass");
  if (value.target !== target.rustTarget) errors.push("host-size target mismatch");
  if (value.deltaBytes <= 0) errors.push("qualification host did not grow");
  if (
    value.deltaBytes !==
    value.qualification?.bytes - value.production?.bytes
  ) {
    errors.push("host-size delta mismatch");
  }
  if (value.production?.features?.join(",") !== "custom-protocol") {
    errors.push("production feature set is not isolated");
  }
  if (
    value.qualification?.features?.join(",") !==
    "custom-protocol,preview-qualification"
  ) {
    errors.push("qualification feature set is incomplete");
  }
  if (!validSha256(value.production?.sha256)) {
    errors.push("invalid production host SHA-256");
  }
  if (!validSha256(value.qualification?.sha256)) {
    errors.push("invalid qualification host SHA-256");
  }
  if (value.production?.sha256 === value.qualification?.sha256) {
    errors.push("production and qualification hosts are identical");
  }
  return errors;
}

export function summarizeMatrix(root, policy = defaultPolicy) {
  const previews = readRecords(root, "native-pdf-preview.json");
  const sizes = readRecords(root, "native-preview-host-size.json");
  const expectedTargets = new Set(policy.targets.map(({ rustTarget }) => rustTarget));
  const unexpectedRecords = [...previews, ...sizes]
    .filter(({ value }) => !expectedTargets.has(value.target))
    .map(({ path }) => path);
  const fixture = resolve(toolDirectory, "fixtures/native-preview-probe.pdf");
  const policyErrors = [];
  if (statSync(fixture).size !== policy.fixture.bytes) {
    policyErrors.push("checked-in fixture byte count does not match policy");
  }
  if (fileSha256(fixture) !== policy.fixture.sha256) {
    policyErrors.push("checked-in fixture SHA-256 does not match policy");
  }
  const targets = policy.targets.map((target) => {
    const matchingPreviews = previews.filter(
      ({ value }) => value.target === target.rustTarget,
    );
    const matchingSizes = sizes.filter(
      ({ value }) => value.target === target.rustTarget,
    );
    const preview = matchingPreviews[0];
    const size = matchingSizes[0];
    const errors = [];
    if (matchingPreviews.length !== 1) {
      errors.push(
        `expected one native preview record, found ${matchingPreviews.length}`,
      );
    } else errors.push(...validatePreview(preview, target, policy));
    if (matchingSizes.length !== 1) {
      errors.push(`expected one host-size record, found ${matchingSizes.length}`);
    } else errors.push(...validateHostSize(size, target));
    return {
      engine: target.engine,
      errors,
      preview: preview?.value ?? null,
      size: size?.value ?? null,
      slug: target.slug,
      status: errors.length === 0 ? "passed" : "failed",
      target: target.rustTarget,
    };
  });
  const fallbackRequiredFor = targets
    .filter(({ status }) => status !== "passed")
    .map(({ slug }) => slug);
  const passed =
    fallbackRequiredFor.length === 0 &&
    unexpectedRecords.length === 0 &&
    policyErrors.length === 0;
  return {
    schema: "canisend.native-pdf-preview-matrix/v1",
    status: passed ? "passed" : "failed",
    commit: process.env.GITHUB_SHA ?? null,
    fixture: policy.fixture,
    policyErrors,
    targets,
    unexpectedRecords,
    decision: {
      fallbackRequiredFor,
      pdfjs:
        passed
          ? "not-required"
          : policy.decision.pdfjs,
      onDirectPreviewFailure: policy.decision.directPreviewFailure,
    },
  };
}

export function writeSummary(root, output, policy = defaultPolicy) {
  const summary = summarizeMatrix(resolve(root), policy);
  writeFileSync(resolve(output), `${JSON.stringify(summary, null, 2)}\n`, "utf8");
  return summary;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [, , root, output] = process.argv;
  if (!root || !output) {
    console.error("usage: summarize-evidence.mjs <evidence-root> <summary.json>");
    process.exitCode = 2;
  } else {
    try {
      const summary = writeSummary(root, output);
      console.log(JSON.stringify(summary));
      if (summary.status !== "passed") process.exitCode = 1;
    } catch (error) {
      console.error(error instanceof Error ? error.message : String(error));
      process.exitCode = 1;
    }
  }
}
