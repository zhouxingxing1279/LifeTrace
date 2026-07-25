import { env } from "cloudflare:workers";
import { englishAnalysisService } from "@/src/services/englishAnalysis";
import { runEnglishSyncTask, scheduleEnglishSync } from "@/src/server/englishSync/service";
import type { Activity, ActivityLog } from "@/src/types";
import type {
  ArticleVocabularyItem,
  CEFRLevel,
  EnglishAIAnalysis,
  EnglishArticle,
  EnglishHighlight,
  EnglishHistoryResponse,
  EnglishLearningRecord,
  EnglishNote,
  EnglishTodayResponse,
  EnglishVocabulary,
  EnglishSourceSyncResult,
} from "@/src/types/english";

type EnglishTable = "articles" | "records" | "vocabulary" | "highlights" | "notes" | "analysis";
type EnglishEntity = EnglishArticle | EnglishLearningRecord | EnglishVocabulary | EnglishHighlight | EnglishNote | EnglishAIAnalysis;

const tableNames: Record<EnglishTable, string> = {
  articles: "english_articles",
  records: "english_learning_records",
  vocabulary: "english_vocabulary",
  highlights: "english_highlights",
  notes: "english_notes",
  analysis: "english_ai_analysis",
};

const USER_ID = "local-user";
const LEVELS: CEFRLevel[] = ["A1", "A2", "B1", "B2", "C1"];

const vocabulary = (word: string, phonetic: string, meaning: string, example: string): ArticleVocabularyItem => ({ word, phonetic, meaning, example });

