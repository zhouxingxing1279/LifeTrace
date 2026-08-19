import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const repo = (path: string) => readFileSync(new URL(`../../../${path}`, import.meta.url), "utf8");

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
