import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  ENTITY_TYPES, REQUESTED_SCOPES, createExecutionCalendarEvent, createExecutionMemo,
  createExecutionProject, createExecutionRecurrenceRule, createExecutionReminder,
  createExecutionSubtask, createExecutionTask, createExecutionTaskDependency,
  createExecutionWaitingItem, createMemoConversionLinks, createWaitingConversionLinks,
  dependencyCreatesCycle, executionTaskDate, materializeCalendarOccurrences,
  materializeTaskOccurrences, moveCalendarOccurrence, recurrenceLabel, reminderIsDue,
  taskBlockers, taskIsInbox, taskMatchesToday,
} from "../web-client/src/core";
import { NAV_GROUPS, ROUTES } from "../web-client/src/navigation";

const read = (path: string) => readFileSync(new URL(`../web-client/src/${path}`, import.meta.url), "utf8");

test("browser cloud registry includes the existing execution domain", () => {
  for (const entityType of [
    "execution.project", "execution.recurrence_rule", "execution.task", "execution.task_dependency",
    "execution.task_occurrence", "execution.waiting_item", "execution.calendar_event",
    "execution.calendar_occurrence", "execution.memo", "execution.reminder",
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

test("memo and waiting conversions preserve bidirectional lineage", () => {
  const memo = createExecutionMemo("u", "d", "整理会议结论");
  const task = createExecutionTask("u", "d", { title: "整理会议结论" });
  const [forward, reverse] = createMemoConversionLinks("u", "d", memo.meta.id, "task", task.meta.id);
  assert.equal(forward.sourceType, "memo");
  assert.equal(forward.relationType, "converted_to");
  assert.equal(reverse.sourceType, "task");
  assert.equal(reverse.relationType, "derived_from");

  const waiting = createExecutionWaitingItem("u", "d", { title: "等待回复", waitingFor: "Alice" });
  const [waitingForward, waitingReverse] = createWaitingConversionLinks("u", "d", waiting.meta.id, task.meta.id);
  assert.equal(waitingForward.sourceType, "waiting_item");
  assert.equal(waitingForward.targetType, "task");
  assert.equal(waitingReverse.sourceType, "task");
  assert.equal(waitingReverse.targetType, "waiting_item");
});

test("waiting reminders subtasks and dependencies follow desktop execution semantics", () => {
  const parent = createExecutionTask("u", "d", { title: "完成报告", projectId: "project-1" });
  const child = createExecutionSubtask("u", "d", parent, { title: "整理数据" });
  assert.equal(child.parentTaskId, parent.meta.id);
  assert.equal(child.projectId, "project-1");

  const prerequisite = createExecutionTask("u", "d", { title: "拿到数据" });
  const dependency = createExecutionTaskDependency("u", "d", parent.meta.id, prerequisite.meta.id);
  assert.equal(dependency.dependencyType, "finish_before_start");
  assert.equal(taskBlockers(parent.meta.id, [parent, prerequisite], [dependency]).length, 1);
  assert.equal(dependencyCreatesCycle(prerequisite.meta.id, parent.meta.id, [dependency]), true);
  assert.equal(dependencyCreatesCycle(parent.meta.id, child.meta.id, [dependency]), false);

  const waiting = createExecutionWaitingItem("u", "d", { title: "等待审批", waitingFor: "主管", sourceTaskId: parent.meta.id });
  assert.equal(waiting.status, "open");
  assert.equal(waiting.sourceTaskId, parent.meta.id);

  const reminder = createExecutionReminder("u", "d", "waiting_item", waiting.meta.id, "2026-08-14T09:00:00Z");
  assert.equal(reminder.status, "scheduled");
  assert.equal(reminderIsDue(reminder, new Date("2026-08-14T09:00:01Z")), true);
});

test("calendar recurrence materializes independent occurrences with per-instance moves", () => {
  const event = createExecutionCalendarEvent("u", "d", {
    title: "深度工作",
    startAt: "2026-08-14T10:00:00Z",
    endAt: "2026-08-14T11:00:00Z",
  });
  const rule = createExecutionRecurrenceRule("u", "d", { frequency: "daily", maxOccurrences: 2 });
  const recurringEvent = { ...event, recurrenceRuleId: rule.meta.id };
  const occurrences = materializeCalendarOccurrences("u", "d", recurringEvent, rule, [], "2026-08-14", 7);
  assert.equal(occurrences.length, 2);
  assert.equal(occurrences[0]?.eventId, event.meta.id);
  assert.equal(occurrences[0]?.status, "scheduled");

  const moved = moveCalendarOccurrence(occurrences[0]!, "2026-08-20", "09:30", 45);
  assert.equal(moved.occurrenceKey, occurrences[0]?.occurrenceKey, "moving one instance must preserve its recurrence identity");
  assert.match(String(moved.startAt), /2026-08-20/);
  assert.equal(moved.status, "scheduled");

  const more = materializeCalendarOccurrences("u", "d", recurringEvent, rule, occurrences, "2026-08-14", 7);
  assert.equal(more.length, 0);
});

test("execution stays one global destination while control center remains an internal subroute", () => {
  assert.ok(ROUTES.has("/execution"));
  assert.ok(ROUTES.has("/execution/control"));
  const exposedRoutes = NAV_GROUPS.flatMap((group) => group.items).map((item) => item.route);
  assert.ok(exposedRoutes.includes("/execution"));
  assert.equal(exposedRoutes.filter((route) => route === "/execution").length, 1);
  assert.equal(exposedRoutes.includes("/execution/control"), false);
});

test("execution workspace exposes recurrence conversion review timeboxing and advanced controls", () => {
  const execution = read("pages/ExecutionPage.tsx");
  const calendar = read("pages/ExecutionCalendarPage.tsx");
  const control = read("pages/ExecutionControlPage.tsx");
  const routes = read("components/RouteView.tsx");
  const dashboard = read("pages/DashboardPage.tsx");
  const search = read("cloud/search.ts");
  assert.match(execution, /QUICK CAPTURE/);
  assert.match(execution, /RECURRENCE RULE/);
  assert.match(execution, /MEMO → CALENDAR/);
  assert.match(execution, /近 7 日计划/);
  assert.match(calendar, /TASK → TIMEBOX/);
  assert.match(calendar, /execution\.calendar_occurrence/);
  assert.match(calendar, /execution\/control/);
  assert.match(control, /WAITING/);
  assert.match(control, /REMINDERS/);
  assert.match(control, /TASK STRUCTURE/);
  assert.match(control, /OCCURRENCE EXCEPTIONS/);
  assert.match(routes, /ExecutionControlPage/);
  assert.match(dashboard, /execution\.task_occurrence/);
  assert.match(search, /execution\.project/);
  assert.match(search, /execution\.memo/);
});
