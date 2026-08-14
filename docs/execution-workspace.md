# LifeTrace Execution Workspace

> Status: browser/desktop P3 implementation, reusing one shared execution domain across Web, cloud sync and the desktop SQLite projection.
>
> Updated: 2026-08-14

## 1. Goal

LifeTrace does not model memo, todo, plan, waiting, calendar and long-term goals as unrelated islands. The execution workspace turns them into one capture-to-review loop:

```text
Goal
  ↓
Project / Plan
  ↓
Task ─────→ Recurrence Rule → Task Occurrence
  │                           ↓
  ├─→ Subtask / Dependency   Dependency-aware Today
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

Quick Capture and Memo remain the low-friction entry points; they eventually resolve into the same execution graph instead of creating separate task databases.

## 2. Canonical execution domain

The shared contract registry now provides:

- `execution.goal`
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

`execution.goal` is a normal user-owned bidirectional sync entity and therefore uses the existing `execution:read` / `execution:write` scopes. It is not a Web-only record.

The desktop SQLite projection adds `execution_goals` and `execution_projects.goal_id`. Goal insert/update/delete operations use the same local sync outbox as the other execution entities, and remote Goal payloads are projected back into the real SQLite tables.

## 3. Browser information architecture

The global sidebar still exposes one execution destination: **计划与待办** (`/execution`). Detail functions stay inside the execution domain.

### `/execution`

The Execution Hub wraps the existing workspace and links to the Goal and Control subroutes without adding more global navigation entries.

Core views remain:

1. **今天** — ordinary tasks plus recurring task occurrences.
2. **收件箱** — captured tasks not yet organized.
3. **计划** — projects and task progress.
4. **备忘** — memo timeline and conversion.
5. **重复** — recurring task editor.
6. **回顾** — rolling seven-day execution analytics.
7. **已完成** — non-recurring task completion history.

### `/execution/goals`

The Goal workspace implements:

```text
Goal → Project → Task
```

A Goal carries long-horizon intent, optional target time and lifecycle state. Projects can be attached to a Goal through `project.goalId`. Goal progress is derived from its Projects and Tasks rather than copied into another checklist.

Creating a Goal together with its first Project uses one atomic sync group so the Project cannot exist without its intended Goal because of a partial network/server write.

### `/execution/control`

1. **等待事项** — external dependencies and follow-up dates.
2. **提醒** — reminder lifecycle for task/calendar/waiting/memo objects.
3. **子任务与依赖** — parent-child structure and finish-before-start dependencies.
4. **重复日历** — calendar recurrence materialization and per-occurrence exceptions.

### `/calendar`

The existing calendar is the Timebox view over execution data while continuing to aggregate habits, finance, workouts, English and daily review records.

## 4. Goal semantics

Goal is deliberately different from Project and Task:

- **Goal** answers *why / what long-term outcome*;
- **Project** answers *which finite workstream*;
- **Task** answers *what can be executed next*.

Goal lifecycle:

```text
active ↔ paused
   │
   ├→ completed
   └→ cancelled
```

Completing a Goal does not delete, rewrite or auto-complete its Projects/Tasks. Historical execution evidence stays intact.

Goal progress is calculated from the current execution graph:

- attached Project count;
- completed Project count;
- Task count under attached Projects;
- completed Task count;
- derived Task completion rate.

## 5. Dependency-aware Today

Today now distinguishes **ready** and **blocked** work.

For each task scheduled/due today (including the parent task behind a recurring occurrence), LifeTrace checks `execution.task_dependency` edges. A task is blocked when any `dependsOnTaskId` prerequisite has not reached `done`.

The root dashboard therefore exposes:

- number of actions that can start immediately;
- number of actions currently blocked;
- explicit blocker titles for each blocked task.

This is advisory ordering at P3: the UI makes blockers visible. Hard server-side rejection of an illegal `todo → in_progress` transition remains future work so desktop, Web and future mobile clients can share one enforcement rule.

## 6. Atomic multi-entity sync

The sync protocol already contained `atomicGroupId`; P3 activates it in the browser through `atomicMutate`.

An atomic mutation can mix entity types:

```text
atomicGroupId = G
  ├─ upsert execution.goal
  ├─ upsert execution.project
  └─ ...
```

The browser does not publish any of those entities into its local CloudState until every result succeeds.

Server behavior was already implemented before P3:

- the in-memory sync store evaluates the complete group before committing it;
- PostgreSQL executes a multi-change group inside a nested transaction/savepoint;
- any rejected/conflicting member causes the whole group to roll back and returns `atomic_group_failed` for the group.

P3 uses this for Goal + first Project bootstrap. Existing older conversion/timebox flows can now be migrated to the same helper incrementally instead of adding a second transaction API.

## 7. Capture, Memo and Waiting semantics

A task is actionable work with explicit state, optional project, priority, due date, schedule and estimate.

```text
todo → in_progress → done
  ├──────────────→ waiting
  └──────────────→ cancelled
