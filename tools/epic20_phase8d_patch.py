from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing patch anchor: {label}")
    return text.replace(old, new, 1)

# ---- typed API: calendar recurrence ----
api = Path("apps/desktop/src/services/executionApi.ts")
text = api.read_text(encoding="utf-8")
calendar_anchor = '''    conflicts: (timing: CalendarTimingInput, excludeEventId?: string) =>
      request<Array<{ eventId: string; occurrenceId?: string | null; title: string; isAllDay: boolean }>>("/api/execution/calendar-conflicts", json("POST", { timing, excludeEventId })),
'''
calendar_replacement = calendar_anchor + '''    recurrence: (id: string) => request<RecurrenceRule | null>(`/api/execution/calendar-events/${encodeURIComponent(id)}/recurrence`),
    setRecurrence: (id: string, input: RecurrenceInput) =>
      request<RecurrenceRule>(`/api/execution/calendar-events/${encodeURIComponent(id)}/recurrence`, json("PUT", input)),
    clearRecurrence: (id: string) =>
      request<{ ok: true }>(`/api/execution/calendar-events/${encodeURIComponent(id)}/recurrence`, { method: "DELETE" }),
'''
if "calendar-events/${encodeURIComponent(id)}/recurrence" not in text:
    text = replace_once(text, calendar_anchor, calendar_replacement, "calendar recurrence API")
api.write_text(text, encoding="utf-8")

# ---- recurrence weekday encoding: backend is Monday=1 ... Sunday=7 ----
view_model = Path("apps/desktop/src/components/feature/execution/executionViewModel.ts")
text = view_model.read_text(encoding="utf-8")
text = replace_once(
    text,
    '.filter((value) => Number.isInteger(value) && value >= 0 && value <= 6)',
    '.filter((value) => Number.isInteger(value) && value >= 1 && value <= 7)',
    "weekday domain",
)
view_model.write_text(text, encoding="utf-8")

advanced = Path("apps/desktop/src/components/feature/execution/TaskAdvancedPanel.tsx")
text = advanced.read_text(encoding="utf-8")
if 'normalizeWeekdays' not in text:
    text = replace_once(
        text,
        'import {\n  browserTimezone,',
        'import { normalizeWeekdays } from "@/src/components/feature/execution/executionViewModel";\nimport {\n  browserTimezone,',
        "TaskAdvanced weekday import",
    )
text = replace_once(
    text,
    '      weekdays: frequency === "weekly" ? weekdays : [],',
    '      weekdays: frequency === "weekly" ? normalizeWeekdays(weekdays) : [],',
    "TaskAdvanced weekday save",
)
text = replace_once(
    text,
    '[[1,"一"],[2,"二"],[3,"三"],[4,"四"],[5,"五"],[6,"六"],[0,"日"]]',
    '[[1,"一"],[2,"二"],[3,"三"],[4,"四"],[5,"五"],[6,"六"],[7,"日"]]',
    "TaskAdvanced Sunday code",
)
advanced.write_text(text, encoding="utf-8")

# ---- ExecutionModule conflict guard + recurrence overlays ----
module = Path("apps/desktop/src/components/feature/execution/ExecutionModule.tsx")
text = module.read_text(encoding="utf-8")
if 'CalendarConflictDialog' not in text:
    text = replace_once(
        text,
        'import CalendarWorkspace from "@/src/components/feature/execution/CalendarWorkspace";\n',
        'import CalendarWorkspace from "@/src/components/feature/execution/CalendarWorkspace";\nimport CalendarConflictDialog, { type CalendarConflict } from "@/src/components/feature/execution/CalendarConflictDialog";\nimport CalendarRecurrencePanel from "@/src/components/feature/execution/CalendarRecurrencePanel";\n',
        "calendar overlay imports",
    )
if '  Repeat2,\n' not in text:
    text = replace_once(text, '  RefreshCw,\n', '  RefreshCw,\n  Repeat2,\n', "Repeat2 import")
if 'type CalendarTimingInput,' not in text:
    text = replace_once(text, '  type CalendarInput,\n', '  type CalendarInput,\n  type CalendarTimingInput,\n', "CalendarTimingInput import")
if 'type PendingCalendarAction' not in text:
    text = replace_once(
        text,
        'type ContextMenuState = { x: number; y: number; items: ExecutionMenuItem[] } | null;\n',
        'type ContextMenuState = { x: number; y: number; items: ExecutionMenuItem[] } | null;\ntype PendingCalendarAction = { title: string; conflicts: CalendarConflict[]; action: () => Promise<unknown>; success: string } | null;\n',
        "pending calendar action type",
    )

