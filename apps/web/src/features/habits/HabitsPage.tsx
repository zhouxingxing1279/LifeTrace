import { useState, type FormEvent } from "react";
import { Check, Flame, Plus } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Button, Card, CardContent, EmptyState, Input, PageHeader, Progress } from "../../components/ui";
import { entities, recentDays, text, todayKey } from "../../lib/entities";
import { createHabitActivity, createHabitLog } from "../../services/core";

export function HabitsPage() {
  const { state, session, upsert, remove } = useApp();
  const [name, setName] = useState("");
  const [showNew, setShowNew] = useState(false);
  const activities = entities(state,"habit.activity").filter((item) => !item.isArchived);
  const logs = entities(state,"habit.log");
  const today = todayKey();
  const days = recentDays(7);

  async function add(event: FormEvent) {
    event.preventDefault(); if (!session) return;
    await upsert("habit.activity", createHabitActivity(session.user.id, session.session.deviceId, { name }));
    setName(""); setShowNew(false);
  }
  async function toggle(activityId: string) {
    const existing = logs.find((item) => item.activityId === activityId && text(item,"logDate") === today && text(item,"status") === "completed");
    if (existing) await remove("habit.log", existing.meta.id);
    else if (session) await upsert("habit.log", createHabitLog(session.user.id, session.session.deviceId, activityId, 1, "", today));
  }

  return <div className="page-shell"><PageHeader title="坚持" description="今日打卡、连续记录和 7 天趋势。信息表达参考 Tremor，操作密度参考 Shadcnblocks。" action={<Button onClick={() => setShowNew(true)}><Plus size={16}/>新建项目</Button>} />
    {showNew ? <Card className="mb-4"><CardContent className="pt-5"><form className="flex gap-2" onSubmit={(event) => void add(event)}><Input autoFocus value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：阅读 30 分钟" required/><Button type="submit">保存</Button><Button variant="ghost" onClick={() => setShowNew(false)}>取消</Button></form></CardContent></Card> : null}
    {activities.length ? <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{activities.map((activity) => {
      const activityLogs = logs.filter((item) => item.activityId === activity.meta.id && text(item,"status") === "completed");
      const completed = activityLogs.some((item) => text(item,"logDate") === today);
      const weekCount = activityLogs.filter((item) => days.includes(text(item,"logDate"))).length;
      const percent = Math.round(weekCount / 7 * 100);
      return <Card key={activity.meta.id}><CardContent className="pt-5"><div className="flex items-start justify-between gap-3"><div><div className="text-base font-semibold">{text(activity,"name","坚持项目")}</div><div className="mt-1 text-xs text-muted-foreground">过去 7 天完成 {weekCount} 次</div></div><button onClick={() => void toggle(activity.meta.id)} className={`flex h-10 w-10 items-center justify-center rounded-full border ${completed ? "border-primary bg-primary text-primary-foreground" : "bg-background"}`} aria-label={completed ? "取消今日打卡" : "今日打卡"}>{completed ? <Check size={18}/> : <Flame size={17}/>}</button></div><div className="mt-5"><div className="mb-2 flex justify-between text-xs"><span>7 天完成率</span><span className="text-muted-foreground">{percent}%</span></div><Progress value={percent}/></div><div className="mt-4 grid grid-cols-7 gap-1">{days.map((day) => <div key={day} title={day} className={`h-6 rounded-sm ${activityLogs.some((item) => text(item,"logDate") === day) ? "bg-primary" : "bg-muted"}`}/>)}</div></CardContent></Card>;
    })}</div> : <EmptyState title="还没有坚持项目" description="创建一个可以长期追踪、每天只需一次明确动作的项目。" action={<Button variant="outline" onClick={() => setShowNew(true)}>创建第一个项目</Button>} />}
  </div>;
}
