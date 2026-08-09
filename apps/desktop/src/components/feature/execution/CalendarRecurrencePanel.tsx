import { useCallback, useEffect, useState } from "react";
import { LoaderCircle, Repeat2, Trash2, X } from "lucide-react";
import {
  browserTimezone,
  executionApi,
  type CalendarEvent,
  type RecurrenceRule,
} from "@/src/services/executionApi";

const weekdayOptions = [[1, "一"], [2, "二"], [3, "三"], [4, "四"], [5, "五"], [6, "六"], [7, "日"]] as const;

function toast(message: string, type: "success" | "error" = "success") {
  window.dispatchEvent(new CustomEvent("hengxu-toast", { detail: { message, type } }));
}

function parseWeekdays(rule: RecurrenceRule | null) {
  if (!rule?.weekdaysJson) return [] as number[];
  try {
    const value = JSON.parse(rule.weekdaysJson);
    return Array.isArray(value) ? value.filter((item) => Number.isInteger(item) && item >= 1 && item <= 7) : [];
  } catch {
    return [];
  }
}

type Props = {
  event: CalendarEvent;
  onClose: () => void;
  onChanged: () => Promise<void> | void;
};

export default function CalendarRecurrencePanel({ event, onClose, onChanged }: Props) {
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [rule, setRule] = useState<RecurrenceRule | null>(null);
  const [frequency, setFrequency] = useState("weekly");
  const [intervalValue, setIntervalValue] = useState("1");
  const [weekdays, setWeekdays] = useState<number[]>([]);
  const [monthDay, setMonthDay] = useState(String(new Date().getDate()));
  const [untilAt, setUntilAt] = useState("");
  const [maxOccurrences, setMaxOccurrences] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const next = await executionApi.calendar.recurrence(event.id);
      setRule(next);
      if (next) {
        setFrequency(next.frequency);
        setIntervalValue(String(next.intervalValue || 1));
        setWeekdays(parseWeekdays(next));
        setMonthDay(String(next.monthDay || new Date().getDate()));
        setUntilAt(next.untilAt?.slice(0, 10) || "");
        setMaxOccurrences(next.maxOccurrences ? String(next.maxOccurrences) : "");
      }
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "重复规则读取失败", "error");
    } finally {
      setLoading(false);
    }
  }, [event.id]);

  useEffect(() => { void load(); }, [load]);

  const run = async (action: () => Promise<unknown>, success: string) => {
    setBusy(true);
    try {
      await action();
      toast(success);
      await load();
      await onChanged();
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "重复规则操作失败", "error");
    } finally {
      setBusy(false);
    }
  };

  const toggleWeekday = (value: number) => setWeekdays((current) => current.includes(value)
    ? current.filter((item) => item !== value)
    : [...current, value].sort((a, b) => a - b));

  const valid = frequency !== "weekly" || weekdays.length > 0;
  const save = () => run(() => executionApi.calendar.setRecurrence(event.id, {
    frequency,
    intervalValue: Math.max(1, Number(intervalValue) || 1),
    weekdays: frequency === "weekly" ? weekdays : [],
    monthDay: frequency === "monthly" ? Math.min(31, Math.max(1, Number(monthDay) || 1)) : null,
    timezone: event.timezone || browserTimezone(),
    untilAt: untilAt || null,
    maxOccurrences: maxOccurrences ? Math.max(1, Number(maxOccurrences) || 1) : null,
  }), rule ? "重复规则已更新" : "重复规则已启用");

  return <div className="lt-exec-editor lt-exec-inspector" role="dialog" aria-modal="true" aria-label={`日历重复规则：${event.title}`}>
    <header>
      <div><strong>{event.title}</strong><span>重复事件规则</span></div>
      <button type="button" onClick={onClose} aria-label="关闭重复规则"><X/></button>
    </header>
    <div className="lt-exec-inspector-body">
      {loading ? <div className="lt-exec-loading compact"><LoaderCircle className="spin"/><span>读取重复规则…</span></div> : <section className="lt-exec-inspector-section">
        <header><div><strong>重复规则</strong><span>{rule ? "已启用" : "未启用"}</span></div><Repeat2/></header>
        <div className="lt-exec-recurrence-grid">
          <label>频率<select value={frequency} onChange={(changeEvent) => setFrequency(changeEvent.target.value)}><option value="daily">每天</option><option value="weekly">每周</option><option value="monthly">每月</option></select></label>
          <label>间隔<input type="number" min="1" value={intervalValue} onChange={(changeEvent) => setIntervalValue(changeEvent.target.value)}/></label>
          {frequency === "monthly" ? <label>每月第几日<input type="number" min="1" max="31" value={monthDay} onChange={(changeEvent) => setMonthDay(changeEvent.target.value)}/></label> : null}
          <label>结束日期<input type="date" value={untilAt} onChange={(changeEvent) => setUntilAt(changeEvent.target.value)}/></label>
          <label>最多次数<input type="number" min="1" value={maxOccurrences} onChange={(changeEvent) => setMaxOccurrences(changeEvent.target.value)} placeholder="不限"/></label>
        </div>
        {frequency === "weekly" ? <div className="lt-exec-weekdays" aria-label="重复星期">{weekdayOptions.map(([day, label]) => <button key={day} type="button" className={weekdays.includes(day) ? "active" : ""} aria-pressed={weekdays.includes(day)} onClick={() => toggleWeekday(day)}>{label}</button>)}</div> : null}
        <p className="lt-exec-muted">修改规则只影响后续重复安排；历史 occurrence 不会被重写。</p>
        <div className="lt-exec-inspector-footer-actions"><button type="button" disabled={busy || !valid} onClick={() => void save()}>{busy ? <LoaderCircle className="spin"/> : null}保存重复规则</button>{rule ? <button className="danger" type="button" disabled={busy} onClick={() => void run(() => executionApi.calendar.clearRecurrence(event.id), "重复规则已关闭")}><Trash2/>关闭重复</button> : null}</div>
      </section>}
    </div>
  </div>;
}
