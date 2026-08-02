import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const featureSets = {
  production: ["custom-protocol"],
  qualification: ["custom-protocol", "preview-qualification"],
};

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

export function recordHostSize(mode, binary, target, output) {
  if (!(mode in featureSets)) {
    throw new Error(`unknown host-size mode: ${mode}`);
  }
  const binaryPath = resolve(binary);
  const outputPath = resolve(output);
  const record = {
    bytes: statSync(binaryPath).size,
    features: featureSets[mode],
    sha256: sha256(binaryPath),
  };
  let evidence = {
    schema: "canisend.native-preview-host-size/v1",
    status: "incomplete",
    target,
  };
  try {
    evidence = JSON.parse(readFileSync(outputPath, "utf8"));
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
  if (evidence.target !== target) {
    throw new Error(
      `host-size target changed from ${evidence.target} to ${target}`,
    );
  }
  evidence[mode] = record;
  if (evidence.production && evidence.qualification) {
    evidence.deltaBytes =
      evidence.qualification.bytes - evidence.production.bytes;
    evidence.deltaPercent = Number(
      ((evidence.deltaBytes / evidence.production.bytes) * 100).toFixed(4),
    );
    evidence.status = evidence.deltaBytes > 0 ? "passed" : "failed";
  }
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
  return evidence;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [, , mode, binary, target, output] = process.argv;
  if (!mode || !binary || !target || !output) {
    console.error(
      "usage: record-host-size.mjs <production|qualification> <binary> <target> <output.json>",
    );
    process.exitCode = 2;
  } else {
    try {
      const evidence = recordHostSize(mode, binary, target, output);
      console.log(JSON.stringify(evidence));
      if (mode === "qualification" && evidence.status !== "passed") {
        process.exitCode = 1;
      }
    } catch (error) {
      console.error(error instanceof Error ? error.message : String(error));
      process.exitCode = 1;
    }
  }
}
