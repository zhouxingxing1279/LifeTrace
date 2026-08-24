import { useCallback, useEffect, useMemo, useState, type MouseEvent as ReactMouseEvent } from "react";
import {
  Archive,
  Bell,
  CalendarDays,
  Check,
  ChevronRight,
  CircleDot,
  Clock3,
  FolderKanban,
  Inbox,
  ListTodo,
  LoaderCircle,
  Pin,
  Pencil,
  Plus,
  RefreshCw,
  Repeat2,
  RotateCcw,
  Search,
  Trash2,
  Users,
  X,
} from "lucide-react";
import TaskAdvancedPanel from "@/src/components/feature/execution/TaskAdvancedPanel";
import SubjectReminderPanel from "@/src/components/feature/execution/SubjectReminderPanel";
import MemoConvertPanel from "@/src/components/feature/execution/MemoConvertPanel";
import CalendarWorkspace from "@/src/components/feature/execution/CalendarWorkspace";
import CalendarConflictDialog, { type CalendarConflict } from "@/src/components/feature/execution/CalendarConflictDialog";
import CalendarRecurrencePanel from "@/src/components/feature/execution/CalendarRecurrencePanel";
import ExecutionContextMenu, { type ExecutionMenuItem } from "@/src/components/feature/execution/ExecutionContextMenu";
import { preserveTaskUpdateFields, waitingToTaskInput } from "@/src/components/feature/execution/executionViewModel";
import {
  browserTimezone,
  executionApi,
  localDateTimeToRfc3339,
  rfc3339ToLocalDateTime,
  type CalendarEvent,
  type CalendarInput,
  type CalendarTimingInput,
  type ExecutionProject,
  type ExecutionTask,
  type ExecutionTaskPriority,
  type ExecutionTaskStatus,
  type Memo,
  type Reminder,
  type TaskInput,
  type WaitingInput,
  type WaitingItem,
} from "@/src/services/executionApi";

const tabs = [
  ["today", "今天", CircleDot],
  ["tasks", "任务", ListTodo],
  ["projects", "项目", FolderKanban],
  ["calendar", "日历", CalendarDays],
  ["waiting", "等待", Users],
  ["memos", "Memo", Inbox],
] as const;

type Tab = (typeof tabs)[number][0];
type Editor =
  | { kind: "task"; value?: ExecutionTask }
  | { kind: "project"; value?: ExecutionProject }
  | { kind: "calendar"; value?: CalendarEvent; sourceTask?: ExecutionTask }
  | { kind: "waiting"; value?: WaitingItem }
  | { kind: "memo"; value?: Memo }
  | null;

type ReminderSubject = { subjectType: Reminder["subjectType"]; subjectId: string; title: string };
type ContextMenuState = { x: number; y: number; items: ExecutionMenuItem[] } | null;
type PendingCalendarAction = { title: string; conflicts: CalendarConflict[]; action: () => Promise<unknown>; success: string } | null;

type Data = {
  projects: ExecutionProject[];
  tasks: ExecutionTask[];
  calendar: CalendarEvent[];
  waiting: WaitingItem[];
  memos: Memo[];
  reminders: Reminder[];
};

const emptyData: Data = {
  projects: [],
  tasks: [],
  calendar: [],
  waiting: [],
  memos: [],
  reminders: [],
};

function toast(message: string, type: "success" | "error" = "success") {
  window.dispatchEvent(
    new CustomEvent("hengxu-toast", {
      detail: { message, type, duration: type === "error" ? 4500 : 2500 },
    }),
  );
}

function isToday(value?: string | null) {
  if (!value) return false;
  const date = new Date(value);
  const now = new Date();
  return (
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate()
  );
}

function isOverdue(value?: string | null) {
  return Boolean(value && new Date(value).getTime() < Date.now());
}

