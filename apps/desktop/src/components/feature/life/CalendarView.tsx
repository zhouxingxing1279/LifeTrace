import { useState } from "react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import { dayKey, money, pad } from "@/src/utils/format";
import { EmptyState, PanelHead, StatDisplay } from "@/src/components/common";

export default function CalendarView() {
  const { activities, logs, transactions, reviews } = useLifeStore();
  const now = new Date();
  const [selected, setSelected] = useState(now.getDate());
  const first = (new Date(now.getFullYear(), now.getMonth(), 1).getDay() + 6) % 7;
  const count = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate();
  const key = (day: number) =>
    `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(day)}`;
  const matchesDay = (value: string, date: string) =>
    dayKey(new Date(value)) === date;
  const selectedKey = key(selected);
  const selectedLogs = logs
    .filter((item) => matchesDay(item.createdAt, selectedKey))
    .sort(
      (left, right) =>
        new Date(right.createdAt).getTime() - new Date(left.createdAt).getTime(),
    );
  const selectedTx = transactions.filter((item) =>
    matchesDay(item.occurredAt, selectedKey),
  );
  const review = reviews.find((item) => item.reviewDate === selectedKey);

  return (
    <div className="hx-view">
      <div className="hx-calendar">
        <div className="hx-week">
          {"一二三四五六日".split("").map((item) => (
            <span key={item}>周{item}</span>
          ))}
        </div>
        <div className="hx-days">
          {Array.from({ length: first }).map((_, i) => (
            <i key={i} />
          ))}
          {Array.from({ length: count }, (_, i) => i + 1).map((day) => (
            <button
              className={selected === day ? "selected" : ""}
              onClick={() => setSelected(day)}
              key={day}
            >
              <b>{day}</b>
              <span>
                {logs.some((item) => matchesDay(item.createdAt, key(day))) ? (
                  <i />
                ) : null}
                {transactions.some((item) =>
                  matchesDay(item.occurredAt, key(day)),
                ) ? (
                  <i />
                ) : null}
                {reviews.some((item) => item.reviewDate === key(day)) ? (
                  <i />
                ) : null}
              </span>
            </button>
          ))}
        </div>
      </div>

      <article className="hx-panel hx-day-detail">
        <PanelHead
          kicker={`${now.getMonth() + 1}月${selected}日`}
          title="当天详情"
        />
        <div className="hx-panel-body">
          <div className="hx-metrics">
            <StatDisplay
              label="项目记录"
              value={`${selectedLogs.length}`}
              sub={`${activities.filter((a) => selectedLogs.some((l) => l.activityId === a.id)).length} 个项目`}
            />
            <StatDisplay
              label="当日支出"
              value={money(
                selectedTx
                  .filter((i) => i.type === "expense")
                  .reduce((s, i) => s + i.amount, 0),
              )}
              sub={`${selectedTx.length} 笔收支`}
            />
            <StatDisplay
              label="每日复盘"
              value={review ? "已完成" : "—"}
              sub={review?.tomorrowPriority ? `明日：${review.tomorrowPriority}` : "尚未填写"}
            />
          </div>
          <section className="hx-day-log">
            <header>
              <div>
                <span className="hx-kicker">生活日志</span>
                <h3>当天做了什么</h3>
              </div>
              <small>{selectedLogs.length} 条记录</small>
            </header>
            {selectedLogs.length ? (
              selectedLogs.map((log) => {
                const activity = activities.find(
                  (item) => item.id === log.activityId,
                );
                return (
                  <article key={log.id}>
                    <time>
                      {new Date(log.createdAt).toLocaleTimeString("zh-CN", {
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </time>
                    <div>
                      <strong>{activity?.name ?? "生活记录"}</strong>
                      <p>
                        {log.note ||
                          `${log.status === "partial" ? "部分完成" : "完成"} ${log.value ?? 1} ${activity?.unit ?? "次"}`}
                      </p>
                    </div>
                    <span className={log.status === "partial" ? "partial" : ""}>
                      {log.status === "partial" ? "部分完成" : "已完成"}
                    </span>
                  </article>
                );
              })
            ) : (
              <EmptyState title="当天还没有生活日志" />
            )}
          </section>
        </div>
      </article>
    </div>
  );
}
