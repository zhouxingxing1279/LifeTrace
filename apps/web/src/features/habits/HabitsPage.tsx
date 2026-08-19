import { useState, type FormEvent } from "react";
import { Check, Flame, Plus } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, Card, CardContent, EmptyState, Input, PageHeader, Progress } from "../../components/ui";
import { entities, recentDays, text, todayKey } from "../../lib/entities";
import { createHabitActivity, createHabitLog } from "../../services/core";

function currentStreak(completedDates: Set<string>): number {
  const cursor = new Date();
  const today = todayKey();
  if (!completedDates.has(today)) cursor.setDate(cursor.getDate() - 1);
  let streak = 0;
  while (streak < 366) {
    const key = `${cursor.getFullYear()}-${String(cursor.getMonth() + 1).padStart(2, "0")}-${String(cursor.getDate()).padStart(2, "0")}`;
    if (!completedDates.has(key)) break;
    streak += 1;
    cursor.setDate(cursor.getDate() - 1);
  }
  return streak;
}

export function HabitsPage() {
  const { state, session, upsert, remove } = useApp();
  const [name, setName] = useState("");
  const [showNew, setShowNew] = useState(false);
  const activities = entities(state, "habit.activity").filter((item) => !item.isArchived);
  const logs = entities(state, "habit.log");
  const today = todayKey();
  const days7 = recentDays(7);
  const days30 = recentDays(30);

  async function add(event: FormEvent) {
    event.preventDefault();
    if (!session) return;
    await upsert("habit.activity", createHabitActivity(session.user.id, session.session.deviceId, { name }));
    setName("");
    setShowNew(false);
  }

  async function toggle(activityId: string) {
    const existing = logs.find((item) => item.activityId === activityId && text(item, "logDate") === today && text(item, "status") === "completed");
    if (existing) await remove("habit.log", existing.meta.id);
    else if (session) await upsert("habit.log", createHabitLog(session.user.id, session.session.deviceId, activityId, 1, "", today));
  }

  return <div className="page-shell">
    <PageHeader
      title="坚持"
      description="今日打卡、streak、7/30 天完成率与 30 天 Heatmap。信息表达参考 Tremor，操作密度参考 Shadcnblocks。"
      action={<Button onClick={() => setShowNew(true)}><Plus size={16} />新建项目</Button>}
    />

    {showNew ? <Card className="mb-4"><CardContent className="pt-5"><form className="flex gap-2" onSubmit={(event) => void add(event)}>
      <Input autoFocus value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：阅读 30 分钟" required />
      <Button type="submit">保存</Button><Button variant="ghost" onClick={() => setShowNew(false)}>取消</Button>
    </form></CardContent></Card> : null}

    {activities.length ? <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{activities.map((activity) => {
      const activityLogs = logs.filter((item) => item.activityId === activity.meta.id && text(item, "status") === "completed");
      const completedDates = new Set(activityLogs.map((item) => text(item, "logDate")));
      const completedToday = completedDates.has(today);
      const weekCount = days7.filter((day) => completedDates.has(day)).length;
      const monthCount = days30.filter((day) => completedDates.has(day)).length;
      const weekPercent = Math.round(weekCount / 7 * 100);
      const monthPercent = Math.round(monthCount / 30 * 100);
      const streak = currentStreak(completedDates);

      return <Card key={activity.meta.id}><CardContent className="pt-5">
        <div className="flex items-start justify-between gap-3">
          <div>
            <div className="flex flex-wrap items-center gap-2"><div className="text-base font-semibold">{text(activity, "name", "坚持项目")}</div>{streak > 0 ? <Badge className="border-warning/25 bg-warning/10 text-warning"><Flame size={12} className="mr-1" />{streak} 天 streak</Badge> : null}</div>
            <div className="mt-1 text-xs text-muted-foreground">7 天 {weekCount}/7 · 30 天 {monthCount}/30</div>
          </div>
          <button onClick={() => void toggle(activity.meta.id)} className={`flex h-10 w-10 items-center justify-center rounded-full border ${completedToday ? "border-primary bg-primary text-primary-foreground" : "bg-background"}`} aria-label={completedToday ? "取消今日打卡" : "今日打卡"}>{completedToday ? <Check size={18} /> : <Flame size={17} />}</button>
        </div>

        <div className="mt-5 space-y-3">
          <div><div className="mb-1.5 flex justify-between text-xs"><span>7 天完成率</span><span className="text-muted-foreground">{weekPercent}%</span></div><Progress value={weekPercent} /></div>
          <div><div className="mb-1.5 flex justify-between text-xs"><span>30 天完成率</span><span className="text-muted-foreground">{monthPercent}%</span></div><Progress value={monthPercent} /></div>
        </div>

        <div className="mt-5">
          <div className="mb-2 flex items-center justify-between text-[11px] text-muted-foreground"><span>30 天 Heatmap</span><span>越深 = 已完成</span></div>
          <div className="grid grid-cols-10 gap-1">{days30.map((day) => <div key={day} title={`${day}${completedDates.has(day) ? " · 已完成" : ""}`} className={`aspect-square min-h-3 rounded-sm ${completedDates.has(day) ? "bg-primary" : "bg-muted"}`} />)}</div>
        </div>
      </CardContent></Card>;
    })}</div> : <EmptyState title="还没有坚持项目" description="创建一个可以长期追踪、每天只需一次明确动作的项目。" action={<Button variant="outline" onClick={() => setShowNew(true)}>创建第一个项目</Button>} />}
  </div>;
}
