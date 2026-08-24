"use client";

import { useCallback, useEffect, useState } from "react";
import { Archive, BookOpenCheck, ChevronRight, RotateCcw, Search, Settings2, Trash2, Volume2, X } from "lucide-react";
import type { DictionaryLookup, UserVocabulary, VocabularyReviewResult, VocabularySettings, VocabularyStatus } from "@/src/types/english";
import { speakEnglish, stopSpeech } from "@/src/services/pronunciation";

type Stats = { total: number; dueToday: number; learning: number; mastered: number; addedWeek: number; reviewedWeek: number; averageAccuracy: number;
  reviewStreak?:number; addedDaily?: Array<{day:string;count:number}>; reviewedDaily?: Array<{day:string;count:number}>; masteryDistribution?: Array<{stage:number;count:number}>; sourceDistribution?: Array<{source:string;count:number}>; frequencyDistribution?:Array<{bucket:string;count:number}> };
const defaults: VocabularySettings = { preferredAccent: "en-US", wordSpeechRate: .8, sentenceSpeechRate: .85, autoPronounce: false, defaultFirstMeaning: true, dailyReviewLimit: 20, showSourceSentence: true, includeMasteredInRecommendations: false };

async function api<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, init); const payload = await response.json() as T & { error?: string };
  if (!response.ok) throw new Error(payload.error || "生词服务暂时不可用"); return payload;
}

export function VocabularyWorkspace({ refreshKey = 0, initialMode = "list" }: { refreshKey?: number; initialMode?: "list" | "review" | "settings" }) {
  const [items, setItems] = useState<UserVocabulary[]>([]);
  const [stats, setStats] = useState<Stats>({ total: 0, dueToday: 0, learning: 0, mastered: 0, addedWeek: 0, reviewedWeek: 0, averageAccuracy: 0 });
  const [settings, setSettings] = useState(defaults);
  const [mode, setMode] = useState<"list" | "review" | "settings">(initialMode);
  const [query, setQuery] = useState(""); const [status, setStatus] = useState("ALL"); const [sort, setSort] = useState("created");
  const [articleId, setArticleId] = useState(""); const [pos, setPos] = useState(""); const [tag, setTag] = useState("");
  const [detail, setDetail] = useState<UserVocabulary | null>(null); const [message, setMessage] = useState("");

  const load = useCallback(async () => {
    const params = new URLSearchParams({ query, status, sort, articleId, pos, tag, pageSize: "100" });
    const [words, nextStats, nextSettings] = await Promise.all([
      api<{ items: UserVocabulary[] }>(`/api/english/vocabulary?${params}`), api<Stats>("/api/english/vocabulary/stats"),
      api<VocabularySettings>("/api/english/vocabulary/settings"),
    ]);
    setItems(words.items); setStats(nextStats); setSettings(nextSettings);
  }, [articleId, pos, query, sort, status, tag]);
  useEffect(() => { const timer = window.setTimeout(() => void load().catch((error) => setMessage(error.message)), 220); return () => window.clearTimeout(timer); }, [load, refreshKey]);
  useEffect(() => () => stopSpeech(), []);

  const openDetail = async (id: string) => setDetail(await api<UserVocabulary>(`/api/english/vocabulary/${id}`));
  const patch = async (id: string, value: object) => {
    await api(`/api/english/vocabulary/${id}`, { method: "PATCH", headers: { "content-type": "application/json" }, body: JSON.stringify(value) });
    setDetail(null); await load();
  };
  const remove = async (id: string) => {
    if (!window.confirm("确认删除这个生词及其全部复习记录吗？")) return;
    await api(`/api/english/vocabulary/${id}`, { method: "DELETE" }); setDetail(null); await load();
  };
  const saveSettings = async (patch: Partial<VocabularySettings>) => {
    const next = await api<VocabularySettings>("/api/english/vocabulary/settings", { method: "PATCH", headers: { "content-type": "application/json" }, body: JSON.stringify(patch) });
    setSettings(next);
  };

  return <div className="en-vocabulary-workspace">
    {message && <div className="en-message" role="status">{message}</div>}
    <header className="en-vocab-header"><div><span className="en-eyebrow">VOCABULARY</span><h2>把遇见过的词，<br />变成真正掌握的词。</h2></div>
      <nav><button className={mode === "list" ? "active" : ""} onClick={() => setMode("list")}>生词本</button><button className={mode === "review" ? "active" : ""} onClick={() => setMode("review")}>今日复习</button><button className={mode === "settings" ? "active" : ""} onClick={() => setMode("settings")}><Settings2 />设置</button></nav>
    </header>
    <section className="en-vocab-stats">
      {[["全部生词", stats.total], ["今日待复习", stats.dueToday], ["学习中", stats.learning], ["已掌握", stats.mastered], ["本周新增", stats.addedWeek]].map(([label, value]) => <article key={label}><span>{label}</span><strong>{value}</strong></article>)}
    </section>
    {mode === "list" && <VocabularyList items={items} query={query} setQuery={setQuery} status={status} setStatus={setStatus} sort={sort} setSort={setSort} articleId={articleId} setArticleId={setArticleId} pos={pos} setPos={setPos} tag={tag} setTag={setTag} open={openDetail} />}
    {mode === "list" && <VocabularyInsights stats={stats} />}
    {mode === "review" && <ReviewSession settings={settings} onDone={load} onMessage={setMessage} />}
    {mode === "settings" && <VocabularySettingsPanel settings={settings} save={saveSettings} stats={stats} />}
    {detail && <VocabularyDetail item={detail} settings={settings} close={() => setDetail(null)} patch={patch} remove={remove} message={setMessage} />}
  </div>;
}

