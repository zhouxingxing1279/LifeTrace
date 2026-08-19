import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis } from "recharts";
import { HeartPulse, Moon, Activity } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Card, CardContent, EmptyState, MetricCard, PageHeader, Section } from "../../components/ui";
import { entities, number, recentDays, text } from "../../lib/entities";

export function HealthPage(){
  const {state}=useApp(); const workouts=entities(state,"workout.workout"); const days=recentDays(14); const data=days.map((day)=>({day:day.slice(5),minutes:Math.round(workouts.filter((w)=>text(w,"localDate")===day).reduce((s,w)=>s+number(w,"durationSeconds"),0)/60)})); const activeDays=data.filter((item)=>item.minutes>0).length; const totalMinutes=data.reduce((s,item)=>s+item.minutes,0);
  return <div className="page-shell"><PageHeader title="健康" description="健康页以趋势优先，不做 KPI 墙。当前云端契约优先展示可验证的训练/活动数据，待健康数据源接入后继续扩展。"/><div className="grid gap-3 sm:grid-cols-3"><MetricCard label="14 天活跃天数" value={`${activeDays} 天`} icon={<Activity size={17}/>}/><MetricCard label="14 天训练时长" value={`${totalMinutes} 分钟`} icon={<HeartPulse size={17}/>}/><MetricCard label="睡眠/恢复" value="待接入" hint="当前 Cloud Schema 未提供睡眠实体" icon={<Moon size={17}/>} /></div><Section className="mt-6" title="活动趋势" description="Tremor-style compact trend"><Card><CardContent className="pt-5"><div className="h-64"><ResponsiveContainer width="100%" height="100%"><AreaChart data={data}><XAxis dataKey="day" tickLine={false} axisLine={false} tick={{fontSize:10}}/><Tooltip/><Area type="monotone" dataKey="minutes" stroke="hsl(var(--chart-2))" fill="hsl(var(--chart-2))" fillOpacity={0.12} strokeWidth={2}/></AreaChart></ResponsiveContainer></div></CardContent></Card></Section><div className="mt-5"><EmptyState title="健康数据源保持契约优先" description="不伪造心率、睡眠或体重数据；后端提供 Health Connect / Xiaomi / Huawei 等合法数据契约后再接入对应卡片。"/></div></div>;
}
