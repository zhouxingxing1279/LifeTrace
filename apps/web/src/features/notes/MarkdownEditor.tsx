import { useRef, type KeyboardEvent } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  Bold,
  Braces,
  Code2,
  Eye,
  Heading2,
  Italic,
  Link,
  List,
  ListChecks,
  ListOrdered,
  Minus,
  PanelLeftClose,
  Quote,
  Strikethrough,
} from "lucide-react";
import { cn } from "../../components/ui";
import { applyMarkdownFormat, type MarkdownFormat } from "./markdown";

export type MarkdownEditorMode = "edit" | "split" | "preview";

interface MarkdownEditorProps {
  value: string;
  onChange(value: string): void;
  mode: MarkdownEditorMode;
  onModeChange(mode: MarkdownEditorMode): void;
  onSave(): void;
}

const tools: Array<{ format: MarkdownFormat; label: string; icon: typeof Bold }> = [
  { format: "heading", label: "二级标题", icon: Heading2 },
  { format: "bold", label: "粗体", icon: Bold },
  { format: "italic", label: "斜体", icon: Italic },
  { format: "strike", label: "删除线", icon: Strikethrough },
  { format: "inline-code", label: "行内代码", icon: Code2 },
  { format: "quote", label: "引用", icon: Quote },
  { format: "unordered", label: "无序列表", icon: List },
  { format: "ordered", label: "有序列表", icon: ListOrdered },
  { format: "task", label: "任务列表", icon: ListChecks },
  { format: "link", label: "链接", icon: Link },
  { format: "code-block", label: "代码块", icon: Braces },
  { format: "horizontal-rule", label: "分割线", icon: Minus },
];

function nextLinePrefix(line: string): string | null {
  const task = line.match(/^(\s*)- \[[ xX]\]\s+/);
  if (task) return `${task[1]}- [ ] `;
  const bullet = line.match(/^(\s*)[-*+]\s+/);
  if (bullet) return `${bullet[1]}- `;
  const ordered = line.match(/^(\s*)(\d+)\.\s+/);
  if (ordered) return `${ordered[1]}${Number(ordered[2]) + 1}. `;
  const quote = line.match(/^(\s*)>\s+/);
  if (quote) return `${quote[1]}> `;
  return null;
}