function VocabularyList({ items, query, setQuery, status, setStatus, sort, setSort, articleId, setArticleId, pos, setPos, tag, setTag, open }: {
  items: UserVocabulary[]; query: string; setQuery: (v:string)=>void; status:string; setStatus:(v:string)=>void; sort:string; setSort:(v:string)=>void;
  articleId:string;setArticleId:(v:string)=>void;pos:string;setPos:(v:string)=>void;tag:string;setTag:(v:string)=>void;open:(id:string)=>void;
}) {
  const articles = [...new Map(items.filter((item)=>item.sourceArticleId).map((item)=>[item.sourceArticleId!,item.sourceArticleTitle||"未知文章"])).entries()];
  const parts = [...new Set(items.map((item)=>item.partOfSpeech).filter(Boolean))];
  const tags = [...new Set(items.flatMap((item)=>item.tags))];
  return <section className="en-vocab-list-section"><div className="en-vocab-tools"><label><Search /><input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="搜索单词或原形" /></label>
    <select aria-label="按状态筛选" value={status} onChange={(e) => setStatus(e.target.value)}><option value="ALL">全部状态</option><option value="LEARNING">学习中</option><option value="REVIEWING">复习中</option><option value="MASTERED">已掌握</option><option value="ARCHIVED">已归档</option></select>
    <select aria-label="排序方式" value={sort} onChange={(e) => setSort(e.target.value)}><option value="created">最近添加</option><option value="review">复习时间</option><option value="frequency">词频排序</option></select>
    <select aria-label="按文章来源筛选" value={articleId} onChange={(e)=>setArticleId(e.target.value)}><option value="">全部文章</option>{articles.map(([id,title])=><option value={id} key={id}>{title}</option>)}</select>
    <select aria-label="按词性筛选" value={pos} onChange={(e)=>setPos(e.target.value)}><option value="">全部词性</option>{parts.map((value)=><option key={value}>{value}</option>)}</select>
    <select aria-label="按考试标签筛选" value={tag} onChange={(e)=>setTag(e.target.value)}><option value="">全部标签</option>{tags.map((value)=><option key={value}>{value.toUpperCase()}</option>)}</select></div>
    <div className="en-vocab-list">{items.map((item) => <button key={item.id} onClick={() => void open(item.id)}>
      <div><strong>{item.word}</strong><span>/{item.phonetic || "暂无音标"}/ · {item.partOfSpeech || "词性未知"}</span></div>
      <p>{item.selectedMeanings.join("；")}</p><small>来源：{item.sourceArticleTitle || "历史文章"} · 遇见 {item.encounterCount} 次</small>
      <b>{item.status === "MASTERED" ? "已掌握" : `掌握度 ${Math.min(5,item.masteryLevel)} / 5`}</b><ChevronRight />
    </button>)}</div>{!items.length && <p className="en-empty">暂无符合条件的生词。阅读文章时点击任意英文单词即可添加。</p>}
  </section>;
}

function VocabularyInsights({ stats }: { stats: Stats }) {
  const maxAdded = Math.max(1, ...(stats.addedDaily?.map((item)=>item.count) ?? []));
  const maxReviewed = Math.max(1, ...(stats.reviewedDaily?.map((item)=>item.count) ?? []));
  return <section className="en-vocab-insights"><article><h3>最近 30 天新增</h3><div className="en-vocab-mini-chart">{stats.addedDaily?.map((item)=><i key={item.day} title={`${item.day}：${item.count}`} style={{height:`${Math.max(8,item.count/maxAdded*100)}%`}} />)}</div></article>
    <article><h3>最近 30 天复习</h3><div className="en-vocab-mini-chart review">{stats.reviewedDaily?.map((item)=><i key={item.day} title={`${item.day}：${item.count}`} style={{height:`${Math.max(8,item.count/maxReviewed*100)}%`}} />)}</div></article>
    <article><h3>掌握度分布</h3>{stats.masteryDistribution?.map((item)=><p key={item.stage}><span>阶段 {item.stage}</span><b style={{width:`${Math.max(4,item.count/Math.max(1,stats.total)*100)}%`}} /><strong>{item.count}</strong></p>)}</article>
    <article><h3>文章来源</h3>{stats.sourceDistribution?.slice(0,5).map((item)=><p key={item.source}><span>{item.source}</span><strong>{item.count}</strong></p>)}</article>
    <article><h3>词频分布</h3>{stats.frequencyDistribution?.map((item)=><p key={item.bucket}><span>{item.bucket}</span><b style={{width:`${Math.max(4,item.count/Math.max(1,stats.total)*100)}%`}}/><strong>{item.count}</strong></p>)}</article>
  </section>;
}

