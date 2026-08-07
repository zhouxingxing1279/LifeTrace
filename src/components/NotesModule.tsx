"use client";
/* eslint-disable react-hooks/set-state-in-effect, react-hooks/preserve-manual-memoization */

import { useCallback, useEffect, useRef, useState } from "react";
import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Link from "@tiptap/extension-link";
import Image from "@tiptap/extension-image";
import TaskList from "@tiptap/extension-task-list";
import TaskItem from "@tiptap/extension-task-item";
import Placeholder from "@tiptap/extension-placeholder";
import CodeBlockLowlight from "@tiptap/extension-code-block-lowlight";
import { common, createLowlight } from "lowlight";
import DOMPurify from "dompurify";
import TurndownService from "turndown";
import {
  Archive, ArchiveRestore, Bold, Braces, ChevronLeft, ChevronRight, Copy, Download,
  File, FileJson, FileText, FileUp, Folder, FolderPlus, Heading1, Heading2,
  History, ImagePlus, Italic, Link as LinkIcon, List, ListChecks, ListOrdered,
  MoreHorizontal, Paperclip, Pin, Plus, Quote, Redo2, RotateCcw, Save, Search,
  Star, Strikethrough, Tag, Trash2, Undo2, Unlink, X,
} from "lucide-react";
import { noteApi, type NoteInputValue } from "@/src/services/noteApi";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { Note, NoteFolder, NoteRelation, NoteRevision, NoteTag, NoteType } from "@/src/types";
import MoreMenu from "@/src/ui/menu/MoreMenu";
import type { AppAction } from "@/src/ui/actions/types";

const lowlight=createLowlight(common);
const turndown=new TurndownService({headingStyle:"atx",bulletListMarker:"-",codeBlockStyle:"fenced"});
const labels:Record<NoteType,string>={quick:"快速记录",document:"普通笔记",daily:"每日复盘",habit_log:"习惯记录",workout_review:"训练复盘",expense_note:"消费笔记",weekly_review:"周总结",monthly_review:"月总结"};
const emptyJson={type:"doc",content:[{type:"paragraph"}]};
const cleanSummary=(text:string)=>text.trim().replace(/\s+/g," ").slice(0,160);
const titleOf=(note:Pick<Note,"title"|"summary">)=>note.title?.trim()||note.summary?.trim().split("\n")[0]||"无标题笔记";
const formatTime=(value:string)=>new Intl.DateTimeFormat("zh-CN",{month:"short",day:"numeric",hour:"2-digit",minute:"2-digit"}).format(new Date(value));
const notify=(message:string)=>window.dispatchEvent(new CustomEvent("hengxu-toast",{detail:message}));

function useDebounced<T>(value:T,delay:number){
  const [result,setResult]=useState(value);
  useEffect(()=>{const timer=window.setTimeout(()=>setResult(value),delay);return()=>window.clearTimeout(timer)},[value,delay]);
  return result;
}

function EditorButton({title,active,onClick,children}:{title:string;active?:boolean;onClick:()=>void;children:React.ReactNode}){
  return <button type="button" title={title} aria-label={title} className={active?"active":""} onMouseDown={event=>{event.preventDefault();onClick()}}>{children}</button>;
}

