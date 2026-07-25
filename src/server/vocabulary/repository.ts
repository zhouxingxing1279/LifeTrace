import { env } from "cloudflare:workers";
import { createId } from "@/src/utils/id";
import type {
  UserVocabulary, VocabularyOccurrence, VocabularyReviewLog, VocabularyReviewResult,
  VocabularySettings, VocabularyStatus,
} from "@/src/types/english";
import { scheduleReview } from "./reviewScheduler";

const DEFAULT_SETTINGS: VocabularySettings = {
  preferredAccent: "en-US", wordSpeechRate: 0.8, sentenceSpeechRate: 0.85,
  autoPronounce: false, defaultFirstMeaning: true, dailyReviewLimit: 20,
  showSourceSentence: true, includeMasteredInRecommendations: false,
};

type Row = Record<string, string | number | null>;
const text = (row: Row, key: string) => String(row[key] ?? "");
const number = (row: Row, key: string) => Number(row[key] ?? 0);
const parseJson = <T>(value: unknown, fallback: T): T => {
  try { return value ? JSON.parse(String(value)) as T : fallback; } catch { return fallback; }
};

let vocabularySchemaReady: Promise<void> | undefined;

async function ensureVocabularySchema() {
  if (!vocabularySchemaReady) {
    vocabularySchemaReady = (async () => {
      await env.DB.batch([
        env.DB.prepare(`CREATE TABLE IF NOT EXISTS english_user_vocabulary (
          id TEXT PRIMARY KEY NOT NULL, word TEXT NOT NULL, normalized_word TEXT NOT NULL,
          lemma TEXT NOT NULL, dictionary_word_id INTEGER, phonetic TEXT,
          selected_meanings_json TEXT NOT NULL, part_of_speech TEXT,
          source_article_id TEXT, source_article_title TEXT, source_sentence TEXT, notes TEXT,
          mastery_level INTEGER DEFAULT 0 NOT NULL, review_stage INTEGER DEFAULT 0 NOT NULL,
          review_count INTEGER DEFAULT 0 NOT NULL, correct_count INTEGER DEFAULT 0 NOT NULL,
          incorrect_count INTEGER DEFAULT 0 NOT NULL, encounter_count INTEGER DEFAULT 1 NOT NULL,
          last_reviewed_at TEXT, next_review_at TEXT, status TEXT DEFAULT 'LEARNING' NOT NULL,
          frequency_rank INTEGER, tags_json TEXT DEFAULT '[]' NOT NULL,
          created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        )`),
        env.DB.prepare("CREATE UNIQUE INDEX IF NOT EXISTS english_user_vocabulary_lemma_unique ON english_user_vocabulary (lemma)"),
        env.DB.prepare("CREATE INDEX IF NOT EXISTS english_user_vocabulary_review_idx ON english_user_vocabulary (status, next_review_at)"),
        env.DB.prepare("CREATE INDEX IF NOT EXISTS english_user_vocabulary_created_idx ON english_user_vocabulary (created_at)"),
        env.DB.prepare(`CREATE TABLE IF NOT EXISTS english_vocabulary_occurrences (
          id TEXT PRIMARY KEY NOT NULL, vocabulary_id TEXT NOT NULL, article_id TEXT,
          article_title TEXT, source_sentence TEXT NOT NULL, created_at TEXT NOT NULL,
          FOREIGN KEY (vocabulary_id) REFERENCES english_user_vocabulary(id) ON DELETE CASCADE
        )`),
        env.DB.prepare(`CREATE UNIQUE INDEX IF NOT EXISTS english_vocabulary_occurrence_unique
          ON english_vocabulary_occurrences (vocabulary_id, article_id, source_sentence)`),
        env.DB.prepare(`CREATE INDEX IF NOT EXISTS english_vocabulary_occurrence_word_idx
          ON english_vocabulary_occurrences (vocabulary_id, created_at)`),
        env.DB.prepare(`CREATE TABLE IF NOT EXISTS english_vocabulary_review_logs (
          id TEXT PRIMARY KEY NOT NULL, vocabulary_id TEXT NOT NULL, result TEXT NOT NULL,
          stage_before INTEGER NOT NULL, stage_after INTEGER NOT NULL, reviewed_at TEXT NOT NULL,
          next_review_at TEXT, response_time_ms INTEGER,
          FOREIGN KEY (vocabulary_id) REFERENCES english_user_vocabulary(id) ON DELETE CASCADE
        )`),
        env.DB.prepare(`CREATE INDEX IF NOT EXISTS english_vocabulary_review_log_idx
          ON english_vocabulary_review_logs (vocabulary_id, reviewed_at)`),
        env.DB.prepare(`CREATE TABLE IF NOT EXISTS english_vocabulary_settings (
          id TEXT PRIMARY KEY NOT NULL, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
        )`),
      ]);

      const legacyTable = await env.DB.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='english_vocabulary'",
      ).first<{ name: string }>();
      if (legacyTable) {
        await env.DB.prepare(`INSERT OR IGNORE INTO english_user_vocabulary
          (id,word,normalized_word,lemma,phonetic,selected_meanings_json,part_of_speech,
           source_article_id,source_sentence,mastery_level,review_stage,review_count,
           next_review_at,status,created_at,updated_at)
          SELECT id,
           json_extract(data_json,'$.word'),
           lower(json_extract(data_json,'$.word')),
           lower(json_extract(data_json,'$.word')),
           json_extract(data_json,'$.phonetic'),
           json_array(json_extract(data_json,'$.meaning')),
           '',
           json_extract(data_json,'$.sourceArticleId'),
           json_extract(data_json,'$.example'),
           coalesce(json_extract(data_json,'$.masterLevel'),0),
           coalesce(json_extract(data_json,'$.masterLevel'),0),
           coalesce(json_extract(data_json,'$.reviewCount'),0),
           json_extract(data_json,'$.nextReviewTime'),
           CASE WHEN coalesce(json_extract(data_json,'$.masterLevel'),0) >= 5 THEN 'MASTERED' ELSE 'LEARNING' END,
           coalesce(json_extract(data_json,'$.createdAt'),datetime('now')),
           coalesce(json_extract(data_json,'$.updatedAt'),datetime('now'))
          FROM english_vocabulary
          WHERE json_extract(data_json,'$.word') IS NOT NULL`).run();
      }
    })().catch((error) => {
      vocabularySchemaReady = undefined;
      throw error;
    });
  }
  return vocabularySchemaReady;
}

