import { useMemo, useRef, useState, type FormEvent, type ReactNode } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { ArrowLeft, BookOpen, CheckCircle2, Highlighter, Languages, NotebookPen, Plus } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, Card, CardContent, EmptyState, Input, MetricCard, PageHeader, Textarea, cn } from "../../components/ui";
import { entities, number, text } from "../../lib/entities";
import { createEnglishHighlight, createEnglishLearningRecord, createEnglishNote, createVocabulary, type JsonEntity } from "../../services/core";

function articleBody(article: JsonEntity): string {
  for (const key of ["contentText", "content", "body", "text", "markdown"]) {
    const value = text(article, key);
    if (value) return value;
  }
  return text(article, "summary", "这篇文章暂无正文内容。");
}

function escapeRegex(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function highlightedParagraph(content: string, terms: string[]): ReactNode {
  const unique = [...new Set(terms.map((term) => term.trim()).filter(Boolean))].sort((a, b) => b.length - a.length);
  if (!unique.length) return content;
  const pattern = new RegExp(`(${unique.map(escapeRegex).join("|")})`, "gi");
  return content.split(pattern).map((part, index) => unique.some((term) => term.toLocaleLowerCase() === part.toLocaleLowerCase())
    ? <mark key={`${part}-${index}`} className="rounded-sm bg-warning/25 px-0.5 text-foreground">{part}</mark>
    : <span key={`${part}-${index}`}>{part}</span>);
}

export function EnglishPage() {
  const { state, session, upsert } = useApp();
  const location = useLocation();
  const navigate = useNavigate();
  const [word, setWord] = useState("");
  const [definition, setDefinition] = useState("");
  const [readerId, setReaderId] = useState<string | null>(null);
  const [selectedText, setSelectedText] = useState("");
  const [quickNote, setQuickNote] = useState("");
  const openedAt = useRef(Date.now());

  const articles = entities(state, "english.article").sort((a, b) => b.meta.updatedAt.localeCompare(a.meta.updatedAt));
  const vocabulary = entities(state, "english.vocabulary").sort((a, b) => b.meta.updatedAt.localeCompare(a.meta.updatedAt));
  const highlights = entities(state, "english.highlight");
  const notes = entities(state, "english.note");
  const records = entities(state, "english.learning_record");
  const tab = location.pathname.includes("vocabulary") ? "vocabulary" : location.pathname.includes("stats") ? "stats" : "articles";
  const readingMinutes = Math.round(records.reduce((sumValue, item) => sumValue + number(item, "readingTimeSeconds"), 0) / 60);
  const completedArticleIds = new Set(records.filter((item) => text(item, "completionStatus") === "completed" || text(item, "readingStatus") === "completed").map((item) => String(item.articleId ?? "")));
  const mastery = useMemo(() => vocabulary.length ? Math.round(vocabulary.reduce((sumValue, item) => sumValue + number(item, "masteryLevel"), 0) / vocabulary.length) : 0, [vocabulary]);
  const readerArticle = articles.find((item) => item.meta.id === readerId) ?? null;

  async function addVocabulary(event: FormEvent) {
    event.preventDefault();
    if (!session) return;
    await upsert("english.vocabulary", createVocabulary(session.user.id, session.session.deviceId, word, definition));
    setWord("");
    setDefinition("");
  }

  function openReader(articleId: string) {
    openedAt.current = Date.now();
    setSelectedText("");
    setQuickNote("");
    setReaderId(articleId);
  }

  async function saveHighlight() {
    if (!session || !readerArticle || !selectedText.trim()) return;
    await upsert("english.highlight", createEnglishHighlight(session.user.id, session.session.deviceId, readerArticle.meta.id, selectedText));
    setSelectedText("");
  }

  async function saveQuickNote() {
    if (!session || !readerArticle || !quickNote.trim()) return;
    await upsert("english.note", createEnglishNote(session.user.id, session.session.deviceId, readerArticle.meta.id, quickNote, selectedText));
    setQuickNote("");
  }

  async function markRead() {
    if (!session || !readerArticle) return;
    const seconds = Math.max(1, Math.round((Date.now() - openedAt.current) / 1000));
    await upsert("english.learning_record", createEnglishLearningRecord(session.user.id, session.session.deviceId, readerArticle.meta.id, text(readerArticle, "summary"), seconds));
  }

  return <div className="page-shell">
    <PageHeader title="英语学习" description="阅读、高亮、快捷笔记、生词和学习历史。阅读模式会弱化全局 Shell，让正文成为视觉中心。" />
    <div className="mb-5 flex w-fit rounded-md border p-0.5">{[["articles", "阅读"], ["vocabulary", "生词本"], ["stats", "统计"]].map(([id, label]) => <button key={id} onClick={() => navigate(id === "articles" ? "/app/english/articles" : `/app/english/${id}`)} className={cn("rounded px-3 py-1.5 text-xs", tab === id && "bg-muted font-medium")}>{label}</button>)}</div>

    {tab === "articles" ? <div>{articles.length ? <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">{articles.map((article) => {
      const completed = completedArticleIds.has(article.meta.id);
      return <button key={article.meta.id} onClick={() => openReader(article.meta.id)} className="text-left"><Card className="h-full transition-colors hover:bg-muted/20"><CardContent className="pt-5"><div className="flex items-start justify-between gap-3"><BookOpen size={18} className="text-primary" /><Badge className={completed ? "text-success" : ""}>{completed ? "已读" : "未读"}</Badge></div><div className="mt-4 text-base font-semibold">{text(article, "title", "Untitled article")}</div><p className="mt-2 line-clamp-3 text-sm leading-6 text-muted-foreground">{text(article, "summary", articleBody(article))}</p><div className="mt-4 text-xs text-muted-foreground">{highlights.filter((item) => item.articleId === article.meta.id).length} 处高亮 · {notes.filter((item) => item.articleId === article.meta.id).length} 条笔记</div></CardContent></Card></button>;
    })}</div> : <EmptyState icon={<BookOpen size={24} />} title="还没有阅读文章" description="现有云端文章、高亮与阅读状态会在这里呈现。" />}</div> : null}

    {tab === "vocabulary" ? <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_340px]"><div>{vocabulary.length ? <Card><div className="divide-y">{vocabulary.map((item) => <div key={item.meta.id} className="px-4 py-3"><div className="flex items-center justify-between"><div className="font-semibold">{text(item, "displayWord")}</div><Badge>Level {number(item, "masteryLevel")}</Badge></div><div className="mt-1 text-sm text-muted-foreground">{text(item, "definition", "暂无释义")}</div></div>)}</div></Card> : <EmptyState title="生词本为空" description="在阅读中添加或在右侧手动录入。" />}</div><Card className="h-fit"><CardContent className="pt-5"><div className="mb-4 flex items-center gap-2 font-semibold"><Plus size={16} />添加生词</div><form className="space-y-3" onSubmit={(event) => void addVocabulary(event)}><Input value={word} onChange={(event) => setWord(event.target.value)} placeholder="Word" required /><Input value={definition} onChange={(event) => setDefinition(event.target.value)} placeholder="释义" /><Button className="w-full" type="submit">保存</Button></form></CardContent></Card></div> : null}

    {tab === "stats" ? <div className="grid gap-3 sm:grid-cols-3"><MetricCard label="完成阅读" value={`${completedArticleIds.size} 篇`} icon={<BookOpen size={17} />} /><MetricCard label="阅读时长" value={`${readingMinutes} 分钟`} icon={<Languages size={17} />} /><MetricCard label="生词平均掌握" value={`${mastery}/5`} hint={`${vocabulary.length} 个生词`} /></div> : null}

    {readerArticle ? <Reader
      article={readerArticle}
      highlights={highlights.filter((item) => item.articleId === readerArticle.meta.id)}
      notes={notes.filter((item) => item.articleId === readerArticle.meta.id)}
      selectedText={selectedText}
      quickNote={quickNote}
      completed={completedArticleIds.has(readerArticle.meta.id)}
      onClose={() => setReaderId(null)}
      onSelection={setSelectedText}
      onQuickNote={setQuickNote}
      onSaveHighlight={() => void saveHighlight()}
      onSaveNote={() => void saveQuickNote()}
      onMarkRead={() => void markRead()}
    /> : null}
  </div>;
}

