import assert from "node:assert/strict";
import test from "node:test";
import { stateFromSnapshot } from "../src/v2/api/cloud";

const meta = (id: string) => ({ id, userId: "user-1", createdAt: "2026-08-22T00:00:00Z", updatedAt: "2026-08-22T00:00:00Z", deletedAt: null, localVersion: 1, serverVersion: "1", modifiedByDevice: null });

test("cloud snapshot maps core sync entities into the shared V2 state", () => {
  const state = stateFromSnapshot([
    { entityType: "execution.task", entityId: "task-1", serverVersion: "1", payload: { meta: meta("task-1"), title: "Ship V2", dueDate: "2026-08-22", project: "LifeTrace", priority: "high", completed: false } },
    { entityType: "habit.activity", entityId: "habit-1", serverVersion: "2", payload: { meta: meta("habit-1"), name: "Read", targetDays: [], targetDayCount: 7, streak: 2, isArchived: false } },
    { entityType: "habit.log", entityId: "log-1", serverVersion: "3", payload: { meta: meta("log-1"), activityId: "habit-1", logDate: "2026-08-22", status: "completed" } },
    { entityType: "workout.workout", entityId: "workout-1", serverVersion: "4", payload: { meta: meta("workout-1"), name: "Push", localDate: "2026-08-22", durationSeconds: 3600, volumeKg: 4200 } },
    { entityType: "finance.transaction", entityId: "tx-1", serverVersion: "5", payload: { meta: meta("tx-1"), transactionType: "expense", amountCents: 3590, localDate: "2026-08-22", item: "Lunch", categoryName: "Food", accountName: "Alipay" } },
    { entityType: "note.note", entityId: "note-1", serverVersion: "6", payload: { meta: meta("note-1"), title: "Idea", contentMarkdown: "Build it", isPinned: true, isArchived: false } },
    { entityType: "english.learning_record", entityId: "read-1", serverVersion: "7", payload: { meta: meta("read-1"), title: "Article", source: "Manual", progress: 80, summary: "Useful", completionStatus: "reading", readingStatus: "reading", highlights: ["phrase"] } },
    { entityType: "review.daily", entityId: "review-1", serverVersion: "8", payload: { meta: meta("review-1"), reviewDate: "2026-08-22", bestThing: "Tests", problem: "None", tomorrowPriority: "Release" } },
    { entityType: "user.preference", entityId: "pref-1", serverVersion: "9", payload: { meta: meta("pref-1"), preferenceKey: "frontend.v2.settings", value: { theme: "dark", reducedMotion: true, accent: "blue" } } }
  ]);

  assert.equal(state.tasks[0]?.title, "Ship V2");
  assert.deepEqual(state.habits[0]?.completedDates, ["2026-08-22"]);
  assert.equal(state.workouts[0]?.durationMinutes, 60);
  assert.equal(state.transactions[0]?.title, "Lunch");
  assert.equal(state.notes[0]?.content, "Build it");
  assert.equal(state.reading[0]?.note, "Useful");
  assert.equal(state.reviews[0]?.tomorrowPriority, "Release");
  assert.equal(state.settings.theme, "dark");
  assert.equal(state.settings.reducedMotion, true);
});

test("snapshot filtering ignores archived habits and notes", () => {
  const state = stateFromSnapshot([
    { entityType: "habit.activity", entityId: "habit-archived", serverVersion: "1", payload: { meta: meta("habit-archived"), name: "Old", isArchived: true } },
    { entityType: "note.note", entityId: "note-archived", serverVersion: "2", payload: { meta: meta("note-archived"), title: "Old", isArchived: true } }
  ]);
  assert.equal(state.habits.length, 0);
  assert.equal(state.notes.length, 0);
});
