import { useMemo, useState, type FormEvent } from "react";
import { ChevronLeft, ChevronRight, Plus } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, Card, CardContent, EmptyState, Input, PageHeader, cn } from "../../components/ui";
import { entities, text, todayKey } from "../../lib/entities";
import { createExecutionCalendarEvent, type JsonEntity } from "../../services/core";

type CalendarView = "month" | "week" | "day" | "agenda";
type CalendarItem = { id: string; date: string; title: string; kind: "event" | "task"; time: string };

function dateKey(date: Date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

function startOfWeek(date: Date) {
  const value = new Date(date);
  const weekday = value.getDay() === 0 ? 7 : value.getDay();
  value.setDate(value.getDate() - weekday + 1);
  value.setHours(0, 0, 0, 0);
  return value;
}

function weekDays(anchor: Date) {
  const start = startOfWeek(anchor);
  return Array.from({ length: 7 }, (_, index) => {
    const value = new Date(start);
    value.setDate(start.getDate() + index);
    return value;
  });
}

function monthCells(anchor: Date) {
  const first = new Date(anchor.getFullYear(), anchor.getMonth(), 1);
  const start = startOfWeek(first);
  return Array.from({ length: 42 }, (_, index) => {
    const value = new Date(start);
    value.setDate(start.getDate() + index);
    return value;
  });
}

function itemDate(entity: JsonEntity, event: boolean) {
  return event
    ? text(entity, "startLocalDate") || text(entity, "startAt").slice(0, 10)
    : (text(entity, "scheduledStartAt") || text(entity, "dueAt")).slice(0, 10);
}

function itemTime(entity: JsonEntity, event: boolean) {
  const raw = event ? text(entity, "startAt") : text(entity, "scheduledStartAt", text(entity, "dueAt"));
  if (!raw || !raw.includes("T")) return "全天";
  const date = new Date(raw);
  if (Number.isNaN(date.getTime())) return "全天";
  return new Intl.DateTimeFormat("zh-CN", { hour: "2-digit", minute: "2-digit" }).format(date);
}

function initialView(): CalendarView {
  if (typeof window !== "undefined" && window.matchMedia("(max-width: 767px)").matches) return "agenda";
  return "month";
}

export function CalendarPage() {
  const { state, session, upsert } = useApp();
  const [anchor, setAnchor] = useState(() => new Date());
  const [view, setView] = useState<CalendarView>(initialView);
  const [showNew, setShowNew] = useState(false);
  const [title, setTitle] = useState("");
  const [date, setDate] = useState(todayKey());

  const events = entities(state, "execution.calendar_event").filter((item) => text(item, "status", "scheduled") !== "cancelled");
  const tasks = entities(state, "execution.task").filter((item) => !["done", "cancelled"].includes(text(item, "status")));
  const items = useMemo<CalendarItem[]>(() => [
    ...events.map((item) => ({ id: item.meta.id, date: itemDate(item, true), title: text(item, "title", "日程"), kind: "event" as const, time: itemTime(item, true) })),
    ...tasks.map((item) => ({ id: item.meta.id, date: itemDate(item, false), title: text(item, "title", "任务"), kind: "task" as const, time: itemTime(item, false) })),
  ].filter((item) => item.date).sort((left, right) => `${left.date}${left.time}`.localeCompare(`${right.date}${right.time}`)), [events, tasks]);

  const cells = useMemo(() => monthCells(anchor), [anchor.getFullYear(), anchor.getMonth()]);
  const week = useMemo(() => weekDays(anchor), [dateKey(startOfWeek(anchor))]);
  const selectedDay = dateKey(anchor);

  async function add(event: FormEvent) {
    event.preventDefault();
    if (!session) return;
    const row = createExecutionCalendarEvent(session.user.id, session.session.deviceId, {
      title,
      isAllDay: true,
      startLocalDate: date,
      endLocalDate: date,
    });
    await upsert("execution.calendar_event", row);
    setTitle("");
    setShowNew(false);
  }

  function move(direction: -1 | 1) {
    const value = new Date(anchor);
    if (view === "month") value.setMonth(value.getMonth() + direction, 1);
    else if (view === "week") value.setDate(value.getDate() + 7 * direction);
    else value.setDate(value.getDate() + direction);
    setAnchor(value);
  }

  const label = view === "month"
    ? `${anchor.getFullYear()} 年 ${anchor.getMonth() + 1} 月`
    : view === "week"
      ? `${dateKey(week[0])} – ${dateKey(week[6])}`
      : new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "long", day: "numeric", weekday: "short" }).format(anchor);

  return <div className="page-shell">
    <PageHeader
      title="日历"
      description="Month / Week / Day / Agenda 四种视图；移动端默认 Agenda，避免强行压缩桌面月视图。"
      action={<Button onClick={() => setShowNew(true)}><Plus size={16} />新建日程</Button>}
    />

    {showNew ? <Card className="mb-4"><CardContent className="pt-5"><form className="grid gap-3 sm:grid-cols-[1fr_170px_auto]" onSubmit={(event) => void add(event)}>
      <Input autoFocus value={title} onChange={(event) => setTitle(event.target.value)} placeholder="日程名称" required />
      <Input type="date" value={date} onChange={(event) => setDate(event.target.value)} />
      <div className="flex gap-2"><Button type="submit">保存</Button><Button variant="ghost" onClick={() => setShowNew(false)}>取消</Button></div>
    </form></CardContent></Card> : null}

    <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
      <div className="flex items-center gap-1">
        <Button size="icon" variant="ghost" onClick={() => move(-1)} aria-label="上一时间段"><ChevronLeft size={17} /></Button>
        <div className="min-w-40 text-center text-sm font-semibold">{label}</div>
        <Button size="icon" variant="ghost" onClick={() => move(1)} aria-label="下一时间段"><ChevronRight size={17} /></Button>
        <Button size="sm" variant="outline" onClick={() => setAnchor(new Date())}>今天</Button>
      </div>
      <div className="scrollbar-thin flex max-w-full overflow-x-auto rounded-md border p-0.5">
        {(["month", "week", "day", "agenda"] as CalendarView[]).map((mode) => <button key={mode} className={cn("shrink-0 rounded px-3 py-1.5 text-xs", view === mode && "bg-muted font-medium")} onClick={() => setView(mode)}>{mode[0].toUpperCase() + mode.slice(1)}</button>)}
      </div>
    </div>

    {view === "month" ? <MonthView anchor={anchor} cells={cells} items={items} /> : null}
    {view === "week" ? <WeekView days={week} items={items} onSelect={(day) => { setAnchor(day); setView("day"); }} /> : null}
    {view === "day" ? <DayView date={selectedDay} items={items} /> : null}
    {view === "agenda" ? <AgendaView items={items} /> : null}
  </div>;
}

