from pathlib import Path

module = Path("apps/desktop/src/components/feature/execution/ExecutionModule.tsx")
text = module.read_text(encoding="utf-8")

if 'CalendarWorkspace' not in text:
    text = text.replace(
        'import MemoConvertPanel from "@/src/components/feature/execution/MemoConvertPanel";\n',
        'import MemoConvertPanel from "@/src/components/feature/execution/MemoConvertPanel";\nimport CalendarWorkspace from "@/src/components/feature/execution/CalendarWorkspace";\n',
        1,
    )

state_anchor = '  const [quickTask, setQuickTask] = useState("");\n'
if 'calendarRefreshToken' not in text:
    text = text.replace(state_anchor, state_anchor + '  const [calendarRefreshToken, setCalendarRefreshToken] = useState(0);\n', 1)

run_anchor = '      await load();\n      if (tab === "memos" && (memoArchived || memoQuery.trim())) {'
if 'setCalendarRefreshToken' in text and 'setCalendarRefreshToken((value) => value + 1);' not in text:
    text = text.replace(
        run_anchor,
        '      await load();\n      setCalendarRefreshToken((value) => value + 1);\n      if (tab === "memos" && (memoArchived || memoQuery.trim())) {',
        1,
    )

start_marker = '  const renderCalendar = () =>'
end_marker = '\n\n  const renderWaiting ='
start = text.find(start_marker)
if start == -1:
    raise SystemExit("renderCalendar start marker not found")
end = text.find(end_marker, start)
if end == -1:
    raise SystemExit("renderCalendar end marker not found")
replacement = '''  const renderCalendar = () => <CalendarWorkspace
    refreshToken={calendarRefreshToken}
    onCreate={() => setEditor({ kind: "calendar" })}
    onEdit={(value) => setEditor({ kind: "calendar", value })}
    onReminder={(subject) => setReminderSubject(subject)}
  />;'''
text = text[:start] + replacement + text[end:]
module.write_text(text, encoding="utf-8")

entry = Path("apps/desktop/tauri-ui/main.tsx")
text = entry.read_text(encoding="utf-8")
if 'app/execution-calendar.css' not in text:
    text = text.replace(
        'import "@/app/execution.css";\n',
        'import "@/app/execution.css";\nimport "@/app/execution-calendar.css";\n',
        1,
    )
entry.write_text(text, encoding="utf-8")
