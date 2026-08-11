import { cloudAuthClient } from "@/src/services/cloudAuth";
import { useCloudAuthStore } from "@/src/stores/useCloudAuthStore";

const previewOrigin = "http://preview.lifetrace.local";
const now = new Date();
const iso = (daysAgo = 0, hour = 9, minute = 0) => {
  const value = new Date(now);
  value.setDate(value.getDate() - daysAgo);
  value.setHours(hour, minute, 0, 0);
  return value.toISOString();
};
const day = (daysAgo = 0) => iso(daysAgo).slice(0, 10);

const activities = [
  { id: "habit-english", userId: "preview-user", name: "英语学习", type: "duration", unit: "分钟", minimumTarget: 15, normalTarget: 30, targetPeriod: "daily", icon: "Languages", color: "emerald", scheduleType: "daily", checkinMethod: "manual", isArchived: false, createdAt: iso(120), updatedAt: iso() },
  { id: "habit-piano", userId: "preview-user", name: "钢琴", type: "duration", unit: "分钟", minimumTarget: 20, normalTarget: 45, targetPeriod: "daily", icon: "Music", color: "blue", scheduleType: "daily", checkinMethod: "manual", isArchived: false, createdAt: iso(100), updatedAt: iso() },
  { id: "habit-reading", userId: "preview-user", name: "阅读", type: "duration", unit: "分钟", minimumTarget: 15, normalTarget: 30, targetPeriod: "daily", icon: "BookOpen", color: "violet", scheduleType: "daily", checkinMethod: "manual", isArchived: false, createdAt: iso(90), updatedAt: iso() },
  { id: "habit-fitness", userId: "preview-user", name: "健身", type: "weekly", unit: "次", minimumTarget: 2, normalTarget: 4, targetPeriod: "weekly", icon: "Dumbbell", color: "orange", scheduleType: "weekly", targetDays: [1, 2, 4, 5], checkinMethod: "automatic", syncSource: "fitness", isArchived: false, createdAt: iso(80), updatedAt: iso() },
];

const logs = [
  { id: "log-1", userId: "preview-user", activityId: "habit-english", value: 24, status: "partial", createdAt: iso(0, 8, 40), updatedAt: iso(0, 8, 40) },
  { id: "log-2", userId: "preview-user", activityId: "habit-piano", value: 45, status: "completed", createdAt: iso(0, 10, 10), updatedAt: iso(0, 10, 10) },
  { id: "log-3", userId: "preview-user", activityId: "habit-reading", value: 30, status: "completed", createdAt: iso(0, 7, 20), updatedAt: iso(0, 7, 20) },
  { id: "log-4", userId: "preview-user", activityId: "habit-fitness", value: 1, status: "completed", createdAt: iso(1, 19, 15), updatedAt: iso(1, 19, 15) },
  { id: "log-5", userId: "preview-user", activityId: "habit-english", value: 30, status: "completed", createdAt: iso(1, 8, 30), updatedAt: iso(1, 8, 30) },
  { id: "log-6", userId: "preview-user", activityId: "habit-reading", value: 35, status: "completed", createdAt: iso(2, 22, 0), updatedAt: iso(2, 22, 0) },
];

const accounts = [
  { id: "acc-bank", userId: "preview-user", name: "招商银行", type: "bank", balance: 18640.82, balanceAt: iso(3), last4: "6888", color: "#1f6f56", icon: "Landmark", isArchived: false, createdAt: iso(180), updatedAt: iso() },
  { id: "acc-alipay", userId: "preview-user", name: "支付宝", type: "alipay", balance: 3280.14, balanceAt: iso(2), color: "#2f6fb3", icon: "WalletCards", isArchived: false, createdAt: iso(170), updatedAt: iso() },
  { id: "acc-wechat", userId: "preview-user", name: "微信零钱", type: "wechat", balance: 1260.55, balanceAt: iso(2), color: "#1e7a4c", icon: "Wallet", isArchived: false, createdAt: iso(160), updatedAt: iso() },
  { id: "acc-invest", userId: "preview-user", name: "余额宝", type: "investment", balance: 24800, balanceAt: iso(1), color: "#966a1c", icon: "TrendingUp", isArchived: false, createdAt: iso(150), updatedAt: iso() },
];