export function MarkdownEditor({ value, onChange, mode, onModeChange, onSave }: MarkdownEditorProps) {
  const editorRef = useRef<HTMLTextAreaElement>(null);

  function format(format: MarkdownFormat) {
    const editor = editorRef.current;
    if (!editor) return;
    const result = applyMarkdownFormat(value, editor.selectionStart, editor.selectionEnd, format);
    onChange(result.value);
    requestAnimationFrame(() => {
      editor.focus();
      editor.setSelectionRange(result.selectionStart, result.selectionEnd);
    });
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      onSave();
      return;
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "b") {
      event.preventDefault();
      format("bold");
      return;
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "i") {
      event.preventDefault();
      format("italic");
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      const editor = event.currentTarget;
      const start = editor.selectionStart;
      const end = editor.selectionEnd;
      const inserted = "  ";
      onChange(`${value.slice(0, start)}${inserted}${value.slice(end)}`);
      requestAnimationFrame(() => editor.setSelectionRange(start + inserted.length, start + inserted.length));
      return;
    }
    if (event.key === "Enter" && !event.shiftKey && event.currentTarget.selectionStart === event.currentTarget.selectionEnd) {
      const editor = event.currentTarget;
      const cursor = editor.selectionStart;
      const lineStart = value.lastIndexOf("\n", cursor - 1) + 1;
      const line = value.slice(lineStart, cursor);
      const prefix = nextLinePrefix(line);
      if (!prefix) return;
      const contentWithoutPrefix = line.replace(/^\s*(?:[-*+]\s+|- \[[ xX]\]\s+|\d+\.\s+|>\s+)/, "");
      if (!contentWithoutPrefix.trim()) return;
      event.preventDefault();
      const inserted = `\n${prefix}`;
      onChange(`${value.slice(0, cursor)}${inserted}${value.slice(cursor)}`);
      requestAnimationFrame(() => editor.setSelectionRange(cursor + inserted.length, cursor + inserted.length));
    }
  }

  const showEditor = mode !== "preview";
  const showPreview = mode !== "edit";

  return <div className="overflow-hidden rounded-lg border bg-background">
    <div className="flex flex-wrap items-center justify-between gap-2 border-b bg-muted/25 px-2 py-2">
      <div className="flex flex-wrap items-center gap-1">
        {tools.map(({ format: itemFormat, label, icon: Icon }) => <button
          key={itemFormat}
          type="button"
          title={label}
          aria-label={label}
          onClick={() => format(itemFormat)}
          disabled={!showEditor}
          className="inline-flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-accent-foreground disabled:cursor-not-allowed disabled:opacity-35"
        ><Icon size={15}/></button>)}
      </div>
      <div className="flex rounded-md border bg-background p-0.5 text-xs">
        <button type="button" onClick={() => onModeChange("edit")} className={cn("rounded px-2.5 py-1.5", mode === "edit" && "bg-accent font-medium")}>编辑</button>
        <button type="button" onClick={() => onModeChange("split")} className={cn("hidden rounded px-2.5 py-1.5 sm:block", mode === "split" && "bg-accent font-medium")}><PanelLeftClose className="mr-1 inline" size={12}/>分屏</button>
        <button type="button" onClick={() => onModeChange("preview")} className={cn("rounded px-2.5 py-1.5", mode === "preview" && "bg-accent font-medium")}><Eye className="mr-1 inline" size={12}/>预览</button>
      </div>
    </div>

    <div className={cn("grid min-h-[500px]", mode === "split" && "sm:grid-cols-2")}>
      {showEditor ? <div className={cn("min-w-0", mode === "split" && "sm:border-r")}>
        <textarea
          ref={editorRef}
          aria-label="Markdown 编辑区"
          spellCheck={false}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="# 开始写作\n\n支持 Markdown、任务列表、表格、代码块和链接。"
          className="min-h-[500px] w-full resize-none bg-transparent px-5 py-5 font-mono text-[14px] leading-7 text-foreground outline-none"
        />
      </div> : null}
      {showPreview ? <article className="markdown-preview min-w-0 overflow-auto px-6 py-5 text-sm leading-7">
        {value.trim() ? <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={{
            h1: ({children}) => <h1 className="mb-4 mt-2 text-3xl font-bold tracking-tight">{children}</h1>,
            h2: ({children}) => <h2 className="mb-3 mt-7 border-b pb-2 text-2xl font-semibold">{children}</h2>,
            h3: ({children}) => <h3 className="mb-2 mt-6 text-xl font-semibold">{children}</h3>,
            p: ({children}) => <p className="my-3 leading-7">{children}</p>,
            ul: ({children}) => <ul className="my-3 list-disc space-y-1 pl-6">{children}</ul>,
            ol: ({children}) => <ol className="my-3 list-decimal space-y-1 pl-6">{children}</ol>,
            li: ({children, className}) => <li className={cn("leading-7", className)}>{children}</li>,
            blockquote: ({children}) => <blockquote className="my-4 border-l-4 pl-4 text-muted-foreground">{children}</blockquote>,
            a: ({children, href}) => <a className="font-medium text-primary underline underline-offset-4" href={href} target="_blank" rel="noreferrer">{children}</a>,
            code: ({children, className}) => className ? <code className={cn("font-mono text-sm", className)}>{children}</code> : <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-[0.9em]">{children}</code>,
            pre: ({children}) => <pre className="my-4 overflow-x-auto rounded-lg border bg-muted/50 p-4 font-mono text-sm leading-6">{children}</pre>,
            table: ({children}) => <div className="my-4 overflow-x-auto"><table className="w-full border-collapse text-sm">{children}</table></div>,
            th: ({children}) => <th className="border bg-muted/50 px-3 py-2 text-left font-semibold">{children}</th>,
            td: ({children}) => <td className="border px-3 py-2">{children}</td>,
            hr: () => <hr className="my-7 border-border"/>,
            input: (props) => <input {...props} disabled className="mr-2 align-middle"/>,
          }}
        >{value}</ReactMarkdown> : <div className="flex min-h-[420px] items-center justify-center text-muted-foreground">Markdown 预览会显示在这里</div>}
      </article> : null}
    </div>
    <div className="flex items-center justify-between border-t bg-muted/20 px-4 py-2 text-[11px] text-muted-foreground">
      <span>Markdown · GFM</span><span>{value.split(/\s+/).filter(Boolean).length} 词 · {value.length} 字符</span>
    </div>
  </div>;
}
