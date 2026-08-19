import { describe, expect, it } from "vitest";
import { applyMarkdownFormat, markdownSummary } from "./markdown";

describe("applyMarkdownFormat", () => {
  it("wraps a selection with bold syntax", () => {
    const result = applyMarkdownFormat("hello world", 6, 11, "bold");
    expect(result.value).toBe("hello **world**");
    expect(result.value.slice(result.selectionStart, result.selectionEnd)).toBe("world");
  });

  it("prefixes all selected lines as an ordered list", () => {
    const source = "first\nsecond\nthird";
    const result = applyMarkdownFormat(source, 0, source.length, "ordered");
    expect(result.value).toBe("1. first\n2. second\n3. third");
  });

  it("creates a fenced code block", () => {
    const result = applyMarkdownFormat("const x = 1", 0, 11, "code-block");
    expect(result.value).toContain("```\nconst x = 1\n```");
  });
});

describe("markdownSummary", () => {
  it("removes common markdown syntax from note summaries", () => {
    expect(markdownSummary("## Project\n\n- **Done** [docs](https://example.com)")).toBe("Project Done docs");
  });
});