const mapWord = (row: Row): UserVocabulary => ({
  id: text(row, "id"), word: text(row, "word"), normalizedWord: text(row, "normalized_word"),
  lemma: text(row, "lemma"), dictionaryWordId: row.dictionary_word_id == null ? undefined : number(row, "dictionary_word_id"),
  phonetic: text(row, "phonetic"), selectedMeanings: parseJson(row.selected_meanings_json, []),
  partOfSpeech: text(row, "part_of_speech"), sourceArticleId: text(row, "source_article_id") || undefined,
  sourceArticleTitle: text(row, "source_article_title") || undefined, sourceSentence: text(row, "source_sentence") || undefined,
  notes: text(row, "notes"), masteryLevel: number(row, "mastery_level"), reviewStage: number(row, "review_stage"),
  reviewCount: number(row, "review_count"), correctCount: number(row, "correct_count"),
  incorrectCount: number(row, "incorrect_count"), encounterCount: number(row, "encounter_count"),
  lastReviewedAt: text(row, "last_reviewed_at") || undefined, nextReviewAt: text(row, "next_review_at") || undefined,
  status: text(row, "status") as VocabularyStatus, frequencyRank: row.frequency_rank == null ? undefined : number(row, "frequency_rank"),
  tags: parseJson(row.tags_json, []), createdAt: text(row, "created_at"), updatedAt: text(row, "updated_at"),
});

