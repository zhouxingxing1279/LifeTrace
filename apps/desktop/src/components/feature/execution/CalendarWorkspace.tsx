import { useCallback, useEffect, useMemo, useState } from "react";
import { Bell, CalendarDays, ChevronLeft, ChevronRight, LoaderCircle, Plus, RefreshCw } from "lucide-react";
import {
  executionApi,
  type CalendarEvent,
  type Reminder,
} from "@/src/services/executionApi";
import {
  calendarPeriodLabel,
  calendarRange,
  enumerateCalendarDays,
  eventsForDay,
  localDateKey,
  shiftCalendarAnchor,
  timedEventPlacement,
  type CalendarView,
} from "@/src/components/feature/execution/calendarViewModel";

const weekdayLabels = ["一", "二", "三", "四", "五", "六", "日"];
const hourLabels = Array.from({ length: 24 }, (_, hour) => `${String(hour).padStart(2, "0")}:00`);
const minutePixel = 0.75;

type Props = {
  refreshToken: number;
  onCreate: () => void;
  onEdit: (event: CalendarEvent) => void;
  onReminder: (subject: { subjectType: Reminder["subjectType"]; subjectId: string; title: string }) => void;
};

function sameLocalDay(left: Date, right: Date) {
  return localDateKey(left) === localDateKey(right);
}

