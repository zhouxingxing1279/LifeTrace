import { useMemo, useRef, useState, type KeyboardEvent } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  Bold,
  Braces,
  Code2,
  Columns2,
  Eye,
  Heading2,
  Italic,
  Link,
  List,
  ListChecks,
  ListOrdered,
  Minus,
  PencilLine,
  Quote,
  Save,
  Strikethrough,
} from "lucide-react";
import { Button, cn } from "../../components/ui";
import { applyMarkdownFormat, type MarkdownFormat } from "./markdown";

type EditorMode = "edit" | "split" | "preview";

interface MarkdownEditorProps {
  value: string;
  onChange(value: string): void;
  onSave(): void | Promise<void>;
  dirty?: boolean;
  disabled?: boolean;
}

const toolbar: Array<{ format: MarkdownFormat; label: string; icon: typeof Bold }> = [
  { format: "heading", label: "二级标题", icon: Heading2 },
  { format: "bold", label: "粗体 (Ctrl/Cmd+B)", icon: Bold },
  { format: "italic", label: "斜体 (Ctrl/Cmd+I)", icon: Italic },
  { format: "strike", label: "删除线", icon: Strikethrough },
  { format: "inline-code", label: "行内代码", icon: Code2 },
  { format: "quote", label: "引用", icon: Quote },
  { format: "bullet", label: "无序列表", icon: List },
  { format: "ordered", label: "有序列表", icon: ListOrdered },
  { format: "task", label: "任务列表", icon: ListChecks },
  { format: "link", label: "链接", icon: Link },
  { format: "code-block", label: "代码块", icon: Braces },
  { format: "horizontal-rule", label: "分隔线", icon: Minus },
];

export function MarkdownEditor({ value, onChange, onSave, dirty = false, disabled = false }: MarkdownEditorProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [mode, setMode] = useState<EditorMode>("split");
  const stats = useMemo(() => {
    const trimmed = value.trim();
    return {
      lines: value ? value.split("\n").length : 1,
      chars: value.length,
      words: trimmed ? trimmed.split(/\s+/).length : 0,
    };
  }, [value]);

  function format(formatType: MarkdownFormat) {
    const textarea = textareaRef.current;
    if (!textarea || disabled) return;
    const result = applyMarkdownFormat(value, textarea.selectionStart, textarea.selectionEnd, formatType);
    onChange(result.value);
    requestAnimationFrame(() => {
      textarea.focus();
      textarea.setSelectionRange(result.selectionStart, result.selectionEnd);
    });
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    const modifier = event.metaKey || event.ctrlKey;
    if (modifier && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void onSave();
      return;
    }
    if (modifier && event.key.toLowerCase() === "b") {
      event.preventDefault();
      format("bold");
      return;
    }
    if (modifier && event.key.toLowerCase() === "i") {
      event.preventDefault();
      format("italic");
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      const textarea = event.currentTarget;
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const next = `${value.slice(0, start)}  ${value.slice(end)}`;
      onChange(next);
      requestAnimationFrame(() => {
        textarea.focus();
        textarea.setSelectionRange(start + 2, start + 2);
      });
    }
  }

  return <div className="markdown-editor flex min-h-[540px] flex-col overflow-hidden rounded-lg border bg-background">
    <div className="flex min-h-11 flex-wrap items-center justify-between gap-2 border-b bg-muted/25 px-2 py-1.5">
      <div className="flex flex-wrap items-center gap-0.5" role="toolbar" aria-label="Markdown 格式工具栏">
        {toolbar.map(({ format: formatType, label, icon: Icon }) => <Button
          key={formatType}
          type="button"
          size="icon"
          variant="ghost"
          className="h-8 w-8"
          title={label}
          aria-label={label}
          disabled={disabled || mode === "preview"}
          onClick={() => format(formatType)}
        ><Icon size={15} /></Button>)}
      </div>
      <div className="flex items-center gap-1">
        <div className="flex rounded-md border bg-background p-0.5" aria-label="编辑器视图">
          <button type="button" onClick={() => setMode("edit")} className={cn("flex h-7 items-center gap-1.5 rounded px-2 text-xs text-muted-foreground", mode === "edit" && "bg-muted font-medium text-foreground")}><PencilLine size={13} />编辑</button>
          <button type="button" onClick={() => setMode("split")} className={cn("hidden h-7 items-center gap-1.5 rounded px-2 text-xs text-muted-foreground sm:flex", mode === "split" && "bg-muted font-medium text-foreground")}><Columns2 size={13} />分屏</button>
          <button type="button" onClick={() => setMode("preview")} className={cn("flex h-7 items-center gap-1.5 rounded px-2 text-xs text-muted-foreground", mode === "preview" && "bg-muted font-medium text-foreground")}><Eye size={13} />预览</button>
        </div>
        <Button type="button" size="sm" variant={dirty ? "default" : "outline"} disabled={disabled || !dirty} onClick={() => void onSave()} title="Ctrl/Cmd+S 保存"><Save size={14} />保存</Button>
      </div>
    </div>

    <div className={cn("min-h-0 flex-1", mode === "split" && "grid xl:grid-cols-2")}>
      {mode !== "preview" ? <section className={cn("relative min-h-[480px] bg-background", mode === "split" && "border-b xl:border-b-0 xl:border-r")} aria-label="Markdown 编辑区">
        <textarea
          ref={textareaRef}
          className="markdown-source scrollbar-thin h-full min-h-[480px] w-full resize-none bg-transparent px-5 py-5 font-mono text-[13px] leading-6 text-foreground outline-none placeholder:text-muted-foreground/60 sm:px-6"
          value={value}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={handleKeyDown}
          spellCheck
          disabled={disabled}
          placeholder="# 标题\n\n使用 Markdown 开始记录。\n\n- 支持列表\n- **粗体**、_斜体_、`代码`\n- [ ] 也支持任务清单"
          aria-label="Markdown 正文"
        />
      </section> : null}

      {mode !== "edit" ? <section className="markdown-preview scrollbar-thin min-h-[480px] overflow-y-auto bg-card px-5 py-5 sm:px-7" aria-label="Markdown 预览">
        {value.trim() ? <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={{
            a: ({ ...props }) => <a {...props} target="_blank" rel="noreferrer" />,
            input: ({ ...props }) => <input {...props} disabled />,
          }}
        >{value}</ReactMarkdown> : <div className="flex min-h-60 items-center justify-center text-sm text-muted-foreground">预览会实时显示在这里</div>}
      </section> : null}
    </div>

    <div className="flex min-h-8 items-center justify-between gap-3 border-t bg-muted/20 px-3 text-[11px] text-muted-foreground">
      <div className="flex items-center gap-3"><span>Markdown</span><span>{stats.lines} 行</span><span>{stats.words} 词</span><span>{stats.chars} 字符</span></div>
      <div className="flex items-center gap-2"><span>{dirty ? "未保存更改" : "已保存"}</span><span className="hidden sm:inline">Ctrl/Cmd+S 保存</span></div>
    </div>
  </div>;
}
