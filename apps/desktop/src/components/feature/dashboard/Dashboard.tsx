import { useMemo, useState } from "react";
import { Check, ChevronRight, NotebookPen, Plus } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { Activity } from "@/src/types";
import { getTotalAccountBalance } from "@/src/utils/finance";
import { dayKey, money, transactionAmountText } from "@/src/utils/format";
import { EmptyState } from "@/src/components/common";
import { Button } from "@/src/components/ui";

type TodayEvent = {
  id: string;
  time: string;
  title: string;
  detail: string;
  kind: "habit" | "finance" | "workout";
};

export default function Dashboard({
  go,
  record,
  openNotes,
}: {
  go: (view: string) => void;
  record: (value: Activity) => void;
  openNotes: (id?: string) => void;
}) {
  const { activities, logs, transactions, accounts, workoutHistory } = useLifeStore();
  const [referenceTime] = useState(() => Date.now());
  const today = dayKey();
  const todayLogs = logs.filter((item) => item.createdAt.startsWith(today));
  const doneIds = new Set(
    todayLogs.filter((log) => log.status !== "skipped").map((log) => log.activityId),
  );
  const pending = activities.filter((item) => !doneIds.has(item.id));
  const done = activities.length - pending.length;
  const month = today.slice(0, 7);
  const monthExpense = transactions
    .filter((item) => item.type === "expense" && item.occurredAt.startsWith(month))
    .reduce((sum, item) => sum + item.amount, 0);
  const assets = getTotalAccountBalance(accounts, transactions);
  const weekWorkouts = workoutHistory.filter(
    (item) => referenceTime - new Date(item.occurredAt).getTime() < 7 * 86400000,
  ).length;

  const events = useMemo<TodayEvent[]>(() => {
    const activityById = new Map(activities.map((item) => [item.id, item]));
    const habitEvents = todayLogs
      .filter((log) => log.status !== "skipped")
      .map((log) => {
        const activity = activityById.get(log.activityId);
        return {
          id: `habit-${log.id}`,
          time: log.createdAt,
          title: activity?.name ?? "坚持记录",
          detail: log.value
            ? `记录 ${log.value} ${activity?.unit ?? ""}`.trim()
            : "已完成",
          kind: "habit" as const,
        };
      });
    const financeEvents = transactions
      .filter((item) => item.occurredAt.startsWith(today))
      .map((item) => ({
        id: `finance-${item.id}`,
        time: item.occurredAt,
        title: item.counterparty || item.category,
        detail: `${item.category} · ${transactionAmountText(item)}`,
        kind: "finance" as const,
      }));
    const workoutEvents = workoutHistory
      .filter((item) => item.occurredAt.startsWith(today))
      .map((item) => ({
        id: `workout-${item.id}`,
        time: item.occurredAt,
        title: item.name,
        detail: `${item.exerciseCount} 个动作 · ${item.setCount} 组`,
        kind: "workout" as const,
      }));
    return [...habitEvents, ...financeEvents, ...workoutEvents].sort(
      (left, right) => new Date(right.time).getTime() - new Date(left.time).getTime(),
    );
  }, [activities, todayLogs, transactions, workoutHistory, today]);

  return (
    <div className="hx-view lt-today-view">
      <div className="lt-today-commandbar">
        <div>
          <strong>
            {new Intl.DateTimeFormat("zh-CN", {
              month: "long",
              day: "numeric",
              weekday: "long",
            }).format(new Date())}
          </strong>
          <span>
            {activities.length
              ? `${done}/${activities.length} 项完成 · ${pending.length ? `还有 ${pending.length} 项` : "今日目标已完成"}`
              : "从一个小目标开始今天"}
          </span>
        </div>
        <div className="lt-today-actions">
          <Button variant="ghost" icon={<NotebookPen aria-hidden="true" />} onClick={() => openNotes()}>
            记笔记
          </Button>
          <Button variant="primary" icon={<Plus aria-hidden="true" />} onClick={() => go("habits")}>
            管理坚持
          </Button>
        </div>
      </div>

      <div className="lt-today-summary" aria-label="今日摘要">
        <button type="button" onClick={() => go("habits")}>
          <span>今日坚持</span><b>{done}/{activities.length}</b>
        </button>
        <button type="button" onClick={() => go("fitness")}>
          <span>本周训练</span><b>{weekWorkouts} 次</b>
        </button>
        <button type="button" onClick={() => go("transactions")}>
          <span>本月支出</span><b>{money(monthExpense)}</b>
        </button>
        <button type="button" onClick={() => go("accounts")}>
          <span>总资产</span><b>{money(assets)}</b>
        </button>
      </div>

      <div className="lt-today-workspace">
        <section className="lt-workspace-pane lt-today-tasks" aria-labelledby="today-tasks-title">
          <header className="lt-workspace-head">
            <div>
              <h2 id="today-tasks-title">待完成</h2>
              <span>{pending.length ? `${pending.length} 项` : "已清空"}</span>
            </div>
            <button type="button" onClick={() => go("habits")}>管理</button>
          </header>
          <div className="lt-workspace-list">
            {pending.map((activity) => (
              <button
                type="button"
                className="lt-today-task-row"
                key={activity.id}
                onClick={() => record(activity)}
              >
                <span className="lt-task-check" aria-hidden="true" />
                <span>
                  <strong>{activity.name}</strong>
                  <small>
                    {activity.targetPeriod === "weekly" ? "每周" : "每天"} · 目标 {activity.normalTarget ?? 1} {activity.unit}
                  </small>
                </span>
                <span className="lt-row-action">记录 <ChevronRight aria-hidden="true" /></span>
              </button>
            ))}
            {activities.length > 0 && !pending.length ? (
              <div className="lt-today-done-state">
                <Check aria-hidden="true" />
                <strong>今天的目标已经完成</strong>
                <span>可以继续记录，或者去做一点别的事。</span>
              </div>
            ) : null}
            {!activities.length ? (
              <EmptyState title="还没有坚持项目" hint="创建一个项目，从今天开始记录。" />
            ) : null}
          </div>
        </section>

        <section className="lt-workspace-pane lt-today-stream" aria-labelledby="today-stream-title">
          <header className="lt-workspace-head">
            <div>
              <h2 id="today-stream-title">今天的动态</h2>
              <span>{events.length ? `${events.length} 条` : "暂无记录"}</span>
            </div>
          </header>
          <div className="lt-activity-stream">
            {events.slice(0, 12).map((event) => (
              <div className="lt-stream-row" key={event.id}>
                <time>
                  {new Date(event.time).toLocaleTimeString("zh-CN", {
                    hour: "2-digit",
                    minute: "2-digit",
                  })}
                </time>
                <i className={`lt-stream-dot ${event.kind}`} aria-hidden="true" />
                <div>
                  <strong>{event.title}</strong>
                  <small>{event.detail}</small>
                </div>
              </div>
            ))}
            {!events.length ? (
              <div className="lt-stream-empty">今天还没有动态。完成坚持、记账或训练后会出现在这里。</div>
            ) : null}
          </div>
        </section>
      </div>
    </div>
  );
}