const transactions = [
  { id: "tx-1", userId: "preview-user", type: "expense", amount: 36, category: "餐饮", account: "支付宝", accountId: "acc-alipay", counterparty: "星巴克", item: "美式咖啡", occurredAt: iso(0, 9, 15), createdAt: iso(0, 9, 15), updatedAt: iso(0, 9, 15) },
  { id: "tx-2", userId: "preview-user", type: "expense", amount: 4, category: "交通", account: "微信零钱", accountId: "acc-wechat", counterparty: "南京地铁", occurredAt: iso(0, 8, 5), createdAt: iso(0, 8, 5), updatedAt: iso(0, 8, 5) },
  { id: "tx-3", userId: "preview-user", type: "expense", amount: 299, category: "购物", account: "招商银行", accountId: "acc-bank", counterparty: "京东", item: "生活用品", occurredAt: iso(1, 20, 30), createdAt: iso(1, 20, 30), updatedAt: iso(1, 20, 30) },
  { id: "tx-4", userId: "preview-user", type: "expense", amount: 58, category: "餐饮", account: "支付宝", accountId: "acc-alipay", counterparty: "轻食餐厅", occurredAt: iso(2, 12, 20), createdAt: iso(2, 12, 20), updatedAt: iso(2, 12, 20) },
  { id: "tx-5", userId: "preview-user", type: "income", amount: 12000, category: "工资", account: "招商银行", accountId: "acc-bank", counterparty: "工资收入", occurredAt: iso(6, 10, 0), createdAt: iso(6, 10, 0), updatedAt: iso(6, 10, 0) },
  { id: "tx-6", userId: "preview-user", type: "expense", amount: 189, category: "运动", account: "招商银行", accountId: "acc-bank", counterparty: "健身用品", occurredAt: iso(4, 18, 45), createdAt: iso(4, 18, 45), updatedAt: iso(4, 18, 45) },
];

const workoutHistory = [
  { id: "workout-1", userId: "preview-user", templateId: "push", name: "胸 + 肩 + 三头", occurredAt: iso(1, 19, 0), durationSeconds: 4380, exerciseCount: 6, setCount: 22, plannedSetCount: 24, status: "completed", source: "xunji", caloriesKcal: 412, volumeKg: 8920, createdAt: iso(1, 20, 20), updatedAt: iso(1, 20, 20) },
  { id: "workout-2", userId: "preview-user", templateId: "pull", name: "背 + 二头", occurredAt: iso(4, 19, 10), durationSeconds: 3960, exerciseCount: 5, setCount: 19, plannedSetCount: 20, status: "completed", source: "xunji", caloriesKcal: 366, volumeKg: 7420, createdAt: iso(4, 20, 20), updatedAt: iso(4, 20, 20) },
];

const reviews = [
  { id: "review-1", userId: "preview-user", reviewDate: day(1), energy: 8, mood: 7, completionScore: 82, bestThing: "完成训练，并把海报修改任务收尾。", problem: "下午的碎片时间利用率不高。", tomorrowPriority: "先完成最重要的一项开发任务。", note: "整体节奏不错。", createdAt: iso(1, 22, 0), updatedAt: iso(1, 22, 0) },
];

const lifeState = {
  activities,
  logs,
  transactions,
  reviews,
  settings: { id: "preferences", dark: false, timer: null, updatedAt: iso() },
  accounts,
  workoutHistory,
};

