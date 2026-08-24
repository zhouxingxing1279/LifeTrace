import { describe, expect, it, vi } from "vitest";
import { LifeTraceBeeCountAdapter, beeCountMonthSeries, filterBeeCountTransactions } from "./adapter";
import { BeeCountFinanceApi, type BeeCountLedgerSnapshot, type BeeCountTransaction } from "../../../services/core";

const tx = (id: string, type: string, amount: number, date: string, note = "") => ({
  id, externalTransactionId: id, transactionType: type, amountCents: amount, currency: "CNY", occurredAt: `${date}T12:00:00.000Z`, localDate: date,
  status: "confirmed", sourceType: "beecount-cloud", note, tags: [], tagIds: [], attachments: [], excludeFromStats: false, excludeFromBudget: false, readOnly: true,
}) as BeeCountTransaction;

function snapshot(items: BeeCountTransaction[], total = items.length, offset = 0): BeeCountLedgerSnapshot {
  return {
    source: "beecount-cloud",
    readOnly: true,
    fetchedAt: "2026-08-20T00:00:00.000Z",
    ledger: {
      id: "ledger", sourceId: "bee-ledger", name: "Bee", currency: "CNY", monthStartDay: 1,
      transactionCount: total, incomeTotalCents: 0, expenseTotalCents: 0, balanceCents: 0, readOnly: true,
    },
    transactions: { items, total, limit: 500, offset },
    accounts: [], categories: [], tags: [], budgets: [],
  };
}

describe("BeeCount adapter view logic", () => {
  it("filters transactions by type and query", () => {
    const items = [tx("1", "expense", 1200, "2026-08-01", "coffee"), tx("2", "income", 5000, "2026-08-02", "salary")];
    expect(filterBeeCountTransactions(items, "coffee", "expense")).toHaveLength(1);
    expect(filterBeeCountTransactions(items, "salary", "expense")).toHaveLength(0);
  });

  it("aggregates monthly income and expense", () => {
    const value = snapshot([tx("1", "expense", 1200, "2026-07-01"), tx("2", "income", 5000, "2026-07-02"), tx("3", "expense", 800, "2026-08-01")]);
    expect(beeCountMonthSeries(value)).toEqual([
      { month: "2026-07", income: 5000, expense: 1200 },
      { month: "2026-08", income: 0, expense: 800 },
    ]);
  });

  it("loads every 500-row snapshot page and sorts newest first", async () => {
    const first = Array.from({ length: 500 }, (_, i) => tx(`old-${i}`, "expense", 100, "2026-06-01"));
    const second = Array.from({ length: 500 }, (_, i) => tx(`mid-${i}`, "expense", 100, "2026-07-01"));
    const third = Array.from({ length: 401 }, (_, i) => tx(`new-${i}`, "expense", 100, "2026-08-20"));
    const api = {
      snapshot: vi.fn(async (_ledgerId: string, _limit: number, offset: number) => {
        if (offset === 0) return snapshot(first, 1401, 0);
        if (offset === 500) return snapshot(second, 1401, 500);
        return snapshot(third, 1401, 1000);
      }),
    } as unknown as BeeCountFinanceApi;
    const adapter = new LifeTraceBeeCountAdapter(api);
    const value = await adapter.snapshotAll("bee-ledger");
    expect(api.snapshot).toHaveBeenCalledTimes(3);
    expect(value.transactions.items).toHaveLength(1401);
    expect(value.transactions.items[0].id).toBe("new-0");
    expect(value.transactions.total).toBe(1401);
  });

  it("does not expose legacy LifeTrace-native ledgers to the BeeCount Web port", async () => {
    const bee = { id: "bee", sourceId: "ledger-1", name: "Bee", currency: "CNY", monthStartDay: 1, transactionCount: 5, incomeTotalCents: 0, expenseTotalCents: 0, balanceCents: 0, readOnly: true as const };
    const legacy = { ...bee, id: "legacy", sourceId: "lifetrace:legacy-ledger", name: "Legacy" };
    const api = {
      ledgers: vi.fn(async () => ({ source: "beecount-cloud" as const, readOnly: true as const, items: [legacy, bee], fetchedAt: "2026-08-20T00:00:00.000Z" })),
    } as unknown as BeeCountFinanceApi;
    const adapter = new LifeTraceBeeCountAdapter(api);
    const value = await adapter.ledgers();
    expect(value.items.map((item) => item.sourceId)).toEqual(["ledger-1"]);
  });
});
