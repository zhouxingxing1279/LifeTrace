from pathlib import Path

# Fix JSX/type details in the draft component before frontend validation.
component = Path("apps/desktop/src/components/feature/execution/ExecutionModule.tsx")
text = component.read_text(encoding="utf-8")
text = text.replace('<span><activeTab[2] />{activeTab[1]}</span>', '<span>{activeTab[1]}</span>')
text = text.replace(
    'save: (input: { name: string; description?: string; status?: string; color?: string }) => Promise<void>;',
    'save: (input: { name: string; description?: string; status?: ExecutionProject["status"]; color?: string }) => Promise<void>;',
)
text = text.replace(
    'const [status, setStatus] = useState(value?.status || "active");',
    'const [status, setStatus] = useState<ExecutionProject["status"]>(value?.status || "active");',
)
text = text.replace(
    'onChange={(e) => setStatus(e.target.value)}><option value="active">进行中</option>',
    'onChange={(e) => setStatus(e.target.value as ExecutionProject["status"])}><option value="active">进行中</option>',
)
component.write_text(text, encoding="utf-8")

# Add one first-level Execution entry; six subviews remain internal to ExecutionModule.
nav = Path("apps/desktop/src/components/layout/navigation.ts")
text = nav.read_text(encoding="utf-8")
if "ListChecks" not in text:
    text = text.replace("  Languages,\n", "  Languages,\n  ListChecks,\n", 1)
if '| "execution"' not in text:
    text = text.replace('  | "dashboard"\n', '  | "dashboard"\n  | "execution"\n', 1)
text = text.replace(
    '    items: [{ id: "dashboard", label: "今天", icon: Home }],',
    '    items: [\n      { id: "dashboard", label: "今天", icon: Home },\n      { id: "execution", label: "执行", icon: ListChecks },\n    ],',
    1,
)
if 'execution: "执行中心"' not in text:
    text = text.replace('  dashboard: "今天",\n', '  dashboard: "今天",\n  execution: "执行中心",\n', 1)
nav.write_text(text, encoding="utf-8")

shell = Path("apps/desktop/src/components/HengXuShell.tsx")
text = shell.read_text(encoding="utf-8")
if 'feature/execution/ExecutionModule' not in text:
    text = text.replace(
        'import DailyEnglish from "@/src/components/english/DailyEnglish";\n',
        'import DailyEnglish from "@/src/components/english/DailyEnglish";\nimport ExecutionModule from "@/src/components/feature/execution/ExecutionModule";\n',
        1,
    )
if 'view === "execution"' not in text:
    text = text.replace(
        '        {view === "assistant" ? (\n',
        '        {view === "execution" ? <ExecutionModule /> : null}\n        {view === "assistant" ? (\n',
        1,
    )
shell.write_text(text, encoding="utf-8")

entry = Path("apps/desktop/tauri-ui/main.tsx")
text = entry.read_text(encoding="utf-8")
if 'app/execution.css' not in text:
    anchor = 'import "@/app/desktop-workspace.css";\n'
    if anchor in text:
        text = text.replace(anchor, anchor + 'import "@/app/execution.css";\n', 1)
    else:
        last_style = 'import "@/app/'
        lines = text.splitlines()
        insert_at = max(i for i, line in enumerate(lines) if line.startswith(last_style)) + 1
        lines.insert(insert_at, 'import "@/app/execution.css";')
        text = "\n".join(lines) + "\n"
entry.write_text(text, encoding="utf-8")

# Focused frontend contract tests for local datetime conversion helpers.
test = Path("apps/desktop/tests/execution-api.test.ts")
test.write_text(r'''import assert from "node:assert/strict";
import test from "node:test";
import {
  localDateTimeToRfc3339,
  rfc3339ToLocalDateTime,
} from "../src/services/executionApi";

test("execution datetime helper returns RFC3339 UTC for valid local input", () => {
  const value = localDateTimeToRfc3339("2026-08-09T10:30");
  assert.ok(value);
  assert.match(value, /^2026-08-09T\d{2}:30:00\.000Z$/);
});

test("execution datetime helper rejects blank or invalid input", () => {
  assert.equal(localDateTimeToRfc3339(""), null);
  assert.equal(localDateTimeToRfc3339("not-a-date"), null);
});

test("execution datetime formatter is reversible to a datetime-local string", () => {
  const local = rfc3339ToLocalDateTime("2026-08-09T02:30:00.000Z");
  assert.match(local, /^2026-08-09T\d{2}:30$/);
  assert.equal(rfc3339ToLocalDateTime(null), "");
});
''', encoding="utf-8")
