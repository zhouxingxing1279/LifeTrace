"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowLeft, BarChart3, BookOpen, Bot, Check, CheckCircle2, ChevronRight, Clock3, Edit3, Flame,
  ExternalLink, Headphones, Highlighter, Languages, Library, ListChecks, Minus, Moon, NotebookPen,
  Plus, Sparkles, Sun, Trash2, Type, X,
} from "lucide-react";
import type {
  CEFRLevel,
  DictionaryLookup,
  EnglishAIAnalysis,
  EnglishArticle,
  EnglishHighlight,
  EnglishHistoryResponse,
  EnglishLearningRecord,
  EnglishNote,
  EnglishReadingStatus,
  EnglishTextAnchor,
  EnglishTodayResponse,
  UserVocabulary,
  VocabularySettings,
} from "@/src/types/english";
import {
  buildAnnotationSegments,
  resolveAnnotations,
  type ArticleTextBlock,
} from "./annotationRanges";
import { splitArticleReadingSections } from "./articleContent";
import { DictionaryPopover } from "./vocabulary/DictionaryPopover";
import { VocabularyWorkspace } from "./vocabulary/VocabularyWorkspace";

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

type ToastKind = "success" | "info" | "warning" | "error";
const notifyEnglish = (message: string, type: ToastKind = "info") => {
  window.dispatchEvent(new CustomEvent("hengxu-toast", {
    detail: {
      message,
      type,
      duration: type === "success" ? 2000 : type === "info" ? 2500 : type === "warning" ? 3000 : 4500,
    },
  }));
};

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
  const [vocabularyVersion, setVocabularyVersion] = useState(0);
  const [vocabularyMode, setVocabularyMode] = useState<"list" | "review">("list");
  const [vocabularyStats, setVocabularyStats] = useState({ dueToday: 0, addedWeek: 0, mastered: 0 });
  const [assistant, setAssistant] = useState<{ sampleSize: number; weakPoints: string[]; message: string; nextStage: string } | null>(null);
  const [currentArticle, setCurrentArticle] = useState<EnglishArticle | null>(null);
  const [summary, setSummary] = useState("");
  const [analysis, setAnalysis] = useState<EnglishAIAnalysis | null>(null);
  const [recordId, setRecordId] = useState<string>();
  const [readingStartedAt, setReadingStartedAt] = useState(() => Date.now());
  const [message, setMessage] = useState("");
  const [loading, setLoading] = useState(true);

  const load = async () => {
    setLoading(true);
    try {
      const [todayData, historyData, vocabularyStatsData, assistantData] = await Promise.all([
        request<EnglishTodayResponse>("/api/english/today"),
        request<EnglishHistoryResponse>("/api/english/history"),
        request<{ dueToday: number; addedWeek: number; mastered: number }>("/api/english/vocabulary/stats"),
        request<{ sampleSize: number; weakPoints: string[]; message: string; nextStage: string }>("/api/english/assistant"),
      ]);
      setToday(todayData);
      setHistory(historyData);
      setVocabularyStats(vocabularyStatsData);
      setAssistant(assistantData);
      setCurrentArticle((value) => value ?? todayData.article);
      // 第三方来源只在后台做增量刷新；一次性历史建库不属于应用启动流程。
      void post<{ taskId?: string }>("/api/english/sync", { startupCheck: true })
        .catch(() => undefined);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "每日英语加载失败");
    } finally {
      setLoading(false);
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

  const handleRecordChange = useCallback((record: EnglishLearningRecord) => {
    setRecordId(record.id);
    setSummary(record.summary);
    setToday((current) => {
      if (!current) return current;
      const existingRecent = current.recentRecords.find((item) => item.id === record.id);
      const article = existingRecent?.article
        ?? (current.article.id === record.articleId ? current.article : currentArticle?.id === record.articleId ? currentArticle : undefined);
      const recentRecord = { ...existingRecent, ...record, article };
      const recentRecords = [recentRecord, ...current.recentRecords.filter((item) => item.id !== record.id)].slice(0, 5);
      const completed = record.readingStatus === "completed" || record.completionStatus === "completed";
      return {
        ...current,
        ...(current.article.id === record.articleId ? { record } : {}),
        recentRecords,
        weekCompleted: completed
          ? [...new Set([...current.weekCompleted, record.date])]
          : current.weekCompleted,
      };
    });
    setHistory((current) => {
      if (!current) return current;
      const existing = current.records.find((item) => item.id === record.id);
      const nextRecord = {
        ...existing,
        ...record,
        article: existing?.article ?? (currentArticle?.id === record.articleId ? currentArticle : undefined),
      };
      const records = [nextRecord, ...current.records.filter((item) => item.id !== record.id)];
      const since = new Date();
      since.setDate(since.getDate() - 29);
      const sinceKey = localDateKey(since);
      const completed = records.filter((item) =>
        item.date >= sinceKey && (item.readingStatus === "completed" || item.completionStatus === "completed"),
      );
      const scored = completed.filter((item) => typeof item.score === "number");
      return {
        ...current,
        records,
        stats: {
          ...current.stats,
          readingCount30: completed.length,
          averageScore30: scored.length
            ? Math.round(scored.reduce((sum, item) => sum + (item.score ?? 0), 0) / scored.length)
            : 0,
        },
      };
    });
  }, [currentArticle]);

  if (loading) return <div className="en-loading"><span>EN</span><p>正在准备今天的英语任务…</p></div>;
  if (!today || !currentArticle) return <div className="en-loading error"><span>!</span><p>{message || "暂时没有可用文章"}</p><button onClick={() => void load()}>重新加载</button></div>;

  return <div className="en-module">
    <EnglishNav view={view} setView={setView} />
    {view === "overview" && <Overview today={today} history={history} vocabularyStats={vocabularyStats} start={() => startReading(today.article)} setView={setView} openVocabulary={(mode) => { setVocabularyMode(mode); setView("vocabulary"); }} />}
    {view === "reader" && <Reader
      key={currentArticle.id}
      article={currentArticle}
      back={() => setView("overview")}
      finish={() => setView("summary")}
      onWordAdded={() => setVocabularyVersion((value) => value + 1)}
      onRecordChange={handleRecordChange}
      setMessage={(value) => notifyEnglish(value, /失败|错误|无法/.test(value) ? "error" : "success")}
    />}
    {view === "summary" && <SummaryTrainer
      article={currentArticle}
      summary={summary}
      setSummary={setSummary}
      back={() => setView("reader")}
      submit={async () => {
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
          setView("feedback");
          await load();
        } catch (error) {
          notifyEnglish(error instanceof Error ? error.message : "AI 反馈生成失败", "error");
        }
      }}
    />}
    {view === "feedback" && analysis && <Feedback analysis={analysis} article={currentArticle} done={() => setView("overview")} />}
    {view === "vocabulary" && <VocabularyWorkspace refreshKey={vocabularyVersion} initialMode={vocabularyMode} />}
    {view === "history" && <History history={history} open={async (articleId) => {
      const article = await request<EnglishArticle>(`/api/english/articles?id=${encodeURIComponent(articleId)}`);
      startReading(article);
    }} />}
    {view === "articles" && <ArticleLibrary
      currentLevel={today.currentLevel}
      start={startReading}
    />}
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