function NoteEditor({note,folders,tags,onSaved,onListChanged,trashMode,registerSave}:{note:Note;folders:NoteFolder[];tags:NoteTag[];onSaved:(note:Note)=>void;onListChanged:()=>void;trashMode:boolean;registerSave:(save:(revision?:boolean)=>Promise<Note|null>)=>()=>void}){
  const store=useLifeStore();
  const [draft,setDraft]=useState(note);
  const [dirty,setDirty]=useState(false);
  const [status,setStatus]=useState<"saved"|"dirty"|"saving"|"failed">("saved");
  const [revisions,setRevisions]=useState<NoteRevision[]>([]);
  const [historyOpen,setHistoryOpen]=useState(false);
  const [menuOpen,setMenuOpen]=useState(false);
  const saveLock=useRef(false);
  const editor=useEditor({
    immediatelyRender:false,
    extensions:[
      StarterKit.configure({codeBlock:false,link:false}),
      Link.configure({openOnClick:false,HTMLAttributes:{rel:"noopener noreferrer nofollow",target:"_blank"}}),
      Image.configure({allowBase64:false}),
      TaskList,TaskItem.configure({nested:true}),
      Placeholder.configure({placeholder:"开始写下你的想法…"}),
      CodeBlockLowlight.configure({lowlight}),
    ],
    content:note.contentJson,
    onUpdate:({editor:instance})=>{
      const html=DOMPurify.sanitize(instance.getHTML(),{USE_PROFILES:{html:true}});
      const text=instance.getText({blockSeparator:"\n"});
      setDraft(current=>({...current,contentJson:instance.getJSON() as Record<string,unknown>,contentHtml:html,contentText:text,contentMarkdown:turndown.turndown(html),summary:cleanSummary(text)}));
      setDirty(true);setStatus("dirty");
    },
  });

  const save=useCallback(async(createRevision=false)=>{
    if(saveLock.current||!dirty&&!createRevision)return draft;
    saveLock.current=true;setStatus("saving");
    try{
      const value:NoteInputValue&{id:string}={
        id:draft.id,title:draft.title,noteType:draft.noteType,folderId:draft.folderId,
        contentJson:draft.contentJson,contentHtml:DOMPurify.sanitize(draft.contentHtml),
        contentText:draft.contentText,contentMarkdown:draft.contentMarkdown,summary:draft.summary,
        isPinned:draft.isPinned,isFavorite:draft.isFavorite,isArchived:draft.isArchived,
        tagIds:draft.tags.map(item=>item.id),relations:draft.relations,createRevision,
      };
      const saved=await noteApi.update(value);
      setDraft(saved);setDirty(false);setStatus("saved");onSaved(saved);return saved;
    }catch(error){setStatus("failed");notify(error instanceof Error?error.message:"保存失败");return null}
    finally{saveLock.current=false}
  },[dirty,draft,onSaved]);

  useEffect(()=>registerSave(save),[registerSave,save]);

  useEffect(()=>{if(!dirty)return;const timer=window.setTimeout(()=>void save(false),800);return()=>window.clearTimeout(timer)},[draft,dirty,save]);
  useEffect(()=>{
    const beforeUnload=()=>{if(!dirty)return;const payload={action:"update",note:{...draft,tagIds:draft.tags.map(x=>x.id),createRevision:false}};navigator.sendBeacon?.("/api/notes",new Blob([JSON.stringify(payload)],{type:"application/json"}))};
    window.addEventListener("beforeunload",beforeUnload);return()=>window.removeEventListener("beforeunload",beforeUnload);
  },[dirty,draft]);

  const patch=(value:Partial<Note>)=>{setDraft(current=>({...current,...value}));setDirty(true);setStatus("dirty")};
  const toggleTag=(tag:NoteTag)=>patch({tags:draft.tags.some(x=>x.id===tag.id)?draft.tags.filter(x=>x.id!==tag.id):[...draft.tags,tag]});
  const loadHistory=async()=>{setRevisions(await noteApi.revisions(note.id));setHistoryOpen(true)};
  const action=async(kind:"trash"|"restore"|"delete"|"duplicate")=>{
    if(kind==="delete"&&!confirm("永久删除后无法恢复，确定继续吗？"))return;
    if(kind==="trash")await noteApi.trash(note.id);
    if(kind==="restore")await noteApi.restore(note.id);
    if(kind==="delete"){
      for(const attachment of note.attachments??[])await window.noteApi?.deleteAttachment(note.id,attachment.fileName);
      await noteApi.delete(note.id);
    }
    if(kind==="duplicate")await noteApi.duplicate(note.id);
    notify(kind==="restore"?"笔记已恢复":kind==="duplicate"?"已创建副本":"操作已完成");onListChanged();
  };
  const exportNote=async(format:"md"|"html"|"json")=>{
    const content=format==="md"?draft.contentMarkdown:format==="html"?`<!doctype html><meta charset="utf-8"><title>${titleOf(draft)}</title><article>${DOMPurify.sanitize(draft.contentHtml)}</article>`:JSON.stringify(draft,null,2);
    if(window.noteApi){const result=await window.noteApi.exportNote({format,title:titleOf(draft),content});if(!result.ok)notify(result.error||"导出失败")}
    else{const blob=new Blob([content],{type:"text/plain;charset=utf-8"});const anchor=document.createElement("a");anchor.href=URL.createObjectURL(blob);anchor.download=`${titleOf(draft)}.${format}`;anchor.click();URL.revokeObjectURL(anchor.href)}
  };
  useEffect(()=>{
    const key=(event:KeyboardEvent)=>{if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==="s"){event.preventDefault();void save(true)}};
    window.addEventListener("keydown",key);
    const dispose=window.noteApi?.onCommand(command=>{if(command==="save")void save(true);if(command==="favorite")patch({isFavorite:!draft.isFavorite});if(command==="pin")patch({isPinned:!draft.isPinned});if(command==="export")void exportNote("md");if(command==="trash")void action("trash")});
    return()=>{window.removeEventListener("keydown",key);dispose?.()};
  });
  const attach=async()=>{
    if(!window.noteApi){notify("附件仅在 Electron 桌面端可用");return}
    const result=await window.noteApi.selectAttachment(note.id);if(!result.ok||!result.file){if(result.error)notify(result.error);return}
    await noteApi.recordAttachment(result.file);notify("附件已添加");onSaved(await noteApi.get(note.id));
  };
  const relationOptions=[
    ...store.activities.map(x=>({type:"habit",id:x.id,label:`习惯 · ${x.name}`})),
    ...store.workoutHistory.slice(0,20).map(x=>({type:"workout",id:x.id,label:`训练 · ${x.name}`})),
    ...store.transactions.slice(0,30).map(x=>({type:"transaction",id:x.id,label:`账单 · ${x.counterparty||x.category} · ¥${x.amount}`})),
  ];
  const addRelation=(value:string)=>{if(!value)return;const [entityType,entityId]=value.split(":");if(draft.relations.some(x=>x.entityType===entityType&&x.entityId===entityId))return;patch({relations:[...draft.relations,{id:crypto.randomUUID(),noteId:note.id,entityType:entityType as NoteRelation["entityType"],entityId,relationType:"reference",createdAt:new Date().toISOString()}]})};
  const editorActions:AppAction<Note>[]=[
    {id:"duplicate",label:"复制笔记",icon:Copy,group:"primary",execute:()=>action("duplicate")},
    {id:"export-md",label:"导出 Markdown",icon:FileText,group:"related",execute:()=>exportNote("md")},
    {id:"export-html",label:"导出 HTML",icon:File,group:"related",execute:()=>exportNote("html")},
    {id:"export-json",label:"导出 JSON",icon:FileJson,group:"related",execute:()=>exportNote("json")},
    {id:"trash",label:"移到回收站",icon:Trash2,group:"danger",danger:true,execute:()=>action("trash")},
  ];

  if(trashMode)return <section className="nt-editor nt-trash-preview"><div><Trash2/><h2>{titleOf(draft)}</h2><p>{draft.summary||"这篇笔记没有摘要。"}</p><small>删除于 {draft.deletedAt?formatTime(draft.deletedAt):"未知时间"}</small><footer><button className="hx-btn primary" onClick={()=>void action("restore")}><ArchiveRestore/>恢复笔记</button><button className="hx-btn secondary danger" onClick={()=>void action("delete")}><Trash2/>永久删除</button></footer></div></section>;

  return <section className="nt-editor">
    <header className="nt-editor-head">
      <div className={`nt-save-state ${status}`}>{status==="saving"?"正在保存":status==="dirty"?"未保存":status==="failed"?"保存失败":"已保存"}</div>
      <div>
        <button className={draft.isFavorite?"active":""} title="收藏" onClick={()=>patch({isFavorite:!draft.isFavorite})}><Star/></button>
        <button className={draft.isPinned?"active":""} title="置顶" onClick={()=>patch({isPinned:!draft.isPinned})}><Pin/></button>
        <button title="版本历史" onClick={()=>void loadHistory()}><History/></button>
        <button title="立即保存" onClick={()=>void save(true)}><Save/></button>
        <MoreMenu actions={editorActions} context={draft} label="更多笔记操作" buttonClassName="nt-more-button"/>
      </div>
    </header>
    <div className="nt-editor-scroll">
      <input className="nt-title" value={draft.title??""} onChange={e=>patch({title:e.target.value||null})} placeholder={draft.noteType==="quick"?"快速记录无需标题":"无标题笔记"}/>
      <div className="nt-formatbar">
        <EditorButton title="撤销" onClick={()=>editor?.chain().focus().undo().run()}><Undo2/></EditorButton><EditorButton title="重做" onClick={()=>editor?.chain().focus().redo().run()}><Redo2/></EditorButton>
        <i/>
        <EditorButton title="一级标题" active={editor?.isActive("heading",{level:1})} onClick={()=>editor?.chain().focus().toggleHeading({level:1}).run()}><Heading1/></EditorButton>
        <EditorButton title="二级标题" active={editor?.isActive("heading",{level:2})} onClick={()=>editor?.chain().focus().toggleHeading({level:2}).run()}><Heading2/></EditorButton>
        <EditorButton title="加粗" active={editor?.isActive("bold")} onClick={()=>editor?.chain().focus().toggleBold().run()}><Bold/></EditorButton>
        <EditorButton title="斜体" active={editor?.isActive("italic")} onClick={()=>editor?.chain().focus().toggleItalic().run()}><Italic/></EditorButton>
        <EditorButton title="删除线" active={editor?.isActive("strike")} onClick={()=>editor?.chain().focus().toggleStrike().run()}><Strikethrough/></EditorButton>
        <EditorButton title="代码块" active={editor?.isActive("codeBlock")} onClick={()=>editor?.chain().focus().toggleCodeBlock().run()}><Braces/></EditorButton>
        <EditorButton title="引用" active={editor?.isActive("blockquote")} onClick={()=>editor?.chain().focus().toggleBlockquote().run()}><Quote/></EditorButton>
        <EditorButton title="无序列表" active={editor?.isActive("bulletList")} onClick={()=>editor?.chain().focus().toggleBulletList().run()}><List/></EditorButton>
        <EditorButton title="有序列表" active={editor?.isActive("orderedList")} onClick={()=>editor?.chain().focus().toggleOrderedList().run()}><ListOrdered/></EditorButton>
        <EditorButton title="待办列表" active={editor?.isActive("taskList")} onClick={()=>editor?.chain().focus().toggleTaskList().run()}><ListChecks/></EditorButton>
        <EditorButton title="链接" onClick={()=>{const href=prompt("输入链接地址","https://");if(href)editor?.chain().focus().extendMarkRange("link").setLink({href}).run()}}><LinkIcon/></EditorButton>
        <EditorButton title="移除链接" onClick={()=>editor?.chain().focus().unsetLink().run()}><Unlink/></EditorButton>
        <EditorButton title="图片链接" onClick={()=>{const src=prompt("输入图片的 HTTPS 地址");if(src?.startsWith("https://"))editor?.chain().focus().setImage({src}).run()}}><ImagePlus/></EditorButton>
        <EditorButton title="清除格式" onClick={()=>editor?.chain().focus().unsetAllMarks().clearNodes().run()}><RotateCcw/></EditorButton>
      </div>
      <EditorContent editor={editor} className="nt-prose"/>
      <div className="nt-meta">
        <label><Folder/>文件夹<select value={draft.folderId??""} onChange={e=>patch({folderId:e.target.value||null})}><option value="">未分类</option>{folders.map(folder=><option key={folder.id} value={folder.id}>{folder.name}</option>)}</select></label>
        <label><FileText/>类型<select value={draft.noteType} onChange={e=>patch({noteType:e.target.value as NoteType})}>{Object.entries(labels).map(([id,label])=><option key={id} value={id}>{label}</option>)}</select></label>
        <div className="nt-tag-field"><span><Tag/>标签</span><div>{tags.map(tag=><button key={tag.id} className={draft.tags.some(x=>x.id===tag.id)?"active":""} style={{"--tag-color":tag.color} as React.CSSProperties} onClick={()=>toggleTag(tag)}>{tag.name}</button>)}</div></div>
        <label><LinkIcon/>关联数据<select value="" onChange={e=>addRelation(e.target.value)}><option value="">添加习惯、训练或账单…</option>{relationOptions.map(x=><option key={`${x.type}:${x.id}`} value={`${x.type}:${x.id}`}>{x.label}</option>)}</select></label>
        {draft.relations.length>0&&<div className="nt-relations">{draft.relations.map(rel=><span key={rel.id}>{rel.entityType} · {rel.entityId.slice(0,8)}<button onClick={()=>patch({relations:draft.relations.filter(x=>x.id!==rel.id)})}><X/></button></span>)}</div>}
        <div className="nt-attachments"><header><span><Paperclip/>附件</span><button onClick={()=>void attach()}><Plus/>添加附件</button></header>{draft.attachments?.map(file=><article key={file.id}><File/><div><strong>{file.originalName}</strong><small>{(file.fileSize/1024).toFixed(1)} KB</small></div><button onClick={()=>void window.noteApi?.openAttachment(note.id,file.fileName)}>打开</button><button onClick={()=>void window.noteApi?.showAttachment(note.id,file.fileName)}>位置</button><button className="danger" onClick={async()=>{if(!confirm("删除这个附件吗？"))return;await window.noteApi?.deleteAttachment(note.id,file.fileName);await noteApi.deleteAttachment(file.id);onSaved(await noteApi.get(note.id))}}><Trash2/></button></article>)}</div>
        <footer>创建于 {formatTime(draft.createdAt)} · 更新于 {formatTime(draft.updatedAt)} · 版本 {draft.version}</footer>
      </div>
    </div>
    {historyOpen&&<aside className="nt-history"><header><div><History/><strong>版本历史</strong></div><button onClick={()=>setHistoryOpen(false)}><X/></button></header>{revisions.length===0?<p>手动保存后会在这里保留快照。</p>:revisions.map(revision=><article key={revision.id}><div><strong>版本 {revision.version}</strong><small>{formatTime(revision.createdAt)}</small></div><p>{revision.contentMarkdown.slice(0,120)||"空白版本"}</p><button onClick={async()=>{if(!confirm("恢复此版本？当前内容会先保存为快照。"))return;const restored=await noteApi.restoreRevision(revision.id);onSaved(restored);setDraft(restored);editor?.commands.setContent(restored.contentJson,{emitUpdate:false});setHistoryOpen(false);notify("历史版本已恢复")}}>恢复</button></article>)}</aside>}
  </section>;
}

