# LifeTrace Execution Workspace

> Status: browser P2 implementation, reusing the execution domain that already exists in the desktop application and cloud contract registry.
>
> Updated: 2026-08-14

## 1. Goal

LifeTrace does not model memo, todo, plan, waiting and calendar as unrelated islands. The execution workspace turns them into one capture-to-review loop:

```text
Quick Capture
    ↓
Inbox / Memo
    ↓
Plan (Project)
    ↓
Task ─────→ Recurrence Rule → Task Occurrence
  │                           ↓
  ├─→ Subtask / Dependency   Today
  │
  ├─→ Waiting Item ─→ Follow-up / Reminder
  │
  └─→ Timebox Calendar ─→ Calendar Recurrence → Calendar Occurrence / Exception
    ↓
Execution
    ↓
Completion Result / Occurrence status
    ↓
Dashboard / Review / Search
```

The browser deliberately reuses the existing `execution.*` contract instead of creating a second web-only task model.

## 2. Canonical execution domain

The shared contract registry provides:

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

The browser requests `execution:read` and `execution:write` and includes these entities in snapshot, pull and push operations.

## 3. Browser information architecture

The global sidebar still exposes one execution destination: **计划与待办** (`/execution`). Detail functions stay inside the execution domain instead of fragmenting global navigation.

### `/execution`

1. **今天** — ordinary tasks due/scheduled today plus recurring task occurrences.
2. **收件箱** — captured tasks not yet organized.
3. **计划** — projects and task progress.
4. **备忘** — memo timeline and conversion.
5. **重复** — recurring task editor.
6. **回顾** — rolling seven-day execution analytics.
7. **已完成** — non-recurring task completion history.

### `/execution/control`

1. **等待事项** — external dependencies and follow-up dates.
2. **提醒** — reminder lifecycle for task/calendar/waiting/memo objects.
3. **子任务与依赖** — parent-child task structure and finish-before-start dependencies.
4. **重复日历** — calendar recurrence materialization and per-occurrence exceptions.

### `/calendar`

The existing calendar is the Timebox view over execution data while continuing to aggregate habits, finance, workouts, English and daily review records.

## 4. Capture semantics

### Task

A task is actionable work with explicit execution state, optional project, priority, due date, schedule and estimate.

```text
todo → in_progress → done
  ├──────────────→ waiting
  └──────────────→ cancelled
```

Quick-captured tasks use `context = "inbox"`. Scheduling or assigning them removes the Inbox-only meaning.

### Memo

A memo is lighter than a formal Note: no required title or folder, quick capture first, chronological timeline, optional pin/archive, and conversion into actionable objects. Formal knowledge remains in Notes.

### Waiting item

A waiting item represents work that cannot currently advance because the next change must come from another person, service or external condition.

Core fields:

- `waitingFor`
- `expectedAt`
- `followUpAt`
- `sourceTaskId`
- `status = open/resolved/cancelled`
- `resolvedAt`
- `resolutionSummary`

Task → Waiting keeps the source task and changes its status to `waiting`. Resolving the waiting item does not silently complete the source task. The UI offers an explicit **恢复任务** action that resolves the waiting item and returns the source task to `todo`.

Waiting → Task conversion writes provenance links:

```text
waiting_item --converted_to--> task
waiting_item <--derived_from-- task
```

### Plan / Project

The stable contract still uses `execution.project` as the plan container. A browser-only Goal entity is intentionally not introduced.

A future shared migration may add:

```text
Goal → Project → Task → Completion / Occurrence
```

## 5. Timeboxing

When a task is placed into a time block, the browser writes both:

1. `execution.task.scheduledStartAt / scheduledEndAt`;
2. `execution.calendar_event` with `sourceTaskId`.

Removing a one-off task time block clears the task schedule and cancels the linked calendar event. Independent calendar blocks can also be created without a source task.

This keeps Today and Calendar consistent: Calendar is a scheduling view over the same execution data, not a second todo database.

## 6. Recurring tasks

Recurring tasks follow the desktop semantics and **do not reset one task back to todo after completion**.

```text
Task definition
  └─ recurrenceRuleId → Recurrence Rule
                          ↓
                    Task Occurrence #1
                    Task Occurrence #2
                    Task Occurrence #3
```

Supported browser recurrence fields:

- daily / weekly / monthly;
- interval greater than one;
- weekday selection for weekly rules;
- month day for monthly rules;
- optional end date;
- optional maximum occurrence count.

Saving a task rule materializes missing occurrences for the next 30 days. Completion updates the individual `execution.task_occurrence`; the recurring task definition remains intact.

## 7. Repeating calendar events and exceptions

Calendar recurrence uses the same shared `execution.recurrence_rule` model but materializes `execution.calendar_occurrence` objects.

The browser control center can:

