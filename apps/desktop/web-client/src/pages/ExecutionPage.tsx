import { useMemo, useState, type FormEvent } from "react";
import {
  atomicMutate, createExecutionCalendarEvent, createExecutionCompletionResult, createExecutionMemo,
  createExecutionWeeklyReview,
  createExecutionProject, createExecutionRecurrenceRule, createExecutionTask,
  createMemoConversionLinks, executionTaskDate, isOpenExecutionTask, localDate,
  materializeTaskOccurrences, recurrenceLabel, taskIsInbox, taskMatchesToday,
  taskPriorityLabel, type JsonEntity,
} from "../core";
import { navigate } from "../navigation";
import { Empty, Metric, MetricGrid, Notice, PageStack, Panel, Toolbar, entities, number, text, type CloudPageProps } from "../ui";

type ExecutionView = "today" | "inbox" | "projects" | "memos" | "recurrence" | "review" | "completed";
type CaptureType = "task" | "memo";

const VIEW_LABELS: Array<[ExecutionView, string]> = [
  ["today", "今天"], ["inbox", "收件箱"], ["projects", "计划"], ["memos", "备忘"],
  ["recurrence", "重复"], ["review", "回顾"], ["completed", "已完成"],
];
const WEEKDAYS = [[1, "一"], [2, "二"], [3, "三"], [4, "四"], [5, "五"], [6, "六"], [7, "日"]] as const;

function sorted(items: JsonEntity[]): JsonEntity[] {
  return [...items].sort((left, right) => right.meta.updatedAt.localeCompare(left.meta.updatedAt));
}

function dueAt(date: string): string | null {
  if (!date) return null;
  const value = new Date(`${date}T23:59:00`);
  return Number.isNaN(value.getTime()) ? null : value.toISOString();
}

