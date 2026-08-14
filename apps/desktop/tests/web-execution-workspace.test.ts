import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  ENTITY_TYPES, REQUESTED_SCOPES, createExecutionMemo, createExecutionProject,
  createExecutionTask, executionTaskDate, taskIsInbox, taskMatchesToday,
} from "../web-client/src/core";
import { NAV_GROUPS, ROUTES } from "../web-client/src/navigation";

const read = (path: string) => readFileSync(new URL(`../web-client/src/${path}`, import.meta.url), "utf8");

test("browser cloud registry includes the existing execution domain", () => {
  for (const entityType of [
    "execution.project", "execution.task", "execution.task_occurrence", "execution.waiting_item",
    "execution.calendar_event", "execution.memo", "execution.reminder", "execution.completion_result",
    "execution.entity_link",
  ]) assert.ok(ENTITY_TYPES.includes(entityType as never), entityType);
  assert.ok(REQUESTED_SCOPES.includes("execution:read" as never));
  assert.ok(REQUESTED_SCOPES.includes("execution:write" as never));
});

test("execution factories keep task plan and memo semantics separate", () => {
  const project = createExecutionProject("u", "d", { name: "完成论文", description: "按阶段推进" });
  const task = createExecutionTask("u", "d", { title: "补实验", projectId: project.meta.id, priority: "high", dueAt: "2026-08-14T15:59:00.000Z" });
  const inboxTask = createExecutionTask("u", "d", { title: "买牛奶", context: "inbox" });
  const memo = createExecutionMemo("u", "d", "SMF 图需要重画", "inbox");

  assert.equal(project.name, "完成论文");
  assert.equal(task.projectId, project.meta.id);
  assert.equal(task.priority, "high");
  assert.equal(taskIsInbox(task), false);
  assert.equal(taskIsInbox(inboxTask), true);
  assert.equal(memo.plainText, "SMF 图需要重画");
  const day = executionTaskDate(task);
  assert.ok(day);
  assert.equal(taskMatchesToday(task, day!), true);
});

test("execution is a first-class browser destination without fragmenting global navigation", () => {
  assert.ok(ROUTES.has("/execution"));
  const exposedRoutes = NAV_GROUPS.flatMap((group) => group.items).map((item) => item.route);
  assert.ok(exposedRoutes.includes("/execution"));
  assert.equal(exposedRoutes.filter((route) => route === "/execution").length, 1);
});

test("execution workspace exposes the capture to execution loop and dashboard consumes it", () => {
  const execution = read("pages/ExecutionPage.tsx");
  const dashboard = read("pages/DashboardPage.tsx");
  const search = read("cloud/search.ts");
  assert.match(execution, /QUICK CAPTURE/);
  assert.match(execution, /收件箱/);
  assert.match(execution, /计划与项目/);
  assert.match(execution, /备忘时间流/);
  assert.match(execution, /execution\.completion_result/);
  assert.match(dashboard, /execution\.task/);
  assert.match(dashboard, /execution\.memo/);
  assert.match(search, /execution\.project/);
  assert.match(search, /execution\.memo/);
});
