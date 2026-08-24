import { useMemo, useState, type FormEvent } from "react";
import { Bar, BarChart, ResponsiveContainer, Tooltip, XAxis } from "recharts";
import { Dumbbell, Plus } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Button, Card, CardContent, EmptyState, Input, MetricCard, PageHeader, Section } from "../../components/ui";
import { entities, number, recentDays, text } from "../../lib/entities";
import { createWorkout } from "../../services/core";

export function FitnessPage() {
  const { state, session, upsert } = useApp();
  const [showNew,setShowNew] = useState(false); const [name,setName] = useState(""); const [minutes,setMinutes] = useState("60"); const [volume,setVolume] = useState("");
  const workouts = entities(state,"workout.workout").sort((a,b) => text(b,"occurredAt").localeCompare(text(a,"occurredAt")));
  const days = recentDays(7);
  const weekly = workouts.filter((item) => days.includes(text(item,"localDate")));
  const chart = useMemo(() => days.map((day) => ({ day: day.slice(5), volume: workouts.filter((item) => text(item,"localDate") === day).reduce((total,item) => total + number(item,"volumeKg"),0) })), [days.join("|"), workouts]);
  const totalVolume = weekly.reduce((total,item) => total + number(item,"volumeKg"),0);
  const totalMinutes = Math.round(weekly.reduce((total,item) => total + number(item,"durationSeconds"),0)/60);

  async function create(event: FormEvent) { event.preventDefault(); if (!session) return; await upsert("workout.workout",createWorkout(session.user.id,session.session.deviceId,{name,durationMinutes:Number(minutes)||0,volumeKg:volume ? Number(volume) : null})); setName("");setVolume("");setShowNew(false); }
  return <div className="page-shell"><PageHeader title="健身" description="本周训练、训练量和训练记录。数据页面参考 Tremor analytics pattern。" action={<Button onClick={() => setShowNew(true)}><Plus size={16}/>记录训练</Button>} />
    <div className="grid gap-3 sm:grid-cols-3"><MetricCard label="本周训练" value={`${weekly.length} 次`} icon={<Dumbbell size={17}/>} /><MetricCard label="训练时长" value={`${totalMinutes} 分钟`} hint="最近 7 天"/><MetricCard label="训练容量" value={`${Math.round(totalVolume).toLocaleString()} kg`} hint="有重量记录的训练"/></div>
    {showNew ? <Card className="mt-5"><CardContent className="pt-5"><form className="grid gap-3 sm:grid-cols-[1fr_140px_160px_auto]" onSubmit={(event)=>void create(event)}><Input autoFocus value={name} onChange={(e)=>setName(e.target.value)} placeholder="训练名称"/><Input type="number" min="0" value={minutes} onChange={(e)=>setMinutes(e.target.value)} placeholder="分钟"/><Input type="number" min="0" step="0.1" value={volume} onChange={(e)=>setVolume(e.target.value)} placeholder="训练容量 kg"/><div className="flex gap-2"><Button type="submit">保存</Button><Button variant="ghost" onClick={()=>setShowNew(false)}>取消</Button></div></form></CardContent></Card>:null}
    <div className="mt-6 grid gap-5 xl:grid-cols-[1fr_420px]"><Section title="7 天训练量"><Card><CardContent className="pt-5"><div className="h-56"><ResponsiveContainer width="100%" height="100%"><BarChart data={chart}><XAxis dataKey="day" tickLine={false} axisLine={false} tick={{fontSize:11}}/><Tooltip/><Bar dataKey="volume" fill="hsl(var(--chart-1))" radius={[4,4,0,0]}/></BarChart></ResponsiveContainer></div></CardContent></Card></Section><Section title="最近训练">{workouts.length ? <Card><div className="divide-y">{workouts.slice(0,7).map((workout)=><div key={workout.meta.id} className="px-4 py-3"><div className="flex justify-between gap-3"><div className="truncate text-sm font-medium">{text(workout,"name","训练")}</div><div className="text-xs text-muted-foreground">{text(workout,"localDate")}</div></div><div className="mt-1 text-xs text-muted-foreground">{Math.round(number(workout,"durationSeconds")/60)} 分钟 · {Math.round(number(workout,"volumeKg")).toLocaleString()} kg</div></div>)}</div></Card>:<EmptyState title="还没有训练记录" description="记录第一次训练后，这里会生成趋势。"/>}</Section></div>
  </div>;
}
