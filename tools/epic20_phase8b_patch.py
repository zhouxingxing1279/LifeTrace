from pathlib import Path
import re

module = Path("apps/desktop/src/components/feature/execution/ExecutionModule.tsx")
text = module.read_text(encoding="utf-8")

text = text.replace(
    'import { useCallback, useEffect, useMemo, useState } from "react";',
    'import { useCallback, useEffect, useMemo, useState, type MouseEvent as ReactMouseEvent } from "react";',
    1,
)
text = text.replace(
    '  Pin,\n  Plus,',
    '  Pin,\n  Pencil,\n  Plus,',
    1,
)

anchor = 'import {\n  browserTimezone,'
if 'TaskAdvancedPanel' not in text:
    imports = '''import TaskAdvancedPanel from "@/src/components/feature/execution/TaskAdvancedPanel";
import SubjectReminderPanel from "@/src/components/feature/execution/SubjectReminderPanel";
import MemoConvertPanel from "@/src/components/feature/execution/MemoConvertPanel";
import ExecutionContextMenu, { type ExecutionMenuItem } from "@/src/components/feature/execution/ExecutionContextMenu";
import { preserveTaskUpdateFields, waitingToTaskInput } from "@/src/components/feature/execution/executionViewModel";
'''
    text = text.replace(anchor, imports + anchor, 1)

text = text.replace(
    '  | { kind: "memo"; value?: Memo }\n  | { kind: "reminder"; subjectType: Reminder["subjectType"]; subjectId: string; title: string }\n  | null;',
    '  | { kind: "memo"; value?: Memo }\n  | null;\n\ntype ReminderSubject = { subjectType: Reminder["subjectType"]; subjectId: string; title: string };\ntype ContextMenuState = { x: number; y: number; items: ExecutionMenuItem[] } | null;',
    1,
)

# Task row: support details + context menu and block invalid cancelled -> done shortcut.
text = text.replace(
    '  onReminder,\n}: {',
    '  onReminder,\n  onContextMenu,\n}: {',
    1,
)
text = text.replace(
    '  onReminder: (task: ExecutionTask) => void;\n}) {',
    '  onReminder: (task: ExecutionTask) => void;\n  onContextMenu: (event: ReactMouseEvent<HTMLElement>, task: ExecutionTask) => void;\n}) {',
    1,
)
text = text.replace(
    '  return <article className={`lt-exec-row lt-exec-task priority-${task.priority}`}>',
    '  return <article className={`lt-exec-row lt-exec-task priority-${task.priority}`} onContextMenu={(event) => onContextMenu(event, task)}>',
    1,
)
text = text.replace(
    '      aria-label={task.status === "done" ? `恢复任务 ${task.title}` : `完成任务 ${task.title}`}\n      onClick={() => onStatus(task, nextStatus)}',
    '      aria-label={task.status === "cancelled" ? `任务已取消 ${task.title}` : task.status === "done" ? `恢复任务 ${task.title}` : `完成任务 ${task.title}`}\n      disabled={task.status === "cancelled"}\n      onClick={() => onStatus(task, nextStatus)}',
    1,
)

# Remove the old add-only reminder editor; reminder management is now a dedicated panel.
text = re.sub(
    r'\nfunction ReminderEditor\([\s\S]*?\n}\n\nexport default function ExecutionModule\(\) \{',
    '\nexport default function ExecutionModule() {',
    text,
    count=1,
)

# Add advanced states.
state_anchor = '  const [editor, setEditor] = useState<Editor>(null);\n'
state_insert = '''  const [editor, setEditor] = useState<Editor>(null);
  const [inspectTask, setInspectTask] = useState<ExecutionTask | null>(null);
  const [reminderSubject, setReminderSubject] = useState<ReminderSubject | null>(null);
  const [convertMemo, setConvertMemo] = useState<Memo | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>(null);
'''
if 'const [inspectTask' not in text:
    text = text.replace(state_anchor, state_insert, 1)

# Keep Memo query/archive mode after generic reload.
text = text.replace(
    '      await load();\n    } catch (cause) {',
    '''      await load();
      if (tab === "memos" && (memoArchived || memoQuery.trim())) {
        const memos = await executionApi.memos.list({
          status: memoArchived ? "archived" : "active",
          q: memoQuery.trim() || undefined,
        });
        setData((current) => ({ ...current, memos }));
      }
    } catch (cause) {''',
    1,
)

