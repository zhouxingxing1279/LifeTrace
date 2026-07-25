import { env } from "cloudflare:workers";
import type {
  Activity,
  ActivityLog,
  TrainingNote,
  WorkoutHistory,
  WorkoutImportRecord,
  XunjiWorkout,
} from "@/src/types";

const USER_ID = "local-user";
const IMPORT_TABLE = "workout_import_records";
const HISTORY_TABLE = "workout_history";
const NOTES_TABLE = "training_notes";

const put = (table: string, value: { id: string; updatedAt: string }) =>
  env.DB.prepare(
    `INSERT INTO ${table} (id,data_json,updated_at) VALUES (?,?,?)
     ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json,updated_at=excluded.updated_at`,
  ).bind(value.id, JSON.stringify(value), value.updatedAt);

export async function ensureXunjiSchema() {
  await env.DB.batch([
    env.DB.prepare(`CREATE TABLE IF NOT EXISTS ${IMPORT_TABLE} (id TEXT PRIMARY KEY,data_json TEXT NOT NULL,updated_at TEXT NOT NULL)`),
    env.DB.prepare(`CREATE TABLE IF NOT EXISTS ${HISTORY_TABLE} (id TEXT PRIMARY KEY,data_json TEXT NOT NULL,updated_at TEXT NOT NULL)`),
    env.DB.prepare(`CREATE TABLE IF NOT EXISTS ${NOTES_TABLE} (id TEXT PRIMARY KEY,data_json TEXT NOT NULL,updated_at TEXT NOT NULL)`),
    env.DB.prepare("CREATE TABLE IF NOT EXISTS activities (id TEXT PRIMARY KEY,data_json TEXT NOT NULL,updated_at TEXT NOT NULL)"),
    env.DB.prepare("CREATE TABLE IF NOT EXISTS activity_logs (id TEXT PRIMARY KEY,data_json TEXT NOT NULL,updated_at TEXT NOT NULL)"),
  ]);
}

const readTable = async <T>(table: string): Promise<T[]> => {
  await ensureXunjiSchema();
  const rows = await env.DB.prepare(`SELECT data_json FROM ${table} ORDER BY updated_at DESC`).all<{ data_json: string }>();
  return rows.results.map((row) => JSON.parse(row.data_json) as T);
};

export async function createWorkoutImport(input: {
  shareUrl: string;
  rawData: unknown;
  workout: XunjiWorkout;
}) {
  const stamp = new Date().toISOString();
  const record: WorkoutImportRecord = {
    id: crypto.randomUUID(),
    userId: USER_ID,
    source: "xunji",
    shareUrl: input.shareUrl,
    rawData: input.rawData,
    workout: input.workout,
    status: "pending",
    createdAt: stamp,
    updatedAt: stamp,
  };
  await ensureXunjiSchema();
  await put(IMPORT_TABLE, record).run();
  return record;
}

export async function createFailedWorkoutImport(error: string, rawData: unknown, shareUrl?: string) {
  const stamp = new Date().toISOString();
  const record: WorkoutImportRecord = {
    id: crypto.randomUUID(),
    userId: USER_ID,
    source: "xunji",
    shareUrl,
    rawData,
    status: "failed",
    error,
    createdAt: stamp,
    updatedAt: stamp,
  };
  await ensureXunjiSchema();
  await put(IMPORT_TABLE, record).run();
  return record;
}

export async function listWorkoutImports() {
  return readTable<WorkoutImportRecord>(IMPORT_TABLE);
}

const shareSourceId = (record: WorkoutImportRecord) => {
  try {
    const shareUrl = new URL(record.shareUrl ?? "");
    return shareUrl.searchParams.get("localid")
      ?? shareUrl.searchParams.get("spid")
      ?? shareUrl.pathname.split("/").filter(Boolean).at(-1)
      ?? record.id;
  } catch {
    return record.id;
  }
};

const workoutNoteContent = (workout: XunjiWorkout) => [
  `训练部位：${workout.title}`,
  `训练时长：${workout.durationMinutes} 分钟`,
  `训练容量：${workout.volumeKg} kg`,
  `动作：${workout.exercises.map((exercise) => exercise.name).join("、")}`,
  "来源：训记同步",
].join("\n");

