/*
 * BeeCount Cloud adapter for LifeTrace Web.
 * Upstream information architecture and behavior are derived from
 * TNT-Likely/BeeCount-Cloud frontend/apps/web/src/pages/sections/*.
 * The LifeTrace backend exposes a read-only aggregate snapshot, so this adapter
 * translates that contract without introducing a second BeeCount login flow.
 */
import { BeeCountFinanceApi, type BeeCountLedgerSnapshot, type BeeCountTransaction } from "../../../services/core";

export class LifeTraceBeeCountAdapter {
  constructor(private readonly api = new BeeCountFinanceApi()) {}
  status() { return this.api.status(); }
  ledgers() { return this.api.ledgers(); }
  snapshot(ledgerId: string, limit = 200, offset = 0) { return this.api.snapshot(ledgerId, limit, offset); }
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
