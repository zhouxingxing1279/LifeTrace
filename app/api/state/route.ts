import { env } from "cloudflare:workers";
import type { Activity, ActivityLog, DailyReview, FinanceAccount, Transaction, WorkoutHistory } from "@/src/types";
import type { LifeSettings } from "@/src/db/sqliteClient";

type TableKey = "activities" | "logs" | "transactions" | "reviews" | "settings" | "accounts" | "workoutHistory";
type Entity = Activity | ActivityLog | Transaction | DailyReview | LifeSettings | FinanceAccount | WorkoutHistory;
type LifeData = { activities: Activity[]; logs: ActivityLog[]; transactions: Transaction[]; reviews: DailyReview[]; settings: LifeSettings; accounts: FinanceAccount[]; workoutHistory: WorkoutHistory[] };

const tableNames: Record<TableKey, string> = {
  activities: "activities",
  logs: "activity_logs",
  transactions: "transactions",
  reviews: "daily_reviews",
  settings: "settings",
  accounts: "finance_accounts",
  workoutHistory: "workout_history",
};

const seedActivities: Activity[] = [
  { id: "piano", userId: "local-user", name: "钢琴练习", type: "duration", unit: "分钟", minimumTarget: 10, normalTarget: 30, targetPeriod: "daily", targetDays: [1,2,3,4,5,6,7], scheduleType: "daily", startDate: "2026-01-01", checkinMethod: "manual", icon: "piano", color: "violet", isArchived: false, createdAt: "2026-01-01T00:00:00.000Z", updatedAt: "2026-01-01T00:00:00.000Z" },
  { id: "fitness", userId: "local-user", name: "健身", type: "count", unit: "次", normalTarget: 3, targetPeriod: "weekly", targetDays: [1,3,5], scheduleType: "weekly", startDate: "2026-01-01", checkinMethod: "automatic", syncSource: "fitness", icon: "fitness", color: "emerald", isArchived: false, createdAt: "2026-01-01T00:00:00.000Z", updatedAt: "2026-01-01T00:00:00.000Z" },
  { id: "english", userId: "local-user", name: "英语学习", type: "completion", unit: "篇", normalTarget: 1, targetPeriod: "daily", targetDays: [1,2,3,4,5,6,7], scheduleType: "daily", startDate: "2026-01-01", checkinMethod: "automatic", syncSource: "english", icon: "english", color: "blue", isArchived: false, createdAt: "2026-01-01T00:00:00.000Z", updatedAt: "2026-01-01T00:00:00.000Z" },
  { id: "control", userId: "local-user", name: "行为管理", type: "control", unit: "状态", normalTarget: 1, targetPeriod: "daily", targetDays: [1,2,3,4,5,6,7], scheduleType: "daily", startDate: "2026-01-01", checkinMethod: "manual", icon: "target", color: "slate", isArchived: false, createdAt: "2026-01-01T00:00:00.000Z", updatedAt: "2026-01-01T00:00:00.000Z" },
];

const seedAccounts: FinanceAccount[] = [
  ["wechat-wallet","微信零钱","wechat",2350.35,"#2a9c69","微"], ["bank-card","银行卡","bank",32520,"#b24945","银"],
  ["alipay-balance","支付宝余额","alipay",12730,"#3179c9","支"], ["cash","现金","cash",500,"#8a7963","现"],
].map(([id,name,type,balance,color,icon]) => ({ id:String(id), userId:"local-user", name:String(name), type:type as FinanceAccount["type"], balance:Number(balance), color:String(color), icon:String(icon), isArchived:false, createdAt:"2026-01-01T00:00:00.000Z", updatedAt:"2026-01-01T00:00:00.000Z" }));

