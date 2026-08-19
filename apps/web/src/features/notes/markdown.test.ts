import { describe, expect, it } from "vitest";
import { applyMarkdownFormat } from "./markdown";

describe("applyMarkdownFormat", () => {
  it("wraps selected text as bold and keeps the selection inside markers", () => {
    const result = applyMarkdownFormat("hello world", 6, 11, "bold");
    expect(result.value).toBe("hello **world**");
    expect(result.value.slice(result.selectionStart, result.selectionEnd)).toBe("world");
  });

  it("prefixes every selected line for ordered lists", () => {
    const source = "first\nsecond\nthird";
    const result = applyMarkdownFormat(source, 0, source.length, "ordered");
    expect(result.value).toBe("1. first\n2. second\n3. third");
  });

  it("creates a markdown link around the selected label", () => {
    const result = applyMarkdownFormat("LifeTrace", 0, 9, "link");
    expect(result.value).toBe("[LifeTrace](https://)");
    expect(result.value.slice(result.selectionStart, result.selectionEnd)).toBe("LifeTrace");
  });

  it("inserts a horizontal rule at the cursor", () => {
    const result = applyMarkdownFormat("before", 6, 6, "horizontal-rule");
    expect(result.value).toBe("before\n---\n");
    expect(result.selectionStart).toBe(result.value.length);
    expect(result.selectionEnd).toBe(result.value.length);
  });
});
