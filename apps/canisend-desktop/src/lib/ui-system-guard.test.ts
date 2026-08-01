import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

import { describe, expect, it } from "vitest";

const sourceRoot = fileURLToPath(new URL("..", import.meta.url));
const appStyles = readFileSync(path.join(sourceRoot, "app.css"), "utf8");
const excludedSegments = [
  `${path.sep}components${path.sep}ui${path.sep}`,
  `${path.sep}components${path.sep}patterns${path.sep}`,
  `${path.sep}test-fixtures${path.sep}`,
];

function svelteSources(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const candidate = path.join(directory, entry.name);
    if (excludedSegments.some((segment) => candidate.includes(segment))) return [];
    if (entry.isDirectory()) return svelteSources(candidate);
    return entry.isFile() && entry.name.endsWith(".svelte") ? [candidate] : [];
  });
}

const featureSources = svelteSources(sourceRoot).map((file) => ({
  file: path.relative(sourceRoot, file),
  source: readFileSync(file, "utf8"),
}));
const pageViewSources = featureSources.filter(
  ({ file }) => file.startsWith(`lib${path.sep}views${path.sep}`) && file.endsWith("View.svelte"),
);

function violations(pattern: RegExp): string[] {
  return featureSources.flatMap(({ file, source }) =>
    Array.from(source.matchAll(pattern), (match) => `${file}: ${match[0]}`),
  );
}

function componentOpeningTags(source: string, component: string): string[] {
  return Array.from(
    source.matchAll(new RegExp(`<${component}\\b[\\s\\S]*?>`, "gu")),
    (match) => match[0],
  );
}

describe("shadcn-svelte migration guard", () => {
  it("routes primary page shells through shared Page compositions", () => {
    expect(pageViewSources.length).toBeGreaterThan(0);

    for (const { file, source } of pageViewSources) {
      expect(source, file).toContain('components/patterns/page/index.js');
      expect(source, file).toContain("<Page.Root");
      expect(source, file).toContain("<Page.Header");
      expect(source, file).not.toMatch(/<section\b[^>]*class=["'][^"']*page-header/u);
    }
  });

  it("keeps native controls inside the UI registry", () => {
    expect(
      violations(/<(?:button|input|textarea|select|details|summary)\b[^>]*>/gu),
    ).toEqual([]);
  });

  it("keeps every feature Button connected to an action or delegated trigger", () => {
    const inertButtons = featureSources.flatMap(({ file, source }) =>
      componentOpeningTags(source, "Button")
        .filter(
          (tag) =>
            !/\bonclick\s*=/u.test(tag) &&
            !/\btype\s*=\s*["']submit["']/u.test(tag) &&
            !/\bhref\s*=/u.test(tag) &&
            !/\{\.\.\.props\}/u.test(tag),
        )
        .map((tag) => `${file}: ${tag.replace(/\s+/gu, " ")}`),
    );

    expect(inertButtons).toEqual([]);
  });

  it("keeps every feature action-menu item connected to an action", () => {
    const inertMenuItems = featureSources.flatMap(({ file, source }) =>
      Array.from(
        source.matchAll(/<DropdownMenu\.Item\b[\s\S]*?>/gu),
        (match) => match[0],
      )
        .filter((tag) => !/\bonclick\s*=/u.test(tag))
        .map((tag) => `${file}: ${tag.replace(/\s+/gu, " ")}`),
    );

    expect(inertMenuItems).toEqual([]);
  });

  it("keeps page-level disclosures on a consistent heading level", () => {
    const implicitAccordionHeadings = pageViewSources.flatMap(({ file, source }) =>
      componentOpeningTags(source, "Accordion.Trigger")
        .filter((tag) => !/\blevel\s*=\s*\{2\}/u.test(tag))
        .map((tag) => `${file}: ${tag.replace(/\s+/gu, " ")}`),
    );

    expect(implicitAccordionHeadings).toEqual([]);
  });

  it("keeps status meaning on semantic tokens", () => {
    expect(
      violations(
        /(?:text|bg|border|ring)-(?:red|amber|yellow|green|emerald|lime|blue|sky|cyan|orange|rose|teal|indigo|violet|purple|fuchsia)-[^\s"']+|\[var\(--success\)\]/giu,
      ),
    ).toEqual([]);
  });

  it("routes announced loading and error panels through shared compositions", () => {
    expect(
      violations(/<(?:div|p|section|article)\b[^>]*role=["'](?:alert|status)["'][^>]*>/giu),
    ).toEqual([]);
  });

  it("keeps feature layouts free of fixed minimum widths", () => {
    expect(violations(/\bmin-w-\[[^\]]+\]/gu)).toEqual([]);
  });

  it("keeps feature surfaces within the shared radius scale", () => {
    expect(appStyles).toContain("--radius: 0.625rem;");
    expect(appStyles).toContain("--radius-xl: calc(var(--radius) + 2px);");
    expect(
      violations(
        /\brounded-(?:xl|2xl|3xl|4xl)\b|\brounded-\[(?:\d+(?:\.\d+)?(?:px|rem)|min\()[^\]]*\]/gu,
      ),
    ).toEqual([]);
  });

  it("keeps default tabs visually distinct from inactive options", () => {
    expect(appStyles).toContain('[data-slot="tabs-list"][data-variant="default"]');
    expect(appStyles).toContain('[data-slot="tabs-trigger"]:is([data-state="active"]');
    expect(appStyles).toContain("background: var(--primary);");
    expect(appStyles).toContain("color: var(--primary-foreground);");
  });
});
