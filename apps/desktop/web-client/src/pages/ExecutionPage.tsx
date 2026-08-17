import { useMemo, useState, type FormEvent } from "react";
import {
  atomicMutate, createExecutionCompletionResult, createExecutionProject, createExecutionTask,
  executionTaskDate, isOpenExecutionTask, localDate, taskMatchesToday,
  type AtomicMutation, type JsonEntity,
} from "../core";
import { Empty, Notice, PageStack, Panel, Toolbar, entities, text, type CloudPageProps } from "../ui";

type TaskView = "today" | "todo" | "completed";

const VIEW_LABELS: Array<[TaskView, string]> = [
  ["today", "今天"],
  ["todo", "待办"],
  ["completed", "已完成"],
];

function sorted(items: JsonEntity[]): JsonEntity[] {
  return [...items].sort((left, right) => {
    const leftDate = executionTaskDate(left) ?? "9999-12-31";
    const rightDate = executionTaskDate(right) ?? "9999-12-31";
    if (leftDate !== rightDate) return leftDate.localeCompare(rightDate);
    return right.meta.updatedAt.localeCompare(left.meta.updatedAt);
  });
}

function dueAt(date: string): string | null {
  if (!date) return null;
  const value = new Date(`${date}T23:59:00`);
  return Number.isNaN(value.getTime()) ? null : value.toISOString();
}

function displayDate(task: JsonEntity): string {
  const date = executionTaskDate(task);
  if (!date) return "";
  if (date === localDate()) return "今天";
  const parsed = new Date(`${date}T00:00:00`);
  if (Number.isNaN(parsed.getTime())) return date;
  return parsed.toLocaleDateString("zh-CN", { month: "numeric", day: "numeric" });
}