function Overview({ today, history, vocabularyStats, start, setView, openVocabulary }: {
  today: EnglishTodayResponse;
  history: EnglishHistoryResponse | null;
  start: () => void;
  setView: (view: EnglishView) => void;
  vocabularyStats: { dueToday: number; addedWeek: number; mastered: number };
  openVocabulary: (mode: "list" | "review") => void;
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
        <button onClick={start}>{today.record?.readingStatus === "completed" || today.record?.completionStatus === "completed" ? "再次阅读" : today.record ? "继续阅读" : "开始阅读"}<ChevronRight /></button>
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
    <section className="en-vocab-home-card">
      <div><span className="en-eyebrow">WORD REVIEW</span><h3>今天也把几个词，真正记下来。</h3></div>
      <dl><div><dt>今日待复习</dt><dd>{vocabularyStats.dueToday}</dd></div><div><dt>本周新增</dt><dd>{vocabularyStats.addedWeek}</dd></div><div><dt>已掌握</dt><dd>{vocabularyStats.mastered}</dd></div></dl>
      <footer><button onClick={() => openVocabulary("review")}>开始今日复习</button><button onClick={() => openVocabulary("list")}>查看生词本</button></footer>
    </section>
    <section className="en-recent">
      <header><div><span className="en-eyebrow">HISTORY</span><h3>最近学习记录</h3></div><button onClick={() => setView("history")}>查看全部 <ChevronRight /></button></header>
      {today.recentRecords.length ? today.recentRecords.map((record) => <article key={record.id}><time>{record.date.slice(5)}</time><div><strong>{record.article?.title ?? "英文阅读"}</strong><small>{record.article?.level} · {record.readingStatus === "completed" || record.completionStatus === "completed" ? "已阅读" : "阅读中"}</small></div><b>{record.score ?? "—"}</b></article>) : <p className="en-empty">完成第一篇文章后，学习轨迹会从这里开始。</p>}
    </section>
  </div>;
}

function Reader({ article, back, finish, onWordAdded, onRecordChange, setMessage }: {
  article: EnglishArticle;
  back: () => void;
  finish: () => void;
  onWordAdded: (item: UserVocabulary) => void;
  onRecordChange: (record: EnglishLearningRecord) => void;
  setMessage: (value: string) => void;
}) {
  const [fontSize, setFontSize] = useState(19);
  const [lineHeight, setLineHeight] = useState(1.9);
  const [dark, setDark] = useState(false);
  const [lookup, setLookup] = useState<DictionaryLookup | null>(null);
  const [lookupLoading, setLookupLoading] = useState(false);
  const [vocabularySettings, setVocabularySettings] = useState<VocabularySettings>({
    preferredAccent: "en-US", wordSpeechRate: .8, sentenceSpeechRate: .85, autoPronounce: false,
    defaultFirstMeaning: true, dailyReviewLimit: 20, showSourceSentence: true, includeMasteredInRecommendations: false,
  });
  const [highlights, setHighlights] = useState<EnglishHighlight[]>([]);
  const [notes, setNotes] = useState<EnglishNote[]>([]);
  const [record, setRecord] = useState<EnglishLearningRecord>();
  const [readingStatus, setReadingStatus] = useState<EnglishReadingStatus>("unread");
  const [selectionAction, setSelectionAction] = useState<{
    anchor: EnglishTextAnchor;
    left: number;
    top: number;
  }>();
  const [activeHighlight, setActiveHighlight] = useState<{
    highlight: EnglishHighlight;
    left: number;
    top: number;
  }>();
  const [noteDraft, setNoteDraft] = useState<{
    note?: EnglishNote;
    anchor?: EnglishTextAnchor;
    highlightId?: string;
    left: number;
    top: number;
  }>();
  const [translationCard, setTranslationCard] = useState<{
    anchor: EnglishTextAnchor;
    translatedText?: string;
    left: number;
    top: number;
    loading: boolean;
    saving: boolean;
  }>();
  const [noteContent, setNoteContent] = useState("");
  const [savingHighlight, setSavingHighlight] = useState(false);
  const [savingNote, setSavingNote] = useState(false);
  const [completing, setCompleting] = useState(false);
  const [completionVisible, setCompletionVisible] = useState(false);
  const [focusedAnnotationId, setFocusedAnnotationId] = useState<string>();
  const noteInputRef = useRef<HTMLTextAreaElement>(null);
  const focusTimerRef = useRef<number | undefined>(undefined);
  const completionTimerRef = useRef<number | undefined>(undefined);
  const readingStartedAtRef = useRef(0);
  const onRecordChangeRef = useRef(onRecordChange);
  const setMessageRef = useRef(setMessage);
  const readingSections = useMemo(() => splitArticleReadingSections(article.content), [article.content]);
  const blocks = useMemo<ArticleTextBlock[]>(() => [
    ...readingSections.bodyParagraphs.map((text, index) => ({ id: `body-${index}`, text })),
    ...readingSections.vocabularyLines.map((text, index) => ({ id: `vocabulary-${index}`, text })),
  ], [readingSections]);
  const resolvedHighlights = useMemo(
    () => resolveAnnotations(highlights, blocks),
    [blocks, highlights],
  );
  const resolvedNotes = useMemo(
    () => resolveAnnotations(notes, blocks),
    [blocks, notes],
  );

  useEffect(() => {
    onRecordChangeRef.current = onRecordChange;
    setMessageRef.current = setMessage;
  }, [onRecordChange, setMessage]);

  useEffect(() => {
    const controller = new AbortController();
    readingStartedAtRef.current = Date.now();

    void Promise.all([
      request<{ highlights: EnglishHighlight[]; notes: EnglishNote[] }>(
        `/api/english/highlights?articleId=${encodeURIComponent(article.id)}`,
        { signal: controller.signal },
      ),
      request<{ status: EnglishReadingStatus; record: EnglishLearningRecord }>(
        "/api/english/reading",
        {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ articleId: article.id, action: "start" }),
          signal: controller.signal,
        },
      ),
      request<VocabularySettings>("/api/english/vocabulary/settings", { signal: controller.signal }),
    ]).then(([annotations, reading, settings]) => {
      setHighlights(annotations.highlights);
      setNotes(annotations.notes);
      setReadingStatus(reading.status);
      setRecord(reading.record);
      setVocabularySettings(settings);
      onRecordChangeRef.current(reading.record);
    }).catch((error) => {
      if (controller.signal.aborted) return;
      setMessageRef.current(error instanceof Error ? error.message : "阅读数据加载失败");
    });

    return () => controller.abort();
  }, [article.id]);

  useEffect(() => {
    if (!noteDraft) return;
    const timer = window.setTimeout(() => noteInputRef.current?.focus(), 0);
    return () => window.clearTimeout(timer);
  }, [noteDraft]);

  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (noteDraft) {
        setNoteDraft(undefined);
        setNoteContent("");
      } else if (translationCard) {
        setTranslationCard(undefined);
      } else {
        setSelectionAction(undefined);
        setActiveHighlight(undefined);
      }
      window.getSelection()?.removeAllRanges();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [noteDraft, translationCard]);

  useEffect(() => () => {
    if (focusTimerRef.current) window.clearTimeout(focusTimerRef.current);
    if (completionTimerRef.current) window.clearTimeout(completionTimerRef.current);
  }, []);

  const openWord = async (word: string, sentence: string) => {
    setLookupLoading(true);
    try {
      const params = new URLSearchParams({ word, articleId: article.id, sentence });
      setLookup(await request<DictionaryLookup>(`/api/english/dictionary/lookup?${params}`));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "离线词典查询失败");
    } finally {
      setLookupLoading(false);
    }
  };

  const renderWords = (text: string, keyPrefix: string) => text.split(/(\b[A-Za-z][A-Za-z'-]*\b)/g).map((part, index) => {
    if (!/^[A-Za-z][A-Za-z'-]*$/.test(part)) return part;
    const known = article.vocabulary.some((item) => item.word.toLowerCase() === part.toLowerCase());
    return <span className={`en-reading-word${known ? " key-word" : ""}`} data-reading-word={part} key={`${keyPrefix}-${part}-${index}`}>{part}</span>;
  });

  const highlightsForBlock = (blockId: string) =>
    resolvedHighlights.filter((item) => item.blockId === blockId);
  const notesForBlock = (blockId: string) =>
    resolvedNotes.filter((item) => item.blockId === blockId);

  const renderBlock = (block: ArticleTextBlock) => buildAnnotationSegments(
    block.text,
    highlightsForBlock(block.id),
    notesForBlock(block.id),
  ).map((segment) => {
    const content = renderWords(segment.text, `${block.id}-${segment.startOffset}`);
    const focused = Boolean(focusedAnnotationId)
      && (segment.highlightIds.includes(focusedAnnotationId!) || segment.noteIds.includes(focusedAnnotationId!));
    if (segment.highlightIds.length) {
      return <mark
        className={focused ? "en-reading-highlight focused" : "en-reading-highlight"}
        data-highlight-id={segment.highlightIds[0]}
        data-highlight-ids={segment.highlightIds.join(" ")}
        data-note-ids={segment.noteIds.join(" ")}
        key={`${block.id}-${segment.startOffset}`}
      >{content}</mark>;
    }
    if (segment.noteIds.length) {
      return <span
        className={focused ? "en-reading-note-anchor focused" : "en-reading-note-anchor"}
        data-note-ids={segment.noteIds.join(" ")}
        key={`${block.id}-${segment.startOffset}`}
      >{content}</span>;
    }
    return <span key={`${block.id}-${segment.startOffset}`}>{content}</span>;
  });

  const floatingPosition = (rect: DOMRect, width = 240) => ({
    left: Math.max(12, Math.min(window.innerWidth - width - 12, rect.left + rect.width / 2 - width / 2)),
    top: rect.top > 64 ? rect.top - 50 : rect.bottom + 10,
  });

  const captureSelection = () => {
    const selection = window.getSelection();
    if (!selection || selection.isCollapsed || !selection.rangeCount) return;
    const range = selection.getRangeAt(0);
    const startElement = range.startContainer.nodeType === Node.ELEMENT_NODE
      ? range.startContainer as Element
      : range.startContainer.parentElement;
    const endElement = range.endContainer.nodeType === Node.ELEMENT_NODE
      ? range.endContainer as Element
      : range.endContainer.parentElement;
    const startBlock = startElement?.closest<HTMLElement>("[data-annotation-block]");
    const endBlock = endElement?.closest<HTMLElement>("[data-annotation-block]");
    if (!startBlock || !endBlock || startBlock !== endBlock) {
      setSelectionAction(undefined);
      setMessage("暂不支持跨段落高亮，请在单个段落内选择。");
      return;
    }

    const rawText = range.toString();
    const selectedText = rawText.trim();
    if (!selectedText || !/[\p{L}\p{N}]/u.test(selectedText)) {
      setSelectionAction(undefined);
      setMessage("请选择包含单词的短语或句子。");
      return;
    }
    const leadingWhitespace = rawText.length - rawText.trimStart().length;
    const trailingWhitespace = rawText.length - rawText.trimEnd().length;
    const before = range.cloneRange();
    before.selectNodeContents(startBlock);
    before.setEnd(range.startContainer, range.startOffset);
    const startOffset = before.toString().length + leadingWhitespace;
    const endOffset = before.toString().length + rawText.length - trailingWhitespace;
    const blockText = startBlock.textContent ?? "";
    const rect = range.getBoundingClientRect();
    setSelectionAction({
      anchor: {
        blockId: startBlock.dataset.annotationBlock,
        startOffset,
        endOffset,
        selectedText,
        prefix: blockText.slice(Math.max(0, startOffset - 32), startOffset),
        suffix: blockText.slice(endOffset, endOffset + 32),
      },
      ...floatingPosition(rect),
    });
    setActiveHighlight(undefined);
    setTranslationCard(undefined);
  };

  const createHighlight = async (anchor: EnglishTextAnchor) => {
    if (savingHighlight) return;
    setSavingHighlight(true);
    try {
      const highlight = await post<EnglishHighlight>("/api/english/highlights", {
        articleId: article.id,
        text: anchor.selectedText,
        color: "yellow",
        ...anchor,
      });
      setHighlights((current) => current.some((item) => item.id === highlight.id)
        ? current
        : [highlight, ...current]);
      setSelectionAction(undefined);
      window.getSelection()?.removeAllRanges();
      notifyEnglish("高亮已保存", "success");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "高亮保存失败");
    } finally {
      setSavingHighlight(false);
    }
  };

  const translateSelection = async (selection: NonNullable<typeof selectionAction>) => {
    if (navigator.onLine === false) {
      setSelectionAction(undefined);
      window.getSelection()?.removeAllRanges();
      notifyEnglish("当前无网络，暂不能提供翻译服务", "warning");
      return;
    }

    const cardLeft = Math.max(12, Math.min(window.innerWidth - 372, selection.left - 64));
    const card = {
      anchor: selection.anchor,
      left: cardLeft,
      top: Math.max(12, Math.min(window.innerHeight - 260, selection.top + 48)),
      loading: true,
      saving: false,
    };
    setTranslationCard(card);
    setSelectionAction(undefined);
    setActiveHighlight(undefined);
    window.getSelection()?.removeAllRanges();

    try {
      const response = await fetch("/api/english/translate", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ text: selection.anchor.selectedText }),
      });
      const payload = await response.json() as {
        translatedText?: string;
        error?: string;
        code?: string;
      };
      if (!response.ok || !payload.translatedText) {
        throw new Error(payload.error || "暂时不能提供翻译服务");
      }
      setTranslationCard((current) => current?.anchor === selection.anchor
        ? { ...current, translatedText: payload.translatedText, loading: false }
        : current);
    } catch (error) {
      setTranslationCard(undefined);
      const message = error instanceof Error && error.message === "翻译服务尚未配置"
        ? "翻译服务尚未配置"
        : "当前无法连接网络，暂不能提供翻译服务";
      notifyEnglish(message, "warning");
    }
  };

  const saveTranslationNote = async () => {
    if (!translationCard?.translatedText || translationCard.saving) return;
    const card = translationCard;
    setTranslationCard({ ...card, saving: true });
    try {
      const saved = await post<EnglishNote>("/api/english/notes", {
        articleId: article.id,
        content: card.translatedText,
        quote: card.anchor.selectedText,
        ...card.anchor,
      });
      setNotes((current) => [saved, ...current]);
      setTranslationCard(undefined);
      notifyEnglish("翻译已保存为快捷笔记", "success");
    } catch (error) {
      setTranslationCard((current) => current ? { ...current, saving: false } : current);
      setMessage(error instanceof Error ? error.message : "快捷笔记保存失败");
    }
  };

  const openQuickNote = (
    anchor: EnglishTextAnchor | undefined,
    position: { left: number; top: number },
    note?: EnglishNote,
    highlightId?: string,
  ) => {
    setNoteContent(note?.content ?? "");
    setNoteDraft({ note, anchor, highlightId, ...position });
    setSelectionAction(undefined);
    setActiveHighlight(undefined);
    setTranslationCard(undefined);
    window.getSelection()?.removeAllRanges();
  };

  const saveQuickNote = async () => {
    const content = noteContent.trim();
    if (!content || savingNote || !noteDraft) return;
    setSavingNote(true);
    try {
      const saved = noteDraft.note
        ? await post<EnglishNote>("/api/english/notes", {
            id: noteDraft.note.id,
            articleId: article.id,
            content,
          }, "PATCH")
        : await post<EnglishNote>("/api/english/notes", {
            articleId: article.id,
            content,
            quote: noteDraft.anchor?.selectedText,
            highlightId: noteDraft.highlightId,
            ...noteDraft.anchor,
          });
      setNotes((current) => noteDraft.note
        ? current.map((item) => item.id === saved.id ? saved : item)
        : [saved, ...current]);
      setNoteDraft(undefined);
      setNoteContent("");
      notifyEnglish(noteDraft.note ? "笔记已更新" : "笔记已保存", "success");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "笔记保存失败");
    } finally {
      setSavingNote(false);
    }
  };

  const removeHighlight = async (highlight: EnglishHighlight) => {
    try {
      await request(`/api/english/highlights?articleId=${encodeURIComponent(article.id)}&id=${encodeURIComponent(highlight.id)}`, { method: "DELETE" });
      setHighlights((current) => current.filter((item) => item.id !== highlight.id));
      setActiveHighlight(undefined);
      notifyEnglish("高亮已取消", "success");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "高亮删除失败");
    }
  };

  const removeNote = async (note: EnglishNote) => {
    if (!window.confirm("删除这条快捷笔记？关联高亮会保留。")) return;
    try {
      await request(`/api/english/notes?articleId=${encodeURIComponent(article.id)}&id=${encodeURIComponent(note.id)}`, { method: "DELETE" });
      setNotes((current) => current.filter((item) => item.id !== note.id));
      notifyEnglish("笔记已删除", "success");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "笔记删除失败");
    }
  };

  const focusAnnotation = (kind: "highlight" | "note", id: string) => {
    const escaped = CSS.escape(id);
    const target = document.querySelector<HTMLElement>(
      kind === "highlight" ? `[data-highlight-ids~="${escaped}"]` : `[data-note-ids~="${escaped}"]`,
    );
    if (!target) {
      setMessage("正文已更新，暂时无法精确定位这条旧标注。");
      return;
    }
    target.scrollIntoView({ behavior: "smooth", block: "center" });
    setFocusedAnnotationId(id);
    if (focusTimerRef.current) window.clearTimeout(focusTimerRef.current);
    focusTimerRef.current = window.setTimeout(() => setFocusedAnnotationId(undefined), 1800);
  };

  const completeReading = async () => {
    if (completing || readingStatus === "completed") return;
    setCompleting(true);
    try {
      const result = await post<{
        status: EnglishReadingStatus;
        record: EnglishLearningRecord;
        transitioned: boolean;
      }>("/api/english/reading", {
        articleId: article.id,
        action: "complete",
        readingTimeSeconds: Math.max(1, Math.floor((Date.now() - readingStartedAtRef.current) / 1000)),
      });
      setReadingStatus(result.status);
      setRecord(result.record);
      onRecordChangeRef.current(result.record);
      if (result.transitioned) {
        setCompletionVisible(true);
        if (completionTimerRef.current) window.clearTimeout(completionTimerRef.current);
        completionTimerRef.current = window.setTimeout(() => setCompletionVisible(false), 2600);
      }
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "完成阅读保存失败");
    } finally {
      setCompleting(false);
    }
  };

  const formatCompletedAt = (value?: string) => value
    ? new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit" }).format(new Date(value))
    : "";

  return <div className={`en-reader ${dark ? "dark" : ""}`} onMouseDown={(event) => {
    const target = event.target as HTMLElement;
    if (!target.closest(".en-dictionary-popover") && !target.closest(".en-reading-word")) setLookup(null);
    if (
      !target.closest(".en-selection-toolbar")
      && !target.closest(".en-highlight-toolbar")
      && !target.closest(".en-quick-note-card")
      && !target.closest(".en-translation-card")
      && !target.closest(".en-reading-content")
    ) {
      setSelectionAction(undefined);
      setActiveHighlight(undefined);
      setTranslationCard(undefined);
    }
  }}>
    <header className="en-reader-bar">
      <button onClick={back}><ArrowLeft />返回</button>
      <div>
        <button aria-label="减小正文字号" onClick={() => setFontSize((value) => Math.max(15, value - 1))}><Minus /><Type /></button>
        <span>{fontSize}px</span>
        <button aria-label="增大正文字号" onClick={() => setFontSize((value) => Math.min(28, value + 1))}><Plus /><Type /></button>
        <button onClick={() => setLineHeight((value) => value >= 2.2 ? 1.6 : value + .2)}>行距 {lineHeight.toFixed(1)}</button>
        <button aria-label={dark ? "切换浅色阅读" : "切换深色阅读"} onClick={() => setDark((value) => !value)}>{dark ? <Sun /> : <Moon />}</button>
      </div>
    </header>
    <main>
      <article className="en-reading-paper">
        <span>{article.level} · {categoryName[article.category]} · {article.estimatedMinutes} MIN</span>
        <h1>{article.title}</h1>
        <div className={`en-reading-status ${readingStatus}`}>
          {readingStatus === "completed" ? <><CheckCircle2 />已阅读{record?.completedAt && <time dateTime={record.completedAt}>{formatCompletedAt(record.completedAt)} 完成</time>}</> : <><BookOpen />阅读中</>}
        </div>
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
        <div className="en-reading-content" style={{ fontSize, lineHeight }} onClick={(event) => {
          const mark = (event.target as HTMLElement).closest<HTMLElement>("[data-highlight-id]");
          if (mark && !window.getSelection()?.toString().trim()) {
            const highlight = highlights.find((item) => item.id === mark.dataset.highlightId);
            if (highlight) {
              setActiveHighlight({ highlight, ...floatingPosition(mark.getBoundingClientRect(), 270) });
              setSelectionAction(undefined);
            }
            return;
          }
          const word = (event.target as HTMLElement).closest<HTMLElement>("[data-reading-word]")?.dataset.readingWord;
          if (!word || window.getSelection()?.toString().trim()) return;
          const paragraph = (event.target as HTMLElement).closest("p")?.textContent ?? word;
          void openWord(word, paragraph);
        }} onMouseUp={captureSelection}>
          {readingSections.bodyParagraphs.map((paragraph, index) => {
            const block = { id: `body-${index}`, text: paragraph };
            return <p data-annotation-block={block.id} key={block.id}>{renderBlock(block)}</p>;
          })}
          {readingSections.vocabularyLines.length > 0 && <section className="en-article-glossary" aria-labelledby="en-article-glossary-title">
            <header>
              <span>VOA LEARNING ENGLISH</span>
              <h2 id="en-article-glossary-title">本文词汇</h2>
              <p>下面是原文附带的重点单词与英文释义。</p>
            </header>
            <div>{readingSections.vocabularyLines.map((line, index) => {
              const block = { id: `vocabulary-${index}`, text: line };
              return <p data-annotation-block={block.id} key={block.id}>{renderBlock(block)}</p>;
            })}</div>
          </section>}
        </div>
        <div className="en-reading-completion-actions">
          <button className="en-finish-reading" disabled={completing || readingStatus === "completed"} onClick={() => void completeReading()}>
            {completing ? "正在保存…" : readingStatus === "completed" ? <><Check />已完成阅读</> : <>完成阅读 <ChevronRight /></>}
          </button>
          {readingStatus === "completed" && <button className="en-continue-summary" onClick={finish}>{record?.summary ? "查看或编辑英文总结" : "开始英文总结"} <ChevronRight /></button>}
        </div>
      </article>
      <aside className="en-reader-side">
        <section>
          <header><NotebookPen /><strong>阅读笔记</strong><small>{notes.length} 条快捷笔记</small></header>
          {record?.summary
            ? <div className="en-reading-summary"><p>{record.summary}</p><button onClick={finish}><Edit3 />编辑总结</button></div>
            : <p>完成阅读后，可以继续写下整篇文章的英文总结。</p>}
          <div className="en-side-subhead"><span>快捷笔记</span></div>
          {notes.length
            ? <div className="en-quick-note-list">{notes.map((item) => <article key={item.id}>
                <button className="en-note-jump" onClick={() => focusAnnotation("note", item.id)}>
                  <p>{item.content}</p>
                  <time dateTime={item.updatedAt}>{new Date(item.updatedAt).toLocaleDateString("zh-CN", { month: "2-digit", day: "2-digit" })}</time>
                </button>
                <div>
                  <button aria-label="编辑快捷笔记" onClick={(event) => openQuickNote(
                    item.selectedText || item.quote ? {
                      blockId: item.blockId,
                      startOffset: item.startOffset,
                      endOffset: item.endOffset,
                      selectedText: item.selectedText ?? item.quote ?? "",
                      prefix: item.prefix,
                      suffix: item.suffix,
                    } : undefined,
                    floatingPosition(event.currentTarget.getBoundingClientRect(), 310),
                    item,
                    item.highlightId,
                  )}><Edit3 /></button>
                  <button aria-label="删除快捷笔记" onClick={() => void removeNote(item)}><Trash2 /></button>
                </div>
              </article>)}</div>
            : <p className="en-annotation-empty">选中文章中的短语，即可添加快捷笔记。</p>}
          <div className="en-side-subhead"><span>高亮</span><small>{highlights.length} 处</small></div>
          {highlights.length > 0 && <div className="en-highlight-list">{highlights.map((item) => <article key={item.id}>
            <button title={item.text} onClick={() => focusAnnotation("highlight", item.id)}>{item.text}</button>
            <button aria-label="取消高亮" onClick={() => void removeHighlight(item)}><Trash2 /></button>
          </article>)}</div>}
        </section>
        <section><header><ListChecks /><strong>理解问题</strong></header><ol>{article.questions.map((question) => <li key={question}>{question}</li>)}</ol></section>
      </aside>
    </main>
    {selectionAction && <div className="en-selection-toolbar" role="toolbar" aria-label="文本操作" style={{ left: selectionAction.left, top: selectionAction.top }} onMouseDown={(event) => event.preventDefault()}>
      <button disabled={savingHighlight} onClick={() => void createHighlight(selectionAction.anchor)}><Highlighter />{savingHighlight ? "保存中…" : "高亮"}</button>
      <button onClick={() => void translateSelection(selectionAction)}><Languages />翻译</button>
      <button onClick={() => openQuickNote(selectionAction.anchor, { left: selectionAction.left, top: selectionAction.top + 48 })}><NotebookPen />添加笔记</button>
    </div>}
    {activeHighlight && <div className="en-highlight-toolbar" role="toolbar" aria-label="高亮操作" style={{ left: activeHighlight.left, top: activeHighlight.top }} onMouseDown={(event) => event.preventDefault()}>
      <button onClick={() => openQuickNote({
        blockId: activeHighlight.highlight.blockId,
        startOffset: activeHighlight.highlight.startOffset,
        endOffset: activeHighlight.highlight.endOffset,
        selectedText: activeHighlight.highlight.selectedText ?? activeHighlight.highlight.text,
        prefix: activeHighlight.highlight.prefix,
        suffix: activeHighlight.highlight.suffix,
      }, { left: activeHighlight.left, top: activeHighlight.top + 48 }, undefined, activeHighlight.highlight.id)}><NotebookPen />添加笔记</button>
      <button onClick={() => void removeHighlight(activeHighlight.highlight)}><X />取消高亮</button>
    </div>}
    {translationCard && <aside
      className="en-translation-card"
      aria-label="划句翻译"
      aria-live="polite"
      style={{ left: translationCard.left, top: translationCard.top }}
      onMouseDown={(event) => event.stopPropagation()}
    >
      <header>
        <div><Languages /><span>百度翻译</span></div>
        <button aria-label="关闭翻译卡片" onClick={() => setTranslationCard(undefined)}><X /></button>
      </header>
      <blockquote lang="en">{translationCard.anchor.selectedText}</blockquote>
      {translationCard.loading
        ? <p className="en-translation-loading">正在翻译…</p>
        : <p className="en-translation-result" lang="zh-CN">{translationCard.translatedText}</p>}
      <footer>
        <small>翻译结果仅供学习参考</small>
        <button
          disabled={translationCard.loading || translationCard.saving || !translationCard.translatedText}
          onClick={() => void saveTranslationNote()}
        ><NotebookPen />{translationCard.saving ? "保存中…" : "保存为快捷笔记"}</button>
      </footer>
    </aside>}
    {noteDraft && <aside className="en-quick-note-card" aria-label={noteDraft.note ? "编辑快捷笔记" : "添加快捷笔记"} style={{ left: noteDraft.left, top: noteDraft.top }}>
      <header><strong>{noteDraft.note ? "编辑快捷笔记" : "快捷笔记"}</strong><button aria-label="关闭快捷笔记" onClick={() => { setNoteDraft(undefined); setNoteContent(""); }}><X /></button></header>
      <textarea
        ref={noteInputRef}
        aria-label="快捷笔记内容"
        value={noteContent}
        onChange={(event) => setNoteContent(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && event.ctrlKey) {
            event.preventDefault();
            void saveQuickNote();
          }
        }}
        placeholder="输入你的理解、用法或联想……"
      />
      <footer>
        <button onClick={() => { setNoteDraft(undefined); setNoteContent(""); }}>取消</button>
        <button disabled={!noteContent.trim() || savingNote} onClick={() => void saveQuickNote()}>{savingNote ? "保存中…" : "保存"}</button>
      </footer>
    </aside>}
    {completionVisible && <div className="en-reading-complete-card" role="status">
      <CheckCircle2 />
      <div><strong>阅读完成</strong><p>本篇已加入阅读记录</p>{(highlights.length > 0 || notes.length > 0) && <small>{[
        highlights.length > 0 ? `高亮 ${highlights.length} 处` : "",
        notes.length > 0 ? `快捷笔记 ${notes.length} 条` : "",
      ].filter(Boolean).join(" · ")}</small>}</div>
    </div>}
    {lookupLoading && <div className="en-dictionary-loading" role="status">正在查询本地词典…</div>}
    {lookup && <DictionaryPopover lookup={lookup} article={article} settings={vocabularySettings} onClose={() => setLookup(null)} onAdded={onWordAdded} onMessage={setMessage} />}
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