function ReviewSession({ settings, onDone, onMessage }: { settings: VocabularySettings; onDone:()=>Promise<void>; onMessage:(v:string)=>void }) {
  const [queue, setQueue] = useState<UserVocabulary[]>([]); const [index, setIndex] = useState(0); const [revealed, setRevealed] = useState(false);
  const [started, setStarted] = useState(() => Date.now()); const [results, setResults] = useState<Record<string,number>>({});
  useEffect(() => { void api<{items:UserVocabulary[]}>("/api/english/vocabulary/review/today").then((data) => setQueue(data.items)); }, []);
  const item = queue[index];
  const answer = async (result: VocabularyReviewResult) => {
    if (!item) return; await api(`/api/english/vocabulary/${item.id}/review`, { method:"POST", headers:{"content-type":"application/json"}, body:JSON.stringify({result,responseTimeMs:Date.now()-started}) });
    setResults((v)=>({...v,[result]:(v[result]??0)+1})); setIndex((v)=>v+1); setRevealed(false); setStarted(Date.now()); await onDone();
  };
  useEffect(() => {
    const key = (e:KeyboardEvent) => { if (!item) return; if (e.key===" ") { e.preventDefault(); setRevealed(true); } const map:Record<string,VocabularyReviewResult>={"1":"FORGOT","2":"HARD","3":"GOOD","4":"EASY"}; if(revealed&&map[e.key]) void answer(map[e.key]); };
    window.addEventListener("keydown",key); return()=>window.removeEventListener("keydown",key);
  });
  if (!queue.length) return <section className="en-review-done"><BookOpenCheck /><h3>今天没有待复习的生词</h3><p>新加入的生词会立即进入复习计划。</p></section>;
  if (!item) return <section className="en-review-done"><BookOpenCheck /><h3>今日复习完成</h3><p>共复习 {queue.length} 个 · 认识 {(results.GOOD??0)+(results.EASY??0)} · 模糊 {results.HARD??0} · 不认识 {results.FORGOT??0}</p></section>;
  return <section className="en-review-session"><header><span>{index+1} / {queue.length}</span><progress value={index} max={queue.length} /></header><article>
    <button className="en-review-speak" aria-label="播放单词" onClick={()=>void speakEnglish(item.word,"word",settings).catch((e)=>onMessage(e.message))}><Volume2 /></button>
    <h3>{item.word}</h3>{settings.showSourceSentence&&item.sourceSentence&&<blockquote>{item.sourceSentence}</blockquote>}
    {!revealed?<button className="primary" onClick={()=>setRevealed(true)}>显示释义 <small>Space</small></button>:<div className="en-review-answer"><strong>/{item.phonetic||"暂无音标"}/</strong><p>{item.selectedMeanings.join("；")}</p><small>{item.partOfSpeech}</small></div>}
  </article>{revealed&&<footer>{([["FORGOT","不认识","1"],["HARD","模糊","2"],["GOOD","认识","3"],["EASY","非常熟悉","4"]] as const).map(([result,label,key])=><button key={result} onClick={()=>void answer(result)}>{label}<small>{key}</small></button>)}</footer>}
  </section>;
}

