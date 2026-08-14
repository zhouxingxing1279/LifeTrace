# LifeTrace Execution Workspace

> Status: **P0–P4 complete**. Browser, cloud sync and desktop SQLite share one execution domain.
>
> Updated: 2026-08-14

## 1. Goal

LifeTrace does not model memo, todo, plan, waiting, calendar and long-term goals as unrelated islands. The execution workspace is one capture-to-review loop:

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
Weekly Review Snapshot / Dashboard / Search
```

Quick Capture and Memo remain low-friction entry points, but actionable work eventually resolves into this same execution graph.

## 2. Canonical execution domain

The shared contract registry provides:

- `execution.goal`
- `execution.weekly_review`
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

These are normal user-owned bidirectional sync entities under `execution:read` / `execution:write`.

Desktop SQLite projects the domain into real local tables. P3 added `execution_goals` and `execution_projects.goal_id`; P4 adds `execution_weekly_reviews`. Local insert/update/delete operations enter the normal sync outbox and remote payloads project back into SQLite.

## 3. Browser information architecture

The global sidebar exposes one destination: **计划与待办** (`/execution`). Internal routes remain domain-level details rather than additional global modules.

### `/execution`

Core views:

1. **今天** — ordinary tasks plus recurring task occurrences.
2. **收件箱** — captured work not yet organized.
3. **计划** — projects and task progress.
4. **备忘** — memo timeline and conversion.
5. **重复** — recurring task editor.
6. **回顾** — rolling seven-day analytics plus persistent weekly snapshots.
7. **已完成** — non-recurring task completion history.

### `/execution/goals`

Implements `Goal → Project → Task`. Goal progress is derived from attached Projects and Tasks, never copied into another checklist.

### `/execution/control`

Contains Waiting Items, Reminders, Subtasks/Dependencies and recurring-calendar exception controls.

### `/calendar`

The calendar is the Timebox view over execution data while still aggregating habits, finance, workouts, English and subjective daily review records.

## 4. Goal semantics

Goal answers *why / what long-term outcome*; Project answers *which finite workstream*; Task answers *what can be executed next*.

```text
active ↔ paused
   │
   ├→ completed
   └→ cancelled
```

Completing a Goal does not rewrite its Projects or Tasks. Historical execution evidence stays intact.

## 5. Dependency-aware Today and hard enforcement

The UI separates **ready** and **blocked** work and names unfinished predecessors.

P4 makes this rule authoritative on the server. Every `execution.task` upsert that attempts to enter `in_progress` or `done` is checked against active `execution.task_dependency` rows of type `finish_before_start`.

The PostgreSQL guard runs inside the actual sync transaction. For an atomic group such as:

```text
1. predecessor: todo → done
2. successor:   todo → in_progress
```

step 2 sees step 1's staged state and is allowed. An out-of-order or stale client that attempts step 2 alone is rejected with a dependency error. Desktop, Web and future clients therefore share the same enforcement rule.

## 6. Atomic multi-entity sync

`atomicGroupId` is now the standard for workflows whose objects must agree.

P4 migrates the remaining important multi-write paths to `atomicMutate`:

- Goal + first Project;
- Task completion + `completion_result`;
- Memo → Task + bidirectional lineage + Memo archive;
- Memo → Calendar + bidirectional lineage + Memo archive;
- Task Timebox + Calendar Event;
- remove Timebox + clear Task scheduling;
- Task → Waiting and Waiting → restored Task;
- Waiting → new Task + bidirectional lineage;
- recurrence rule + parent object + initial occurrences;
- recurrence shutdown + parent recurrence reference removal.

The browser publishes no local CloudState changes until every member succeeds. PostgreSQL uses a nested transaction/savepoint; any rejected/conflicting member rolls the group back.

## 7. Capture, Memo and Waiting semantics

A task is actionable work with explicit state, optional project, priority, due date, schedule and estimate.

```text
todo → in_progress → done
  ├──────────────→ waiting
  └──────────────→ cancelled
