import { useState } from "react";
import { Plus } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { Activity } from "@/src/types";
import { DashboardNotes } from "@/src/components/NotesModule";
import { getTotalAccountBalance } from "@/src/utils/finance";
import { dayKey, money, pad, transactionAmountText } from "@/src/utils/format";
import { PanelHead, StatDisplay, EmptyState } from "@/src/components/common";
import { Button } from "@/src/components/ui";

export default function Dashboard({
  go,
  record,
  openNotes,
}: {
  go: (view: string) => void;
  record: (value: Activity) => void;
  openNotes: (id?: string) => void;
}) {
  const { activities, logs, transactions, accounts, workoutHistory } =
    useLifeStore();
  const [referenceTime] = useState(() => Date.now());
  const today = dayKey();
  const todayLogs = logs.filter((item) => item.createdAt.startsWith(today));
  const done = activities.filter((item) =>
    todayLogs.some(
      (log) => log.activityId === item.id && log.status !== "skipped",
    ),
  ).length;
  const month = today.slice(0, 7);
  const monthExpense = transactions
    .filter((item) => item.type === "expense" && item.occurredAt.startsWith(month))
    .reduce((sum, item) => sum + item.amount, 0);
  const assets = getTotalAccountBalance(accounts, transactions);
  const days = Array.from({ length: 7 }, (_, index) => {
    const date = new Date();
    date.setDate(date.getDate() - (6 - index));
    return date;
  });
  const spend = days.map((date) =>
    transactions
      .filter(
        (item) =>
          item.type === "expense" && item.occurredAt.startsWith(dayKey(date)),
      )
      .reduce((sum, item) => sum + item.amount, 0),
  );
  const max = Math.max(...spend, 1);
  const recentWorkout = workoutHistory[0];
  const weekWorkouts = workoutHistory.filter(
    (item) => referenceTime - new Date(item.occurredAt).getTime() < 7 * 86400000,
  ).length;

  return (
    <div className="hx-view">
      <div className="lt-dash-hero">
        <div>
          <span className="hx-kicker">
            {new Intl.DateTimeFormat("zh-CN", {
              month: "long",
              day: "numeric",
              weekday: "long",
            }).format(new Date())}
          </span>
          <h2>今天完成了 {done} / {activities.length} 项坚持</h2>
          <div className="lt-hero-progress">
            <i>
              <b
                style={{
                  width: `${activities.length ? (done / activities.length) * 100 : 0}%`,
                }}
              />
            </i>
            <span>
              {done} 项完成 · 剩余 {Math.max(activities.length - done, 0)} 项
            </span>
          </div>
        </div>
        <Button variant="primary" icon={<Plus aria-hidden="true" />} onClick={() => go("habits")}>
          管理项目
        </Button>
      </div>

      <div className="hx-metrics">
        <StatDisplay
          label="今日完成"
          value={`${done} 项`}
          sub={`还有 ${Math.max(activities.length - done, 0)} 项等待完成`}
        />
        <StatDisplay
          label="本周训练"
          value={`${weekWorkouts} 次`}
          sub="训练完成后自动同步坚持项目"
          tone="positive"
        />
        <StatDisplay
          label="本月支出"
          value={money(monthExpense)}
          sub={`${transactions.filter((item) => item.occurredAt.startsWith(month)).length} 笔收支记录`}
        />
        <StatDisplay
          label="当前总资产"
          value={money(assets)}
          sub={`${accounts.length} 个账户`}
          tone="positive"
        />
      </div>

      <div className="lt-dash-grid">
        <div className="lt-dash-stack">
          <article className="hx-panel">
            <PanelHead
              kicker="今日"
              title="今天的坚持"
              action="管理项目"
              onClick={() => go("habits")}
            />
            <div className="hx-panel-body hx-list">
              {activities.slice(0, 6).map((activity) => {
                const own = todayLogs.filter(
                  (log) => log.activityId === activity.id,
                );
                const value = own.reduce(
                  (sum, log) =>
                    log.status === "skipped" ? sum : sum + (log.value ?? 1),
                  0,
                );
                return (
                  <div className="hx-row" key={activity.id}>
                    <span className="hx-row-icon">{activity.name.slice(0, 1)}</span>
                    <div>
                      <strong>{activity.name}</strong>
                      <small>
                        {own.length
                          ? `已记录 ${value} ${activity.unit}`
                          : `${activity.targetPeriod === "weekly" ? "每周" : "每天"} · 目标 ${activity.normalTarget ?? 1} ${activity.unit}`}
                      </small>
                    </div>
                    <button
                      className={value > 0 ? "done" : ""}
                      onClick={() => record(activity)}
                    >
                      {value > 0 ? "继续记录" : "记录"}
                    </button>
                  </div>
                );
              })}
              {!activities.length ? (
                <EmptyState
                  title="还没有坚持项目"
                  hint="创建一个项目，从今天开始记录。"
                />
              ) : null}
            </div>
          </article>

          <article className="hx-panel">
            <PanelHead
              kicker="财务"
              title="最近账单"
              action="全部账单"
              onClick={() => go("transactions")}
            />
            <div className="hx-panel-body hx-list">
              {transactions.slice(0, 6).map((item) => (
                <div className="hx-row" key={item.id}>
                  <span className="hx-row-icon">
                    {(item.counterparty || item.category).slice(0, 1)}
                  </span>
                  <div>
                    <strong>{item.counterparty || item.category}</strong>
                    <small>
                      {item.type === "transfer"
                        ? `${item.account} → ${item.toAccount ?? "未匹配账户"}`
                        : `${item.category} · ${new Date(item.occurredAt).toLocaleDateString("zh-CN")}`}
                    </small>
                  </div>
                  <b className={item.type}>{transactionAmountText(item)}</b>
                </div>
              ))}
              {!transactions.length ? (
                <EmptyState title="暂无账单" hint="手动记账或导入账单后会显示在这里。" />
              ) : null}
            </div>
          </article>
        </div>

        <div className="lt-dash-stack">
          <article className="hx-panel">
            <PanelHead
              kicker="财务"
              title="近 7 天支出"
              action="查看分析"
              onClick={() => go("finance")}
            />
            <div className="hx-panel-body">
              <div className="hx-bars">
                {spend.map((value, index) => (
                  <div key={days[index].toISOString()}>
                    <i
                      style={{
                        height: `${Math.max((value / max) * 100, value ? 8 : 2)}%`,
                      }}
                    />
                    <small>
                      {pad(days[index].getMonth() + 1)}-{pad(days[index].getDate())}
                    </small>
                  </div>
                ))}
              </div>
            </div>
          </article>

          <article className="hx-panel">
            <PanelHead
              kicker="训练"
              title="最近训练"
              action="查看训练数据"
              onClick={() => go("fitness")}
            />
            <div className="hx-panel-body">
              {recentWorkout ? (
                <div className="hx-row">
                  <span className="hx-row-icon">训</span>
                  <div>
                    <strong>{recentWorkout.name}</strong>
                    <small>
                      {recentWorkout.exerciseCount} 个动作 · {recentWorkout.setCount} 组 ·{" "}
                      {new Date(recentWorkout.occurredAt).toLocaleDateString("zh-CN")}
                    </small>
                  </div>
                  <button onClick={() => go("fitness")}>查看</button>
                </div>
              ) : (
                <EmptyState
                  title="暂无训练记录"
                  hint="导入训练截图后，这里会显示最近一次训练。"
                />
              )}
            </div>
          </article>

          <DashboardNotes openNotes={openNotes} />
        </div>
      </div>
    </div>
  );
}
