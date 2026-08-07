import { FormEvent, useMemo, useRef, useState } from "react";
import { createEnglishHighlight, createEnglishLearningRecord, createVocabulary, type JsonEntity } from "../core";
import { CloudPageProps, Empty, EnglishTabs, Metric, Notice, PageStack, Panel, entities, number, text } from "../ui";

export function ArticlesPage(props: CloudPageProps) {
  const articles = entities(props.state, "english.article").sort((a, b) => b.meta.updatedAt.localeCompare(a.meta.updatedAt));
  const highlights = entities(props.state, "english.highlight");
  const [selectedId, setSelectedId] = useState(articles[0]?.meta.id ?? "");
  const [summary, setSummary] = useState("");
  const [highlightNote, setHighlightNote] = useState("");
  const [message, setMessage] = useState("");
  const startedAt = useRef(Date.now());
  const article = articles.find((item) => item.meta.id === selectedId) ?? null;

  async function saveHighlight() {
    if (!article) return;
    const selection = window.getSelection()?.toString().trim() ?? "";
    if (!selection) { setMessage("请先在文章正文中选择需要高亮的文字"); return; }
    await props.run((store) => store.upsert("english.highlight", createEnglishHighlight(props.session.user.id, props.session.session.deviceId, article.meta.id, selection, highlightNote)));
    setHighlightNote(""); setMessage("高亮已保存到云端");
  }

  async function complete() {
    if (!article || !summary.trim()) return;
    const seconds = Math.max(1, Math.round((Date.now() - startedAt.current) / 1000));
    const words = highlights.filter((item) => item.articleId === article.meta.id).map((item) => text(item, "selectedText"));
    await props.run((store) => store.upsert("english.learning_record", createEnglishLearningRecord(props.session.user.id, props.session.session.deviceId, article.meta.id, summary, seconds, words)));
    setSummary(""); setMessage("阅读总结已保存");
  }

  return <PageStack><EnglishTabs />{message && <Notice kind="neutral">{message}</Notice>}<div className="english-layout"><Panel title="文章目录" eyebrow="ARTICLES"><div className="article-list">{articles.map((item) => <button className={item.meta.id === selectedId ? "active" : ""} key={item.meta.id} onClick={() => { setSelectedId(item.meta.id); startedAt.current = Date.now(); setMessage(""); }}><div><span>{text(item, "level")} · {text(item, "category")}</span><h4>{text(item, "title") || "Untitled article"}</h4><p>{text(item, "summary") || text(item, "content").slice(0, 130)}</p></div><b>阅读 →</b></button>)}{!articles.length && <Empty title="暂无文章" description="文章目录由 LifeTrace Cloud 只读下发。" />}</div></Panel><div className="page-stack">{article ? <><Panel title={text(article, "title") || "English article"} eyebrow={`${text(article, "level")} · ${number(article, "wordCount")} WORDS`}><article className="article-reader">{text(article, "content").split(/\n{2,}/).map((paragraph, index) => <p key={index}>{paragraph}</p>)}</article><div className="highlight-form"><input value={highlightNote} onChange={(event) => setHighlightNote(event.target.value)} placeholder="高亮备注（可选）" /><button className="secondary-button" disabled={!props.online} onClick={() => void saveHighlight()}>保存选中文本为高亮</button></div></Panel><Panel title="阅读总结" eyebrow="SUMMARY"><textarea rows={6} value={summary} onChange={(event) => setSummary(event.target.value)} placeholder="用英文或中文总结文章重点…" /><button className="primary-button" disabled={!props.online || !summary.trim()} onClick={() => void complete()}>完成阅读并保存</button></Panel><Panel title="本文高亮" eyebrow="HIGHLIGHTS"><div className="quote-list">{highlights.filter((item) => item.articleId === article.meta.id).map((item) => <blockquote key={item.meta.id}>{text(item, "selectedText")}<small>{text(item, "note")}</small></blockquote>)}{!highlights.some((item) => item.articleId === article.meta.id) && <p className="muted">尚未保存高亮。</p>}</div></Panel></> : <Empty title="选择一篇文章" description="从左侧目录开始阅读。" />}</div></div></PageStack>;
}