export function ExecutionPage({ session, state, run, online }: CloudPageProps) {
  const [view, setView] = useState<TaskView>("today");
  const [title, setTitle] = useState("");
  const [taskDate, setTaskDate] = useState(localDate());
  const [taskProject, setTaskProject] = useState("");
  const [projectFilter, setProjectFilter] = useState("all");
  const [showProjectForm, setShowProjectForm] = useState(false);
  const [projectName, setProjectName] = useState("");
  const [message, setMessage] = useState("");

  const projects = useMemo(
    () => entities(state, "execution.project")
      .filter((item) => item.status !== "archived" && item.status !== "cancelled")
      .sort((left, right) => text(left, "name").localeCompare(text(right, "name"), "zh-CN")),
    [state],
  );
  const tasks = useMemo(
    () => sorted(entities(state, "execution.task").filter((item) => item.status !== "cancelled")),
    [state],
  );
  const occurrences = useMemo(() => entities(state, "execution.task_occurrence"), [state]);
  const completionResults = useMemo(() => entities(state, "execution.completion_result"), [state]);
  const dependencies = useMemo(() => entities(state, "execution.task_dependency"), [state]);
  const reminders = useMemo(() => entities(state, "execution.reminder"), [state]);
  const links = useMemo(() => entities(state, "execution.entity_link"), [state]);
  const recurrenceRules = useMemo(() => entities(state, "execution.recurrence_rule"), [state]);

  const projectMap = useMemo(() => new Map(projects.map((project) => [project.meta.id, project])), [projects]);
  const taskMap = useMemo(() => new Map(tasks.map((task) => [task.meta.id, task])), [tasks]);
  const today = localDate();
  const openTasks = tasks.filter(isOpenExecutionTask);
  const completedTasks = tasks.filter((task) => task.status === "done");
  const todayTasks = openTasks.filter((task) => !text(task, "recurrenceRuleId") && taskMatchesToday(task));
  const todayOccurrences = occurrences.filter((occurrence) => text(occurrence, "occurrenceKey") === today && occurrence.status !== "skipped");

  const filteredTasks = (view === "today" ? todayTasks : view === "completed" ? completedTasks : openTasks)
    .filter((task) => projectFilter === "all" || text(task, "projectId") === projectFilter);
  const filteredOccurrences = todayOccurrences.filter((occurrence) => {
    if (projectFilter === "all") return true;
    const task = taskMap.get(text(occurrence, "taskId"));
    return task ? text(task, "projectId") === projectFilter : false;
  });
  const openCount = openTasks.length;
  const todayPendingCount = todayTasks.length + todayOccurrences.filter((item) => item.status !== "completed").length;

  async function createTask(event: FormEvent) {
    event.preventDefault();
    const value = title.trim();
    if (!value) return;
    const task = createExecutionTask(session.user.id, session.session.deviceId, {
      title: value,
      projectId: taskProject || null,
      dueAt: dueAt(taskDate),
      context: taskProject || taskDate ? null : "inbox",
    });
    await run((store) => store.upsert("execution.task", task));
    setTitle("");
    setMessage("任务已添加");
  }

  async function createProject(event: FormEvent) {
    event.preventDefault();
    const value = projectName.trim();
    if (!value) return;
    const project = createExecutionProject(session.user.id, session.session.deviceId, { name: value });
    await run((store) => store.upsert("execution.project", project));
    setProjectName("");
    setShowProjectForm(false);
    setMessage("计划已创建，可在新增任务时直接选择");
  }

  async function setTaskDone(task: JsonEntity, done: boolean) {
    const updated: JsonEntity = {
      ...task,
      status: done ? "done" : "todo",
      completedAt: done ? new Date().toISOString() : null,
      cancelledAt: null,
    };
    if (!done || text(task, "recurrenceRuleId")) {
      await run((store) => store.upsert("execution.task", updated));
      setMessage(done ? "任务已完成" : "任务已恢复");
      return;
    }

    const existingResult = completionResults.find((item) => text(item, "taskId") === task.meta.id);
    if (existingResult) {
      await run((store) => store.upsert("execution.task", updated));
    } else {
      const result = createExecutionCompletionResult(session.user.id, session.session.deviceId, task.meta.id, null);
      await run((store) => atomicMutate(store, [
        { operation: "upsert", entityType: "execution.task", entity: updated },
        { operation: "upsert", entityType: "execution.completion_result", entity: result },
      ]));
    }
    setMessage("任务已完成");
  }

  async function completeOccurrence(occurrence: JsonEntity) {
    await run((store) => store.upsert("execution.task_occurrence", {
      ...occurrence,
      status: "completed",
      completedAt: new Date().toISOString(),
      skippedAt: null,
    }));
    setMessage("本次任务已完成");
  }

  async function reopenOccurrence(occurrence: JsonEntity) {
    await run((store) => store.upsert("execution.task_occurrence", {
      ...occurrence,
      status: "pending",
      completedAt: null,
      skippedAt: null,
    }));
    setMessage("本次任务已恢复");
  }

  async function deleteTask(task: JsonEntity) {
    const taskTitle = text(task, "title") || "这条任务";
    if (!window.confirm(`确定删除“${taskTitle}”吗？删除后无法撤销。`)) return;

    const mutations: AtomicMutation[] = [
      { operation: "delete", entityType: "execution.task", entityId: task.meta.id },
    ];
    const ruleId = text(task, "recurrenceRuleId");
    if (ruleId && recurrenceRules.some((item) => item.meta.id === ruleId)) {
      mutations.push({ operation: "delete", entityType: "execution.recurrence_rule", entityId: ruleId });
    }
    for (const occurrence of occurrences.filter((item) => text(item, "taskId") === task.meta.id)) {
      mutations.push({ operation: "delete", entityType: "execution.task_occurrence", entityId: occurrence.meta.id });
    }
    for (const result of completionResults.filter((item) => text(item, "taskId") === task.meta.id)) {
      mutations.push({ operation: "delete", entityType: "execution.completion_result", entityId: result.meta.id });
    }
    for (const dependency of dependencies.filter((item) => text(item, "taskId") === task.meta.id || text(item, "dependsOnTaskId") === task.meta.id)) {
      mutations.push({ operation: "delete", entityType: "execution.task_dependency", entityId: dependency.meta.id });
    }
    for (const reminder of reminders.filter((item) => text(item, "subjectType") === "task" && text(item, "subjectId") === task.meta.id)) {
      mutations.push({ operation: "delete", entityType: "execution.reminder", entityId: reminder.meta.id });
    }
    for (const link of links.filter((item) =>
      (text(item, "sourceType") === "task" && text(item, "sourceId") === task.meta.id)
      || (text(item, "targetType") === "task" && text(item, "targetId") === task.meta.id))) {
      mutations.push({ operation: "delete", entityType: "execution.entity_link", entityId: link.meta.id });
    }

    await run((store) => mutations.length === 1
      ? store.delete("execution.task", task.meta.id)
      : atomicMutate(store, mutations));
    setMessage("任务已删除");
  }

  const panelTitle = view === "today"
    ? `今天 · ${todayPendingCount} 项待完成`
    : view === "todo"
      ? `待办 · ${openCount} 项`
      : `已完成 · ${completedTasks.length} 项`;

  return <PageStack>
    <Panel eyebrow="QUICK ADD" title="添加任务">
      <form className="hx-form" onSubmit={(event) => void createTask(event)}>
        <label>
          要做什么？
          <input
            required
            autoFocus
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder="例如：整理本周工作总结"
          />
        </label>
        <div className="hx-form-grid">
          <label>
            日期
            <input type="date" value={taskDate} onChange={(event) => setTaskDate(event.target.value)} />
          </label>
          <label>
            计划（可选）
            <select value={taskProject} onChange={(event) => setTaskProject(event.target.value)}>
              <option value="">无计划</option>
              {projects.map((project) => <option key={project.meta.id} value={project.meta.id}>{text(project, "name")}</option>)}
            </select>
          </label>
        </div>
        <Toolbar>
          <button className="hx-btn primary" disabled={!online}>添加任务</button>
          <button type="button" className="hx-btn ghost" onClick={() => setTaskDate(taskDate ? "" : localDate())}>
            {taskDate ? "清除日期" : "设为今天"}
          </button>
        </Toolbar>
      </form>
    </Panel>

    {message && <Notice kind="success">{message}</Notice>}

    <Toolbar>
      {VIEW_LABELS.map(([key, label]) => <button
        key={key}
        className={`hx-btn ${view === key ? "primary" : "secondary"}`}
        onClick={() => setView(key)}
      >{label}</button>)}
      <button type="button" className="hx-btn ghost" onClick={() => setShowProjectForm((value) => !value)}>
        {showProjectForm ? "收起" : "+ 新建计划"}
      </button>
    </Toolbar>

    {showProjectForm && <Panel eyebrow="PLAN" title="新建一个轻量计划">
      <form className="hx-form" onSubmit={(event) => void createProject(event)}>
        <label>
          计划名称
          <input required value={projectName} onChange={(event) => setProjectName(event.target.value)} placeholder="例如：毕业论文" />
        </label>
        <Toolbar>
          <button className="hx-btn primary" disabled={!online}>创建计划</button>
          <button type="button" className="hx-btn ghost" onClick={() => setShowProjectForm(false)}>取消</button>
        </Toolbar>
      </form>
    </Panel>}

    <Panel
      eyebrow="TASKS"
      title={panelTitle}
      actions={projects.length ? <select aria-label="按计划筛选" value={projectFilter} onChange={(event) => setProjectFilter(event.target.value)}>
        <option value="all">全部计划</option>
        {projects.map((project) => <option key={project.meta.id} value={project.meta.id}>{text(project, "name")}</option>)}
      </select> : undefined}
    >
      <div className="hx-list">
        {view === "today" && filteredOccurrences.map((occurrence) => {
          const task = taskMap.get(text(occurrence, "taskId"));
          if (!task) return null;
          const done = occurrence.status === "completed";
          const project = projectMap.get(text(task, "projectId"));
          return <article className="hx-row" key={occurrence.meta.id}>
            <span className="hx-row-icon">{done ? "✓" : "↻"}</span>
            <div className="hx-row-main">
              <strong>{text(task, "title")}</strong>
              <small>{[project ? text(project, "name") : "", "重复任务"].filter(Boolean).join(" · ")}</small>
            </div>
            <div className="hx-row-actions">
              <button className={`hx-btn ${done ? "ghost" : "primary"}`} disabled={!online} onClick={() => void (done ? reopenOccurrence(occurrence) : completeOccurrence(occurrence))}>
                {done ? "恢复" : "完成"}
              </button>
              <button className="hx-btn ghost" disabled={!online} onClick={() => void deleteTask(task)}>删除任务</button>
            </div>
          </article>;
        })}

        {filteredTasks.map((task) => {
          const done = task.status === "done";
          const recurring = Boolean(text(task, "recurrenceRuleId"));
          const project = projectMap.get(text(task, "projectId"));
          const date = displayDate(task);
          const metadata = [project ? text(project, "name") : "", recurring ? "重复任务" : "", date].filter(Boolean).join(" · ");
          return <article className="hx-row" key={task.meta.id}>
            <span className="hx-row-icon">{done ? "✓" : recurring ? "↻" : "□"}</span>
            <div className="hx-row-main">
              <strong>{text(task, "title")}</strong>
              {metadata && <small>{metadata}</small>}
            </div>
            <div className="hx-row-actions">
              {!recurring && <button className={`hx-btn ${done ? "ghost" : "primary"}`} disabled={!online} onClick={() => void setTaskDone(task, !done)}>
                {done ? "恢复" : "完成"}
              </button>}
              <button className="hx-btn ghost" disabled={!online} onClick={() => void deleteTask(task)}>删除</button>
            </div>
          </article>;
        })}

        {!filteredTasks.length && (view !== "today" || !filteredOccurrences.length) && <Empty
          title={view === "today" ? "今天没有待办" : view === "todo" ? "待办已清空" : "还没有完成记录"}
          description={view === "completed" ? "完成任务后会自动出现在这里。" : "在上方输入任务，回车或点击“添加任务”即可。"}
        />}
      </div>
    </Panel>
  </PageStack>;
}
