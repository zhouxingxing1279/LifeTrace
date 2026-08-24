import { useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis } from "recharts";
import { ArrowRight, CheckCircle2, Dumbbell, Flame, WalletCards } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Button, Card, CardContent, EmptyState, MetricCard, PageHeader, Progress, Section } from "../../components/ui";
import { entities, number, recentDays, sum, text, todayKey } from "../../lib/entities";
import { formatMoney, isOpenExecutionTask, taskMatchesToday } from "../../services/core";

export function TodayPage() {
  const { state, privacy } = useApp();
  const navigate = useNavigate();
  const today = todayKey();
  const tasks = entities(state, "execution.task");
  const openTasks = tasks.filter(isOpenExecutionTask);
  const todayTasks = openTasks.filter((task) => taskMatchesToday(task, today));
  const completedToday = tasks.filter((task) => text(task, "status") === "done" && text(task, "completedAt").slice(0, 10) === today);
  const habits = entities(state, "habit.activity").filter((item) => !item.isArchived);
  const habitLogs = entities(state, "habit.log");
  const todayHabitLogs = habitLogs.filter((item) => text(item, "logDate") === today && text(item, "status") === "completed");
  const workouts = entities(state, "workout.workout");
  const finance = entities(state, "finance.transaction");
  const english = entities(state, "english.learning_record");
  const reviews = entities(state, "review.daily");

  const days = recentDays(7);
  const trend = useMemo(() => days.map((day) => ({
    day: day.slice(5),
    actions: tasks.filter((item) => text(item, "completedAt").slice(0, 10) === day).length
      + habitLogs.filter((item) => text(item, "logDate") === day && text(item, "status") === "completed").length
      + workouts.filter((item) => text(item, "localDate") === day).length
      + english.filter((item) => text(item, "recordDate") === day).length,
  })), [days.join("|"), english, habitLogs, tasks, workouts]);

  const month = today.slice(0, 7);
  const monthTx = finance.filter((item) => text(item, "localDate").startsWith(month) && text(item, "status", "confirmed") === "confirmed");
  const expenses = sum(monthTx.filter((item) => ["expense", "fee"].includes(text(item, "transactionType"))).map((item) => number(item, "amountCents")));
  const habitProgress = habits.length ? Math.round(todayHabitLogs.length / habits.length * 100) : 0;
  const latestReview = reviews.sort((a, b) => text(b, "reviewDate").localeCompare(text(a, "reviewDate")))[0];

  return <div className="page-shell">
    <PageHeader title="今天" description="只保留今天真正需要关注的事项：优先级、完成度、下一步和异常趋势。" action={<Button onClick={() => navigate("/app/execution?new=task")}>新建任务</Button>} />

    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      <MetricCard label="今日任务" value={`${completedToday.length}/${todayTasks.length + completedToday.length}`} hint={todayTasks.length ? `还有 ${todayTasks.length} 项待完成` : "今日任务已清空"} icon={<CheckCircle2 size={17} />} />
      <MetricCard label="坚持完成" value={`${habitProgress}%`} hint={`${todayHabitLogs.length} / ${habits.length || 0} 个项目`} icon={<Flame size={17} />} />
      <MetricCard label="本月支出" value={formatMoney(expenses, "CNY", privacy)} hint={`${monthTx.length} 笔已确认交易`} icon={<WalletCards size={17} />} />
      <MetricCard label="本周训练" value={`${workouts.filter((item) => days.includes(text(item, "localDate"))).length} 次`} hint="训练记录自动汇入趋势" icon={<Dumbbell size={17} />} />
    </div>

    <div className="mt-6 grid gap-5 xl:grid-cols-[minmax(0,1.35fr)_minmax(320px,.65fr)]">
      <Section title="今日焦点" description="Primary reference: Shadcnblocks Dashboard · Catalyst composition">
        <Card>
          <CardContent className="pt-5">
            {todayTasks.length ? <div className="space-y-2">{todayTasks.slice(0, 6).map((task, index) => <button key={task.meta.id} onClick={() => navigate("/app/execution")} className="flex w-full items-center gap-3 rounded-md border px-3 py-3 text-left hover:bg-muted/50">
              <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-muted text-xs font-semibold">{index + 1}</div>
              <div className="min-w-0 flex-1"><div className="truncate text-sm font-medium">{text(task, "title", "未命名任务")}</div><div className="mt-0.5 text-xs text-muted-foreground">{text(task, "priority", "normal")} · {number(task, "estimatedMinutes") ? `${number(task, "estimatedMinutes")} 分钟` : "未估时"}</div></div>
              <ArrowRight size={15} className="text-muted-foreground" />
            </button>)}</div> : <EmptyState title="今天没有待办" description="可以从计划页安排任务，或把 Inbox 中的事项拖到今天。" action={<Button variant="outline" onClick={() => navigate("/app/execution")}>打开计划</Button>} />}
          </CardContent>
        </Card>
      </Section>

      <Section title="今日节奏" description="把行动完成度和当天状态放在同一视图。">
        <Card><CardContent className="space-y-5 pt-5">
          <div><div className="mb-2 flex justify-between text-xs"><span className="font-medium">坚持完成度</span><span className="text-muted-foreground">{habitProgress}%</span></div><Progress value={habitProgress} /></div>
          <div><div className="eyebrow">最近复盘</div><div className="mt-2 text-sm font-medium">{latestReview ? text(latestReview, "bestThing", "已完成复盘") : "今天还没有复盘"}</div><p className="mt-1 text-xs leading-5 text-muted-foreground">{latestReview ? text(latestReview, "tomorrowPriority", "查看复盘详情") : "晚上用两分钟记录能量、心情和明日重点。"}</p></div>
          <Button variant="outline" className="w-full" onClick={() => navigate("/app/review")}>打开复盘</Button>
        </CardContent></Card>
      </Section>
    </div>

    <Section className="mt-6" title="7 天行动趋势" description="Data reference: Tremor compact analytics pattern">
      <Card><CardContent className="pt-5"><div className="h-56 w-full"><ResponsiveContainer width="100%" height="100%"><AreaChart data={trend} margin={{ top: 8, right: 8, bottom: 0, left: 8 }}><defs><linearGradient id="todayTrend" x1="0" x2="0" y1="0" y2="1"><stop offset="5%" stopColor="hsl(var(--chart-1))" stopOpacity={0.25} /><stop offset="95%" stopColor="hsl(var(--chart-1))" stopOpacity={0} /></linearGradient></defs><XAxis dataKey="day" tickLine={false} axisLine={false} tick={{ fontSize: 11 }} /><Tooltip cursor={{ stroke: "hsl(var(--border))" }} /><Area type="monotone" dataKey="actions" stroke="hsl(var(--chart-1))" fill="url(#todayTrend)" strokeWidth={2} /></AreaChart></ResponsiveContainer></div></CardContent></Card>
    </Section>
  </div>;
}
