import { useMemo, useState, type FormEvent } from "react";
import { useSearchParams } from "react-router-dom";
import { Check, Circle, Clock3, FolderKanban, Plus, Trash2 } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, Card, CardContent, EmptyState, Input, PageHeader, Section, Select, cn } from "../../components/ui";
import { entities, formatDateTime, text, todayKey } from "../../lib/entities";
import {
  createExecutionProject,
  createExecutionTask,
  isOpenExecutionTask,
  taskMatchesToday,
  type JsonEntity,
} from "../../services/core";

type View = "today" | "inbox" | "upcoming" | "projects" | "completed";

const labels: Record<View, string> = {
  today: "今天",
  inbox: "Inbox",
  upcoming: "即将到来",
  projects: "Projects",
  completed: "已完成",
};

export function ExecutionPage() {
  const { state, session, upsert, remove, loading } = useApp();
  const [params, setParams] = useSearchParams();
  const [view, setView] = useState<View>("today");
  const [title, setTitle] = useState("");
  const [dueDate, setDueDate] = useState("");
  const [priority, setPriority] = useState("normal");
  const [projectId, setProjectId] = useState("");
  const [projectName, setProjectName] = useState("");
  const [showProjectComposer, setShowProjectComposer] = useState(false);
  const tasks = entities(state, "execution.task");
  const projects = entities(state, "execution.project").filter((project) => text(project, "status", "active") !== "archived");
  const today = todayKey();
  const showComposer = params.get("new") === "task";

  const visible = useMemo(() => tasks.filter((task) => {
    if (view === "completed") return text(task, "status") === "done";
    if (view === "projects") return false;
    if (!isOpenExecutionTask(task)) return false;
    if (view === "today") return taskMatchesToday(task, today);
    if (view === "inbox") return !task.projectId && !task.dueAt && !task.scheduledStartAt;
    const key = text(task, "scheduledStartAt") || text(task, "dueAt");
    return Boolean(key && key.slice(0, 10) > today);
  }).sort((a, b) => text(a, "scheduledStartAt", text(a, "dueAt")).localeCompare(text(b, "scheduledStartAt", text(b, "dueAt")))), [tasks, today, view]);

  async function createTask(event: FormEvent) {
    event.preventDefault();
    if (!session) return;
    const dueAt = dueDate
      ? new Date(`${dueDate}T23:59:00`).toISOString()
      : view === "today"
        ? new Date(`${today}T23:59:00`).toISOString()
        : null;
    const task = createExecutionTask(session.user.id, session.session.deviceId, {
      title,
      priority: priority as "low" | "normal" | "high" | "urgent",
      dueAt,
      projectId: projectId || null,
      context: view === "inbox" ? "inbox" : null,
    });
    await upsert("execution.task", task);
    setTitle("");
    setDueDate("");
    setProjectId("");
    setParams({}, { replace: true });
  }

  async function createProject(event: FormEvent) {
    event.preventDefault();
    if (!session) return;
    await upsert("execution.project", createExecutionProject(session.user.id, session.session.deviceId, { name: projectName }));
    setProjectName("");
    setShowProjectComposer(false);
  }

  async function toggle(task: JsonEntity) {
    const done = text(task, "status") === "done";
    await upsert("execution.task", {
      ...task,
      status: done ? "todo" : "done",
      completedAt: done ? null : new Date().toISOString(),
    });
  }

  return <div className="page-shell">
    <PageHeader
      title="计划与待办"
      description="Inbox → Today → Upcoming → Projects → Completed。页面结构参考 Shadcnblocks Todo，列表/详情层级参考 Catalyst。"
      action={<Button onClick={() => setParams({ new: "task" })}><Plus size={16} />新建任务</Button>}
    />

    <div className="grid gap-5 lg:grid-cols-[220px_minmax(0,1fr)] xl:grid-cols-[220px_minmax(0,1fr)_280px]">
      <Card className="h-fit">
        <CardContent className="p-2">
          {(["today", "inbox", "upcoming", "projects", "completed"] as View[]).map((item) => (
            <button
              key={item}
              onClick={() => setView(item)}
              className={cn("flex w-full items-center justify-between rounded-md px-3 py-2 text-sm", view === item ? "bg-accent font-medium text-accent-foreground" : "text-muted-foreground hover:bg-muted")}
            >
              <span className="flex items-center gap-2">{item === "projects" ? <FolderKanban size={15} /> : null}{labels[item]}</span>
              <span className="text-xs">{item === "projects" ? projects.length : item === "completed" ? tasks.filter((task) => text(task, "status") === "done").length : ""}</span>
            </button>
          ))}
        </CardContent>
      </Card>

      <div className="min-w-0">
        {showComposer ? <Card className="mb-4"><CardContent className="pt-5"><form className="grid gap-3 xl:grid-cols-[minmax(0,1fr)_150px_130px_180px_auto]" onSubmit={(event) => void createTask(event)}>
          <Input autoFocus placeholder="任务内容" value={title} onChange={(event) => setTitle(event.target.value)} required />
          <Input type="date" value={dueDate} onChange={(event) => setDueDate(event.target.value)} />
          <Select value={priority} onChange={(event) => setPriority(event.target.value)}><option value="normal">普通</option><option value="high">高</option><option value="urgent">紧急</option><option value="low">低</option></Select>
          <Select value={projectId} onChange={(event) => setProjectId(event.target.value)}><option value="">无项目</option>{projects.map((project) => <option key={project.meta.id} value={project.meta.id}>{text(project, "name", "项目")}</option>)}</Select>
          <div className="flex gap-2"><Button type="submit" disabled={loading}>保存</Button><Button variant="ghost" onClick={() => setParams({}, { replace: true })}>取消</Button></div>
        </form></CardContent></Card> : null}

        {view === "projects" ? <ProjectsView
          projects={projects}
          tasks={tasks}
          showComposer={showProjectComposer}
          projectName={projectName}
          loading={loading}
          onShowComposer={() => setShowProjectComposer(true)}
          onHideComposer={() => setShowProjectComposer(false)}
          onProjectName={setProjectName}
          onCreate={(event) => void createProject(event)}
          onNewTask={(id) => { setProjectId(id); setParams({ new: "task" }); }}
        /> : <Section title={labels[view]} description={`${visible.length} 项`}>
          {visible.length ? <Card><div className="divide-y">{visible.map((task) => <div key={task.meta.id} className="group flex items-start gap-3 px-4 py-3.5">
            <button className={cn("mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border", text(task, "status") === "done" && "border-primary bg-primary text-primary-foreground")} onClick={() => void toggle(task)} aria-label="切换完成状态">{text(task, "status") === "done" ? <Check size={13} /> : <Circle size={12} className="opacity-0" />}</button>
            <div className="min-w-0 flex-1"><div className={cn("text-sm font-medium", text(task, "status") === "done" && "text-muted-foreground line-through")}>{text(task, "title", "未命名任务")}</div><div className="mt-1 flex flex-wrap gap-2 text-xs text-muted-foreground">{task.dueAt ? <span className="flex items-center gap-1"><Clock3 size={12} />{formatDateTime(task.dueAt)}</span> : null}<Badge className={text(task, "priority") === "urgent" ? "border-destructive/30 text-destructive" : ""}>{text(task, "priority", "normal")}</Badge>{task.projectId ? <span>{text(projects.find((item) => item.meta.id === task.projectId), "name", "项目")}</span> : null}</div></div>
            <Button size="icon" variant="ghost" className="opacity-60 sm:opacity-0 sm:group-hover:opacity-100" onClick={() => void remove("execution.task", task.meta.id)} aria-label="删除任务"><Trash2 size={15} /></Button>
          </div>)}</div></Card> : <EmptyState title="这里还没有任务" description={view === "today" ? "把真正要做的事情安排到今天，避免 KPI 墙和无意义堆积。" : "新建任务后会自动出现在对应视图。"} action={<Button variant="outline" onClick={() => setParams({ new: "task" })}>添加任务</Button>} />}
        </Section>}
      </div>

      <Card className="hidden h-fit xl:block"><CardContent className="pt-5"><div className="eyebrow">执行概览</div><div className="mt-3 space-y-3 text-sm"><div className="flex justify-between"><span className="text-muted-foreground">开放任务</span><strong>{tasks.filter(isOpenExecutionTask).length}</strong></div><div className="flex justify-between"><span className="text-muted-foreground">Projects</span><strong>{projects.length}</strong></div><div className="flex justify-between"><span className="text-muted-foreground">今日</span><strong>{tasks.filter((task) => taskMatchesToday(task, today) && isOpenExecutionTask(task)).length}</strong></div></div></CardContent></Card>
    </div>
  </div>;
}

