import { useMemo, useState, type FormEvent } from "react";
import {
  createExecutionRecurrenceRule,
  createExecutionReminder,
  createExecutionSubtask,
  createExecutionTask,
  createExecutionTaskDependency,
  createExecutionWaitingItem,
  createWaitingConversionLinks,
  dependencyCreatesCycle,
  dismissExecutionReminder,
  isOpenExecutionTask,
  localDate,
  materializeCalendarOccurrences,
  moveCalendarOccurrence,
  recurrenceLabel,
  reminderEffectiveAt,
  reminderIsDue,
  resolveExecutionWaitingItem,
  snoozeExecutionReminder,
  taskBlockers,
  type ExecutionReminderSubject,
  type JsonEntity,
} from "../core";
import { navigate } from "../navigation";
import { Empty, Metric, MetricGrid, Notice, PageStack, Panel, Toolbar, entities, text, type CloudPageProps } from "../ui";

type ControlView = "waiting" | "reminders" | "structure" | "calendar";

const VIEW_LABELS: Array<[ControlView, string]> = [
  ["waiting", "等待事项"],
  ["reminders", "提醒"],
  ["structure", "子任务与依赖"],
  ["calendar", "重复日历"],
];
const WEEKDAYS = [[1, "一"], [2, "二"], [3, "三"], [4, "四"], [5, "五"], [6, "六"], [7, "日"]] as const;

function isoFromDateTime(value: string): string | null {
  if (!value) return null;
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed.toISOString();
}

