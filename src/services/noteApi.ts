import type { Note, NoteFolder, NoteRevision, NoteTag, NoteType } from "@/src/types";

type ListOptions = { q?:string;scope?:string;folderId?:string;tagId?:string;noteType?:NoteType;sort?:string;limit?:number };
type NoteInput = Pick<Note,"title"|"noteType"|"folderId"|"contentJson"|"contentHtml"|"contentText"|"contentMarkdown"|"summary"|"isPinned"|"isFavorite"|"isArchived"> & {
  id?:string; tagIds:string[]; relations: Note["relations"]; createRevision?:boolean;
};

async function request<T>(url:string, init?:RequestInit):Promise<T>{
  const response=await fetch(url,init);
  const payload=await response.json() as T & {error?:string};
  if(!response.ok)throw new Error(payload.error||"笔记服务暂时不可用");
  return payload;
}
const post=<T>(body:unknown)=>request<T>("/api/notes",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify(body)});

export const noteApi={
  list:(options:ListOptions={})=>{const query=new URLSearchParams();Object.entries(options).forEach(([key,value])=>{if(value!==undefined&&value!=="")query.set(key,String(value))});return request<Note[]>(`/api/notes?${query}`)},
  get:(id:string)=>request<Note>(`/api/notes?action=get&id=${encodeURIComponent(id)}`),
  meta:()=>request<{folders:NoteFolder[];tags:(NoteTag&{usageCount?:number})[]}>("/api/notes?action=meta"),
  revisions:(id:string)=>request<NoteRevision[]>(`/api/notes?action=revisions&id=${encodeURIComponent(id)}`),
  backup:()=>request<Record<string,unknown>>("/api/notes?action=backup"),
  create:(note:NoteInput)=>post<Note>({action:"create",note}),
  update:(note:NoteInput&{id:string})=>post<Note>({action:"update",note}),
  trash:(id:string)=>post<{ok:true}>({action:"trash",id}),
  restore:(id:string)=>post<{ok:true}>({action:"restore",id}),
  delete:(id:string)=>post<{ok:true}>({action:"delete",id}),
  duplicate:(id:string)=>post<Note>({action:"duplicate",id}),
  saveFolder:(folder:Partial<NoteFolder>&Pick<NoteFolder,"name">)=>post<{ok:true;id:string}>({action:"folder.save",folder}),
  deleteFolder:(id:string)=>post<{ok:true}>({action:"folder.delete",id}),
  saveTag:(tag:Partial<NoteTag>&Pick<NoteTag,"name">)=>post<{ok:true;id:string}>({action:"tag.save",tag}),
  deleteTag:(id:string)=>post<{ok:true}>({action:"tag.delete",id}),
  restoreRevision:(id:string)=>post<Note>({action:"revision.restore",id}),
  recordAttachment:(file:Record<string,unknown>)=>post<{ok:true}>({action:"attachment.record",file}),
  deleteAttachment:(id:string)=>post<{ok:true}>({action:"attachment.delete",id}),
  restoreBackup:(data:Record<string,unknown>)=>post<{ok:true}>({action:"backup.restore",data}),
};

export type NoteInputValue = NoteInput;
