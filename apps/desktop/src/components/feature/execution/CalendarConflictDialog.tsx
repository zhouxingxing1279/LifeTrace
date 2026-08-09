import { AlertTriangle, X } from "lucide-react";

export type CalendarConflict = {
  eventId: string;
  occurrenceId?: string | null;
  title: string;
  isAllDay: boolean;
};

type Props = {
  title: string;
  conflicts: CalendarConflict[];
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
};

export default function CalendarConflictDialog({ title, conflicts, busy, onCancel, onConfirm }: Props) {
  return <div className="lt-exec-editor compact lt-calendar-conflict-dialog" role="alertdialog" aria-modal="true" aria-label="日历时间冲突">
    <header>
      <div><strong>存在时间冲突</strong><span>{title}</span></div>
      <button type="button" onClick={onCancel} aria-label="关闭冲突提示"><X/></button>
    </header>
    <div className="lt-calendar-conflict-body">
      <div className="lt-calendar-conflict-summary"><AlertTriangle/><span>这个时间段与 {conflicts.length} 个已有安排重叠。冲突只是警告，你仍然可以继续。</span></div>
      <div className="lt-calendar-conflict-list">{conflicts.map((item, index) => <div key={`${item.eventId}:${item.occurrenceId || index}`}><strong>{item.title}</strong><span>{item.occurrenceId ? "重复事件实例" : item.isAllDay ? "全天事件" : "日历事件"}</span></div>)}</div>
    </div>
    <footer><span/><div><button type="button" disabled={busy} onClick={onCancel}>返回调整</button><button className="hx-btn primary" type="button" disabled={busy} onClick={onConfirm}>仍然继续</button></div></footer>
  </div>;
}
