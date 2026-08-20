/*
 * BeeCount Cloud adapter for LifeTrace Web.
 *
 * The visible finance experience is source-derived from TNT-Likely/BeeCount-Cloud
 * (upstream SHA tracked in UPSTREAM.md). LifeTrace only adapts authentication and
 * the PostgreSQL-backed compatibility API; it must not reintroduce a second
 * finance model.
 */
import { BeeCountFinanceApi, type BeeCountLedgerSnapshot, type BeeCountTransaction } from "../../../services/core";

const SNAPSHOT_PAGE_SIZE = 500;

export class LifeTraceBeeCountAdapter {
  constructor(private readonly api = new BeeCountFinanceApi()) {}

  status() { return this.api.status(); }
  ledgers() { return this.api.ledgers(); }
  snapshot(ledgerId: string, limit = 200, offset = 0) { return this.api.snapshot(ledgerId, limit, offset); }

  /**
   * BeeCount Cloud's transaction page is server paginated. LifeTrace's aggregate
   * snapshot endpoint has the same hard page limit (500), so the Web port must
   * consume every page before running BeeCount's client-side analytics and
   * dictionary joins. This prevents the old 500-row truncation bug.
   */
  async snapshotAll(ledgerId: string): Promise<BeeCountLedgerSnapshot> {
    const first = await this.api.snapshot(ledgerId, SNAPSHOT_PAGE_SIZE, 0);
    const total = Math.max(0, first.transactions.total);
    if (first.transactions.items.length >= total) {
      return withSortedTransactions(first, first.transactions.items);
    }

    const offsets: number[] = [];
    for (let offset = SNAPSHOT_PAGE_SIZE; offset < total; offset += SNAPSHOT_PAGE_SIZE) {
      offsets.push(offset);
    }
    const pages = await Promise.all(
      offsets.map((offset) => this.api.snapshot(ledgerId, SNAPSHOT_PAGE_SIZE, offset)),
    );

    const byId = new Map<string, BeeCountTransaction>();
    for (const item of first.transactions.items) byId.set(item.id, item);
    for (const page of pages) {
      for (const item of page.transactions.items) byId.set(item.id, item);
    }
    return withSortedTransactions(first, [...byId.values()]);
  }
}

function transactionTime(item: BeeCountTransaction): number {
  const parsed = Date.parse(item.occurredAt);
  return Number.isFinite(parsed) ? parsed : 0;
}

function withSortedTransactions(
  snapshot: BeeCountLedgerSnapshot,
  items: BeeCountTransaction[],
): BeeCountLedgerSnapshot {
  const sorted = [...items].sort((a, b) => transactionTime(b) - transactionTime(a));
  return {
    ...snapshot,
    transactions: {
      ...snapshot.transactions,
      items: sorted,
      total: Math.max(snapshot.transactions.total, sorted.length),
      limit: sorted.length,
      offset: 0,
    },
  };
}

export function filterBeeCountTransactions(items: BeeCountTransaction[], query: string, type: string): BeeCountTransaction[] {
  const needle = query.trim().toLocaleLowerCase("zh-CN");
  return items.filter((item) => {
    if (type !== "all" && item.transactionType !== type) return false;
    if (!needle) return true;
    return [item.note, item.accountName, item.fromAccountName, item.toAccountName, item.categoryName, item.localDate, ...item.tags]
      .filter(Boolean).join(" ").toLocaleLowerCase("zh-CN").includes(needle);
  });
}

export function beeCountMonthSeries(snapshot: BeeCountLedgerSnapshot) {
  const buckets = new Map<string, { month: string; income: number; expense: number }>();
  for (const item of snapshot.transactions.items) {
    const month = (item.localDate || item.occurredAt.slice(0, 10)).slice(0, 7);
    const current = buckets.get(month) ?? { month, income: 0, expense: 0 };
    if (item.transactionType === "income") current.income += item.amountCents;
    else if (["expense", "fee"].includes(item.transactionType)) current.expense += item.amountCents;
    buckets.set(month, current);
  }
  return [...buckets.values()].sort((a, b) => a.month.localeCompare(b.month)).slice(-12);
}
