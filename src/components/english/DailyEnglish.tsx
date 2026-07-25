"use client";

import { useEffect, useState } from "react";
import {
  ArrowLeft, BarChart3, BookOpen, Bot, Check, ChevronRight, Clock3, Flame,
  ExternalLink, Headphones, Highlighter, Languages, Library, ListChecks, Minus, Moon, NotebookPen,
  Plus, RefreshCw, Sparkles, Sun, Type, Volume2,
} from "lucide-react";
import type {
  ArticleVocabularyItem,
  CEFRLevel,
  EnglishAIAnalysis,
  EnglishArticle,
  EnglishHistoryResponse,
  EnglishSourceSyncResult,
  EnglishTodayResponse,
  EnglishVocabulary,
} from "@/src/types/english";

type EnglishView = "overview" | "reader" | "summary" | "feedback" | "vocabulary" | "history" | "articles" | "assistant";

// 统一处理每日英语接口错误，页面组件只关心业务数据。
const request = async <T,>(input: RequestInfo, init?: RequestInit): Promise<T> => {
  const response = await fetch(input, init);
  const payload = await response.json() as T & { error?: string };
  if (!response.ok) throw new Error(payload.error ?? "每日英语服务暂时不可用");
  return payload;
};

const post = <T,>(url: string, body: unknown, method = "POST") => request<T>(url, {
  method,
  headers: { "content-type": "application/json" },
  body: JSON.stringify(body),
});

const levelName: Record<CEFRLevel, string> = {
  A1: "Beginner",
  A2: "Elementary",
  B1: "Intermediate",
  B2: "Upper Intermediate",
  C1: "Advanced",
};

const categoryName: Record<EnglishArticle["category"], string> = {
  Technology: "科技",
  Science: "科学",
  Life: "生活",
  Business: "商业",
  Culture: "文化",
};

const localDateKey = (date = new Date()) => {
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60000);
  return local.toISOString().slice(0, 10);
};

const currentWeek = () => Array.from({ length: 7 }, (_, index) => {
  const date = new Date();
  const mondayOffset = (date.getDay() + 6) % 7;
  date.setDate(date.getDate() - mondayOffset + index);
  return { key: localDateKey(date), label: ["一", "二", "三", "四", "五", "六", "日"][index] };
});