function ProjectsView({ projects, tasks, showComposer, projectName, loading, onShowComposer, onHideComposer, onProjectName, onCreate, onNewTask }: {
  projects: JsonEntity[];
  tasks: JsonEntity[];
  showComposer: boolean;
  projectName: string;
  loading: boolean;
  onShowComposer(): void;
  onHideComposer(): void;
  onProjectName(value: string): void;
  onCreate(event: FormEvent): void;
  onNewTask(projectId: string): void;
}) {
  return <Section title="Projects" description={`${projects.length} 个活跃项目`} action={<Button size="sm" variant="outline" onClick={onShowComposer}><Plus size={14} />新建 Project</Button>}>
    {showComposer ? <Card className="mb-4"><CardContent className="pt-5"><form className="flex gap-2" onSubmit={onCreate}><Input autoFocus placeholder="Project 名称" value={projectName} onChange={(event) => onProjectName(event.target.value)} required /><Button type="submit" disabled={loading}>保存</Button><Button variant="ghost" onClick={onHideComposer}>取消</Button></form></CardContent></Card> : null}
    {projects.length ? <div className="grid gap-3 md:grid-cols-2">{projects.map((project) => {
      const projectTasks = tasks.filter((task) => task.projectId === project.meta.id);
      const open = projectTasks.filter(isOpenExecutionTask).length;
      const done = projectTasks.filter((task) => text(task, "status") === "done").length;
      const total = open + done;
      return <Card key={project.meta.id}><CardContent className="pt-5"><div className="flex items-start justify-between gap-3"><div><div className="font-semibold">{text(project, "name", "Project")}</div><p className="mt-1 text-xs leading-5 text-muted-foreground">{text(project, "description", "把相关任务聚合成一个持续推进的结果。")}</p></div><Badge>{open} open</Badge></div><div className="mt-4 flex items-center justify-between text-xs text-muted-foreground"><span>{done} 已完成 / {total} 总任务</span><Button size="sm" variant="ghost" onClick={() => onNewTask(project.meta.id)}><Plus size={13} />添加任务</Button></div></CardContent></Card>;
    })}</div> : <EmptyState icon={<FolderKanban size={24} />} title="还没有 Project" description="用 Project 聚合同一目标下的任务，避免把所有事情塞进 Today。" action={<Button variant="outline" onClick={onShowComposer}>创建第一个 Project</Button>} />}
  </Section>;
}