const noteFolders = [
  { id: "folder-research", name: "研究", icon: "FlaskConical", color: "#1f6f56", sortOrder: 0, createdAt: iso(60), updatedAt: iso() },
  { id: "folder-work", name: "工作", icon: "Briefcase", color: "#2f6fb3", sortOrder: 1, createdAt: iso(50), updatedAt: iso() },
];
const noteTags = [
  { id: "tag-control", name: "控制", color: "#1f6f56", createdAt: iso(40), updatedAt: iso(), usageCount: 3 },
  { id: "tag-ui", name: "UI", color: "#2f6fb3", createdAt: iso(30), updatedAt: iso(), usageCount: 2 },
];
const notes = [
  { id: "note-1", title: "ICGNC 海报修改记录", noteType: "document", folderId: "folder-research", contentJson: { type: "doc", content: [{ type: "heading", attrs: { level: 2 }, content: [{ type: "text", text: "ICGNC 海报修改记录" }] }, { type: "paragraph", content: [{ type: "text", text: "保持公式密集型科研海报风格，重点检查 SMF、Tube MPC 与实验数据的一致性。" }] }] }, contentHtml: "<h2>ICGNC 海报修改记录</h2><p>保持公式密集型科研海报风格，重点检查 SMF、Tube MPC 与实验数据的一致性。</p>", contentText: "ICGNC 海报修改记录\n保持公式密集型科研海报风格，重点检查 SMF、Tube MPC 与实验数据的一致性。", contentMarkdown: "## ICGNC 海报修改记录\n\n保持公式密集型科研海报风格，重点检查 SMF、Tube MPC 与实验数据的一致性。", summary: "保持公式密集型科研海报风格，重点检查 SMF、Tube MPC 与实验数据的一致性。", isPinned: true, isFavorite: true, isArchived: false, createdAt: iso(8), updatedAt: iso(0, 10), deletedAt: null, version: 3, tags: [noteTags[0]], relations: [], attachments: [] },
  { id: "note-2", title: "LifeTrace UI 审查清单", noteType: "document", folderId: "folder-work", contentJson: { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "检查信息密度、间距、层级、空状态与桌面尺寸适配。" }] }] }, contentHtml: "<p>检查信息密度、间距、层级、空状态与桌面尺寸适配。</p>", contentText: "检查信息密度、间距、层级、空状态与桌面尺寸适配。", contentMarkdown: "检查信息密度、间距、层级、空状态与桌面尺寸适配。", summary: "检查信息密度、间距、层级、空状态与桌面尺寸适配。", isPinned: false, isFavorite: false, isArchived: false, createdAt: iso(3), updatedAt: iso(1), deletedAt: null, version: 1, tags: [noteTags[1]], relations: [], attachments: [] },
];

const englishArticle = {
  id: "english-article-1",
  title: "Small Systems, Better Days",
  level: "B1",
  category: "Life",
  content: "A good personal system does not need to control every minute of the day. It only needs to make the next useful action easier to see.\n\nWhen information is collected in one place, people spend less energy remembering small details and more energy making decisions. The best systems stay quiet in the background and become useful exactly when they are needed.",
  vocabulary: [
    { word: "useful", phonetic: "/ˈjuːsfəl/", meaning: "有用的", example: "Make the next useful action easier to see." },
    { word: "background", phonetic: "/ˈbækɡraʊnd/", meaning: "后台；背景", example: "The system stays quiet in the background." },
  ],
  questions: ["What makes a personal system useful?", "Why should a system stay quiet in the background?"],
  difficulty: 3,
  estimatedMinutes: 8,
  createdTime: iso(5),
  updatedAt: iso(1),
  source: "local",
  sourceName: "LifeTrace Preview",
  summary: "A short article about designing calm personal systems.",
  wordCount: 92,
  processingStatus: "READY",
  fetchStatus: "SUCCESS",
};
const englishRecord = { id: "english-record-1", userId: "preview-user", date: day(1), articleId: englishArticle.id, readingTimeSeconds: 510, summary: "A useful personal system reduces the effort needed to remember and decide.", score: 86, newWords: ["background"], completionStatus: "completed", readingStatus: "completed", startedAt: iso(1, 8, 0), completedAt: iso(1, 8, 12), createdAt: iso(1, 8, 0), updatedAt: iso(1, 8, 12), article: englishArticle };

