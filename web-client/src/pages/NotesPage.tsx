import { FormEvent, useMemo, useState } from "react";
import TurndownService from "turndown";
import { createNote, createNoteFolder, createNoteTag, createNoteTagRelation, type JsonEntity, type NoteContent } from "../core";
import { RichTextEditor, type EditorValue } from "../components/RichTextEditor";
import { CloudPageProps, Empty, Notice, PageStack, Panel, entities, text } from "../ui";

const turndown = new TurndownService({ headingStyle: "atx", bulletListMarker: "-" });

export function NotesPage(props: CloudPageProps) {
  const notes = entities(props.state, "note.note");
  const folders = entities(props.state, "note.folder");
  const tags = entities(props.state, "note.tag");
  const tagRelations = entities(props.state, "note.tag_relation");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [folderId, setFolderId] = useState("");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [content, setContent] = useState<EditorValue>({ html: "<p></p>", text: "", json: { type: "doc", content: [] } });
  const [markdown, setMarkdown] = useState("");
  const [mode, setMode] = useState<"rich" | "markdown">("rich");
  const [query, setQuery] = useState("");
  const [folderFilter, setFolderFilter] = useState("");
  const [localError, setLocalError] = useState("");

  const filtered = useMemo(() => notes.filter((item) => {
    const search = `${text(item, "title")} ${text(item, "contentText")} ${text(item, "contentMarkdown")}`.toLowerCase();
    return search.includes(query.toLowerCase()) && (!folderFilter || item.folderId === folderFilter);
  }).sort((a, b) => Number(b.isPinned === true) - Number(a.isPinned === true) || b.meta.updatedAt.localeCompare(a.meta.updatedAt)), [notes, query, folderFilter]);

  function reset() {
    setEditingId(null); setTitle(""); setFolderId(""); setSelectedTags([]);
    setContent({ html: "<p></p>", text: "", json: { type: "doc", content: [] } });
    setMarkdown(""); setMode("rich"); setLocalError("");
  }

  function edit(item: JsonEntity) {
    setEditingId(item.meta.id);
    setTitle(text(item, "title"));
    setFolderId(typeof item.folderId === "string" ? item.folderId : "");
    const html = text(item, "contentHtml") || `<p>${text(item, "contentText")}</p>`;
    setContent({ html, text: text(item, "contentText"), json: item.contentJson ?? { type: "doc", content: [] } });
    setMarkdown(text(item, "contentMarkdown") || turndown.turndown(html));
    setSelectedTags(tagRelations.filter((relation) => relation.noteId === item.meta.id).map((relation) => String(relation.tagId)));
    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  function changeMode(next: "rich" | "markdown") {
    if (next === mode) return;
    if (next === "markdown") setMarkdown(turndown.turndown(content.html));
    else setContent({ html: `<p>${markdown.replace(/\n/g, "<br>")}</p>`, text: markdown, json: { type: "doc", content: markdown } });
    setMode(next);
  }

  async function save(event: FormEvent) {
    event.preventDefault(); setLocalError("");
    try {
      const existing = notes.find((item) => item.meta.id === editingId);
      const value: NoteContent = mode === "markdown"
        ? { html: `<p>${markdown.replace(/[&<>]/g, "").replace(/\n/g, "<br>")}</p>`, text: markdown, json: { type: "doc", content: markdown }, markdown }
        : { ...content, markdown: turndown.turndown(content.html) };
      const entity = existing
        ? { ...existing, title: title.trim() || null, folderId: folderId || null, contentHtml: value.html, contentText: value.text, contentJson: value.json, contentMarkdown: value.markdown, summary: value.text.slice(0, 160), meta: { ...existing.meta } }
        : createNote(props.session.user.id, props.session.session.deviceId, title, value, folderId || null);
      const next = await props.run((store) => store.upsert("note.note", entity));
      const noteId = entity.meta.id;
      const current = tagRelations.filter((relation) => relation.noteId === noteId);
      for (const relation of current.filter((relation) => !selectedTags.includes(String(relation.tagId)))) {
        await props.run((store) => store.delete("note.tag_relation", relation.meta.id));
      }
      for (const tagId of selectedTags.filter((id) => !current.some((relation) => relation.tagId === id))) {
        await props.run((store) => store.upsert("note.tag_relation", createNoteTagRelation(props.session.user.id, props.session.session.deviceId, noteId, tagId)));
      }
      void next;
      reset();
    } catch (cause) { setLocalError(cause instanceof Error ? cause.message : "保存失败"); }
  }

  async function createFolder() {
    const name = window.prompt("文件夹名称");
    if (!name?.trim()) return;
    await props.run((store) => store.upsert("note.folder", createNoteFolder(props.session.user.id, props.session.session.deviceId, name, folders.length)));
  }

  async function createTag() {
    const name = window.prompt("标签名称");
    if (!name?.trim()) return;
    await props.run((store) => store.upsert("note.tag", createNoteTag(props.session.user.id, props.session.session.deviceId, name)));
  }

  return <PageStack><div className="notes-toolbar"><div className="filter-row"><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索标题或正文" /><select value={folderFilter} onChange={(event) => setFolderFilter(event.target.value)}><option value="">全部文件夹</option>{folders.map((item) => <option key={item.meta.id} value={item.meta.id}>{text(item, "name")}</option>)}</select></div><div className="toolbar-actions"><button className="secondary-button" onClick={() => void createFolder()}>新建文件夹</button><button className="secondary-button" onClick={() => void createTag()}>新建标签</button></div></div><div className="notes-layout"><Panel title={editingId ? "编辑笔记" : "新建笔记"} eyebrow="TIPTAP / MARKDOWN" actions={editingId ? <button className="link-button" onClick={reset}>取消</button> : undefined}><form className="note-editor" onSubmit={(event) => void save(event)}><input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="标题（可选）" /><div className="editor-controls"><button type="button" className={mode === "rich" ? "active" : ""} onClick={() => changeMode("rich")}>富文本</button><button type="button" className={mode === "markdown" ? "active" : ""} onClick={() => changeMode("markdown")}>Markdown</button></div>{mode === "rich" ? <RichTextEditor value={content.html} onChange={setContent} /> : <textarea className="markdown-editor" rows={15} value={markdown} onChange={(event) => setMarkdown(event.target.value)} placeholder="# Markdown 内容" />}<label>文件夹<select value={folderId} onChange={(event) => setFolderId(event.target.value)}><option value="">未分类</option>{folders.map((item) => <option key={item.meta.id} value={item.meta.id}>{text(item, "name")}</option>)}</select></label><fieldset className="tag-picker"><legend>标签</legend>{tags.map((item) => <label key={item.meta.id}><input type="checkbox" checked={selectedTags.includes(item.meta.id)} onChange={(event) => setSelectedTags((values) => event.target.checked ? [...values, item.meta.id] : values.filter((id) => id !== item.meta.id))} />{text(item, "name")}</label>)}</fieldset><div className="attachment-placeholder"><strong>附件上传</strong><p>对象存储上传接口属于 EPIC-12，当前云端尚未提供文件字节上传端点，因此此处不伪造上传成功。接口完成后可直接接入。</p><button type="button" disabled>选择附件</button></div>{localError && <Notice kind="error">{localError}</Notice>}<button className="primary-button" disabled={!props.online || !(mode === "rich" ? content.text.trim() : markdown.trim())}>{editingId ? "保存修改到云端" : "创建云端笔记"}</button></form></Panel><section className="note-grid">{filtered.map((item) => { const itemTags = tagRelations.filter((relation) => relation.noteId === item.meta.id).map((relation) => tags.find((tag) => tag.meta.id === relation.tagId)).filter(Boolean) as JsonEntity[]; return <article className="note-card" key={item.meta.id}><div className="note-card-top"><span>{folders.find((folder) => folder.meta.id === item.folderId)?.name as string || "未分类"}</span>{item.isPinned === true && <b>置顶</b>}</div><h3>{text(item, "title") || "无标题笔记"}</h3><p>{text(item, "summary") || text(item, "contentText") || "暂无内容"}</p><div className="tag-row">{itemTags.map((tag) => <span key={tag.meta.id}>#{text(tag, "name")}</span>)}</div><small>{new Date(item.meta.updatedAt).toLocaleString("zh-CN")}</small><div className="card-actions"><button onClick={() => edit(item)}>编辑</button><button onClick={() => void props.run((store) => store.upsert("note.note", { ...item, isPinned: item.isPinned !== true, meta: { ...item.meta } }))}>{item.isPinned === true ? "取消置顶" : "置顶"}</button><button className="danger" onClick={() => void props.run((store) => store.delete("note.note", item.meta.id))}>删除</button></div></article>; })}{!filtered.length && <Empty title="没有匹配笔记" description="创建笔记或调整筛选条件。" />}</section></div></PageStack>;
}