export default function DailyEnglish() {
  const [view, setView] = useState<EnglishView>("overview");
  const [today, setToday] = useState<EnglishTodayResponse | null>(null);
  const [history, setHistory] = useState<EnglishHistoryResponse | null>(null);
  const [articles, setArticles] = useState<EnglishArticle[]>([]);
  const [vocabulary, setVocabulary] = useState<EnglishVocabulary[]>([]);
  const [assistant, setAssistant] = useState<{ sampleSize: number; weakPoints: string[]; message: string; nextStage: string } | null>(null);
  const [currentArticle, setCurrentArticle] = useState<EnglishArticle | null>(null);
  const [summary, setSummary] = useState("");
  const [analysis, setAnalysis] = useState<EnglishAIAnalysis | null>(null);
  const [recordId, setRecordId] = useState<string>();
  const [readingStartedAt, setReadingStartedAt] = useState(() => Date.now());
  const [message, setMessage] = useState("");
  const [loading, setLoading] = useState(true);
  const [syncingSource, setSyncingSource] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      const [todayData, historyData, articleData, vocabularyData, assistantData] = await Promise.all([
        request<EnglishTodayResponse>("/api/english/today"),
        request<EnglishHistoryResponse>("/api/english/history"),
        request<{ articles: EnglishArticle[] }>("/api/english/articles"),
        request<{ items: EnglishVocabulary[] }>("/api/english/vocabulary"),
        request<{ sampleSize: number; weakPoints: string[]; message: string; nextStage: string }>("/api/english/assistant"),
      ]);
      setToday(todayData);
      setHistory(historyData);
      setArticles(articleData.articles);
      setVocabulary(vocabularyData.items);
      setAssistant(assistantData);
      setCurrentArticle((value) => value ?? todayData.article);
      if (todayData.record) {
        setRecordId(todayData.record.id);
        setSummary(todayData.record.summary);
      }
      // 第三方来源在后台刷新；失败不会阻塞内置文章和已缓存文章。
      void post<EnglishSourceSyncResult>("/api/english/sync", { force: false })
        .then(async (result) => {
          if (!result.imported) return;
          const refreshed = await request<{ articles: EnglishArticle[] }>("/api/english/articles");
          setArticles(refreshed.articles);
        })
        .catch(() => undefined);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "每日英语加载失败");
    } finally {
      setLoading(false);
    }
  };

  const syncVoa = async () => {
    setSyncingSource(true);
    setMessage("正在调用本机 Python 抓取 VOA 文章…");
    try {
      const result = await post<EnglishSourceSyncResult>("/api/english/sync", { force: true });
      const articleData = await request<{ articles: EnglishArticle[] }>("/api/english/articles");
      setArticles(articleData.articles);
      setMessage(result.imported
        ? `Python 已同步 ${result.imported} 篇 VOA 文章`
        : result.cached ? "VOA 文章已经是最新状态" : "本次没有发现可导入的新文章");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "VOA 文章同步失败");
    } finally {
      setSyncingSource(false);
    }
  };

  useEffect(() => {
    const timer = window.setTimeout(() => void load(), 0);
    return () => window.clearTimeout(timer);
  }, []);

  const startReading = (article = today?.article) => {
    if (!article) return;
    setCurrentArticle(article);
    setReadingStartedAt(Date.now());
    setSummary("");
    setRecordId(undefined);
    setAnalysis(null);
    setView("reader");
  };

  if (loading) return <div className="en-loading"><span>EN</span><p>正在准备今天的英语任务…</p></div>;
  if (!today || !currentArticle) return <div className="en-loading error"><span>!</span><p>{message || "暂时没有可用文章"}</p><button onClick={() => void load()}>重新加载</button></div>;

  return <div className="en-module">
    <EnglishNav view={view} setView={setView} />
    {message && <div className="en-message" role="status">{message}</div>}
    {view === "overview" && <Overview today={today} history={history} start={() => startReading(today.article)} setView={setView} />}
    {view === "reader" && <Reader
      article={currentArticle}
      back={() => setView("overview")}
      finish={() => setView("summary")}
      addWord={async (item) => {
        const saved = await post<EnglishVocabulary>("/api/english/vocabulary", { ...item, sourceArticleId: currentArticle.id });
        setVocabulary((items) => items.some((word) => word.id === saved.id) ? items : [saved, ...items]);
        setMessage(`“${saved.word}”已加入生词本`);
      }}
      setMessage={setMessage}
    />}
    {view === "summary" && <SummaryTrainer
      article={currentArticle}
      summary={summary}
      setSummary={setSummary}
      back={() => setView("reader")}
      submit={async () => {
        setMessage("正在保存总结并生成学习反馈…");
        try {
          const record = await post<{ id: string }>("/api/english/summary", {
            articleId: currentArticle.id,
            summary,
            readingTimeSeconds: Math.max(1, Math.floor((Date.now() - readingStartedAt) / 1000)),
            recordId,
          });
          setRecordId(record.id);
          const result = await post<{ analysis: EnglishAIAnalysis }>("/api/english/analyze", {
            recordId: record.id,
            userLevel: today.currentLevel,
          });
          setAnalysis(result.analysis);
          setMessage("");
          setView("feedback");
          await load();
        } catch (error) {
          setMessage(error instanceof Error ? error.message : "AI 反馈生成失败");
        }
      }}
    />}
    {view === "feedback" && analysis && <Feedback analysis={analysis} article={currentArticle} done={() => setView("overview")} />}
    {view === "vocabulary" && <VocabularyBook items={vocabulary} review={async (id, mastered) => {
      const updated = await post<EnglishVocabulary>("/api/english/vocabulary", { id, mastered }, "PATCH");
      setVocabulary((items) => items.map((item) => item.id === id ? updated : item));
    }} />}
    {view === "history" && <History history={history} />}
    {view === "articles" && <ArticleLibrary articles={articles} currentLevel={today.currentLevel} start={startReading} syncing={syncingSource} syncVoa={() => void syncVoa()} />}
    {view === "assistant" && <Assistant insight={assistant} history={history} />}
  </div>;
}