function displayDateTime(value: string): string {
  if (!value) return "未安排";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

function memoTitle(memo: JsonEntity): string {
  return (text(memo, "plainText") || text(memo, "content") || "备忘").trim().slice(0, 80);
}

function dayOffset(date: Date, offset: number): string {
  return localDate(new Date(date.getFullYear(), date.getMonth(), date.getDate() + offset));
}

function inDayRange(value: string | null, start: string, end: string): boolean {
  return Boolean(value && value >= start && value <= end);
}

export function ExecutionPage({ session, state, run, online }: CloudPageProps) {
  const [view, setView] = useState<ExecutionView>("today");
  const [captureType, setCaptureType] = useState<CaptureType>("task");
  const [capture, setCapture] = useState("");
  const [message, setMessage] = useState("");
  const [taskTitle, setTaskTitle] = useState("");
  const [taskDescription, setTaskDescription] = useState("");
  const [taskProject, setTaskProject] = useState("");
  const [taskDate, setTaskDate] = useState(localDate());
  const [taskPriority, setTaskPriority] = useState("normal");
  const [taskMinutes, setTaskMinutes] = useState("30");
  const [projectName, setProjectName] = useState("");
  const [projectDescription, setProjectDescription] = useState("");
  const [recurrenceTaskId, setRecurrenceTaskId] = useState("");
  const [frequency, setFrequency] = useState<"daily" | "weekly" | "monthly">("weekly");
  const [intervalValue, setIntervalValue] = useState("1");
  const [weekdays, setWeekdays] = useState<number[]>([]);
  const [monthDay, setMonthDay] = useState(String(new Date().getDate()));
  const [recurrenceUntil, setRecurrenceUntil] = useState("");
  const [maxOccurrences, setMaxOccurrences] = useState("");
  const [memoCalendarId, setMemoCalendarId] = useState("");
  const [memoDate, setMemoDate] = useState(localDate());
  const [memoTime, setMemoTime] = useState("19:00");
  const [memoDuration, setMemoDuration] = useState("30");
  const [reviewNote, setReviewNote] = useState("");

  const projects = useMemo(
    () => sorted(entities(state, "execution.project")).filter((item) => item.status !== "archived" && item.status !== "cancelled"),
    [state],
  );
  const tasks = useMemo(() => sorted(entities(state, "execution.task")), [state]);
  const memos = useMemo(() => sorted(entities(state, "execution.memo")).filter((item) => item.status !== "archived"), [state]);
  const rules = useMemo(() => entities(state, "execution.recurrence_rule"), [state]);
  const occurrences = useMemo(() => entities(state, "execution.task_occurrence"), [state]);
  const completionResults = entities(state, "execution.completion_result");
  const weeklyReviews = sorted(entities(state, "execution.weekly_review"));
  const openTasks = tasks.filter(isOpenExecutionTask);
  const recurringTaskIds = new Set(tasks.filter((item) => text(item, "recurrenceRuleId")).map((item) => item.meta.id));
  const today = localDate();
  const todayTasks = tasks.filter((item) => !recurringTaskIds.has(item.meta.id) && taskMatchesToday(item));
  const todayOccurrences = occurrences.filter((item) => text(item, "occurrenceKey") === today && item.status !== "skipped");
  const todayOpen = todayTasks.filter(isOpenExecutionTask);
  const todayDone = todayTasks.filter((item) => item.status === "done");
  const todayPendingOccurrences = todayOccurrences.filter((item) => item.status !== "completed");
  const todayCompletedOccurrences = todayOccurrences.filter((item) => item.status === "completed");
  const todayTotal = todayTasks.length + todayOccurrences.length;
  const todayCompleted = todayDone.length + todayCompletedOccurrences.length;
  const inbox = tasks.filter(taskIsInbox);
  const completed = tasks.filter((item) => item.status === "done");
  const projectMap = new Map(projects.map((item) => [item.meta.id, item]));
  const taskMap = new Map(tasks.map((item) => [item.meta.id, item]));
  const ruleMap = new Map(rules.map((item) => [item.meta.id, item]));
  const todayCompletion = todayTotal ? Math.round((todayCompleted / todayTotal) * 100) : 0;

  async function quickCapture(event: FormEvent) {
    event.preventDefault();
    const value = capture.trim();
    if (!value) return;
    if (captureType === "memo") {
      const memo = createExecutionMemo(session.user.id, session.session.deviceId, value, "inbox");
      await run((store) => store.upsert("execution.memo", memo));
      setMessage("已保存到备忘时间流");
    } else {
      const task = createExecutionTask(session.user.id, session.session.deviceId, { title: value, context: "inbox" });
      await run((store) => store.upsert("execution.task", task));
      setMessage("已加入任务收件箱");
    }
    setCapture("");
  }

  async function createTask(event: FormEvent) {
    event.preventDefault();
    const task = createExecutionTask(session.user.id, session.session.deviceId, {
      title: taskTitle,
      description: taskDescription,
      projectId: taskProject || null,
      priority: taskPriority as "low" | "normal" | "high" | "urgent",
      estimatedMinutes: taskMinutes ? Math.max(0, Number(taskMinutes)) : null,
      dueAt: dueAt(taskDate),
      context: taskProject ? null : "inbox",
    });
    await run((store) => store.upsert("execution.task", task));
    setTaskTitle("");
    setTaskDescription("");
    setMessage("任务已创建并同步");
  }

  async function createProject(event: FormEvent) {
    event.preventDefault();
    const project = createExecutionProject(session.user.id, session.session.deviceId, { name: projectName, description: projectDescription });
    await run((store) => store.upsert("execution.project", project));
    setProjectName("");
    setProjectDescription("");
    setMessage("计划已创建");
  }

  async function setTaskStatus(task: JsonEntity, status: "todo" | "in_progress" | "done" | "cancelled") {
    const now = new Date().toISOString();
    const updated: JsonEntity = {
      ...task,
      status,
      completedAt: status === "done" ? now : status === "todo" || status === "in_progress" ? null : task.completedAt,
      cancelledAt: status === "cancelled" ? now : null,
    };
    await run(async (store) => {
      if (status === "done" && !text(task, "recurrenceRuleId")) {
        const existing = completionResults.find((item) => text(item, "taskId") === task.meta.id);
        if (!existing) {
          const result = createExecutionCompletionResult(session.user.id, session.session.deviceId, task.meta.id, typeof task.actualMinutes === "number" ? task.actualMinutes : null);
          return atomicMutate(store, [
            { operation: "upsert", entityType: "execution.task", entity: updated },
            { operation: "upsert", entityType: "execution.completion_result", entity: result },
          ]);
        }
      }
      return store.upsert("execution.task", updated);
    });
  }

  async function completeOccurrence(occurrence: JsonEntity) {
    await run((store) => store.upsert("execution.task_occurrence", { ...occurrence, status: "completed", completedAt: new Date().toISOString(), skippedAt: null }));
    setMessage("本次重复任务已完成，后续实例不会被覆盖");
  }

  async function arrangeToday(task: JsonEntity) {
    const updated: JsonEntity = { ...task, dueAt: dueAt(today), context: task.context === "inbox" ? null : task.context };
    await run((store) => store.upsert("execution.task", updated));
    setMessage("已安排到今天");
  }

  async function pinMemo(memo: JsonEntity) {
    await run((store) => store.upsert("execution.memo", { ...memo, isPinned: memo.isPinned !== true }));
  }

  async function archiveMemo(memo: JsonEntity) {
    await run((store) => store.upsert("execution.memo", { ...memo, status: "archived", archivedAt: new Date().toISOString() }));
  }

  async function memoToTask(memo: JsonEntity) {
    const target = createExecutionTask(session.user.id, session.session.deviceId, {
      title: memoTitle(memo),
      description: text(memo, "content"),
      context: text(memo, "context") || null,
    });
    const [forward, reverse] = createMemoConversionLinks(session.user.id, session.session.deviceId, memo.meta.id, "task", target.meta.id);
    await run((store) => atomicMutate(store, [
      { operation: "upsert", entityType: "execution.task", entity: target },
      { operation: "upsert", entityType: "execution.entity_link", entity: forward },
      { operation: "upsert", entityType: "execution.entity_link", entity: reverse },
      { operation: "upsert", entityType: "execution.memo", entity: { ...memo, status: "archived", archivedAt: new Date().toISOString() } },
    ]));
    setMessage("备忘已转换成任务，并保留来源关系");
  }

  async function memoToCalendar(event: FormEvent) {
    event.preventDefault();
    const memo = memos.find((item) => item.meta.id === memoCalendarId);
    if (!memo) return;
    const start = new Date(`${memoDate}T${memoTime}:00`);
    if (Number.isNaN(start.getTime())) return;
    const end = new Date(start.getTime() + Math.max(5, Number(memoDuration) || 30) * 60_000);
    const target = createExecutionCalendarEvent(session.user.id, session.session.deviceId, { title: memoTitle(memo), description: text(memo, "content"), startAt: start.toISOString(), endAt: end.toISOString() });
    const [forward, reverse] = createMemoConversionLinks(session.user.id, session.session.deviceId, memo.meta.id, "calendar_event", target.meta.id);
    await run((store) => atomicMutate(store, [
      { operation: "upsert", entityType: "execution.calendar_event", entity: target },
      { operation: "upsert", entityType: "execution.entity_link", entity: forward },
      { operation: "upsert", entityType: "execution.entity_link", entity: reverse },
      { operation: "upsert", entityType: "execution.memo", entity: { ...memo, status: "archived", archivedAt: new Date().toISOString() } },
    ]));
    setMemoCalendarId("");
    setMessage("备忘已转换成日历时间块，并保留来源关系");
  }

  async function saveRecurrence(event: FormEvent) {
    event.preventDefault();
    const task = tasks.find((item) => item.meta.id === recurrenceTaskId);
    if (!task) return;
    const existingRule = text(task, "recurrenceRuleId") ? ruleMap.get(text(task, "recurrenceRuleId")) : undefined;
    const rule = createExecutionRecurrenceRule(session.user.id, session.session.deviceId, {
      frequency,
      intervalValue: Math.max(1, Number(intervalValue) || 1),
      weekdays: frequency === "weekly" ? weekdays : [],
      monthDay: frequency === "monthly" ? Math.max(1, Math.min(31, Number(monthDay) || 1)) : null,
      untilAt: recurrenceUntil || null,
      maxOccurrences: maxOccurrences ? Math.max(1, Number(maxOccurrences) || 1) : null,
    }, existingRule?.meta.id);
    const anchored: JsonEntity = executionTaskDate(task) ? task : { ...task, dueAt: dueAt(today) };
    const updated: JsonEntity = { ...anchored, recurrenceRuleId: rule.meta.id, context: anchored.context === "inbox" ? null : anchored.context };
    const nextOccurrences = materializeTaskOccurrences(session.user.id, session.session.deviceId, updated, rule, occurrences, today, 30);
    await run((store) => atomicMutate(store, [
      { operation: "upsert", entityType: "execution.recurrence_rule", entity: rule },
      { operation: "upsert", entityType: "execution.task", entity: updated },
      ...nextOccurrences.map((entity) => ({ operation: "upsert" as const, entityType: "execution.task_occurrence" as const, entity })),
    ]));
    setMessage(`重复规则已保存，并生成未来 30 天内 ${nextOccurrences.length} 个新实例`);
  }

  async function materialize(task: JsonEntity) {
    const rule = ruleMap.get(text(task, "recurrenceRuleId"));
    if (!rule) return;
    const next = materializeTaskOccurrences(session.user.id, session.session.deviceId, task, rule, occurrences, today, 30);
    if (!next.length) {
      setMessage("未来 30 天实例已经齐全");
      return;
    }
    await run(async (store) => (await store.batchUpsert("execution.task_occurrence", next)).state);
    setMessage(`已补齐 ${next.length} 个重复任务实例`);
  }

  async function clearRecurrence(task: JsonEntity) {
    const ruleId = text(task, "recurrenceRuleId");
    if (!ruleId) return;
    await run((store) => atomicMutate(store, [
      { operation: "upsert", entityType: "execution.task", entity: { ...task, recurrenceRuleId: null } },
      { operation: "delete", entityType: "execution.recurrence_rule", entityId: ruleId },
    ]));
    setMessage("重复规则已关闭；历史 occurrence 保留");
  }

  const visibleTasks = view === "today" ? todayOpen : view === "inbox" ? inbox : view === "completed" ? completed : openTasks;
  const reviewStart = dayOffset(new Date(), -6);
  const plannedTasks = tasks.filter((item) => !text(item, "recurrenceRuleId") && inDayRange(executionTaskDate(item), reviewStart, today));
  const reviewOccurrences = occurrences.filter((item) => inDayRange(text(item, "occurrenceKey"), reviewStart, today) && item.status !== "skipped");
  const reviewPlanned = plannedTasks.length + reviewOccurrences.length;
  const reviewCompleted = plannedTasks.filter((item) => item.status === "done").length + reviewOccurrences.filter((item) => item.status === "completed").length;
  const reviewRate = reviewPlanned ? Math.round((reviewCompleted / reviewPlanned) * 100) : 0;
  const plannedMinutes = plannedTasks.reduce((sum, item) => sum + number(item, "estimatedMinutes"), 0) + reviewOccurrences.reduce((sum, item) => sum + number(taskMap.get(text(item, "taskId")) ?? ({ meta: {} } as JsonEntity), "estimatedMinutes"), 0);
  const actualMinutes = completionResults.filter((item) => inDayRange(text(item, "completedAt").slice(0, 10), reviewStart, today)).reduce((sum, item) => sum + number(item, "actualMinutes"), 0);
  const overdueTasks = openTasks.filter((item) => !text(item, "recurrenceRuleId") && Boolean(executionTaskDate(item) && executionTaskDate(item)! < today));
  const overdueOccurrences = occurrences.filter((item) => item.status === "pending" && text(item, "occurrenceKey") < today);
  const currentWeeklyReview = weeklyReviews.find((item) => text(item, "weekStart") === reviewStart && text(item, "weekEnd") === today);

  async function saveWeeklyReview() {
    if (currentWeeklyReview) {
      setMessage("当前 7 日区间已经保存过快照");
      return;
    }
    const review = createExecutionWeeklyReview(session.user.id, session.session.deviceId, {
      weekStart: reviewStart, weekEnd: today, plannedCount: reviewPlanned, completedCount: reviewCompleted,
      completionRate: reviewRate, plannedMinutes, actualMinutes, overdueTaskCount: overdueTasks.length,
      overdueOccurrenceCount: overdueOccurrences.length, note: reviewNote,
    });
    await run((store) => store.upsert("execution.weekly_review", review));
    setReviewNote("");
    setMessage("本周执行复盘快照已持久化并同步");
  }

  return <PageStack>
    <Toolbar>
      {VIEW_LABELS.map(([key, label]) => <button key={key} className={`hx-btn ${view === key ? "primary" : "secondary"}`} onClick={() => setView(key)}>{label}</button>)}
      <button className="hx-btn ghost" onClick={() => navigate("/calendar")}>打开 Timebox 日历</button>
    </Toolbar>

    <MetricGrid>
      <Metric label="今日行动" value={`${todayCompleted} / ${todayTotal}`} detail={todayTotal - todayCompleted ? `还有 ${todayTotal - todayCompleted} 项` : todayTotal ? "今天已清空" : "今天还未安排任务"} positive={todayTotal > 0 && todayCompleted === todayTotal} />
      <Metric label="今日完成率" value={`${todayCompletion}%`} detail={`普通任务 ${todayTasks.length} · 重复实例 ${todayOccurrences.length}`} positive={todayCompletion === 100 && todayTotal > 0} />
      <Metric label="收件箱" value={String(inbox.length)} detail="待整理的临时任务" />
      <Metric label="计划 / 备忘" value={`${projects.length} / ${memos.length}`} detail="活跃计划与备忘" />
    </MetricGrid>

    {message && <Notice kind="success">{message}</Notice>}

    {(view === "today" || view === "inbox") && <div className="hx-content-grid two">
      <Panel eyebrow="QUICK CAPTURE" title="先记下来，再决定放哪里">
        <form className="hx-form" onSubmit={(event) => void quickCapture(event)}>
          <div className="hx-inline-actions"><button type="button" className={`hx-btn ${captureType === "task" ? "primary" : "secondary"}`} onClick={() => setCaptureType("task")}>任务</button><button type="button" className={`hx-btn ${captureType === "memo" ? "primary" : "secondary"}`} onClick={() => setCaptureType("memo")}>备忘</button></div>
          <label>{captureType === "task" ? "任务内容" : "备忘内容"}<textarea required rows={3} value={capture} onChange={(event) => setCapture(event.target.value)} placeholder={captureType === "task" ? "例如：周末整理本月账单" : "例如：突然想到的想法、灵感或临时信息"} /></label>
          <button className="hx-btn primary" disabled={!online}>立即收集</button>
        </form>
      </Panel>

      <Panel eyebrow="NEW TASK" title="安排一个明确任务">
        <form className="hx-form" onSubmit={(event) => void createTask(event)}>
          <label>任务名称<input required value={taskTitle} onChange={(event) => setTaskTitle(event.target.value)} placeholder="要完成什么？" /></label>
          <label>说明<textarea rows={2} value={taskDescription} onChange={(event) => setTaskDescription(event.target.value)} /></label>
          <div className="hx-form-grid"><label>所属计划<select value={taskProject} onChange={(event) => setTaskProject(event.target.value)}><option value="">收件箱 / 独立任务</option>{projects.map((project) => <option key={project.meta.id} value={project.meta.id}>{text(project, "name")}</option>)}</select></label><label>计划日期<input type="date" value={taskDate} onChange={(event) => setTaskDate(event.target.value)} /></label><label>优先级<select value={taskPriority} onChange={(event) => setTaskPriority(event.target.value)}><option value="low">低</option><option value="normal">普通</option><option value="high">高</option><option value="urgent">紧急</option></select></label><label>预计分钟<input type="number" min="0" value={taskMinutes} onChange={(event) => setTaskMinutes(event.target.value)} /></label></div>
          <button className="hx-btn primary" disabled={!online}>创建任务</button>
        </form>
      </Panel>
    </div>}

    {view === "today" && todayPendingOccurrences.length > 0 && <Panel eyebrow="RECURRING TODAY" title="今天的重复任务实例">
      <div className="hx-list">{todayPendingOccurrences.map((occurrence) => { const task = taskMap.get(text(occurrence, "taskId")); return <article className="hx-row" key={occurrence.meta.id}><span className="hx-row-icon">↻</span><div className="hx-row-main"><strong>{task ? text(task, "title") : "重复任务"}</strong><small>{task ? taskPriorityLabel(task) : "普通"} · {text(occurrence, "scheduledStartAt") ? displayDateTime(text(occurrence, "scheduledStartAt")) : "今天"}</small></div><button className="hx-btn primary" disabled={!online} onClick={() => void completeOccurrence(occurrence)}>完成本次</button></article>; })}</div>
    </Panel>}

    {view !== "projects" && view !== "memos" && view !== "recurrence" && view !== "review" && <Panel eyebrow={view.toUpperCase()} title={view === "today" ? "今天的普通任务" : view === "inbox" ? "等待整理" : view === "completed" ? "完成历史" : "全部进行中任务"}>
      <div className="hx-list">
        {visibleTasks.map((task) => <article className="hx-row" key={task.meta.id}><span className="hx-row-icon">{task.status === "done" ? "✓" : task.status === "in_progress" ? "→" : "□"}</span><div className="hx-row-main"><strong>{text(task, "title")}</strong><small>{projectMap.get(text(task, "projectId")) ? `${text(projectMap.get(text(task, "projectId"))!, "name")} · ` : ""}优先级 {taskPriorityLabel(task)} · {text(task, "dueAt") || text(task, "scheduledStartAt") ? displayDateTime(text(task, "scheduledStartAt") || text(task, "dueAt")) : "未安排日期"}</small>{text(task, "description") && <small>{text(task, "description")}</small>}</div><div className="hx-row-actions">{view === "inbox" && <button className="hx-btn ghost" disabled={!online} onClick={() => void arrangeToday(task)}>安排今天</button>}{task.status === "todo" && <button className="hx-btn secondary" disabled={!online} onClick={() => void setTaskStatus(task, "in_progress")}>开始</button>}{isOpenExecutionTask(task) && !text(task, "recurrenceRuleId") && <button className="hx-btn primary" disabled={!online} onClick={() => void setTaskStatus(task, "done")}>完成</button>}{isOpenExecutionTask(task) && <button className="hx-btn ghost" disabled={!online} onClick={() => void setTaskStatus(task, "cancelled")}>取消</button>}</div></article>)}
        {!visibleTasks.length && <Empty title={view === "today" ? "今天没有普通待办" : view === "inbox" ? "收件箱已清空" : "暂无任务"} description={view === "today" ? "重复任务会显示在上方独立实例区；普通任务可从收件箱安排到今天。" : "使用快速收集，把脑中的事情先放进 LifeTrace。"} />}
      </div>
    </Panel>}

    {view === "projects" && <div className="hx-content-grid two">
      <Panel eyebrow="PLANS" title="计划与项目"><div className="hx-list">{projects.map((project) => { const own = tasks.filter((task) => text(task, "projectId") === project.meta.id); const ownDone = own.filter((task) => task.status === "done").length; return <article className="hx-row" key={project.meta.id}><span className="hx-row-icon">计</span><div className="hx-row-main"><strong>{text(project, "name")}</strong><small>{text(project, "description") || "暂无说明"}</small><small>{ownDone} / {own.length} 个任务已完成</small></div></article>; })}{!projects.length && <Empty title="还没有计划" description="用计划把长期事项拆成可执行任务，而不是只留下一个模糊目标。" />}</div></Panel>
      <Panel eyebrow="NEW PLAN" title="创建计划"><form className="hx-form" onSubmit={(event) => void createProject(event)}><label>计划名称<input required value={projectName} onChange={(event) => setProjectName(event.target.value)} placeholder="例如：完成毕业论文" /></label><label>说明<textarea rows={4} value={projectDescription} onChange={(event) => setProjectDescription(event.target.value)} /></label><button className="hx-btn primary" disabled={!online}>创建计划</button></form></Panel>
    </div>}

    {view === "memos" && <div className="hx-content-grid two">
      <Panel eyebrow="MEMO TIMELINE" title="备忘时间流"><div className="hx-list">{[...memos].sort((a, b) => Number(b.isPinned === true) - Number(a.isPinned === true) || b.meta.updatedAt.localeCompare(a.meta.updatedAt)).map((memo) => <article className="hx-row" key={memo.meta.id}><span className="hx-row-icon">{memo.isPinned === true ? "置" : "记"}</span><div className="hx-row-main"><strong>{memoTitle(memo)}</strong><small>{new Date(memo.meta.createdAt).toLocaleString("zh-CN")}{text(memo, "context") ? ` · ${text(memo, "context")}` : ""}</small></div><div className="hx-row-actions"><button className="hx-btn secondary" disabled={!online} onClick={() => void memoToTask(memo)}>转任务</button><button className="hx-btn secondary" disabled={!online} onClick={() => setMemoCalendarId(memo.meta.id)}>转日程</button><button className="hx-btn ghost" onClick={() => void pinMemo(memo)}>{memo.isPinned === true ? "取消置顶" : "置顶"}</button><button className="hx-btn ghost" onClick={() => void archiveMemo(memo)}>归档</button></div></article>)}{!memos.length && <Empty title="还没有备忘" description="备忘不要求标题和分类，先快速记录，需要执行时再转换。" />}</div></Panel>
      <Panel eyebrow="MEMO → CALENDAR" title="把备忘变成时间块">{memoCalendarId ? <form className="hx-form" onSubmit={(event) => void memoToCalendar(event)}><p className="hx-muted">{memoTitle(memos.find((item) => item.meta.id === memoCalendarId) ?? ({ meta: {} } as JsonEntity))}</p><div className="hx-form-grid"><label>日期<input type="date" value={memoDate} onChange={(event) => setMemoDate(event.target.value)} /></label><label>开始时间<input type="time" value={memoTime} onChange={(event) => setMemoTime(event.target.value)} /></label><label>时长（分钟）<input type="number" min="5" step="5" value={memoDuration} onChange={(event) => setMemoDuration(event.target.value)} /></label></div><div className="hx-inline-actions"><button className="hx-btn primary" disabled={!online}>转换并归档原备忘</button><button type="button" className="hx-btn ghost" onClick={() => setMemoCalendarId("")}>取消</button></div></form> : <Empty title="选择一条备忘" description="点击“转日程”后设置日期和时间。转换会写入双向来源关系，并归档原备忘。" />}</Panel>
    </div>}

    {view === "recurrence" && <div className="hx-content-grid two">
      <Panel eyebrow="RECURRENCE RULE" title="重复任务规则">
        <form className="hx-form" onSubmit={(event) => void saveRecurrence(event)}>
          <label>任务<select required value={recurrenceTaskId} onChange={(event) => { const id = event.target.value; setRecurrenceTaskId(id); const task = tasks.find((item) => item.meta.id === id); const rule = task ? ruleMap.get(text(task, "recurrenceRuleId")) : undefined; if (rule) { setFrequency(String(rule.frequency) as typeof frequency); setIntervalValue(String(rule.intervalValue ?? 1)); setWeekdays(Array.isArray(rule.weekdays) ? rule.weekdays.filter((value): value is number => typeof value === "number") : []); setMonthDay(String(rule.monthDay ?? new Date().getDate())); setRecurrenceUntil(text(rule, "untilAt").slice(0, 10)); setMaxOccurrences(rule.maxOccurrences ? String(rule.maxOccurrences) : ""); } }}><option value="">选择任务</option>{openTasks.map((task) => <option key={task.meta.id} value={task.meta.id}>{text(task, "title")}</option>)}</select></label>
          <div className="hx-form-grid"><label>频率<select value={frequency} onChange={(event) => setFrequency(event.target.value as typeof frequency)}><option value="daily">每天</option><option value="weekly">每周</option><option value="monthly">每月</option></select></label><label>间隔<input type="number" min="1" value={intervalValue} onChange={(event) => setIntervalValue(event.target.value)} /></label>{frequency === "monthly" && <label>每月第几日<input type="number" min="1" max="31" value={monthDay} onChange={(event) => setMonthDay(event.target.value)} /></label>}<label>结束日期<input type="date" value={recurrenceUntil} onChange={(event) => setRecurrenceUntil(event.target.value)} /></label><label>最多次数<input type="number" min="1" value={maxOccurrences} onChange={(event) => setMaxOccurrences(event.target.value)} placeholder="不限" /></label></div>
          {frequency === "weekly" && <div className="hx-inline-actions" aria-label="重复星期">{WEEKDAYS.map(([day, label]) => <button key={day} type="button" className={`hx-btn ${weekdays.includes(day) ? "primary" : "secondary"}`} aria-pressed={weekdays.includes(day)} onClick={() => setWeekdays((current) => current.includes(day) ? current.filter((item) => item !== day) : [...current, day].sort((a, b) => a - b))}>周{label}</button>)}</div>}
          <button className="hx-btn primary" disabled={!online || !recurrenceTaskId || (frequency === "weekly" && !weekdays.length)}>保存规则并生成未来 30 天实例</button>
        </form>
      </Panel>
      <Panel eyebrow="ACTIVE RULES" title="已启用的重复任务"><div className="hx-list">{tasks.filter((task) => text(task, "recurrenceRuleId")).map((task) => { const rule = ruleMap.get(text(task, "recurrenceRuleId")); const own = occurrences.filter((item) => text(item, "taskId") === task.meta.id); return <article className="hx-row" key={task.meta.id}><span className="hx-row-icon">↻</span><div className="hx-row-main"><strong>{text(task, "title")}</strong><small>{recurrenceLabel(rule)} · 已物化 {own.length} 次 · 已完成 {own.filter((item) => item.status === "completed").length} 次</small></div><div className="hx-row-actions"><button className="hx-btn secondary" disabled={!online} onClick={() => void materialize(task)}>补齐 30 天</button><button className="hx-btn ghost" disabled={!online} onClick={() => void clearRecurrence(task)}>关闭重复</button></div></article>; })}{!tasks.some((task) => text(task, "recurrenceRuleId")) && <Empty title="还没有重复任务" description="选择一个任务设置每天、每周或每月规则。每次执行会写成独立 occurrence，完成历史不会被下一次覆盖。" />}</div></Panel>
    </div>}

    {view === "review" && <PageStack>
      <MetricGrid><Metric label="近 7 日计划" value={String(reviewPlanned)} detail={`${reviewStart} 至 ${today}`} /><Metric label="近 7 日完成" value={String(reviewCompleted)} detail={`计划完成率 ${reviewRate}%`} positive={reviewRate >= 80} /><Metric label="计划投入" value={`${plannedMinutes} 分钟`} detail="基于任务预计时间" /><Metric label="已记录实际" value={`${actualMinutes} 分钟`} detail="来自 completion_result" positive={actualMinutes > 0} /></MetricGrid>
      <div className="hx-content-grid two"><Panel eyebrow="MISSED" title="需要重新决定的事项"><div className="hx-list">{overdueTasks.map((task) => <article className="hx-row" key={task.meta.id}><span className="hx-row-icon">!</span><div className="hx-row-main"><strong>{text(task, "title")}</strong><small>原计划 {executionTaskDate(task)}</small></div><button className="hx-btn secondary" disabled={!online} onClick={() => void arrangeToday(task)}>改到今天</button></article>)}{overdueOccurrences.map((occurrence) => <article className="hx-row" key={occurrence.meta.id}><span className="hx-row-icon">↻</span><div className="hx-row-main"><strong>{text(taskMap.get(text(occurrence, "taskId")) ?? ({ meta: {} } as JsonEntity), "title") || "重复任务"}</strong><small>{text(occurrence, "occurrenceKey")} 的实例仍未完成</small></div></article>)}{!overdueTasks.length && !overdueOccurrences.length && <Empty title="没有逾期遗留" description="近期待办都已完成、取消或仍在未来。" />}</div></Panel><Panel eyebrow="WEEKLY SNAPSHOT" title="保存执行复盘快照"><p className="hx-muted">指标仍直接读取 Task、Occurrence、Completion Result 和 Timebox；保存时只冻结这一周的结果，不复制任务。</p><label>本周备注<textarea rows={3} value={reviewNote} onChange={(event) => setReviewNote(event.target.value)} placeholder="本周最值得保留的经验、下周调整……" /></label><div className="hx-inline-actions"><button className="hx-btn primary" disabled={!online || Boolean(currentWeeklyReview)} onClick={() => void saveWeeklyReview()}>{currentWeeklyReview ? "本周快照已保存" : "保存本周快照"}</button><button className="hx-btn secondary" onClick={() => navigate("/review")}>写主观每日复盘</button></div>{weeklyReviews.length > 0 && <div className="hx-list">{weeklyReviews.slice(0, 4).map((review) => <article className="hx-row" key={review.meta.id}><span className="hx-row-icon">周</span><div className="hx-row-main"><strong>{text(review, "weekStart")} – {text(review, "weekEnd")}</strong><small>完成 {number(review, "completedCount")} / {number(review, "plannedCount")} · {number(review, "completionRate")}% · 实际 {number(review, "actualMinutes")} 分钟</small>{text(review, "note") && <small>{text(review, "note")}</small>}</div></article>)}</div>}</Panel></div>
    </PageStack>}
  </PageStack>;
}
