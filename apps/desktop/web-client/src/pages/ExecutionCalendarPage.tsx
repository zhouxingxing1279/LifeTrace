import { useMemo, useState, type FormEvent } from "react";
import {
  createExecutionCalendarEvent, isOpenExecutionTask, localDate, type JsonEntity,
} from "../core";
import { Empty, Notice, PageStack, Panel, Toolbar, entities, number, text, type CloudPageProps } from "../ui";

interface CalendarItem {
  id: string;
  date: string;
  type: string;
  title: string;
  detail: string;
  startAt?: string | null;
  sourceTaskId?: string | null;
  event?: JsonEntity;
}

function displayDate(value: string): string {
  const date = new Date(`${value}T12:00:00`);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleDateString("zh-CN", { month: "long", day: "numeric", weekday: "long" });
}

function localTime(value?: string | null): string {
  if (!value) return "全天";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  return date.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false });
}

function dayOf(value?: string | null): string {
  if (!value) return "";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value.slice(0, 10) : localDate(date);
}

function buildItems(state: CloudPageProps["state"]): CalendarItem[] {
  const values: CalendarItem[] = [];
  for (const item of entities(state, "execution.calendar_event")) {
    if (item.status === "cancelled") continue;
    const date = item.isAllDay === true ? text(item, "startLocalDate") : dayOf(text(item, "startAt"));
    if (!date) continue;
    values.push({
      id: item.meta.id,
      date,
      type: "计划",
      title: text(item, "title") || "时间块",
      detail: item.isAllDay === true ? "全天" : `${localTime(text(item, "startAt"))}–${localTime(text(item, "endAt"))}`,
      startAt: text(item, "startAt"),
      sourceTaskId: text(item, "sourceTaskId") || null,
      event: item,
    });
  }
  for (const item of entities(state, "habit.log")) values.push({ id: item.meta.id, date: text(item, "logDate"), type: "坚持", title: "完成坚持记录", detail: text(item, "note") });
  for (const item of entities(state, "finance.transaction")) values.push({ id: item.meta.id, date: text(item, "localDate"), type: "财务", title: text(item, "merchant") || text(item, "counterparty") || "财务流水", detail: "账单记录" });
  for (const item of entities(state, "workout.workout")) values.push({ id: item.meta.id, date: text(item, "localDate"), type: "训练", title: text(item, "name") || "训练", detail: `${Math.round(number(item, "durationSeconds") / 60)} 分钟` });
  for (const item of entities(state, "english.learning_record")) values.push({ id: item.meta.id, date: text(item, "recordDate"), type: "英语", title: "完成英语阅读", detail: text(item, "summary").slice(0, 80) });
  for (const item of entities(state, "review.daily")) values.push({ id: item.meta.id, date: text(item, "reviewDate"), type: "复盘", title: "每日复盘", detail: text(item, "tomorrowPriority") || text(item, "bestThing") });
  return values.filter((item) => item.date).sort((left, right) => (left.startAt || "").localeCompare(right.startAt || ""));
}

