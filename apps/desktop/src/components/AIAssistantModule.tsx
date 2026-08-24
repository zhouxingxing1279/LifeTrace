"use client";

import { Component, useEffect, useMemo, useRef, useState, useSyncExternalStore, type ErrorInfo, type ReactNode } from "react";
import { ArrowUp, Bot, BrainCircuit, Database, KeyRound, LoaderCircle, MessageSquare, Pencil, Plus, ShieldCheck, Sparkles, Trash2 } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

type Dataset = { key: string; label: string; count: number };
type Catalog = { datasets: Dataset[]; readOnly: boolean; error?: string };
type Settings = { configured: boolean; model: string; error?: string };
type Message = {
  id: string;
  role: "user" | "assistant";
  content: string;
  datasets?: string[];
  model?: string;
  tokens?: number;
};
type ChatResponse = {
  message?: string;
  model?: string;
  datasets?: string[];
  usage?: { total_tokens?: number; prompt_tokens?: number; completion_tokens?: number };
  error?: string;
  code?: string;
};
type ConversationSummary = { id: string; title: string; messageCount: number; createdAt: string; updatedAt: string };
type ConversationDetail = ConversationSummary & { messages: Message[] };

const localDate = () => {
  const date = new Date();
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
};
let fallbackId = 0;
const makeId = () => {
  const randomUUID = globalThis.crypto?.randomUUID;
  if (typeof randomUUID === "function") return randomUUID.call(globalThis.crypto);
  fallbackId += 1;
  return `ai-message-${Date.now()}-${fallbackId}`;
};
const initialMessage = (): Message => ({
  id: makeId(),
  role: "assistant",
  content: "你好，我是你的 LifeTrace AI 管家。我可以读取你的坚持、训练、财务、复盘、笔记、英语学习和照片元数据，帮你回顾一天、分析趋势或寻找某段记录。",
});
const quickPrompts = [
  { label: "总结今天", prompt: `请读取 ${localDate()} 的生活快照，总结我今天做了什么、有什么亮点和一个明天可以继续的小建议。` },
  { label: "回顾最近一周", prompt: "请结合我最近 7 天的记录，概括生活节奏、完成情况和值得注意的变化。" },
  { label: "分析消费", prompt: "请分析我本月的消费结构，指出主要支出方向和一个不制造焦虑的改进建议。" },
  { label: "发现规律", prompt: "请查看我的各类记录，找出有足够数据支持的生活规律；明确说明样本范围，并区分相关性与因果。" },
];

type AssistantSession = {
  conversationId: string;
  title: string;
  messages: Message[];
  input: string;
  loading: boolean;
  error: string;
};

let assistantSession: AssistantSession = {
  conversationId: "",
  title: "",
  messages: [initialMessage()],
  input: "",
  loading: false,
  error: "",
};
const assistantSessionListeners = new Set<() => void>();
const subscribeAssistantSession = (listener: () => void) => {
  assistantSessionListeners.add(listener);
  return () => assistantSessionListeners.delete(listener);
};
const getAssistantSession = () => assistantSession;
const updateAssistantSession = (patch: Partial<AssistantSession>) => {
  assistantSession = { ...assistantSession, ...patch };
  assistantSessionListeners.forEach((listener) => listener());
};

class AIAssistantErrorBoundary extends Component<
  { children: ReactNode },
  { error: string }
