import { describe, expect, it } from "vitest";
import { amountToCents, formatMoney, normalizeApiBase } from "../core";

describe("reused cloud contract helpers", () => {
  it("converts decimal money without floating point drift", () => {
    expect(amountToCents("12.34")).toBe(1234);
    expect(amountToCents("-1.2")).toBe(-120);
  });
  it("rejects over-precision", () => expect(() => amountToCents("1.234")).toThrow());
  it("supports privacy masking", () => expect(formatMoney(1234, "CNY", true)).toBe("••••"));
  it("normalizes API base URLs", () => expect(normalizeApiBase("https://example.test///")).toBe("https://example.test"));
});