function formatDateTime(value?: string | null) {
  if (!value) return "未设置";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function taskStatusLabel(status: ExecutionTaskStatus) {
  return {
    todo: "待办",
    in_progress: "进行中",
    waiting: "等待",
    done: "完成",
    cancelled: "取消",
  }[status];
}

function priorityLabel(priority: ExecutionTaskPriority) {
  return { low: "低", normal: "普通", high: "高", urgent: "紧急" }[priority];
}

function quickTaskInput(title: string, projectId?: string): TaskInput {
  return {
    title,
    projectId: projectId || null,
    priority: "normal",
    timezone: browserTimezone(),
  };
}

function SectionEmpty({ children }: { children: string }) {
  return <div className="lt-exec-empty"><Inbox aria-hidden="true"/><span>{children}</span></div>;
}

function TaskRow({
  task,
  project,
  onStatus,
  onEdit,
  onSchedule,
  onReminder,
  onContextMenu,
}: {
  task: ExecutionTask;
  project?: ExecutionProject;
  onStatus: (task: ExecutionTask, status: ExecutionTaskStatus) => void;
  onEdit: (task: ExecutionTask) => void;
  onSchedule: (task: ExecutionTask) => void;
  onReminder: (task: ExecutionTask) => void;
  onContextMenu: (event: ReactMouseEvent<HTMLElement>, task: ExecutionTask) => void;
}) {
  const nextStatus: ExecutionTaskStatus = task.status === "done" ? "todo" : "done";
  return <article className={`lt-exec-row lt-exec-task priority-${task.priority}`} onContextMenu={(event) => onContextMenu(event, task)}>
    <button
      className={`lt-exec-check ${task.status === "done" ? "done" : ""}`}
      type="button"
      aria-label={task.status === "cancelled" ? `任务已取消 ${task.title}` : task.status === "done" ? `恢复任务 ${task.title}` : `完成任务 ${task.title}`}
      disabled={task.status === "cancelled"}
      onClick={() => onStatus(task, nextStatus)}
    >{task.status === "done" ? <Check aria-hidden="true"/> : null}</button>
    <button className="lt-exec-row-main" type="button" onClick={() => onEdit(task)}>
      <strong>{task.title}</strong>
      <span>
        {project?.name || "无项目"}
        {task.dueAt ? ` · ${formatDateTime(task.dueAt)}` : ""}
        {task.estimatedMinutes ? ` · ${task.estimatedMinutes} 分钟` : ""}
      </span>
    </button>
    <div className="lt-exec-row-meta">
      <span className={`lt-exec-status ${task.status}`}>{taskStatusLabel(task.status)}</span>
      <span>{priorityLabel(task.priority)}</span>
    </div>
    <div className="lt-exec-row-actions">
      <button type="button" title="安排到日历" onClick={() => onSchedule(task)}><CalendarDays/></button>
      <button type="button" title="添加提醒" onClick={() => onReminder(task)}><Bell/></button>
    </div>
  </article>;
}

function TaskEditor({
  value,
  projects,
  busy,
  close,
  save,
  remove,
}: {
  value?: ExecutionTask;
  projects: ExecutionProject[];
  busy: boolean;
  close: () => void;
  save: (input: TaskInput) => Promise<void>;
  remove: () => Promise<void>;
}) {
  const [title, setTitle] = useState(value?.title || "");
  const [description, setDescription] = useState(value?.description || "");
  const [projectId, setProjectId] = useState(value?.projectId || "");
  const [priority, setPriority] = useState<ExecutionTaskPriority>(value?.priority || "normal");
  const [estimatedMinutes, setEstimatedMinutes] = useState(value?.estimatedMinutes ? String(value.estimatedMinutes) : "");
  const [dueAt, setDueAt] = useState(rfc3339ToLocalDateTime(value?.dueAt));
  const [context, setContext] = useState(value?.context || "");
  return <div className="lt-exec-editor" role="dialog" aria-modal="true" aria-label={value ? "编辑任务" : "新建任务"}>
    <header><div><strong>{value ? "编辑任务" : "新建任务"}</strong><span>行动项</span></div><button type="button" onClick={close} aria-label="关闭"><X/></button></header>
    <div className="lt-exec-form">
      <label>标题<input autoFocus value={title} onChange={(e) => setTitle(e.target.value)} placeholder="要完成什么？"/></label>
      <label>项目<select value={projectId} onChange={(e) => setProjectId(e.target.value)}><option value="">无项目</option>{projects.filter((item) => item.status === "active").map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>
      <div className="lt-exec-form-grid">
        <label>优先级<select value={priority} onChange={(e) => setPriority(e.target.value as ExecutionTaskPriority)}><option value="low">低</option><option value="normal">普通</option><option value="high">高</option><option value="urgent">紧急</option></select></label>
        <label>预计分钟<input type="number" min="0" value={estimatedMinutes} onChange={(e) => setEstimatedMinutes(e.target.value)}/></label>
      </div>
      <label>截止时间<input type="datetime-local" value={dueAt} onChange={(e) => setDueAt(e.target.value)}/></label>
      <label>上下文<input value={context} onChange={(e) => setContext(e.target.value)} placeholder="例如：电脑、办公室"/></label>
      <label>说明<textarea rows={5} value={description} onChange={(e) => setDescription(e.target.value)} placeholder="补充完成标准或背景"/></label>
    </div>
    <footer>
      {value ? <button className="lt-exec-danger" type="button" disabled={busy} onClick={() => void remove()}><Trash2/>删除</button> : <span/>}
      <div><button type="button" disabled={busy} onClick={close}>取消</button><button className="hx-btn primary" type="button" disabled={busy || !title.trim()} onClick={() => void save({ title, description: description || null, projectId: projectId || null, priority, estimatedMinutes: estimatedMinutes ? Number(estimatedMinutes) : null, dueAt: localDateTimeToRfc3339(dueAt), timezone: browserTimezone(), context: context || null })}>{busy ? <LoaderCircle className="spin"/> : null}保存</button></div>
    </footer>
  </div>;
}

function ProjectEditor({ value, busy, close, save, remove }: { value?: ExecutionProject; busy: boolean; close: () => void; save: (input: { name: string; description?: string; status?: ExecutionProject["status"]; color?: string }) => Promise<void>; remove: () => Promise<void> }) {
  const [name, setName] = useState(value?.name || "");
  const [description, setDescription] = useState(value?.description || "");
  const [status, setStatus] = useState<ExecutionProject["status"]>(value?.status || "active");
  return <div className="lt-exec-editor" role="dialog" aria-modal="true" aria-label={value ? "编辑项目" : "新建项目"}>
    <header><div><strong>{value ? "编辑项目" : "新建项目"}</strong><span>组织任务</span></div><button type="button" onClick={close} aria-label="关闭"><X/></button></header>
    <div className="lt-exec-form"><label>名称<input autoFocus value={name} onChange={(e) => setName(e.target.value)} placeholder="例如：准备论文答辩"/></label><label>状态<select value={status} onChange={(e) => setStatus(e.target.value as ExecutionProject["status"])}><option value="active">进行中</option><option value="completed">已完成</option><option value="archived">已归档</option><option value="cancelled">已取消</option></select></label><label>说明<textarea rows={6} value={description} onChange={(e) => setDescription(e.target.value)}/></label></div>
    <footer>{value ? <button className="lt-exec-danger" type="button" disabled={busy} onClick={() => void remove()}><Trash2/>删除</button> : <span/>}<div><button type="button" onClick={close}>取消</button><button className="hx-btn primary" type="button" disabled={busy || !name.trim()} onClick={() => void save({ name, description: description || undefined, status })}>保存</button></div></footer>
  </div>;
}

function CalendarEditor({ value, sourceTask, busy, close, save, remove, onRecurrence }: { value?: CalendarEvent; sourceTask?: ExecutionTask; busy: boolean; close: () => void; save: (input: CalendarInput) => Promise<void>; remove: () => Promise<void>; onRecurrence: (event: CalendarEvent) => void }) {
  const [title, setTitle] = useState(value?.title || sourceTask?.title || "");
  const [description, setDescription] = useState(value?.description || "");
  const [allDay, setAllDay] = useState(value?.isAllDay || false);
  const [startAt, setStartAt] = useState(rfc3339ToLocalDateTime(value?.startAt));
  const [endAt, setEndAt] = useState(rfc3339ToLocalDateTime(value?.endAt));
  const [startDate, setStartDate] = useState(value?.startLocalDate || new Date().toISOString().slice(0, 10));
  const [endDate, setEndDate] = useState(value?.endLocalDate || new Date().toISOString().slice(0, 10));
  return <div className="lt-exec-editor" role="dialog" aria-modal="true" aria-label="日历事件">
    <header><div><strong>{sourceTask ? "安排任务" : value ? "编辑事件" : "新建事件"}</strong><span>时间块</span></div><button type="button" onClick={close} aria-label="关闭"><X/></button></header>
    <div className="lt-exec-form"><label>标题<input autoFocus value={title} onChange={(e) => setTitle(e.target.value)}/></label><label className="lt-exec-checkbox"><input type="checkbox" checked={allDay} onChange={(e) => setAllDay(e.target.checked)}/>全天事件</label>{allDay ? <div className="lt-exec-form-grid"><label>开始日期<input type="date" value={startDate} onChange={(e) => setStartDate(e.target.value)}/></label><label>结束日期<input type="date" value={endDate} onChange={(e) => setEndDate(e.target.value)}/></label></div> : <div className="lt-exec-form-grid"><label>开始<input type="datetime-local" value={startAt} onChange={(e) => setStartAt(e.target.value)}/></label><label>结束<input type="datetime-local" value={endAt} onChange={(e) => setEndAt(e.target.value)}/></label></div>}<label>说明<textarea rows={4} value={description} onChange={(e) => setDescription(e.target.value)}/></label></div>
    <footer>{value && !sourceTask ? <div className="lt-exec-editor-secondary"><button className="lt-exec-danger" type="button" onClick={() => void remove()}><Trash2/>删除</button><button type="button" onClick={() => onRecurrence(value)}><Repeat2/>重复规则</button></div> : <span/>}<div><button type="button" onClick={close}>取消</button><button className="hx-btn primary" type="button" disabled={busy || !title.trim() || (!allDay && (!startAt || !endAt))} onClick={() => void save({ title, description: description || null, isAllDay: allDay, startAt: allDay ? null : localDateTimeToRfc3339(startAt), endAt: allDay ? null : localDateTimeToRfc3339(endAt), startLocalDate: allDay ? startDate : null, endLocalDate: allDay ? endDate : null, timezone: browserTimezone(), sourceTaskId: sourceTask?.id || value?.sourceTaskId || null })}>保存</button></div></footer>
  </div>;
}

function WaitingEditor({ value, busy, close, save, remove }: { value?: WaitingItem; busy: boolean; close: () => void; save: (input: WaitingInput) => Promise<void>; remove: () => Promise<void> }) {
  const [title, setTitle] = useState(value?.title || "");
  const [waitingFor, setWaitingFor] = useState(value?.waitingFor || "");
  const [description, setDescription] = useState(value?.description || "");
  const [expectedAt, setExpectedAt] = useState(rfc3339ToLocalDateTime(value?.expectedAt));
  const [followUpAt, setFollowUpAt] = useState(rfc3339ToLocalDateTime(value?.followUpAt));
  return <div className="lt-exec-editor" role="dialog" aria-modal="true" aria-label="等待事项"><header><div><strong>{value ? "编辑等待事项" : "新建等待事项"}</strong><span>依赖外部结果</span></div><button type="button" onClick={close} aria-label="关闭"><X/></button></header><div className="lt-exec-form"><label>标题<input autoFocus value={title} onChange={(e) => setTitle(e.target.value)}/></label><label>等待对象<input value={waitingFor} onChange={(e) => setWaitingFor(e.target.value)} placeholder="人、团队或外部结果"/></label><div className="lt-exec-form-grid"><label>预计返回<input type="datetime-local" value={expectedAt} onChange={(e) => setExpectedAt(e.target.value)}/></label><label>跟进时间<input type="datetime-local" value={followUpAt} onChange={(e) => setFollowUpAt(e.target.value)}/></label></div><label>说明<textarea rows={5} value={description} onChange={(e) => setDescription(e.target.value)}/></label></div><footer>{value ? <button className="lt-exec-danger" type="button" onClick={() => void remove()}><Trash2/>删除</button> : <span/>}<div><button type="button" onClick={close}>取消</button><button className="hx-btn primary" type="button" disabled={busy || !title.trim() || !waitingFor.trim()} onClick={() => void save({ title, waitingFor, description: description || null, expectedAt: localDateTimeToRfc3339(expectedAt), followUpAt: localDateTimeToRfc3339(followUpAt), sourceTaskId: value?.sourceTaskId || null })}>保存</button></div></footer></div>;
}

function MemoEditor({ value, busy, close, save, remove }: { value?: Memo; busy: boolean; close: () => void; save: (input: { content: string; context?: string; tags: string[] }) => Promise<void>; remove: () => Promise<void> }) {
  const [content, setContent] = useState(value?.content || "");
  const [context, setContext] = useState(value?.context || "");
  const [tags, setTags] = useState(value?.tags.join(", ") || "");
  return <div className="lt-exec-editor" role="dialog" aria-modal="true" aria-label="Memo"><header><div><strong>{value ? "编辑 Memo" : "快速记一下"}</strong><span>临时信息，不要求行动</span></div><button type="button" onClick={close} aria-label="关闭"><X/></button></header><div className="lt-exec-form"><label>内容<textarea autoFocus rows={9} value={content} onChange={(e) => setContent(e.target.value)} placeholder="先记下来，之后再决定是否转成任务或日历"/></label><label>标签<input value={tags} onChange={(e) => setTags(e.target.value)} placeholder="工作, 生活"/></label><label>上下文<input value={context} onChange={(e) => setContext(e.target.value)} placeholder="可选"/></label></div><footer>{value ? <button className="lt-exec-danger" type="button" onClick={() => void remove()}><Trash2/>删除</button> : <span/>}<div><button type="button" onClick={close}>取消</button><button className="hx-btn primary" type="button" disabled={busy || !content.trim()} onClick={() => void save({ content, context: context || undefined, tags: tags.split(/[,，]/).map((item) => item.trim()).filter(Boolean) })}>保存</button></div></footer></div>;
}

export default function ExecutionModule() {
  const [tab, setTab] = useState<Tab>("today");
  const [data, setData] = useState<Data>(emptyData);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [editor, setEditor] = useState<Editor>(null);
  const [inspectTask, setInspectTask] = useState<ExecutionTask | null>(null);
  const [reminderSubject, setReminderSubject] = useState<ReminderSubject | null>(null);
  const [convertMemo, setConvertMemo] = useState<Memo | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>(null);
  const [taskProjectFilter, setTaskProjectFilter] = useState("");
  const [taskStatusFilter, setTaskStatusFilter] = useState("");
  const [memoQuery, setMemoQuery] = useState("");
  const [memoArchived, setMemoArchived] = useState(false);
  const [quickTask, setQuickTask] = useState("");
  const [calendarRefreshToken, setCalendarRefreshToken] = useState(0);
  const [recurrenceEvent, setRecurrenceEvent] = useState<CalendarEvent | null>(null);
  const [pendingCalendarAction, setPendingCalendarAction] = useState<PendingCalendarAction>(null);

  const load = useCallback(async () => {
    setError("");
    try {
      const now = new Date();
      const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
      const end = new Date(start.getTime() + 7 * 24 * 60 * 60 * 1000);
      const [projects, tasks, calendar, waiting, memos, reminders] = await Promise.all([
        executionApi.projects.list(),
        executionApi.tasks.list(),
        executionApi.calendar.list({ timedStart: start.toISOString(), timedEnd: end.toISOString(), localStartDate: start.toISOString().slice(0, 10), localEndDate: end.toISOString().slice(0, 10) }),
        executionApi.waiting.list(),
        executionApi.memos.list({ status: "active" }),
        executionApi.reminders.due(),
      ]);
      setData({ projects, tasks, calendar, waiting, memos, reminders });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "执行数据加载失败");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  const projectById = useMemo(() => new Map(data.projects.map((item) => [item.id, item])), [data.projects]);
  const visibleTasks = useMemo(() => data.tasks.filter((task) => (!taskProjectFilter || task.projectId === taskProjectFilter) && (!taskStatusFilter || task.status === taskStatusFilter)), [data.tasks, taskProjectFilter, taskStatusFilter]);
  const todayTasks = useMemo(() => data.tasks.filter((task) => task.status !== "done" && task.status !== "cancelled" && (isToday(task.dueAt) || isToday(task.scheduledStartAt) || isOverdue(task.dueAt))).slice(0, 8), [data.tasks]);
  const openWaiting = useMemo(() => data.waiting.filter((item) => item.status === "open"), [data.waiting]);
  const todayEvents = useMemo(() => data.calendar.filter((item) => item.status === "scheduled" && (isToday(item.startAt) || item.startLocalDate === new Date().toISOString().slice(0, 10))), [data.calendar]);
  const pinnedMemos = useMemo(() => data.memos.filter((memo) => memo.isPinned).slice(0, 5), [data.memos]);

  const run = async (action: () => Promise<unknown>, success: string) => {
    setBusy(true);
    try {
      await action();
      toast(success);
      setEditor(null);
      await load();
      setCalendarRefreshToken((value) => value + 1);
      if (tab === "memos" && (memoArchived || memoQuery.trim())) {
        const memos = await executionApi.memos.list({
          status: memoArchived ? "archived" : "active",
          q: memoQuery.trim() || undefined,
        });
        setData((current) => ({ ...current, memos }));
      }
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "操作失败", "error");
    } finally {
      setBusy(false);
    }
  };

  const guardCalendarAction = async (
    title: string,
    timing: CalendarTimingInput,
    excludeEventId: string | undefined,
    action: () => Promise<unknown>,
    success: string,
  ) => {
    if (timing.isAllDay) {
      await run(action, success);
      return;
    }
    setBusy(true);
    try {
      const conflicts = await executionApi.calendar.conflicts(timing, excludeEventId);
      if (conflicts.length) {
        setPendingCalendarAction({ title, conflicts, action, success });
        return;
      }
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "冲突检查失败", "error");
      return;
    } finally {
      setBusy(false);
    }
    await run(action, success);
  };

  const createQuickTask = async () => {
    const title = quickTask.trim();
    if (!title) return;
    setQuickTask("");
    await run(() => executionApi.tasks.create(quickTaskInput(title, taskProjectFilter || undefined)), "任务已创建");
  };

  const setTaskStatus = (task: ExecutionTask, status: ExecutionTaskStatus) => void run(() => executionApi.tasks.setStatus(task.id, status), status === "done" ? "任务已完成" : "任务状态已更新");

  const menuPosition = (event: ReactMouseEvent<HTMLElement>) => {
    event.preventDefault();
    const rect = event.currentTarget.getBoundingClientRect();
    return {
      x: event.clientX || rect.left + 24,
      y: event.clientY || rect.top + 24,
    };
  };

  const openTaskMenu = (event: ReactMouseEvent<HTMLElement>, task: ExecutionTask) => {
    const position = menuPosition(event);
    const items: ExecutionMenuItem[] = [
      { id: "details", label: "查看任务详情", action: () => setInspectTask(task) },
      { id: "edit", label: "编辑任务", icon: Pencil, action: () => setEditor({ kind: "task", value: task }) },
      { id: "schedule", label: "安排到日历", icon: CalendarDays, action: () => setEditor({ kind: "calendar", sourceTask: task }) },
      { id: "reminder", label: "管理提醒", icon: Bell, action: () => setReminderSubject({ subjectType: "task", subjectId: task.id, title: task.title }) },
      { id: "complete", label: task.status === "done" ? "恢复为待办" : "标记完成", icon: Check, disabled: task.status === "cancelled", action: () => setTaskStatus(task, task.status === "done" ? "todo" : "done") },
      { id: "delete", label: "删除任务", icon: Trash2, danger: true, action: () => { if (window.confirm(`确定删除“${task.title}”吗？`)) void run(() => executionApi.tasks.remove(task.id), "任务已删除"); } },
    ];
    setContextMenu({ ...position, items });
  };

  const openMemoMenu = (event: ReactMouseEvent<HTMLElement>, memo: Memo) => {
    const position = menuPosition(event);
    const items: ExecutionMenuItem[] = [
      { id: "edit", label: "编辑 Memo", icon: Pencil, action: () => setEditor({ kind: "memo", value: memo }) },
      { id: "pin", label: memo.isPinned ? "取消置顶" : "置顶", icon: Pin, action: () => void run(() => executionApi.memos.pin(memo.id, !memo.isPinned), memo.isPinned ? "已取消置顶" : "已置顶") },
      { id: "reminder", label: "管理提醒", icon: Bell, action: () => setReminderSubject({ subjectType: "memo", subjectId: memo.id, title: memo.plainText.slice(0, 30) }) },
      { id: "convert", label: "转换为…", action: () => setConvertMemo(memo), disabled: memo.status !== "active" },
      { id: "archive", label: memo.status === "active" ? "归档" : "恢复", icon: memo.status === "active" ? Archive : RotateCcw, action: () => void run(() => memo.status === "active" ? executionApi.memos.archive(memo.id) : executionApi.memos.restore(memo.id), memo.status === "active" ? "Memo 已归档" : "Memo 已恢复") },
      { id: "delete", label: "删除 Memo", icon: Trash2, danger: true, action: () => { if (window.confirm("确定删除这条 Memo 吗？")) void run(() => executionApi.memos.remove(memo.id), "Memo 已删除"); } },
    ];
    setContextMenu({ ...position, items });
  };

  const renderToday = () => <div className="lt-exec-today">
    <div className="lt-exec-summary">
      <button type="button" onClick={() => setTab("tasks")}><span>今日 / 逾期任务</span><strong>{todayTasks.length}</strong></button>
      <button type="button" onClick={() => setTab("calendar")}><span>今日安排</span><strong>{todayEvents.length}</strong></button>
      <button type="button" onClick={() => setTab("waiting")}><span>正在等待</span><strong>{openWaiting.length}</strong></button>
      <button type="button" onClick={() => setTab("memos")}><span>置顶 Memo</span><strong>{pinnedMemos.length}</strong></button>
    </div>
    <div className="lt-exec-today-grid">
      <section><header><div><strong>现在要做</strong><span>按优先级和截止时间排序</span></div><button type="button" onClick={() => setEditor({ kind: "task" })}><Plus/>任务</button></header>{todayTasks.length ? todayTasks.map((task) => <TaskRow key={task.id} task={task} project={task.projectId ? projectById.get(task.projectId) : undefined} onStatus={setTaskStatus} onEdit={(value) => setInspectTask(value)} onSchedule={(sourceTask) => setEditor({ kind: "calendar", sourceTask })} onReminder={(value) => setReminderSubject({ subjectType: "task", subjectId: value.id, title: value.title })}onContextMenu={openTaskMenu}/>) : <SectionEmpty>今天没有需要推进的任务</SectionEmpty>}</section>
      <section><header><div><strong>时间与等待</strong><span>未来七天</span></div></header><div className="lt-exec-stream">{todayEvents.map((event) => <button key={event.id} type="button" onClick={() => setEditor({ kind: "calendar", value: event })}><CalendarDays/><span><strong>{event.title}</strong><small>{event.isAllDay ? "全天" : `${formatDateTime(event.startAt)} – ${formatDateTime(event.endAt)}`}</small></span></button>)}{openWaiting.slice(0, 5).map((item) => <button key={item.id} type="button" onClick={() => setEditor({ kind: "waiting", value: item })}><Users/><span><strong>{item.title}</strong><small>等待 {item.waitingFor}{item.followUpAt ? ` · ${formatDateTime(item.followUpAt)} 跟进` : ""}</small></span></button>)}{!todayEvents.length && !openWaiting.length ? <SectionEmpty>暂无时间块或等待事项</SectionEmpty> : null}</div></section>
    </div>
    {data.reminders.length ? <section className="lt-exec-due"><header><Bell/><strong>已到期提醒</strong><span>{data.reminders.length}</span></header><div>{data.reminders.slice(0, 6).map((reminder) => <button key={reminder.id} type="button" onClick={() => void run(() => executionApi.reminders.dismiss(reminder.id), "提醒已处理")}><Clock3/><span>{formatDateTime(reminder.snoozedUntil || reminder.triggerAt)}</span><small>点击处理</small></button>)}</div></section> : null}
  </div>;

  const renderTasks = () => <div className="lt-exec-workspace"><div className="lt-exec-toolbar"><div><select aria-label="项目筛选" value={taskProjectFilter} onChange={(e) => setTaskProjectFilter(e.target.value)}><option value="">全部项目</option>{data.projects.filter((item) => item.status === "active").map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select><select aria-label="状态筛选" value={taskStatusFilter} onChange={(e) => setTaskStatusFilter(e.target.value)}><option value="">全部状态</option><option value="todo">待办</option><option value="in_progress">进行中</option><option value="waiting">等待</option><option value="done">完成</option><option value="cancelled">取消</option></select></div><button className="hx-btn primary" type="button" onClick={() => setEditor({ kind: "task" })}><Plus/>新建任务</button></div><div className="lt-exec-quick"><Plus/><input value={quickTask} onChange={(e) => setQuickTask(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter") void createQuickTask(); }} placeholder="快速添加任务，按 Enter 保存"/><span>{taskProjectFilter ? projectById.get(taskProjectFilter)?.name : "无项目"}</span></div><div className="lt-exec-list">{visibleTasks.length ? visibleTasks.map((task) => <TaskRow key={task.id} task={task} project={task.projectId ? projectById.get(task.projectId) : undefined} onStatus={setTaskStatus} onEdit={(value) => setInspectTask(value)} onSchedule={(sourceTask) => setEditor({ kind: "calendar", sourceTask })} onReminder={(value) => setReminderSubject({ subjectType: "task", subjectId: value.id, title: value.title })}onContextMenu={openTaskMenu}/>) : <SectionEmpty>当前筛选下没有任务</SectionEmpty>}</div></div>;

  const renderProjects = () => <div className="lt-exec-workspace"><div className="lt-exec-toolbar"><div><strong>{data.projects.filter((item) => item.status === "active").length}</strong><span> 个进行中项目</span></div><button className="hx-btn primary" type="button" onClick={() => setEditor({ kind: "project" })}><Plus/>新建项目</button></div><div className="lt-exec-project-grid">{data.projects.length ? data.projects.map((project) => { const tasks = data.tasks.filter((task) => task.projectId === project.id && task.status !== "cancelled"); const done = tasks.filter((task) => task.status === "done").length; const ratio = tasks.length ? Math.round(done / tasks.length * 100) : 0; return <button key={project.id} type="button" onClick={() => setEditor({ kind: "project", value: project })}><div><span className={`lt-exec-project-dot ${project.status}`}/><strong>{project.name}</strong><small>{project.status}</small></div><p>{project.description || "暂无说明"}</p><div className="lt-exec-progress"><i style={{ width: `${ratio}%` }}/></div><footer><span>{done}/{tasks.length} 完成</span><ChevronRight/></footer></button>; }) : <SectionEmpty>还没有项目</SectionEmpty>}</div></div>;

  const renderCalendar = () => <CalendarWorkspace
    refreshToken={calendarRefreshToken}
    onCreate={() => setEditor({ kind: "calendar" })}
    onEdit={(value) => setEditor({ kind: "calendar", value })}
    onMove={(value, timing) => guardCalendarAction(value.title, timing, value.id, () => executionApi.calendar.move(value.id, timing), "事件时间已调整")}
    onRecurrence={(value) => setRecurrenceEvent(value)}
    onReminder={(subject) => setReminderSubject(subject)}
  />;

  const renderWaiting = () => <div className="lt-exec-workspace"><div className="lt-exec-toolbar"><div><strong>{openWaiting.length}</strong><span> 个事项正在等待外部结果</span></div><button className="hx-btn primary" type="button" onClick={() => setEditor({ kind: "waiting" })}><Plus/>新建等待</button></div><div className="lt-exec-list">{data.waiting.length ? data.waiting.map((item) => <article key={item.id} className="lt-exec-row"><Users/><button className="lt-exec-row-main" type="button" onClick={() => setEditor({ kind: "waiting", value: item })}><strong>{item.title}</strong><span>等待 {item.waitingFor}{item.expectedAt ? ` · 预计 ${formatDateTime(item.expectedAt)}` : ""}{item.followUpAt ? ` · ${formatDateTime(item.followUpAt)} 跟进` : ""}</span></button><span className={`lt-exec-status ${item.status}`}>{item.status}</span><div className="lt-exec-row-actions">{item.status === "open" ? <><button type="button" title="添加提醒" onClick={() => setReminderSubject({ subjectType: "waiting_item", subjectId: item.id, title: item.title })}><Bell/></button><button type="button" title="转为任务" onClick={() => void run(() => executionApi.waiting.convertToTask(item.id, waitingToTaskInput(item)), "已转为任务")}><ChevronRight/></button><button type="button" title="标记已解决" onClick={() => void run(() => executionApi.waiting.resolve(item.id), "等待事项已解决")}><Check/></button></> : null}</div></article>) : <SectionEmpty>没有等待事项</SectionEmpty>}</div></div>;

  const refreshMemos = async (archived = memoArchived, q = memoQuery) => {
    setBusy(true);
    try {
      const memos = await executionApi.memos.list({ status: archived ? "archived" : "active", q: q || undefined });
      setData((current) => ({ ...current, memos }));
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "Memo 加载失败", "error");
    } finally { setBusy(false); }
  };

  const renderMemos = () => <div className="lt-exec-workspace"><div className="lt-exec-toolbar"><div className="lt-exec-search"><Search/><input value={memoQuery} onChange={(e) => setMemoQuery(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter") void refreshMemos(); }} placeholder="搜索 Memo、上下文或标签"/><button type="button" onClick={() => void refreshMemos()}>搜索</button></div><div><button type="button" className={memoArchived ? "active" : ""} onClick={() => { const next = !memoArchived; setMemoArchived(next); void refreshMemos(next); }}>{memoArchived ? <RotateCcw/> : <Archive/>}{memoArchived ? "返回当前" : "归档"}</button><button className="hx-btn primary" type="button" onClick={() => setEditor({ kind: "memo" })}><Plus/>快速记</button></div></div><div className="lt-exec-memo-grid">{data.memos.length ? data.memos.map((memo) => <article key={memo.id} className={memo.isPinned ? "pinned" : ""} onContextMenu={(event) => openMemoMenu(event, memo)}><header><button type="button" title={memo.isPinned ? "取消置顶" : "置顶"} onClick={() => void run(() => executionApi.memos.pin(memo.id, !memo.isPinned), memo.isPinned ? "已取消置顶" : "已置顶")}><Pin className={memo.isPinned ? "filled" : ""}/></button><span>{formatDateTime(memo.updatedAt)}</span></header><button className="lt-exec-memo-content" type="button" onClick={() => setEditor({ kind: "memo", value: memo })}>{memo.content}</button>{memo.tags.length ? <div className="lt-exec-tags">{memo.tags.map((tag) => <span key={tag}>#{tag}</span>)}</div> : null}<footer>{memo.status === "active" ? <button type="button" onClick={() => void run(() => executionApi.memos.archive(memo.id), "Memo 已归档")}><Archive/>归档</button> : <button type="button" onClick={() => void run(() => executionApi.memos.restore(memo.id), "Memo 已恢复")}><RotateCcw/>恢复</button>}<button type="button" onClick={() => setReminderSubject({ subjectType: "memo", subjectId: memo.id, title: memo.plainText.slice(0, 30) })}><Bell/>提醒</button>{memo.status === "active" ? <button type="button" onClick={() => setConvertMemo(memo)}>转换<ChevronRight/></button> : null}</footer></article>) : <SectionEmpty>{memoArchived ? "没有已归档 Memo" : "还没有 Memo，先快速记一条"}</SectionEmpty>}</div></div>;

  const activeTab = tabs.find(([id]) => id === tab)!;
  const renderContent = () => ({ today: renderToday, tasks: renderTasks, projects: renderProjects, calendar: renderCalendar, waiting: renderWaiting, memos: renderMemos }[tab])();

  return <div className="lt-exec-root">
    <div className="lt-exec-head"><div><h1>执行中心</h1><span>把计划变成下一步行动</span></div><button type="button" className="lt-exec-refresh" onClick={() => void load()}><RefreshCw className={loading ? "spin" : ""}/>刷新</button></div>
    <nav className="lt-exec-tabs" aria-label="执行中心"><div>{tabs.map(([id, label, Icon]) => <button key={id} type="button" className={tab === id ? "active" : ""} aria-current={tab === id ? "page" : undefined} onClick={() => setTab(id)}><Icon/>{label}</button>)}</div><span>{activeTab[1]}</span></nav>
    {loading ? <div className="lt-exec-loading"><LoaderCircle className="spin"/><span>正在读取本地执行数据…</span></div> : error ? <div className="lt-exec-error"><strong>执行数据暂时无法读取</strong><span>{error}</span><button className="hx-btn primary" type="button" onClick={() => { setLoading(true); void load(); }}>重试</button></div> : renderContent()}
    {editor ? <div className="lt-exec-editor-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setEditor(null); }}>
      {editor.kind === "task" ? <TaskEditor value={editor.value} projects={data.projects} busy={busy} close={() => setEditor(null)} save={(input) => run(() => editor.value ? executionApi.tasks.update(editor.value.id, preserveTaskUpdateFields(editor.value, input)) : executionApi.tasks.create(input), editor.value ? "任务已更新" : "任务已创建")} remove={() => run(() => executionApi.tasks.remove(editor.value!.id), "任务已删除")}/>: null}
      {editor.kind === "project" ? <ProjectEditor value={editor.value} busy={busy} close={() => setEditor(null)} save={(input) => run(() => editor.value ? executionApi.projects.update(editor.value.id, input) : executionApi.projects.create(input), editor.value ? "项目已更新" : "项目已创建")} remove={() => run(() => executionApi.projects.remove(editor.value!.id), "项目已删除")}/>: null}
      {editor.kind === "calendar" ? <CalendarEditor value={editor.value} sourceTask={editor.sourceTask} busy={busy} close={() => setEditor(null)} save={(input) => guardCalendarAction(input.title, input, editor.value?.id, () => editor.sourceTask ? executionApi.tasks.schedule(editor.sourceTask.id, input) : editor.value ? executionApi.calendar.update(editor.value.id, input) : executionApi.calendar.create(input), editor.sourceTask ? "任务已安排到日历" : editor.value ? "事件已更新" : "事件已创建")} remove={() => run(() => executionApi.calendar.remove(editor.value!.id), "事件已删除")} onRecurrence={(value) => { setEditor(null); setRecurrenceEvent(value); }}/>: null}
      {editor.kind === "waiting" ? <WaitingEditor value={editor.value} busy={busy} close={() => setEditor(null)} save={(input) => run(() => editor.value ? executionApi.waiting.update(editor.value.id, input) : executionApi.waiting.create(input), editor.value ? "等待事项已更新" : "等待事项已创建")} remove={() => run(() => executionApi.waiting.remove(editor.value!.id), "等待事项已删除")}/>: null}
      {editor.kind === "memo" ? <MemoEditor value={editor.value} busy={busy} close={() => setEditor(null)} save={(input) => run(() => editor.value ? executionApi.memos.update(editor.value.id, input) : executionApi.memos.create(input), editor.value ? "Memo 已更新" : "Memo 已保存")} remove={() => run(() => executionApi.memos.remove(editor.value!.id), "Memo 已删除")}/>: null}
    </div> : null}
    {inspectTask ? <div className="lt-exec-editor-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setInspectTask(null); }}>
      <TaskAdvancedPanel
        task={inspectTask}
        projects={data.projects}
        allTasks={data.tasks}
        onClose={() => setInspectTask(null)}
        onEdit={() => { const value = inspectTask; setInspectTask(null); setEditor({ kind: "task", value }); }}
        onSchedule={() => { const sourceTask = inspectTask; setInspectTask(null); setEditor({ kind: "calendar", sourceTask }); }}
        onReminder={() => { const value = inspectTask; setInspectTask(null); setReminderSubject({ subjectType: "task", subjectId: value.id, title: value.title }); }}
        onChanged={load}
      />
    </div> : null}
    {reminderSubject ? <div className="lt-exec-editor-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setReminderSubject(null); }}>
      <SubjectReminderPanel {...reminderSubject} onClose={() => setReminderSubject(null)} onChanged={load}/>
    </div> : null}
    {convertMemo ? <div className="lt-exec-editor-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setConvertMemo(null); }}>
      <MemoConvertPanel memo={convertMemo} projects={data.projects} onClose={() => setConvertMemo(null)} onConverted={async () => { await load(); if (tab === "memos") await refreshMemos(); }}/>
    </div> : null}
    {recurrenceEvent ? <div className="lt-exec-editor-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setRecurrenceEvent(null); }}>
      <CalendarRecurrencePanel event={recurrenceEvent} onClose={() => setRecurrenceEvent(null)} onChanged={async () => { await load(); setCalendarRefreshToken((value) => value + 1); }}/>
    </div> : null}
    {pendingCalendarAction ? <div className="lt-exec-editor-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) setPendingCalendarAction(null); }}>
      <CalendarConflictDialog
        title={pendingCalendarAction.title}
        conflicts={pendingCalendarAction.conflicts}
        busy={busy}
        onCancel={() => setPendingCalendarAction(null)}
        onConfirm={() => { const pending = pendingCalendarAction; setPendingCalendarAction(null); void run(pending.action, pending.success); }}
      />
    </div> : null}
    {contextMenu ? <ExecutionContextMenu {...contextMenu} onClose={() => setContextMenu(null)}/> : null}
  </div>;
}
