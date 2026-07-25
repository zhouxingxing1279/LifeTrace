import { env } from "cloudflare:workers";
import { folderSchema, idSchema, notePayloadSchema, safeJson, tagSchema } from "@/src/server/noteSchemas";

type Row = Record<string, unknown>;
const now = () => new Date().toISOString();
const uid = () => crypto.randomUUID();
const bool = (value: unknown) => Boolean(Number(value));

async function ensureSchema() {
  const sql = [
    `CREATE TABLE IF NOT EXISTS notes (
      id TEXT PRIMARY KEY, title TEXT, note_type TEXT NOT NULL, folder_id TEXT,
      content_json TEXT NOT NULL, content_html TEXT NOT NULL, content_text TEXT NOT NULL,
      content_markdown TEXT NOT NULL, summary TEXT NOT NULL DEFAULT '',
      is_pinned INTEGER NOT NULL DEFAULT 0, is_favorite INTEGER NOT NULL DEFAULT 0,
      is_archived INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
      deleted_at TEXT, version INTEGER NOT NULL DEFAULT 1, ai_summary TEXT, ai_tags TEXT,
      embedding_status TEXT, last_ai_processed_at TEXT)`,
    `CREATE TABLE IF NOT EXISTS note_folders (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, icon TEXT NOT NULL, color TEXT NOT NULL, sort_order INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL)`,
    `CREATE TABLE IF NOT EXISTS note_tags (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE, color TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL)`,
    `CREATE TABLE IF NOT EXISTS note_tag_relations (note_id TEXT NOT NULL, tag_id TEXT NOT NULL, PRIMARY KEY(note_id, tag_id), FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE, FOREIGN KEY(tag_id) REFERENCES note_tags(id) ON DELETE CASCADE)`,
    `CREATE TABLE IF NOT EXISTS note_relations (id TEXT PRIMARY KEY, note_id TEXT NOT NULL, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, relation_type TEXT NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE)`,
    `CREATE TABLE IF NOT EXISTS note_attachments (id TEXT PRIMARY KEY, note_id TEXT NOT NULL, file_name TEXT NOT NULL, original_name TEXT NOT NULL, mime_type TEXT NOT NULL, file_size INTEGER NOT NULL, storage_path TEXT NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE)`,
    `CREATE TABLE IF NOT EXISTS note_revisions (id TEXT PRIMARY KEY, note_id TEXT NOT NULL, version INTEGER NOT NULL, title TEXT, content_json TEXT NOT NULL, content_html TEXT NOT NULL, content_markdown TEXT NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE)`,
    `CREATE INDEX IF NOT EXISTS notes_updated_at_idx ON notes(updated_at)`,
    `CREATE INDEX IF NOT EXISTS notes_created_at_idx ON notes(created_at)`,
    `CREATE INDEX IF NOT EXISTS notes_deleted_at_idx ON notes(deleted_at)`,
    `CREATE INDEX IF NOT EXISTS notes_folder_id_idx ON notes(folder_id)`,
    `CREATE INDEX IF NOT EXISTS notes_note_type_idx ON notes(note_type)`,
    `CREATE INDEX IF NOT EXISTS notes_favorite_idx ON notes(is_favorite)`,
    `CREATE INDEX IF NOT EXISTS notes_pinned_idx ON notes(is_pinned)`,
    `CREATE INDEX IF NOT EXISTS note_relations_entity_idx ON note_relations(entity_type, entity_id)`,
    `CREATE INDEX IF NOT EXISTS note_tags_note_idx ON note_tag_relations(note_id)`,
    `CREATE INDEX IF NOT EXISTS note_tags_tag_idx ON note_tag_relations(tag_id)`,
  ];
  await env.DB.batch(sql.map((statement) => env.DB.prepare(statement)));
  try {
    await env.DB.batch([
      env.DB.prepare("CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(title,content_text,summary,content='notes',content_rowid='rowid',tokenize='trigram')"),
      env.DB.prepare("CREATE TRIGGER IF NOT EXISTS notes_fts_insert AFTER INSERT ON notes BEGIN INSERT INTO notes_fts(rowid,title,content_text,summary) VALUES(new.rowid,new.title,new.content_text,new.summary); END"),
      env.DB.prepare("CREATE TRIGGER IF NOT EXISTS notes_fts_delete AFTER DELETE ON notes BEGIN INSERT INTO notes_fts(notes_fts,rowid,title,content_text,summary) VALUES('delete',old.rowid,old.title,old.content_text,old.summary); END"),
      env.DB.prepare("CREATE TRIGGER IF NOT EXISTS notes_fts_update AFTER UPDATE ON notes BEGIN INSERT INTO notes_fts(notes_fts,rowid,title,content_text,summary) VALUES('delete',old.rowid,old.title,old.content_text,old.summary); INSERT INTO notes_fts(rowid,title,content_text,summary) VALUES(new.rowid,new.title,new.content_text,new.summary); END"),
    ]);
  } catch {
    // Some D1 builds do not expose FTS5 trigram; the indexed LIKE path below remains functional.
  }
  const count = await env.DB.prepare("SELECT COUNT(*) count FROM note_folders").first<{ count: number }>();
  if (!count?.count) {
    const stamp = now();
    const names = [["工作", "briefcase", "#416b5c"], ["学习", "book", "#5975a4"], ["健身", "dumbbell", "#b06943"], ["生活", "home", "#887257"], ["财务", "wallet", "#8b654d"], ["项目", "folder", "#6c668f"]];
    await env.DB.batch(names.map(([name, icon, color], sortOrder) =>
      env.DB.prepare("INSERT INTO note_folders (id,name,icon,color,sort_order,created_at,updated_at) VALUES (?,?,?,?,?,?,?)")
        .bind(uid(), name, icon, color, sortOrder, stamp, stamp)));
  }
}

