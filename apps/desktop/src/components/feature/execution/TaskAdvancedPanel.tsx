import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Bell,
  CalendarDays,
  Check,
  GitBranch,
  LoaderCircle,
  Pencil,
  Plus,
  Repeat2,
  Trash2,
  X,
} from "lucide-react";
import { normalizeWeekdays } from "@/src/components/feature/execution/executionViewModel";
import {
  browserTimezone,
  executionApi,
  rfc3339ToLocalDateTime,
  localDateTimeToRfc3339,
  type ExecutionProject,
  type ExecutionTask,
  type RecurrenceRule,
  type TaskBlocker,
} from "@/src/services/executionApi";

type Dependency = { id: string; taskId: string; dependsOnTaskId: string };

type Props = {
  task: ExecutionTask;
  projects: ExecutionProject[];
  allTasks: ExecutionTask[];
  onClose: () => void;
  onEdit: () => void;
  onSchedule: () => void;
  onReminder: () => void;
  onChanged: () => Promise<void> | void;
};

function toast(message: string, type: "success" | "error" = "success") {
  window.dispatchEvent(new CustomEvent("hengxu-toast", { detail: { message, type } }));
}

function parseWeekdays(rule: RecurrenceRule | null) {
  if (!rule?.weekdaysJson) return [] as number[];
  try {
    const value = JSON.parse(rule.weekdaysJson);
    return Array.isArray(value) ? value.filter((item) => typeof item === "number") : [];
  } catch {
    return [];
  }
}

