import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft, Check, Loader2, Plus, Save, Search, Trash2 } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Button, EmptyState, Input, PageHeader, Textarea, cn } from "../../components/ui";
import { entities, text } from "../../lib/entities";
import { createNote, type JsonEntity } from "../../services/core";

type SaveState = "saved" | "saving" | "dirty" | "error";

function escapeHtml(value: string): string {
  return value.replace(/[&<>]/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[character] ?? character));
}

function notePayload(note: JsonEntity, title: string, content: string): JsonEntity {
  const normalizedTitle = title.trim();
  return {
    ...note,
    title: normalizedTitle || null,
    contentText: content,
    contentMarkdown: content,
    contentHtml: content.trim() ? `<p>${escapeHtml(content).replace(/\n/g, "<br>")}</p>` : "",
    summary: content.trim().slice(0, 160),
  };
}

export function NotesPage() {
  const { state, session, upsert, remove } = useApp();
  const notes = useMemo(
    () => entities(state, "note.note")
      .filter((item) => !item.isArchived)
      .sort((a, b) => b.meta.updatedAt.localeCompare(a.meta.updatedAt)),
    [state],
  );
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [saveState, setSaveState] = useState<SaveState>("saved");
  const [mobileEditing, setMobileEditing] = useState(false);
  const autosaveRef = useRef<number | null>(null);

  const filtered = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase("zh-CN");
    if (!needle) return notes;
    return notes.filter((note) => `${text(note, "title")} ${text(note, "contentText")}`.toLocaleLowerCase("zh-CN").includes(needle));
  }, [notes, query]);
  const selected = notes.find((note) => note.meta.id === selectedId) ?? null;

  useEffect(() => {
    if (!selectedId && notes[0]) setSelectedId(notes[0].meta.id);
    if (selectedId && !selected && notes[0]) setSelectedId(notes[0].meta.id);
    if (selectedId && !selected && !notes.length) setSelectedId(null);
  }, [notes, selected, selectedId]);

  useEffect(() => {
    if (!selected) {
      setTitle("");
      setContent("");
      setSaveState("saved");
      return;
    }
    setTitle(text(selected, "title"));
    setContent(text(selected, "contentText"));
    setSaveState("saved");
  }, [selected?.meta.id]);

  const save = useCallback(async () => {
    if (!session || !selected) return;
    if (autosaveRef.current !== null) window.clearTimeout(autosaveRef.current);
    setSaveState("saving");
    try {
      await upsert("note.note", notePayload(selected, title, content));
      setSaveState("saved");
    } catch {
      setSaveState("error");
      throw new Error("笔记保存失败");
    }
  }, [content, selected, session, title, upsert]);

  useEffect(() => {
    if (!selected || saveState !== "dirty") return;
    if (autosaveRef.current !== null) window.clearTimeout(autosaveRef.current);
    autosaveRef.current = window.setTimeout(() => { void save().catch(() => undefined); }, 800);
    return () => {
      if (autosaveRef.current !== null) window.clearTimeout(autosaveRef.current);
    };
  }, [content, title, selected?.meta.id, saveState, save]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey)) return;
      if (event.key.toLowerCase() === "s") {
        event.preventDefault();
        void save().catch(() => undefined);
      }
      if (event.key.toLowerCase() === "n") {
        event.preventDefault();
        void newNote();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  async function newNote() {
    if (!session) return;
    if (selected && saveState === "dirty") await save().catch(() => undefined);
    const note = createNote(session.user.id, session.session.deviceId, "新笔记", "");
    await upsert("note.note", note);
    setSelectedId(note.meta.id);
    setMobileEditing(true);
  }

  async function selectNote(id: string) {
    if (id === selectedId) { setMobileEditing(true); return; }
    if (selected && saveState === "dirty") await save().catch(() => undefined);
    setSelectedId(id);
    setMobileEditing(true);
  }

  async function deleteSelected() {
    if (!selected) return;
    const currentIndex = notes.findIndex((note) => note.meta.id === selected.meta.id);
    const next = notes[currentIndex + 1] ?? notes[currentIndex - 1] ?? null;
    await remove("note.note", selected.meta.id);
    setSelectedId(next?.meta.id ?? null);
    setMobileEditing(Boolean(next));
  }

  const saveLabel = saveState === "saving" ? "保存中" : saveState === "dirty" ? "未保存" : saveState === "error" ? "保存失败" : "已保存";

  return <div className="page-shell">
    <PageHeader
      title="笔记"
      description="云端同步的双栏笔记工作区；编辑后自动保存，也支持 Ctrl/Cmd+S。"
      action={<Button onClick={() => void newNote()}><Plus size={16}/>新建笔记</Button>}
    />
    <div className="grid min-h-[620px] overflow-hidden rounded-lg border bg-card lg:grid-cols-[320px_minmax(0,1fr)]">
      <aside className={cn("border-b lg:block lg:border-b-0 lg:border-r", mobileEditing && "hidden lg:block")}>
        <div className="flex items-center gap-2 border-b p-3"><Search size={15} className="text-muted-foreground"/><Input className="h-9 border-0 bg-muted/55 focus:ring-0" placeholder="搜索标题或正文" value={query} onChange={(event) => setQuery(event.target.value)}/></div>
        <div className="scrollbar-thin max-h-[560px] overflow-y-auto p-2">
          {filtered.length ? filtered.map((note) => <button key={note.meta.id} onClick={() => void selectNote(note.meta.id)} className={cn("mb-1 w-full rounded-md px-3 py-2.5 text-left", selectedId === note.meta.id ? "bg-accent" : "hover:bg-muted")}>
            <div className="truncate text-sm font-medium">{text(note, "title", "无标题")}</div>
            <div className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{text(note, "summary", text(note, "contentText", "空笔记"))}</div>
            <div className="mt-1 text-[11px] text-muted-foreground">{new Date(note.meta.updatedAt).toLocaleString("zh-CN", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" })}</div>
          </button>) : <EmptyState title={query ? "没有匹配的笔记" : "还没有笔记"} description={query ? "换个关键词试试。" : "创建第一篇笔记开始记录。"}/>} 
        </div>
      </aside>
      <main className={cn("min-w-0 p-4 sm:p-6", !mobileEditing && "hidden lg:block")}>
        {selected ? <>
          <div className="mb-3 flex items-center gap-2">
            <Button className="lg:hidden" variant="ghost" size="icon" aria-label="返回笔记列表" onClick={() => setMobileEditing(false)}><ArrowLeft size={17}/></Button>
            <Input className="h-auto min-w-0 flex-1 border-0 px-0 text-xl font-semibold shadow-none focus:ring-0" placeholder="无标题" value={title} onChange={(event) => { setTitle(event.target.value); setSaveState("dirty"); }}/>
            <span className={cn("hidden items-center gap-1 text-xs sm:flex", saveState === "error" ? "text-destructive" : "text-muted-foreground")}>{saveState === "saving" ? <Loader2 size={13} className="animate-spin"/> : saveState === "saved" ? <Check size={13}/> : null}{saveLabel}</span>
            <Button variant="outline" size="icon" aria-label="保存笔记" onClick={() => void save().catch(() => undefined)}><Save size={15}/></Button>
            <Button variant="ghost" size="icon" aria-label="删除笔记" onClick={() => void deleteSelected()}><Trash2 size={15}/></Button>
          </div>
          <Textarea className="min-h-[500px] resize-none border-0 px-0 leading-7 focus:ring-0" placeholder="开始记录…" value={content} onChange={(event) => { setContent(event.target.value); setSaveState("dirty"); }}/>
        </> : <EmptyState title="选择一篇笔记" description="从列表选择，或创建新笔记。"/>}
      </main>
    </div>
  </div>;
}