> {
  state = { error: "" };

  static getDerivedStateFromError(error: unknown) {
    return { error: error instanceof Error ? error.message : "AI 管家页面加载失败" };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("AI assistant render failed", error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;
    return <div className="hx-view ai-empty-state ai-crash-state" role="alert">
      <span>!</span>
      <small>页面加载失败</small>
      <h2>AI 管家暂时没有正常打开</h2>
      <p>{this.state.error}</p>
      <button className="hx-btn primary" onClick={() => this.setState({ error: "" })}>重新加载管家</button>
    </div>;
  }
}

function AIAssistantContent({ openSettings }: { openSettings: () => void }) {
  const { conversationId, messages, input, loading, error } = useSyncExternalStore(
    subscribeAssistantSession,
    getAssistantSession,
    getAssistantSession,
  );
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const controller = new AbortController();
    void Promise.all([
      fetch("/api/assistant/catalog", { signal: controller.signal }).then(async (response) => {
        const value = await response.json() as Catalog;
        if (!response.ok) throw new Error(value.error || "无法读取数据目录");
        return value;
      }),
      fetch("/api/settings/ai", { signal: controller.signal }).then(async (response) => {
        const value = await response.json() as Settings;
        if (!response.ok) throw new Error(value.error || "无法读取 AI 设置");
        return value;
      }),
      fetch("/api/assistant/conversations", { signal: controller.signal }).then(async (response) => {
        const value = await response.json() as { items?: ConversationSummary[]; error?: string };
        if (!response.ok) throw new Error(value.error || "无法读取历史会话");
        return value.items ?? [];
      }),
    ]).then(([nextCatalog, nextSettings, nextConversations]) => {
      setCatalog(nextCatalog);
      setSettings(nextSettings);
      setConversations(nextConversations);
    }).catch((reason) => {
      if (!controller.signal.aborted) updateAssistantSession({ error: reason instanceof Error ? reason.message : "AI 管家初始化失败" });
    });
    return () => controller.abort();
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages, loading]);

  const totalRecords = useMemo(() => catalog?.datasets.reduce((sum, item) => sum + item.count, 0) ?? 0, [catalog]);
  const labels = useMemo(() => new Map(catalog?.datasets.map((item) => [item.key, item.label]) ?? []), [catalog]);

  const saveConversation = async (id: string, title: string, nextMessages: Message[]) => {
    const response = await fetch("/api/assistant/conversations", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ id, title, messages: nextMessages }),
    });
    const payload = await response.json() as { updatedAt?: string; error?: string };
    if (!response.ok) throw new Error(payload.error || "保存历史会话失败");
    const summary: ConversationSummary = {
      id,
      title,
      messageCount: nextMessages.length,
      createdAt: payload.updatedAt ?? new Date().toISOString(),
      updatedAt: payload.updatedAt ?? new Date().toISOString(),
    };
    setConversations((current) => [summary, ...current.filter((item) => item.id !== id)].slice(0, 50));
  };

  const loadConversation = async (id: string) => {
    if (loading || id === conversationId) return;
    try {
      const response = await fetch(`/api/assistant/conversations?id=${encodeURIComponent(id)}`);
      const payload = await response.json() as { item?: ConversationDetail; error?: string };
      if (!response.ok || !payload.item) throw new Error(payload.error || "读取历史会话失败");
      updateAssistantSession({ conversationId: payload.item.id, title: payload.item.title, messages: payload.item.messages, input: "", error: "" });
    } catch (reason) {
      updateAssistantSession({ error: reason instanceof Error ? reason.message : "读取历史会话失败" });
    }
  };

  const renameConversation = async (item: ConversationSummary) => {
    const title = window.prompt("修改会话名称", item.title)?.trim();
    if (!title || title === item.title) return;
    try {
      const response = await fetch(`/api/assistant/conversations?id=${encodeURIComponent(item.id)}`);
      const payload = await response.json() as { item?: ConversationDetail; error?: string };
      if (!response.ok || !payload.item) throw new Error(payload.error || "读取历史会话失败");
      await saveConversation(item.id, title.slice(0, 80), payload.item.messages);
      if (assistantSession.conversationId === item.id) updateAssistantSession({ title: title.slice(0, 80) });
    } catch (reason) {
      updateAssistantSession({ error: reason instanceof Error ? reason.message : "重命名失败" });
    }
  };

  const deleteConversation = async (item: ConversationSummary) => {
    if (!window.confirm(`删除历史会话“${item.title}”？此操作无法撤销。`)) return;
    try {
      const response = await fetch(`/api/assistant/conversations?id=${encodeURIComponent(item.id)}`, { method: "DELETE" });
      const payload = await response.json() as { error?: string };
      if (!response.ok) throw new Error(payload.error || "删除历史会话失败");
      setConversations((current) => current.filter((value) => value.id !== item.id));
      if (assistantSession.conversationId === item.id) reset();
    } catch (reason) {
      updateAssistantSession({ error: reason instanceof Error ? reason.message : "删除历史会话失败" });
    }
  };

  const send = async (raw: string) => {
    const content = raw.trim();
    if (!content || loading) return;
    const userMessage: Message = { id: makeId(), role: "user", content };
    const nextMessages = [...assistantSession.messages, userMessage];
    const nextConversationId = assistantSession.conversationId || `ai-conversation-${makeId()}`;
    const nextTitle = assistantSession.title || content.replace(/\s+/g, " ").slice(0, 32);
    updateAssistantSession({ conversationId: nextConversationId, title: nextTitle, messages: nextMessages, input: "", loading: true, error: "" });
    void saveConversation(nextConversationId, nextTitle, nextMessages).catch(console.error);
    try {
      const response = await fetch("/api/assistant/chat", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          messages: nextMessages.map(({ role, content: messageContent }) => ({ role, content: messageContent })),
        }),
      });
      const payload = await response.json() as ChatResponse;
      if (!response.ok || !payload.message) throw new Error(payload.error || "AI 管家暂时无法回答");
      const answeredMessages: Message[] = [...assistantSession.messages, {
        id: makeId(), role: "assistant", content: payload.message!, datasets: payload.datasets, model: payload.model, tokens: payload.usage?.total_tokens,
      }];
      updateAssistantSession({ messages: answeredMessages });
      void saveConversation(nextConversationId, nextTitle, answeredMessages).catch(console.error);
    } catch (reason) {
      updateAssistantSession({ error: reason instanceof Error ? reason.message : "AI 管家暂时无法回答" });
    } finally {
      updateAssistantSession({ loading: false });
    }
  };

  const reset = () => {
    updateAssistantSession({ conversationId: "", title: "", messages: [initialMessage()], input: "", error: "" });
  };

  if (settings && !settings.configured) {
    return <div className="hx-view ai-empty-state">
      <span><KeyRound /></span>
      <small>DEEPSEEK 未配置</small>
      <h2>先为 AI 管家连接模型</h2>
      <p>配置 DeepSeek API Key 后，管家才能按需读取你的本地记录并回答问题。密钥不会暴露给前端页面。</p>
      <button className="hx-btn primary" onClick={openSettings}><KeyRound />前往 AI 设置</button>
    </div>;
  }

  return <div className="hx-view ai-assistant">
    <section className="ai-hero">
      <div className="ai-orb"><BrainCircuit /></div>
      <div><span className="hx-pill">DeepSeek · 只读模式</span><h2>有什么想一起梳理的？</h2><p>我会在需要时读取相关数据，并告诉你本次参考了哪些记录。</p></div>
      <div className="ai-status">
        <span><ShieldCheck />只读 · 省流检索</span>
        <strong>{totalRecords.toLocaleString("zh-CN")}</strong>
        <small>{catalog?.datasets.length ?? 0} 类本地数据记录</small>
      </div>
    </section>

    <section className="ai-workspace">
      <aside className="ai-side">
        <div><span>快捷提问</span>{quickPrompts.map((item) => <button key={item.label} disabled={loading} onClick={() => void send(item.prompt)}><Sparkles />{item.label}</button>)}</div>
        <div className="ai-history"><span>历史会话</span><button className="ai-new-chat" disabled={loading} onClick={reset}><Plus />新对话</button>{conversations.length === 0 ? <p>还没有保存的会话</p> : <div>{conversations.slice(0, 8).map((item) => <article className={conversationId === item.id ? "active" : ""} key={item.id}>
          <button title={item.title} disabled={loading} onClick={() => void loadConversation(item.id)}><MessageSquare /><span>{item.title}</span></button>
          <button aria-label={`重命名 ${item.title}`} onClick={() => void renameConversation(item)}><Pencil /></button>
          <button aria-label={`删除 ${item.title}`} onClick={() => void deleteConversation(item)}><Trash2 /></button>
        </article>)}</div>}</div>
        <div className="ai-data-scope"><span>可访问范围</span><p>坚持、训练、财务、复盘、笔记、英语、照片及导入记录。</p><small><Database />按需发送，不会一次上传整个数据库</small></div>
      </aside>

      <div className="ai-chat-panel">
        <div className="ai-messages" aria-live="polite">
          {messages.map((message) => <article className={`ai-message ${message.role}`} key={message.id}>
            <span>{message.role === "assistant" ? <Bot /> : "我"}</span>
            <div>{message.role === "assistant" ? <div className="ai-markdown"><ReactMarkdown remarkPlugins={[remarkGfm]} components={{
              a: ({ children, ...props }) => <a {...props} target="_blank" rel="noreferrer">{children}</a>,
            }}>{message.content}</ReactMarkdown></div> : <p>{message.content}</p>}{message.role === "assistant" && ((message.datasets?.length ?? 0) > 0 || message.tokens) && <footer>
              {(message.datasets?.length ?? 0) > 0 && <span><Database />参考了 {message.datasets!.map((key) => labels.get(key) || key).join("、")}</span>}
              {message.tokens && <span>{message.tokens.toLocaleString("zh-CN")} tokens</span>}
            </footer>}</div>
          </article>)}
          {loading && <article className="ai-message assistant pending"><span><Bot /></span><div><LoaderCircle /><p>正在查阅相关记录并整理回答…</p></div></article>}
          {error && <div className="ai-error" role="alert">{error}</div>}
          <div ref={bottomRef} />
        </div>
        <form className="ai-composer" onSubmit={(event) => { event.preventDefault(); void send(input); }}>
          <textarea
            value={input}
            onChange={(event) => updateAssistantSession({ input: event.target.value })}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                void send(input);
              }
            }}
            placeholder="问问今天做了什么，或分析某段时间的生活记录…"
            maxLength={4000}
            disabled={loading}
            aria-label="给 AI 管家发送消息"
          />
          <button type="submit" disabled={loading || !input.trim()} aria-label="发送"><ArrowUp /></button>
          <small>Enter 发送 · Shift + Enter 换行 · 回答仅供个人参考</small>
        </form>
      </div>
    </section>
  </div>;
}

export default function AIAssistantModule(props: { openSettings: () => void }) {
  return <AIAssistantErrorBoundary><AIAssistantContent {...props} /></AIAssistantErrorBoundary>;
}
