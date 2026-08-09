import assert from "node:assert/strict";
import test from "node:test";
import type { CalendarEvent } from "../src/services/executionApi";
import {
  calendarRange,
  enumerateCalendarDays,
  eventOverlapsDay,
  eventsForDay,
  localDateKey,
  shiftCalendarAnchor,
  timedEventPlacement,
} from "../src/components/feature/execution/calendarViewModel";

function event(overrides: Partial<CalendarEvent>): CalendarEvent {
  return {
    id: "event-1",
    userId: "local",
    title: "Calendar test",
    isAllDay: false,
    status: "scheduled",
    version: 1,
    createdAt: new Date(2026, 7, 9, 8).toISOString(),
    updatedAt: new Date(2026, 7, 9, 8).toISOString(),
    ...overrides,
  };
}

test("week range starts on Monday and contains seven local days", () => {
  const range = calendarRange("week", new Date(2026, 7, 9, 12));
  assert.equal(localDateKey(range.start), "2026-08-03");
  assert.equal(range.localEndDate, "2026-08-09");
  assert.equal(enumerateCalendarDays(range).length, 7);
});

test("month range covers complete Monday-to-Sunday grid", () => {
  const range = calendarRange("month", new Date(2026, 7, 9, 12));
  assert.equal(localDateKey(range.start), "2026-07-27");
  assert.equal(range.localEndDate, "2026-09-06");
  assert.equal(enumerateCalendarDays(range).length, 42);
});

test("calendar anchor navigation follows active view granularity", () => {
  const anchor = new Date(2026, 7, 9, 12);
  assert.equal(localDateKey(shiftCalendarAnchor("day", anchor, 1)), "2026-08-10");
  assert.equal(localDateKey(shiftCalendarAnchor("week", anchor, -1)), "2026-08-02");
  assert.equal(localDateKey(shiftCalendarAnchor("month", anchor, 1)), "2026-09-01");
});

test("all-day events preserve local-date semantics across multiple days", () => {
  const allDay = event({
    id: "all-day",
    isAllDay: true,
    startLocalDate: "2026-08-08",
    endLocalDate: "2026-08-10",
    startAt: null,
    endAt: null,
  });
  assert.equal(eventOverlapsDay(allDay, new Date(2026, 7, 8, 12)), true);
  assert.equal(eventOverlapsDay(allDay, new Date(2026, 7, 10, 12)), true);
  assert.equal(eventOverlapsDay(allDay, new Date(2026, 7, 11, 12)), false);
});

test("timed placement uses local wall-clock minutes and minimum duration", () => {
  const timed = event({
    startAt: new Date(2026, 7, 9, 9, 30).toISOString(),
    endAt: new Date(2026, 7, 9, 10, 45).toISOString(),
  });
  assert.deepEqual(timedEventPlacement(timed, new Date(2026, 7, 9, 12)), {
    topMinutes: 570,
    durationMinutes: 75,
  });
});

test("eventsForDay excludes cancelled events and sorts all-day first", () => {
  const day = new Date(2026, 7, 9, 12);
  const timed = event({ id: "timed", startAt: new Date(2026, 7, 9, 8).toISOString(), endAt: new Date(2026, 7, 9, 9).toISOString() });
  const allDay = event({ id: "all", isAllDay: true, startLocalDate: "2026-08-09", endLocalDate: "2026-08-09", startAt: null, endAt: null });
  const cancelled = event({ id: "cancelled", status: "cancelled", startAt: new Date(2026, 7, 9, 7).toISOString(), endAt: new Date(2026, 7, 9, 8).toISOString() });
  assert.deepEqual(eventsForDay([timed, cancelled, allDay], day).map((item) => item.id), ["all", "timed"]);
});
