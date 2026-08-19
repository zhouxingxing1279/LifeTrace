import { describe, expect, it } from "vitest";
import { beeCountMonthSeries, filterBeeCountTransactions } from "./adapter";
import type { BeeCountLedgerSnapshot, BeeCountTransaction } from "../../../services/core";

const tx = (id: string, type: string, amount: number, date: string, note = "") => ({
  id, externalTransactionId: id, transactionType: type, amountCents: amount, currency: "CNY", occurredAt: `${date}T12:00:00.000Z`, localDate: date,
  status: "confirmed", sourceType: "beecount-cloud", note, tags: [], tagIds: [], attachments: [], excludeFromStats: false, excludeFromBudget: false, readOnly: true,
}) as BeeCountTransaction;

describe("BeeCount adapter view logic", () => {
  it("filters transactions by type and query", () => {
    const items = [tx("1", "expense", 1200, "2026-08-01", "coffee"), tx("2", "income", 5000, "2026-08-02", "salary")];
    expect(filterBeeCountTransactions(items, "coffee", "expense")).toHaveLength(1);
    expect(filterBeeCountTransactions(items, "salary", "expense")).toHaveLength(0);
  });

  it("aggregates monthly income and expense", () => {
    const snapshot = { transactions: { items: [tx("1", "expense", 1200, "2026-07-01"), tx("2", "income", 5000, "2026-07-02"), tx("3", "expense", 800, "2026-08-01")], total: 3, limit: 200, offset: 0 } } as BeeCountLedgerSnapshot;
    expect(beeCountMonthSeries(snapshot)).toEqual([
      { month: "2026-07", income: 5000, expense: 1200 },
      { month: "2026-08", income: 0, expense: 800 },
    ]);
  });
});
