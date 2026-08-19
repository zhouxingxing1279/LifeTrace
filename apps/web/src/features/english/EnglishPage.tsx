import { useMemo, useState, type FormEvent } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { BookOpen, Plus, Languages } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, Card, CardContent, EmptyState, Input, MetricCard, PageHeader, cn } from "../../components/ui";
import { entities, number, text } from "../../lib/entities";
import { createVocabulary } from "../../services/core";

export function EnglishPage(){
  const {state,session,upsert}=useApp(); const location=useLocation(); const navigate=useNavigate(); const [word,setWord]=useState(""); const [definition,setDefinition]=useState("");
  const articles=entities(state,"english.article").sort((a,b)=>b.meta.updatedAt.localeCompare(a.meta.updatedAt)); const vocabulary=entities(state,"english.vocabulary").sort((a,b)=>b.meta.updatedAt.localeCompare(a.meta.updatedAt)); const records=entities(state,"english.learning_record");
  const tab=location.pathname.includes("vocabulary")?"vocabulary":location.pathname.includes("stats")?"stats":"articles";
  const readingMinutes=Math.round(records.reduce((sum,item)=>sum+number(item,"readingTimeSeconds"),0)/60);
  const completed=records.filter((item)=>text(item,"completionStatus")==="completed"||text(item,"readingStatus")==="completed").length;
  const mastery=useMemo(()=>vocabulary.length?Math.round(vocabulary.reduce((sum,item)=>sum+number(item,"masteryLevel"),0)/vocabulary.length):0,[vocabulary]);
  async function add(event:FormEvent){event.preventDefault();if(!session)return;await upsert("english.vocabulary",createVocabulary(session.user.id,session.session.deviceId,word,definition));setWord("");setDefinition("");}
  return <div className="page-shell"><PageHeader title="英语学习" description="阅读、高亮、快捷笔记、生词和学习历史。参考 Catalyst content workspace + shadcn components。"/>
    <div className="mb-5 flex w-fit rounded-md border p-0.5">{[["articles","阅读"],["vocabulary","生词本"],["stats","统计"]].map(([id,label])=><button key={id} onClick={()=>navigate(id==="articles"?"/app/english/articles":`/app/english/${id}`)} className={cn("rounded px-3 py-1.5 text-xs",tab===id&&"bg-muted font-medium")}>{label}</button>)}</div>
    {tab==="articles"?<div>{articles.length?<div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">{articles.map((article)=><Card key={article.meta.id}><CardContent className="pt-5"><div className="flex items-start justify-between gap-3"><BookOpen size={18} className="text-primary"/><Badge>{text(article,"readingStatus","未读")}</Badge></div><div className="mt-4 text-base font-semibold">{text(article,"title","Untitled article")}</div><p className="mt-2 line-clamp-3 text-sm leading-6 text-muted-foreground">{text(article,"summary",text(article,"content","暂无摘要"))}</p></CardContent></Card>)}</div>:<EmptyState icon={<BookOpen size={24}/>} title="还没有阅读文章" description="现有云端文章、高亮与阅读状态会在这里呈现；阅读页不再沿用 legacy Web 布局。"/>}</div>:null}
    {tab==="vocabulary"?<div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_340px]"><div>{vocabulary.length?<Card><div className="divide-y">{vocabulary.map((item)=><div key={item.meta.id} className="px-4 py-3"><div className="flex items-center justify-between"><div className="font-semibold">{text(item,"displayWord")}</div><Badge>Level {number(item,"masteryLevel")}</Badge></div><div className="mt-1 text-sm text-muted-foreground">{text(item,"definition","暂无释义")}</div></div>)}</div></Card>:<EmptyState title="生词本为空" description="在阅读中添加或在右侧手动录入。"/>}</div><Card className="h-fit"><CardContent className="pt-5"><div className="mb-4 flex items-center gap-2 font-semibold"><Plus size={16}/>添加生词</div><form className="space-y-3" onSubmit={(e)=>void add(e)}><Input value={word} onChange={(e)=>setWord(e.target.value)} placeholder="Word" required/><Input value={definition} onChange={(e)=>setDefinition(e.target.value)} placeholder="释义"/><Button className="w-full" type="submit">保存</Button></form></CardContent></Card></div>:null}
    {tab==="stats"?<div className="grid gap-3 sm:grid-cols-3"><MetricCard label="完成阅读" value={`${completed} 篇`} icon={<BookOpen size={17}/>}/><MetricCard label="阅读时长" value={`${readingMinutes} 分钟`} icon={<Languages size={17}/>}/><MetricCard label="生词平均掌握" value={`${mastery}/5`} hint={`${vocabulary.length} 个生词`}/></div>:null}
  </div>;
}