- attach a recurrence rule to an existing calendar event;
- materialize missing occurrences for the next 60 days;
- skip one occurrence without modifying the parent recurrence rule;
- restore a skipped occurrence;
- move one occurrence to another date/time without moving the entire series;
- close the recurrence rule while preserving historical occurrences.

`/calendar` renders scheduled calendar occurrences directly. Once an event becomes recurring, the parent event acts as the recurrence template and the calendar renders the materialized occurrences to avoid showing the template and occurrence twice.

## 8. Reminder lifecycle

`execution.reminder` references an existing subject instead of copying its content.

Supported subjects:

- `task`
- `calendar_event`
- `waiting_item`
- `memo`

Browser lifecycle:

```text
scheduled → dismissed
     │
     ├─ snooze → scheduled with snoozedUntil
     └─ cancel → cancelled
```

Due state is derived from `COALESCE(snoozedUntil, triggerAt)`. The browser manages cloud reminder state; OS-level/background notification delivery remains the responsibility of the desktop/mobile notification executor.

## 9. Subtasks and dependencies

A subtask is a normal `execution.task` with `parentTaskId` and inherits the parent project.

Dependencies use `execution.task_dependency` with:

```text
dependencyType = finish_before_start
```

The Web editor checks for cycles before writing a new dependency. A task is shown as blocked when any prerequisite is not `done`.

This keeps hierarchy and dependency separate:

- parent/child = decomposition;
- dependency = execution ordering.

## 10. Conversion lineage

Memo conversion follows the desktop convention. Memo → Task and Memo → Calendar write two `execution.entity_link` records:

```text
memo --converted_to--> target
memo <--derived_from-- target
```

The source Memo is archived after conversion.

Waiting → Task uses the same lineage convention with `waiting_item` as the source type.

## 11. Completion history

Non-recurring task completion writes:

1. `execution.task.status = "done"` and `completedAt`;
2. one `execution.completion_result` for the task.

Recurring task execution uses `execution.task_occurrence.status = "completed"`. Calendar recurrence uses calendar occurrence status and does not overwrite task completion evidence.

## 12. Today dashboard and execution review

The root dashboard calculates daily action progress from:

- ordinary tasks assigned to today;
- recurring task occurrences for today;
- active habits.

The Execution **回顾** view computes a rolling seven-day review from execution evidence:

- planned actions;
- completed actions;
- completion rate;
- planned minutes;
- recorded actual minutes;
- overdue ordinary tasks;
- overdue recurring task occurrences.

Subjective mood/energy/reflection remains in `/review`; execution review and subjective review are complementary.

## 13. Search and sync

Global search includes tasks, projects, memos and waiting items. All browser writes continue through `CloudDataStore`, using the normal optimistic-versioned snapshot/pull/push protocol.

Photos, encrypted local albums, credentials and local secrets remain outside the browser sync boundary.

## 14. Implementation status

Completed:

- [x] Browser execution sync registry and scopes
- [x] Unified execution workspace
- [x] Quick task/memo capture
- [x] Inbox and Today views
- [x] Project/plan view
- [x] Memo timeline and conversion lineage
- [x] Task state transitions and completion-result history
- [x] Root dashboard aggregation and global search
- [x] Calendar Timeboxing
- [x] Task recurrence and task occurrence materialization
- [x] Seven-day execution review
- [x] Waiting-item editor and Task → Waiting flow
- [x] Waiting → Task conversion lineage
- [x] Reminder editor and snooze/dismiss/cancel lifecycle
- [x] Subtask editor
- [x] Task dependency editor with cycle prevention
- [x] Calendar recurrence materialization
- [x] Per-calendar-occurrence skip/restore/move exceptions
- [x] Recurring calendar occurrences rendered in `/calendar`
- [x] Regression coverage

Follow-up work:

- [ ] Shared Goal contract
- [ ] Long-horizon/server-side recurrence materialization scheduler
- [ ] Atomic multi-entity browser transaction API for conversion/scheduling writes
- [ ] Browser/desktop reminder delivery convergence and notification permission UX
- [ ] Dependency-aware Today ordering and automatic start blocking
- [ ] Weekly review persistence and comparison across weeks
- [ ] AI-assisted Inbox classification and project decomposition

## 15. Consistency note

The desktop application has richer local transactional execution APIs. Browser P2 mirrors their entity semantics, but browser cloud writes that span multiple entities are still sequential. The sync protocol already carries `atomicGroupId`; a later backend/client increment should expose a public atomic multi-entity mutation helper so conversion and scheduling become server-atomic as well as semantically consistent.

## 16. Verification

Relevant regression coverage:

```text
apps/desktop/tests/web-execution-workspace.test.ts
apps/desktop/tests/browser-parity.test.ts
apps/desktop/tests/browser-ui-architecture.test.ts
```

Validation includes linting, unit tests, Web build, browser build, Rust tests and clippy. Execution changes must not merge until GitHub Actions reports success.
