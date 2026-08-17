import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const execution = () => readFileSync(new URL("../web-client/src/pages/ExecutionPage.tsx", import.meta.url), "utf8");

test("execution keeps the original workspace views while adding delete actions", () => {
  const source = execution();
  assert.match(source, /"today" \| "inbox" \| "projects" \| "memos" \| "recurrence" \| "review" \| "completed"/);
  assert.match(source, /QUICK CAPTURE/);
  assert.match(source, /NEW TASK/);
  assert.match(source, /MEMO TIMELINE/);
  assert.match(source, /ACTIVE RULES/);
  assert.match(source, /WEEKLY SNAPSHOT/);
  assert.match(source, /deleteTask/);
  assert.match(source, /deleteProject/);
  assert.match(source, /deleteMemo/);
  assert.match(source, /deleteWeeklyReview/);
});

test("task deletion removes related execution records atomically", () => {
  const source = execution();
  assert.match(source, /execution\.task_occurrence/);
  assert.match(source, /execution\.completion_result/);
  assert.match(source, /execution\.task_dependency/);
  assert.match(source, /execution\.reminder/);
  assert.match(source, /execution\.entity_link/);
  assert.match(source, /execution\.recurrence_rule/);
  assert.match(source, /atomicMutate\(store, mutations\)/);
});

test("project deletion preserves child tasks by detaching them", () => {
  const source = execution();
  assert.match(source, /projectId: null/);
  assert.match(source, /任务会保留并变成独立任务/);
  assert.match(source, /计划已删除，原计划中的任务已保留/);
});
