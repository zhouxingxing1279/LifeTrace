import type { ReactNode } from "react";
import {
  ArrowRight, BookOpen, CircleDollarSign, Dumbbell, ListTodo, NotebookPen, ShieldCheck, Sparkles, Target,
} from "lucide-react";
import { formatMoney, taskMatchesToday, type CloudState, type JsonEntity } from "../core";
import { Empty, Metric, MetricGrid, Panel, Toolbar, entities, number, text } from "../ui";
import { navigate } from "../navigation";

interface DashboardPageProps {
  state: CloudState;
  privacy: boolean;
}

export function DashboardPage({ state, privacy }: DashboardPageProps) {
  const activities = entities(state, "habit.activity").filter((item) => item.isArchived !== true);
  const logs = entities(state, "habit.log");
  const tasks = entities(state, "execution.task");
  const projects = entities(state, "execution.project").filter((item) => item.status !== "archived" && item.status !== "cancelled");
  const memos = entities(state, "execution.memo").filter((item) => item.status !== "archived");
  const transactions = entities(state, "finance.transaction");
  const accounts = entities(state, "finance.account").filter((item) => item.isArchived !== true);
  const workouts = entities(state, "workout.workout");
  const notes = entities(state, "note.note").filter((item) => item.isArchived !== true);
  const records = entities(state, "english.learning_record");
  const reviews = entities(state, "review.daily");
  const today = new Date();
  const todayKey = localDay(today);
  const month = todayKey.slice(0, 7);
  const todayTasks = tasks.filter((item) => taskMatchesToday(item, todayKey));
  const todayOpenTasks = todayTasks.filter((item) => item.status !== "done" && item.status !== "cancelled");
  const todayDoneTasks = todayTasks.filter((item) => item.status === "done");
  const inboxTasks = tasks.filter((item) => item.status !== "done" && item.status !== "cancelled" && (item.context === "inbox" || (!item.projectId && !item.dueAt && !item.scheduledStartAt)));
  const todayLogs = logs.filter((item) => text(item, "logDate") === todayKey && item.status !== "skipped");
  const completedIds = new Set(todayLogs.map((item) => text(item, "activityId")));
  const completedHabits = completedIds.size;
  const habitRemaining = Math.max(activities.length - completedHabits, 0);
  const actionTotal = todayTasks.length + activities.length;
  const actionCompleted = todayDoneTasks.length + completedHabits;
  const completion = actionTotal ? Math.round((actionCompleted / actionTotal) * 100) : 0;
  const monthTransactions = transactions.filter((item) => text(item, "localDate").startsWith(month));
  const monthExpense = monthTransactions
    .filter((item) => ["expense", "fee"].includes(text(item, "transactionType")) && text(item, "status") !== "ignored")
    .reduce((sum, item) => sum + number(item, "amountCents"), 0);
  const assets = accounts.reduce((sum, item) => sum + number(item, "openingBalanceCents"), 0);
  const weekWorkouts = workouts.filter((item) => Date.now() - new Date(text(item, "occurredAt")).getTime() < 7 * 86400000);
  const timeline = buildTimeline(state).slice(0, 10);
  const pinnedNotes = notes.filter((item) => item.isPinned === true).length;

  return <div className="lt-dashboard lt-page-stack">
    <section className="lt-dashboard-focus" aria-labelledby="dashboard-focus-title">
      <div className="lt-dashboard-focus-copy">
        <span className="lt-overline"><Sparkles /> TODAY</span>
        <h2 id="dashboard-focus-title">今天只看真正重要的事。</h2>
        <p>{actionTotal
          ? `今天安排了 ${todayTasks.length} 个任务和 ${activities.length} 个坚持项目，已经完成 ${actionCompleted} 项。临时想到的事情先进入收件箱，真正要做的再安排到今天。`
          : "先把脑中的事情快速收集到任务或备忘，再从收件箱安排今天。训练、学习、财务和复盘会继续自动汇总到这里。"}</p>
        <Toolbar className="lt-dashboard-actions">
          <button className="hx-btn primary" onClick={() => navigate("/execution")}><ListTodo />打开今日计划</button>
          <button className="hx-btn secondary" onClick={() => navigate("/habits")}><Target />记录坚持</button>
          <button className="hx-btn ghost" onClick={() => navigate("/assistant")}>问 AI 管家<ArrowRight /></button>
        </Toolbar>
      </div>
      <div className="lt-completion-card" aria-label={`今日行动完成 ${completion}%`}>
        <span>今日行动完成度</span>
        <strong>{completion}%</strong>
        <div className="lt-progress"><i style={{ width: `${completion}%` }} /></div>
        <small>{actionCompleted} / {actionTotal} 项行动</small>
      </div>
    </section>

    <MetricGrid>
      <Metric label="今日待办" value={`${todayDoneTasks.length} / ${todayTasks.length}`} detail={todayOpenTasks.length ? `还有 ${todayOpenTasks.length} 项` : todayTasks.length ? "今天已清空" : `${inboxTasks.length} 项等待安排`} positive={todayTasks.length > 0 && todayOpenTasks.length === 0} />
      <Metric label="今日坚持" value={`${completedHabits} / ${activities.length}`} detail={habitRemaining ? `还有 ${habitRemaining} 项` : activities.length ? "今天已全部完成" : "等待创建项目"} positive={habitRemaining === 0 && activities.length > 0} />
      <Metric label="近 7 天训练" value={`${weekWorkouts.length} 次`} detail="来自云端训练记录" positive={weekWorkouts.length > 0} />
      <Metric label="本月支出" value={formatMoney(monthExpense, "CNY", privacy)} detail={`${monthTransactions.length} 笔本月流水`} />
    </MetricGrid>

    <div className="lt-dashboard-layout">
      <div className="lt-dashboard-main">
        <Panel eyebrow="TODAY" title="今天的行动" actions={<button className="hx-btn ghost sm" onClick={() => navigate("/execution")}>管理计划<ArrowRight /></button>}>
          <div className="lt-action-list">
            {todayOpenTasks.slice(0, 6).map((task) => <button className="lt-action-row" key={task.meta.id} onClick={() => navigate("/execution")}>
              <span className="lt-action-check" />
              <div><strong>{text(task, "title")}</strong><small>{text(task, "description") || `${priorityLabel(text(task, "priority"))}优先级 · ${text(task, "dueAt") ? formatSchedule(text(task, "dueAt")) : "今天"}`}</small></div>
              <b>{task.status === "in_progress" ? "进行中" : "待完成"}</b>
            </button>)}
            {activities.slice(0, Math.max(0, 8 - todayOpenTasks.length)).map((activity) => {
              const done = completedIds.has(activity.meta.id);
              return <button className="lt-action-row" key={activity.meta.id} onClick={() => navigate("/habits")}>
                <span className={`lt-action-check ${done ? "done" : ""}`}>{done ? "✓" : ""}</span>
                <div><strong>{text(activity, "name")}</strong><small>{done ? "今天已记录" : `目标 ${number(activity, "normalTarget") || 1} ${text(activity, "unit")}`}</small></div>
                <b>{done ? "完成" : "坚持"}</b>
              </button>;
            })}
            {!todayOpenTasks.length && !activities.length && <Empty title="还没有今日行动" description="先在“计划与待办”里快速收集一件事，再把它安排到今天。" />}
          </div>
        </Panel>

        <Panel eyebrow="TIMELINE" title="最近动态" actions={<button className="hx-btn ghost sm" onClick={() => navigate("/calendar")}>打开日历<ArrowRight /></button>}>
          <div className="lt-timeline">
            {timeline.map((item) => <article className="lt-timeline-row" key={`${item.type}-${item.id}`}>
              <span data-kind={item.type}>{item.type.slice(0, 1)}</span>
              <div><strong>{item.title}</strong><small>{item.type} · {formatRelativeTime(item.updatedAt)}</small></div>
            </article>)}
            {!timeline.length && <Empty title="暂无动态" description="新增任务、备忘、坚持、训练、账单、笔记、英语或复盘记录后，这里会形成统一时间线。" />}
          </div>
        </Panel>
      </div>

      <aside className="lt-dashboard-rail" aria-label="今日摘要">
        <Panel eyebrow="EXECUTION" title="计划执行">
          <SummaryRow icon={<ListTodo />} label="任务收件箱" value={`${inboxTasks.length} 项`} detail="待整理" route="/execution" />
          <SummaryRow icon={<Target />} label="活跃计划" value={`${projects.length} 个`} route="/execution" />
          <SummaryRow icon={<NotebookPen />} label="快速备忘" value={`${memos.length} 条`} route="/execution" />
        </Panel>

        <Panel eyebrow="HEALTH" title="训练与健康">
          <SummaryRow icon={<Dumbbell />} label="近 7 天训练" value={`${weekWorkouts.length} 次`} route="/fitness" />
          <SummaryRow icon={<Target />} label="坚持完成" value={`${activities.length ? Math.round((completedHabits / activities.length) * 100) : 0}%`} route="/habits" />
        </Panel>

        <Panel eyebrow="KNOWLEDGE" title="知识与学习">
          <SummaryRow icon={<NotebookPen />} label="笔记" value={`${notes.length} 条`} detail={`${pinnedNotes} 条置顶`} route="/notes" />
          <SummaryRow icon={<BookOpen />} label="英语阅读" value={`${records.length} 次`} route="/english/articles" />
          <SummaryRow icon={<Sparkles />} label="每日复盘" value={`${reviews.length} 天`} route="/review" />
        </Panel>

        <Panel eyebrow="FINANCE" title="财务快照">
          <SummaryRow icon={<CircleDollarSign />} label="本月支出" value={formatMoney(monthExpense, "CNY", privacy)} route="/finance" />
          <p className="lt-panel-note">账户基准资产 {formatMoney(assets, "CNY", privacy)}，财务详情继续集中在财务中心。</p>
        </Panel>

        <Panel eyebrow="CLOUD" title="数据状态">
          <div className="lt-cloud-summary"><ShieldCheck /><div><strong>{state.lastLoadedAt ? "云端快照已加载" : "等待云端数据"}</strong><small>{state.lastLoadedAt ? `最后加载 ${formatRelativeTime(state.lastLoadedAt)}` : "登录后自动加载"}</small></div></div>
          <p className="lt-panel-note">任务、计划、备忘与完成历史使用 LifeTrace Cloud 通用同步协议；相册和本地密钥仍只属于桌面端。</p>
        </Panel>
      </aside>
    </div>
  </div>;
}