export async function addUserVocabulary(input: {
  word: string; normalizedWord: string; lemma: string; dictionaryWordId?: number; phonetic?: string;
  selectedMeanings: string[]; partOfSpeech?: string; sourceArticleId?: string; sourceArticleTitle?: string;
  sourceSentence?: string; frequencyRank?: number; tags?: string[];
}) {
  await ensureVocabularySchema();
  const stamp = new Date().toISOString();
  const id = createId();
  await env.DB.prepare(`INSERT INTO english_user_vocabulary
    (id,word,normalized_word,lemma,dictionary_word_id,phonetic,selected_meanings_json,part_of_speech,
     source_article_id,source_article_title,source_sentence,next_review_at,frequency_rank,tags_json,created_at,updated_at)
    VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
    ON CONFLICT(lemma) DO UPDATE SET encounter_count=encounter_count+1,updated_at=excluded.updated_at`)
    .bind(id, input.word, input.normalizedWord, input.lemma, input.dictionaryWordId ?? null, input.phonetic ?? "",
      JSON.stringify(input.selectedMeanings), input.partOfSpeech ?? "", input.sourceArticleId ?? null,
      input.sourceArticleTitle ?? null, input.sourceSentence ?? null, stamp, input.frequencyRank ?? null,
      JSON.stringify(input.tags ?? []), stamp, stamp).run();
  const row = await env.DB.prepare("SELECT * FROM english_user_vocabulary WHERE lemma=?").bind(input.lemma).first<Row>();
  if (!row) throw new Error("生词保存失败");
  if (input.sourceSentence?.trim()) await addOccurrence(text(row, "id"), {
    articleId: input.sourceArticleId, articleTitle: input.sourceArticleTitle, sourceSentence: input.sourceSentence,
  });
  return mapWord(row);
}

export async function addOccurrence(vocabularyId: string, input: { articleId?: string; articleTitle?: string; sourceSentence: string }) {
  await ensureVocabularySchema();
  const item = { id: createId(), vocabularyId, ...input, createdAt: new Date().toISOString() };
  await env.DB.prepare(`INSERT OR IGNORE INTO english_vocabulary_occurrences
    (id,vocabulary_id,article_id,article_title,source_sentence,created_at) VALUES (?,?,?,?,?,?)`)
    .bind(item.id, vocabularyId, input.articleId ?? null, input.articleTitle ?? null, input.sourceSentence.trim(), item.createdAt).run();
  return item;
}

export async function listUserVocabulary(options: {
  query?: string; status?: string; sort?: string; articleId?: string; pos?: string; tag?: string; due?: boolean; page?: number; pageSize?: number;
} = {}) {
  await ensureVocabularySchema();
  const conditions: string[] = []; const values: unknown[] = [];
  if (options.query) { conditions.push("(word LIKE ? OR lemma LIKE ?)"); values.push(`%${options.query}%`, `%${options.query}%`); }
  if (options.status && options.status !== "ALL") { conditions.push("status=?"); values.push(options.status); }
  if (options.articleId) { conditions.push("source_article_id=?"); values.push(options.articleId); }
  if (options.pos) { conditions.push("part_of_speech=?"); values.push(options.pos); }
  if (options.tag) { conditions.push("tags_json LIKE ?"); values.push(`%\"${options.tag}\"%`); }
  if (options.due) { conditions.push("status NOT IN ('MASTERED','ARCHIVED') AND next_review_at<=?"); values.push(new Date().toISOString()); }
  const where = conditions.length ? `WHERE ${conditions.join(" AND ")}` : "";
  const order = options.sort === "review" ? "next_review_at ASC" : options.sort === "frequency" ? "frequency_rank ASC" : "created_at DESC";
  const pageSize = Math.max(1, Math.min(100, options.pageSize ?? 50));
  const page = Math.max(1, options.page ?? 1);
  const result = await env.DB.prepare(`SELECT * FROM english_user_vocabulary ${where} ORDER BY ${order} LIMIT ? OFFSET ?`)
    .bind(...values, pageSize, (page - 1) * pageSize).all<Row>();
  const count = await env.DB.prepare(`SELECT COUNT(*) count FROM english_user_vocabulary ${where}`).bind(...values).first<{count:number}>();
  return { items: result.results.map(mapWord), total: Number(count?.count ?? 0), page, pageSize };
}

export async function getUserVocabulary(id: string) {
  await ensureVocabularySchema();
  const row = await env.DB.prepare("SELECT * FROM english_user_vocabulary WHERE id=?").bind(id).first<Row>();
  if (!row) return null;
  const [occurrences, logs] = await Promise.all([
    env.DB.prepare("SELECT * FROM english_vocabulary_occurrences WHERE vocabulary_id=? ORDER BY created_at DESC").bind(id).all<Row>(),
    env.DB.prepare("SELECT * FROM english_vocabulary_review_logs WHERE vocabulary_id=? ORDER BY reviewed_at DESC").bind(id).all<Row>(),
  ]);
  return {
    ...mapWord(row),
    occurrences: occurrences.results.map((item): VocabularyOccurrence => ({
      id: text(item, "id"), vocabularyId: text(item, "vocabulary_id"), articleId: text(item, "article_id") || undefined,
      articleTitle: text(item, "article_title") || undefined, sourceSentence: text(item, "source_sentence"), createdAt: text(item, "created_at"),
    })),
    reviewLogs: logs.results.map((item): VocabularyReviewLog => ({
      id: text(item, "id"), vocabularyId: text(item, "vocabulary_id"), result: text(item, "result") as VocabularyReviewResult,
      stageBefore: number(item, "stage_before"), stageAfter: number(item, "stage_after"), reviewedAt: text(item, "reviewed_at"),
      nextReviewAt: text(item, "next_review_at") || undefined, responseTimeMs: number(item, "response_time_ms") || undefined,
    })),
  };
}