function eventTimeLabel(event: CalendarEvent) {
  if (event.isAllDay) return "全天";
  if (!event.startAt) return "";
  const date = new Date(event.startAt);
  if (Number.isNaN(date.getTime())) return "";
  return new Intl.DateTimeFormat("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }).format(date);
}

function EventBlock({ event, day, onEdit, onReminder }: { event: CalendarEvent; day: Date; onEdit: (event: CalendarEvent) => void; onReminder: Props["onReminder"] }) {
  const placement = timedEventPlacement(event, day);
  if (!placement) return null;
  const style = {
    top: `${placement.topMinutes * minutePixel}px`,
    minHeight: `${placement.durationMinutes * minutePixel}px`,
  };
  return <div
    className="lt-calendar-time-event"
    style={style}
    role="button"
    tabIndex={0}
    aria-label={`${eventTimeLabel(event)} ${event.title}`}
    onClick={() => onEdit(event)}
    onKeyDown={(keyEvent) => { if (keyEvent.key === "Enter" || keyEvent.key === " ") { keyEvent.preventDefault(); onEdit(event); } }}
  >
    <strong>{event.title}</strong>
    <span>{eventTimeLabel(event)}</span>
    <button
      type="button"
      title="管理提醒"
      aria-label={`管理 ${event.title} 的提醒`}
      onClick={(clickEvent) => {
        clickEvent.stopPropagation();
        onReminder({ subjectType: "calendar_event", subjectId: event.id, title: event.title });
      }}
    ><Bell aria-hidden="true"/></button>
  </div>;
}

export default function CalendarWorkspace({ refreshToken, onCreate, onEdit, onReminder }: Props) {
  const [view, setView] = useState<CalendarView>("week");
  const [anchor, setAnchor] = useState(() => new Date());
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const range = useMemo(() => calendarRange(view, anchor), [view, anchor]);
  const days = useMemo(() => enumerateCalendarDays(range), [range]);

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const next = await executionApi.calendar.list({
        timedStart: range.timedStart,
        timedEnd: range.timedEnd,
        localStartDate: range.localStartDate,
        localEndDate: range.localEndDate,
      });
      setEvents(next);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "日历加载失败");
    } finally {
      setLoading(false);
    }
  }, [range]);

  useEffect(() => { void load(); }, [load, refreshToken]);

  const move = (direction: -1 | 1) => setAnchor((current) => shiftCalendarAnchor(view, current, direction));
  const today = new Date();

  const renderMonth = () => <div className="lt-calendar-month">
    <div className="lt-calendar-weekdays">{weekdayLabels.map((label) => <span key={label}>周{label}</span>)}</div>
    <div className="lt-calendar-month-grid">{days.map((day) => {
      const dayEvents = eventsForDay(events, day);
      const inMonth = day.getMonth() === anchor.getMonth();
      const isToday = sameLocalDay(day, today);
      return <section key={localDateKey(day)} className={`${inMonth ? "" : "outside"} ${isToday ? "today" : ""}`}>
        <header><span>{day.getDate()}</span>{isToday ? <small>今天</small> : null}</header>
        <div>{dayEvents.slice(0, 4).map((event) => <button key={event.id} type="button" className={event.isAllDay ? "all-day" : "timed"} onClick={() => onEdit(event)} title={event.title}>
          <time>{eventTimeLabel(event)}</time><strong>{event.title}</strong>
        </button>)}{dayEvents.length > 4 ? <span className="lt-calendar-more">还有 {dayEvents.length - 4} 项</span> : null}</div>
      </section>;
    })}</div>
  </div>;

  const timelineDays = view === "day" ? days.slice(0, 1) : days.slice(0, 7);
  const renderTimeline = () => <div className={`lt-calendar-timeline ${view}`}>
    <div className="lt-calendar-day-heads">
      <span className="gutter"/>
      {timelineDays.map((day) => <div key={localDateKey(day)} className={sameLocalDay(day, today) ? "today" : ""}><strong>{weekdayLabels[(day.getDay() + 6) % 7]}</strong><span>{day.getMonth() + 1}/{day.getDate()}</span></div>)}
    </div>
    <div className="lt-calendar-all-day">
      <span className="gutter">全天</span>
      {timelineDays.map((day) => <div key={localDateKey(day)}>{eventsForDay(events, day).filter((event) => event.isAllDay).map((event) => <button key={event.id} type="button" onClick={() => onEdit(event)}>{event.title}</button>)}</div>)}
    </div>
    <div className="lt-calendar-scroll">
      <div className="lt-calendar-hours">
        {hourLabels.map((label) => <span key={label} style={{ top: `${Number(label.slice(0, 2)) * 60 * minutePixel}px` }}>{label}</span>)}
      </div>
      <div className="lt-calendar-time-grid" style={{ height: `${24 * 60 * minutePixel}px` }}>
        {timelineDays.map((day) => {
          const timed = eventsForDay(events, day).filter((event) => !event.isAllDay);
          const nowMinutes = today.getHours() * 60 + today.getMinutes();
          return <div key={localDateKey(day)} className={`lt-calendar-time-column ${sameLocalDay(day, today) ? "today" : ""}`}>
            {hourLabels.map((_, hour) => <i key={hour} style={{ top: `${hour * 60 * minutePixel}px` }}/>) }
            {sameLocalDay(day, today) ? <span className="lt-calendar-now" style={{ top: `${nowMinutes * minutePixel}px` }}/>: null}
            {timed.map((event) => <EventBlock key={event.id} event={event} day={day} onEdit={onEdit} onReminder={onReminder}/>) }
          </div>;
        })}
      </div>
    </div>
  </div>;

  return <div className="lt-exec-workspace lt-calendar-workspace">
    <div className="lt-calendar-toolbar">
      <div className="lt-calendar-nav">
        <button type="button" aria-label="上一段" onClick={() => move(-1)}><ChevronLeft/></button>
        <button type="button" onClick={() => setAnchor(new Date())}>今天</button>
        <button type="button" aria-label="下一段" onClick={() => move(1)}><ChevronRight/></button>
        <strong>{calendarPeriodLabel(view, anchor)}</strong>
      </div>
      <div className="lt-calendar-actions">
        <div className="lt-calendar-view-switch" role="group" aria-label="日历视图">
          {(["month", "week", "day"] as const).map((mode) => <button key={mode} type="button" className={view === mode ? "active" : ""} aria-pressed={view === mode} onClick={() => setView(mode)}>{mode === "month" ? "月" : mode === "week" ? "周" : "日"}</button>)}
        </div>
        <button type="button" title="刷新日历" aria-label="刷新日历" onClick={() => void load()}><RefreshCw className={loading ? "spin" : ""}/></button>
        <button className="hx-btn primary" type="button" onClick={onCreate}><Plus/>新建事件</button>
      </div>
    </div>
    {loading && !events.length ? <div className="lt-calendar-state"><LoaderCircle className="spin"/><span>正在读取日历…</span></div> : null}
    {error ? <div className="lt-calendar-state error"><CalendarDays/><strong>日历暂时无法读取</strong><span>{error}</span><button type="button" onClick={() => void load()}>重试</button></div> : null}
    {!error && (!loading || events.length) ? (view === "month" ? renderMonth() : renderTimeline()) : null}
  </div>;
}