```

Quick-captured tasks use `context = "inbox"`.

A Memo is lighter than a formal Note: no required title/folder, quick capture first, chronological timeline, optional pin/archive, and conversion into actionable objects. Formal knowledge remains in Notes.

A Waiting Item represents work that cannot currently advance because the next change must come from another person, service or condition. Task → Waiting keeps the source task and sets it to `waiting`; resolving the waiting item does not silently finish the source task.

## 8. Timeboxing and recurrence

Task Timeboxing keeps task scheduling fields and `execution.calendar_event` aligned. Calendar remains a scheduling view over the same execution data, not another todo database.

Recurring tasks use:

```text
Task definition → Recurrence Rule → Task Occurrence
```

They never reset a completed task back to `todo`. Each occurrence owns its completion state.

Recurring calendar events use the same `execution.recurrence_rule` model with `execution.calendar_occurrence`. Per-instance skip/restore/move changes the occurrence without rewriting the parent recurrence rule.

## 9. Reminders, subtasks and dependencies

`execution.reminder` references an existing task/calendar/waiting/memo subject. Web manages lifecycle (`scheduled`, snooze, dismiss, cancel); reliable OS/background delivery remains the job of an active desktop/mobile notification executor.

A Subtask is a normal task with `parentTaskId` and inherits the parent Project.

Dependencies remain separate from hierarchy:

- parent/child = decomposition;
- `finish_before_start` = execution ordering.

The Web editor prevents dependency cycles before writing an edge.

## 10. Completion history and review

Non-recurring completion writes task state plus one `execution.completion_result`. Recurring task completion writes the individual Task Occurrence. Calendar occurrences keep their own status.

The Execution review continues to derive rolling seven-day metrics from actual execution evidence, while `/review` remains the subjective mood/energy/reflection record.

## 11. Search and sync

Global search now includes Goals in addition to tasks, projects, memos and waiting items. Goal results open `/execution/goals`.

Browser writes use the normal optimistic snapshot/pull/push protocol. Photos, encrypted local albums, credentials and local secrets remain outside this execution boundary.

## 12. Server-side scheduler decision

The current cloud service has a dedicated mail worker but no generic execution scheduler/lease framework. P3 therefore does **not** bolt recurrence and reminder polling onto the mail worker or pretend browser background execution is reliable.

Long-horizon materialization and due-reminder delivery remain a separate backend work item that should introduce one reusable execution maintenance worker with:

- distributed lease / single-owner processing;
- idempotent occurrence keys and reminder fire keys;
- bounded horizon materialization;
- retry/backoff and observability;
- notification channel adapters.

This keeps scheduling correctness independent of whether a Web page happens to be open.

## 13. Implementation status

Completed:

- [x] Shared `execution.goal` contract registration
- [x] Web Goal workspace and Goal → Project → Task progress
- [x] Desktop `execution_goals` migration and Project `goal_id` storage
- [x] Goal local outbox triggers and remote SQLite projection
- [x] Goal global search integration
- [x] Dependency-aware Today ready/blocked summary
- [x] Browser mixed-entity `atomicGroupId` helper
- [x] Atomic Goal + first Project bootstrap
- [x] Unified execution workspace / Inbox / Memo / Project / completion history
- [x] Task and calendar recurrence with occurrence history
- [x] Waiting items, reminders, subtasks and dependency cycle prevention
- [x] Calendar occurrence exceptions
- [x] Regression coverage for the P3 execution layer

Follow-up work:

- [ ] Migrate older Memo conversion and task-timebox multi-write flows to `atomicMutate`
- [ ] Shared hard enforcement of dependency-aware task start transitions
- [ ] Long-horizon/server-side recurrence materialization worker
- [ ] Reminder delivery worker and notification permission/channel UX
- [ ] Weekly review persistence and comparison across weeks
- [ ] AI-assisted Inbox classification and Goal/Project decomposition

## 14. Verification

Relevant regression coverage:

```text
apps/desktop/tests/web-execution-workspace.test.ts
apps/desktop/src-tauri/src/database/migrations/m0014_execution_goals.rs
apps/desktop/src-tauri/src/sync/execution.rs
crates/lifetrace-contracts/src/registry.rs
```

Validation includes linting, unit tests, Web build, browser build, Rust tests and clippy. Execution changes must not merge until GitHub Actions reports success.