status_anchor = '  const setTaskStatus = (task: ExecutionTask, status: ExecutionTaskStatus) => void run(() => executionApi.tasks.setStatus(task.id, status), status === "done" ? "任务已完成" : "任务状态已更新");\n'
menu_helpers = '''  const setTaskStatus = (task: ExecutionTask, status: ExecutionTaskStatus) => void run(() => executionApi.tasks.setStatus(task.id, status), status === "done" ? "任务已完成" : "任务状态已更新");

  const menuPosition = (event: ReactMouseEvent<HTMLElement>) => {
    event.preventDefault();
    const rect = event.currentTarget.getBoundingClientRect();
    return {
      x: event.clientX || rect.left + 24,
      y: event.clientY || rect.top + 24,
    };
  };

  const openTaskMenu = (event: ReactMouseEvent<HTMLElement>, task: ExecutionTask) => {
    const position = menuPosition(event);
    const items: ExecutionMenuItem[] = [
      { id: "details", label: "查看任务详情", action: () => setInspectTask(task) },
      { id: "edit", label: "编辑任务", icon: Pencil, action: () => setEditor({ kind: "task", value: task }) },
      { id: "schedule", label: "安排到日历", icon: CalendarDays, action: () => setEditor({ kind: "calendar", sourceTask: task }) },
      { id: "reminder", label: "管理提醒", icon: Bell, action: () => setReminderSubject({ subjectType: "task", subjectId: task.id, title: task.title }) },
      { id: "complete", label: task.status === "done" ? "恢复为待办" : "标记完成", icon: Check, disabled: task.status === "cancelled", action: () => setTaskStatus(task, task.status === "done" ? "todo" : "done") },
      { id: "delete", label: "删除任务", icon: Trash2, danger: true, action: () => { if (window.confirm(`确定删除“${task.title}”吗？`)) void run(() => executionApi.tasks.remove(task.id), "任务已删除"); } },
    ];
    setContextMenu({ ...position, items });
  };

  const openMemoMenu = (event: ReactMouseEvent<HTMLElement>, memo: Memo) => {
    const position = menuPosition(event);
    const items: ExecutionMenuItem[] = [
      { id: "edit", label: "编辑 Memo", icon: Pencil, action: () => setEditor({ kind: "memo", value: memo }) },
      { id: "pin", label: memo.isPinned ? "取消置顶" : "置顶", icon: Pin, action: () => void run(() => executionApi.memos.pin(memo.id, !memo.isPinned), memo.isPinned ? "已取消置顶" : "已置顶") },
      { id: "reminder", label: "管理提醒", icon: Bell, action: () => setReminderSubject({ subjectType: "memo", subjectId: memo.id, title: memo.plainText.slice(0, 30) }) },
      { id: "convert", label: "转换为…", action: () => setConvertMemo(memo), disabled: memo.status !== "active" },
      { id: "archive", label: memo.status === "active" ? "归档" : "恢复", icon: memo.status === "active" ? Archive : RotateCcw, action: () => void run(() => memo.status === "active" ? executionApi.memos.archive(memo.id) : executionApi.memos.restore(memo.id), memo.status === "active" ? "Memo 已归档" : "Memo 已恢复") },
      { id: "delete", label: "删除 Memo", icon: Trash2, danger: true, action: () => { if (window.confirm("确定删除这条 Memo 吗？")) void run(() => executionApi.memos.remove(memo.id), "Memo 已删除"); } },
    ];
    setContextMenu({ ...position, items });
  };
'''
if 'const openTaskMenu' not in text:
    if status_anchor not in text:
        raise SystemExit('task status helper anchor not found')
    text = text.replace(status_anchor, menu_helpers, 1)

# Task rows: clicking title opens advanced inspector; reminder/context actions go to new managers.
text = text.replace('onEdit={(value) => setEditor({ kind: "task", value })}', 'onEdit={(value) => setInspectTask(value)}')
text = text.replace('onReminder={(value) => setEditor({ kind: "reminder", subjectType: "task", subjectId: value.id, title: value.title })}', 'onReminder={(value) => setReminderSubject({ subjectType: "task", subjectId: value.id, title: value.title })}')
text = text.replace('/>) : <SectionEmpty>今天没有需要推进的任务</SectionEmpty>', 'onContextMenu={openTaskMenu}/>) : <SectionEmpty>今天没有需要推进的任务</SectionEmpty>', 1)
text = text.replace('/>) : <SectionEmpty>当前筛选下没有任务</SectionEmpty>', 'onContextMenu={openTaskMenu}/>) : <SectionEmpty>当前筛选下没有任务</SectionEmpty>', 1)