function SummaryRow({ icon, label, value, detail, route }: { icon: ReactNode; label: string; value: string; detail?: string; route: Parameters<typeof navigate>[0] }) {
  return <button className="lt-summary-row" onClick={() => navigate(route)}>
    <span>{icon}</span><div><strong>{label}</strong><small>{detail || "查看详情"}</small></div><b>{value}</b><ArrowRight />
  </button>;
}

function buildTimeline(state: CloudState): Array<{ id: string; type: string; title: string; updatedAt: string }> {
  const values: Array<{ id: string; type: string; title: string; updatedAt: string }> = [];
  const add = (items: JsonEntity[], type: string, title: (item: JsonEntity) => string) => items.forEach((item) => values.push({ id: item.meta.id, type, title: title(item), updatedAt: item.meta.updatedAt }));
  add(entities(state, "execution.task"), "任务", (item) => text(item, "title") || "任务");
  add(entities(state, "execution.memo"), "备忘", (item) => text(item, "plainText").slice(0, 80) || "备忘");
  add(entities(state, "habit.log"), "坚持", () => "完成坚持记录");
  add(entities(state, "workout.workout"), "训练", (item) => text(item, "name") || "训练记录");
  add(entities(state, "finance.transaction"), "财务", (item) => text(item, "merchant") || text(item, "counterparty") || "财务流水");
  add(entities(state, "note.note"), "笔记", (item) => text(item, "title") || "无标题笔记");
  add(entities(state, "english.learning_record"), "英语", () => "完成英语阅读");
  add(entities(state, "review.daily"), "复盘", (item) => `${text(item, "reviewDate")} 每日复盘`);
  return values.sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}

function priorityLabel(priority: string): string {
  return ({ urgent: "紧急", high: "高", normal: "普通", low: "低" } as Record<string, string>)[priority] ?? "普通";
}

function formatSchedule(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" });
}

function localDay(value: Date): string {
  return `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, "0")}-${String(value.getDate()).padStart(2, "0")}`;
}

function formatRelativeTime(value: string): string {
  const time = new Date(value).getTime();
  const diff = Date.now() - time;
  if (!Number.isFinite(time)) return value;
  if (diff < 60_000) return "刚刚";
  if (diff < 3_600_000) return `${Math.max(1, Math.floor(diff / 60_000))} 分钟前`;
  if (diff < 86_400_000) return `${Math.max(1, Math.floor(diff / 3_600_000))} 小时前`;
  if (diff < 7 * 86_400_000) return `${Math.max(1, Math.floor(diff / 86_400_000))} 天前`;
  return new Date(value).toLocaleDateString("zh-CN", { month: "short", day: "numeric" });
}