// 文章均为项目内原创内容，作为本地首批每日任务；后续可以通过文章 API 持续增加。
const seedArticles: EnglishArticle[] = [
  {
    id: "english-b1-exercise-brain",
    title: "How Exercise Changes Your Brain",
    level: "B1",
    category: "Science",
    difficulty: 3,
    estimatedMinutes: 15,
    content: `Many people exercise because they want stronger muscles or a healthier heart. However, physical activity also changes the brain. When we move, the heart sends more blood and oxygen to brain cells. This helps us feel awake and makes it easier to pay attention.

Exercise also encourages the brain to produce chemicals that support learning and memory. One of these chemicals helps new connections grow between brain cells. These connections are important when we learn a language, solve a problem, or remember a new skill.

The benefits do not require a difficult workout. A fast walk, a short bike ride, or fifteen minutes of dancing can improve mood and reduce stress. Regular movement is more useful than one very hard session. Scientists suggest choosing an activity that is enjoyable, because people are more likely to repeat it.

In this way, exercise is not only training for the body. It is also a daily investment in clearer thinking, better memory, and emotional health.`,
    vocabulary: [
      vocabulary("encourage", "/ɪnˈkʌrɪdʒ/", "促进；鼓励", "Exercise encourages the brain to build new connections."),
      vocabulary("connection", "/kəˈnekʃən/", "连接", "Learning creates connections between brain cells."),
      vocabulary("investment", "/ɪnˈvestmənt/", "投入；投资", "Daily movement is an investment in long-term health."),
    ],
    questions: ["How does movement help brain cells?", "Why is enjoyable exercise easier to maintain?", "What is the article's main conclusion?"],
    createdTime: "2026-07-24T00:00:00.000Z",
    updatedAt: "2026-07-24T00:00:00.000Z",
  },
  {
    id: "english-a1-small-habits",
    title: "Small Habits, Big Changes",
    level: "A1",
    category: "Life",
    difficulty: 1,
    estimatedMinutes: 8,
    content: `A habit is something we do often. Some habits are small. We drink water in the morning, walk after dinner, or read before bed.

Small habits can make a big change. The first step should be easy. A person can read one page, learn three English words, or exercise for five minutes. An easy action is simple to repeat.

It also helps to do the habit at the same time each day. After many days, the action feels natural. We do not need to be perfect. We only need to begin again when we miss a day.`,
    vocabulary: [
      vocabulary("habit", "/ˈhæbɪt/", "习惯", "Reading every morning is a useful habit."),
      vocabulary("repeat", "/rɪˈpiːt/", "重复", "Repeat the new word three times."),
      vocabulary("natural", "/ˈnætʃərəl/", "自然的", "The action feels natural after a few weeks."),
    ],
    questions: ["What is a habit?", "Why should the first step be easy?", "What should we do after missing a day?"],
    createdTime: "2026-07-24T00:00:00.000Z",
    updatedAt: "2026-07-24T00:00:00.000Z",
  },
  {
    id: "english-a2-city-trees",
    title: "Why Cities Need More Trees",
    level: "A2",
    category: "Science",
    difficulty: 2,
    estimatedMinutes: 10,
    content: `Trees make city streets more comfortable. Their leaves create shade, so roads and buildings stay cooler on hot days. Trees also take in carbon dioxide and release oxygen.

City trees support animals too. Birds and insects use them for food and shelter. People benefit from green spaces because natural places can lower stress and encourage walking.

Planting a tree is only the beginning. Young trees need water, healthy soil, and protection. City planners must choose the right tree for each street. With long-term care, trees can make a neighborhood healthier for many years.`,
    vocabulary: [
      vocabulary("shade", "/ʃeɪd/", "阴凉处；遮阴", "We sat in the shade of a large tree."),
      vocabulary("shelter", "/ˈʃeltər/", "庇护处", "Trees provide shelter for birds."),
      vocabulary("neighborhood", "/ˈneɪbərhʊd/", "社区", "The park makes our neighborhood greener."),
    ],
    questions: ["How do trees cool a city?", "Which animals use trees?", "What care do young trees need?"],
    createdTime: "2026-07-24T00:00:00.000Z",
    updatedAt: "2026-07-24T00:00:00.000Z",
  },
  {
    id: "english-b2-ai-decisions",
    title: "Keeping Humans in AI Decisions",
    level: "B2",
    category: "Technology",
    difficulty: 4,
    estimatedMinutes: 18,
    content: `Artificial intelligence can compare large amounts of information faster than any individual. Hospitals use it to identify patterns in medical images, while businesses use it to predict demand. Yet speed does not guarantee a fair or responsible decision.

An AI system learns from historical data. If that data reflects old inequalities or incomplete records, the system may repeat those problems. Its answer can look objective even when the process behind it is weak. For this reason, organizations need people who can question the result, explain its limits, and take responsibility for the final choice.

Human oversight should involve more than approving an automated recommendation. Reviewers need enough knowledge, time, and authority to disagree. The organization should also record why a decision was made and provide a way for affected people to appeal.

The most useful approach is therefore collaboration. Machines can find patterns and reduce routine work, while humans contribute context, values, and accountability. Good AI does not remove human judgment; it gives that judgment better tools.`,
    vocabulary: [
      vocabulary("oversight", "/ˈoʊvərsaɪt/", "监督", "Human oversight is necessary for important decisions."),
      vocabulary("inequality", "/ˌɪnɪˈkwɑːləti/", "不平等", "Biased data can reflect social inequality."),
      vocabulary("accountability", "/əˌkaʊntəˈbɪləti/", "问责；责任", "Clear accountability improves public trust."),
    ],
    questions: ["Why can historical data create unfair outcomes?", "What authority should a human reviewer have?", "How does the article define useful AI collaboration?"],
    createdTime: "2026-07-24T00:00:00.000Z",
    updatedAt: "2026-07-24T00:00:00.000Z",
  },
  {
    id: "english-c1-value-of-boredom",
    title: "The Unexpected Value of Boredom",
    level: "C1",
    category: "Culture",
    difficulty: 5,
    estimatedMinutes: 20,
    content: `Modern life treats boredom as a problem to be eliminated. A spare minute is quickly filled by a notification, a short video, or an endless stream of headlines. This constant stimulation feels harmless, but it can deprive the mind of an important form of rest.

When attention is not occupied by an immediate task, thought begins to wander. Such wandering is often dismissed as unproductive, yet it allows distant ideas to meet. People revisit unfinished questions, imagine alternative futures, and notice emotions that were hidden beneath activity. Creative insight frequently appears during an undemanding walk or a quiet shower rather than at a crowded desk.

Boredom is not automatically beneficial. Long periods without purpose can be distressing, and forced inactivity is very different from chosen solitude. The useful form is a modest interval in which the mind is free from both pressure and entertainment.

Protecting these intervals requires deliberate restraint. Leaving a phone behind for a brief walk or waiting without opening an app may initially feel uncomfortable. Over time, however, the absence of input can become a space for reflection. Boredom, in moderation, is not emptiness; it is room for the mind to rearrange itself.`,
    vocabulary: [
      vocabulary("stimulation", "/ˌstɪmjəˈleɪʃən/", "刺激", "Constant stimulation can exhaust our attention."),
      vocabulary("restraint", "/rɪˈstreɪnt/", "克制", "Digital restraint creates time for reflection."),
      vocabulary("moderation", "/ˌmɑːdəˈreɪʃən/", "适度", "Technology is most useful when used in moderation."),
    ],
    questions: ["Why can mind-wandering support creativity?", "How does chosen solitude differ from forced inactivity?", "What practical restraint does the author recommend?"],
    createdTime: "2026-07-24T00:00:00.000Z",
    updatedAt: "2026-07-24T00:00:00.000Z",
  },
];