const executionProjects = [
  { id: "project-life", userId: "preview-user", name: "LifeTrace", description: "个人管理平台", status: "active", color: "#1f6f56", icon: "PanelsTopLeft", sortOrder: 0, version: 1, createdAt: iso(80), updatedAt: iso() },
  { id: "project-paper", userId: "preview-user", name: "论文与会议", description: "ICGNC 与后续研究", status: "active", color: "#2f6fb3", icon: "FileText", sortOrder: 1, version: 1, createdAt: iso(70), updatedAt: iso() },
];
const executionTasks = [
  { id: "task-1", userId: "preview-user", projectId: "project-life", title: "完成 UI Preview 在线审查流程", description: "让桌面应用前端可以脱离 Tauri/SQLite 在线渲染。", status: "in_progress", priority: "high", estimatedMinutes: 60, dueAt: iso(0, 18, 0), scheduledStartAt: iso(0, 14, 0), scheduledEndAt: iso(0, 15, 0), timezone: "Asia/Singapore", context: "开发", version: 1, createdAt: iso(2), updatedAt: iso() },
  { id: "task-2", userId: "preview-user", projectId: "project-paper", title: "整理 ICGNC 海报修改记录", status: "todo", priority: "normal", estimatedMinutes: 40, dueAt: iso(1, 18, 0), timezone: "Asia/Singapore", context: "研究", version: 1, createdAt: iso(4), updatedAt: iso(1) },
  { id: "task-3", userId: "preview-user", projectId: "project-life", title: "检查账单页面信息密度", status: "todo", priority: "normal", estimatedMinutes: 30, dueAt: iso(0, 20, 0), timezone: "Asia/Singapore", context: "UI", version: 1, createdAt: iso(1), updatedAt: iso() },
];
const calendarEvents = [
  { id: "event-1", userId: "preview-user", title: "UI 审查", description: "检查今天、笔记、财务、执行页面。", isAllDay: false, startAt: iso(0, 14, 0), endAt: iso(0, 15, 0), timezone: "Asia/Singapore", status: "scheduled", sourceTaskId: "task-1", version: 1, createdAt: iso(1), updatedAt: iso() },
];
const waitingItems = [
  { id: "waiting-1", userId: "preview-user", title: "等待在线 Preview 部署", status: "open", waitingFor: "GitHub Pages", expectedAt: iso(0, 12, 30), followUpAt: iso(0, 13, 0), version: 1, createdAt: iso(1), updatedAt: iso() },
];
const memos = [
  { id: "memo-1", userId: "preview-user", content: "UI 审查原则：先看层级，再看密度，最后检查交互状态。", plainText: "UI 审查原则：先看层级，再看密度，最后检查交互状态。", isPinned: true, status: "active", context: "UI", version: 1, createdAt: iso(2), updatedAt: iso(), tags: ["UI", "LifeTrace"] },
];

const mailAccount = { id: "mail-1", userId: "preview-user", provider: "qq", emailAddress: "preview@example.com", displayName: "工作邮箱", imapHost: "imap.qq.com", imapPort: 993, imapSecurity: "tls", smtpHost: "smtp.qq.com", smtpPort: 465, smtpSecurity: "tls", username: "preview@example.com", status: "active", idleSupported: true, lastValidatedAt: iso(1), lastSyncAt: iso(0, 9, 30), createdAt: iso(90), updatedAt: iso() };
const mailMessages = [
  { id: "mail-msg-1", accountId: "mail-1", folderId: "inbox", threadId: "thread-1", subject: "LifeTrace UI Review", fromJson: [{ name: "GitHub", address: "notifications@github.com" }], toJson: [{ address: "preview@example.com" }], receivedAt: iso(0, 10, 5), isRead: false, isArchived: false, snippet: "The latest UI preview build is ready for review.", hasAttachments: false },
  { id: "mail-msg-2", accountId: "mail-1", folderId: "inbox", threadId: "thread-2", subject: "会议资料确认", fromJson: [{ name: "Conference", address: "conference@example.com" }], toJson: [{ address: "preview@example.com" }], receivedAt: iso(1, 16, 20), isRead: true, isArchived: false, snippet: "Please confirm the latest poster materials.", hasAttachments: true },
];

const timelineItems = [
  { id: "timeline-1", occurredAt: iso(0, 10, 10), localDate: day(), domain: "habits", eventType: "habit.completed", title: "钢琴 45 分钟", summary: "完成今日钢琴练习目标", entityType: "habit", entityId: "habit-piano", metrics: { minutes: 45 }, tags: [] },
  { id: "timeline-2", occurredAt: iso(0, 9, 15), localDate: day(), domain: "finance", eventType: "transaction.expense", title: "星巴克 ¥36", summary: "餐饮 · 支付宝", entityType: "transaction", entityId: "tx-1", metrics: { amount: 36 }, tags: [] },
  { id: "timeline-3", occurredAt: iso(1, 20, 20), localDate: day(1), domain: "fitness", eventType: "workout.completed", title: "胸 + 肩 + 三头", summary: "73 分钟 · 8920 kg 总容量", entityType: "workout", entityId: "workout-1", metrics: { durationSeconds: 4380 }, tags: [] },
  { id: "timeline-4", occurredAt: iso(1, 8, 12), localDate: day(1), domain: "english", eventType: "english.completed", title: englishArticle.title, summary: "B1 · 评分 86", entityType: "english_learning_record", entityId: englishRecord.id, metrics: { score: 86 }, tags: [] },
];