old_calendar_editor = 'function CalendarEditor({ value, sourceTask, busy, close, save, remove }: { value?: CalendarEvent; sourceTask?: ExecutionTask; busy: boolean; close: () => void; save: (input: CalendarInput) => Promise<void>; remove: () => Promise<void> }) {'
new_calendar_editor = 'function CalendarEditor({ value, sourceTask, busy, close, save, remove, onRecurrence }: { value?: CalendarEvent; sourceTask?: ExecutionTask; busy: boolean; close: () => void; save: (input: CalendarInput) => Promise<void>; remove: () => Promise<void>; onRecurrence: (event: CalendarEvent) => void }) {'
text = replace_once(text, old_calendar_editor, new_calendar_editor, "CalendarEditor props")
old_footer = '<footer>{value && !sourceTask ? <button className="lt-exec-danger" type="button" onClick={() => void remove()}><Trash2/>删除</button> : <span/>}<div><button type="button" onClick={close}>取消</button><button className="hx-btn primary" type="button" disabled={busy || !title.trim() || (!allDay && (!startAt || !endAt))} onClick={() => void save({ title, description: description || null, isAllDay: allDay, startAt: allDay ? null : localDateTimeToRfc3339(startAt), endAt: allDay ? null : localDateTimeToRfc3339(endAt), startLocalDate: allDay ? startDate : null, endLocalDate: allDay ? endDate : null, timezone: browserTimezone(), sourceTaskId: sourceTask?.id || value?.sourceTaskId || null })}>保存</button></div></footer>'
new_footer = '<footer>{value && !sourceTask ? <div className="lt-exec-editor-secondary"><button className="lt-exec-danger" type="button" onClick={() => void remove()}><Trash2/>删除</button><button type="button" onClick={() => onRecurrence(value)}><Repeat2/>重复规则</button></div> : <span/>}<div><button type="button" onClick={close}>取消</button><button className="hx-btn primary" type="button" disabled={busy || !title.trim() || (!allDay && (!startAt || !endAt))} onClick={() => void save({ title, description: description || null, isAllDay: allDay, startAt: allDay ? null : localDateTimeToRfc3339(startAt), endAt: allDay ? null : localDateTimeToRfc3339(endAt), startLocalDate: allDay ? startDate : null, endLocalDate: allDay ? endDate : null, timezone: browserTimezone(), sourceTaskId: sourceTask?.id || value?.sourceTaskId || null })}>保存</button></div></footer>'
text = replace_once(text, old_footer, new_footer, "CalendarEditor recurrence footer")

state_anchor = '  const [calendarRefreshToken, setCalendarRefreshToken] = useState(0);\n'
if 'recurrenceEvent' not in text:
    text = replace_once(
        text,
        state_anchor,
        state_anchor + '  const [recurrenceEvent, setRecurrenceEvent] = useState<CalendarEvent | null>(null);\n  const [pendingCalendarAction, setPendingCalendarAction] = useState<PendingCalendarAction>(null);\n',
        "calendar interaction state",
    )

run_end_anchor = '''  const createQuickTask = async () => {
'''
if 'guardCalendarAction' not in text:
    guard = '''  const guardCalendarAction = async (
    title: string,
    timing: CalendarTimingInput,
    excludeEventId: string | undefined,
    action: () => Promise<unknown>,
    success: string,
  ) => {
    if (timing.isAllDay) {
      await run(action, success);
      return;
    }
    setBusy(true);
    try {
      const conflicts = await executionApi.calendar.conflicts(timing, excludeEventId);
      if (conflicts.length) {
        setPendingCalendarAction({ title, conflicts, action, success });
        return;
      }
    } catch (cause) {
      toast(cause instanceof Error ? cause.message : "冲突检查失败", "error");
      return;
    } finally {
      setBusy(false);
    }
    await run(action, success);
  };

'''
    text = replace_once(text, run_end_anchor, guard + run_end_anchor, "calendar conflict guard")

old_render_calendar = '''  const renderCalendar = () => <CalendarWorkspace
    refreshToken={calendarRefreshToken}
    onCreate={() => setEditor({ kind: "calendar" })}
    onEdit={(value) => setEditor({ kind: "calendar", value })}
    onReminder={(subject) => setReminderSubject(subject)}
  />;'''
