# LifeTrace Execution Workspace

> Status: browser P1 implementation, reusing the execution domain that already exists in the desktop application and cloud contract registry.
>
> Updated: 2026-08-14

## 1. Goal

LifeTrace does not model memo, todo, plan and calendar as unrelated islands. The execution workspace turns them into one capture-to-review loop:

```text
Quick Capture
    ↓
Inbox / Memo
    ↓
Plan (Project)
    ↓
Task ─────→ Recurrence Rule → Task Occurrence
    ↓                           ↓
Today / Timebox Calendar        Today
    ↓                           ↓
Execution ──────────────────────┘
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

The browser requests `execution:read` and `execution:write` and includes the execution entities in snapshot, pull and push operations.

## 3. Browser information architecture

The browser exposes one global destination: **计划与待办** (`/execution`). Internal views remain local to that domain:

1. **今天** — ordinary tasks due/scheduled today plus materialized recurring occurrences.
2. **收件箱** — captured tasks that have not yet been organized.
3. **计划** — projects and task progress.
4. **备忘** — title-free memo timeline with pin/archive/conversion actions.
5. **重复** — recurrence editor and occurrence materialization controls.
6. **回顾** — seven-day planned-vs-completed execution analytics and overdue decisions.
7. **已完成** — non-recurring task completion history.

The existing `/calendar` route is now the execution timeboxing calendar while still aggregating habits, finance, workouts, English and daily review records.

## 4. Capture semantics

### Task

A task is an actionable item with explicit execution state, optional project, priority, date, schedule and estimate.

```text
todo → in_progress → done
  └──────────────→ cancelled
```

Quick-captured tasks use `context = "inbox"`. Scheduling or assigning them removes the Inbox-only meaning.

### Memo

A memo is lighter than a formal Note:

- no title required;
- no folder required;
- quick capture first;
- chronological timeline;
- optional pin/archive;
- convertible into an actionable item.

Formal knowledge remains in Notes.

### Plan / Project

The current stable contract uses `execution.project` as the plan container. A browser-only Goal entity is intentionally not introduced.

A future shared migration may add:

```text
Goal → Project → Task → Completion / Occurrence
```

## 5. Timeboxing

`/calendar` now supports real execution scheduling.

When a task is placed into a time block, the browser writes both:

1. `execution.task.scheduledStartAt / scheduledEndAt`;
2. `execution.calendar_event` with `sourceTaskId`.

Removing the time block clears the task schedule and cancels the linked calendar event. Independent calendar blocks can also be created without a source task.

This keeps Today and Calendar consistent: Calendar is a scheduling view over the same task data, not a second todo database.

## 6. Recurring tasks

Recurring work follows the desktop execution semantics and **does not reset one task back to todo after completion**.

The model is:

```text
Task definition
  └─ recurrenceRuleId → Recurrence Rule
                          ↓
                    Task Occurrence #1
                    Task Occurrence #2
                    Task Occurrence #3
```

Supported Web recurrence frequencies:

- daily;
- weekly with weekday selection;
- monthly with month day;
- interval greater than one;
- optional end date;
- optional maximum occurrence count.

Saving a rule materializes missing occurrences for the next 30 days. Materialization is idempotent by `taskId + occurrenceKey`, respects `maxOccurrences`, and can be run again to extend the horizon.

Completing a recurring occurrence updates that occurrence to `completed`. The parent task remains the recurring definition, so future occurrences and historical evidence are preserved.

## 7. Memo conversion lineage

Memo conversion follows the desktop domain convention. Converting Memo → Task or Memo → Calendar writes two `execution.entity_link` entities:

```text
memo --converted_to--> target
memo <--derived_from-- target
```

The source Memo is then archived. This preserves provenance instead of silently copying text and losing where the actionable item came from.

The current browser conversion flow supports:

- Memo → Task;
- Memo → Calendar time block.

Waiting-item conversion remains a follow-up because the browser does not yet expose the waiting-item editor.

## 8. Completion history

Non-recurring task completion writes:

1. `execution.task.status = "done"` and `completedAt`;
2. one `execution.completion_result` for the task.

Recurring task execution uses `execution.task_occurrence.status = "completed"` instead. `completion_result` is not repeatedly overwritten for recurring instances.

## 9. Today dashboard

The root dashboard calculates daily action progress from:

- ordinary tasks assigned to today;
- recurring task occurrences for today;
- active habits.

It also surfaces Inbox size, active project count, memo count and recurring occurrence activity while retaining fitness, learning, finance and review summaries.

## 10. Daily / weekly execution review

The Execution **回顾** view computes a rolling seven-day review from execution evidence rather than introducing a second review database.

Metrics include:

- planned ordinary tasks + recurring occurrences;
- completed actions;
- completion rate;
- estimated planned minutes;
- actual minutes recorded in `completion_result`;
- overdue ordinary tasks;
- overdue recurring occurrences.

Subjective mood/energy/reflection remains in the existing `/review` daily-review module. Execution review and subjective review therefore complement rather than duplicate each other.

## 11. Search and sync

Global search includes tasks, projects, memos and waiting items. All browser writes continue through `CloudDataStore`, using the normal optimistic-versioned LifeTrace snapshot/pull/push protocol.

Photos, encrypted local albums, credentials and local secrets remain outside the browser sync boundary.

## 12. Implementation status

Completed:

- [x] Browser execution sync registry and scopes
- [x] Unified execution workspace
- [x] Quick task/memo capture
- [x] Inbox and Today views
- [x] Project/plan view
- [x] Memo timeline
- [x] Task state transitions
- [x] Completion-result history
- [x] Root dashboard aggregation
- [x] Global search integration
- [x] Mobile navigation integration
- [x] Calendar timeboxing UI
- [x] Task ↔ calendar scheduling consistency
- [x] Recurrence editor on Web
- [x] Recurrence occurrence materialization
- [x] Memo → task conversion with lineage
- [x] Memo → calendar conversion with lineage
- [x] Seven-day execution review analytics
- [x] Regression tests

Follow-up work:

- [ ] Shared Goal contract
- [ ] Waiting-item editor on Web
- [ ] Reminder editor on Web
- [ ] Task dependency/subtask editor on Web
- [ ] Calendar recurrence/occurrence exception editor on Web
- [ ] Long-horizon/server-side recurrence materialization scheduler
- [ ] Atomic multi-entity browser transaction API for conversion/scheduling writes
- [ ] Weekly review persistence and comparison across weeks
- [ ] AI-assisted Inbox classification and project decomposition

## 13. Consistency note

The desktop application already has richer local execution APIs. Browser P1 mirrors their entity semantics, but the browser cloud store currently performs cross-entity conversion/scheduling writes sequentially. The shared sync protocol already has `atomicGroupId`; a later backend/client increment should expose a public atomic multi-entity mutation helper so Memo conversion and task scheduling become server-atomic as well as semantically consistent.

## 14. Verification

Relevant regression coverage:

```text
apps/desktop/tests/web-execution-workspace.test.ts
apps/desktop/tests/browser-parity.test.ts
apps/desktop/tests/browser-ui-architecture.test.ts
```

The normal browser/desktop validation includes linting, unit tests, Web build and browser build. Cloud CI also runs Rust tests and clippy. Changes must not merge until GitHub Actions reports success.