function mapNote(row: Row) {
  return {
    id: String(row.id), title: row.title === null ? null : String(row.title), noteType: String(row.note_type),
    folderId: row.folder_id === null ? null : String(row.folder_id),
    contentJson: safeJson(String(row.content_json ?? ""), { type: "doc", content: [] }),
    contentHtml: String(row.content_html ?? ""), contentText: String(row.content_text ?? ""),
    contentMarkdown: String(row.content_markdown ?? ""), summary: String(row.summary ?? ""),
    isPinned: bool(row.is_pinned), isFavorite: bool(row.is_favorite), isArchived: bool(row.is_archived),
    createdAt: String(row.created_at), updatedAt: String(row.updated_at),
    deletedAt: row.deleted_at === null ? null : String(row.deleted_at), version: Number(row.version),
  };
}

async function details(id: string) {
  const row = await env.DB.prepare("SELECT * FROM notes WHERE id=?").bind(id).first<Row>();
  if (!row) return null;
  const [tags, relations, attachments] = await Promise.all([
    env.DB.prepare("SELECT t.* FROM note_tags t JOIN note_tag_relations r ON r.tag_id=t.id WHERE r.note_id=? ORDER BY t.name").bind(id).all<Row>(),
    env.DB.prepare("SELECT * FROM note_relations WHERE note_id=? ORDER BY created_at").bind(id).all<Row>(),
    env.DB.prepare("SELECT id,note_id,file_name,original_name,mime_type,file_size,created_at FROM note_attachments WHERE note_id=? ORDER BY created_at DESC").bind(id).all<Row>(),
  ]);
  return {
    ...mapNote(row),
    tags: tags.results.map((tag) => ({ id: tag.id, name: tag.name, color: tag.color, createdAt: tag.created_at, updatedAt: tag.updated_at })),
    relations: relations.results.map((rel) => ({ id: rel.id, noteId: rel.note_id, entityType: rel.entity_type, entityId: rel.entity_id, relationType: rel.relation_type, createdAt: rel.created_at })),
    attachments: attachments.results.map((file) => ({ id: file.id, noteId: file.note_id, fileName: file.file_name, originalName: file.original_name, mimeType: file.mime_type, fileSize: file.file_size, createdAt: file.created_at })),
  };
}

