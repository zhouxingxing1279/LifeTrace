import { useLifeStore } from "@/src/stores/useLifeStore";
import {
  getAccountBalanceSnapshot,
  getTotalAccountBalance,
} from "@/src/utils/finance";
import { dayKey, money } from "@/src/utils/format";
import { EmptyState, PanelHead, StatDisplay } from "@/src/components/common";

export default function Finance() {
  const { transactions, accounts } = useLifeStore();
  const month = dayKey().slice(0, 7);
  const current = transactions.filter((item) => item.occurredAt.startsWith(month));
  const expense = current
    .filter((item) => item.type === "expense")
    .reduce((sum, item) => sum + item.amount, 0);
  const income = current
    .filter((item) => item.type === "income")
    .reduce((sum, item) => sum + item.amount, 0);
  const categories = Object.entries(
    current
      .filter((item) => item.type === "expense")
      .reduce<Record<string, number>>(
        (all, item) => ({
          ...all,
          [item.category]: (all[item.category] ?? 0) + item.amount,
        }),
        {},
      ),
  )
    .sort((a, b) => b[1] - a[1])
    .slice(0, 7);
  const max = Math.max(...categories.map((item) => item[1]), 1);

  return (
    <div className="hx-view">
      <div className="hx-metrics">
        <StatDisplay
          label="总资产"
          value={money(getTotalAccountBalance(accounts, transactions))}
          sub={`${accounts.length} 个账户`}
          tone="positive"
        />
        <StatDisplay
          label="本月收入"
          value={money(income)}
          sub={`${current.filter((item) => item.type === "income").length} 笔收入`}
          tone="positive"
        />
        <StatDisplay
          label="本月支出"
          value={money(expense)}
          sub={`${current.filter((item) => item.type === "expense").length} 笔支出`}
        />
        <StatDisplay
          label="本月结余"
          value={money(income - expense)}
          sub={
            income
              ? `储蓄率 ${Math.round(((income - expense) / income) * 100)}%`
              : "等待收入数据"
          }
        />
      </div>

      <div className="hx-finance-grid">
        <article className="hx-panel">
          <PanelHead kicker="分类" title="支出分类" />
          <div className="hx-panel-body hx-category-list">
            {categories.length ? (
              categories.map(([name, value]) => (
                <div key={name}>
                  <span>{name}</span>
                  <i>
                    <b style={{ width: `${(value / max) * 100}%` }} />
                  </i>
                  <strong>{money(value)}</strong>
                </div>
              ))
            ) : (
              <EmptyState title="暂无支出分类" hint="记录支出后显示分类结构。" />
            )}
          </div>
        </article>
        <article className="hx-panel">
          <PanelHead kicker="账户" title="资产账户" />
          <div className="hx-panel-body hx-account-mini">
            {accounts.map((item) => {
              const snapshot = getAccountBalanceSnapshot(item, transactions);
              return (
                <div key={item.id}>
                  <i style={{ background: item.color }}>{item.icon}</i>
                  <span>
                    <strong>{item.name}</strong>
                    <small>
                      {snapshot.hasBaseline
                        ? `基准后 ${snapshot.transactionCount} 笔流水`
                        : "尚未设置余额基准时间"}
                    </small>
                  </span>
                  <b>
                    {snapshot.currentBalance === null
                      ? "未设置"
                      : money(snapshot.currentBalance)}
                  </b>
                </div>
              );
            })}
          </div>
        </article>
      </div>
    </div>
  );
}