const jsonResponse = (body: unknown, status = 200) => new Response(JSON.stringify(body), {
  status,
  headers: { "content-type": "application/json; charset=utf-8" },
});

function requestPath(input: RequestInfo | URL) {
  const raw = input instanceof Request ? input.url : input instanceof URL ? input.toString() : input;
  const url = new URL(raw, window.location.href);
  return `${url.pathname}${url.search}`;
}

function noteById(path: string) {
  const id = new URL(path, window.location.href).searchParams.get("id");
  return notes.find((item) => item.id === id) ?? notes[0];
}

function previewApi(path: string, method: string): Response | undefined {
  const url = new URL(path, window.location.href);
  const pathname = url.pathname;

  if (pathname === "/api/state") return jsonResponse(method === "GET" ? lifeState : { ok: true });

  if (pathname === "/api/notes") {
    const action = url.searchParams.get("action");
    if (method !== "GET") return jsonResponse(action === "create" || action === "update" ? notes[0] : { ok: true, id: "preview-note" });
    if (action === "meta") return jsonResponse({ folders: noteFolders, tags: noteTags });
    if (action === "get") return jsonResponse(noteById(path));
    if (action === "revisions") return jsonResponse([]);
    if (action === "backup") return jsonResponse({ notes, folders: noteFolders, tags: noteTags });
    return jsonResponse(notes);
  }

  if (pathname === "/api/english/today") return jsonResponse({ article: englishArticle, currentLevel: "B1", streak: 12, weekCompleted: [day(3), day(2), day(1)], recentRecords: [englishRecord] });
  if (pathname === "/api/english/history") return jsonResponse({ records: [englishRecord], stats: { readingCount30: 18, averageScore30: 84, vocabularyGrowth30: 42, streak: 12 } });
  if (pathname === "/api/english/vocabulary/stats") return jsonResponse({ dueToday: 8, addedWeek: 16, mastered: 64 });
  if (pathname === "/api/english/assistant") return jsonResponse({ sampleSize: 18, weakPoints: ["冠词", "长句结构"], message: "最近阅读稳定，可以把重点放在英文总结的句子结构上。", nextStage: "保持 B1 阅读，逐步提高输出比例" });
  if (pathname === "/api/english/articles") return jsonResponse(url.searchParams.has("id") ? englishArticle : [englishArticle]);
  if (pathname === "/api/english/highlights") return jsonResponse({ highlights: [], notes: [] });
  if (pathname === "/api/english/reading") return jsonResponse({ status: "reading", record: { ...englishRecord, id: "english-record-preview", date: day(), readingStatus: "reading", completionStatus: "reading", score: undefined, completedAt: undefined, startedAt: iso(0, 8, 0), updatedAt: iso() } });
  if (pathname === "/api/english/vocabulary/settings") return jsonResponse({ preferredAccent: "en-US", wordSpeechRate: 0.8, sentenceSpeechRate: 0.85, autoPronounce: false, defaultFirstMeaning: true, dailyReviewLimit: 20, showSourceSentence: true, includeMasteredInRecommendations: false });
  if (pathname === "/api/english/dictionary/lookup") return jsonResponse({ queryWord: url.searchParams.get("word") ?? "system", normalizedWord: (url.searchParams.get("word") ?? "system").toLowerCase(), found: true, phonetic: "/ˈsɪstəm/", partsOfSpeech: [{ type: "n.", translation: ["系统"], definition: ["a set of connected things"] }], sourceSentence: url.searchParams.get("sentence") ?? "" });
  if (pathname === "/api/english/sync") return jsonResponse({ taskId: "preview-sync" });
  if (pathname === "/api/english/summary") return jsonResponse({ id: "english-record-preview" });
  if (pathname === "/api/english/analyze") return jsonResponse({ analysis: { id: "analysis-preview", userId: "preview-user", recordId: "english-record-preview", articleId: englishArticle.id, provider: "mock", score: 86, contentScore: 88, grammarScore: 82, vocabularyScore: 84, structureScore: 90, mistakes: [], suggestions: ["尝试使用一个复合句连接观点。"], improvedSummary: "A useful personal system reduces memory overhead and makes the next action clear.", weakPoints: ["长句结构"], createdAt: iso(), updatedAt: iso() } });
  if (pathname.startsWith("/api/english/vocabulary")) return jsonResponse(pathname.endsWith("/stats") ? { dueToday: 8, addedWeek: 16, mastered: 64 } : []);

  if (pathname === "/api/execution/projects") return jsonResponse(executionProjects);
  if (pathname === "/api/execution/tasks") return jsonResponse(executionTasks);
  if (pathname === "/api/execution/calendar-events") return jsonResponse(calendarEvents);
  if (pathname === "/api/execution/waiting-items") return jsonResponse(waitingItems);
  if (pathname === "/api/execution/memos") return jsonResponse(memos);
  if (pathname === "/api/execution/reminders" || pathname === "/api/execution/reminders/due") return jsonResponse([]);
  if (pathname === "/api/execution/entity-links") return jsonResponse([]);
  if (pathname.startsWith("/api/execution/tasks/")) {
    const id = pathname.split("/")[4];
    return jsonResponse(executionTasks.find((item) => item.id === id) ?? executionTasks[0]);
  }
  if (pathname.startsWith("/api/execution/")) return jsonResponse(method === "GET" ? [] : { ok: true });

  if (pathname === "/api/analytics/status" || pathname === "/api/analytics/rebuild") return jsonResponse({ dirty: false, eventCount: 128, searchDocumentCount: 76, lastRebuiltAt: iso(0, 7, 0), projectionVersion: 1, lastError: null });
  if (pathname === "/api/analytics/timeline") return jsonResponse({ items: timelineItems, nextCursor: null });
  if (pathname === "/api/analytics/search") return jsonResponse([]);
  if (pathname === "/api/analytics/insights") return jsonResponse([{ id: "insight-1", insightType: "habit_consistency", periodStart: day(6), periodEnd: day(), title: "早晨记录更稳定", summary: "最近一周，上午完成的坚持项目更容易达到目标。", evidence: { sampleSize: 12 }, sampleSize: 12, confidence: { level: "medium", causal: false }, algorithmVersion: "preview-1" }]);
  if (pathname === "/api/analytics/report") return jsonResponse({ id: "report-1", reportType: "weekly", periodStart: day(6), periodEnd: day(), timezone: "Asia/Singapore", generatedAt: iso(), factsVersion: 1, coverage: { finance: true, habits: true, fitness: true, english: true, notes: true, execution: true, reviews: true }, facts: { period: { start: day(6), end: day(), timezone: "Asia/Singapore" }, finance: { transactionCount: 6, expenseCents: 58600, incomeCents: 1200000, netCents: 1141400 }, habits: { logCount: 6, completedCount: 5, completionRate: 0.83 }, fitness: { workoutCount: 2, durationSeconds: 8340, volumeKg: 16340, caloriesKcal: 778 }, english: { sessionCount: 4, readingTimeSeconds: 2400, completedCount: 4, newVocabularyCount: 16 }, notes: { createdCount: 2 }, execution: { taskCount: 3, completedTaskCount: 0, calendarEventCount: 1 }, reviews: { count: 1, averageMood: 7, averageEnergy: 8 } } });

  if (pathname === "/api/assistant/catalog") return jsonResponse({ readOnly: true, datasets: [{ key: "habits", label: "坚持", count: 24 }, { key: "fitness", label: "训练", count: 12 }, { key: "finance", label: "财务", count: 86 }, { key: "notes", label: "笔记", count: 18 }, { key: "english", label: "英语", count: 31 }] });
  if (pathname === "/api/settings/ai") return jsonResponse({ configured: true, model: "deepseek-chat" });
  if (pathname === "/api/assistant/conversations") return jsonResponse(method === "GET" ? { items: [{ id: "conversation-1", title: "回顾最近一周", messageCount: 4, createdAt: iso(3), updatedAt: iso(2) }] } : { updatedAt: iso() });
  if (pathname === "/api/assistant/chat") return jsonResponse({ message: "这是 UI Preview 模式。当前数据来自本地 Mock，用于检查布局、信息密度与交互状态。", model: "preview-model", datasets: ["habits", "finance", "fitness"], usage: { total_tokens: 128 } });

  if (pathname === "/api/v1/auth/refresh") return jsonResponse({ accessToken: "preview-access-token", refreshToken: "preview-refresh-token", tokenType: "Bearer", expiresIn: 3600, refreshExpiresIn: 2592000, user: { id: "preview-user", email: "preview@lifetrace.local", displayName: "UI Preview", state: "active" }, session: { id: "preview-session", appId: "lifetrace-desktop", deviceId: "preview-device", status: "active", createdAt: iso(30), lastSeenAt: iso(), absoluteExpiresAt: iso(-30) }, scopes: [] });
  if (pathname === "/api/v1/mail/accounts") return jsonResponse({ items: [mailAccount] });
  if (pathname.endsWith("/folders") && pathname.startsWith("/api/v1/mail/accounts/")) return jsonResponse({ items: [{ id: "inbox", accountId: "mail-1", remoteName: "INBOX", normalizedRole: "inbox", lastSeenUid: 20, syncEnabled: true, lastSyncAt: iso() }] });
  if (pathname === "/api/v1/mail/threads") return jsonResponse({ items: mailMessages.map((item) => ({ id: item.threadId, accountId: item.accountId, normalizedSubject: item.subject, latestMessageAt: item.receivedAt, messageCount: 1, unreadCount: item.isRead ? 0 : 1, participantSummary: "Preview", snippet: item.snippet })) });
  if (pathname === "/api/v1/mail/messages") return jsonResponse({ items: mailMessages, hasMore: false, nextOffset: mailMessages.length });
  if (pathname.startsWith("/api/v1/mail/threads/") && pathname.endsWith("/messages")) return jsonResponse({ items: mailMessages });
  if (pathname.startsWith("/api/v1/mail/messages/")) return jsonResponse({ ...mailMessages[0], remoteUid: 1, uidvalidity: 1, ccJson: [], replyToJson: [], flagsJson: [], bodyText: "The latest UI preview build is ready for review.", bodyHtmlSanitized: "<p>The latest UI preview build is ready for review.</p>", sizeBytes: 2048 });
  if (pathname.startsWith("/api/v1/mail/")) return jsonResponse({ ok: true, items: [] });

  if (pathname.startsWith("/api/")) return jsonResponse(method === "GET" ? [] : { ok: true });
  return undefined;
}