async function ensureSchema() {
  const statements = Object.values(tableNames).map((table) => env.DB.prepare(`CREATE TABLE IF NOT EXISTS ${table} (id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL)`));
  await env.DB.batch([...statements, env.DB.prepare("CREATE TABLE IF NOT EXISTS app_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)")]);
  const initialized = await env.DB.prepare("SELECT value FROM app_meta WHERE key = ?").bind("initial-seed-complete").first<{ value: string }>();
  const settings = await env.DB.prepare("SELECT COUNT(*) AS count FROM settings").first<{ count: number }>();
  if (!settings?.count) await putStatement("settings", { id: "preferences", dark: false, timer: null, updatedAt: new Date().toISOString() }).run();
  if (!initialized) {
    const count = await env.DB.prepare("SELECT COUNT(*) AS count FROM activities").first<{ count: number }>();
    if (!count?.count) await env.DB.batch(seedActivities.map((item) => putStatement("activities", item)));
    const accountCount = await env.DB.prepare("SELECT COUNT(*) AS count FROM finance_accounts").first<{ count: number }>();
    if (!accountCount?.count) await env.DB.batch(seedAccounts.map((item) => putStatement("accounts", item)));
    await env.DB.prepare("INSERT INTO app_meta (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
      .bind("initial-seed-complete", new Date().toISOString()).run();
  }
}

function putStatement(table: TableKey, value: Entity) {
  return env.DB.prepare(`INSERT INTO ${tableNames[table]} (id, data_json, updated_at) VALUES (?, ?, ?) ON CONFLICT(id) DO UPDATE SET data_json = excluded.data_json, updated_at = excluded.updated_at`)
    .bind(value.id, JSON.stringify(value), value.updatedAt);
}

async function readTable<T>(table: TableKey): Promise<T[]> {
  const rows = await env.DB.prepare(`SELECT data_json FROM ${tableNames[table]} ORDER BY updated_at DESC`).all<{ data_json: string }>();
  return rows.results.map((row) => JSON.parse(row.data_json) as T);
}

async function readState(): Promise<LifeData> {
  const [activities, logs, transactions, reviews, storedSettings, accounts, workoutHistory] = await Promise.all([
    readTable<Activity>("activities"), readTable<ActivityLog>("logs"), readTable<Transaction>("transactions"), readTable<DailyReview>("reviews"), readTable<LifeSettings>("settings"), readTable<FinanceAccount>("accounts"), readTable<WorkoutHistory>("workoutHistory"),
  ]);
  return { activities, logs, transactions, reviews, settings: storedSettings[0], accounts, workoutHistory };
}

export async function GET() {
  try { await ensureSchema(); return Response.json(await readState()); }
  catch (error) { return Response.json({ error: error instanceof Error ? error.message : "SQLite 初始化失败" }, { status: 500 }); }
}

export async function POST(request: Request) {
  try {
    await ensureSchema();
    const body = await request.json() as { operation?: string; table?: TableKey; value?: Entity; id?: string; patch?: Record<string, unknown>; data?: Omit<LifeData, "settings"> };
    if (body.operation === "put" && body.table && body.value) await putStatement(body.table, body.value).run();
    else if (body.operation === "patch" && body.table && body.table !== "settings" && body.id && body.patch) {
      const row = await env.DB.prepare(`SELECT data_json FROM ${tableNames[body.table]} WHERE id = ?`).bind(body.id).first<{ data_json: string }>();
      if (!row) return Response.json({ error: "项目不存在" }, { status: 404 });
      await putStatement(body.table, { ...JSON.parse(row.data_json) as Entity, ...body.patch, id: body.id } as Entity).run();
    } else if (body.operation === "delete" && body.table && body.table !== "settings" && body.id) {
      await env.DB.prepare(`DELETE FROM ${tableNames[body.table]} WHERE id = ?`).bind(body.id).run();
    } else if (body.operation === "restore" && body.data) {
      const entries = (Object.entries(tableNames) as [TableKey, string][]).filter((entry): entry is [Exclude<TableKey, "settings">, string] => entry[0] !== "settings");
      await env.DB.batch(entries.map(([, table]) => env.DB.prepare(`DELETE FROM ${table}`)));
      const statements = entries.flatMap(([key]) => (body.data![key] ?? []).map((item) => putStatement(key, item)));
      if (statements.length) await env.DB.batch(statements);
    } else return Response.json({ error: "不支持的数据操作" }, { status: 400 });
    return Response.json({ ok: true });
  } catch (error) { return Response.json({ error: error instanceof Error ? error.message : "SQLite 写入失败" }, { status: 500 }); }
}