export async function GET(request: Request) {
  try {
    await ensureSchema();
    const url = new URL(request.url);
    const action = url.searchParams.get("action") ?? "list";
    if (action === "get") {
      const id = idSchema.parse(url.searchParams.get("id"));
      const note = await details(id);
      return note ? Response.json(note) : Response.json({ error: "笔记不存在" }, { status: 404 });
    }
    if (action === "meta") {
      const [folders, tags] = await Promise.all([
        env.DB.prepare("SELECT * FROM note_folders ORDER BY sort_order,name").all<Row>(),
        env.DB.prepare("SELECT t.*,COUNT(r.note_id) usage_count FROM note_tags t LEFT JOIN note_tag_relations r ON r.tag_id=t.id GROUP BY t.id ORDER BY usage_count DESC,t.name").all<Row>(),
      ]);
      return Response.json({ folders: folders.results.map((x) => ({ id:x.id,name:x.name,icon:x.icon,color:x.color,sortOrder:x.sort_order,createdAt:x.created_at,updatedAt:x.updated_at })), tags: tags.results.map((x) => ({ id:x.id,name:x.name,color:x.color,createdAt:x.created_at,updatedAt:x.updated_at,usageCount:x.usage_count })) });
    }
    if (action === "revisions") {
      const id = idSchema.parse(url.searchParams.get("id"));
      const result = await env.DB.prepare("SELECT * FROM note_revisions WHERE note_id=? ORDER BY version DESC LIMIT 20").bind(id).all<Row>();
      return Response.json(result.results.map((x) => ({ id:x.id,noteId:x.note_id,version:x.version,title:x.title,contentJson:safeJson(String(x.content_json),{}),contentHtml:x.content_html,contentMarkdown:x.content_markdown,createdAt:x.created_at })));
    }
    if (action === "backup") {
      const tables = ["notes","note_folders","note_tags","note_tag_relations","note_relations","note_attachments","note_revisions"];
      const data: Record<string, Row[]> = {};
      for (const table of tables) data[table] = (await env.DB.prepare(`SELECT * FROM ${table}`).all<Row>()).results;
      return Response.json({ format: "lifetrace-notes", version: 1, createdAt: now(), ...data });
    }

    const query = (url.searchParams.get("q") ?? "").trim().slice(0, 200);
    const scope = url.searchParams.get("scope") ?? "all";
    const folderId = url.searchParams.get("folderId");
    const tagId = url.searchParams.get("tagId");
    const noteType = url.searchParams.get("noteType");
    const sort = url.searchParams.get("sort") ?? "updated_desc";
    const limit = Math.min(Math.max(Number(url.searchParams.get("limit")) || 100, 1), 250);
    const where: string[] = [];
    const binds: unknown[] = [];
    if (scope === "trash") where.push("n.deleted_at IS NOT NULL"); else where.push("n.deleted_at IS NULL");
    if (scope === "favorite") where.push("n.is_favorite=1");
    if (scope === "pinned") where.push("n.is_pinned=1");
    if (scope === "archived") where.push("n.is_archived=1"); else if (scope !== "trash") where.push("n.is_archived=0");
    if (scope === "quick") { where.push("n.note_type=?"); binds.push("quick"); }
    if (folderId) { where.push("n.folder_id=?"); binds.push(idSchema.parse(folderId)); }
    if (tagId) { where.push("EXISTS(SELECT 1 FROM note_tag_relations tr WHERE tr.note_id=n.id AND tr.tag_id=?)"); binds.push(idSchema.parse(tagId)); }
    if (noteType) { where.push("n.note_type=?"); binds.push(noteType); }
    if (query) {
      const like = `%${query.replaceAll("%", "\\%").replaceAll("_", "\\_")}%`;
      const fts = query.length >= 3 && Boolean(await env.DB.prepare("SELECT 1 ok FROM sqlite_master WHERE type='table' AND name='notes_fts'").first());
      where.push(`(${fts?"n.rowid IN (SELECT rowid FROM notes_fts WHERE notes_fts MATCH ?) OR ":""}n.title LIKE ? ESCAPE '\\' COLLATE NOCASE OR n.content_text LIKE ? ESCAPE '\\' COLLATE NOCASE OR n.summary LIKE ? ESCAPE '\\' COLLATE NOCASE OR EXISTS(SELECT 1 FROM note_tag_relations tr JOIN note_tags t ON t.id=tr.tag_id WHERE tr.note_id=n.id AND t.name LIKE ? ESCAPE '\\' COLLATE NOCASE) OR EXISTS(SELECT 1 FROM note_folders f WHERE f.id=n.folder_id AND f.name LIKE ? ESCAPE '\\' COLLATE NOCASE))`);
      if(fts)binds.push(`"${query.replaceAll('"','""')}"`);
      binds.push(like, like, like, like, like);
    }
    const order: Record<string, string> = { updated_desc:"n.is_pinned DESC,n.updated_at DESC,n.created_at DESC", created_desc:"n.is_pinned DESC,n.created_at DESC", created_asc:"n.created_at ASC", title_asc:"COALESCE(NULLIF(n.title,''),n.summary) COLLATE NOCASE ASC", title_desc:"COALESCE(NULLIF(n.title,''),n.summary) COLLATE NOCASE DESC" };
    const sql = `SELECT n.id,n.title,n.note_type,n.folder_id,n.summary,n.is_pinned,n.is_favorite,n.is_archived,n.created_at,n.updated_at,n.deleted_at,n.version FROM notes n WHERE ${where.join(" AND ")} ORDER BY ${order[sort] ?? order.updated_desc} LIMIT ?`;
    const result = await env.DB.prepare(sql).bind(...binds, limit).all<Row>();
    const ids = result.results.map((x) => String(x.id));
    let tagRows: Row[] = [];
    if (ids.length) tagRows = (await env.DB.prepare(`SELECT r.note_id,t.* FROM note_tag_relations r JOIN note_tags t ON t.id=r.tag_id WHERE r.note_id IN (${ids.map(() => "?").join(",")})`).bind(...ids).all<Row>()).results;
    return Response.json(result.results.map((row) => ({ ...mapNote({ ...row, content_json:"{}", content_html:"", content_text:"", content_markdown:"" }), tags: tagRows.filter((x) => x.note_id===row.id).map((x) => ({ id:x.id,name:x.name,color:x.color,createdAt:x.created_at,updatedAt:x.updated_at })), relations:[] })));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "笔记读取失败" }, { status: 400 });
  }
}

