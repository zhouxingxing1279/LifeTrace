import { useEffect, useMemo, useState } from "react";
import { Plus, Save, Search } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Button, Card, EmptyState, Input, PageHeader, Textarea, cn } from "../../components/ui";
import { entities, text } from "../../lib/entities";
import { createNote, type JsonEntity } from "../../services/core";

export function NotesPage() {
  const { state, session, upsert } = useApp();
  const notes = entities(state,"note.note").filter((item)=>!item.isArchived).sort((a,b)=>b.meta.updatedAt.localeCompare(a.meta.updatedAt));
  const [query,setQuery]=useState(""); const [selectedId,setSelectedId]=useState<string|null>(null); const [title,setTitle]=useState(""); const [content,setContent]=useState("");
  const filtered=useMemo(()=>notes.filter((note)=>`${text(note,"title")} ${text(note,"contentText")}`.toLowerCase().includes(query.toLowerCase())),[notes,query]);
  const selected=notes.find((note)=>note.meta.id===selectedId) ?? null;
  useEffect(()=>{ if (!selected && notes[0] && !selectedId) setSelectedId(notes[0].meta.id); },[notes,selected,selectedId]);
  useEffect(()=>{ if (selected) { setTitle(text(selected,"title")); setContent(text(selected,"contentText")); } },[selected?.meta.id]);
  async function newNote(){ if(!session)return; const note=createNote(session.user.id,session.session.deviceId,"新笔记",""); await upsert("note.note",note); setSelectedId(note.meta.id); }
  async function save(){ if(!session)return; let next:JsonEntity; if(selected){ next={...selected,title:title.trim()||null,contentText:content,contentMarkdown:content,contentHtml:content.trim()?`<p>${content.replace(/[&<>]/g,(c)=>({"&":"&amp;","<":"&lt;",">":"&gt;"}[c]||c)).replace(/\n/g,"<br>")}</p>`:"",summary:content.slice(0,160)}; } else { next=createNote(session.user.id,session.session.deviceId,title,content); setSelectedId(next.meta.id); } await upsert("note.note",next); }
  return <div className="page-shell"><PageHeader title="笔记" description="列表 + 内容工作区，参考 Catalyst workspace / Preline content layout。" action={<Button onClick={()=>void newNote()}><Plus size={16}/>新建笔记</Button>} />
    <div className="grid min-h-[620px] overflow-hidden rounded-lg border bg-card lg:grid-cols-[300px_minmax(0,1fr)]"><aside className="border-b lg:border-b-0 lg:border-r"><div className="flex items-center gap-2 border-b p-3"><Search size={15} className="text-muted-foreground"/><Input className="h-9 border-0 bg-muted/55 focus:ring-0" placeholder="搜索笔记" value={query} onChange={(e)=>setQuery(e.target.value)}/></div><div className="scrollbar-thin max-h-[260px] overflow-y-auto p-2 lg:max-h-[560px]">{filtered.map((note)=><button key={note.meta.id} onClick={()=>setSelectedId(note.meta.id)} className={cn("mb-1 w-full rounded-md px-3 py-2.5 text-left",selectedId===note.meta.id?"bg-accent":"hover:bg-muted")}><div className="truncate text-sm font-medium">{text(note,"title","无标题")}</div><div className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{text(note,"summary",text(note,"contentText","空笔记"))}</div></button>)}</div></aside><main className="min-w-0 p-4 sm:p-6">{selected||!notes.length?<><div className="flex items-center gap-2"><Input className="h-auto border-0 px-0 text-xl font-semibold shadow-none focus:ring-0" placeholder="无标题" value={title} onChange={(e)=>setTitle(e.target.value)}/><Button variant="outline" onClick={()=>void save()}><Save size={15}/>保存</Button></div><Textarea className="mt-4 min-h-[460px] resize-none border-0 px-0 leading-7 focus:ring-0" placeholder="开始记录…" value={content} onChange={(e)=>setContent(e.target.value)}/></>:<EmptyState title="选择一篇笔记" description="从左侧列表选择，或创建新笔记。"/>}</main></div>
  </div>;
}