function Reader({ article, highlights, notes, selectedText, quickNote, completed, onClose, onSelection, onQuickNote, onSaveHighlight, onSaveNote, onMarkRead }: {
  article: JsonEntity;
  highlights: JsonEntity[];
  notes: JsonEntity[];
  selectedText: string;
  quickNote: string;
  completed: boolean;
  onClose(): void;
  onSelection(value: string): void;
  onQuickNote(value: string): void;
  onSaveHighlight(): void;
  onSaveNote(): void;
  onMarkRead(): void;
}) {
  const highlightTerms = highlights.map((item) => text(item, "selectedText"));
  const paragraphs = articleBody(article).split(/\n{2,}/).filter(Boolean);

  function captureSelection() {
    const value = window.getSelection()?.toString().trim() ?? "";
    if (value) onSelection(value);
  }

  return <div className="fixed inset-0 z-[70] overflow-y-auto bg-background">
    <header className="sticky top-0 z-10 border-b bg-background/95 backdrop-blur"><div className="mx-auto flex h-14 max-w-6xl items-center justify-between gap-3 px-4 sm:px-6"><Button variant="ghost" onClick={onClose}><ArrowLeft size={16} />返回文章</Button><div className="flex items-center gap-2"><Badge className={completed ? "text-success" : ""}>{completed ? "已读" : "阅读中"}</Badge><Button variant="outline" onClick={onMarkRead}><CheckCircle2 size={15} />标记已读</Button></div></div></header>
    <main className="mx-auto grid max-w-6xl gap-8 px-4 py-8 sm:px-6 lg:grid-cols-[minmax(0,1fr)_300px]">
      <article className="min-w-0" onMouseUp={captureSelection}>
        <div className="eyebrow">Reading</div>
        <h1 className="mt-3 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">{text(article, "title", "Untitled article")}</h1>
        {text(article, "summary") ? <p className="mt-4 text-base leading-7 text-muted-foreground">{text(article, "summary")}</p> : null}
        <div className="mt-8 space-y-6 text-[17px] leading-8 text-foreground/95">{paragraphs.map((paragraph, index) => <p key={index}>{highlightedParagraph(paragraph, highlightTerms)}</p>)}</div>
      </article>
      <aside className="h-fit space-y-4 lg:sticky lg:top-20">
        <Card><CardContent className="pt-5"><div className="flex items-center gap-2 font-semibold"><Highlighter size={16} />高亮</div><p className="mt-2 text-xs leading-5 text-muted-foreground">在正文中拖选文字，会自动进入下方输入框。</p><Textarea className="mt-3 min-h-20" value={selectedText} onChange={(event) => onSelection(event.target.value)} placeholder="选中的短语或句子" /><Button className="mt-2 w-full" variant="outline" disabled={!selectedText.trim()} onClick={onSaveHighlight}>保存高亮</Button></CardContent></Card>
        <Card><CardContent className="pt-5"><div className="flex items-center gap-2 font-semibold"><NotebookPen size={16} />快捷笔记</div><Textarea className="mt-3 min-h-24" value={quickNote} onChange={(event) => onQuickNote(event.target.value)} placeholder="只记录你的想法…" /><Button className="mt-2 w-full" disabled={!quickNote.trim()} onClick={onSaveNote}>保存笔记</Button>{notes.length ? <div className="mt-4 space-y-2 border-t pt-4">{notes.slice(-5).reverse().map((note) => <div key={note.meta.id} className="rounded-md bg-muted/45 p-2.5 text-xs leading-5">{text(note, "content")}</div>)}</div> : null}</CardContent></Card>
      </aside>
    </main>
  </div>;
}
