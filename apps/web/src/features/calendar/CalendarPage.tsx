import { useMemo, useState, type FormEvent } from "react";
import { ChevronLeft, ChevronRight, Plus } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, Card, CardContent, EmptyState, Input, PageHeader, cn } from "../../components/ui";
import { entities, text, todayKey } from "../../lib/entities";
import { createExecutionCalendarEvent, type JsonEntity } from "../../services/core";

function monthCells(anchor: Date) {
  const first = new Date(anchor.getFullYear(), anchor.getMonth(), 1);
  const start = new Date(first);
  const weekday = first.getDay() === 0 ? 7 : first.getDay();
  start.setDate(first.getDate() - weekday + 1);
  return Array.from({ length: 42 }, (_, index) => {
    const value = new Date(start); value.setDate(start.getDate() + index); return value;
  });
}
function key(date: Date) { return `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,"0")}-${String(date.getDate()).padStart(2,"0")}`; }
function eventDay(item: JsonEntity) { return text(item,"startLocalDate") || text(item,"startAt").slice(0,10); }

export function CalendarPage(){
  const {state,session,upsert}=useApp(); const [anchor,setAnchor]=useState(()=>new Date()); const [view,setView]=useState<"month"|"agenda">("month"); const [showNew,setShowNew]=useState(false); const [title,setTitle]=useState(""); const [date,setDate]=useState(todayKey());
  const events=entities(state,"execution.calendar_event").filter((item)=>text(item,"status","scheduled")!=="cancelled");
  const tasks=entities(state,"execution.task").filter((item)=>text(item,"status")!=="done"&&text(item,"status")!=="cancelled");
  const cells=useMemo(()=>monthCells(anchor),[anchor.getFullYear(),anchor.getMonth()]);
  const agenda=useMemo(()=>[...events.map((item)=>({id:item.meta.id,date:eventDay(item),title:text(item,"title","日程"),kind:"event"})),...tasks.map((item)=>({id:item.meta.id,date:(text(item,"scheduledStartAt")||text(item,"dueAt")).slice(0,10),title:text(item,"title","任务"),kind:"task"}))].filter((item)=>item.date).sort((a,b)=>a.date.localeCompare(b.date)),[events,tasks]);
  async function add(event:FormEvent){event.preventDefault();if(!session)return;const row=createExecutionCalendarEvent(session.user.id,session.session.deviceId,{title,isAllDay:true,startLocalDate:date,endLocalDate:date});await upsert("execution.calendar_event",row);setTitle("");setShowNew(false);}
  return <div className="page-shell"><PageHeader title="日历" description="Desktop 支持 Month / Agenda，移动端默认更适合 Agenda。参考 Shadcnblocks Calendar + Catalyst Toolbar。" action={<Button onClick={()=>setShowNew(true)}><Plus size={16}/>新建日程</Button>}/>
    {showNew?<Card className="mb-4"><CardContent className="pt-5"><form className="grid gap-3 sm:grid-cols-[1fr_170px_auto]" onSubmit={(e)=>void add(e)}><Input autoFocus value={title} onChange={(e)=>setTitle(e.target.value)} placeholder="日程名称" required/><Input type="date" value={date} onChange={(e)=>setDate(e.target.value)}/><div className="flex gap-2"><Button type="submit">保存</Button><Button variant="ghost" onClick={()=>setShowNew(false)}>取消</Button></div></form></CardContent></Card>:null}
    <div className="mb-3 flex flex-wrap items-center justify-between gap-2"><div className="flex items-center gap-1"><Button size="icon" variant="ghost" onClick={()=>setAnchor(new Date(anchor.getFullYear(),anchor.getMonth()-1,1))}><ChevronLeft size={17}/></Button><div className="min-w-28 text-center text-sm font-semibold">{anchor.getFullYear()} 年 {anchor.getMonth()+1} 月</div><Button size="icon" variant="ghost" onClick={()=>setAnchor(new Date(anchor.getFullYear(),anchor.getMonth()+1,1))}><ChevronRight size={17}/></Button><Button size="sm" variant="outline" onClick={()=>setAnchor(new Date())}>今天</Button></div><div className="flex rounded-md border p-0.5"><button className={cn("rounded px-3 py-1.5 text-xs",view==="month"&&"bg-muted font-medium")} onClick={()=>setView("month")}>Month</button><button className={cn("rounded px-3 py-1.5 text-xs",view==="agenda"&&"bg-muted font-medium")} onClick={()=>setView("agenda")}>Agenda</button></div></div>
    {view==="month"?<Card className="overflow-hidden"><div className="grid grid-cols-7 border-b bg-muted/25 text-center text-[11px] font-medium text-muted-foreground">{"一二三四五六日".split("").map((d)=><div key={d} className="py-2">周{d}</div>)}</div><div className="grid grid-cols-7">{cells.map((day)=>{const dayKey=key(day);const dayEvents=agenda.filter((item)=>item.date===dayKey);const outside=day.getMonth()!==anchor.getMonth();return <div key={dayKey} className={cn("min-h-24 border-b border-r p-2 sm:min-h-28",outside&&"bg-muted/20 text-muted-foreground",dayKey===todayKey()&&"bg-accent/35")}><div className="mb-1 text-xs font-medium">{day.getDate()}</div><div className="space-y-1">{dayEvents.slice(0,3).map((item)=><div key={`${item.kind}-${item.id}`} className="truncate rounded bg-muted px-1.5 py-1 text-[10px]">{item.title}</div>)}{dayEvents.length>3?<div className="text-[10px] text-muted-foreground">+{dayEvents.length-3}</div>:null}</div></div>})}</div></Card>:<Card><div className="divide-y">{agenda.length?agenda.slice(0,80).map((item)=><div key={`${item.kind}-${item.id}`} className="flex items-center gap-4 px-4 py-3"><div className="w-24 shrink-0 text-xs text-muted-foreground">{item.date}</div><div className="min-w-0 flex-1 truncate text-sm font-medium">{item.title}</div><Badge>{item.kind==="task"?"任务":"日程"}</Badge></div>):<CardContent className="pt-5"><EmptyState title="没有日程" description="创建日程或为任务设置时间后会显示在这里。"/></CardContent>}</div></Card>}
  </div>;
}
