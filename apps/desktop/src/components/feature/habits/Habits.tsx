import { useState } from "react";
import {
  Archive,
  Check,
  NotebookPen,
  Pencil,
  Plus,
} from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { Activity, ActivityLog } from "@/src/types";
import ContextMenu from "@/src/ui/menu/ContextMenu";
import MoreMenu from "@/src/ui/menu/MoreMenu";
import type { AppAction } from "@/src/ui/actions/types";
import { ActivityGlyph, PROJECT_COLORS } from "@/src/components/persist-project/ProjectControls";
import { EmptyState, PanelHead } from "@/src/components/common";
import { Button } from "@/src/components/ui";
import { dayKey } from "@/src/utils/format";

export default function Habits({
  edit,
  record,
  note,
}: {
  edit: (value?: Activity) => void;
  record: (value: Activity) => void;
  note: (value: Activity) => void;
}) {
  const { activities, logs, archiveActivity } = useLifeStore();
  const [filter, setFilter] = useState<"all" | "pending" | "done">("all");
  const today = dayKey();
  const shown = activities.filter(
    (item) =>
      filter === "all" ||
      (filter === "done") ===
        logs.some(
          (log) =>
            log.activityId === item.id &&
            log.createdAt.startsWith(today) &&
            log.status !== "skipped",
        ),
  );

  return (
    <div className="hx-view">
      <div className="hx-toolbar">
        <div className="hx-segmented">
          {(
            [
              ["all", "全部"],
              ["pending", "待完成"],
              ["done", "已完成"],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              className={filter === id ? "active" : ""}
              onClick={() => setFilter(id)}
            >
              {label}
            </button>
          ))}
        </div>
        <Button variant="primary" icon={<Plus aria-hidden="true" />} onClick={() => edit()}>
          创建坚持项目
        </Button>
      </div>

      <div className="hx-habit-list">
        {shown.map((item) => {
          const itemLogs = logs.filter((log) => log.activityId === item.id);
          const todayValue = itemLogs
            .filter((log) => log.createdAt.startsWith(today))
            .reduce(
              (sum, log) =>
                log.status === "skipped" ? sum : sum + (log.value ?? 1),
              0,
            );
          const target = item.normalTarget ?? 1;
          const total = itemLogs.reduce(
            (sum, log) =>
              log.status === "skipped" ? sum : sum + (log.value ?? 1),
            0,
          );
          const projectColor =
            PROJECT_COLORS[item.color ?? "emerald"] ?? PROJECT_COLORS.emerald;
          const actions: AppAction<Activity>[] = [
            {
              id: "record",
              label: todayValue >= target ? "继续记录" : "记录完成",
              icon: Check,
              group: "primary",
              execute: record,
            },
            {
              id: "note",
              label: "添加练习笔记",
              icon: NotebookPen,
              group: "related",
              execute: note,
            },
            {
              id: "edit",
              label: "编辑项目",
              icon: Pencil,
              group: "organize",
              execute: edit,
            },
            {
              id: "archive",
              label: "归档项目",
              icon: Archive,
              group: "organize",
              execute: (context) => archiveActivity(context.id),
            },
          ];
          return (
            <ContextMenu
              as="article"
              className="hx-habit-row"
              style={
                {
                  "--habit-color": projectColor.value,
                  "--habit-soft": projectColor.soft,
                } as React.CSSProperties
              }
              actions={actions}
              context={item}
              ariaLabel={`${item.name}操作`}
              key={item.id}
            >
              <span className="hx-habit-glyph">
                <ActivityGlyph icon={item.icon} />
              </span>
              <div className="hx-habit-copy">
                <h3>{item.name}</h3>
                <p>
                  {item.description ||
                    `${item.targetPeriod === "weekly" ? "每周" : "每天"}目标 ${target} ${item.unit}`}
                </p>
              </div>
              <div className="hx-habit-progress">
                <div>
                  <span>今日进度</span>
                  <strong>
                    {todayValue} / {target} {item.unit}
                  </strong>
                </div>
                <i className="hx-track">
                  <b
                    style={{
                      width: `${Math.min(100, target > 0 ? (todayValue / target) * 100 : 0)}%`,
                    }}
                  />
                </i>
                <small>累计 {total} {item.unit}</small>
              </div>
              <div className="hx-habit-actions">
                <Button
                  variant={todayValue >= target ? "secondary" : "primary"}
                  onClick={() => record(item)}
                >
                  {todayValue >= target ? "继续记录" : "打卡"}
                </Button>
                <MoreMenu
                  actions={actions}
                  context={item}
                  label={`${item.name}更多操作`}
                />
              </div>
            </ContextMenu>
          );
        })}
        {!shown.length ? (
          <EmptyState
            title={activities.length ? "当前筛选下没有项目" : "创建第一个坚持项目"}
            hint={
              activities.length
                ? undefined
                : "设定一个每天或每周的小目标，从今天开始记录。"
            }
            icon={activities.length ? undefined : <Check aria-hidden="true" />}
          />
        ) : null}
      </div>

      <HabitAnalytics activities={activities} logs={logs} />
    </div>
  );
}

