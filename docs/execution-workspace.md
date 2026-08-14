# LifeTrace Execution Workspace

> Status: implemented on the browser client in the `execution-workspace-web` refactor, based on the execution domain that already exists in the desktop application and cloud contract registry.
>
> Updated: 2026-08-14

## 1. Goal

LifeTrace should not model memo, todo, plan and calendar as unrelated islands. The execution workspace turns them into one capture-to-review loop:

```text
Quick Capture
    ↓
Inbox / Memo
    ↓
Plan (Project)
    ↓
Task
    ↓
Today / Schedule
    ↓
Execution
    ↓
Completion Result
    ↓
Dashboard / Review / Search
```

The browser implementation deliberately reuses the existing `execution.*` contract instead of creating a second web-only task model.

## 2. Existing execution domain

The canonical contract registry already provides these syncable entities:

- `execution.project`
- `execution.recurrence_rule`
- `execution.task`
- `execution.task_dependency`
- `execution.task_occurrence`
- `execution.waiting_item`
- `execution.calendar_event`
- `execution.calendar_occurrence`
- `execution.memo`
- `execution.memo_tag`
- `execution.memo_tag_relation`
- `execution.reminder`
- `execution.completion_result`
- `execution.entity_link`

The cloud authorization layer already exposes `execution:read` and `execution:write`. The browser now requests those scopes and includes execution entities in snapshot, pull and push operations.

## 3. Browser information architecture

The browser exposes one global destination: **计划与待办** (`/execution`). Internal views stay local to that domain so the global sidebar does not become a list of every feature.

Current local views:

1. **今天** — tasks scheduled or due today.
2. **收件箱** — unprocessed tasks captured without a concrete plan/date.
3. **计划** — projects and their task completion progress.
4. **备忘** — title-free memo timeline with pin/archive operations.
5. **已完成** — completion history backed by `execution.completion_result`.

The mobile bottom navigation also exposes the execution workspace because it is part of the daily path, not a settings/detail function.

## 4. Capture rules

### Task

A task is an actionable item. It has explicit execution state and may have project, priority, due date, schedule and time estimate.

Supported states in the current web UI:

```text
todo → in_progress → done
  └──────────────→ cancelled
```

A task created by quick capture enters the Inbox through `context = "inbox"`. Once it is scheduled or assigned to a plan it can leave that capture context.

### Memo

A memo is deliberately lighter than a note:

- no title required;
- no folder required;
- quick capture first;
- chronological timeline;
- optional pin/archive;
- may later become an actionable item.

Formal knowledge continues to belong to the Notes module. Memo is for low-friction capture, not long-form editing.

### Plan / Project

The current stable execution contract contains `execution.project`, so the first refactor uses Project as the plan container. A separate Goal entity is **not** introduced only for the browser because that would create contract drift between desktop, cloud and web.

A future Goal layer should be added only as a shared contract migration, then linked as:

```text
Goal → Project → Task → Completion Result
```

## 5. Today dashboard integration

The root dashboard is execution-first after this refactor.

`今日行动完成度` is calculated from both:

- tasks assigned to today;
- active habit items.

The dashboard now surfaces:

- today's task progress;
- task Inbox size;
- active plan count;
- memo count;
- recent task/memo activity;
- existing habits, fitness, learning, finance and review summaries.

This keeps habits and tasks separate at the data-model level while presenting them together at the daily-action level.

## 6. Search and sync

Global search includes:

- execution tasks;
- projects;
- memos;
- waiting items.

All browser execution writes go through `CloudDataStore`, so they use the same optimistic versioning, conflict handling, snapshot/pull/push protocol and server authorization as the other LifeTrace domains.

## 7. Completion history

Completing a task performs two writes:

1. update `execution.task.status = "done"` and `completedAt`;
2. create `execution.completion_result` when no result exists for that task.

The completion entity is intentional. Recurring or analytical features must not erase previous completion evidence by simply toggling a boolean back to false.

## 8. Boundaries of this increment

Implemented now:

- [x] Browser execution sync registry and scopes
- [x] Unified execution workspace
- [x] Quick task/memo capture
- [x] Inbox
- [x] Today task view
- [x] Project/plan view
- [x] Memo timeline
- [x] Task state transitions
- [x] Completion-result history
- [x] Root dashboard aggregation
- [x] Global search integration
- [x] Mobile navigation integration
- [x] Regression tests

Follow-up work, intentionally not duplicated as browser-only schema:

- [ ] Shared Goal contract
- [ ] Full recurrence editor on Web
- [ ] Waiting-item editor on Web
- [ ] Reminder editor on Web
- [ ] Task dependency/subtask editor on Web
- [ ] Dedicated calendar timeboxing UI on Web
- [ ] Memo → task/calendar/waiting conversion UI on Web
- [ ] Daily/weekly review analytics based on planned vs actual execution
- [ ] AI-assisted Inbox classification and plan decomposition

The desktop application already contains richer execution UI for several of these concepts. Follow-up web work should continue to reuse those semantics rather than invent parallel behavior.

## 9. Verification

The refactor must not be merged until the repository test/build workflow passes. Relevant browser regression coverage includes:

```text
apps/desktop/tests/web-execution-workspace.test.ts
apps/desktop/tests/browser-parity.test.ts
apps/desktop/tests/browser-ui-architecture.test.ts
```

The normal desktop test command also performs TypeScript checking, unit tests and browser builds:

```bash
npm test --prefix apps/desktop
```

GitHub Actions is the source of truth when the repository cannot be checked out in the current execution environment.