function EnglishNav({ view, setView }: { view: EnglishView; setView: (view: EnglishView) => void }) {
  const items: Array<{ id: EnglishView; label: string; icon: typeof BookOpen }> = [
    { id: "overview", label: "今日任务", icon: BookOpen },
    { id: "articles", label: "文章库", icon: Library },
    { id: "vocabulary", label: "生词本", icon: Languages },
    { id: "history", label: "学习记录", icon: BarChart3 },
    { id: "assistant", label: "AI 助手", icon: Bot },
  ];
  return <nav className="en-nav" aria-label="每日英语功能导航">
    {items.map(({ id, label, icon: Icon }) => <button key={id} className={view === id ? "active" : ""} onClick={() => setView(id)}><Icon /><span>{label}</span></button>)}
  </nav>;
}

function Overview({ today, history, start, setView }: {
  today: EnglishTodayResponse;
  history: EnglishHistoryResponse | null;
  start: () => void;
  setView: (view: EnglishView) => void;
}) {
  const week = currentWeek();
  return <div className="en-overview">
    <section className="en-hero">
      <div><span className="en-eyebrow">DAILY ENGLISH</span><h2>每天读一点，<br />把输入变成表达。</h2><p>阅读、英文总结、AI 反馈，完成一次真正有效的语言训练。</p></div>
      <div className="en-streak"><Flame /><strong>{today.streak}</strong><span>连续学习天数</span><small>稳定回来，比一次学很多更重要</small></div>
    </section>
    <div className="en-overview-grid">
      <article className="en-today-card">
        <header><div><span>今日英语任务</span><h3>{today.article.title}</h3></div><b>{today.article.level}</b></header>
        <p>{today.article.content.split("\n")[0]}</p>
        <div className="en-task-meta"><span><BookOpen /> {levelName[today.article.level]}</span><span><Clock3 /> {today.article.estimatedMinutes} 分钟</span><span>{categoryName[today.article.category]}</span></div>
        <button onClick={start}>{today.record?.completionStatus === "completed" ? "再次阅读" : "开始阅读"}<ChevronRight /></button>
      </article>
      <aside className="en-week-card">
        <header><span>本周节奏</span><strong>{today.weekCompleted.length} / 7</strong></header>
        <div>{week.map((day) => <span key={day.key} className={today.weekCompleted.includes(day.key) ? "done" : ""}><b>{day.label}</b><i>{today.weekCompleted.includes(day.key) ? <Check /> : "·"}</i></span>)}</div>
        <p>当前等级 <strong>{today.currentLevel} · {levelName[today.currentLevel]}</strong></p>
      </aside>
    </div>
    <section className="en-dashboard-row">
      <article><span>最近 30 天</span><strong>{history?.stats.readingCount30 ?? 0}<small>篇阅读</small></strong></article>
      <article><span>平均评分</span><strong>{history?.stats.averageScore30 || "—"}<small>/ 100</small></strong></article>
      <article><span>词汇增长</span><strong>{history?.stats.vocabularyGrowth30 ?? 0}<small>个生词</small></strong></article>
    </section>
    <section className="en-recent">
      <header><div><span className="en-eyebrow">HISTORY</span><h3>最近学习记录</h3></div><button onClick={() => setView("history")}>查看全部 <ChevronRight /></button></header>
      {today.recentRecords.length ? today.recentRecords.map((record) => <article key={record.id}><time>{record.date.slice(5)}</time><div><strong>{record.article?.title ?? "英文阅读"}</strong><small>{record.article?.level} · {record.completionStatus === "completed" ? "已完成闭环" : "继续完成"}</small></div><b>{record.score ?? "—"}</b></article>) : <p className="en-empty">完成第一篇文章后，学习轨迹会从这里开始。</p>}
    </section>
  </div>;
}