export default function NotesModule(){
  const [scope,setScope]=useState("all");
  const [folderId,setFolderId]=useState("");
  const [tagId,setTagId]=useState("");
  const [query,setQuery]=useState("");
  const [sort,setSort]=useState("updated_desc");
  const [notes,setNotes]=useState<Note[]>([]);
  const [folders,setFolders]=useState<NoteFolder[]>([]);
  const [tags,setTags]=useState<NoteTag[]>([]);
  const [selected,setSelected]=useState<Note|null>(null);
  const [loading,setLoading]=useState(true);
  const [leftCollapsed,setLeftCollapsed]=useState(false);
  const [listCollapsed,setListCollapsed]=useState(false);
  const [leftWidth,setLeftWidth]=useState(210);
  const [listWidth,setListWidth]=useState(330);
  const [commandOpen,setCommandOpen]=useState(false);
  const saveBeforeSwitch=useRef<((revision?:boolean)=>Promise<Note|null>)|null>(null);
  const debouncedQuery=useDebounced(query,300);
  const searchRef=useRef<HTMLInputElement>(null);
  useEffect(()=>{
    setLeftWidth(Math.min(300,Math.max(170,Number(window.localStorage.getItem("lifetrace:notes-left-width"))||210)));
    setListWidth(Math.min(520,Math.max(260,Number(window.localStorage.getItem("lifetrace:notes-list-width"))||330)));
    setLeftCollapsed(window.localStorage.getItem("lifetrace:notes-left-collapsed")==="1");
    setListCollapsed(window.localStorage.getItem("lifetrace:notes-list-collapsed")==="1");
  },[]);
  const resize=(column:"left"|"list",event:React.PointerEvent)=>{
    event.preventDefault();const start=event.clientX;const original=column==="left"?leftWidth:listWidth;let latest=original;
    const move=(next:PointerEvent)=>{latest=Math.round(Math.min(column==="left"?300:520,Math.max(column==="left"?170:260,original+next.clientX-start)));if(column==="left")setLeftWidth(latest);else setListWidth(latest)};
    const up=()=>{window.removeEventListener("pointermove",move);window.removeEventListener("pointerup",up);window.localStorage.setItem(`lifetrace:notes-${column}-width`,String(latest))};
    window.addEventListener("pointermove",move);window.addEventListener("pointerup",up);
  };
  useEffect(()=>{window.localStorage.setItem("lifetrace:notes-left-collapsed",leftCollapsed?"1":"0");window.localStorage.setItem("lifetrace:notes-list-collapsed",listCollapsed?"1":"0")},[leftCollapsed,listCollapsed]);

  const loadMeta=useCallback(async()=>{const meta=await noteApi.meta();setFolders(meta.folders);setTags(meta.tags)},[]);
  const loadList=useCallback(async(preferId?:string)=>{
    setLoading(true);
    try{
      const list=await noteApi.list({q:debouncedQuery,scope,folderId,tagId,sort,limit:150});setNotes(list);
      const target=preferId||selected?.id||window.localStorage.getItem("lifetrace:last-note")||list[0]?.id;
      if(target&&list.some(item=>item.id===target)){const full=await noteApi.get(target);setSelected(full);window.localStorage.setItem("lifetrace:last-note",target)}
      else setSelected(list[0]?await noteApi.get(list[0].id):null);
    }catch(error){notify(error instanceof Error?error.message:"笔记加载失败")}finally{setLoading(false)}
  },[debouncedQuery,folderId,scope,selected?.id,sort,tagId]);
  useEffect(()=>{void loadMeta()},[loadMeta]);
  useEffect(()=>{void loadList()},[debouncedQuery,folderId,scope,sort,tagId]); // eslint-disable-line react-hooks/exhaustive-deps

  const open=async(id:string)=>{if(selected?.id===id)return;await saveBeforeSwitch.current?.(false);const full=await noteApi.get(id);setSelected(full);window.localStorage.setItem("lifetrace:last-note",id)};
  const create=useCallback(async(type:NoteType="document",seed?:Partial<NoteInputValue>)=>{
    const created=await noteApi.create({title:null,noteType:type,folderId:null,contentJson:emptyJson,contentHtml:"<p></p>",contentText:"",contentMarkdown:"",summary:"",isPinned:false,isFavorite:false,isArchived:false,tagIds:[],relations:[],...seed});
    setScope("all");setFolderId("");setTagId("");await loadList(created.id);setSelected(created);notify(type==="quick"?"快速记录已创建":"新笔记已创建");
  },[loadList]);
  const importMarkdown=useCallback(async()=>{
    if(!window.noteApi){notify("Markdown 导入仅在 Electron 桌面端可用");return}
    const result=await window.noteApi.importMarkdown();if(!result.ok||result.canceled)return;if(result.error){notify(result.error);return}
    const content=result.content??"";const lines=content.split(/\r?\n/);const contentJson={type:"doc",content:lines.map(line=>({type:"paragraph",content:line?[{type:"text",text:line}]:undefined}))};
    const escaped=lines.map(line=>`<p>${line.replace(/[&<>"]/g,char=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}[char]!) )||"<br>"}</p>`).join("");
    await create("document",{title:result.title||null,contentJson,contentHtml:escaped,contentText:content,contentMarkdown:content,summary:cleanSummary(content)});
  },[create]);
  const refresh=useCallback(()=>void loadList(),[loadList]);

  useEffect(()=>{
    const handler=(event:KeyboardEvent)=>{
      if(event.key==="Escape"){setCommandOpen(false);return}
      if(!(event.ctrlKey||event.metaKey))return;
      if(event.key.toLowerCase()==="p"){event.preventDefault();setCommandOpen(true);return}
      if(event.key.toLowerCase()==="n"){event.preventDefault();void create(event.shiftKey?"quick":"document")}
      if(event.shiftKey&&event.key.toLowerCase()==="f"){event.preventDefault();searchRef.current?.focus()}
    };
    window.addEventListener("keydown",handler);
    const dispose=window.noteApi?.onCommand(command=>{if(command==="new")void create("document");if(command==="quick")void create("quick");if(command==="import")void importMarkdown();if(command==="search")searchRef.current?.focus()});
    return()=>{window.removeEventListener("keydown",handler);dispose?.()};
  },[create,importMarkdown]);

  const makeFolder=async()=>{const name=prompt("文件夹名称");if(!name)return;await noteApi.saveFolder({name,icon:"folder",color:"#2a7a5e",sortOrder:folders.length});await loadMeta()};
  const makeTag=async()=>{const name=prompt("标签名称");if(!name)return;await noteApi.saveTag({name,color:"#5f7d70"});await loadMeta()};
  const manageFolder=async(folder:NoteFolder)=>{const name=prompt("修改文件夹名称；留空并确定可删除（其中笔记会移到未分类）",folder.name);if(name===null)return;if(!name.trim()){if(confirm(`删除文件夹“${folder.name}”？笔记不会被删除。`))await noteApi.deleteFolder(folder.id)}else await noteApi.saveFolder({...folder,name:name.trim()});await loadMeta();await loadList()};
  const manageTag=async(tag:NoteTag)=>{const name=prompt("修改标签名称；留空并确定可删除",tag.name);if(name===null)return;if(!name.trim()){if(confirm(`删除标签“${tag.name}”？笔记不会被删除。`))await noteApi.deleteTag(tag.id)}else await noteApi.saveTag({...tag,name:name.trim()});await loadMeta();await loadList()};
  const restoreTrash=async()=>{for(const note of notes)await noteApi.restore(note.id);notify(`已恢复 ${notes.length} 篇笔记`);await loadList()};
  const emptyTrash=async()=>{if(!confirm(`永久删除回收站中的 ${notes.length} 篇笔记？此操作无法撤销。`))return;for(const item of notes){const full=await noteApi.get(item.id);for(const file of full.attachments??[])await window.noteApi?.deleteAttachment(item.id,file.fileName);await noteApi.delete(item.id)}notify("回收站已清空");await loadList()};
  const toggleSelected=async(field:"isFavorite"|"isPinned")=>{if(!selected)return;const saved=await noteApi.update({...selected,[field]:!selected[field],tagIds:selected.tags.map(x=>x.id),relations:selected.relations,createRevision:false});setSelected(saved);setCommandOpen(false);await loadList(saved.id)};
  const exportSelected=async()=>{if(!selected)return;const content=selected.contentMarkdown||selected.contentText;if(window.noteApi)await window.noteApi.exportNote({format:"md",title:titleOf(selected),content});setCommandOpen(false)};
  const choose=(nextScope:string,nextFolder="",nextTag="")=>{setScope(nextScope);setFolderId(nextFolder);setTagId(nextTag)};

  return <><div className={`nt-workspace ${leftCollapsed?"left-collapsed":""} ${listCollapsed?"list-collapsed":""}`} style={{gridTemplateColumns:`${leftCollapsed?0:leftWidth}px ${listCollapsed?0:listWidth}px minmax(460px,1fr)`}}>
    <aside className="nt-sidebar">
      <header><strong>笔记库</strong><button onClick={()=>setLeftCollapsed(true)} title="折叠分类"><ChevronLeft/></button></header>
      <nav>
        {[["quick","快速记录",FileText],["all","全部笔记",File],["recent","最近编辑",History],["favorite","收藏",Star],["pinned","置顶",Pin],["archived","归档",Archive],["trash","回收站",Trash2]].map(([id,label,Icon])=><button key={String(id)} className={scope===id&&!folderId&&!tagId?"active":""} onClick={()=>choose(String(id))}><span><Icon/>{String(label)}</span>{id==="all"&&<b>{notes.length}</b>}</button>)}
      </nav>
      <section><header><span>文件夹 · 右键管理</span><button onClick={()=>void makeFolder()}><FolderPlus/></button></header>{folders.map(folder=><button key={folder.id} className={folderId===folder.id?"active":""} onClick={()=>choose("all",folder.id)} onContextMenu={event=>{event.preventDefault();void manageFolder(folder)}}><span><i style={{background:folder.color}}/><Folder/>{folder.name}</span></button>)}</section>
      <section><header><span>标签 · 右键管理</span><button onClick={()=>void makeTag()}><Plus/></button></header><div className="nt-sidebar-tags">{tags.map(tag=><button key={tag.id} className={tagId===tag.id?"active":""} onClick={()=>choose("all","",tag.id)} onContextMenu={event=>{event.preventDefault();void manageTag(tag)}}><i style={{background:tag.color}}/>{tag.name}</button>)}</div></section>
    </aside>
    {!leftCollapsed&&<i className="nt-resizer left" style={{left:leftWidth-3}} onPointerDown={event=>resize("left",event)}/>}
    {leftCollapsed&&<button className="nt-expand left" onClick={()=>setLeftCollapsed(false)} title="展开分类"><ChevronRight/></button>}
    <section className="nt-list">
      <header>
        <div className="nt-search"><Search/><input ref={searchRef} value={query} onChange={e=>setQuery(e.target.value)} placeholder="搜索标题、正文、标签…"/>{query&&<button onClick={()=>setQuery("")}><X/></button>}</div>
        <div>{scope==="trash"&&notes.length>0&&<><button title="恢复全部" onClick={()=>void restoreTrash()}><ArchiveRestore/></button><button title="清空回收站" onClick={()=>void emptyTrash()}><Trash2/></button></>}<select value={sort} onChange={e=>setSort(e.target.value)}><option value="updated_desc">最近编辑</option><option value="created_desc">最近创建</option><option value="created_asc">最早创建</option><option value="title_asc">标题 A–Z</option><option value="title_desc">标题 Z–A</option></select><button title="导入 Markdown" onClick={()=>void importMarkdown()}><FileUp/></button><button title="折叠列表" onClick={()=>setListCollapsed(true)}><ChevronLeft/></button><button className="primary" title="新建笔记" onClick={()=>void create("document")}><Plus/></button></div>
      </header>
      <div className="nt-list-scroll">{loading?<p className="nt-list-empty">正在读取笔记…</p>:notes.length===0?<div className="nt-list-empty"><FileText/><strong>这里还没有笔记</strong><p>创建一篇笔记，或调整搜索和筛选条件。</p></div>:notes.map(note=><button key={note.id} className={selected?.id===note.id?"active":""} onClick={()=>void open(note.id)}><header><strong>{titleOf(note)}</strong><span>{note.isPinned&&<Pin/>}{note.isFavorite&&<Star/>}</span></header><p>{note.summary||"暂无正文"}</p><footer><span>{labels[note.noteType]}</span><time>{formatTime(note.updatedAt)}</time></footer>{note.tags.length>0&&<div>{note.tags.slice(0,3).map(tag=><i key={tag.id} style={{"--tag-color":tag.color} as React.CSSProperties}>{tag.name}</i>)}</div>}</button>)}</div>
    </section>
    {!listCollapsed&&<i className="nt-resizer list" style={{left:(leftCollapsed?0:leftWidth)+listWidth-3}} onPointerDown={event=>resize("list",event)}/>}
    {listCollapsed&&<button className="nt-expand list" onClick={()=>setListCollapsed(false)} title="展开列表"><ChevronRight/></button>}
    {selected?<NoteEditor key={selected.id} note={selected} folders={folders} tags={tags} trashMode={scope==="trash"} registerSave={save=>{saveBeforeSwitch.current=save;return()=>{if(saveBeforeSwitch.current===save)saveBeforeSwitch.current=null}}} onSaved={saved=>{setSelected(saved);setNotes(current=>current.map(item=>item.id===saved.id?{...item,...saved}:item))}} onListChanged={refresh}/>:<section className="nt-editor nt-empty-editor"><div><FileText/><h2>选择或创建一篇笔记</h2><p>内容会自动保存到本机 SQLite 数据库。</p><button className="hx-btn primary" onClick={()=>void create("document")}><Plus/>新建笔记</button></div></section>}
  </div>{commandOpen&&<div className="nt-command-backdrop" onMouseDown={event=>{if(event.target===event.currentTarget)setCommandOpen(false)}}><section className="nt-command"><header><Search/><strong>快速命令</strong><kbd>Esc</kbd><button onClick={()=>setCommandOpen(false)}><X/></button></header><div>
    <button onClick={()=>{setCommandOpen(false);void create("document")}}><Plus/><span><strong>新建笔记</strong><small>Ctrl + N</small></span></button>
    <button onClick={()=>{setCommandOpen(false);void create("quick")}}><FileText/><span><strong>新建快速记录</strong><small>Ctrl + Shift + N</small></span></button>
    <button onClick={()=>{setCommandOpen(false);searchRef.current?.focus()}}><Search/><span><strong>搜索笔记</strong><small>Ctrl + Shift + F</small></span></button>
    <button onClick={()=>{setCommandOpen(false);void importMarkdown()}}><FileUp/><span><strong>导入 Markdown</strong><small>桌面文件</small></span></button>
    {selected&&<><button onClick={()=>void toggleSelected("isFavorite")}><Star/><span><strong>{selected.isFavorite?"取消收藏":"收藏当前笔记"}</strong></span></button><button onClick={()=>void toggleSelected("isPinned")}><Pin/><span><strong>{selected.isPinned?"取消置顶":"置顶当前笔记"}</strong></span></button><button onClick={()=>void exportSelected()}><Download/><span><strong>导出当前笔记</strong><small>Markdown</small></span></button></>}
    {notes.slice(0,8).map(note=><button key={note.id} onClick={()=>{setCommandOpen(false);void open(note.id)}}><File/><span><strong>打开 · {titleOf(note)}</strong><small>{formatTime(note.updatedAt)}</small></span></button>)}
  </div></section></div>}</>;
}