new_render_calendar = '''  const renderCalendar = () => <CalendarWorkspace
    refreshToken={calendarRefreshToken}
    onCreate={() => setEditor({ kind: "calendar" })}
    onEdit={(value) => setEditor({ kind: "calendar", value })}
    onMove={(value, timing) => guardCalendarAction(value.title, timing, value.id, () => executionApi.calendar.move(value.id, timing), "事件时间已调整")}
    onRecurrence={(value) => setRecurrenceEvent(value)}
    onReminder={(subject) => setReminderSubject(subject)}
  />;'''
text = replace_once(text, old_render_calendar, new_render_calendar, "CalendarWorkspace interaction props")

old_editor_call = '{editor.kind === "calendar" ? <CalendarEditor value={editor.value} sourceTask={editor.sourceTask} busy={busy} close={() => setEditor(null)} save={(input) => run(() => editor.sourceTask ? executionApi.tasks.schedule(editor.sourceTask.id, input) : editor.value ? executionApi.calendar.update(editor.value.id, input) : executionApi.calendar.create(input), editor.sourceTask ? "任务已安排到日历" : editor.value ? "事件已更新" : "事件已创建")} remove={() => run(() => executionApi.calendar.remove(editor.value!.id), "事件已删除")}/>: null}'
new_editor_call = '{editor.kind === "calendar" ? <CalendarEditor value={editor.value} sourceTask={editor.sourceTask} busy={busy} close={() => setEditor(null)} save={(input) => guardCalendarAction(input.title, input, editor.value?.id, () => editor.sourceTask ? executionApi.tasks.schedule(editor.sourceTask.id, input) : editor.value ? executionApi.calendar.update(editor.value.id, input) : executionApi.calendar.create(input), editor.sourceTask ? "任务已安排到日历" : editor.value ? "事件已更新" : "事件已创建")} remove={() => run(() => executionApi.calendar.remove(editor.value!.id), "事件已删除")} onRecurrence={(value) => { setEditor(null); setRecurrenceEvent(value); }}/>: null}'
text = replace_once(text, old_editor_call, new_editor_call, "CalendarEditor guarded save")

overlay_anchor = '    {contextMenu ? <ExecutionContextMenu {...contextMenu} onClose={() => setContextMenu(null)}/> : null}\n'
if 'CalendarRecurrencePanel' in text and '<CalendarRecurrencePanel event=' not in text:
    overlays = '''    {recurrenceEvent ? <div className="lt-exec-editor-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setRecurrenceEvent(null); }}>
      <CalendarRecurrencePanel event={recurrenceEvent} onClose={() => setRecurrenceEvent(null)} onChanged={async () => { await load(); setCalendarRefreshToken((value) => value + 1); }}/>
    </div> : null}
    {pendingCalendarAction ? <div className="lt-exec-editor-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) setPendingCalendarAction(null); }}>
      <CalendarConflictDialog
        title={pendingCalendarAction.title}
        conflicts={pendingCalendarAction.conflicts}
        busy={busy}
        onCancel={() => setPendingCalendarAction(null)}
        onConfirm={() => { const pending = pendingCalendarAction; setPendingCalendarAction(null); void run(pending.action, pending.success); }}
      />
    </div> : null}
'''
    text = replace_once(text, overlay_anchor, overlays + overlay_anchor, "calendar overlays")
module.write_text(text, encoding="utf-8")