function displayDateTime(value: string): string {
  if (!value) return "未设置";
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString("zh-CN", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

function localDateTimeValue(value: string): string {
  if (!value) return "";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return "";
  const pad = (part: number) => String(part).padStart(2, "0");
  return `${parsed.getFullYear()}-${pad(parsed.getMonth() + 1)}-${pad(parsed.getDate())}T${pad(parsed.getHours())}:${pad(parsed.getMinutes())}`;
}

function memoTitle(item: JsonEntity): string {
  return (text(item, "plainText") || text(item, "content") || "备忘").slice(0, 60);
}

export function ExecutionControlPage({ session, state, run, online }: CloudPageProps) {
  const [view, setView] = useState<ControlView>("waiting");
  const [message, setMessage] = useState("");

  const tasks = useMemo(() => entities(state, "execution.task").filter((item) => item.status !== "cancelled"), [state]);
  const openTasks = tasks.filter(isOpenExecutionTask);
  const waitingItems = useMemo(() => entities(state, "execution.waiting_item"), [state]);
  const reminders = useMemo(() => entities(state, "execution.reminder"), [state]);
  const dependencies = useMemo(() => entities(state, "execution.task_dependency"), [state]);
  const events = useMemo(() => entities(state, "execution.calendar_event").filter((item) => item.status !== "cancelled"), [state]);
  const calendarOccurrences = useMemo(() => entities(state, "execution.calendar_occurrence"), [state]);
  const recurrenceRules = useMemo(() => entities(state, "execution.recurrence_rule"), [state]);
  const memos = useMemo(() => entities(state, "execution.memo").filter((item) => item.status !== "archived"), [state]);

  const taskMap = new Map(tasks.map((item) => [item.meta.id, item]));
  const waitingMap = new Map(waitingItems.map((item) => [item.meta.id, item]));
  const eventMap = new Map(events.map((item) => [item.meta.id, item]));
  const memoMap = new Map(memos.map((item) => [item.meta.id, item]));
  const ruleMap = new Map(recurrenceRules.map((item) => [item.meta.id, item]));
  const openWaiting = waitingItems.filter((item) => item.status === "open");
  const overdueWaiting = openWaiting.filter((item) => text(item, "expectedAt") && new Date(text(item, "expectedAt")).getTime() < Date.now());
  const dueReminders = reminders.filter((item) => reminderIsDue(item));
  const blockedTasks = openTasks.filter((task) => taskBlockers(task.meta.id, tasks, dependencies).length > 0);
  const recurringEvents = events.filter((event) => text(event, "recurrenceRuleId"));

  return <PageStack>
    <Toolbar>
      {VIEW_LABELS.map(([key, label]) => <button key={key} className={`hx-btn ${view === key ? "primary" : "secondary"}`} onClick={() => setView(key)}>{label}</button>)}
      <button className="hx-btn ghost" onClick={() => navigate("/execution")}>返回计划与待办</button>
      <button className="hx-btn ghost" onClick={() => navigate("/calendar")}>打开 Timebox 日历</button>
    </Toolbar>

    <MetricGrid>
      <Metric label="开放等待" value={String(openWaiting.length)} detail={`${overdueWaiting.length} 项已超过预计返回时间`} positive={overdueWaiting.length === 0 && openWaiting.length > 0} />
      <Metric label="到期提醒" value={String(dueReminders.length)} detail={`${reminders.filter((item) => item.status === "scheduled").length} 个提醒仍在排程`} positive={dueReminders.length === 0} />
      <Metric label="被阻塞任务" value={String(blockedTasks.length)} detail="前置任务尚未完成" positive={blockedTasks.length === 0} />
      <Metric label="重复日历" value={String(recurringEvents.length)} detail={`${calendarOccurrences.length} 个日历实例`} />
    </MetricGrid>

    {message && <Notice kind="success">{message}</Notice>}

    {view === "waiting" && <WaitingView
      session={session}
      online={online}
      run={run}
      tasks={tasks}
      openTasks={openTasks}
      waitingItems={waitingItems}
      taskMap={taskMap}
      setMessage={setMessage}
    />}

    {view === "reminders" && <ReminderView
      session={session}
      online={online}
      run={run}
      reminders={reminders}
      tasks={tasks}
      events={events}
      waitingItems={waitingItems}
      memos={memos}
      taskMap={taskMap}
      eventMap={eventMap}
      waitingMap={waitingMap}
      memoMap={memoMap}
      setMessage={setMessage}
    />}

    {view === "structure" && <StructureView
      session={session}
      online={online}
      run={run}
      tasks={tasks}
      openTasks={openTasks}
      dependencies={dependencies}
      taskMap={taskMap}
      setMessage={setMessage}
    />}

    {view === "calendar" && <CalendarRecurrenceView
      session={session}
      online={online}
      run={run}
      events={events}
      occurrences={calendarOccurrences}
      ruleMap={ruleMap}
      eventMap={eventMap}
      setMessage={setMessage}
    />}
  </PageStack>;
}

function WaitingView({ session, online, run, tasks, openTasks, waitingItems, taskMap, setMessage }: {
  session: CloudPageProps["session"];
  online: boolean;
  run: CloudPageProps["run"];
  tasks: JsonEntity[];
  openTasks: JsonEntity[];
  waitingItems: JsonEntity[];
  taskMap: Map<string, JsonEntity>;
  setMessage: (value: string) => void;
}) {
  const [title, setTitle] = useState("");
  const [waitingFor, setWaitingFor] = useState("");
  const [expectedAt, setExpectedAt] = useState("");
  const [followUpAt, setFollowUpAt] = useState("");
  const [sourceTaskId, setSourceTaskId] = useState("");
  const active = waitingItems.filter((item) => item.status === "open");
  const closed = waitingItems.filter((item) => item.status !== "open");

  async function createWaiting(event: FormEvent) {
    event.preventDefault();
    const source = sourceTaskId ? taskMap.get(sourceTaskId) : undefined;
    const existing = source ? active.find((item) => text(item, "sourceTaskId") === source.meta.id) : undefined;
    if (existing) {
      setMessage("这个任务已经有开放的等待事项");
      return;
    }
    const item = createExecutionWaitingItem(session.user.id, session.session.deviceId, {
      title: source ? text(source, "title") : title,
      description: source ? text(source, "description") : undefined,
      waitingFor,
      expectedAt: isoFromDateTime(expectedAt),
      followUpAt: isoFromDateTime(followUpAt),
      sourceTaskId: source?.meta.id ?? null,
    });
    await run(async (store) => {
      await store.upsert("execution.waiting_item", item);
      if (source) return store.upsert("execution.task", { ...source, status: "waiting" });
      return store.snapshot();
    });
    setTitle("");
    setWaitingFor("");
    setSourceTaskId("");
    setMessage(source ? "任务已转入等待，并保留 sourceTaskId" : "等待事项已创建");
  }

  async function resolve(item: JsonEntity) {
    await run((store) => store.upsert("execution.waiting_item", resolveExecutionWaitingItem(item)));
    setMessage("等待事项已解决；来源任务状态保持不变，可按实际情况恢复");
  }

  async function resumeSource(item: JsonEntity) {
    const task = taskMap.get(text(item, "sourceTaskId"));
    if (!task) return;
    await run(async (store) => {
      await store.upsert("execution.waiting_item", resolveExecutionWaitingItem(item, "等待结束，恢复来源任务"));
      return store.upsert("execution.task", { ...task, status: "todo" });
    });
    setMessage("等待已结束，来源任务恢复为待办");
  }

  async function cancel(item: JsonEntity) {
    await run((store) => store.upsert("execution.waiting_item", { ...item, status: "cancelled", resolvedAt: null }));
    setMessage("等待事项已取消");
  }

  async function convertToTask(item: JsonEntity) {
    const target = createExecutionTask(session.user.id, session.session.deviceId, {
      title: text(item, "title"),
      description: text(item, "description"),
      context: "follow-up",
    });
    const [forward, reverse] = createWaitingConversionLinks(session.user.id, session.session.deviceId, item.meta.id, target.meta.id);
    await run(async (store) => {
      await store.upsert("execution.task", target);
      await store.upsert("execution.entity_link", forward);
      await store.upsert("execution.entity_link", reverse);
      return store.upsert("execution.waiting_item", resolveExecutionWaitingItem(item, "已转换为后续任务"));
    });
    setMessage("等待事项已转换为新任务，并保留双向来源关系");
  }

  return <div className="hx-content-grid two">
    <Panel eyebrow="WAITING" title="等待别人、结果或外部条件">
      <div className="hx-list">{active.map((item) => {
        const source = taskMap.get(text(item, "sourceTaskId"));
        const overdue = text(item, "expectedAt") && new Date(text(item, "expectedAt")).getTime() < Date.now();
        return <article className="hx-row" key={item.meta.id}>
          <span className="hx-row-icon">待</span>
          <div className="hx-row-main"><strong>{text(item, "title")}</strong><small>等待：{text(item, "waitingFor")} · 预计 {displayDateTime(text(item, "expectedAt"))}</small><small>{overdue ? "已超过预计返回时间 · " : ""}{text(item, "followUpAt") ? `跟进 ${displayDateTime(text(item, "followUpAt"))}` : "未设置跟进"}{source ? ` · 来源任务 ${text(source, "title")}` : ""}</small></div>
          <div className="hx-row-actions">{source && <button className="hx-btn secondary" disabled={!online} onClick={() => void resumeSource(item)}>恢复任务</button>}<button className="hx-btn primary" disabled={!online} onClick={() => void resolve(item)}>解决</button><button className="hx-btn secondary" disabled={!online} onClick={() => void convertToTask(item)}>转新任务</button><button className="hx-btn ghost" disabled={!online} onClick={() => void cancel(item)}>取消</button></div>
        </article>;
      })}{!active.length && <Empty title="没有开放的等待事项" description="把“等回复、等审批、等快递、等结果”从普通 Todo 中分离出来，避免它们长期占据今日待办。" />}</div>
      {closed.length > 0 && <p className="hx-muted">历史：已解决 {closed.filter((item) => item.status === "resolved").length} · 已取消 {closed.filter((item) => item.status === "cancelled").length}</p>}
    </Panel>

    <Panel eyebrow="NEW WAITING" title="创建等待事项 / 任务转等待">
      <form className="hx-form" onSubmit={(event) => void createWaiting(event)}>
        <label>来源任务（可选）<select value={sourceTaskId} onChange={(event) => setSourceTaskId(event.target.value)}><option value="">独立等待事项</option>{openTasks.filter((item) => item.status !== "waiting").map((task) => <option key={task.meta.id} value={task.meta.id}>{text(task, "title")}</option>)}</select></label>
        {!sourceTaskId && <label>标题<input required value={title} onChange={(event) => setTitle(event.target.value)} placeholder="例如：等待体检报告" /></label>}
        <label>等待对象 / 条件<input required value={waitingFor} onChange={(event) => setWaitingFor(event.target.value)} placeholder="例如：导师回复、医院、快递" /></label>
        <div className="hx-form-grid"><label>预计返回<input type="datetime-local" value={expectedAt} onChange={(event) => setExpectedAt(event.target.value)} /></label><label>主动跟进<input type="datetime-local" value={followUpAt} onChange={(event) => setFollowUpAt(event.target.value)} /></label></div>
        <button className="hx-btn primary" disabled={!online || (!sourceTaskId && !title.trim()) || !waitingFor.trim()}>{sourceTaskId ? "任务转入等待" : "创建等待事项"}</button>
      </form>
      <p className="hx-muted">当前任务总数 {tasks.length}。任务转等待后状态变为 waiting，不会被错误统计为未执行的普通 todo。</p>
    </Panel>
  </div>;
}

function ReminderView({ session, online, run, reminders, tasks, events, waitingItems, memos, taskMap, eventMap, waitingMap, memoMap, setMessage }: {
  session: CloudPageProps["session"];
  online: boolean;
  run: CloudPageProps["run"];
  reminders: JsonEntity[];
  tasks: JsonEntity[];
  events: JsonEntity[];
  waitingItems: JsonEntity[];
  memos: JsonEntity[];
  taskMap: Map<string, JsonEntity>;
  eventMap: Map<string, JsonEntity>;
  waitingMap: Map<string, JsonEntity>;
  memoMap: Map<string, JsonEntity>;
  setMessage: (value: string) => void;
}) {
  const [subjectType, setSubjectType] = useState<ExecutionReminderSubject>("task");
  const [subjectId, setSubjectId] = useState("");
  const [triggerAt, setTriggerAt] = useState("");

  const subjects = subjectType === "task" ? tasks : subjectType === "calendar_event" ? events : subjectType === "waiting_item" ? waitingItems : memos;
  const subjectLabel = (type: string, id: string) => {
    if (type === "task") return text(taskMap.get(id) ?? ({} as JsonEntity), "title") || "任务";
    if (type === "calendar_event") return text(eventMap.get(id) ?? ({} as JsonEntity), "title") || "日程";
    if (type === "waiting_item") return text(waitingMap.get(id) ?? ({} as JsonEntity), "title") || "等待事项";
    return memoMap.get(id) ? memoTitle(memoMap.get(id)!) : "备忘";
  };

  async function createReminder(event: FormEvent) {
    event.preventDefault();
    const iso = isoFromDateTime(triggerAt);
    if (!subjectId || !iso) return;
    const duplicate = reminders.find((item) => item.status === "scheduled" && text(item, "subjectType") === subjectType && text(item, "subjectId") === subjectId && text(item, "triggerAt") === iso);
    if (duplicate) {
      setMessage("同一对象在该时间已经存在提醒");
      return;
    }
    const reminder = createExecutionReminder(session.user.id, session.session.deviceId, subjectType, subjectId, iso);
    await run((store) => store.upsert("execution.reminder", reminder));
    setSubjectId("");
    setTriggerAt("");
    setMessage("提醒已创建并进入云同步");
  }

  async function snooze(reminder: JsonEntity) {
    const until = new Date(Date.now() + 60 * 60_000).toISOString();
    await run((store) => store.upsert("execution.reminder", snoozeExecutionReminder(reminder, until)));
    setMessage("已稍后 1 小时提醒");
  }

  async function dismiss(reminder: JsonEntity) {
    await run((store) => store.upsert("execution.reminder", dismissExecutionReminder(reminder)));
    setMessage("提醒已忽略");
  }

  async function cancel(reminder: JsonEntity) {
    await run((store) => store.upsert("execution.reminder", { ...reminder, status: "cancelled", snoozedUntil: null }));
    setMessage("提醒已取消");
  }

  return <div className="hx-content-grid two">
    <Panel eyebrow="REMINDERS" title="排程与到期提醒">
      <div className="hx-list">{[...reminders].sort((a, b) => reminderEffectiveAt(a).localeCompare(reminderEffectiveAt(b))).map((reminder) => {
        const due = reminderIsDue(reminder);
        return <article className="hx-row" key={reminder.meta.id}>
          <span className="hx-row-icon">铃</span>
          <div className="hx-row-main"><strong>{subjectLabel(text(reminder, "subjectType"), text(reminder, "subjectId"))}</strong><small>{due ? "已到提醒时间 · " : ""}{displayDateTime(reminderEffectiveAt(reminder))} · {text(reminder, "status")}</small>{text(reminder, "snoozedUntil") && <small>原定 {displayDateTime(text(reminder, "triggerAt"))}，当前已稍后</small>}</div>
          <div className="hx-row-actions">{["scheduled", "fired"].includes(text(reminder, "status")) && <button className="hx-btn secondary" disabled={!online} onClick={() => void snooze(reminder)}>稍后 1h</button>}{["scheduled", "fired"].includes(text(reminder, "status")) && <button className="hx-btn primary" disabled={!online} onClick={() => void dismiss(reminder)}>忽略</button>}{text(reminder, "status") !== "dismissed" && text(reminder, "status") !== "cancelled" && <button className="hx-btn ghost" disabled={!online} onClick={() => void cancel(reminder)}>取消</button>}</div>
        </article>;
      })}{!reminders.length && <Empty title="还没有提醒" description="提醒只是执行对象的时间触发器，不复制任务或日程本身。" />}</div>
    </Panel>

    <Panel eyebrow="NEW REMINDER" title="给现有对象加提醒">
      <form className="hx-form" onSubmit={(event) => void createReminder(event)}>
        <label>对象类型<select value={subjectType} onChange={(event) => { setSubjectType(event.target.value as ExecutionReminderSubject); setSubjectId(""); }}><option value="task">任务</option><option value="calendar_event">日程</option><option value="waiting_item">等待事项</option><option value="memo">备忘</option></select></label>
        <label>对象<select required value={subjectId} onChange={(event) => setSubjectId(event.target.value)}><option value="">选择对象</option>{subjects.map((item) => <option key={item.meta.id} value={item.meta.id}>{subjectType === "memo" ? memoTitle(item) : text(item, "title") || item.meta.id}</option>)}</select></label>
        <label>提醒时间<input required type="datetime-local" value={triggerAt} onChange={(event) => setTriggerAt(event.target.value)} /></label>
        <button className="hx-btn primary" disabled={!online || !subjectId || !triggerAt}>创建提醒</button>
      </form>
      <p className="hx-muted">Web 端当前维护提醒的云端生命周期；系统级后台通知仍由桌面/移动通知执行器负责。</p>
    </Panel>
  </div>;
}

function StructureView({ session, online, run, tasks, openTasks, dependencies, taskMap, setMessage }: {
  session: CloudPageProps["session"];
  online: boolean;
  run: CloudPageProps["run"];
  tasks: JsonEntity[];
  openTasks: JsonEntity[];
  dependencies: JsonEntity[];
  taskMap: Map<string, JsonEntity>;
  setMessage: (value: string) => void;
}) {
  const [parentTaskId, setParentTaskId] = useState("");
  const [subtaskTitle, setSubtaskTitle] = useState("");
  const [dependencyTaskId, setDependencyTaskId] = useState("");
  const [prerequisiteId, setPrerequisiteId] = useState("");

  async function addSubtask(event: FormEvent) {
    event.preventDefault();
    const parent = taskMap.get(parentTaskId);
    if (!parent) return;
    const subtask = createExecutionSubtask(session.user.id, session.session.deviceId, parent, { title: subtaskTitle, priority: "normal" });
    await run((store) => store.upsert("execution.task", subtask));
    setSubtaskTitle("");
    setMessage("子任务已添加，并继承父任务所属计划");
  }

  async function addDependency(event: FormEvent) {
    event.preventDefault();
    if (!dependencyTaskId || !prerequisiteId) return;
    const exists = dependencies.some((item) => text(item, "taskId") === dependencyTaskId && text(item, "dependsOnTaskId") === prerequisiteId);
    if (exists) {
      setMessage("这个前置依赖已经存在");
      return;
    }
    if (dependencyCreatesCycle(dependencyTaskId, prerequisiteId, dependencies)) {
      setMessage("不能添加该依赖：会形成任务依赖环");
      return;
    }
    const dependency = createExecutionTaskDependency(session.user.id, session.session.deviceId, dependencyTaskId, prerequisiteId);
    await run((store) => store.upsert("execution.task_dependency", dependency));
    setPrerequisiteId("");
    setMessage("前置依赖已添加");
  }

  async function removeDependency(dependency: JsonEntity) {
    await run((store) => store.delete("execution.task_dependency", dependency.meta.id));
    setMessage("前置依赖已移除");
  }

  return <div className="hx-content-grid two">
    <Panel eyebrow="TASK STRUCTURE" title="子任务与阻塞关系">
      <div className="hx-list">{openTasks.map((task) => {
        const children = tasks.filter((item) => text(item, "parentTaskId") === task.meta.id);
        const blockers = taskBlockers(task.meta.id, tasks, dependencies);
        const parent = taskMap.get(text(task, "parentTaskId"));
        return <article className="hx-row" key={task.meta.id}><span className="hx-row-icon">{blockers.length ? "阻" : children.length ? "树" : "□"}</span><div className="hx-row-main"><strong>{text(task, "title")}</strong><small>{parent ? `子任务 · 父级 ${text(parent, "title")}` : "顶层任务"} · 子任务 {children.length}</small><small>{blockers.length ? `等待前置：${blockers.map((item) => text(item, "title")).join("、")}` : "当前没有未完成前置依赖"}</small></div></article>;
      })}{!openTasks.length && <Empty title="没有进行中的任务" description="先创建任务，再拆子任务或建立前置依赖。" />}</div>
    </Panel>

    <PageStack>
      <Panel eyebrow="SUBTASK" title="拆一个更小的下一步"><form className="hx-form" onSubmit={(event) => void addSubtask(event)}><label>父任务<select required value={parentTaskId} onChange={(event) => setParentTaskId(event.target.value)}><option value="">选择父任务</option>{openTasks.map((task) => <option key={task.meta.id} value={task.meta.id}>{text(task, "title")}</option>)}</select></label><label>子任务<input required value={subtaskTitle} onChange={(event) => setSubtaskTitle(event.target.value)} placeholder="例如：整理实验数据" /></label><button className="hx-btn primary" disabled={!online || !parentTaskId || !subtaskTitle.trim()}>添加子任务</button></form></Panel>
      <Panel eyebrow="DEPENDENCY" title="建立 finish-before-start 前置依赖"><form className="hx-form" onSubmit={(event) => void addDependency(event)}><label>被阻塞任务<select required value={dependencyTaskId} onChange={(event) => { setDependencyTaskId(event.target.value); setPrerequisiteId(""); }}><option value="">选择任务</option>{openTasks.map((task) => <option key={task.meta.id} value={task.meta.id}>{text(task, "title")}</option>)}</select></label><label>必须先完成<select required value={prerequisiteId} onChange={(event) => setPrerequisiteId(event.target.value)}><option value="">选择前置任务</option>{tasks.filter((task) => task.meta.id !== dependencyTaskId && task.status !== "cancelled").map((task) => <option key={task.meta.id} value={task.meta.id}>{text(task, "title")}{task.status === "done" ? "（已完成）" : ""}</option>)}</select></label><button className="hx-btn primary" disabled={!online || !dependencyTaskId || !prerequisiteId}>添加依赖</button></form><div className="hx-list">{dependencies.map((dependency) => <article className="hx-row" key={dependency.meta.id}><span className="hx-row-icon">→</span><div className="hx-row-main"><strong>{text(taskMap.get(text(dependency, "taskId")) ?? ({} as JsonEntity), "title") || "任务"}</strong><small>必须等待：{text(taskMap.get(text(dependency, "dependsOnTaskId")) ?? ({} as JsonEntity), "title") || "前置任务"}</small></div><button className="hx-btn ghost" disabled={!online} onClick={() => void removeDependency(dependency)}>移除</button></article>)}</div></Panel>
    </PageStack>
  </div>;
}

function CalendarRecurrenceView({ session, online, run, events, occurrences, ruleMap, eventMap, setMessage }: {
  session: CloudPageProps["session"];
  online: boolean;
  run: CloudPageProps["run"];
  events: JsonEntity[];
  occurrences: JsonEntity[];
  ruleMap: Map<string, JsonEntity>;
  eventMap: Map<string, JsonEntity>;
  setMessage: (value: string) => void;
}) {
  const [eventId, setEventId] = useState("");
  const [frequency, setFrequency] = useState<"daily" | "weekly" | "monthly">("weekly");
  const [intervalValue, setIntervalValue] = useState("1");
  const [weekdays, setWeekdays] = useState<number[]>([]);
  const [monthDay, setMonthDay] = useState(String(new Date().getDate()));
  const [untilAt, setUntilAt] = useState("");
  const [maxOccurrences, setMaxOccurrences] = useState("");
  const [moveOccurrenceId, setMoveOccurrenceId] = useState("");
  const [moveDate, setMoveDate] = useState(localDate());
  const [moveTime, setMoveTime] = useState("19:00");
  const [moveDuration, setMoveDuration] = useState("60");

  async function saveRecurrence(event: FormEvent) {
    event.preventDefault();
    const calendarEvent = eventMap.get(eventId);
    if (!calendarEvent) return;
    const existingRule = text(calendarEvent, "recurrenceRuleId") ? ruleMap.get(text(calendarEvent, "recurrenceRuleId")) : undefined;
    const rule = createExecutionRecurrenceRule(session.user.id, session.session.deviceId, {
      frequency,
      intervalValue: Math.max(1, Number(intervalValue) || 1),
      weekdays: frequency === "weekly" ? weekdays : [],
      monthDay: frequency === "monthly" ? Math.max(1, Math.min(31, Number(monthDay) || 1)) : null,
      untilAt: untilAt || null,
      maxOccurrences: maxOccurrences ? Math.max(1, Number(maxOccurrences) || 1) : null,
    }, existingRule?.meta.id);
    const updated = { ...calendarEvent, recurrenceRuleId: rule.meta.id };
    const next = materializeCalendarOccurrences(session.user.id, session.session.deviceId, updated, rule, occurrences, localDate(), 60);
    await run(async (store) => {
      await store.upsert("execution.recurrence_rule", rule);
      await store.upsert("execution.calendar_event", updated);
      return next.length ? (await store.batchUpsert("execution.calendar_occurrence", next)).state : store.snapshot();
    });
    setMessage(`重复日历规则已保存，并生成未来 60 天 ${next.length} 个实例`);
  }

  async function fillOccurrences(calendarEvent: JsonEntity) {
    const rule = ruleMap.get(text(calendarEvent, "recurrenceRuleId"));
    if (!rule) return;
    const next = materializeCalendarOccurrences(session.user.id, session.session.deviceId, calendarEvent, rule, occurrences, localDate(), 60);
    if (!next.length) {
      setMessage("未来 60 天日历实例已经齐全");
      return;
    }
    await run(async (store) => (await store.batchUpsert("execution.calendar_occurrence", next)).state);
    setMessage(`已补齐 ${next.length} 个日历实例`);
  }

  async function clearRecurrence(calendarEvent: JsonEntity) {
    const ruleId = text(calendarEvent, "recurrenceRuleId");
    if (!ruleId) return;
    await run(async (store) => {
      const next = await store.upsert("execution.calendar_event", { ...calendarEvent, recurrenceRuleId: null });
      try { return await store.delete("execution.recurrence_rule", ruleId); }
      catch { return next; }
    });
    setMessage("日历重复规则已关闭，历史 occurrence 保留");
  }

  async function skipOccurrence(occurrence: JsonEntity) {
    await run((store) => store.upsert("execution.calendar_occurrence", { ...occurrence, status: "skipped" }));
    setMessage("本次日历实例已跳过，不影响后续重复安排");
  }

  async function restoreOccurrence(occurrence: JsonEntity) {
    await run((store) => store.upsert("execution.calendar_occurrence", { ...occurrence, status: "scheduled" }));
    setMessage("本次日历实例已恢复");
  }

  async function moveOccurrence(event: FormEvent) {
    event.preventDefault();
    const occurrence = occurrences.find((item) => item.meta.id === moveOccurrenceId);
    if (!occurrence) return;
    const updated = moveCalendarOccurrence(occurrence, moveDate, moveTime, Math.max(5, Number(moveDuration) || 60));
    await run((store) => store.upsert("execution.calendar_occurrence", updated));
    setMoveOccurrenceId("");
    setMessage("只移动了这一次日历实例，重复规则本身没有改变");
  }

  const activeOccurrences = [...occurrences].sort((a, b) => text(a, "occurrenceKey").localeCompare(text(b, "occurrenceKey"))).slice(0, 80);

  return <div className="hx-content-grid two">
    <PageStack>
      <Panel eyebrow="CALENDAR RECURRENCE" title="给日历时间块设置重复规则">
        <form className="hx-form" onSubmit={(event) => void saveRecurrence(event)}>
          <label>日历事件<select required value={eventId} onChange={(event) => { const id = event.target.value; setEventId(id); const item = eventMap.get(id); const rule = item ? ruleMap.get(text(item, "recurrenceRuleId")) : undefined; if (rule) { setFrequency(String(rule.frequency) as typeof frequency); setIntervalValue(String(rule.intervalValue ?? 1)); setWeekdays(Array.isArray(rule.weekdays) ? rule.weekdays.filter((value): value is number => typeof value === "number") : []); setMonthDay(String(rule.monthDay ?? new Date().getDate())); setUntilAt(text(rule, "untilAt").slice(0, 10)); setMaxOccurrences(rule.maxOccurrences ? String(rule.maxOccurrences) : ""); } }}><option value="">选择日历时间块</option>{events.map((item) => <option key={item.meta.id} value={item.meta.id}>{text(item, "title")}</option>)}</select></label>
          <div className="hx-form-grid"><label>频率<select value={frequency} onChange={(event) => setFrequency(event.target.value as typeof frequency)}><option value="daily">每天</option><option value="weekly">每周</option><option value="monthly">每月</option></select></label><label>间隔<input type="number" min="1" value={intervalValue} onChange={(event) => setIntervalValue(event.target.value)} /></label>{frequency === "monthly" && <label>每月第几日<input type="number" min="1" max="31" value={monthDay} onChange={(event) => setMonthDay(event.target.value)} /></label>}<label>结束日期<input type="date" value={untilAt} onChange={(event) => setUntilAt(event.target.value)} /></label><label>最多次数<input type="number" min="1" value={maxOccurrences} onChange={(event) => setMaxOccurrences(event.target.value)} placeholder="不限" /></label></div>
          {frequency === "weekly" && <div className="hx-inline-actions">{WEEKDAYS.map(([day, label]) => <button key={day} type="button" className={`hx-btn ${weekdays.includes(day) ? "primary" : "secondary"}`} onClick={() => setWeekdays((current) => current.includes(day) ? current.filter((item) => item !== day) : [...current, day].sort((a, b) => a - b))}>周{label}</button>)}</div>}
          <button className="hx-btn primary" disabled={!online || !eventId || (frequency === "weekly" && !weekdays.length)}>保存并物化 60 天</button>
        </form>
      </Panel>

      <Panel eyebrow="ACTIVE CALENDAR RULES" title="已启用重复日历"><div className="hx-list">{events.filter((item) => text(item, "recurrenceRuleId")).map((calendarEvent) => { const rule = ruleMap.get(text(calendarEvent, "recurrenceRuleId")); const own = occurrences.filter((item) => text(item, "eventId") === calendarEvent.meta.id); return <article className="hx-row" key={calendarEvent.meta.id}><span className="hx-row-icon">↻</span><div className="hx-row-main"><strong>{text(calendarEvent, "title")}</strong><small>{recurrenceLabel(rule)} · 已物化 {own.length} 次 · 例外 {own.filter((item) => item.status === "skipped" || item.status === "cancelled").length} 次</small></div><div className="hx-row-actions"><button className="hx-btn secondary" disabled={!online} onClick={() => void fillOccurrences(calendarEvent)}>补齐 60 天</button><button className="hx-btn ghost" disabled={!online} onClick={() => void clearRecurrence(calendarEvent)}>关闭重复</button></div></article>; })}{!events.some((item) => text(item, "recurrenceRuleId")) && <Empty title="没有重复日历" description="普通 Timebox 仍是一次性日程；只有明确设置规则后才会产生 calendar occurrence。" />}</div></Panel>
    </PageStack>

    <PageStack>
      <Panel eyebrow="OCCURRENCE EXCEPTIONS" title="只修改某一次，不改整条规则"><div className="hx-list">{activeOccurrences.map((occurrence) => { const calendarEvent = eventMap.get(text(occurrence, "eventId")); return <article className="hx-row" key={occurrence.meta.id}><span className="hx-row-icon">{occurrence.status === "skipped" ? "跳" : "次"}</span><div className="hx-row-main"><strong>{calendarEvent ? text(calendarEvent, "title") : "重复日历"}</strong><small>{text(occurrence, "occurrenceKey")} · {text(occurrence, "startAt") ? displayDateTime(text(occurrence, "startAt")) : text(occurrence, "startLocalDate")} · {text(occurrence, "status")}</small></div><div className="hx-row-actions">{occurrence.status === "scheduled" && <button className="hx-btn secondary" onClick={() => { setMoveOccurrenceId(occurrence.meta.id); setMoveDate(text(occurrence, "occurrenceKey") || localDate()); setMoveTime(text(occurrence, "startAt") ? localDateTimeValue(text(occurrence, "startAt")).slice(11, 16) : "19:00"); }}>只移动本次</button>}{occurrence.status === "scheduled" ? <button className="hx-btn ghost" disabled={!online} onClick={() => void skipOccurrence(occurrence)}>跳过本次</button> : <button className="hx-btn ghost" disabled={!online} onClick={() => void restoreOccurrence(occurrence)}>恢复本次</button>}</div></article>; })}{!activeOccurrences.length && <Empty title="还没有日历实例" description="先给一个日历事件设置重复规则并物化实例。" />}</div></Panel>
      <Panel eyebrow="MOVE ONE OCCURRENCE" title="移动单次实例">{moveOccurrenceId ? <form className="hx-form" onSubmit={(event) => void moveOccurrence(event)}><div className="hx-form-grid"><label>新日期<input type="date" value={moveDate} onChange={(event) => setMoveDate(event.target.value)} /></label><label>新开始时间<input type="time" value={moveTime} onChange={(event) => setMoveTime(event.target.value)} /></label><label>时长（分钟）<input type="number" min="5" step="5" value={moveDuration} onChange={(event) => setMoveDuration(event.target.value)} /></label></div><div className="hx-inline-actions"><button className="hx-btn primary" disabled={!online}>只移动这一次</button><button type="button" className="hx-btn ghost" onClick={() => setMoveOccurrenceId("")}>取消</button></div></form> : <Empty title="选择一个实例" description="移动 occurrence 只修改单次例外，RecurrenceRule 保持不变。" />}</Panel>
    </PageStack>
  </div>;
}
