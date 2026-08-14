import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  ENTITY_TYPES,
  createExecutionWeeklyReview,
} from "../web-client/src/core";

const web = (path: string) => readFileSync(new URL(`../web-client/src/${path}`, import.meta.url), "utf8");
const repo = (path: string) => readFileSync(new URL(`../../../${path}`, import.meta.url), "utf8");

test("P4 weekly execution review is a shared sync entity", () => {
  assert.ok(ENTITY_TYPES.includes("execution.weekly_review" as never));
  const review = createExecutionWeeklyReview("user-1", "device-1", {
    weekStart: "2026-08-08",
    weekEnd: "2026-08-14",
    plannedCount: 10,
    completedCount: 8,
    completionRate: 80,
    plannedMinutes: 420,
    actualMinutes: 390,
    overdueTaskCount: 1,
    overdueOccurrenceCount: 2,
    note: "减少无意义的延期",
  });
  assert.equal(review.weekStart, "2026-08-08");
  assert.equal(review.completedCount, 8);
  assert.equal(review.completionRate, 80);
  assert.equal(review.note, "减少无意义的延期");
});

test("P4 multi-entity execution flows use atomicMutate", () => {
  const execution = web("pages/ExecutionPage.tsx");
  const calendar = web("pages/ExecutionCalendarPage.tsx");
  const control = web("pages/ExecutionControlPage.tsx");
  assert.match(execution, /execution\.completion_result/);
  assert.match(execution, /atomicMutate\(store/);
  assert.match(execution, /execution\.weekly_review/);
  assert.match(calendar, /atomicMutate\(store/);
  assert.match(control, /atomicMutate\(store/);
});

test("P4 server owns dependency enforcement and recurrence/reminder maintenance", () => {
  const guard = repo("services/cloud/src/postgres_repository/push/execution_guard.rs");
  const worker = repo("services/cloud/src/bin/execution_worker.rs");
  const migration = repo("services/cloud/migrations/0021_execution_worker.sql");
  assert.match(guard, /finish_before_start/);
  assert.match(guard, /status.*in_progress.*done/s);
  assert.match(worker, /execution_worker_leases/);
  assert.match(worker, /fire_due_reminders/);
  assert.match(worker, /materialize_task_occurrences/);
  assert.match(worker, /materialize_calendar_occurrences/);
  assert.match(worker, /deterministic_id/);
  assert.match(migration, /lease_until/);
});

test("P4 cloud image and production compose run the dedicated execution worker", () => {
  const dockerfile = repo("services/cloud/Dockerfile");
  const production = repo("deploy/cloud/docker-compose.production.yml");
  const local = repo("deploy/cloud/docker-compose.local.yml");
  assert.match(dockerfile, /--bin execution_worker/);
  assert.match(dockerfile, /release\/execution_worker \/app\/execution_worker/);
  assert.match(production, /lifetrace-execution-worker:/);
  assert.match(production, /entrypoint: \["\/app\/execution_worker"\]/);
  assert.match(local, /lifetrace-execution-worker:/);
});