export async function updateUserVocabulary(id: string, patch: { selectedMeanings?: string[]; notes?: string; status?: VocabularyStatus; reset?: boolean }) {
  await ensureVocabularySchema();
  const existing = await getUserVocabulary(id);
  if (!existing) throw new Error("生词不存在");
  const status = patch.reset ? "LEARNING" : patch.status ?? existing.status;
  const stage = patch.reset ? 0 : status === "MASTERED" ? 6 : existing.reviewStage;
  const mastery = status === "MASTERED" ? 5 : Math.min(5, stage);
  const next = patch.reset ? new Date().toISOString() : status === "MASTERED" ? null : existing.nextReviewAt ?? null;
  await env.DB.prepare(`UPDATE english_user_vocabulary SET selected_meanings_json=?,notes=?,status=?,review_stage=?,
    mastery_level=?,next_review_at=?,updated_at=? WHERE id=?`)
    .bind(JSON.stringify(patch.selectedMeanings ?? existing.selectedMeanings), patch.notes ?? existing.notes,
      status, stage, mastery, next, new Date().toISOString(), id).run();
  return getUserVocabulary(id);
}

export async function deleteUserVocabulary(id: string) {
  await ensureVocabularySchema();
  await env.DB.batch([
    env.DB.prepare("DELETE FROM english_vocabulary_review_logs WHERE vocabulary_id=?").bind(id),
    env.DB.prepare("DELETE FROM english_vocabulary_occurrences WHERE vocabulary_id=?").bind(id),
    env.DB.prepare("DELETE FROM english_user_vocabulary WHERE id=?").bind(id),
  ]);
}

export async function reviewUserVocabulary(id: string, result: VocabularyReviewResult, responseTimeMs?: number) {
  await ensureVocabularySchema();
  const existing = await getUserVocabulary(id);
  if (!existing) throw new Error("生词不存在");
  if (existing.status === "ARCHIVED") throw new Error("已归档生词不能参与复习");
  const now = new Date();
  const schedule = scheduleReview(existing.reviewStage, result, now);
  await env.DB.batch([
    env.DB.prepare(`UPDATE english_user_vocabulary SET review_stage=?,mastery_level=?,status=?,review_count=review_count+1,
      correct_count=correct_count+?,incorrect_count=incorrect_count+?,last_reviewed_at=?,next_review_at=?,updated_at=? WHERE id=?`)
      .bind(schedule.stageAfter, Math.min(5, schedule.stageAfter), schedule.status,
        result === "GOOD" || result === "EASY" ? 1 : 0, result === "FORGOT" ? 1 : 0,
        now.toISOString(), schedule.nextReviewAt, now.toISOString(), id),
    env.DB.prepare(`INSERT INTO english_vocabulary_review_logs
      (id,vocabulary_id,result,stage_before,stage_after,reviewed_at,next_review_at,response_time_ms) VALUES (?,?,?,?,?,?,?,?)`)
      .bind(createId(), id, result, schedule.stageBefore, schedule.stageAfter, now.toISOString(), schedule.nextReviewAt, responseTimeMs ?? null),
  ]);
  return getUserVocabulary(id);
}