export async function confirmWorkoutImport(importId: string, editedWorkout?: XunjiWorkout) {
  const imports = await readTable<WorkoutImportRecord>(IMPORT_TABLE);
  const record = imports.find((item) => item.id === importId);
  if (!record) throw new Error("导入记录不存在");
  if (record.status === "failed") throw new Error("失败的解析记录不能导入");
  const workout = editedWorkout ?? record.workout;
  if (!workout?.exercises.length) throw new Error("训练数据缺少动作");

  const sourceId = shareSourceId(record);
  const historyRows = await readTable<WorkoutHistory>(HISTORY_TABLE);
  const duplicate = historyRows.find((item) => item.source === "xunji" && item.sourceId === sourceId);
  if (duplicate) {
    const completedImport: WorkoutImportRecord = {
      ...record,
      workout,
      status: "success",
      workoutRecordId: duplicate.id,
      updatedAt: new Date().toISOString(),
    };
    await put(IMPORT_TABLE, completedImport).run();
    return { workoutRecord: duplicate, importRecord: completedImport, duplicate: true };
  }

  const stamp = new Date().toISOString();
  const occurredAt = `${workout.date}T12:00:00+08:00`;
  const exercises = workout.exercises.map((exercise) => ({
    name: exercise.name,
    plannedSets: exercise.sets.length,
    completedSets: exercise.sets.length,
    sets: exercise.sets.map((set) => ({ weight: set.weightKg, reps: set.reps, completed: true })),
  }));
  const setCount = exercises.reduce((sum, exercise) => sum + exercise.completedSets, 0);
  const history: WorkoutHistory = {
    id: `xunji-${sourceId}`,
    userId: USER_ID,
    templateId: "",
    name: workout.title,
    occurredAt,
    durationSeconds: workout.durationMinutes * 60,
    exerciseCount: exercises.length,
    setCount,
    plannedSetCount: setCount,
    status: "completed",
    source: "xunji",
    sourceId,
    caloriesKcal: workout.caloriesKcal,
    volumeKg: workout.volumeKg,
    exercises,
    createdAt: stamp,
    updatedAt: stamp,
  };

  const activities = await readTable<Activity>("activities");
  const configuredFitness = activities.filter((item) => !item.isArchived && item.checkinMethod === "automatic" && item.syncSource === "fitness");
  const legacyFitness = activities.find((item) => !item.isArchived && (item.name.includes("健身") || item.name.includes("训练")));
  const fallbackFitness: Activity = {
    id: "system-fitness-training",
    userId: USER_ID,
    name: "健身训练",
    type: "count",
    unit: "次",
    normalTarget: 4,
    targetPeriod: "weekly",
    targetDays: [1, 3, 5],
    scheduleType: "weekly",
    startDate: occurredAt.slice(0, 10),
    checkinMethod: "automatic",
    syncSource: "fitness",
    icon: "fitness",
    color: "emerald",
    description: "由训练记录自动完成打卡。",
    isArchived: false,
    createdAt: stamp,
    updatedAt: stamp,
  };
  const fitnessActivities = configuredFitness.length ? configuredFitness : [legacyFitness ?? fallbackFitness];
  const activityLogs: ActivityLog[] = fitnessActivities.map((fitness) => ({
    id: `workout-log-${history.id}-${fitness.id}`,
    userId: USER_ID,
    activityId: fitness.id,
    value: 1,
    status: "completed",
    note: `完成「${workout.title}」· ${workout.durationMinutes} 分钟 · ${setCount} 组 · 训记同步`,
    createdAt: occurredAt,
    updatedAt: stamp,
  }));
  const note: TrainingNote = {
    id: `training-note-${history.id}`,
    userId: USER_ID,
    title: `${workout.date}训练记录`,
    content: workoutNoteContent(workout),
    workoutRecordId: history.id,
    source: "xunji",
    noteDate: workout.date,
    createdAt: occurredAt,
    updatedAt: stamp,
  };
  const completedImport: WorkoutImportRecord = {
    ...record,
    workout,
    status: "success",
    workoutRecordId: history.id,
    updatedAt: stamp,
  };

  await env.DB.batch([
    put(HISTORY_TABLE, history),
    ...(!legacyFitness && !configuredFitness.length ? [put("activities", fallbackFitness)] : []),
    ...activityLogs.map((log) => put("activity_logs", log)),
    put(NOTES_TABLE, note),
    put(IMPORT_TABLE, completedImport),
  ]);
  return { workoutRecord: history, importRecord: completedImport, duplicate: false };
}

export async function cancelWorkoutImport(importId: string) {
  const imports = await readTable<WorkoutImportRecord>(IMPORT_TABLE);
  const record = imports.find((item) => item.id === importId);
  if (!record) throw new Error("导入记录不存在");
  const updated: WorkoutImportRecord = {
    ...record,
    status: "failed",
    error: "用户取消导入",
    updatedAt: new Date().toISOString(),
  };
  await put(IMPORT_TABLE, updated).run();
  return updated;
}