async function replaceLinks(noteId: string, tagIds: string[], relations: { entityType:string; entityId:string; relationType:string }[]) {
  const stamp = now();
  const statements = [
    env.DB.prepare("DELETE FROM note_tag_relations WHERE note_id=?").bind(noteId),
    env.DB.prepare("DELETE FROM note_relations WHERE note_id=?").bind(noteId),
    ...tagIds.map((tagId) => env.DB.prepare("INSERT OR IGNORE INTO note_tag_relations(note_id,tag_id) VALUES(?,?)").bind(noteId, tagId)),
    ...relations.map((rel) => env.DB.prepare("INSERT INTO note_relations(id,note_id,entity_type,entity_id,relation_type,created_at) VALUES(?,?,?,?,?,?)").bind(uid(),noteId,rel.entityType,rel.entityId,rel.relationType,stamp)),
  ];
  await env.DB.batch(statements);
}

export async function POST(request: Request) {
  try {
    await ensureSchema();
    const body = await request.json() as Record<string, unknown>;
    const action = String(body.action ?? "");
    if (action === "create" || action === "update") {
      const value = notePayloadSchema.parse(body.note);
      const id = action === "create" ? (value.id ?? uid()) : idSchema.parse(value.id);
      const old = action === "update" ? await env.DB.prepare("SELECT * FROM notes WHERE id=?").bind(id).first<Row>() : null;
      if (action === "update" && !old) return Response.json({ error: "笔记不存在" }, { status: 404 });
      const stamp = now();
      const createdAt = old ? String(old.created_at) : stamp;
      const version = old ? Number(old.version) + 1 : 1;
      if (old && value.createRevision) {
        await env.DB.prepare("INSERT INTO note_revisions(id,note_id,version,title,content_json,content_html,content_markdown,created_at) VALUES(?,?,?,?,?,?,?,?)")
          .bind(uid(),id,old.version,old.title,old.content_json,old.content_html,old.content_markdown,stamp).run();
      }
      await env.DB.prepare(`INSERT INTO notes(id,title,note_type,folder_id,content_json,content_html,content_text,content_markdown,summary,is_pinned,is_favorite,is_archived,created_at,updated_at,deleted_at,version)
        VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,NULL,?)
        ON CONFLICT(id) DO UPDATE SET title=excluded.title,note_type=excluded.note_type,folder_id=excluded.folder_id,content_json=excluded.content_json,content_html=excluded.content_html,content_text=excluded.content_text,content_markdown=excluded.content_markdown,summary=excluded.summary,is_pinned=excluded.is_pinned,is_favorite=excluded.is_favorite,is_archived=excluded.is_archived,updated_at=excluded.updated_at,version=excluded.version`)
        .bind(id,value.title?.trim()||null,value.noteType,value.folderId??null,JSON.stringify(value.contentJson),value.contentHtml,value.contentText,value.contentMarkdown,value.summary,value.isPinned?1:0,value.isFavorite?1:0,value.isArchived?1:0,createdAt,stamp,version).run();
      await replaceLinks(id, value.tagIds, value.relations);
      await env.DB.prepare("DELETE FROM note_revisions WHERE note_id=? AND id NOT IN (SELECT id FROM note_revisions WHERE note_id=? ORDER BY version DESC LIMIT 20)").bind(id,id).run();
      return Response.json(await details(id));
    }
    if (["trash","restore","delete"].includes(action)) {
      const id = idSchema.parse(body.id);
      if (action === "trash") await env.DB.prepare("UPDATE notes SET deleted_at=?,updated_at=? WHERE id=?").bind(now(),now(),id).run();
      if (action === "restore") await env.DB.prepare("UPDATE notes SET deleted_at=NULL,updated_at=? WHERE id=?").bind(now(),id).run();
      if (action === "delete") await env.DB.batch([
        env.DB.prepare("DELETE FROM note_tag_relations WHERE note_id=?").bind(id), env.DB.prepare("DELETE FROM note_relations WHERE note_id=?").bind(id),
        env.DB.prepare("DELETE FROM note_attachments WHERE note_id=?").bind(id), env.DB.prepare("DELETE FROM note_revisions WHERE note_id=?").bind(id),
        env.DB.prepare("DELETE FROM notes WHERE id=?").bind(id),
      ]);
      return Response.json({ ok:true });
    }
    if (action === "duplicate") {
      const sourceId = idSchema.parse(body.id); const source = await details(sourceId);
      if (!source) return Response.json({ error:"笔记不存在" }, { status:404 });
      const copyId=uid(), stamp=now();
      await env.DB.prepare("INSERT INTO notes(id,title,note_type,folder_id,content_json,content_html,content_text,content_markdown,summary,is_pinned,is_favorite,is_archived,created_at,updated_at,deleted_at,version) SELECT ?,?,note_type,folder_id,content_json,content_html,content_text,content_markdown,summary,0,is_favorite,0,?,?,NULL,1 FROM notes WHERE id=?").bind(copyId,`${source.title || source.summary || "无标题笔记"} · 副本`,stamp,stamp,sourceId).run();
      await replaceLinks(copyId, source.tags.map((x) => String(x.id)), source.relations.map((x) => ({entityType:String(x.entityType),entityId:String(x.entityId),relationType:String(x.relationType)})));
      return Response.json(await details(copyId));
    }
    if (action === "folder.save") {
      const value=folderSchema.parse(body.folder), stamp=now(), id=value.id??uid();
      await env.DB.prepare("INSERT INTO note_folders(id,name,icon,color,sort_order,created_at,updated_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,icon=excluded.icon,color=excluded.color,sort_order=excluded.sort_order,updated_at=excluded.updated_at").bind(id,value.name,value.icon,value.color,value.sortOrder,stamp,stamp).run();
      return Response.json({ok:true,id});
    }
    if (action === "folder.delete") {
      const id=idSchema.parse(body.id);
      await env.DB.batch([env.DB.prepare("UPDATE notes SET folder_id=NULL WHERE folder_id=?").bind(id),env.DB.prepare("DELETE FROM note_folders WHERE id=?").bind(id)]);
      return Response.json({ok:true});
    }
    if (action === "tag.save") {
      const value=tagSchema.parse(body.tag), stamp=now(), id=value.id??uid();
      await env.DB.prepare("INSERT INTO note_tags(id,name,color,created_at,updated_at) VALUES(?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,color=excluded.color,updated_at=excluded.updated_at").bind(id,value.name,value.color,stamp,stamp).run();
      return Response.json({ok:true,id});
    }
    if (action === "tag.delete") {
      const id=idSchema.parse(body.id);
      await env.DB.batch([env.DB.prepare("DELETE FROM note_tag_relations WHERE tag_id=?").bind(id),env.DB.prepare("DELETE FROM note_tags WHERE id=?").bind(id)]);
      return Response.json({ok:true});
    }
    if (action === "revision.restore") {
      const revisionId=idSchema.parse(body.id);
      const revision=await env.DB.prepare("SELECT * FROM note_revisions WHERE id=?").bind(revisionId).first<Row>();
      if(!revision)return Response.json({error:"历史版本不存在"},{status:404});
      const current=await env.DB.prepare("SELECT * FROM notes WHERE id=?").bind(revision.note_id).first<Row>();
      if(current) await env.DB.prepare("INSERT INTO note_revisions(id,note_id,version,title,content_json,content_html,content_markdown,created_at) VALUES(?,?,?,?,?,?,?,?)").bind(uid(),current.id,current.version,current.title,current.content_json,current.content_html,current.content_markdown,now()).run();
      await env.DB.prepare("UPDATE notes SET title=?,content_json=?,content_html=?,content_markdown=?,content_text=?,summary=?,version=version+1,updated_at=? WHERE id=?").bind(revision.title,revision.content_json,revision.content_html,revision.content_markdown,String(revision.content_markdown),String(revision.content_markdown).slice(0,160),now(),revision.note_id).run();
      return Response.json(await details(String(revision.note_id)));
    }
    if (action === "attachment.record") {
      const file = body.file as Record<string, unknown>, id=idSchema.parse(file.id), noteId=idSchema.parse(file.noteId);
      await env.DB.prepare("INSERT INTO note_attachments(id,note_id,file_name,original_name,mime_type,file_size,storage_path,created_at) VALUES(?,?,?,?,?,?,?,?)").bind(id,noteId,String(file.fileName),String(file.originalName),String(file.mimeType),Number(file.fileSize),String(file.storagePath),now()).run();
      return Response.json({ok:true});
    }
    if (action === "attachment.delete") {
      await env.DB.prepare("DELETE FROM note_attachments WHERE id=?").bind(idSchema.parse(body.id)).run();
      return Response.json({ok:true});
    }
    if (action === "backup.restore") {
      const data=body.data as Record<string,unknown>;
      if(data?.format!=="lifetrace-notes"||Number(data.version)!==1)throw new Error("不支持的笔记备份格式");
      const required=["notes","note_folders","note_tags","note_tag_relations","note_relations","note_attachments","note_revisions"] as const;
      if(!required.every(key=>Array.isArray(data[key])))throw new Error("笔记备份数据不完整");
      const statements=[
        ...required.slice().reverse().map(table=>env.DB.prepare(`DELETE FROM ${table}`)),
        ...(data.note_folders as Row[]).map(x=>env.DB.prepare("INSERT INTO note_folders(id,name,icon,color,sort_order,created_at,updated_at) VALUES(?,?,?,?,?,?,?)").bind(x.id,x.name,x.icon,x.color,x.sort_order,x.created_at,x.updated_at)),
        ...(data.note_tags as Row[]).map(x=>env.DB.prepare("INSERT INTO note_tags(id,name,color,created_at,updated_at) VALUES(?,?,?,?,?)").bind(x.id,x.name,x.color,x.created_at,x.updated_at)),
        ...(data.notes as Row[]).map(x=>env.DB.prepare("INSERT INTO notes(id,title,note_type,folder_id,content_json,content_html,content_text,content_markdown,summary,is_pinned,is_favorite,is_archived,created_at,updated_at,deleted_at,version,ai_summary,ai_tags,embedding_status,last_ai_processed_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)").bind(x.id,x.title,x.note_type,x.folder_id,x.content_json,x.content_html,x.content_text,x.content_markdown,x.summary,x.is_pinned,x.is_favorite,x.is_archived,x.created_at,x.updated_at,x.deleted_at,x.version,x.ai_summary,x.ai_tags,x.embedding_status,x.last_ai_processed_at)),
        ...(data.note_tag_relations as Row[]).map(x=>env.DB.prepare("INSERT INTO note_tag_relations(note_id,tag_id) VALUES(?,?)").bind(x.note_id,x.tag_id)),
        ...(data.note_relations as Row[]).map(x=>env.DB.prepare("INSERT INTO note_relations(id,note_id,entity_type,entity_id,relation_type,created_at) VALUES(?,?,?,?,?,?)").bind(x.id,x.note_id,x.entity_type,x.entity_id,x.relation_type,x.created_at)),
        ...(data.note_attachments as Row[]).map(x=>env.DB.prepare("INSERT INTO note_attachments(id,note_id,file_name,original_name,mime_type,file_size,storage_path,created_at) VALUES(?,?,?,?,?,?,?,?)").bind(x.id,x.note_id,x.file_name,x.original_name,x.mime_type,x.file_size,x.storage_path,x.created_at)),
        ...(data.note_revisions as Row[]).map(x=>env.DB.prepare("INSERT INTO note_revisions(id,note_id,version,title,content_json,content_html,content_markdown,created_at) VALUES(?,?,?,?,?,?,?,?)").bind(x.id,x.note_id,x.version,x.title,x.content_json,x.content_html,x.content_markdown,x.created_at)),
      ];
      await env.DB.batch(statements);
      return Response.json({ok:true});
    }
    return Response.json({ error:"不支持的笔记操作" }, { status:400 });
  } catch (error) {
    return Response.json({ error:error instanceof Error?error.message:"笔记写入失败" }, { status:400 });
  }
}
