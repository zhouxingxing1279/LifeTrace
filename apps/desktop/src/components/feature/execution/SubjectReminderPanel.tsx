import { useCallback, useEffect, useState } from "react";
import { Bell, Check, Clock3, LoaderCircle, Plus, RotateCcw, Trash2, X } from "lucide-react";
import {
  browserTimezone,
  executionApi,
  localDateTimeToRfc3339,
  rfc3339ToLocalDateTime,
  type Reminder,
} from "@/src/services/executionApi";

type Props = {
  subjectType: Reminder["subjectType"];
  subjectId: string;
  title: string;
  onClose: () => void;
  onChanged?: () => Promise<void> | void;
};

function toast(message: string, type: "success" | "error" = "success") {
  window.dispatchEvent(new CustomEvent("hengxu-toast", { detail: { message, type } }));
}

function format(value?: string | null) {
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

export default function SubjectReminderPanel({ subjectType, subjectId, title, onClose, onChanged }: Props) {
  const [items, setItems] = useState<Reminder[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [newTime, setNewTime] = useState("");
  const [snoozeId, setSnoozeId] = useState<string | null>(null);
  const [snoozeTime, setSnoozeTime] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setItems(await executionApi.reminders.list(subjectType, subjectId));
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "提醒读取失败", "error");
    } finally {
      setLoading(false);
    }
  }, [subjectId, subjectType]);

  useEffect(() => { void load(); }, [load]);

  const run = async (id: string, action: () => Promise<unknown>, success: string) => {
    setBusyId(id);
    try {
      await action();
      toast(success);
      await load();
      await onChanged?.();
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "提醒操作失败", "error");
    } finally {
      setBusyId(null);
    }
  };

  const add = async () => {
    const triggerAt = localDateTimeToRfc3339(newTime);
    if (!triggerAt) return;
    setBusyId("new");
    try {
      await executionApi.reminders.create({
        subjectType,
        subjectId,
        triggerAt,
        timezone: browserTimezone(),
      });
      setNewTime("");
      toast("提醒已添加");
      await load();
      await onChanged?.();
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "提醒创建失败", "error");
    } finally {
      setBusyId(null);
    }
  };

  const saveSnooze = async (item: Reminder) => {
    const untilAt = localDateTimeToRfc3339(snoozeTime);
    if (!untilAt) return;
    await run(item.id, () => executionApi.reminders.snooze(item.id, untilAt), "提醒已稍后处理");
    setSnoozeId(null);
    setSnoozeTime("");
  };

  return <div className="lt-exec-editor lt-exec-reminder-panel" role="dialog" aria-modal="true" aria-label={`提醒：${title}`}>
    <header>
      <div><strong>提醒</strong><span>{title}</span></div>
      <button type="button" onClick={onClose} aria-label="关闭提醒管理"><X/></button>
    </header>
    <div className="lt-exec-reminder-body">
      <div className="lt-exec-reminder-create">
        <label>新增提醒<input type="datetime-local" value={newTime} onChange={(event) => setNewTime(event.target.value)}/></label>
        <button type="button" disabled={!newTime || busyId === "new"} onClick={() => void add()}>{busyId === "new" ? <LoaderCircle className="spin"/> : <Plus/>}添加</button>
      </div>
      {loading ? <div className="lt-exec-loading compact"><LoaderCircle className="spin"/><span>读取提醒…</span></div> : items.length ? <div className="lt-exec-reminder-list">{items.map((item) => <article key={item.id}>
        <div className="lt-exec-reminder-main"><Bell/><div><strong>{format(item.snoozedUntil || item.triggerAt)}</strong><span>{item.status}{item.snoozedUntil ? ` · 原定 ${format(item.triggerAt)}` : ""}</span></div></div>
        {snoozeId === item.id ? <div className="lt-exec-reminder-snooze"><input autoFocus type="datetime-local" value={snoozeTime} onChange={(event) => setSnoozeTime(event.target.value)}/><button type="button" disabled={!snoozeTime || busyId === item.id} onClick={() => void saveSnooze(item)}><Clock3/>确认</button><button type="button" onClick={() => setSnoozeId(null)}><X/>取消</button></div> : null}
        <footer>
          {(item.status === "scheduled" || item.status === "fired") ? <button type="button" onClick={() => { setSnoozeId(item.id); setSnoozeTime(rfc3339ToLocalDateTime(item.snoozedUntil || item.triggerAt)); }}><RotateCcw/>稍后</button> : null}
          {(item.status === "scheduled" || item.status === "fired") ? <button type="button" disabled={busyId === item.id} onClick={() => void run(item.id, () => executionApi.reminders.dismiss(item.id), "提醒已处理")}><Check/>处理</button> : null}
          {item.status === "scheduled" ? <button type="button" disabled={busyId === item.id} onClick={() => void run(item.id, () => executionApi.reminders.cancel(item.id), "提醒已取消")}><X/>取消</button> : null}
          <button className="danger" type="button" disabled={busyId === item.id} onClick={() => void run(item.id, () => executionApi.reminders.remove(item.id), "提醒已删除")}><Trash2/>删除</button>
        </footer>
      </article>)}</div> : <div className="lt-exec-empty compact"><Bell/><span>还没有提醒</span></div>}
    </div>
  </div>;
}