function MonthView({ anchor, cells, items }: { anchor: Date; cells: Date[]; items: CalendarItem[] }) {
  return <Card className="overflow-hidden">
    <div className="grid grid-cols-7 border-b bg-muted/25 text-center text-[11px] font-medium text-muted-foreground">{"一二三四五六日".split("").map((day) => <div key={day} className="py-2">周{day}</div>)}</div>
    <div className="grid grid-cols-7">{cells.map((day) => {
      const key = dateKey(day);
      const dayItems = items.filter((item) => item.date === key);
      return <div key={key} className={cn("min-h-24 border-b border-r p-2 sm:min-h-28", day.getMonth() !== anchor.getMonth() && "bg-muted/20 text-muted-foreground", key === todayKey() && "bg-accent/35")}>
        <div className="mb-1 text-xs font-medium">{day.getDate()}</div>
        <div className="space-y-1">{dayItems.slice(0, 3).map((item) => <div key={`${item.kind}-${item.id}`} className="truncate rounded bg-muted px-1.5 py-1 text-[10px]">{item.time !== "全天" ? `${item.time} ` : ""}{item.title}</div>)}{dayItems.length > 3 ? <div className="text-[10px] text-muted-foreground">+{dayItems.length - 3}</div> : null}</div>
      </div>;
    })}</div>
  </Card>;
}

function WeekView({ days, items, onSelect }: { days: Date[]; items: CalendarItem[]; onSelect(day: Date): void }) {
  return <div className="grid gap-2 md:grid-cols-7">{days.map((day) => {
    const key = dateKey(day);
    const dayItems = items.filter((item) => item.date === key);
    return <Card key={key} className={cn("min-h-40", key === todayKey() && "border-primary")}>
      <button className="w-full border-b px-3 py-2 text-left hover:bg-muted/40" onClick={() => onSelect(day)}><div className="text-xs text-muted-foreground">{new Intl.DateTimeFormat("zh-CN", { weekday: "short" }).format(day)}</div><div className="mt-1 font-semibold">{day.getMonth() + 1}/{day.getDate()}</div></button>
      <div className="space-y-1 p-2">{dayItems.map((item) => <CalendarRow key={`${item.kind}-${item.id}`} item={item} compact />)}{!dayItems.length ? <div className="px-1 py-3 text-xs text-muted-foreground">无安排</div> : null}</div>
    </Card>;
  })}</div>;
}

function DayView({ date, items }: { date: string; items: CalendarItem[] }) {
  const dayItems = items.filter((item) => item.date === date);
  return <Card>{dayItems.length ? <div className="divide-y">{dayItems.map((item) => <CalendarRow key={`${item.kind}-${item.id}`} item={item} />)}</div> : <CardContent className="pt-5"><EmptyState title="今天没有安排" description="新建日程或为任务设置时间后会显示在这里。" /></CardContent>}</Card>;
}

function AgendaView({ items }: { items: CalendarItem[] }) {
  const visible = items.filter((item) => item.date >= todayKey()).slice(0, 100);
  return <Card>{visible.length ? <div className="divide-y">{visible.map((item) => <CalendarRow key={`${item.kind}-${item.id}`} item={item} showDate />)}</div> : <CardContent className="pt-5"><EmptyState title="没有即将到来的安排" /></CardContent>}</Card>;
}

function CalendarRow({ item, showDate = false, compact = false }: { item: CalendarItem; showDate?: boolean; compact?: boolean }) {
  return <div className={cn("flex items-center gap-3", compact ? "rounded-md bg-muted/45 px-2 py-2" : "px-4 py-3")}>
    {showDate ? <div className="w-24 shrink-0 text-xs text-muted-foreground">{item.date}</div> : null}
    <div className="w-12 shrink-0 text-xs text-muted-foreground">{item.time}</div>
    <div className="min-w-0 flex-1 truncate text-sm font-medium">{item.title}</div>
    {!compact ? <Badge>{item.kind === "task" ? "任务" : "日程"}</Badge> : null}
  </div>;
}