# ---- calendar styles ----
css = Path("apps/desktop/app/execution-calendar.css")
text = css.read_text(encoding="utf-8")
extra = r'''

.lt-calendar-month-grid button strong,
.lt-calendar-all-day button {
  align-items: center;
}

.lt-calendar-month-grid button strong svg,
.lt-calendar-all-day button svg,
.lt-calendar-time-event strong svg {
  width: 11px;
  height: 11px;
  flex: 0 0 auto;
  color: var(--ui-primary);
}

.lt-calendar-month-grid button strong,
.lt-calendar-time-event strong {
  display: flex;
  gap: 3px;
  align-items: center;
}

.lt-calendar-all-day button {
  display: inline-flex;
  gap: 3px;
}

.lt-calendar-time-column.drag-target {
  background: color-mix(in srgb, var(--ui-primary-soft) 58%, var(--ui-bg-surface));
  box-shadow: inset 0 0 0 1px var(--ui-primary);
}

.lt-calendar-time-event[draggable="true"] {
  cursor: grab;
}

.lt-calendar-time-event[draggable="true"]:active {
  cursor: grabbing;
}

.lt-calendar-event-actions {
  position: absolute;
  top: 3px;
  right: 3px;
  display: flex;
  gap: 1px;
}

.lt-calendar-time-event .lt-calendar-event-actions button {
  position: static;
}

.lt-calendar-drag-hint {
  padding: 5px 10px;
  border-top: 1px solid var(--ui-border);
  color: var(--ui-faint);
  font-size: var(--ui-font-micro);
  text-align: right;
}

.lt-calendar-conflict-body {
  display: grid;
  gap: var(--ui-space-3);
  padding: var(--ui-space-4);
}

.lt-calendar-conflict-summary {
  display: flex;
  align-items: flex-start;
  gap: var(--ui-space-2);
  padding: var(--ui-space-3);
  border-radius: var(--ui-radius-sm);
  background: color-mix(in srgb, var(--ui-warning) 10%, var(--ui-bg-surface));
  font-size: var(--ui-font-caption);
}

.lt-calendar-conflict-summary svg {
  width: 17px;
  height: 17px;
  flex: 0 0 auto;
  color: var(--ui-warning);
}

.lt-calendar-conflict-list {
  display: grid;
  max-height: 210px;
  overflow: auto;
  border: 1px solid var(--ui-border);
  border-radius: var(--ui-radius-sm);
}

.lt-calendar-conflict-list > div {
  display: flex;
  justify-content: space-between;
  gap: var(--ui-space-3);
  padding: var(--ui-space-2) var(--ui-space-3);
  border-bottom: 1px solid var(--ui-border);
}

.lt-calendar-conflict-list > div:last-child {
  border-bottom: 0;
}

.lt-calendar-conflict-list strong,
.lt-calendar-conflict-list span {
  font-size: var(--ui-font-caption);
}

.lt-calendar-conflict-list span {
  color: var(--ui-muted);
}

.lt-exec-editor-secondary {
  display: flex;
  gap: var(--ui-space-2);
}

.lt-exec-editor-secondary button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.lt-exec-editor-secondary svg {
  width: 14px;
  height: 14px;
}
'''
if '.lt-calendar-time-column.drag-target' not in text:
    text += extra
css.write_text(text, encoding="utf-8")

# ---- tests ----
calendar_test = Path("apps/desktop/tests/execution-calendar-view.test.ts")
text = calendar_test.read_text(encoding="utf-8")
if 'moveTimedEventToSlot' not in text:
    text = text.replace('  localDateKey,\n', '  localDateKey,\n  moveTimedEventToSlot,\n  snapCalendarMinutes,\n', 1)
    text += r'''

test("drag minute snapping clamps and rounds to quarter hours", () => {
  assert.equal(snapCalendarMinutes(-30), 0);
  assert.equal(snapCalendarMinutes(67), 60);
  assert.equal(snapCalendarMinutes(68), 75);
  assert.equal(snapCalendarMinutes(2000), 1439);
});

test("dragging a timed event preserves duration and targets local wall-clock time", () => {
  const source = event({
    id: "drag-source",
    timezone: "Asia/Shanghai",
    startAt: new Date(2026, 7, 9, 9, 30).toISOString(),
    endAt: new Date(2026, 7, 9, 10, 45).toISOString(),
  });
  const timing = moveTimedEventToSlot(source, new Date(2026, 7, 11, 12), 14 * 60 + 7);
  assert.ok(timing?.startAt && timing.endAt);
  const start = new Date(timing.startAt);
  const end = new Date(timing.endAt);
  assert.equal(start.getFullYear(), 2026);
  assert.equal(start.getMonth(), 7);
  assert.equal(start.getDate(), 11);
  assert.equal(start.getHours(), 14);
  assert.equal(start.getMinutes(), 0);
  assert.equal(end.getTime() - start.getTime(), 75 * 60 * 1000);
  assert.equal(timing.timezone, "Asia/Shanghai");
});

test("all-day events cannot be dragged into timed slots", () => {
  const allDay = event({ isAllDay: true, startAt: null, endAt: null, startLocalDate: "2026-08-09", endLocalDate: "2026-08-09" });
  assert.equal(moveTimedEventToSlot(allDay, new Date(2026, 7, 10), 600), null);
});
'''
calendar_test.write_text(text, encoding="utf-8")

vm_test = Path("apps/desktop/tests/execution-view-model.test.ts")
text = vm_test.read_text(encoding="utf-8")
text = replace_once(
    text,
    'assert.deepEqual(normalizeWeekdays([5, 1, 1, 9, -1, 0]), [0, 1, 5]);',
    'assert.deepEqual(normalizeWeekdays([7, 5, 1, 1, 9, -1, 0]), [1, 5, 7]);',
    "weekday normalization test",
)
vm_test.write_text(text, encoding="utf-8")