function VocabularyDetail({ item, settings, close, patch, remove, message }: { item:UserVocabulary;settings:VocabularySettings;close:()=>void;patch:(id:string,p:object)=>Promise<void>;remove:(id:string)=>Promise<void>;message:(v:string)=>void }) {
  const [notes,setNotes]=useState(item.notes); const [meanings,setMeanings]=useState(item.selectedMeanings.join("\n"));
  const [dictionary,setDictionary]=useState<DictionaryLookup | null>(null);
  useEffect(()=>{ void api<DictionaryLookup>(`/api/english/dictionary/lookup?word=${encodeURIComponent(item.lemma)}`).then(setDictionary).catch(()=>undefined); },[item.lemma]);
  return <div className="en-vocab-detail-overlay" onMouseDown={close}><aside onMouseDown={(e)=>e.stopPropagation()}><button className="en-icon-button" aria-label="关闭生词详情" onClick={close}><X /></button>
    <header><span>WORD DETAIL</span><h3>{item.word}</h3><p>原形 {item.lemma} · /{item.phonetic||"暂无音标"}/</p><button onClick={()=>void speakEnglish(item.word,"word",settings).catch((e)=>message(e.message))}><Volume2 />朗读</button></header>
    <label>需要记忆的释义<textarea value={meanings} onChange={(e)=>setMeanings(e.target.value)} /></label><label>个人笔记<textarea value={notes} onChange={(e)=>setNotes(e.target.value)} /></label>
    {dictionary?.found&&<section><h4>完整词典信息</h4>{dictionary.partsOfSpeech?.map((part)=><div key={part.type}><strong>{part.type}</strong><p>{part.translation.join("；")}</p>{part.definition.map((value)=><small key={value}>{value}</small>)}</div>)}<p>{dictionary.tags?.map((tag)=><b key={tag}>{tag.toUpperCase()} </b>)}</p><p>{Object.entries(dictionary.exchange??{}).map(([key,value])=><span key={key}>{key}: {value}　</span>)}</p></section>}
    <section><h4>来源句子 · {item.occurrences?.length??0}</h4>{item.occurrences?.map((o)=><blockquote key={o.id}>{o.sourceSentence}<small>{o.articleTitle}</small><button aria-label="播放来源句" onClick={()=>void speakEnglish(o.sourceSentence,"sentence",settings).catch((e)=>message(e.message))}><Volume2 /></button></blockquote>)}</section>
    <section><h4>复习记录 · {item.reviewLogs?.length??0}</h4><p>阶段 {item.reviewStage} · 正确 {item.correctCount} · 错误 {item.incorrectCount}</p></section>
    <footer><button onClick={()=>void patch(item.id,{status:"MASTERED" satisfies VocabularyStatus})}><BookOpenCheck />标记已掌握</button><button onClick={()=>void patch(item.id,{status:"ARCHIVED" satisfies VocabularyStatus})}><Archive />归档</button><button onClick={()=>void patch(item.id,{reset:true})}><RotateCcw />重置</button><button className="danger" onClick={()=>void remove(item.id)}><Trash2 />删除</button><button className="primary" onClick={()=>void patch(item.id,{notes,selectedMeanings:meanings.split("\n").map(v=>v.trim()).filter(Boolean)})}>保存修改</button></footer>
  </aside></div>;
}

function VocabularySettingsPanel({ settings, save, stats }: {settings:VocabularySettings;save:(p:Partial<VocabularySettings>)=>Promise<void>;stats:Stats}) {
  return <section className="en-vocab-settings"><div><h3>发音</h3><label>默认发音<select value={settings.preferredAccent} onChange={(e)=>void save({preferredAccent:e.target.value as "en-US"|"en-GB"})}><option value="en-US">美音</option><option value="en-GB">英音</option></select></label>
    <label>单词语速 <input type="range" min=".5" max="1.2" step=".05" value={settings.wordSpeechRate} onChange={(e)=>void save({wordSpeechRate:Number(e.target.value)})} /></label><label>句子语速 <input type="range" min=".5" max="1.2" step=".05" value={settings.sentenceSpeechRate} onChange={(e)=>void save({sentenceSpeechRate:Number(e.target.value)})} /></label>
    <label><input type="checkbox" checked={settings.autoPronounce} onChange={(e)=>void save({autoPronounce:e.target.checked})}/>点击单词自动发音</label></div>
    <div><h3>复习</h3><label>每日复习上限<input type="number" min="5" max="100" value={settings.dailyReviewLimit} onChange={(e)=>void save({dailyReviewLimit:Number(e.target.value)})}/></label>
    <label><input type="checkbox" checked={settings.showSourceSentence} onChange={(e)=>void save({showSourceSentence:e.target.checked})}/>复习时显示来源句子</label><label><input type="checkbox" checked={settings.defaultFirstMeaning} onChange={(e)=>void save({defaultFirstMeaning:e.target.checked})}/>默认选择第一条释义</label>
    <label><input type="checkbox" checked={settings.includeMasteredInRecommendations} onChange={(e)=>void save({includeMasteredInRecommendations:e.target.checked})}/>已掌握词参与推荐</label></div>
    <div><h3>本周学习</h3><p>新增 <strong>{stats.addedWeek}</strong></p><p>复习 <strong>{stats.reviewedWeek}</strong></p><p>平均正确率 <strong>{stats.averageAccuracy}%</strong></p><p>连续复习 <strong>{stats.reviewStreak??0} 天</strong></p></div>
  </section>;
}
