import assert from "node:assert/strict";
import test from "node:test";
import { normalizeWeekdays, preserveTaskUpdateFields, waitingToTaskInput } from "../src/components/feature/execution/executionViewModel";

const task = {
  id: "t1",
  userId: "local",
  title: "Task",
  status: "todo" as const,
  priority: "normal" as const,
  actualMinutes: 42,
  scheduledStartAt: "2026-08-09T02:00:00.000Z",
  scheduledEndAt: "2026-08-09T03:00:00.000Z",
  timezone: "Asia/Shanghai",
  version: 1,
  createdAt: "2026-08-09T00:00:00Z",
  updatedAt: "2026-08-09T00:00:00Z",
};

test("task edit preserves fields not exposed by the basic editor", () => {
  const result = preserveTaskUpdateFields(task, { title: "Renamed", priority: "high" });
  assert.equal(result.actualMinutes, 42);
  assert.equal(result.scheduledStartAt, task.scheduledStartAt);
  assert.equal(result.scheduledEndAt, task.scheduledEndAt);
  assert.equal(result.timezone, "Asia/Shanghai");
});

test("waiting conversion resolves the source and carries expected time", () => {
  const result = waitingToTaskInput({
    id: "w1",
    userId: "local",
    title: "Wait for Alice",
    status: "open",
    waitingFor: "Alice",
    expectedAt: "2026-08-10T00:00:00Z",
    version: 1,
    createdAt: "2026-08-09T00:00:00Z",
    updatedAt: "2026-08-09T00:00:00Z",
  });
  assert.equal(result.resolveSource, true);
  assert.equal(result.dueAt, "2026-08-10T00:00:00Z");
  assert.equal(result.context, "等待：Alice");
});

test("weekday normalization removes duplicates and invalid values", () => {
  assert.deepEqual(normalizeWeekdays([7, 5, 1, 1, 9, -1, 0]), [1, 5, 7]);
});