export async function vocabularyStats() {
  await ensureVocabularySchema();
  const now = new Date().toISOString();
  const week = new Date(Date.now() - 7 * 86400000).toISOString();
  const rows = await env.DB.prepare(`SELECT
    COUNT(*) total, SUM(status='MASTERED') mastered, SUM(status IN ('LEARNING','REVIEWING')) learning,
    SUM(status!='ARCHIVED' AND next_review_at<=?) due_today, SUM(created_at>=?) added_week
    FROM english_user_vocabulary`).bind(now, week).first<Record<string, number>>();
  const reviewed = await env.DB.prepare("SELECT COUNT(*) count FROM english_vocabulary_review_logs WHERE reviewed_at>=?").bind(week).first<{count:number}>();
  const accuracy = await env.DB.prepare("SELECT SUM(correct_count) correct,SUM(correct_count+incorrect_count) attempts FROM english_user_vocabulary").first<Record<string, number>>();
  const [addedDaily, reviewedDaily, mastery, sources, frequency, reviewDays] = await Promise.all([
    env.DB.prepare("SELECT substr(created_at,1,10) day,COUNT(*) count FROM english_user_vocabulary WHERE created_at>=? GROUP BY day ORDER BY day").bind(new Date(Date.now()-30*86400000).toISOString()).all<Row>(),
    env.DB.prepare("SELECT substr(reviewed_at,1,10) day,COUNT(*) count FROM english_vocabulary_review_logs WHERE reviewed_at>=? GROUP BY day ORDER BY day").bind(new Date(Date.now()-30*86400000).toISOString()).all<Row>(),
    env.DB.prepare("SELECT review_stage stage,COUNT(*) count FROM english_user_vocabulary GROUP BY review_stage ORDER BY review_stage").all<Row>(),
    env.DB.prepare("SELECT coalesce(source_article_title,'未知来源') source,COUNT(*) count FROM english_user_vocabulary GROUP BY source ORDER BY count DESC LIMIT 8").all<Row>(),
    env.DB.prepare(`SELECT CASE WHEN frequency_rank IS NULL THEN '未知' WHEN frequency_rank<=1000 THEN '高频'
      WHEN frequency_rank<=5000 THEN '中频' ELSE '低频' END bucket,COUNT(*) count FROM english_user_vocabulary GROUP BY bucket`).all<Row>(),
    env.DB.prepare("SELECT DISTINCT substr(reviewed_at,1,10) day FROM english_vocabulary_review_logs ORDER BY day DESC LIMIT 365").all<Row>(),
  ]);
  const reviewedSet = new Set(reviewDays.results.map((row)=>text(row,"day")));
  let reviewStreak = 0; const cursor = new Date();
  if (!reviewedSet.has(cursor.toISOString().slice(0,10))) cursor.setUTCDate(cursor.getUTCDate()-1);
  while (reviewedSet.has(cursor.toISOString().slice(0,10))) { reviewStreak += 1; cursor.setUTCDate(cursor.getUTCDate()-1); }
  return { total: Number(rows?.total ?? 0), mastered: Number(rows?.mastered ?? 0), learning: Number(rows?.learning ?? 0),
    dueToday: Number(rows?.due_today ?? 0), addedWeek: Number(rows?.added_week ?? 0), reviewedWeek: Number(reviewed?.count ?? 0),
    averageAccuracy: accuracy?.attempts ? Math.round(100 * Number(accuracy.correct) / Number(accuracy.attempts)) : 0,
    addedDaily: addedDaily.results.map((row)=>({day:text(row,"day"),count:number(row,"count")})),
    reviewedDaily: reviewedDaily.results.map((row)=>({day:text(row,"day"),count:number(row,"count")})),
    masteryDistribution: mastery.results.map((row)=>({stage:number(row,"stage"),count:number(row,"count")})),
    sourceDistribution: sources.results.map((row)=>({source:text(row,"source"),count:number(row,"count")})),
    frequencyDistribution: frequency.results.map((row)=>({bucket:text(row,"bucket"),count:number(row,"count")})),
    reviewStreak,
  };
}

export async function getVocabularySettings(): Promise<VocabularySettings> {
  await ensureVocabularySchema();
  const row = await env.DB.prepare("SELECT data_json FROM english_vocabulary_settings WHERE id='preferences'").first<{data_json:string}>();
  return { ...DEFAULT_SETTINGS, ...parseJson(row?.data_json, {}) };
}

export async function saveVocabularySettings(patch: Partial<VocabularySettings>) {
  await ensureVocabularySchema();
  const settings = { ...await getVocabularySettings(), ...patch };
  await env.DB.prepare(`INSERT INTO english_vocabulary_settings(id,data_json,updated_at) VALUES ('preferences',?,?)
    ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json,updated_at=excluded.updated_at`)
    .bind(JSON.stringify(settings), new Date().toISOString()).run();
  return settings;
}