export function DashboardNotes({openNotes}:{openNotes:(id?:string)=>void}){
  const [text,setText]=useState("");const [recent,setRecent]=useState<Note[]>([]);const [saving,setSaving]=useState(false);
  const reload=useCallback(()=>noteApi.list({scope:"all",sort:"updated_desc",limit:5}).then(setRecent).catch(()=>undefined),[]);
  useEffect(()=>{void reload()},[reload]);
  const submit=async()=>{const value=text.trim();if(!value||saving)return;setSaving(true);try{await noteApi.create({title:null,noteType:"quick",folderId:null,contentJson:{type:"doc",content:[{type:"paragraph",content:[{type:"text",text:value}]}]},contentHtml:`<p>${value.replace(/[&<>"]/g,char=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;"}[char]!) )}</p>`,contentText:value,contentMarkdown:value,summary:cleanSummary(value),isPinned:false,isFavorite:false,isArchived:false,tagIds:[],relations:[]});setText("");await reload();notify("快速记录已保存")}finally{setSaving(false)}};
  return <article className="hx-panel nt-dashboard-widget"><header><div><span>快速记录</span><h2>记录此刻的想法</h2></div><button onClick={()=>openNotes()}>打开笔记 <ChevronRight/></button></header><div className="nt-quick"><textarea value={text} onChange={e=>setText(e.target.value)} onKeyDown={e=>{if(e.ctrlKey&&e.key==="Enter"){e.preventDefault();void submit()}}} placeholder="记录此刻的想法……"/><footer><small>Ctrl + Enter 提交</small><button disabled={!text.trim()||saving} onClick={()=>void submit()}>{saving?"保存中":"保存记录"}</button></footer></div>{recent.length>0&&<div className="nt-recent"><strong>最近笔记</strong>{recent.map(note=><button key={note.id} onClick={()=>openNotes(note.id)}><span>{titleOf(note)}</span><small>{formatTime(note.updatedAt)}</small></button>)}</div>}</article>;
}
