import { useState } from "react";
import { CalendarDays, ListTodo, LoaderCircle, Users, X } from "lucide-react";
import {
  browserTimezone,
  executionApi,
  localDateTimeToRfc3339,
  type ExecutionProject,
  type Memo,
} from "@/src/services/executionApi";

type Target = "task" | "calendar" | "waiting";

type Props = {
  memo: Memo;
  projects: ExecutionProject[];
  onClose: () => void;
  onConverted: () => Promise<void> | void;
};

function toast(message: string, type: "success" | "error" = "success") {
  window.dispatchEvent(new CustomEvent("hengxu-toast", { detail: { message, type } }));
}

export default function MemoConvertPanel({ memo, projects, onClose, onConverted }: Props) {
  const [target, setTarget] = useState<Target>("task");
  const [title, setTitle] = useState(memo.plainText.slice(0, 80));
  const [projectId, setProjectId] = useState("");
  const [dueAt, setDueAt] = useState("");
  const [priority, setPriority] = useState<"low" | "normal" | "high" | "urgent">("normal");
  const [allDay, setAllDay] = useState(false);
  const [startAt, setStartAt] = useState("");
  const [endAt, setEndAt] = useState("");
  const [startDate, setStartDate] = useState(new Date().toISOString().slice(0, 10));
  const [endDate, setEndDate] = useState(new Date().toISOString().slice(0, 10));
  const [waitingFor, setWaitingFor] = useState("");
  const [expectedAt, setExpectedAt] = useState("");
  const [followUpAt, setFollowUpAt] = useState("");
  const [busy, setBusy] = useState(false);

  const convert = async () => {
    if (!title.trim()) return;
    setBusy(true);
    try {
      if (target === "task") {
        await executionApi.memos.convertToTask(memo.id, {
          title: title.trim(),
          description: memo.content,
          projectId: projectId || null,
          priority,
          dueAt: localDateTimeToRfc3339(dueAt),
          timezone: browserTimezone(),
        });
      } else if (target === "calendar") {
        if (allDay && (!startDate || !endDate)) return;
        if (!allDay && (!startAt || !endAt)) return;
        await executionApi.memos.convertToCalendar(memo.id, {
          title: title.trim(),
          description: memo.content,
          timing: {
            isAllDay: allDay,
            startAt: allDay ? null : localDateTimeToRfc3339(startAt),
            endAt: allDay ? null : localDateTimeToRfc3339(endAt),
            startLocalDate: allDay ? startDate : null,
            endLocalDate: allDay ? endDate : null,
            timezone: browserTimezone(),
          },
        });
      } else {
        if (!waitingFor.trim()) return;
        await executionApi.memos.convertToWaiting(memo.id, {
          title: title.trim(),
          description: memo.content,
          waitingFor: waitingFor.trim(),
          expectedAt: localDateTimeToRfc3339(expectedAt),
          followUpAt: localDateTimeToRfc3339(followUpAt),
        });
      }
      toast(target === "task" ? "Memo 已转为任务" : target === "calendar" ? "Memo 已转为日历事件" : "Memo 已转为等待事项");
      await onConverted();
      onClose();
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "Memo 转换失败", "error");
    } finally {
      setBusy(false);
    }
  };

  const disabled = !title.trim()
    || (target === "calendar" && (allDay ? (!startDate || !endDate) : (!startAt || !endAt)))
    || (target === "waiting" && !waitingFor.trim());

  return <div className="lt-exec-editor lt-exec-convert-panel" role="dialog" aria-modal="true" aria-label="转换 Memo">
    <header><div><strong>转换 Memo</strong><span>保留来源关联，转换后自动归档</span></div><button type="button" onClick={onClose} aria-label="关闭转换面板"><X/></button></header>
    <div className="lt-exec-convert-body">
      <div className="lt-exec-target-tabs" role="tablist" aria-label="转换目标">
        <button type="button" role="tab" aria-selected={target === "task"} className={target === "task" ? "active" : ""} onClick={() => setTarget("task")}><ListTodo/>任务</button>
        <button type="button" role="tab" aria-selected={target === "calendar"} className={target === "calendar" ? "active" : ""} onClick={() => setTarget("calendar")}><CalendarDays/>日历</button>
        <button type="button" role="tab" aria-selected={target === "waiting"} className={target === "waiting" ? "active" : ""} onClick={() => setTarget("waiting")}><Users/>等待</button>
      </div>
      <div className="lt-exec-form">
        <label>标题<input autoFocus value={title} onChange={(event) => setTitle(event.target.value)}/></label>
        {target === "task" ? <>
          <label>项目<select value={projectId} onChange={(event) => setProjectId(event.target.value)}><option value="">无项目</option>{projects.filter((project) => project.status === "active").map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}</select></label>
          <div className="lt-exec-form-grid"><label>优先级<select value={priority} onChange={(event) => setPriority(event.target.value as typeof priority)}><option value="low">低</option><option value="normal">普通</option><option value="high">高</option><option value="urgent">紧急</option></select></label><label>截止时间<input type="datetime-local" value={dueAt} onChange={(event) => setDueAt(event.target.value)}/></label></div>
        </> : null}
        {target === "calendar" ? <>
          <label className="lt-exec-checkbox"><input type="checkbox" checked={allDay} onChange={(event) => setAllDay(event.target.checked)}/>全天事件</label>
          {allDay ? <div className="lt-exec-form-grid"><label>开始日期<input type="date" value={startDate} onChange={(event) => setStartDate(event.target.value)}/></label><label>结束日期<input type="date" value={endDate} onChange={(event) => setEndDate(event.target.value)}/></label></div> : <div className="lt-exec-form-grid"><label>开始<input type="datetime-local" value={startAt} onChange={(event) => setStartAt(event.target.value)}/></label><label>结束<input type="datetime-local" value={endAt} onChange={(event) => setEndAt(event.target.value)}/></label></div>}
        </> : null}
        {target === "waiting" ? <>
          <label>等待对象<input value={waitingFor} onChange={(event) => setWaitingFor(event.target.value)} placeholder="人、团队或外部结果"/></label>
          <div className="lt-exec-form-grid"><label>预计返回<input type="datetime-local" value={expectedAt} onChange={(event) => setExpectedAt(event.target.value)}/></label><label>跟进时间<input type="datetime-local" value={followUpAt} onChange={(event) => setFollowUpAt(event.target.value)}/></label></div>
        </> : null}
        <div className="lt-exec-source-preview"><strong>原 Memo</strong><p>{memo.content}</p></div>
      </div>
    </div>
    <footer><span/><div><button type="button" onClick={onClose}>取消</button><button className="hx-btn primary" type="button" disabled={busy || disabled} onClick={() => void convert()}>{busy ? <LoaderCircle className="spin"/> : null}确认转换</button></div></footer>
  </div>;
}
