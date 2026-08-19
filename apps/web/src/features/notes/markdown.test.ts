import { describe, expect, it } from "vitest";
import { markdownSummary, plainTextFromMarkdown } from "./markdown";

describe("plainTextFromMarkdown", () => {
  it("removes common markdown syntax while preserving note content", () => {
    expect(plainTextFromMarkdown("## Project\n\n- [x] **Done** [docs](https://example.com)")).toBe("Project Done docs");
  });

  it("keeps fenced code text for search", () => {
    expect(plainTextFromMarkdown("```ts\nconst answer = 42\n```"))
      .toContain("const answer = 42");
  });
});

describe("markdownSummary", () => {
  it("limits the plain-text summary", () => {
    expect(markdownSummary("# Title\n\nabcdef", 8)).toBe("Title ab");
  });
});