# Waiting actions: reminder manager + Waiting -> Task.
text = text.replace(
    'onClick={() => setEditor({ kind: "reminder", subjectType: "waiting_item", subjectId: item.id, title: item.title })}><Bell/></button><button type="button" title="标记已解决"',
    'onClick={() => setReminderSubject({ subjectType: "waiting_item", subjectId: item.id, title: item.title })}><Bell/></button><button type="button" title="转为任务" onClick={() => void run(() => executionApi.waiting.convertToTask(item.id, waitingToTaskInput(item)), "已转为任务")}><ChevronRight/></button><button type="button" title="标记已解决"',
    1,
)

# Memo cards: context menu + full conversion + reminder manager.
text = text.replace(
    'data.memos.map((memo) => <article key={memo.id} className={memo.isPinned ? "pinned" : ""}>',
    'data.memos.map((memo) => <article key={memo.id} className={memo.isPinned ? "pinned" : ""} onContextMenu={(event) => openMemoMenu(event, memo)}>',
    1,
)
text = text.replace(
    'onClick={() => setEditor({ kind: "reminder", subjectType: "memo", subjectId: memo.id, title: memo.plainText.slice(0, 30) })}><Bell/>提醒</button>{memo.status === "active" ? <button type="button" onClick={() => void run(() => executionApi.memos.convertToTask(memo.id, { title: memo.plainText.slice(0, 80), priority: "normal", timezone: browserTimezone() }), "已转换为任务")}>转任务<ChevronRight/></button> : null}',
    'onClick={() => setReminderSubject({ subjectType: "memo", subjectId: memo.id, title: memo.plainText.slice(0, 30) })}><Bell/>提醒</button>{memo.status === "active" ? <button type="button" onClick={() => setConvertMemo(memo)}>转换<ChevronRight/></button> : null}',
    1,
)

# Preserve hidden task fields during basic editor updates and remove old ReminderEditor render.
text = text.replace(
    'editor.value ? executionApi.tasks.update(editor.value.id, input) : executionApi.tasks.create(input)',
    'editor.value ? executionApi.tasks.update(editor.value.id, preserveTaskUpdateFields(editor.value, input)) : executionApi.tasks.create(input)',
    1,
)
text = re.sub(r'\n      \{editor\.kind === "reminder"[\s\S]*?\}/>: null\}', '', text, count=1)

# Add advanced overlays and context menu after the basic editor overlay.
overlay_anchor = '    </div> : null}\n  </div>;\n}'
overlays = '''    </div> : null}
    {inspectTask ? <div className="lt-exec-editor-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setInspectTask(null); }}>
      <TaskAdvancedPanel
        task={inspectTask}
        projects={data.projects}
        allTasks={data.tasks}
        onClose={() => setInspectTask(null)}
        onEdit={() => { const value = inspectTask; setInspectTask(null); setEditor({ kind: "task", value }); }}
        onSchedule={() => { const sourceTask = inspectTask; setInspectTask(null); setEditor({ kind: "calendar", sourceTask }); }}
        onReminder={() => { const value = inspectTask; setInspectTask(null); setReminderSubject({ subjectType: "task", subjectId: value.id, title: value.title }); }}
        onChanged={load}
      />
    </div> : null}
    {reminderSubject ? <div className="lt-exec-editor-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setReminderSubject(null); }}>
      <SubjectReminderPanel {...reminderSubject} onClose={() => setReminderSubject(null)} onChanged={load}/>
    </div> : null}
    {convertMemo ? <div className="lt-exec-editor-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setConvertMemo(null); }}>
      <MemoConvertPanel memo={convertMemo} projects={data.projects} onClose={() => setConvertMemo(null)} onConverted={async () => { await load(); if (tab === "memos") await refreshMemos(); }}/>
    </div> : null}
    {contextMenu ? <ExecutionContextMenu {...contextMenu} onClose={() => setContextMenu(null)}/> : null}
  </div>;
}'''
if 'TaskAdvancedPanel' in text and 'inspectTask ? <div' not in text:
    if overlay_anchor not in text:
        raise SystemExit('overlay anchor not found')
    text = text.replace(overlay_anchor, overlays, 1)

module.write_text(text, encoding="utf-8")