function Reader({ article, back, finish, addWord, setMessage }: {
  article: EnglishArticle;
  back: () => void;
  finish: () => void;
  addWord: (item: ArticleVocabularyItem) => Promise<void>;
  setMessage: (value: string) => void;
}) {
  const [fontSize, setFontSize] = useState(19);
  const [lineHeight, setLineHeight] = useState(1.9);
  const [dark, setDark] = useState(false);
  const [lookup, setLookup] = useState<ArticleVocabularyItem | null>(null);
  const [selectedText, setSelectedText] = useState("");
  const [note, setNote] = useState("");
  const [counts, setCounts] = useState({ highlights: 0, notes: 0 });

  useEffect(() => {
    void request<{ highlights: unknown[]; notes: unknown[] }>(`/api/english/highlights?articleId=${encodeURIComponent(article.id)}`)
      .then((data) => setCounts({ highlights: data.highlights.length, notes: data.notes.length }))
      .catch(() => undefined);
  }, [article.id]);

  const renderParagraph = (paragraph: string) => paragraph.split(/(\b[A-Za-z][A-Za-z'-]*\b)/g).map((part, index) => {
    if (!/^[A-Za-z][A-Za-z'-]*$/.test(part)) return part;
    const known = article.vocabulary.find((item) => item.word.toLowerCase() === part.toLowerCase());
    return <button className={known ? "key-word" : ""} key={`${part}-${index}`} onClick={() => setLookup(known ?? {
      word: part,
      phonetic: "",
      meaning: "该词未列入今日重点词汇",
      example: "",
    })}>{part}</button>;
  });

  return <div className={`en-reader ${dark ? "dark" : ""}`}>
    <header className="en-reader-bar">
      <button onClick={back}><ArrowLeft />返回</button>
      <div>
        <button onClick={() => setFontSize((value) => Math.max(15, value - 1))}><Minus /><Type /></button>
        <span>{fontSize}px</span>
        <button onClick={() => setFontSize((value) => Math.min(28, value + 1))}><Plus /><Type /></button>
        <button onClick={() => setLineHeight((value) => value >= 2.2 ? 1.6 : value + .2)}>行距 {lineHeight.toFixed(1)}</button>
        <button onClick={() => setDark((value) => !value)}>{dark ? <Sun /> : <Moon />}</button>
      </div>
    </header>
    <main>
      <article className="en-reading-paper">
        <span>{article.level} · {categoryName[article.category]} · {article.estimatedMinutes} MIN</span>
        <h1>{article.title}</h1>
        {(article.sourceUrl || article.author || article.publishedAt || article.wordCount) && <div className="en-article-provenance">
          {article.sourceUrl && <a className="en-article-source" href={article.sourceUrl} target="_blank" rel="noreferrer">
            {article.sourceName ?? "查看文章来源"} <ExternalLink aria-hidden />
          </a>}
          {article.author && <span>{article.author}</span>}
          {article.publishedAt && <time dateTime={article.publishedAt}>{new Date(article.publishedAt).toLocaleDateString("zh-CN")}</time>}
          {article.wordCount && <span>{article.wordCount} words</span>}
        </div>}
        {article.audioUrl && <section className="en-article-audio">
          <span><Headphones aria-hidden /> VOA 原文音频</span>
          <audio controls preload="none" src={article.audioUrl}>当前环境不支持音频播放。</audio>
        </section>}
        <div className="en-reading-content" style={{ fontSize, lineHeight }} onMouseUp={() => {
          const text = window.getSelection()?.toString().trim() ?? "";
          if (text.length > 2) setSelectedText(text);
        }}>{article.content.split("\n\n").map((paragraph, index) => <p key={index}>{renderParagraph(paragraph)}</p>)}</div>
        <button className="en-finish-reading" onClick={finish}>完成阅读，开始英文总结 <ChevronRight /></button>
      </article>
      <aside className="en-reader-side">
        <section>
          <header><Highlighter /><strong>标记与笔记</strong></header>
          {selectedText ? <blockquote>{selectedText}</blockquote> : <p>选择文章中的文字，即可高亮或添加笔记。</p>}
          <button disabled={!selectedText} onClick={async () => {
            try {
              await post("/api/english/highlights", { articleId: article.id, text: selectedText, color: "yellow" });
              setCounts((value) => ({ ...value, highlights: value.highlights + 1 }));
              setMessage("高亮已保存");
            } catch (error) { setMessage(error instanceof Error ? error.message : "高亮保存失败"); }
          }}>高亮所选</button>
          <textarea value={note} onChange={(event) => setNote(event.target.value)} placeholder="写下你的理解或问题…" />
          <button disabled={!note.trim()} onClick={async () => {
            try {
              await post("/api/english/notes", { articleId: article.id, quote: selectedText, content: note });
              setCounts((value) => ({ ...value, notes: value.notes + 1 }));
              setNote("");
              setMessage("笔记已保存");
            } catch (error) { setMessage(error instanceof Error ? error.message : "笔记保存失败"); }
          }}><NotebookPen />保存笔记</button>
          <small>{counts.highlights} 条高亮 · {counts.notes} 条笔记</small>
        </section>
        <section><header><ListChecks /><strong>理解问题</strong></header><ol>{article.questions.map((question) => <li key={question}>{question}</li>)}</ol></section>
      </aside>
    </main>
    {lookup && <div className="en-word-popover" role="dialog">
      <button onClick={() => setLookup(null)}>×</button><span>WORD</span><h3>{lookup.word}</h3>
      <p>{lookup.phonetic} <Volume2 /></p><strong>{lookup.meaning}</strong>
      {lookup.example && <blockquote>{lookup.example}</blockquote>}
      <button className="primary" disabled={!lookup.phonetic} onClick={() => void addWord(lookup)}>加入生词本</button>
    </div>}
  </div>;
}

function SummaryTrainer({ article, summary, setSummary, back, submit }: {
  article: EnglishArticle;
  summary: string;
  setSummary: (value: string) => void;
  back: () => void;
  submit: () => Promise<void>;
}) {
  const [submitting, setSubmitting] = useState(false);
  const wordCount = summary.trim().split(/\s+/).filter(Boolean).length;
  const hasChinese = /[\u3400-\u9fff]/.test(summary);
  const targetOk = wordCount >= 100 && wordCount <= 200;
  return <div className="en-summary">
    <button className="en-back" onClick={back}><ArrowLeft />返回文章</button>
    <div className="en-summary-grid">
      <section><span className="en-eyebrow">OUTPUT TRAINING</span><h2>用自己的英语，<br />重新讲一遍。</h2><p>不要逐句翻译。先找出文章的核心观点，再说明关键原因与结论。</p><article><b>{article.level}</b><div><strong>{article.title}</strong><small>{categoryName[article.category]} · {article.estimatedMinutes} 分钟阅读</small></div></article><ul><li>覆盖文章的主要观点</li><li>使用完整英文句子</li><li>目标长度 100–200 词</li></ul></section>
      <section className="en-summary-editor">
        <header><div><span>Write a summary of this article</span><strong className={targetOk ? "ok" : ""}>{wordCount} / 100–200 words</strong></div><small>{summary.length} characters</small></header>
        <textarea autoFocus value={summary} onChange={(event) => setSummary(event.target.value)} placeholder="The article explains that…" />
        {hasChinese && <p className="error">总结必须全部使用英文。</p>}
        <footer><span>{wordCount < 100 ? `还建议补充 ${100 - wordCount} 词` : wordCount > 200 ? `建议删减 ${wordCount - 200} 词` : "长度符合目标"}</span><button disabled={submitting || wordCount < 20 || hasChinese} onClick={async () => {
          setSubmitting(true);
          try { await submit(); } finally { setSubmitting(false); }
        }}><Sparkles />{submitting ? "正在分析…" : "提交并生成 AI 反馈"}</button></footer>
      </section>
    </div>
  </div>;
}

function Feedback({ analysis, article, done }: { analysis: EnglishAIAnalysis; article: EnglishArticle; done: () => void }) {
  const metrics = [["内容", analysis.contentScore], ["语法", analysis.grammarScore], ["词汇", analysis.vocabularyScore], ["结构", analysis.structureScore]] as const;
  return <div className="en-feedback">
    <section className="en-score-hero"><div><span>AI LEARNING FEEDBACK · MOCK</span><h2>{article.title}</h2><p>本次输出已经沉淀到学习历史，并同步完成“每日英语”坚持记录。</p></div><div className="en-score-ring" style={{ "--score": `${analysis.score}%` } as React.CSSProperties}><strong>{analysis.score}</strong><small>/ 100</small></div></section>
    <div className="en-score-grid">{metrics.map(([label, score]) => <article key={label}><span>{label}</span><strong>{score}</strong><i><b style={{ width: `${score}%` }} /></i></article>)}</div>
    <div className="en-feedback-grid">
      <section><header><span>主要问题</span><b>{analysis.mistakes.length}</b></header>{analysis.mistakes.length ? analysis.mistakes.map((mistake, index) => <article key={`${mistake.original}-${index}`}><span>{index + 1}</span><div><del>{mistake.original}</del><strong>{mistake.correction}</strong><p>{mistake.reason}</p></div></article>) : <p className="en-empty">没有发现明显基础错误，继续关注表达的丰富度。</p>}</section>
      <section><header><span>改进建议</span><Sparkles /></header><ol>{analysis.suggestions.map((suggestion) => <li key={suggestion}>{suggestion}</li>)}</ol><div className="en-connectors"><span>TRY THESE</span><b>although</b><b>however</b><b>therefore</b></div></section>
    </div>
    <section className="en-reference"><header><span>参考版本</span><button onClick={() => void navigator.clipboard?.writeText(analysis.improvedSummary)}>复制</button></header><blockquote>{analysis.improvedSummary}</blockquote></section>
    <button onClick={done}>完成本次学习</button>
  </div>;
}

function VocabularyBook({ items, review }: { items: EnglishVocabulary[]; review: (id: string, mastered: boolean) => Promise<void> }) {
  const [referenceTime] = useState(() => Date.now());
  const due = items.filter((item) => new Date(item.nextReviewTime).getTime() <= referenceTime);
  const [reviewItem, setReviewItem] = useState<EnglishVocabulary | null>(due[0] ?? items[0] ?? null);
  const nextRandom = () => setReviewItem((due.length ? due : items)[Math.floor(Math.random() * Math.max(1, (due.length ? due : items).length))] ?? null);
  return <div>
    <div className="en-section-head"><div><span className="en-eyebrow">VOCABULARY</span><h2>生词不是收藏，<br />而是要再次遇见。</h2><p>{items.length} 个生词 · {due.length} 个待复习</p></div></div>
    {reviewItem && <section className="en-vocab-review"><div><span>今日复习</span><h3>{reviewItem.word}</h3><p>{reviewItem.phonetic} · {reviewItem.meaning}</p></div><div><button onClick={async () => { await review(reviewItem.id, false); nextRandom(); }}>还不熟</button><button onClick={async () => { await review(reviewItem.id, true); nextRandom(); }}>已掌握</button><button onClick={nextRandom}><RefreshCw /></button></div></section>}
    <div className="en-vocab-grid">{items.map((item) => <article key={item.id}><h3>{item.word}</h3><span>{item.phonetic}</span><p>{item.meaning}</p><blockquote>{item.example}</blockquote><footer><i><b style={{ width: `${item.masterLevel * 20}%` }} /></i><small>熟练度 {item.masterLevel}/5</small></footer></article>)}</div>
    {!items.length && <p className="en-empty">阅读时点击重点词汇，即可加入生词本。</p>}
  </div>;
}

function History({ history }: { history: EnglishHistoryResponse | null }) {
  const records = history?.records ?? [];
  const chart = Array.from({ length: 30 }, (_, index) => {
    const date = new Date();
    date.setDate(date.getDate() - (29 - index));
    return records.some((record) => record.date === localDateKey(date)) ? 100 : 8;
  });
  return <div>
    <div className="en-section-head"><div><span className="en-eyebrow">LEARNING CURVE</span><h2>看见长期积累。</h2><p>每次阅读、总结和反馈都会成为可追踪的成长数据。</p></div></div>
    <section className="en-history-stats"><article><span>30 天阅读</span><strong>{history?.stats.readingCount30 ?? 0}</strong></article><article><span>平均评分</span><strong>{history?.stats.averageScore30 || "—"}</strong></article><article><span>词汇增长</span><strong>+{history?.stats.vocabularyGrowth30 ?? 0}</strong></article></section>
    <div className="en-chart" aria-label="最近 30 天学习次数">{chart.map((height, index) => <i key={index} style={{ height: `${height}%` }} />)}</div>
    <section className="en-history-list">{records.map((record) => <article key={record.id}><time>{record.date.slice(5)}</time><div><strong>{record.article?.title ?? "英文阅读"}</strong><small>{record.summary ? `${record.summary.split(/\s+/).filter(Boolean).length} 词总结` : "尚未提交总结"}</small></div><small>{Math.round(record.readingTimeSeconds / 60)} 分钟</small><b>{record.score ?? "—"}</b></article>)}</section>
    {!records.length && <p className="en-empty">还没有学习记录，先完成今天的文章吧。</p>}
  </div>;
}

function ArticleLibrary({ articles, currentLevel, start, syncing, syncVoa }: {
  articles: EnglishArticle[];
  currentLevel: CEFRLevel;
  start: (article: EnglishArticle) => void;
  syncing: boolean;
  syncVoa: () => void;
}) {
  const [level, setLevel] = useState<CEFRLevel | "all">("all");
  const [query, setQuery] = useState("");
  const shown = articles.filter((article) => (level === "all" || article.level === level) && article.title.toLowerCase().includes(query.toLowerCase()));
  return <div>
    <div className="en-section-head"><div><span className="en-eyebrow">ARTICLE LIBRARY</span><h2>按你的水平，<br />选择下一篇文章。</h2><p>当前推荐等级：{currentLevel} · {levelName[currentLevel]}</p></div><div><button className="en-source-sync" type="button" disabled={syncing} onClick={syncVoa}><RefreshCw className={syncing ? "syncing" : ""} />{syncing ? "同步中" : "同步 VOA"}</button><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索文章" /><select value={level} onChange={(event) => setLevel(event.target.value as CEFRLevel | "all")}><option value="all">全部等级</option>{(["A1", "A2", "B1", "B2", "C1"] as CEFRLevel[]).map((item) => <option value={item} key={item}>{item}</option>)}</select></div></div>
    <div className="en-article-grid">{shown.map((article) => <article key={article.id}><span>{article.level} · {categoryName[article.category]}</span>{article.source === "voa" && <small className="en-source-badge">VOA Learning English</small>}<h3>{article.title}</h3><p>{article.content}</p><footer><small>{article.estimatedMinutes} 分钟 · 难度 {article.difficulty}/5</small><button onClick={() => start(article)}>阅读 <ChevronRight /></button></footer></article>)}</div>
  </div>;
}

function Assistant({ insight, history }: {
  insight: { sampleSize: number; weakPoints: string[]; message: string; nextStage: string } | null;
  history: EnglishHistoryResponse | null;
}) {
  return <div>
    <div className="en-section-head"><div><span className="en-eyebrow">AI ENGLISH ASSISTANT</span><h2>从历史中找到，<br />下一步该练什么。</h2></div></div>
    <section className="en-assistant-card"><div className="en-assistant-orb"><Bot /></div><div><h2>{insight?.message ?? "完成学习后，我会分析你的薄弱点。"}</h2><p>分析样本：最近 {insight?.sampleSize ?? 0} 篇总结 · 当前平均分 {history?.stats.averageScore30 || "—"}</p>{Boolean(insight?.weakPoints.length) && <ol>{insight?.weakPoints.map((point) => <li key={point}>{point}</li>)}</ol>}<blockquote>{insight?.nextStage ?? "先完成第一篇阅读与英文总结，系统会开始建立你的能力画像。"}</blockquote></div></section>
  </div>;
}