```

Quick-captured tasks use `context = "inbox"`.

A Memo is lighter than a formal Note and can later be converted into Task or Calendar objects while preserving lineage.

A Waiting Item represents work that cannot advance until an external person, service or condition changes. Task → Waiting keeps the source task and moves it to `waiting`; resolving the Waiting Item never silently completes the source Task.

## 8. Timeboxing and recurrence

Task Timeboxing keeps Task scheduling fields and `execution.calendar_event` aligned as one atomic operation.

Recurring tasks use:

```text
Task definition → Recurrence Rule → Task Occurrence
```

Recurring calendar events use the same `execution.recurrence_rule` with `execution.calendar_occurrence`. Per-instance skip/restore/move edits the occurrence rather than rewriting the parent rule.

## 9. Dedicated Execution maintenance worker

P4 introduces `services/cloud/src/bin/execution_worker.rs`. It is deliberately separate from `mail_worker`.

A database-backed lease (`execution_worker_leases`) provides one active owner across multiple cloud instances. The worker runs a bounded maintenance cycle and:

- materializes Task Occurrences for the next 60 days;
- materializes Calendar Occurrences for the next 60 days;
- respects recurrence interval, weekdays/month-day, `untilAt` and `maxOccurrences`;
- uses deterministic occurrence IDs plus semantic occurrence keys for idempotency;
- transitions due reminders from `scheduled` to `fired` and records `lastFiredAt`;
- writes worker-generated updates into both `sync_entities` and `sync_change_log`, so normal client pull/snapshot observes them.

The worker does **not** claim to deliver an OS notification itself. `fired` is the durable synchronized event; desktop/mobile notification adapters can present it through the appropriate OS channel.

The release Docker image now packages `/app/execution_worker`. Production and local Compose stacks run it as a separate service after PostgreSQL and the cloud API become healthy.

## 10. Completion history and weekly review

Non-recurring completion writes Task state and exactly one `execution.completion_result` atomically. Recurring completion belongs to the occurrence.

The seven-day execution review is still calculated from source evidence:

- Tasks;
- Task Occurrences;
- Completion Results;
- Timebox/scheduling data.

`execution.weekly_review` persists a snapshot of those calculated metrics plus an optional note. It does not copy Task state. The snapshot stores week range, planned/completed counts, completion rate, planned/actual minutes and overdue counts, allowing later week-to-week comparison without making analytics the source of truth.

`/review` remains the separate subjective mood/energy/reflection record.

## 11. Search and sync

Global search includes Goals, Tasks, Projects, Memos and Waiting Items. Browser writes continue through normal optimistic snapshot/pull/push sync. Photos, encrypted local albums, credentials and local secrets stay outside this execution boundary.

## 12. P0–P4 completion status

### P0 — foundation

- [x] unified Execution workspace, Inbox, Projects, Memo and completion history
- [x] shared cloud sync entities

### P1 — scheduling and recurrence

- [x] Timeboxing
- [x] task recurrence / occurrence history
- [x] Memo conversions
- [x] rolling seven-day review

### P2 — advanced execution control

- [x] Waiting Items
- [x] Reminders
- [x] Subtasks and dependency graph
- [x] recurring-calendar occurrence exceptions

### P3 — goals and atomic sync

- [x] shared Goal layer and desktop Goal projection
- [x] Goal → Project → Task progress
- [x] dependency-aware Today
- [x] mixed-entity `atomicMutate`

### P4 — reliability and closure

- [x] server-authoritative dependency transition enforcement
- [x] migration of remaining critical multi-entity workflows to `atomicMutate`
- [x] dedicated multi-instance-safe Execution maintenance worker
- [x] server recurrence materialization
- [x] durable due-reminder firing state
- [x] persistent `execution.weekly_review`
- [x] desktop weekly-review projection and outbox triggers
- [x] Docker/Compose execution-worker deployment

P0–P4 is the completed Execution refactor. Future features such as AI Inbox classification, notification-channel UX or richer goal coaching are product enhancements rather than unfinished architecture work.

## 13. Verification

Primary regression coverage includes:

```text
apps/desktop/tests/web-execution-workspace.test.ts
apps/desktop/tests/execution-p4.test.ts
apps/desktop/src-tauri/src/database/migrations/m0015_execution_weekly_reviews.rs
apps/desktop/src-tauri/src/sync/execution.rs
services/cloud/tests/execution_p4_postgres.rs
services/cloud/src/postgres_repository/push/execution_guard.rs
services/cloud/src/bin/execution_worker.rs
services/cloud/migrations/0021_execution_worker.sql
crates/lifetrace-contracts/src/registry.rs
```

Merge gate: Browser Web, PostgreSQL, Windows Sync and Local Encrypted Album workflows must all be successful, including lint, unit tests, browser/Web builds, Rust tests, migrations, clippy and Docker/Compose smoke coverage.