export default function TaskAdvancedPanel({
  task,
  projects,
  allTasks,
  onClose,
  onEdit,
  onSchedule,
  onReminder,
  onChanged,
}: Props) {
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [subtasks, setSubtasks] = useState<ExecutionTask[]>([]);
  const [dependencies, setDependencies] = useState<Dependency[]>([]);
  const [blockers, setBlockers] = useState<TaskBlocker[]>([]);
  const [recurrence, setRecurrence] = useState<RecurrenceRule | null>(null);
  const [subtaskTitle, setSubtaskTitle] = useState("");
  const [dependencyId, setDependencyId] = useState("");
  const [frequency, setFrequency] = useState("weekly");
  const [intervalValue, setIntervalValue] = useState("1");
  const [weekdays, setWeekdays] = useState<number[]>([]);
  const [untilAt, setUntilAt] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [nextSubtasks, nextDependencies, nextBlockers, nextRecurrence] = await Promise.all([
        executionApi.tasks.subtasks(task.id),
        executionApi.tasks.dependencies(task.id),
        executionApi.tasks.blockers(task.id),
        executionApi.tasks.recurrence(task.id),
      ]);
      setSubtasks(nextSubtasks);
      setDependencies(nextDependencies);
      setBlockers(nextBlockers);
      setRecurrence(nextRecurrence);
      if (nextRecurrence) {
        setFrequency(nextRecurrence.frequency);
        setIntervalValue(String(nextRecurrence.intervalValue || 1));
        setWeekdays(parseWeekdays(nextRecurrence));
        setUntilAt(rfc3339ToLocalDateTime(nextRecurrence.untilAt));
      }
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "任务详情加载失败", "error");
    } finally {
      setLoading(false);
    }
  }, [task.id]);

  useEffect(() => { void load(); }, [load]);

  const dependencyCandidates = useMemo(() => {
    const existing = new Set(dependencies.map((item) => item.dependsOnTaskId));
    return allTasks.filter((item) => item.id !== task.id && !existing.has(item.id) && item.status !== "cancelled");
  }, [allTasks, dependencies, task.id]);
  const taskById = useMemo(() => new Map(allTasks.map((item) => [item.id, item])), [allTasks]);
  const project = task.projectId ? projects.find((item) => item.id === task.projectId) : undefined;

  const run = async (action: () => Promise<unknown>, success: string) => {
    setBusy(true);
    try {
      await action();
      toast(success);
      await load();
      await onChanged();
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "操作失败", "error");
    } finally {
      setBusy(false);
    }
  };

  const addSubtask = async () => {
    const title = subtaskTitle.trim();
    if (!title) return;
    setSubtaskTitle("");
    await run(
      () => executionApi.tasks.addSubtask(task.id, {
        title,
        projectId: task.projectId || null,
        priority: "normal",
        timezone: browserTimezone(),
      }),
      "子任务已添加",
    );
  };

  const saveRecurrence = () => run(
    () => executionApi.tasks.setRecurrence(task.id, {
      frequency,
      intervalValue: Math.max(1, Number(intervalValue) || 1),
      weekdays: frequency === "weekly" ? normalizeWeekdays(weekdays) : [],
      untilAt: localDateTimeToRfc3339(untilAt),
      timezone: browserTimezone(),
    }),
    "重复规则已保存",
  );

  const toggleWeekday = (day: number) => {
    setWeekdays((current) => current.includes(day) ? current.filter((item) => item !== day) : [...current, day].sort());
  };

  return <div className="lt-exec-editor lt-exec-inspector" role="dialog" aria-modal="true" aria-label={`任务详情：${task.title}`}>
    <header>
      <div><strong>{task.title}</strong><span>{project?.name || "无项目"} · {task.status}</span></div>
      <button type="button" onClick={onClose} aria-label="关闭任务详情"><X/></button>
    </header>
    <div className="lt-exec-inspector-body">
      <div className="lt-exec-inspector-actions">
        <button type="button" onClick={onEdit}><Pencil/>编辑</button>
        <button type="button" onClick={onSchedule}><CalendarDays/>安排</button>
        <button type="button" onClick={onReminder}><Bell/>提醒</button>
      </div>
      {loading ? <div className="lt-exec-loading compact"><LoaderCircle className="spin"/><span>读取任务结构…</span></div> : <>
        <section className="lt-exec-inspector-section">
          <header><div><strong>子任务</strong><span>{subtasks.length}</span></div></header>
          <div className="lt-exec-inline-create"><input value={subtaskTitle} onChange={(event) => setSubtaskTitle(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") void addSubtask(); }} placeholder="添加一个更小的下一步"/><button type="button" disabled={!subtaskTitle.trim() || busy} onClick={() => void addSubtask()}><Plus/>添加</button></div>
          <div className="lt-exec-compact-list">{subtasks.length ? subtasks.map((item) => <div key={item.id}><button type="button" className={item.status === "done" ? "done" : ""} onClick={() => void run(() => executionApi.tasks.setStatus(item.id, item.status === "done" ? "todo" : "done"), "子任务状态已更新")}><span className="lt-exec-mini-check">{item.status === "done" ? <Check/> : null}</span><span>{item.title}</span></button></div>) : <span className="lt-exec-muted">暂无子任务</span>}</div>
        </section>

        <section className="lt-exec-inspector-section">
          <header><div><strong>前置依赖</strong><span>{dependencies.length}</span></div><GitBranch/></header>
          <div className="lt-exec-inline-create"><select value={dependencyId} onChange={(event) => setDependencyId(event.target.value)}><option value="">选择必须先完成的任务</option>{dependencyCandidates.map((item) => <option key={item.id} value={item.id}>{item.title}</option>)}</select><button type="button" disabled={!dependencyId || busy} onClick={() => void run(async () => { await executionApi.tasks.addDependency(task.id, dependencyId); setDependencyId(""); }, "依赖已添加")}><Plus/>添加</button></div>
          <div className="lt-exec-compact-list">{dependencies.length ? dependencies.map((item) => <div key={item.id}><span><GitBranch/><strong>{taskById.get(item.dependsOnTaskId)?.title || item.dependsOnTaskId}</strong></span><button type="button" title="移除依赖" onClick={() => void run(() => executionApi.tasks.removeDependency(task.id, item.dependsOnTaskId), "依赖已移除")}><Trash2/></button></div>) : <span className="lt-exec-muted">暂无前置依赖</span>}</div>
          {blockers.length ? <div className="lt-exec-blockers"><strong>当前被阻塞</strong>{blockers.map((item) => <span key={item.taskId}>{item.title} · {item.status}</span>)}</div> : null}
        </section>

        <section className="lt-exec-inspector-section">
          <header><div><strong>重复规则</strong><span>{recurrence ? "已启用" : "未启用"}</span></div><Repeat2/></header>
          <div className="lt-exec-recurrence-grid">
            <label>频率<select value={frequency} onChange={(event) => setFrequency(event.target.value)}><option value="daily">每天</option><option value="weekly">每周</option><option value="monthly">每月</option></select></label>
            <label>间隔<input type="number" min="1" value={intervalValue} onChange={(event) => setIntervalValue(event.target.value)}/></label>
            <label className="wide">结束时间<input type="datetime-local" value={untilAt} onChange={(event) => setUntilAt(event.target.value)}/></label>
          </div>
          {frequency === "weekly" ? <div className="lt-exec-weekdays" aria-label="重复星期">{[[1,"一"],[2,"二"],[3,"三"],[4,"四"],[5,"五"],[6,"六"],[7,"日"]].map(([day,label]) => <button key={day} type="button" className={weekdays.includes(day as number) ? "active" : ""} aria-pressed={weekdays.includes(day as number)} onClick={() => toggleWeekday(day as number)}>{label}</button>)}</div> : null}
          <div className="lt-exec-inspector-footer-actions"><button type="button" disabled={busy} onClick={() => void saveRecurrence()}>{busy ? <LoaderCircle className="spin"/> : null}保存重复规则</button>{recurrence ? <button className="danger" type="button" disabled={busy} onClick={() => void run(() => executionApi.tasks.clearRecurrence(task.id), "重复规则已关闭")}>关闭重复</button> : null}</div>
        </section>
      </>}
    </div>
  </div>;
}