function History({ history, open }: { history: EnglishHistoryResponse | null; open: (articleId: string) => Promise<void> }) {
  const records = history?.records ?? [];
  const [openingId, setOpeningId] = useState<string>();
  const [error, setError] = useState("");
  const chart = Array.from({ length: 30 }, (_, index) => {
    const date = new Date();
    date.setDate(date.getDate() - (29 - index));
    return records.some((record) => record.date === localDateKey(date)) ? 100 : 8;
  });
  return <div>
    <div className="en-section-head"><div><span className="en-eyebrow">LEARNING CURVE</span><h2>看见长期积累。</h2><p>每次阅读、总结和反馈都会成为可追踪的成长数据。</p></div></div>
    <section className="en-history-stats"><article><span>30 天阅读</span><strong>{history?.stats.readingCount30 ?? 0}</strong></article><article><span>平均评分</span><strong>{history?.stats.averageScore30 || "—"}</strong></article><article><span>词汇增长</span><strong>+{history?.stats.vocabularyGrowth30 ?? 0}</strong></article></section>
    <div className="en-chart" aria-label="最近 30 天学习次数">{chart.map((height, index) => <i key={index} style={{ height: `${height}%` }} />)}</div>
    {error && <div className="en-library-error" role="alert">{error}</div>}
    <section className="en-history-list">{records.map((record) => <article key={record.id}><time>{record.date.slice(5)}</time><div><button disabled={openingId === record.id} onClick={async () => {
      setOpeningId(record.id);
      setError("");
      try { await open(record.articleId); }
      catch (openError) { setError(openError instanceof Error ? openError.message : "历史文章打开失败"); }
      finally { setOpeningId(undefined); }
    }}><strong>{record.article?.title ?? "英文阅读"}</strong></button><small>{record.summary ? `${record.summary.split(/\s+/).filter(Boolean).length} 词总结` : "尚未提交总结"} · {record.readingStatus === "completed" || record.completionStatus === "completed" ? "已读" : "阅读中"}</small></div><small>{Math.round(record.readingTimeSeconds / 60)} 分钟</small><b>{record.score ?? "—"}</b></article>)}</section>
    {!records.length && <p className="en-empty">还没有学习记录，先完成今天的文章吧。</p>}
  </div>;
}

