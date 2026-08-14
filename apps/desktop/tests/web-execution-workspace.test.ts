import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  ENTITY_TYPES, REQUESTED_SCOPES, createExecutionCalendarEvent, createExecutionMemo,
  createExecutionProject, createExecutionRecurrenceRule, createExecutionTask,
  createMemoConversionLinks, executionTaskDate, materializeTaskOccurrences,
  recurrenceLabel, taskIsInbox, taskMatchesToday,
} from "../web-client/src/core";
import { NAV_GROUPS, ROUTES } from "../web-client/src/navigation";

const read = (path: string) => readFileSync(new URL(`../web-client/src/${path}`, import.meta.url), "utf8");

test("browser cloud registry includes the existing execution domain", () => {
  for (const entityType of [
    "execution.project", "execution.recurrence_rule", "execution.task", "execution.task_occurrence",
    "execution.waiting_item", "execution.calendar_event", "execution.memo", "execution.reminder",
    "execution.completion_result", "execution.entity_link",
  ]) assert.ok(ENTITY_TYPES.includes(entityType as never), entityType);
  assert.ok(REQUESTED_SCOPES.includes("execution:read" as never));
  assert.ok(REQUESTED_SCOPES.includes("execution:write" as never));
});

test("execution factories keep task plan memo and calendar semantics separate", () => {
  const project = createExecutionProject("u", "d", { name: "完成论文", description: "按阶段推进" });
  const task = createExecutionTask("u", "d", { title: "补实验", projectId: project.meta.id, priority: "high", dueAt: "2026-08-14T15:59:00.000Z" });
  const inboxTask = createExecutionTask("u", "d", { title: "买牛奶", context: "inbox" });
  const memo = createExecutionMemo("u", "d", "SMF 图需要重画", "inbox");
  const calendar = createExecutionCalendarEvent("u", "d", { title: "深度工作", startAt: "2026-08-14T10:00:00.000Z", endAt: "2026-08-14T11:00:00.000Z", sourceTaskId: task.meta.id });

  assert.equal(project.name, "完成论文");
  assert.equal(task.projectId, project.meta.id);
  assert.equal(task.priority, "high");
  assert.equal(taskIsInbox(task), false);
  assert.equal(taskIsInbox(inboxTask), true);
  assert.equal(memo.plainText, "SMF 图需要重画");
  assert.equal(calendar.sourceTaskId, task.meta.id);
  assert.equal(calendar.status, "scheduled");
  const day = executionTaskDate(task);
  assert.ok(day);
  assert.equal(taskMatchesToday(task, day!), true);
});

test("recurring tasks materialize independent occurrences instead of resetting the parent task", () => {
  const task = createExecutionTask("u", "d", { title: "练琴", dueAt: "2026-08-14T12:00:00.000Z", estimatedMinutes: 30 });
  const anchor = executionTaskDate(task)!;
  const rule = createExecutionRecurrenceRule("u", "d", { frequency: "daily", intervalValue: 1, maxOccurrences: 3 });
  const occurrences = materializeTaskOccurrences("u", "d", { ...task, recurrenceRuleId: rule.meta.id }, rule, [], anchor, 7);

  assert.equal(recurrenceLabel(rule), "每天");
  assert.equal(occurrences.length, 3);
  assert.equal(occurrences[0]?.taskId, task.meta.id);
  assert.equal(occurrences[0]?.status, "pending");
  assert.equal(occurrences[0]?.occurrenceKey, anchor);
  assert.notEqual(occurrences[0]?.meta.id, task.meta.id);

  const more = materializeTaskOccurrences("u", "d", task, rule, occurrences, anchor, 7);
  assert.equal(more.length, 0, "materialization must be idempotent and respect maxOccurrences");
});

test("memo conversion creates the same bidirectional lineage used by the desktop execution domain", () => {
  const memo = createExecutionMemo("u", "d", "整理会议结论");
  const task = createExecutionTask("u", "d", { title: "整理会议结论" });
  const [forward, reverse] = createMemoConversionLinks("u", "d", memo.meta.id, "task", task.meta.id);

  assert.equal(forward.sourceType, "memo");
  assert.equal(forward.sourceId, memo.meta.id);
  assert.equal(forward.relationType, "converted_to");
  assert.equal(forward.targetType, "task");
  assert.equal(reverse.sourceType, "task");
  assert.equal(reverse.relationType, "derived_from");
  assert.equal(reverse.targetType, "memo");
  assert.equal(reverse.targetId, memo.meta.id);
});

test("execution is a first-class browser destination without fragmenting global navigation", () => {
  assert.ok(ROUTES.has("/execution"));
  const exposedRoutes = NAV_GROUPS.flatMap((group) => group.items).map((item) => item.route);
  assert.ok(exposedRoutes.includes("/execution"));
  assert.equal(exposedRoutes.filter((route) => route === "/execution").length, 1);
});

test("execution workspace exposes capture recurrence conversion review and timeboxing", () => {
  const execution = read("pages/ExecutionPage.tsx");
  const calendar = read("pages/ExecutionCalendarPage.tsx");
  const routes = read("components/RouteView.tsx");
  const dashboard = read("pages/DashboardPage.tsx");
  const search = read("cloud/search.ts");
  assert.match(execution, /QUICK CAPTURE/);
  assert.match(execution, /收件箱/);
  assert.match(execution, /计划与项目/);
  assert.match(execution, /备忘时间流/);
  assert.match(execution, /RECURRENCE RULE/);
  assert.match(execution, /MEMO → CALENDAR/);
  assert.match(execution, /近 7 日计划/);
  assert.match(execution, /execution\.completion_result/);
  assert.match(calendar, /TASK → TIMEBOX/);
  assert.match(calendar, /execution\.calendar_event/);
  assert.match(routes, /ExecutionCalendarPage/);
  assert.match(dashboard, /execution\.task_occurrence/);
  assert.match(search, /execution\.project/);
  assert.match(search, /execution\.memo/);
});