export function ExecutionCalendarPage({ session, state, run, online }: CloudPageProps) {
  const items = useMemo(() => buildItems(state), [state]);
  const tasks = useMemo(() => entities(state, "execution.task").filter(isOpenExecutionTask), [state]);
  const [month, setMonth] = useState(() => localDate().slice(0, 7));
  const [selected, setSelected] = useState(() => localDate());
  const [taskId, setTaskId] = useState("");
  const [startTime, setStartTime] = useState("19:00");
  const [duration, setDuration] = useState("60");
  const [manualTitle, setManualTitle] = useState("");
  const [message, setMessage] = useState("");
  const [year, monthNumber] = month.split("-").map(Number);
  const first = (new Date(year!, monthNumber! - 1, 1).getDay() + 6) % 7;
  const count = new Date(year!, monthNumber!, 0).getDate();
  const selectedItems = items.filter((item) => item.date === selected);

  function move(offset: number) {
    const value = new Date(year!, monthNumber! - 1 + offset, 1);
    const next = `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, "0")}`;
    setMonth(next);
    setSelected(`${next}-01`);
  }

  function windowForSelection() {
    const start = new Date(`${selected}T${startTime}:00`);
    if (Number.isNaN(start.getTime())) throw new Error("请选择有效时间");
    const minutes = Math.max(5, Number(duration) || 60);
    return { start, end: new Date(start.getTime() + minutes * 60_000) };
  }

  async function scheduleTask(event: FormEvent) {
    event.preventDefault();
    const task = tasks.find((item) => item.meta.id === taskId);
    if (!task) return;
    const { start, end } = windowForSelection();
    const currentEvent = entities(state, "execution.calendar_event").find((item) => text(item, "sourceTaskId") === task.meta.id && item.status !== "cancelled");
    const nextTask: JsonEntity = {
      ...task,
      scheduledStartAt: start.toISOString(),
      scheduledEndAt: end.toISOString(),
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
      context: task.context === "inbox" ? null : task.context,
    };
    const nextEvent: JsonEntity = currentEvent
      ? { ...currentEvent, title: text(task, "title"), description: text(task, "description") || null, isAllDay: false, startAt: start.toISOString(), endAt: end.toISOString(), startLocalDate: null, endLocalDate: null, status: "scheduled", sourceTaskId: task.meta.id }
      : createExecutionCalendarEvent(session.user.id, session.session.deviceId, { title: text(task, "title"), description: text(task, "description"), startAt: start.toISOString(), endAt: end.toISOString(), sourceTaskId: task.meta.id });
    await run(async (store) => {
      await store.upsert("execution.task", nextTask);
      return store.upsert("execution.calendar_event", nextEvent);
    });
    setTaskId("");
    setMessage("任务已放入时间块，并同步回 Today");
  }

  async function createManual(event: FormEvent) {
    event.preventDefault();
    const title = manualTitle.trim();
    if (!title) return;
    const { start, end } = windowForSelection();
    const value = createExecutionCalendarEvent(session.user.id, session.session.deviceId, { title, startAt: start.toISOString(), endAt: end.toISOString() });
    await run((store) => store.upsert("execution.calendar_event", value));
    setManualTitle("");
    setMessage("时间块已创建");
  }

  async function unschedule(item: CalendarItem) {
    if (!item.event) return;
    const task = item.sourceTaskId ? tasks.find((candidate) => candidate.meta.id === item.sourceTaskId) : undefined;
    await run(async (store) => {
      if (task) await store.upsert("execution.task", { ...task, scheduledStartAt: null, scheduledEndAt: null });
      return store.upsert("execution.calendar_event", { ...item.event!, status: "cancelled" });
    });
    setMessage(task ? "任务已移出时间块" : "时间块已取消");
  }

  return <PageStack>
    {message && <Notice kind="success">{message}</Notice>}
    <div className="hx-calendar-layout">
      <Panel eyebrow="TIMEBOX CALENDAR" title={`${year} 年 ${monthNumber} 月`} actions={<Toolbar><button className="hx-btn ghost" onClick={() => move(-1)}>上个月</button><button className="hx-btn ghost" onClick={() => { setMonth(localDate().slice(0, 7)); setSelected(localDate()); }}>今天</button><button className="hx-btn ghost" onClick={() => move(1)}>下个月</button></Toolbar>}>
        <div className="hx-week">{"一二三四五六日".split("").map((item) => <span key={item}>周{item}</span>)}</div>
        <div className="hx-days">
          {Array.from({ length: first }).map((_, index) => <i key={`empty-${index}`} />)}
          {Array.from({ length: count }, (_, index) => index + 1).map((date) => {
            const key = `${month}-${String(date).padStart(2, "0")}`;
            const own = items.filter((item) => item.date === key);
            return <button key={key} className={selected === key ? "selected" : ""} onClick={() => setSelected(key)}><b>{date}</b><span>{own.slice(0, 4).map((item) => <i key={`${item.type}-${item.id}`} title={`${item.type}：${item.title}`} />)}</span><small>{own.length || ""}</small></button>;
          })}
        </div>
      </Panel>

      <Panel eyebrow="DAY" title={displayDate(selected)}>
        <div className="hx-list">
          {selectedItems.map((item) => <article className="hx-row" key={`${item.type}-${item.id}`}><span className="hx-row-icon">{item.type.slice(0, 1)}</span><div className="hx-row-main"><strong>{item.title}</strong><small>{item.type} · {item.detail}</small></div>{item.event && <button className="hx-btn ghost" disabled={!online} onClick={() => void unschedule(item)}>移出</button>}</article>)}
          {!selectedItems.length && <Empty title="这一天还没有安排" description="把任务拖进具体时间并不必要；选择下方任务和开始时间即可创建 timebox。" />}
        </div>
      </Panel>
    </div>

    <div className="hx-content-grid two">
      <Panel eyebrow="TASK → TIMEBOX" title="把任务安排到具体时间">
        <form className="hx-form" onSubmit={(event) => void scheduleTask(event)}>
          <label>任务<select required value={taskId} onChange={(event) => setTaskId(event.target.value)}><option value="">选择待办任务</option>{tasks.map((task) => <option key={task.meta.id} value={task.meta.id}>{text(task, "title")}</option>)}</select></label>
          <div className="hx-form-grid"><label>开始时间<input type="time" value={startTime} onChange={(event) => setStartTime(event.target.value)} /></label><label>时长（分钟）<input type="number" min="5" step="5" value={duration} onChange={(event) => setDuration(event.target.value)} /></label></div>
          <button className="hx-btn primary" disabled={!online || !taskId}>安排到 {displayDate(selected)}</button>
        </form>
      </Panel>

      <Panel eyebrow="CALENDAR BLOCK" title="创建独立时间块">
        <form className="hx-form" onSubmit={(event) => void createManual(event)}>
          <label>标题<input required value={manualTitle} onChange={(event) => setManualTitle(event.target.value)} placeholder="例如：论文深度工作" /></label>
          <div className="hx-form-grid"><label>开始时间<input type="time" value={startTime} onChange={(event) => setStartTime(event.target.value)} /></label><label>时长（分钟）<input type="number" min="5" step="5" value={duration} onChange={(event) => setDuration(event.target.value)} /></label></div>
          <button className="hx-btn secondary" disabled={!online}>创建时间块</button>
        </form>
      </Panel>
    </div>
  </PageStack>;
}
