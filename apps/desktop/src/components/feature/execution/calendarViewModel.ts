import type { CalendarEvent, CalendarTimingInput } from "@/src/services/executionApi";

export type CalendarView = "month" | "week" | "day";

export type CalendarRange = {
  start: Date;
  endExclusive: Date;
  timedStart: string;
  timedEnd: string;
  localStartDate: string;
  localEndDate: string;
};

export function localDateKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function startOfLocalDay(value: Date): Date {
  return new Date(value.getFullYear(), value.getMonth(), value.getDate());
}

export function addLocalDays(value: Date, amount: number): Date {
  const next = new Date(value);
  next.setDate(next.getDate() + amount);
  return startOfLocalDay(next);
}

export function startOfWeekMonday(value: Date): Date {
  const start = startOfLocalDay(value);
  const day = start.getDay();
  const distance = day === 0 ? 6 : day - 1;
  return addLocalDays(start, -distance);
}

function rangeFromBounds(start: Date, endExclusive: Date): CalendarRange {
  return {
    start,
    endExclusive,
    timedStart: start.toISOString(),
    timedEnd: endExclusive.toISOString(),
    localStartDate: localDateKey(start),
    localEndDate: localDateKey(addLocalDays(endExclusive, -1)),
  };
}

export function calendarRange(view: CalendarView, anchor: Date): CalendarRange {
  if (view === "day") {
    const start = startOfLocalDay(anchor);
    return rangeFromBounds(start, addLocalDays(start, 1));
  }

  if (view === "week") {
    const start = startOfWeekMonday(anchor);
    return rangeFromBounds(start, addLocalDays(start, 7));
  }

  const firstOfMonth = new Date(anchor.getFullYear(), anchor.getMonth(), 1);
  const lastOfMonth = new Date(anchor.getFullYear(), anchor.getMonth() + 1, 0);
  const start = startOfWeekMonday(firstOfMonth);
  const endExclusive = addLocalDays(startOfWeekMonday(lastOfMonth), 7);
  return rangeFromBounds(start, endExclusive);
}

export function shiftCalendarAnchor(view: CalendarView, anchor: Date, direction: -1 | 1): Date {
  if (view === "day") return addLocalDays(anchor, direction);
  if (view === "week") return addLocalDays(anchor, direction * 7);
  return new Date(anchor.getFullYear(), anchor.getMonth() + direction, 1);
}

export function calendarPeriodLabel(view: CalendarView, anchor: Date): string {
  if (view === "month") {
    return `${anchor.getFullYear()}年${anchor.getMonth() + 1}月`;
  }
  if (view === "day") {
    return new Intl.DateTimeFormat("zh-CN", {
      year: "numeric",
      month: "long",
      day: "numeric",
      weekday: "long",
    }).format(anchor);
  }
  const start = startOfWeekMonday(anchor);
  const end = addLocalDays(start, 6);
  const left = `${start.getMonth() + 1}月${start.getDate()}日`;
  const right = `${end.getMonth() + 1}月${end.getDate()}日`;
  return `${start.getFullYear()}年 ${left} – ${right}`;
}

export function enumerateCalendarDays(range: CalendarRange): Date[] {
  const days: Date[] = [];
  for (let cursor = range.start; cursor < range.endExclusive; cursor = addLocalDays(cursor, 1)) {
    days.push(cursor);
  }
  return days;
}

export function eventOverlapsDay(event: CalendarEvent, day: Date): boolean {
  if (event.status !== "scheduled") return false;
  const dayStart = startOfLocalDay(day);
  const dayEnd = addLocalDays(dayStart, 1);
  if (event.isAllDay) {
    const key = localDateKey(dayStart);
    return Boolean(event.startLocalDate && event.endLocalDate && event.startLocalDate <= key && key <= event.endLocalDate);
  }
  if (!event.startAt || !event.endAt) return false;
  const start = new Date(event.startAt);
  const end = new Date(event.endAt);
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) return false;
  return start < dayEnd && dayStart < end;
}

export function eventsForDay(events: CalendarEvent[], day: Date): CalendarEvent[] {
  return events
    .filter((event) => eventOverlapsDay(event, day))
    .sort((left, right) => {
      if (left.isAllDay !== right.isAllDay) return left.isAllDay ? -1 : 1;
      return String(left.startAt || left.startLocalDate).localeCompare(String(right.startAt || right.startLocalDate));
    });
}

export function timedEventPlacement(event: CalendarEvent, day: Date): { topMinutes: number; durationMinutes: number } | null {
  if (event.isAllDay || !event.startAt || !event.endAt || !eventOverlapsDay(event, day)) return null;
  const dayStart = startOfLocalDay(day);
  const dayEnd = addLocalDays(dayStart, 1);
  const rawStart = new Date(event.startAt);
  const rawEnd = new Date(event.endAt);
  if (Number.isNaN(rawStart.getTime()) || Number.isNaN(rawEnd.getTime())) return null;
  const start = rawStart < dayStart ? dayStart : rawStart;
  const end = rawEnd > dayEnd ? dayEnd : rawEnd;
  const topMinutes = start.getHours() * 60 + start.getMinutes();
  const endMinutes = end >= dayEnd ? 24 * 60 : end.getHours() * 60 + end.getMinutes();
  return { topMinutes, durationMinutes: Math.max(30, endMinutes - topMinutes) };
}

export function snapCalendarMinutes(value: number, step = 15): number {
  const safeStep = Math.max(1, step);
  const snapped = Math.round(value / safeStep) * safeStep;
  return Math.min(24 * 60 - 1, Math.max(0, snapped));
}

export function moveTimedEventToSlot(event: CalendarEvent, day: Date, minutes: number): CalendarTimingInput | null {
  if (event.isAllDay || !event.startAt || !event.endAt) return null;
  const currentStart = new Date(event.startAt);
  const currentEnd = new Date(event.endAt);
  if (Number.isNaN(currentStart.getTime()) || Number.isNaN(currentEnd.getTime()) || currentEnd <= currentStart) return null;
  const duration = currentEnd.getTime() - currentStart.getTime();
  const snappedMinutes = snapCalendarMinutes(minutes);
  const targetStart = new Date(day.getFullYear(), day.getMonth(), day.getDate(), Math.floor(snappedMinutes / 60), snappedMinutes % 60, 0, 0);
  const targetEnd = new Date(targetStart.getTime() + duration);
  return {
    isAllDay: false,
    startAt: targetStart.toISOString(),
    endAt: targetEnd.toISOString(),
    startLocalDate: null,
    endLocalDate: null,
    timezone: event.timezone || Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
  };
}