export function VocabularyPage(props: CloudPageProps) {
  const words = useMemo(() => entities(props.state, "english.vocabulary").sort((a, b) => b.meta.updatedAt.localeCompare(a.meta.updatedAt)), [props.state]);
  const [word, setWord] = useState("");
  const [definition, setDefinition] = useState("");
  const [query, setQuery] = useState("");
  async function submit(event: FormEvent) {
    event.preventDefault();
    await props.run((store) => store.upsert("english.vocabulary", createVocabulary(props.session.user.id, props.session.session.deviceId, word, definition)));
    setWord(""); setDefinition("");
  }
  const filtered = words.filter((item) => `${text(item, "displayWord")} ${text(item, "definition")}`.toLowerCase().includes(query.toLowerCase()));
  return <PageStack><EnglishTabs /><Panel title="添加生词" eyebrow="VOCABULARY"><form className="inline-form" onSubmit={(event) => void submit(event)}><input required value={word} onChange={(event) => setWord(event.target.value)} placeholder="resilient" /><input value={definition} onChange={(event) => setDefinition(event.target.value)} placeholder="有韧性的" /><button className="primary-button" disabled={!props.online}>保存到云端</button></form></Panel><div className="filter-row"><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索单词或释义" /></div><div className="vocab-grid">{filtered.map((item) => <article className="data-card" key={item.meta.id}><span className={`status-pill ${text(item, "status").toLowerCase()}`}>{text(item, "status") || "LEARNING"}</span><h3>{text(item, "displayWord")}</h3><p>{text(item, "definition") || "暂无释义"}</p><small>遇见 {number(item, "encounterCount")} 次 · 复习 {number(item, "reviewCount")} 次</small><div className="card-actions"><button onClick={() => void props.run((store) => store.upsert("english.vocabulary", { ...item, status: text(item, "status") === "MASTERED" ? "LEARNING" : "MASTERED", masteryLevel: text(item, "status") === "MASTERED" ? 1 : 5, meta: { ...item.meta } }))}>{text(item, "status") === "MASTERED" ? "继续学习" : "标记掌握"}</button><button className="danger" onClick={() => void props.run((store) => store.delete("english.vocabulary", item.meta.id))}>删除</button></div></article>)}{!filtered.length && <Empty title="暂无生词" description="手动添加或从文章阅读中积累。" />}</div></PageStack>;
}

export function EnglishStatsPage({ state }: { state: CloudPageProps["state"] }) {
  const words = entities(state, "english.vocabulary");
  const records = entities(state, "english.learning_record");
  const highlights = entities(state, "english.highlight");
  const totalSeconds = records.reduce((sum, item) => sum + number(item, "readingTimeSeconds"), 0);
  const mastered = words.filter((item) => text(item, "status") === "MASTERED").length;
  return <PageStack><EnglishTabs /><div className="metric-grid"><Metric label="阅读完成" value={String(records.length)} detail={`${Math.round(totalSeconds / 60)} 分钟`} /><Metric label="生词" value={String(words.length)} detail={`${mastered} 个已掌握`} /><Metric label="高亮" value={String(highlights.length)} detail="文章摘录" /><Metric label="平均阅读" value={`${records.length ? Math.round(totalSeconds / records.length / 60) : 0} 分钟`} detail="每篇文章" /></div><Panel title="最近总结" eyebrow="LEARNING RECORDS"><div className="timeline">{[...records].sort((a, b) => b.meta.updatedAt.localeCompare(a.meta.updatedAt)).map((item) => <div className="timeline-item" key={item.meta.id}><span>{text(item, "recordDate")}</span><div><strong>{text(item, "summary") || "未填写总结"}</strong><small>{number(item, "readingTimeSeconds")} 秒</small></div></div>)}{!records.length && <Empty title="暂无学习记录" description="完成文章阅读并提交总结后会显示统计。" />}</div></Panel></PageStack>;
}
