import { describe, expect, it } from "vitest";

import { exactUtf8Span } from "./generic-application-form";

describe("generic Application form contracts", () => {
  it("computes exact UTF-8 byte spans for localized reviewed source text", () => {
    const source = "Eligibility: 申请人必须提交项目说明。";
    const statement = "申请人必须提交项目说明";
    const span = exactUtf8Span(source, statement);

    expect(span).not.toBeNull();
    const bytes = new TextEncoder().encode(source);
    const [start, end] = span!;
    expect(new TextDecoder().decode(bytes.slice(start, end))).toBe(statement);
  });

  it("rejects a requirement that is not an exact source excerpt", () => {
    expect(exactUtf8Span("A signed form is required.", "A form is required."))
      .toBeNull();
  });
});