type ArticlePage = {
  articles: EnglishArticle[];
  total: number;
  page: number;
  pageSize: number;
  hasMore: boolean;
};

function ArticleLibrary({ currentLevel, start }: {
  currentLevel: CEFRLevel;
  start: (article: EnglishArticle) => void;
}) {
  const [level, setLevel] = useState<CEFRLevel | "all">("all");
  const [query, setQuery] = useState("");
  const [articles, setArticles] = useState<EnglishArticle[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [status, setStatus] = useState<"loading" | "ready" | "loading-more" | "error">("loading");
  const [error, setError] = useState("");
  const [openingId, setOpeningId] = useState<string>();
  const loadMoreRef = useRef<HTMLDivElement>(null);
  const loadingMoreRef = useRef(false);
  const loadMoreActionRef = useRef<() => void>(() => undefined);
  const libraryStatusRef = useRef(status);

  const fetchPage = useCallback(async (pageNumber: number, replace: boolean, signal?: AbortSignal) => {
    const params = new URLSearchParams({
      page: String(pageNumber),
      pageSize: "18",
      summary: "1",
    });
    if (level !== "all") params.set("level", level);
    if (query.trim()) params.set("q", query.trim());
    const result = await request<ArticlePage>(`/api/english/articles?${params}`, { signal });
    setArticles((current) => {
      if (replace) return result.articles;
      const known = new Set(current.map((article) => article.id));
      return [...current, ...result.articles.filter((article) => !known.has(article.id))];
    });
    setTotal(result.total);
    setPage(result.page);
    setHasMore(result.hasMore);
  }, [level, query]);

  useEffect(() => {
    const controller = new AbortController();
    const timer = window.setTimeout(() => {
      setStatus("loading");
      setError("");
      void fetchPage(1, true, controller.signal)
        .then(() => setStatus("ready"))
        .catch((loadError) => {
          if (controller.signal.aborted) return;
          setError(loadError instanceof Error ? loadError.message : "文章库加载失败");
          setStatus("error");
        });
    }, query.trim() ? 180 : 0);
    return () => {
      window.clearTimeout(timer);
      controller.abort();
    };
  }, [fetchPage, query]);

  const loadMore = useCallback(async () => {
    if (!hasMore || status !== "ready" || loadingMoreRef.current) return;
    loadingMoreRef.current = true;
    setStatus("loading-more");
    try {
      await fetchPage(page + 1, false);
      setStatus("ready");
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : "更多文章加载失败");
      setStatus("error");
    } finally {
      loadingMoreRef.current = false;
    }
  }, [fetchPage, hasMore, page, status]);

  useEffect(() => {
    libraryStatusRef.current = status;
    loadMoreActionRef.current = () => void loadMore();
  }, [loadMore, status]);

  useEffect(() => {
    const target = loadMoreRef.current;
    if (!target) return;
    let armed = true;
    const observer = new IntersectionObserver((entries) => {
      const entry = entries[0];
      if (!entry?.isIntersecting) {
        armed = true;
        return;
      }
      if (armed && libraryStatusRef.current === "ready") {
        armed = false;
        loadMoreActionRef.current();
      }
    }, { rootMargin: "240px" });
    observer.observe(target);
    return () => observer.disconnect();
  }, []);

  const openArticle = async (article: EnglishArticle) => {
    setOpeningId(article.id);
    setError("");
    try {
      const detail = await request<EnglishArticle>(`/api/english/articles?id=${encodeURIComponent(article.id)}`);
      start(detail);
    } catch (openError) {
      setError(openError instanceof Error ? openError.message : "文章打开失败");
    } finally {
      setOpeningId(undefined);
    }
  };

  const retry = async () => {
    setStatus("loading");
    setError("");
    try {
      await fetchPage(1, true);
      setStatus("ready");
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : "文章库加载失败");
      setStatus("error");
    }
  };

  return <div>
    <div className="en-section-head"><div><span className="en-eyebrow">ARTICLE LIBRARY</span><h2>按你的水平，<br />选择下一篇文章。</h2><p aria-live="polite">{status === "loading" ? "正在准备文章库…" : `共 ${total} 篇 · 已加载 ${articles.length} 篇 · 当前推荐等级：${currentLevel} · ${levelName[currentLevel]}`}</p></div><div><input aria-label="搜索文章" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索文章" /><select aria-label="按等级筛选" value={level} onChange={(event) => setLevel(event.target.value as CEFRLevel | "all")}><option value="all">全部等级</option>{(["A1", "A2", "B1", "B2", "C1"] as CEFRLevel[]).map((item) => <option value={item} key={item}>{item}</option>)}</select></div></div>
    {error && <div className="en-library-error" role="status">{error}<button onClick={() => void retry()}>重新加载</button></div>}
    {status === "loading" ? <ArticleLibrarySkeleton /> : <>
      <div className="en-article-grid">{articles.map((article) => <article key={article.id}><span>{article.level} · {categoryName[article.category]}</span>{article.readingStatus && article.readingStatus !== "unread" && <small className={`en-article-reading-state ${article.readingStatus}`}>{article.readingStatus === "completed" ? <><Check />已读</> : "阅读中"}</small>}<h3>{article.title}</h3><p>{article.content}</p><footer><small>{article.estimatedMinutes} 分钟 · 难度 {article.difficulty}/5</small><button disabled={openingId === article.id} onClick={() => void openArticle(article)}>{openingId === article.id ? "正在打开…" : article.readingStatus === "completed" ? "再次查看" : article.readingStatus === "reading" ? "继续阅读" : "阅读"} {!openingId && <ChevronRight />}</button></footer></article>)}</div>
      {!articles.length && status !== "error" && <p className="en-empty">没有找到符合条件的文章。</p>}
    </>}
    <div className="en-library-load-more" ref={loadMoreRef}>
      {status !== "loading" && hasMore && <button disabled={status === "loading-more"} onClick={() => void loadMore()}>{status === "loading-more" ? "正在加载更多文章…" : "加载更多文章"}</button>}
      {status !== "loading" && !hasMore && articles.length > 0 && <span>已加载全部 {total} 篇文章</span>}
    </div>
  </div>;
}

function ArticleLibrarySkeleton() {
  return <div className="en-article-grid en-article-skeleton" aria-label="正在加载文章">
    {Array.from({ length: 6 }, (_, index) => <article key={index} aria-hidden="true"><i /><h3 /><p /><p /><footer><small /><b /></footer></article>)}
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
