import { useMemo, useState, type FormEvent } from "react";
import {
  atomicMutate, createExecutionGoal, createExecutionProject, goalProjectProgress,
  setGoalStatus, type EntityType, type JsonEntity,
} from "../core";
import { navigate } from "../navigation";
import { Empty, Metric, MetricGrid, Notice, PageStack, Panel, Toolbar, entities, text, type CloudPageProps } from "../ui";

function targetAt(date: string): string | null {
  if (!date) return null;
  const value = new Date(`${date}T23:59:00`);
  return Number.isNaN(value.getTime()) ? null : value.toISOString();
}

function displayTarget(value: string): string {
  if (!value) return "未设置目标日期";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value.slice(0, 10) : date.toLocaleDateString("zh-CN");
}

export function ExecutionGoalsPage({ session, state, run, online }: CloudPageProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [targetDate, setTargetDate] = useState("");
  const [firstProject, setFirstProject] = useState("");
  const [projectId, setProjectId] = useState("");
  const [goalId, setGoalId] = useState("");
  const [message, setMessage] = useState("");

  const goals = useMemo(() => entities(state, "execution.goal" as EntityType).filter((item) => item.status !== "cancelled"), [state]);
  const projects = useMemo(() => entities(state, "execution.project").filter((item) => item.status !== "cancelled" && item.status !== "archived"), [state]);
  const tasks = useMemo(() => entities(state, "execution.task"), [state]);
  const unassignedProjects = projects.filter((project) => !text(project, "goalId"));
  const activeGoals = goals.filter((goal) => goal.status === "active" || goal.status === "paused");
  const completedGoals = goals.filter((goal) => goal.status === "completed");

  async function createGoal(event: FormEvent) {
    event.preventDefault();
    const goal = createExecutionGoal(session.user.id, session.session.deviceId, {
      name,
      description,
      targetAt: targetAt(targetDate),
    });
    const projectName = firstProject.trim();
    await run(async (store) => {
      if (!projectName) return store.upsert("execution.goal" as EntityType, goal);
      const project = { ...createExecutionProject(session.user.id, session.session.deviceId, { name: projectName }), goalId: goal.meta.id };
      return atomicMutate(store, [
        { operation: "upsert", entityType: "execution.goal" as EntityType, entity: goal },
        { operation: "upsert", entityType: "execution.project", entity: project },
      ]);
    });
    setName("");
    setDescription("");
    setTargetDate("");
    setFirstProject("");
    setMessage(projectName ? "目标与首个计划已作为一个原子组创建" : "目标已创建");
  }

  async function attachProject(event: FormEvent) {
    event.preventDefault();
    const project = projects.find((item) => item.meta.id === projectId);
    const goal = goals.find((item) => item.meta.id === goalId);
    if (!project || !goal) return;
    await run((store) => store.upsert("execution.project", { ...project, goalId: goal.meta.id }));
    setProjectId("");
    setGoalId("");
    setMessage("计划已关联到目标");
  }

  async function changeStatus(goal: JsonEntity, status: "active" | "paused" | "completed" | "cancelled") {
    await run((store) => store.upsert("execution.goal" as EntityType, setGoalStatus(goal, status)));
    setMessage(status === "completed" ? "目标已完成，历史计划和任务仍保留" : "目标状态已更新");
  }

  return <PageStack>
    <Toolbar>
      <button className="hx-btn secondary" onClick={() => navigate("/execution")}>执行工作台</button>
      <button className="hx-btn secondary" onClick={() => navigate("/execution/control")}>执行控制台</button>
      <button className="hx-btn primary">目标</button>
    </Toolbar>

    <MetricGrid>
      <Metric label="进行中目标" value={String(activeGoals.length)} detail="Goal → Project → Task" />
      <Metric label="已完成目标" value={String(completedGoals.length)} detail="保留完整执行历史" positive={completedGoals.length > 0} />
      <Metric label="未归属计划" value={String(unassignedProjects.length)} detail="可关联到一个长期目标" />
      <Metric label="目标覆盖计划" value={String(projects.length - unassignedProjects.length)} detail={`共 ${projects.length} 个活跃计划`} />
    </MetricGrid>

    {message && <Notice kind="success">{message}</Notice>}

    <div className="hx-content-grid two">
      <Panel eyebrow="NEW GOAL" title="创建长期目标">
        <form className="hx-form" onSubmit={(event) => void createGoal(event)}>
          <label>目标名称<input required value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：完成毕业论文" /></label>
          <label>为什么要完成<textarea rows={3} value={description} onChange={(event) => setDescription(event.target.value)} /></label>
          <div className="hx-form-grid">
            <label>目标日期<input type="date" value={targetDate} onChange={(event) => setTargetDate(event.target.value)} /></label>
            <label>首个计划（可选）<input value={firstProject} onChange={(event) => setFirstProject(event.target.value)} placeholder="例如：完成实验章节" /></label>
          </div>
          <button className="hx-btn primary" disabled={!online}>创建目标</button>
        </form>
      </Panel>

      <Panel eyebrow="PROJECT → GOAL" title="整理现有计划">
        <form className="hx-form" onSubmit={(event) => void attachProject(event)}>
          <label>计划<select required value={projectId} onChange={(event) => setProjectId(event.target.value)}><option value="">选择计划</option>{projects.map((project) => <option key={project.meta.id} value={project.meta.id}>{text(project, "name")}{text(project, "goalId") ? " · 已有关联" : ""}</option>)}</select></label>
          <label>目标<select required value={goalId} onChange={(event) => setGoalId(event.target.value)}><option value="">选择目标</option>{activeGoals.map((goal) => <option key={goal.meta.id} value={goal.meta.id}>{text(goal, "name")}</option>)}</select></label>
          <button className="hx-btn secondary" disabled={!online || !projectId || !goalId}>建立关联</button>
        </form>
      </Panel>
    </div>

    <Panel eyebrow="GOALS" title="目标 → 计划 → 任务">
      <div className="hx-list">
        {goals.map((goal) => {
          const progress = goalProjectProgress(goal.meta.id, projects, tasks);
          return <article className="hx-row" key={goal.meta.id}>
            <span className="hx-row-icon">◎</span>
            <div className="hx-row-main">
              <strong>{text(goal, "name")}</strong>
              <small>{text(goal, "description") || "暂无说明"}</small>
              <small>{displayTarget(text(goal, "targetAt"))} · {progress.completedProjects}/{progress.projects} 个计划完成 · {progress.completedTasks}/{progress.tasks} 个任务完成 · {progress.rate}%</small>
            </div>
            <div className="hx-row-actions">
              {goal.status === "active" && <button className="hx-btn ghost" disabled={!online} onClick={() => void changeStatus(goal, "paused")}>暂停</button>}
              {goal.status === "paused" && <button className="hx-btn secondary" disabled={!online} onClick={() => void changeStatus(goal, "active")}>继续</button>}
              {goal.status !== "completed" && <button className="hx-btn primary" disabled={!online} onClick={() => void changeStatus(goal, "completed")}>完成目标</button>}
            </div>
          </article>;
        })}
        {!goals.length && <Empty title="还没有长期目标" description="目标不是另一种待办。它用于解释多个计划为什么存在，并通过计划与任务的真实完成情况计算进度。" />}
      </div>
    </Panel>
  </PageStack>;
}