function installNativePreviewApis() {
  const anyWindow = window as any;
  anyWindow.cloudCredentialApi = {
    set: async () => undefined,
    get: async () => "preview-refresh-token",
    clear: async () => undefined,
  };
  anyWindow.syncApi = {
    setSession: async () => ({ profileId: "preview-profile", cloudUserId: "preview-user", bindingRequired: false, cloudBindingState: "bound" }),
    clearSession: async () => undefined,
    bindCurrentProfile: async () => "preview-user",
    createCloudProfile: async () => "preview-profile",
    profiles: async () => [{ id: "preview-profile", name: "UI Preview", cloudUserId: "preview-user", cloudBindingState: "bound" }],
    setActiveProfile: async () => undefined,
    status: async () => ({ state: "idle", pending: 0, conflicts: 0 }),
    now: async () => ({ ok: true }),
    conflicts: async () => [],
    resolveConflict: async () => undefined,
  };
  const photoStatus = { ok: true, status: { available: true, active: true, managed: true, port: 43110, urls: ["https://192.168.1.8:43110"], photoSyncUrls: ["https://192.168.1.8:43110/photos"], computerName: "LifeTrace Preview", bindAddress: "0.0.0.0", mediaUrl: "https://192.168.1.8:43110", certificateReady: true, certificateAddresses: ["192.168.1.8"], certificateExported: true, certificateCommonName: "LifeTrace Local CA", allowInsecureHttp: false, transportProtocol: "https" } };
  anyWindow.mobileUploadApi = { status: async () => photoStatus, start: async () => photoStatus, stop: async () => ({ ...photoStatus, status: { ...photoStatus.status, active: false } }) };
  anyWindow.photoSyncApi = { status: async () => photoStatus, createPairing: async () => ({ ...photoStatus, status: { ...photoStatus.status, pairing: { success: true, pairCode: "483921", expiresAt: iso(-1), entryUrl: "https://192.168.1.8:43110/pair" } } }), cancelPairing: async () => photoStatus, recover: async () => photoStatus, exportCertificate: async () => photoStatus, setCompatibilityMode: async () => photoStatus };
  anyWindow.storageApi = { status: async () => ({ currentPath: "C:\\Users\\Preview\\LifeTrace", defaultPath: "C:\\Users\\Preview\\LifeTrace", phase: "ready", filesTotal: 8, filesCopied: 8, bytesTotal: 52428800, bytesCopied: 52428800, progress: 1, restartRequired: false, error: null }), chooseAndMigrate: async () => ({ canceled: true }), restart: async () => undefined };
  anyWindow.noteApi = { selectAttachment: async () => ({ ok: false, canceled: true }), openAttachment: async () => ({ ok: true }), showAttachment: async () => ({ ok: true }), deleteAttachment: async () => ({ ok: true }), exportNote: async () => ({ ok: true, filePath: "C:\\Preview\\note.md" }), importMarkdown: async () => ({ ok: false, canceled: true }), onCommand: () => () => undefined };
  anyWindow.vaultApi = { status: async () => ({ configured: true, unlocked: true, assetCount: 36, trashCount: 2, albumCount: 3, autoLockSeconds: 300, lockOnBlur: true }), initialize: async () => ({ configured: true, unlocked: true, autoLockSeconds: 300, lockOnBlur: true }), unlock: async () => ({ configured: true, unlocked: true, autoLockSeconds: 300, lockOnBlur: true }), lock: async () => ({ configured: true, unlocked: false, autoLockSeconds: 300, lockOnBlur: true }), listAssets: async () => [], listAlbums: async () => [{ id: "album-1", name: "旅行", createdAt: iso(20) }, { id: "album-2", name: "生活", createdAt: iso(18) }], hidePhotosFromSyncAlbum: async () => ({ started: true, count: 0 }), restoreToSyncAlbum: async () => ({}), readAsset: async () => ({}), readThumbnail: async () => ({}), moveToTrash: async () => undefined, restoreAsset: async () => undefined, deleteAssetPermanently: async () => undefined, createAlbum: async (name: string) => ({ id: `album-${Date.now()}`, name, createdAt: iso() }), renameAlbum: async () => undefined, deleteAlbum: async () => undefined, setAssetAlbum: async () => undefined, verifyIntegrity: async () => ({ checked: 36, healthy: 36, corruptedAssetIds: [] }), changePassword: async () => ({ configured: true, unlocked: true, autoLockSeconds: 300, lockOnBlur: true }), setAutoLock: async (seconds: number) => ({ configured: true, unlocked: true, autoLockSeconds: seconds, lockOnBlur: true }), setLockOnBlur: async (enabled: boolean) => ({ configured: true, unlocked: true, autoLockSeconds: 300, lockOnBlur: enabled }), deleteAll: async () => undefined };
}

export function installUiPreviewMocks() {
  document.documentElement.dataset.uiPreview = "true";
  installNativePreviewApis();

  cloudAuthClient.configure(previewOrigin);
  useCloudAuthStore.setState({
    origin: previewOrigin,
    user: { id: "preview-user", email: "preview@lifetrace.local", displayName: "UI Preview", state: "active" },
    authenticated: true,
    scopes: [],
    phase: "authenticated",
    loading: false,
    initialized: true,
    error: undefined,
  });

  const nativeFetch = window.fetch.bind(window);
  window.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const path = requestPath(input);
    const method = (init?.method || (input instanceof Request ? input.method : "GET") || "GET").toUpperCase();
    const mocked = previewApi(path, method);
    if (mocked) return mocked;
    return nativeFetch(input, init);
  }) as typeof window.fetch;
}