const dateKey = (date = new Date()) => {
  // 使用分段格式化，避免不同 Worker 运行时把 en-CA 日期输出为斜杠格式。
  const parts = new Intl.DateTimeFormat("en-US", {
    timeZone: "Asia/Shanghai",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(date);
  const value = Object.fromEntries(parts.map((part) => [part.type, part.value]));
  return `${value.year}-${value.month}-${value.day}`;
};

const addDays = (value: string, days: number) => {
  const date = new Date(`${value}T12:00:00+08:00`);
  date.setUTCDate(date.getUTCDate() + days);
  return dateKey(date);
};

const uid = () => crypto.randomUUID();

export async function ensureEnglishSchema() {
  await env.DB.batch(Object.values(tableNames).map((table) =>
    env.DB.prepare(`CREATE TABLE IF NOT EXISTS ${table} (id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL)`),
  ));
  const count = await env.DB.prepare(`SELECT COUNT(*) AS count FROM ${tableNames.articles}`).first<{ count: number }>();
  if (!count?.count) await env.DB.batch(seedArticles.map((article) => putStatement("articles", article)));
}

const putStatement = (table: EnglishTable, value: EnglishEntity) =>
  env.DB.prepare(
    `INSERT INTO ${tableNames[table]} (id, data_json, updated_at) VALUES (?, ?, ?)
     ON CONFLICT(id) DO UPDATE SET data_json = excluded.data_json, updated_at = excluded.updated_at`,
  ).bind(value.id, JSON.stringify(value), "updatedAt" in value ? value.updatedAt : new Date().toISOString());

export const putEnglishEntity = async (table: EnglishTable, value: EnglishEntity) => {
  await ensureEnglishSchema();
  await putStatement(table, value).run();
};

export const readEnglishTable = async <T>(table: EnglishTable): Promise<T[]> => {
  await ensureEnglishSchema();
  const rows = await env.DB.prepare(`SELECT data_json FROM ${tableNames[table]} ORDER BY updated_at DESC`).all<{ data_json: string }>();
  return rows.results.map((row) => JSON.parse(row.data_json) as T);
};

export const getArticle = async (id: string) => (await readEnglishTable<EnglishArticle>("articles")).find((article) => article.id === id);

export async function syncVoaArticles(force = false): Promise<EnglishSourceSyncResult> {
  const scheduled = await scheduleEnglishSync("incremental", undefined, force);
  if (!scheduled.task) {
    return { source: "voa", engine: "python", imported: 0, skipped: 0, failed: 0, syncedAt: new Date().toISOString(), cached: true };
  }
  const task = scheduled.created
    ? await runEnglishSyncTask(scheduled.task.taskId)
    : scheduled.task;
  return {
    source: "voa",
    engine: "python",
    imported: task?.insertedCount ?? 0,
    inserted: task?.insertedCount ?? 0,
    updated: task?.updatedCount ?? 0,
    skipped: task?.skippedCount ?? 0,
    failed: task?.failedCount ?? 0,
    syncedAt: task?.finishedAt ?? new Date().toISOString(),
    cached: !scheduled.created,
    taskId: task?.taskId,
    status: task?.status,
  };
}

const calculateStreak = (records: EnglishLearningRecord[]) => {
  const completed = new Set(records.filter((record) => record.completionStatus === "completed").map((record) => record.date));
  let cursor = dateKey();
  if (!completed.has(cursor)) cursor = addDays(cursor, -1);
  let streak = 0;
  while (completed.has(cursor)) {
    streak += 1;
    cursor = addDays(cursor, -1);
  }
  return streak;
};

const calculateLevel = (records: EnglishLearningRecord[]): CEFRLevel => {
  const scored = records.filter((record) => typeof record.score === "number").slice(0, 10);
  if (!scored.length) return "B1";
  const average = scored.reduce((sum, record) => sum + (record.score ?? 0), 0) / scored.length;
  return average >= 92 ? "C1" : average >= 85 ? "B2" : average >= 70 ? "B1" : average >= 55 ? "A2" : "A1";
};

export async function getTodayEnglish(requestedLevel?: CEFRLevel, articleId?: string): Promise<EnglishTodayResponse> {
  const [articles, records] = await Promise.all([
    readEnglishTable<EnglishArticle>("articles"),
    readEnglishTable<EnglishLearningRecord>("records"),
  ]);
  const readableArticles = articles.filter((article) =>
    !article.processingStatus || article.processingStatus === "READY",
  );
  const currentLevel = requestedLevel ?? calculateLevel(records);
  const sameLevel = readableArticles.filter((article) => article.level === currentLevel);
  const dayNumber = Math.floor(new Date(`${dateKey()}T12:00:00+08:00`).getTime() / 86400000);
  const article = readableArticles.find((item) => item.id === articleId)
    ?? sameLevel[dayNumber % Math.max(1, sameLevel.length)]
    ?? readableArticles[dayNumber % readableArticles.length];
  if (!article) throw new Error("英语文章库为空");
  const weekStart = addDays(dateKey(), -((new Date().getDay() + 6) % 7));
  const recentRecords = records.slice(0, 5).map((record) => ({ ...record, article: articles.find((item) => item.id === record.articleId) }));
  return {
    article,
    record: records.find((record) => record.date === dateKey() && record.articleId === article.id),
    currentLevel,
    streak: calculateStreak(records),
    weekCompleted: records.filter((record) => record.completionStatus === "completed" && record.date >= weekStart).map((record) => record.date),
    recentRecords,
  };
}

export async function listArticles(level?: CEFRLevel, category?: string) {
  const articles = await readEnglishTable<EnglishArticle>("articles");
  return articles.filter((article) =>
    (!article.processingStatus || article.processingStatus === "READY")
    && (!level || article.level === level)
    && (!category || article.category === category),
  );
}

export async function listArticlesPage(options: {
  page?: number;
  pageSize?: number;
  level?: CEFRLevel;
  category?: string;
  query?: string;
}) {
  await ensureEnglishSchema();
  const page = Math.max(1, Math.floor(options.page ?? 1));
  const pageSize = Math.min(48, Math.max(6, Math.floor(options.pageSize ?? 18)));
  const clauses = ["COALESCE(json_extract(data_json, '$.processingStatus'), 'READY') = 'READY'"];
  const values: Array<string | number> = [];

  if (options.level) {
    clauses.push("json_extract(data_json, '$.level') = ?");
    values.push(options.level);
  }
  if (options.category) {
    clauses.push("json_extract(data_json, '$.category') = ?");
    values.push(options.category);
  }
  if (options.query?.trim()) {
    clauses.push("LOWER(json_extract(data_json, '$.title')) LIKE ?");
    values.push(`%${options.query.trim().toLowerCase()}%`);
  }

  const where = clauses.join(" AND ");
  const offset = (page - 1) * pageSize;
  const count = await env.DB.prepare(
    `SELECT COUNT(*) AS count FROM ${tableNames.articles} WHERE ${where}`,
  ).bind(...values).first<{ count: number }>();
  const rows = await env.DB.prepare(
    `SELECT data_json FROM ${tableNames.articles}
     WHERE ${where}
     ORDER BY updated_at DESC
     LIMIT ? OFFSET ?`,
  ).bind(...values, pageSize, offset).all<{ data_json: string }>();
  const total = Number(count?.count ?? 0);

  return {
    articles: rows.results.map((row) => JSON.parse(row.data_json) as EnglishArticle),
    total,
    page,
    pageSize,
    hasMore: offset + rows.results.length < total,
  };
}

export async function saveSummary(input: { articleId: string; summary: string; readingTimeSeconds?: number; recordId?: string }) {
  const records = await readEnglishTable<EnglishLearningRecord>("records");
  const existing = input.recordId
    ? records.find((record) => record.id === input.recordId)
    : records.find((record) => record.articleId === input.articleId && record.date === dateKey());
  const stamp = new Date().toISOString();
  const record: EnglishLearningRecord = {
    id: existing?.id ?? uid(),
    userId: USER_ID,
    date: existing?.date ?? dateKey(),
    articleId: input.articleId,
    readingTimeSeconds: Math.max(existing?.readingTimeSeconds ?? 0, input.readingTimeSeconds ?? 0),
    summary: input.summary.trim(),
    score: existing?.score,
    analysisId: existing?.analysisId,
    newWords: existing?.newWords ?? [],
    completionStatus: "summarized",
    startedAt: existing?.startedAt ?? stamp,
    createdAt: existing?.createdAt ?? stamp,
    updatedAt: stamp,
  };
  await putEnglishEntity("records", record);
  return record;
}

const ensureEnglishHabitLog = async (record: EnglishLearningRecord, article: EnglishArticle) => {
  // 复用平台已有坚持项目和生活日志表，英语闭环完成后自动留下当天记录。
  await env.DB.batch([
    env.DB.prepare("CREATE TABLE IF NOT EXISTS activities (id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL)"),
    env.DB.prepare("CREATE TABLE IF NOT EXISTS activity_logs (id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL)"),
  ]);
  const stamp = record.updatedAt;
  const rows = await env.DB.prepare("SELECT data_json FROM activities").all<{ data_json: string }>();
  const activities = rows.results.map((row) => JSON.parse(row.data_json) as Activity);
  const configuredEnglish = activities.filter((item) => !item.isArchived && item.checkinMethod === "automatic" && item.syncSource === "english");
  const legacyEnglish = activities.find((item) => item.id === "system-daily-english");
  const fallbackActivity: Activity = {
    id: "system-daily-english",
    userId: USER_ID,
    name: "每日英语",
    type: "completion",
    unit: "篇",
    normalTarget: 1,
    targetPeriod: "daily",
    targetDays: [1, 2, 3, 4, 5, 6, 7],
    scheduleType: "daily",
    startDate: record.createdAt.slice(0, 10),
    checkinMethod: "automatic",
    syncSource: "english",
    icon: "english",
    color: "blue",
    description: "完成英文阅读、总结与 AI 反馈后自动记录。",
    isArchived: false,
    createdAt: record.createdAt,
    updatedAt: stamp,
  };
  const targetActivities = configuredEnglish.length ? configuredEnglish : [legacyEnglish ?? fallbackActivity];
  const logs: ActivityLog[] = targetActivities.map((activity) => ({
    id: `english-log-${record.id}-${activity.id}`,
    userId: USER_ID,
    activityId: activity.id,
    value: 1,
    status: "completed",
    note: `完成「${article.title}」阅读与英文总结 · AI 评分 ${record.score ?? 0} 分`,
    createdAt: record.completedAt ?? stamp,
    updatedAt: stamp,
  }));
  await env.DB.batch([
    ...(!legacyEnglish && !configuredEnglish.length
      ? [env.DB.prepare("INSERT INTO activities (id,data_json,updated_at) VALUES (?,?,?) ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json,updated_at=excluded.updated_at")
          .bind(fallbackActivity.id, JSON.stringify(fallbackActivity), fallbackActivity.updatedAt)]
      : []),
    ...logs.map((log) => env.DB.prepare("INSERT INTO activity_logs (id,data_json,updated_at) VALUES (?,?,?) ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json,updated_at=excluded.updated_at")
      .bind(log.id, JSON.stringify(log), log.updatedAt)),
  ]);
};

export async function analyzeSummary(recordId: string, userLevel: CEFRLevel) {
  const records = await readEnglishTable<EnglishLearningRecord>("records");
  const record = records.find((item) => item.id === recordId);
  if (!record) throw new Error("学习记录不存在");
  const article = await getArticle(record.articleId);
  if (!article) throw new Error("文章不存在");
  const analysis = await englishAnalysisService.analyze({ article, summary: record.summary, userLevel, recordId, userId: USER_ID });
  const completedAt = new Date().toISOString();
  const completedRecord: EnglishLearningRecord = {
    ...record,
    score: analysis.score,
    analysisId: analysis.id,
    completionStatus: "completed",
    completedAt,
    updatedAt: completedAt,
  };
  await env.DB.batch([putStatement("analysis", analysis), putStatement("records", completedRecord)]);
  await ensureEnglishHabitLog(completedRecord, article);
  return { analysis, record: completedRecord };
}

export async function getEnglishHistory(): Promise<EnglishHistoryResponse> {
  const [records, articles, analyses, vocabularyItems] = await Promise.all([
    readEnglishTable<EnglishLearningRecord>("records"),
    readEnglishTable<EnglishArticle>("articles"),
    readEnglishTable<EnglishAIAnalysis>("analysis"),
    readEnglishTable<EnglishVocabulary>("vocabulary"),
  ]);
  const since = addDays(dateKey(), -29);
  const recent = records.filter((record) => record.date >= since && record.completionStatus === "completed");
  const wordsSince = vocabularyItems.filter((item) => dateKey(new Date(item.createdAt)) >= since);
  return {
    records: records.map((record) => ({
      ...record,
      article: articles.find((article) => article.id === record.articleId),
      analysis: analyses.find((analysis) => analysis.id === record.analysisId),
    })),
    stats: {
      readingCount30: recent.length,
      averageScore30: recent.length ? Math.round(recent.reduce((sum, record) => sum + (record.score ?? 0), 0) / recent.length) : 0,
      vocabularyGrowth30: wordsSince.length,
      streak: calculateStreak(records),
    },
  };
}

const reviewIntervals = [1, 1, 2, 4, 7, 15, 30];
const nextReview = (level: number) => {
  const date = new Date();
  date.setDate(date.getDate() + reviewIntervals[Math.min(level, reviewIntervals.length - 1)]);
  return date.toISOString();
};

export async function addVocabulary(input: Omit<EnglishVocabulary, "id" | "userId" | "reviewCount" | "masterLevel" | "nextReviewTime" | "createdAt" | "updatedAt">) {
  const items = await readEnglishTable<EnglishVocabulary>("vocabulary");
  const existing = items.find((item) => item.word.toLowerCase() === input.word.toLowerCase());
  if (existing) return existing;
  const stamp = new Date().toISOString();
  const item: EnglishVocabulary = {
    id: uid(),
    userId: USER_ID,
    ...input,
    reviewCount: 0,
    masterLevel: 0,
    nextReviewTime: stamp,
    createdAt: stamp,
    updatedAt: stamp,
  };
  await putEnglishEntity("vocabulary", item);
  return item;
}

export async function listVocabulary(dueOnly = false) {
  const now = new Date().toISOString();
  const items = await readEnglishTable<EnglishVocabulary>("vocabulary");
  return items.filter((item) => !dueOnly || item.nextReviewTime <= now);
}

export async function reviewVocabulary(id: string, mastered: boolean) {
  const items = await readEnglishTable<EnglishVocabulary>("vocabulary");
  const existing = items.find((item) => item.id === id);
  if (!existing) throw new Error("生词不存在");
  const masterLevel = Math.max(0, Math.min(5, existing.masterLevel + (mastered ? 1 : -1)));
  const updated: EnglishVocabulary = {
    ...existing,
    reviewCount: existing.reviewCount + 1,
    masterLevel,
    nextReviewTime: nextReview(masterLevel),
    updatedAt: new Date().toISOString(),
  };
  await putEnglishEntity("vocabulary", updated);
  return updated;
}

export async function saveHighlight(input: Pick<EnglishHighlight, "articleId" | "text"> & Partial<Pick<EnglishHighlight, "color">>) {
  const stamp = new Date().toISOString();
  const item: EnglishHighlight = { id: uid(), userId: USER_ID, articleId: input.articleId, text: input.text.trim(), color: input.color ?? "yellow", createdAt: stamp, updatedAt: stamp };
  await putEnglishEntity("highlights", item);
  return item;
}

export async function saveNote(input: Pick<EnglishNote, "articleId" | "content"> & Partial<Pick<EnglishNote, "quote">>) {
  const stamp = new Date().toISOString();
  const item: EnglishNote = { id: uid(), userId: USER_ID, articleId: input.articleId, quote: input.quote?.trim(), content: input.content.trim(), createdAt: stamp, updatedAt: stamp };
  await putEnglishEntity("notes", item);
  return item;
}

export async function articleAnnotations(articleId: string) {
  const [highlights, notes] = await Promise.all([
    readEnglishTable<EnglishHighlight>("highlights"),
    readEnglishTable<EnglishNote>("notes"),
  ]);
  return { highlights: highlights.filter((item) => item.articleId === articleId), notes: notes.filter((item) => item.articleId === articleId) };
}

export async function assistantInsight() {
  const history = await getEnglishHistory();
  const analyses = history.records.map((record) => record.analysis).filter((item): item is EnglishAIAnalysis => Boolean(item)).slice(0, 10);
  const counts = new Map<string, number>();
  analyses.flatMap((analysis) => analysis.weakPoints).forEach((point) => counts.set(point, (counts.get(point) ?? 0) + 1));
  const weakPoints = [...counts.entries()].sort((left, right) => right[1] - left[1]).slice(0, 3).map(([point]) => point);
  return {
    sampleSize: analyses.length,
    weakPoints,
    message: analyses.length
      ? `最近 ${analyses.length} 篇总结中，${weakPoints.length ? `需要优先改善：${weakPoints.join("、")}` : "整体表达稳定，可以提高文章难度。"}`
      : "完成第一篇阅读和英文总结后，我会分析你的长期薄弱点。",
    nextStage: weakPoints.includes("语法准确性")
      ? "下一阶段每次提交前，用 2 分钟检查时态和主谓一致。"
      : weakPoints.includes("段落结构与连接词")
        ? "下一阶段重点练习 however、therefore、although 等连接词。"
        : "保持每日输入与输出，连续完成 7 天后再评估等级。",
  };
}

export { dateKey, LEVELS };
