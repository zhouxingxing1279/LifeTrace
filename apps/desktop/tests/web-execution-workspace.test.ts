import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  ENTITY_TYPES, REQUESTED_SCOPES, atomicMutate, createExecutionCalendarEvent,
  createExecutionGoal, createExecutionMemo, createExecutionProject, createExecutionRecurrenceRule,
  createExecutionReminder, createExecutionSubtask, createExecutionTask,
  createExecutionTaskDependency, createExecutionWaitingItem, createMemoConversionLinks,
  createWaitingConversionLinks, dependencyCreatesCycle, executionTaskDate, goalProjectProgress,
  materializeCalendarOccurrences, materializeTaskOccurrences, moveCalendarOccurrence,
  recurrenceLabel, reminderIsDue, taskBlockers, taskIsInbox, taskMatchesToday,
  type JsonEntity, type SyncChange,
} from "../web-client/src/core";
import { NAV_GROUPS, ROUTES } from "../web-client/src/navigation";

const read = (path: string) => readFileSync(new URL(`../web-client/src/${path}`, import.meta.url), "utf8");

test("browser cloud registry includes the shared execution domain including goals", () => {
  for (const entityType of [
    "execution.goal", "execution.project", "execution.recurrence_rule", "execution.task", "execution.task_dependency",
    "execution.task_occurrence", "execution.waiting_item", "execution.calendar_event",
    "execution.calendar_occurrence", "execution.memo", "execution.reminder",
    "execution.completion_result", "execution.entity_link",
  ]) assert.ok(ENTITY_TYPES.includes(entityType as never), entityType);
  assert.ok(REQUESTED_SCOPES.includes("execution:read" as never));
  assert.ok(REQUESTED_SCOPES.includes("execution:write" as never));
});

test("goals measure progress through projects and tasks instead of duplicating todo state", () => {
  const goal = createExecutionGoal("u", "d", { name: "完成毕业论文" });
  const project = { ...createExecutionProject("u", "d", { name: "实验章节" }), goalId: goal.meta.id };
  const taskA = createExecutionTask("u", "d", { title: "补实验", projectId: project.meta.id });
  const taskB = { ...createExecutionTask("u", "d", { title: "画图", projectId: project.meta.id }), status: "done" };
  const progress = goalProjectProgress(goal.meta.id, [project], [taskA, taskB]);
  assert.equal(goal.status, "active");
  assert.equal(progress.projects, 1);
  assert.equal(progress.tasks, 2);
  assert.equal(progress.completedTasks, 1);
  assert.equal(progress.rate, 50);
});

test("atomic mixed-entity writes send one non-null group id and publish local state only after success", async () => {
  const goal = createExecutionGoal("u", "d", { name: "目标" });
  const project = { ...createExecutionProject("u", "d", { name: "计划" }), goalId: goal.meta.id };
  const groups: Array<string | null> = [];
  const stored: string[] = [];
  const state = { cursor: null, entities: {}, conflicts: [], lastLoadedAt: null };
  let index = 0;
  const fakeStore = {
    state,
    prepareUpsert(entityType: string, entity: JsonEntity) {
      index += 1;
      const change: SyncChange = {
        changeId: `c${index}`, entityType: entityType as never, entityId: entity.meta.id,
        operation: "upsert", baseServerVersion: "0", entitySchemaVersion: 1,
        clientModifiedAt: new Date().toISOString(), payload: entity, atomicGroupId: null, dependencies: [],
      };
      return { entity: structuredClone(entity), change };
    },
    prepareDelete() { throw new Error("unused"); },
    async push(changes: SyncChange[]) {
      groups.push(...changes.map((change) => change.atomicGroupId));
      return changes.map((change) => ({ status: "accepted", changeId: change.changeId, serverVersion: "1" }));
    },
    put(_entityType: string, entity: JsonEntity) { stored.push(entity.meta.id); },
    remove() {},
    snapshot() { return structuredClone(state); },
  };
  await atomicMutate(fakeStore as never, [
    { operation: "upsert", entityType: "execution.goal", entity: goal },
    { operation: "upsert", entityType: "execution.project", entity: project },
  ]);
  assert.equal(groups.length, 2);
  assert.ok(groups[0]);
  assert.equal(groups[0], groups[1]);
  assert.deepEqual(stored, [goal.meta.id, project.meta.id]);
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

test("execution stays one global destination while goals and control remain internal subroutes", () => {
  assert.ok(ROUTES.has("/execution"));
  assert.ok(ROUTES.has("/execution/goals"));
  assert.ok(ROUTES.has("/execution/control"));
  const exposedRoutes = NAV_GROUPS.flatMap((group) => group.items).map((item) => item.route);
  assert.ok(exposedRoutes.includes("/execution"));
  assert.equal(exposedRoutes.filter((route) => route === "/execution").length, 1);
  assert.equal(exposedRoutes.includes("/execution/goals"), false);
  assert.equal(exposedRoutes.includes("/execution/control"), false);
});

test("execution workspace keeps the daily task flow simple while advanced capabilities stay available", () => {
  const execution = read("pages/ExecutionPage.tsx");
  const hub = read("pages/ExecutionHubPage.tsx");
  const goals = read("pages/ExecutionGoalsPage.tsx");
  const today = read("pages/DependencyAwareDashboardPage.tsx");
  const calendar = read("pages/ExecutionCalendarPage.tsx");
  const control = read("pages/ExecutionControlPage.tsx");
  const routes = read("components/RouteView.tsx");
  const atomic = read("cloud/atomic.ts");
  const search = read("cloud/search.ts");

  assert.match(execution, /type TaskView = "today" \| "todo" \| "completed"/);
  assert.match(execution, /QUICK ADD/);
  assert.match(execution, /计划（可选）/);
  assert.match(execution, /window\.confirm/);
  assert.match(execution, /store\.delete\("execution\.task"/);
  assert.match(execution, /execution\.task_dependency/);
  assert.doesNotMatch(execution, /createExecutionMemo/);
  assert.doesNotMatch(execution, /createExecutionWeeklyReview/);
  assert.doesNotMatch(execution, /createExecutionRecurrenceRule/);

  assert.match(hub, /execution\/goals/);
  assert.match(goals, /Goal → Project → Task/);
  assert.match(goals, /atomicMutate/);
  assert.match(today, /DEPENDENCY-AWARE TODAY/);
  assert.match(today, /dependsOnTaskId/);
  assert.match(atomic, /atomicGroupId/);
  assert.match(atomic, /internals\.push/);
  assert.match(calendar, /execution\.calendar_occurrence/);
  assert.match(control, /WAITING/);
  assert.match(control, /REMINDERS/);
  assert.match(control, /TASK STRUCTURE/);
  assert.match(routes, /ExecutionGoalsPage/);
  assert.match(routes, /DependencyAwareDashboardPage/);
  assert.match(search, /execution\.goal/);
});