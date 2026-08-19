import { useEffect, useMemo, useState } from "react";
import { FileText, Plus, Search } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, EmptyState, Input, PageHeader, cn } from "../../components/ui";
import { entities, text } from "../../lib/entities";
import { createNote, type JsonEntity } from "../../services/core";
import { MarkdownEditor } from "./MarkdownEditor";

function noteMarkdown(note: JsonEntity | null): string {
  if (!note) return "";
  return text(note, "contentMarkdown", text(note, "contentText"));
}

function compatibilityHtml(markdown: string): string {
  if (!markdown.trim()) return "";
  const escaped = markdown.replace(/[&<>]/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[character] || character));
  return `<p>${escaped.replace(/\n/g, "<br>")}</p>`;
}

function summaryFromMarkdown(markdown: string): string {
  return markdown
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/!?(\[)([^\]]+)(\])\([^)]*\)/g, "$2")
    .replace(/^\s{0,3}(#{1,6}|>|[-+*]|\d+\.)\s+/gm, "")
    .replace(/[*_~]/g, "")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 180);
}

export function NotesPage() {
  const { state, session, upsert, loading } = useApp();
  const notes = entities(state, "note.note")
    .filter((item) => !item.isArchived)
    .sort((left, right) => right.meta.updatedAt.localeCompare(left.meta.updatedAt));
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [saving, setSaving] = useState(false);

  const filtered = useMemo(() => notes.filter((note) => {
    const haystack = `${text(note, "title")} ${noteMarkdown(note)}`.toLowerCase();
    return haystack.includes(query.trim().toLowerCase());
  }), [notes, query]);
  const selected = notes.find((note) => note.meta.id === selectedId) ?? null;
  const persistedTitle = selected ? text(selected, "title") : "";
  const persistedContent = noteMarkdown(selected);
  const dirty = Boolean(selected ? title !== persistedTitle || content !== persistedContent : title.trim() || content.trim());

  useEffect(() => {
    if (!selected && notes[0] && !selectedId) setSelectedId(notes[0].meta.id);
  }, [notes, selected, selectedId]);

  useEffect(() => {
    if (!selected) return;
    setTitle(text(selected, "title"));
    setContent(noteMarkdown(selected));
  }, [selected?.meta.id]);

  async function save(): Promise<void> {
    if (!session || (!selected && !title.trim() && !content.trim())) return;
    setSaving(true);
    try {
      let next: JsonEntity;
      if (selected) {
        next = {
          ...selected,
          title: title.trim() || null,
          contentText: content,
          contentMarkdown: content,
          contentHtml: compatibilityHtml(content),
          summary: summaryFromMarkdown(content),
        };
      } else {
        next = createNote(session.user.id, session.session.deviceId, title || "新笔记", content);
        next.contentMarkdown = content;
        next.contentText = content;
        next.contentHtml = compatibilityHtml(content);
        next.summary = summaryFromMarkdown(content);
        setSelectedId(next.meta.id);
      }
      await upsert("note.note", next);
    } finally {
      setSaving(false);
    }
  }

  async function selectNote(id: string) {
    if (id === selectedId) return;
    if (dirty) await save();
    setSelectedId(id);
  }

  async function newNote() {
    if (!session) return;
    if (dirty) await save();
    const note = createNote(session.user.id, session.session.deviceId, "新笔记", "# 新笔记\n\n");
    note.contentMarkdown = "# 新笔记\n\n";
    note.contentText = "# 新笔记\n\n";
    await upsert("note.note", note);
    setSelectedId(note.meta.id);
  }

  return <div className="page-shell">
    <PageHeader
      title="笔记"
      description="Markdown 写作工作区：编辑、分屏预览、GFM 任务清单与代码块。桌面采用 Notes List + Editor，移动端保持单列可写。"
      action={<Button onClick={() => void newNote()}><Plus size={16} />新建笔记</Button>}
    />

    <div className="grid min-h-[680px] overflow-hidden rounded-lg border bg-card lg:grid-cols-[280px_minmax(0,1fr)] xl:grid-cols-[300px_minmax(0,1fr)]">
      <aside className="border-b bg-muted/10 lg:border-b-0 lg:border-r">
        <div className="border-b p-3">
          <div className="flex items-center gap-2 rounded-md bg-muted/55 px-2">
            <Search size={15} className="shrink-0 text-muted-foreground" />
            <Input className="h-9 border-0 bg-transparent px-0 shadow-none focus:ring-0" placeholder="搜索标题或 Markdown 正文" value={query} onChange={(event) => setQuery(event.target.value)} />
          </div>
        </div>
        <div className="scrollbar-thin max-h-[280px] overflow-y-auto p-2 lg:max-h-[640px]">
          {filtered.length ? filtered.map((note) => <button
            key={note.meta.id}
            onClick={() => void selectNote(note.meta.id)}
            className={cn("mb-1 w-full rounded-md border border-transparent px-3 py-3 text-left transition-colors", selectedId === note.meta.id ? "border-border bg-background shadow-sm" : "hover:bg-muted")}
          >
            <div className="flex items-start gap-2">
              <FileText size={14} className="mt-0.5 shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-medium">{text(note, "title", "无标题")}</div>
                <div className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{text(note, "summary", summaryFromMarkdown(noteMarkdown(note)) || "空笔记")}</div>
                <div className="mt-2 text-[10px] text-muted-foreground">{new Date(note.meta.updatedAt).toLocaleString("zh-CN", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" })}</div>
              </div>
            </div>
          </button>) : <div className="px-2 py-8 text-center text-xs text-muted-foreground">没有匹配的笔记</div>}
        </div>
      </aside>

      <main className="min-w-0 bg-background p-3 sm:p-5 lg:p-6">
        {selected || !notes.length ? <div className="space-y-3">
          <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
            <Input
              className="h-11 flex-1 border-0 bg-transparent px-1 text-xl font-semibold tracking-[-0.02em] shadow-none focus:ring-0 sm:text-2xl"
              placeholder="无标题"
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              onKeyDown={(event) => {
                if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
                  event.preventDefault();
                  void save();
                }
              }}
            />
            <div className="flex items-center gap-2 px-1 text-xs text-muted-foreground">
              {saving || loading ? <Badge>保存中</Badge> : dirty ? <Badge className="border-warning/30 text-warning">未保存</Badge> : <Badge className="border-success/30 text-success">已保存</Badge>}
            </div>
          </div>
          <MarkdownEditor value={content} onChange={setContent} onSave={save} dirty={dirty} disabled={saving} />
        </div> : <EmptyState title="选择一篇笔记" description="从左侧列表选择笔记，或创建一篇新的 Markdown 笔记。" action={<Button variant="outline" onClick={() => void newNote()}>新建 Markdown 笔记</Button>} />}
      </main>
    </div>
  </div>;
}
