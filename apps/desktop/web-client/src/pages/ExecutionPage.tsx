import { useMemo, useState, type FormEvent } from "react";
import {
  createExecutionCompletionResult, createExecutionMemo, createExecutionProject, createExecutionTask,
  isOpenExecutionTask, localDate, taskIsInbox, taskMatchesToday, taskPriorityLabel,
  type JsonEntity,
} from "../core";
import { Empty, Metric, MetricGrid, Notice, PageStack, Panel, Toolbar, entities, text, type CloudPageProps } from "../ui";

type ExecutionView = "today" | "inbox" | "projects" | "memos" | "completed";
type CaptureType = "task" | "memo";

const VIEW_LABELS: Array<[ExecutionView, string]> = [
  ["today", "今天"], ["inbox", "收件箱"], ["projects", "计划"], ["memos", "备忘"], ["completed", "已完成"],
];

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

  const projects = useMemo(
    () => sorted(entities(state, "execution.project")).filter((item) => item.status !== "archived" && item.status !== "cancelled"),
    [state],
  );
  const tasks = useMemo(() => sorted(entities(state, "execution.task")), [state]);
  const memos = useMemo(() => sorted(entities(state, "execution.memo")).filter((item) => item.status !== "archived"), [state]);
  const completionResults = entities(state, "execution.completion_result");
  const openTasks = tasks.filter(isOpenExecutionTask);
  const todayTasks = tasks.filter((item) => taskMatchesToday(item));
  const todayOpen = todayTasks.filter(isOpenExecutionTask);
  const todayDone = todayTasks.filter((item) => item.status === "done");
  const inbox = tasks.filter(taskIsInbox);
  const completed = tasks.filter((item) => item.status === "done");
  const projectMap = new Map(projects.map((item) => [item.meta.id, item]));
  const todayCompletion = todayTasks.length ? Math.round((todayDone.length / todayTasks.length) * 100) : 0;

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
      let next = await store.upsert("execution.task", updated);
      if (status === "done") {
        const existing = completionResults.find((item) => text(item, "taskId") === task.meta.id);
        if (!existing) {
          const result = createExecutionCompletionResult(session.user.id, session.session.deviceId, task.meta.id, typeof task.actualMinutes === "number" ? task.actualMinutes : null);
          next = await store.upsert("execution.completion_result", result);
        }
      }
      return next;
    });
  }

  async function arrangeToday(task: JsonEntity) {
    const updated: JsonEntity = { ...task, dueAt: dueAt(localDate()), context: task.context === "inbox" ? null : task.context };
    await run((store) => store.upsert("execution.task", updated));
    setMessage("已安排到今天");
  }

  async function pinMemo(memo: JsonEntity) {
    await run((store) => store.upsert("execution.memo", { ...memo, isPinned: memo.isPinned !== true }));
  }

  async function archiveMemo(memo: JsonEntity) {
    await run((store) => store.upsert("execution.memo", { ...memo, status: "archived", archivedAt: new Date().toISOString() }));
  }

  const visibleTasks = view === "today" ? todayOpen : view === "inbox" ? inbox : view === "completed" ? completed : openTasks;

  return <PageStack>
    <Toolbar>
      {VIEW_LABELS.map(([key, label]) => <button key={key} className={`hx-btn ${view === key ? "primary" : "secondary"}`} onClick={() => setView(key)}>{label}</button>)}
    </Toolbar>

    <MetricGrid>
      <Metric label="今日待办" value={`${todayDone.length} / ${todayTasks.length}`} detail={todayOpen.length ? `还有 ${todayOpen.length} 项` : todayTasks.length ? "今天已清空" : "今天还未安排任务"} positive={todayTasks.length > 0 && todayOpen.length === 0} />
      <Metric label="今日完成率" value={`${todayCompletion}%`} detail="按安排到今天的任务计算" positive={todayCompletion === 100 && todayTasks.length > 0} />
      <Metric label="收件箱" value={String(inbox.length)} detail="待整理的临时任务" />
      <Metric label="计划 / 备忘" value={`${projects.length} / ${memos.length}`} detail="活跃计划与备忘" />
    </MetricGrid>

    {message && <Notice kind="success">{message}</Notice>}

    <div className="hx-content-grid two">
      <Panel eyebrow="QUICK CAPTURE" title="先记下来，再决定放哪里">
        <form className="hx-form" onSubmit={(event) => void quickCapture(event)}>
          <div className="hx-inline-actions">
            <button type="button" className={`hx-btn ${captureType === "task" ? "primary" : "secondary"}`} onClick={() => setCaptureType("task")}>任务</button>
            <button type="button" className={`hx-btn ${captureType === "memo" ? "primary" : "secondary"}`} onClick={() => setCaptureType("memo")}>备忘</button>
          </div>
          <label>{captureType === "task" ? "任务内容" : "备忘内容"}<textarea required rows={3} value={capture} onChange={(event) => setCapture(event.target.value)} placeholder={captureType === "task" ? "例如：周末整理本月账单" : "例如：突然想到的想法、灵感或临时信息"} /></label>
          <button className="hx-btn primary" disabled={!online}>立即收集</button>
        </form>
      </Panel>

      <Panel eyebrow="NEW TASK" title="安排一个明确任务">
        <form className="hx-form" onSubmit={(event) => void createTask(event)}>
          <label>任务名称<input required value={taskTitle} onChange={(event) => setTaskTitle(event.target.value)} placeholder="要完成什么？" /></label>
          <label>说明<textarea rows={2} value={taskDescription} onChange={(event) => setTaskDescription(event.target.value)} /></label>
          <div className="hx-form-grid">
            <label>所属计划<select value={taskProject} onChange={(event) => setTaskProject(event.target.value)}><option value="">收件箱 / 独立任务</option>{projects.map((project) => <option key={project.meta.id} value={project.meta.id}>{text(project, "name")}</option>)}</select></label>
            <label>计划日期<input type="date" value={taskDate} onChange={(event) => setTaskDate(event.target.value)} /></label>
            <label>优先级<select value={taskPriority} onChange={(event) => setTaskPriority(event.target.value)}><option value="low">低</option><option value="normal">普通</option><option value="high">高</option><option value="urgent">紧急</option></select></label>
            <label>预计分钟<input type="number" min="0" value={taskMinutes} onChange={(event) => setTaskMinutes(event.target.value)} /></label>
          </div>
          <button className="hx-btn primary" disabled={!online}>创建任务</button>
        </form>
      </Panel>
    </div>

    {view !== "projects" && view !== "memos" && <Panel eyebrow={view.toUpperCase()} title={view === "today" ? "今天的行动" : view === "inbox" ? "等待整理" : view === "completed" ? "完成历史" : "全部进行中任务"}>
      <div className="hx-list">
        {visibleTasks.map((task) => <article className="hx-row" key={task.meta.id}>
          <span className="hx-row-icon">{task.status === "done" ? "✓" : task.status === "in_progress" ? "→" : "□"}</span>
          <div className="hx-row-main">
            <strong>{text(task, "title")}</strong>
            <small>{projectMap.get(text(task, "projectId")) ? `${text(projectMap.get(text(task, "projectId"))!, "name")} · ` : ""}优先级 {taskPriorityLabel(task)} · {text(task, "dueAt") ? displayDateTime(text(task, "dueAt")) : "未安排日期"}</small>
            {text(task, "description") && <small>{text(task, "description")}</small>}
          </div>
          <div className="hx-row-actions">
            {view === "inbox" && <button className="hx-btn ghost" disabled={!online} onClick={() => void arrangeToday(task)}>安排今天</button>}
            {task.status === "todo" && <button className="hx-btn secondary" disabled={!online} onClick={() => void setTaskStatus(task, "in_progress")}>开始</button>}
            {isOpenExecutionTask(task) && <button className="hx-btn primary" disabled={!online} onClick={() => void setTaskStatus(task, "done")}>完成</button>}
            {isOpenExecutionTask(task) && <button className="hx-btn ghost" disabled={!online} onClick={() => void setTaskStatus(task, "cancelled")}>取消</button>}
          </div>
        </article>)}
        {!visibleTasks.length && <Empty title={view === "today" ? "今天没有待办" : view === "inbox" ? "收件箱已清空" : "暂无任务"} description={view === "today" ? "从收件箱安排任务，或直接创建一个今天要完成的任务。" : "使用上方快速收集，把脑中的事情先放进 LifeTrace。"} />}
      </div>
    </Panel>}

    {view === "projects" && <div className="hx-content-grid two">
      <Panel eyebrow="PLANS" title="计划与项目">
        <div className="hx-list">{projects.map((project) => {
          const own = tasks.filter((task) => text(task, "projectId") === project.meta.id);
          const ownDone = own.filter((task) => task.status === "done").length;
          return <article className="hx-row" key={project.meta.id}><span className="hx-row-icon">计</span><div className="hx-row-main"><strong>{text(project, "name")}</strong><small>{text(project, "description") || "暂无说明"}</small><small>{ownDone} / {own.length} 个任务已完成</small></div></article>;
        })}{!projects.length && <Empty title="还没有计划" description="用计划把长期事项拆成可执行任务，而不是只留下一个模糊目标。" />}</div>
      </Panel>
      <Panel eyebrow="NEW PLAN" title="创建计划">
        <form className="hx-form" onSubmit={(event) => void createProject(event)}><label>计划名称<input required value={projectName} onChange={(event) => setProjectName(event.target.value)} placeholder="例如：完成毕业论文" /></label><label>说明<textarea rows={4} value={projectDescription} onChange={(event) => setProjectDescription(event.target.value)} /></label><button className="hx-btn primary" disabled={!online}>创建计划</button></form>
      </Panel>
    </div>}

    {view === "memos" && <Panel eyebrow="MEMO TIMELINE" title="备忘时间流">
      <div className="hx-list">{[...memos].sort((a, b) => Number(b.isPinned === true) - Number(a.isPinned === true) || b.meta.updatedAt.localeCompare(a.meta.updatedAt)).map((memo) => <article className="hx-row" key={memo.meta.id}><span className="hx-row-icon">{memo.isPinned === true ? "置" : "记"}</span><div className="hx-row-main"><strong>{text(memo, "plainText").slice(0, 80) || "备忘"}</strong><small>{new Date(memo.meta.createdAt).toLocaleString("zh-CN")}{text(memo, "context") ? ` · ${text(memo, "context")}` : ""}</small></div><div className="hx-row-actions"><button className="hx-btn ghost" onClick={() => void pinMemo(memo)}>{memo.isPinned === true ? "取消置顶" : "置顶"}</button><button className="hx-btn ghost" onClick={() => void archiveMemo(memo)}>归档</button></div></article>)}{!memos.length && <Empty title="还没有备忘" description="备忘不要求标题和分类，先快速记录，需要执行时再转成任务。" />}</div>
    </Panel>}
  </PageStack>;
}
