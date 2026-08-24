import { describe, expect, it } from "vitest";
import { dateKey, recentDays, sum, text } from "./entities";
import type { JsonEntity } from "../services/core";

const entity = { meta: { id: "1", userId: "u", createdAt: "2026-08-19T00:00:00.000Z", updatedAt: "2026-08-19T00:00:00.000Z", localVersion: 1 }, title: "Hello" } as JsonEntity;

describe("entity view helpers", () => {
  it("reads typed text safely", () => expect(text(entity, "title")).toBe("Hello"));
  it("sums numeric series", () => expect(sum([1, 2, 3, 4])).toBe(10));
  it("normalizes ISO dates", () => expect(dateKey("2026-08-19T12:00:00+08:00")).toMatch(/^2026-08-19$/));
  it("creates a chronological recent-day window", () => {
    const days = recentDays(7);
    expect(days).toHaveLength(7);
    expect(days).toEqual([...days].sort());
  });
});