# Append advanced UI styles.
css = Path("apps/desktop/app/execution.css")
style = css.read_text(encoding="utf-8")
if ".lt-exec-inspector-body" not in style:
    style += r'''

/* EPIC20 Phase 8B advanced interactions */
.lt-exec-check:disabled{cursor:not-allowed;opacity:.35}
.lt-exec-loading.compact,.lt-exec-empty.compact{min-height:110px}
.lt-exec-inspector{width:min(580px,94vw)}
.lt-exec-inspector-body,.lt-exec-reminder-body,.lt-exec-convert-body{overflow:auto;padding:var(--ui-space-4);background:var(--ui-bg-surface)}
.lt-exec-inspector-actions{display:grid;grid-template-columns:repeat(3,1fr);gap:8px;margin-bottom:var(--ui-space-4)}
.lt-exec-inspector-actions button,.lt-exec-inline-create button,.lt-exec-inspector-footer-actions button,.lt-exec-reminder-create button,.lt-exec-reminder-list footer button,.lt-exec-reminder-snooze button,.lt-exec-target-tabs button{display:inline-flex;align-items:center;justify-content:center;gap:6px;min-height:34px;padding:0 10px;border:1px solid var(--ui-border);border-radius:var(--ui-radius-sm);background:var(--ui-bg-surface);color:var(--ui-foreground);font:inherit;font-size:var(--ui-font-caption)}
.lt-exec-inspector-actions button:hover,.lt-exec-inline-create button:hover,.lt-exec-inspector-footer-actions button:hover,.lt-exec-reminder-list footer button:hover,.lt-exec-target-tabs button:hover{background:var(--ui-bg-hover)}
.lt-exec-inspector-actions svg,.lt-exec-inline-create svg,.lt-exec-inspector-footer-actions svg,.lt-exec-reminder-create svg,.lt-exec-reminder-list svg,.lt-exec-target-tabs svg{width:14px;height:14px}
.lt-exec-inspector-section{margin-bottom:var(--ui-space-4);border:1px solid var(--ui-border);border-radius:var(--ui-radius-md);background:var(--ui-bg-surface);overflow:hidden}
.lt-exec-inspector-section>header{display:flex;align-items:center;justify-content:space-between;min-height:42px;padding:0 var(--ui-space-3);border-bottom:1px solid var(--ui-border);background:var(--ui-bg-inset)}
.lt-exec-inspector-section>header>div{display:flex;align-items:baseline;gap:7px}.lt-exec-inspector-section>header span,.lt-exec-muted{color:var(--ui-muted);font-size:var(--ui-font-caption)}.lt-exec-inspector-section>header svg{width:15px;height:15px;color:var(--ui-muted)}
.lt-exec-inline-create{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:7px;padding:var(--ui-space-3);border-bottom:1px solid var(--ui-border)}
.lt-exec-inline-create input,.lt-exec-inline-create select,.lt-exec-recurrence-grid input,.lt-exec-recurrence-grid select,.lt-exec-reminder-create input,.lt-exec-reminder-snooze input{min-height:34px;padding:0 9px;border:1px solid var(--ui-border);border-radius:var(--ui-radius-sm);background:var(--ui-bg-surface);color:var(--ui-foreground);font:inherit;font-size:var(--ui-font-caption)}
.lt-exec-compact-list{display:grid;padding:var(--ui-space-2)}.lt-exec-compact-list>div{display:flex;align-items:center;justify-content:space-between;gap:8px;min-height:36px;padding:0 5px}.lt-exec-compact-list>div>button:first-child{display:flex;align-items:center;gap:8px;min-width:0;border:0;background:transparent;color:var(--ui-foreground);font:inherit;text-align:left}.lt-exec-compact-list>div>button:first-child.done{text-decoration:line-through;color:var(--ui-muted)}.lt-exec-compact-list>div>span{display:flex;align-items:center;gap:7px;min-width:0}.lt-exec-compact-list>div>span svg{width:13px;height:13px;color:var(--ui-muted)}.lt-exec-compact-list>div>button:last-child{display:grid;place-items:center;width:28px;height:28px;border:0;border-radius:var(--ui-radius-sm);background:transparent;color:var(--ui-muted)}.lt-exec-compact-list>div>button:last-child:hover{background:var(--ui-bg-hover);color:var(--ui-danger)}
.lt-exec-mini-check{display:grid;place-items:center;width:17px;height:17px;border:1px solid var(--ui-border-strong);border-radius:50%}.lt-exec-mini-check svg{width:11px;height:11px}
.lt-exec-blockers{display:grid;gap:4px;padding:var(--ui-space-3);border-top:1px solid var(--ui-border);background:color-mix(in srgb,var(--ui-warning) 7%,transparent);font-size:var(--ui-font-caption)}.lt-exec-blockers span{color:var(--ui-muted)}
.lt-exec-recurrence-grid{display:grid;grid-template-columns:1fr 1fr;gap:var(--ui-space-3);padding:var(--ui-space-3)}.lt-exec-recurrence-grid label{display:grid;gap:5px;color:var(--ui-muted);font-size:var(--ui-font-caption)}.lt-exec-recurrence-grid .wide{grid-column:1/-1}
.lt-exec-weekdays{display:flex;gap:6px;padding:0 var(--ui-space-3) var(--ui-space-3)}.lt-exec-weekdays button{width:32px;height:32px;padding:0;border:1px solid var(--ui-border);border-radius:50%;background:var(--ui-bg-surface);color:var(--ui-muted)}.lt-exec-weekdays button.active{border-color:var(--ui-primary);background:var(--ui-primary-soft);color:var(--ui-primary);font-weight:600}
.lt-exec-inspector-footer-actions{display:flex;justify-content:flex-end;gap:7px;padding:0 var(--ui-space-3) var(--ui-space-3)}.lt-exec-inspector-footer-actions .danger{color:var(--ui-danger)}
.lt-exec-reminder-panel{width:min(520px,94vw)}.lt-exec-reminder-create{display:grid;grid-template-columns:minmax(0,1fr) auto;align-items:end;gap:8px;margin-bottom:var(--ui-space-4)}.lt-exec-reminder-create label{display:grid;gap:6px;color:var(--ui-muted);font-size:var(--ui-font-caption)}
.lt-exec-reminder-list{display:grid;gap:var(--ui-space-2)}.lt-exec-reminder-list article{padding:var(--ui-space-3);border:1px solid var(--ui-border);border-radius:var(--ui-radius-md);background:var(--ui-bg-surface)}.lt-exec-reminder-main{display:flex;align-items:flex-start;gap:10px}.lt-exec-reminder-main>svg{width:16px;height:16px;margin-top:2px;color:var(--ui-primary)}.lt-exec-reminder-main strong,.lt-exec-reminder-main span{display:block}.lt-exec-reminder-main span{margin-top:2px;color:var(--ui-muted);font-size:var(--ui-font-caption)}.lt-exec-reminder-list footer{display:flex;flex-wrap:wrap;gap:5px;margin-top:var(--ui-space-3)}.lt-exec-reminder-list footer .danger{margin-left:auto;color:var(--ui-danger)}.lt-exec-reminder-snooze{display:grid;grid-template-columns:minmax(0,1fr) auto auto;gap:6px;margin-top:var(--ui-space-3)}
.lt-exec-convert-panel{width:min(560px,94vw)}.lt-exec-convert-body{padding:0}.lt-exec-target-tabs{display:grid;grid-template-columns:repeat(3,1fr);gap:6px;padding:var(--ui-space-3);border-bottom:1px solid var(--ui-border);background:var(--ui-bg-inset)}.lt-exec-target-tabs button.active{border-color:var(--ui-primary);background:var(--ui-primary-soft);color:var(--ui-primary)}.lt-exec-source-preview{padding:var(--ui-space-3);border:1px solid var(--ui-border);border-radius:var(--ui-radius-sm);background:var(--ui-bg-inset)}.lt-exec-source-preview p{max-height:130px;overflow:auto;margin:7px 0 0;color:var(--ui-muted);font-size:var(--ui-font-caption);line-height:1.55;white-space:pre-wrap}
.lt-exec-context-menu{position:fixed;z-index:1300;display:grid;min-width:210px;padding:5px;border:1px solid var(--ui-border);border-radius:var(--ui-radius-md);background:var(--ui-bg-surface);box-shadow:0 14px 38px rgb(0 0 0/.18)}.lt-exec-context-menu button{display:grid;grid-template-columns:18px minmax(0,1fr);align-items:center;gap:7px;min-height:34px;padding:0 9px;border:0;border-radius:var(--ui-radius-sm);background:transparent;color:var(--ui-foreground);font:inherit;font-size:var(--ui-font-caption);text-align:left}.lt-exec-context-menu button:hover{background:var(--ui-bg-hover)}.lt-exec-context-menu button.danger{color:var(--ui-danger)}.lt-exec-context-menu button:disabled{opacity:.45}.lt-exec-context-menu svg{width:14px;height:14px}
@media(max-width:720px){.lt-exec-recurrence-grid{grid-template-columns:1fr}.lt-exec-recurrence-grid .wide{grid-column:auto}.lt-exec-reminder-snooze{grid-template-columns:1fr}.lt-exec-inspector-actions{grid-template-columns:1fr 1fr 1fr}}
'''
    css.write_text(style, encoding="utf-8")