function HabitAnalytics({
  activities,
  logs,
}: {
  activities: Activity[];
  logs: ActivityLog[];
}) {
  const [activityId, setActivityId] = useState(activities[0]?.id ?? "");
  const activity =
    activities.find((item) => item.id === activityId) ?? activities[0];
  const days = Array.from({ length: 84 }, (_, index) => {
    const date = new Date();
    date.setHours(12, 0, 0, 0);
    date.setDate(date.getDate() - (83 - index));
    return date;
  });
  const values = days.map((date) =>
    logs
      .filter(
        (item) =>
          item.activityId === activity?.id &&
          item.createdAt.startsWith(dayKey(date)),
      )
      .reduce(
        (sum, item) => (item.status === "skipped" ? sum : sum + (item.value ?? 1)),
        0,
      ),
  );
  const activeDays = values.filter((value) => value > 0).length;
  const total = values.reduce((sum, value) => sum + value, 0);
  const rate = Math.round((activeDays / 84) * 100);

  return (
    <div className="hx-analytics">
      <article className="hx-panel">
        <PanelHead kicker="坚持趋势" title="过去 12 周坚持轨迹" />
        <div className="hx-panel-body">
          <select
            value={activity?.id ?? ""}
            onChange={(event) => setActivityId(event.target.value)}
            aria-label="选择坚持项目"
          >
            {activities.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </select>
          <div className="hx-heatmap">
            {values.map((value, index) => (
              <i
                key={days[index].toISOString()}
                className={
                  value <= 0
                    ? ""
                    : value < (activity?.normalTarget ?? 1) * 0.5
                      ? "l1"
                      : value < (activity?.normalTarget ?? 1)
                        ? "l2"
                        : value < (activity?.normalTarget ?? 1) * 1.5
                          ? "l3"
                          : "l4"
                }
                title={`${dayKey(days[index])} · ${value} ${activity?.unit ?? ""}`}
              />
            ))}
          </div>
          <div className="hx-heat-legend">
            <span>少</span>
            <i />
            <i className="l2" />
            <i className="l3" />
            <i className="l4" />
            <span>多</span>
          </div>
        </div>
      </article>
      <article className="hx-panel hx-insight">
        <span className="hx-kicker">数据洞察</span>
        <div className="hx-ring" style={{ "--p": `${rate}%` } as React.CSSProperties}>
          <strong>{rate}%</strong>
        </div>
        <h3>{activity?.name ?? "坚持项目"}</h3>
        <p>过去 12 周有 {activeDays} 天留下记录。</p>
        <div>
          <span>
            <b>{total}</b>累计 {activity?.unit}
          </span>
          <span>
            <b>{activeDays}</b>活跃天数
          </span>
        </div>
      </article>
    </div>
  );
}
