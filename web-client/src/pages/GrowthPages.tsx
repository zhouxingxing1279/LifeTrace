import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import {
  AssistantApi, ENTITY_TYPES, createDailyReview, createHabitActivity, createHabitLog,
  createPreference, createTrainingNote, createWorkout, formatMoney, localDate,
  type CloudState, type EntityType, type JsonEntity,
} from "../core";
import { Empty, Metric, Notice, PageStack, Panel, entities, number, text, type CloudPageProps } from "../ui";

const day = (value: Date | string = new Date()) => localDate(typeof value === "string" ? new Date(value) : value);
const displayDate = (value: string) => new Date(`${value}T12:00:00`).toLocaleDateString("zh-CN", { month: "long", day: "numeric", weekday: "short" });
const sorted = (items: JsonEntity[]) => [...items].sort((left, right) => right.meta.updatedAt.localeCompare(left.meta.updatedAt));

export function HabitsPage({ session, state, run, online }: CloudPageProps) {
  const activities = sorted(entities(state, "habit.activity")).filter((item) => item.isArchived !== true);
  const logs = entities(state, "habit.log");
  const [name, setName] = useState("");
  const [unit, setUnit] = useState("次");
  const [target, setTarget] = useState("1");
  const [note, setNote] = useState<Record<string, string>>({});
  const today = localDate();

  async function create(event: FormEvent) {
    event.preventDefault();
    const value = createHabitActivity(session.user.id, session.session.deviceId, {
      name, unit, normalTarget: Number(target) || 1,
    });
    await run((store) => store.upsert("habit.activity", value));
    setName("");
  }

  async function checkIn(activity: JsonEntity) {
    const value = createHabitLog(session.user.id, session.session.deviceId, activity.meta.id, 1, note[activity.meta.id] ?? "");
    await run((store) => store.upsert("habit.log", value));
    setNote((current) => ({ ...current, [activity.meta.id]: "" }));
  }

  async function archive(activity: JsonEntity) {
    await run((store) => store.upsert("habit.activity", { ...activity, isArchived: true }));
  }

  const todayLogs = logs.filter((item) => text(item, "logDate") === today && item.status !== "skipped");
  const completed = new Set(todayLogs.map((item) => text(item, "activityId"))).size;
  const weekStart = new Date(); weekStart.setDate(weekStart.getDate() - 6);
  const weekLogs = logs.filter((item) => new Date(`${text(item, "logDate")}T12:00:00`) >= weekStart);

  return <PageStack>
    <div className="hx-metrics">
      <Metric label="今日完成" value={`${completed} / ${activities.length}`} detail="已完成项目" positive />
      <Metric label="近 7 天记录" value={String(weekLogs.length)} detail="所有坚持记录" />
      <Metric label="累计记录" value={String(logs.length)} detail="云端已保存" />
    </div>
    <div className="hx-content-grid two">
      <Panel eyebrow="TODAY" title="今天的坚持">
        <div className="hx-list">
          {activities.map((activity) => {
            const own = todayLogs.filter((item) => text(item, "activityId") === activity.meta.id);
            const value = own.reduce((sum, item) => sum + number(item, "value"), 0);
            return <article className="hx-row" key={activity.meta.id}>
              <span className="hx-row-icon" style={{ background: text(activity, "color") || undefined }}>{text(activity, "icon") || text(activity, "name").slice(0, 1)}</span>
              <div className="hx-row-main"><strong>{text(activity, "name")}</strong><small>{value > 0 ? `今天已记录 ${value} ${text(activity, "unit")}` : `目标 ${number(activity, "normalTarget") || 1} ${text(activity, "unit")}`}</small><input value={note[activity.meta.id] ?? ""} onChange={(event) => setNote((current) => ({ ...current, [activity.meta.id]: event.target.value }))} placeholder="可选：记录感受" /></div>
              <div className="hx-row-actions"><button className="hx-btn primary" disabled={!online} onClick={() => void checkIn(activity)}>{value > 0 ? "继续记录" : "完成"}</button><button className="hx-btn ghost" onClick={() => void archive(activity)}>归档</button></div>
            </article>;
          })}
          {!activities.length && <Empty title="还没有坚持项目" description="在右侧创建练琴、阅读、英语或任何长期项目。" />}
        </div>
      </Panel>
      <Panel eyebrow="NEW PROJECT" title="创建坚持项目">
        <form className="hx-form" onSubmit={(event) => void create(event)}>
          <label>项目名称<input required value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：练钢琴" /></label>
          <div className="hx-form-grid"><label>单位<input value={unit} onChange={(event) => setUnit(event.target.value)} /></label><label>每日目标<input type="number" min="0.1" step="0.1" value={target} onChange={(event) => setTarget(event.target.value)} /></label></div>
          <button className="hx-btn primary" disabled={!online}>创建项目</button>
        </form>
      </Panel>
    </div>
  </PageStack>;
}

interface ImportedWorkout { name?: string; occurredAt?: string; durationMinutes?: number; exerciseCount?: number; setCount?: number; volumeKg?: number; caloriesKcal?: number; }

function parseWorkoutFile(raw: string, filename: string): ImportedWorkout[] {
  if (filename.toLowerCase().endsWith(".json")) {
    const parsed = JSON.parse(raw) as unknown;
    const values = Array.isArray(parsed) ? parsed : typeof parsed === "object" && parsed && Array.isArray((parsed as { workouts?: unknown }).workouts) ? (parsed as { workouts: unknown[] }).workouts : [];
    return values.filter((item): item is ImportedWorkout => typeof item === "object" && item !== null);
  }
  const lines = raw.split(/\r?\n/).filter(Boolean);
  if (lines.length < 2) return [];
  const headers = lines[0]!.split(",").map((item) => item.trim().replace(/^"|"$/g, ""));
  return lines.slice(1).map((line) => {
    const cells = line.split(",").map((item) => item.trim().replace(/^"|"$/g, ""));
    const value = Object.fromEntries(headers.map((header, index) => [header, cells[index] ?? ""]));
    return {
      name: value.name || value.训练名称 || value.名称,
      occurredAt: value.occurredAt || value.训练时间 || value.日期,
      durationMinutes: Number(value.durationMinutes || value.时长 || 0),
      exerciseCount: Number(value.exerciseCount || value.动作数 || 0),
      setCount: Number(value.setCount || value.组数 || 0),
      volumeKg: Number(value.volumeKg || value.容量 || 0) || undefined,
      caloriesKcal: Number(value.caloriesKcal || value.热量 || 0) || undefined,
    };
  });
}

export function FitnessPage({ session, state, run, online }: CloudPageProps) {
  const workouts = sorted(entities(state, "workout.workout"));
  const exercises = entities(state, "workout.exercise");
  const sets = entities(state, "workout.set");
  const trainingNotes = sorted(entities(state, "workout.training_note"));
  const [name, setName] = useState("");
  const [occurredAt, setOccurredAt] = useState(() => new Date().toISOString().slice(0, 16));
  const [duration, setDuration] = useState("60");
  const [exerciseCount, setExerciseCount] = useState("5");
  const [setCount, setSetCount] = useState("20");
  const [volume, setVolume] = useState("");
  const [noteTitle, setNoteTitle] = useState("");
  const [noteContent, setNoteContent] = useState("");
  const [message, setMessage] = useState("");
  const fileRef = useRef<HTMLInputElement>(null);

  async function create(event: FormEvent) {
    event.preventDefault();
    const workout = createWorkout(session.user.id, session.session.deviceId, {
      name, occurredAt: new Date(occurredAt).toISOString(), durationMinutes: Number(duration),
      exerciseCount: Number(exerciseCount), setCount: Number(setCount), volumeKg: volume ? Number(volume) : null,
    });
    await run((store) => store.upsert("workout.workout", workout));
    setName(""); setMessage("训练记录已保存到云端");
  }

  async function importFile(file?: File) {
    if (!file) return;
    try {
      const records = parseWorkoutFile(await file.text(), file.name);
      if (!records.length) throw new Error("文件中没有可识别的训练记录");
      const values = records.map((record) => createWorkout(session.user.id, session.session.deviceId, record));
      const result = await run(async (store) => (await store.batchUpsert("workout.workout", values)).state);
      setMessage(`已导入 ${Object.keys(result.entities["workout.workout"] ?? {}).length} 条训练记录`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "训练文件解析失败");
    } finally {
      if (fileRef.current) fileRef.current.value = "";
    }
  }

  async function saveNote(event: FormEvent) {
    event.preventDefault();
    const value = createTrainingNote(session.user.id, session.session.deviceId, noteTitle, noteContent);
    await run((store) => store.upsert("workout.training_note", value));
    setNoteTitle(""); setNoteContent("");
  }

  async function remove(workout: JsonEntity) {
    const ownExercises = exercises.filter((item) => text(item, "workoutId") === workout.meta.id);
    const ownExerciseIds = new Set(ownExercises.map((item) => item.meta.id));
    const ownSets = sets.filter((item) => ownExerciseIds.has(text(item, "exerciseId")));
    await run(async (store) => {
      for (const item of ownSets) await store.delete("workout.set", item.meta.id);
      for (const item of ownExercises) await store.delete("workout.exercise", item.meta.id);
      return store.delete("workout.workout", workout.meta.id);
    });
  }

  const week = workouts.filter((item) => Date.now() - new Date(text(item, "occurredAt")).getTime() < 7 * 86400000);
  const monthVolume = workouts.filter((item) => text(item, "localDate").startsWith(localDate().slice(0, 7))).reduce((sum, item) => sum + number(item, "volumeKg"), 0);

  return <PageStack>
    <div className="hx-metrics"><Metric label="近 7 天训练" value={`${week.length} 次`} detail="云端训练记录" positive /><Metric label="累计训练" value={`${workouts.length} 次`} detail={`${sets.length} 个训练组`} /><Metric label="本月容量" value={`${Math.round(monthVolume).toLocaleString()} kg`} detail="已记录训练容量" /></div>
    <div className="hx-content-grid two">
      <Panel eyebrow="WORKOUTS" title="训练历史" actions={<><input ref={fileRef} hidden type="file" accept=".json,.csv,application/json,text/csv" onChange={(event) => void importFile(event.target.files?.[0])} /><button className="hx-btn secondary" onClick={() => fileRef.current?.click()}>导入 JSON / CSV</button></>}>
        {message && <Notice kind="neutral">{message}</Notice>}
        <div className="hx-list">{workouts.map((workout) => <article className="hx-row" key={workout.meta.id}><span className="hx-row-icon">训</span><div className="hx-row-main"><strong>{text(workout, "name")}</strong><small>{new Date(text(workout, "occurredAt")).toLocaleString("zh-CN")} · {Math.round(number(workout, "durationSeconds") / 60)} 分钟 · {number(workout, "exerciseCount")} 个动作 · {number(workout, "setCount")} 组</small></div><button className="hx-btn ghost danger" onClick={() => void remove(workout)}>删除</button></article>)}{!workouts.length && <Empty title="还没有训练记录" description="手动新增，或导入 JSON/CSV 训练文件。" />}</div>
      </Panel>
      <Panel eyebrow="NEW WORKOUT" title="新增训练记录">
        <form className="hx-form" onSubmit={(event) => void create(event)}><label>训练名称<input required value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：胸肩训练" /></label><label>训练时间<input type="datetime-local" value={occurredAt} onChange={(event) => setOccurredAt(event.target.value)} /></label><div className="hx-form-grid"><label>时长（分钟）<input type="number" min="0" value={duration} onChange={(event) => setDuration(event.target.value)} /></label><label>动作数<input type="number" min="0" value={exerciseCount} onChange={(event) => setExerciseCount(event.target.value)} /></label><label>组数<input type="number" min="0" value={setCount} onChange={(event) => setSetCount(event.target.value)} /></label><label>训练容量（kg）<input type="number" min="0" step="0.1" value={volume} onChange={(event) => setVolume(event.target.value)} /></label></div><button className="hx-btn primary" disabled={!online}>保存训练</button></form>
      </Panel>
    </div>
    <div className="hx-content-grid two">
      <Panel eyebrow="TRAINING NOTES" title="训练笔记"><div className="hx-list">{trainingNotes.slice(0, 20).map((item) => <article className="hx-row" key={item.meta.id}><span className="hx-row-icon">记</span><div><strong>{text(item, "title")}</strong><small>{text(item, "noteDate")} · {text(item, "content").slice(0, 120)}</small></div></article>)}{!trainingNotes.length && <Empty title="暂无训练笔记" description="记录动作感受、疼痛情况和下次调整。" />}</div></Panel>
      <Panel eyebrow="NEW NOTE" title="写训练笔记"><form className="hx-form" onSubmit={(event) => void saveNote(event)}><label>标题<input required value={noteTitle} onChange={(event) => setNoteTitle(event.target.value)} /></label><label>内容<textarea required rows={5} value={noteContent} onChange={(event) => setNoteContent(event.target.value)} /></label><button className="hx-btn primary" disabled={!online}>保存笔记</button></form></Panel>
    </div>
  </PageStack>;
}

interface CalendarEvent { id: string; date: string; type: string; title: string; detail: string; }

function calendarEvents(state: CloudState): CalendarEvent[] {
  const events: CalendarEvent[] = [];
  for (const item of entities(state, "habit.log")) events.push({ id: item.meta.id, date: text(item, "logDate"), type: "坚持", title: "完成坚持项目", detail: `${number(item, "value") || 1} 次` });
  for (const item of entities(state, "finance.transaction")) events.push({ id: item.meta.id, date: text(item, "localDate"), type: "财务", title: text(item, "merchant") || text(item, "counterparty") || "账单", detail: formatMoney(number(item, "amountCents")) });
  for (const item of entities(state, "workout.workout")) events.push({ id: item.meta.id, date: text(item, "localDate"), type: "训练", title: text(item, "name"), detail: `${Math.round(number(item, "durationSeconds") / 60)} 分钟` });
  for (const item of entities(state, "english.learning_record")) events.push({ id: item.meta.id, date: text(item, "recordDate"), type: "英语", title: "完成英语阅读", detail: text(item, "summary").slice(0, 80) });
  for (const item of entities(state, "review.daily")) events.push({ id: item.meta.id, date: text(item, "reviewDate"), type: "复盘", title: "每日复盘", detail: text(item, "tomorrowPriority") || text(item, "bestThing") });
  return events;
}

export function CalendarPage({ state }: CloudPageProps) {
  const events = useMemo(() => calendarEvents(state), [state]);
  const [month, setMonth] = useState(() => localDate().slice(0, 7));
  const [selected, setSelected] = useState(() => localDate());
  const [year, monthNumber] = month.split("-").map(Number);
  const first = (new Date(year!, monthNumber! - 1, 1).getDay() + 6) % 7;
  const count = new Date(year!, monthNumber!, 0).getDate();
  const selectedEvents = events.filter((item) => item.date === selected);
  const move = (offset: number) => {
    const value = new Date(year!, monthNumber! - 1 + offset, 1);
    const next = `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, "0")}`;
    setMonth(next); setSelected(`${next}-01`);
  };

  return <PageStack><div className="hx-calendar-layout"><Panel eyebrow="CALENDAR" title={`${year} 年 ${monthNumber} 月`} actions={<div className="hx-inline-actions"><button className="hx-btn ghost" onClick={() => move(-1)}>上个月</button><button className="hx-btn ghost" onClick={() => move(1)}>下个月</button></div>}><div className="hx-week">{"一二三四五六日".split("").map((item) => <span key={item}>周{item}</span>)}</div><div className="hx-days">{Array.from({ length: first }).map((_, index) => <i key={`empty-${index}`} />)}{Array.from({ length: count }, (_, index) => index + 1).map((date) => { const key = `${month}-${String(date).padStart(2, "0")}`; const own = events.filter((item) => item.date === key); return <button key={key} className={selected === key ? "selected" : ""} onClick={() => setSelected(key)}><b>{date}</b><span>{own.slice(0, 4).map((item) => <i key={item.id} title={item.type} />)}</span><small>{own.length || ""}</small></button>; })}</div></Panel><Panel eyebrow="DAY" title={displayDate(selected)}><div className="hx-list">{selectedEvents.map((item) => <article className="hx-row" key={`${item.type}-${item.id}`}><span className="hx-row-icon">{item.type.slice(0, 1)}</span><div><strong>{item.title}</strong><small>{item.type} · {item.detail}</small></div></article>)}{!selectedEvents.length && <Empty title="这一天还没有记录" description="坚持、训练、英语、账单和复盘会自动汇总到这里。" />}</div></Panel></div></PageStack>;
}

export function ReviewPage({ session, state, run, online }: CloudPageProps) {
  const reviews = entities(state, "review.daily");
  const [reviewDate, setReviewDate] = useState(localDate());
  const existing = reviews.find((item) => text(item, "reviewDate") === reviewDate);
  const [energy, setEnergy] = useState("3");
  const [mood, setMood] = useState("3");
  const [bestThing, setBestThing] = useState("");
  const [problem, setProblem] = useState("");
  const [tomorrow, setTomorrow] = useState("");
  const [note, setNote] = useState("");

  useEffect(() => {
    setEnergy(String(number(existing ?? ({ meta: {} } as JsonEntity), "energy") || 3));
    setMood(String(number(existing ?? ({ meta: {} } as JsonEntity), "mood") || 3));
    setBestThing(existing ? text(existing, "bestThing") : "");
    setProblem(existing ? text(existing, "problem") : "");
    setTomorrow(existing ? text(existing, "tomorrowPriority") : "");
    setNote(existing ? text(existing, "note") : "");
  }, [existing?.meta.id, reviewDate]);

  async function save(event: FormEvent) {
    event.preventDefault();
    const value = createDailyReview(session.user.id, session.session.deviceId, { reviewDate, energy: Number(energy), mood: Number(mood), bestThing, problem, tomorrowPriority: tomorrow, note }, existing?.meta.id);
    if (existing) value.meta = { ...existing.meta, updatedAt: new Date().toISOString() };
    await run((store) => store.upsert("review.daily", value));
  }

  return <PageStack><div className="hx-content-grid two"><Panel eyebrow="DAILY REVIEW" title={displayDate(reviewDate)}><form className="hx-form" onSubmit={(event) => void save(event)}><label>复盘日期<input type="date" value={reviewDate} onChange={(event) => setReviewDate(event.target.value)} /></label><div className="hx-form-grid"><label>精力（1-5）<input type="range" min="1" max="5" value={energy} onChange={(event) => setEnergy(event.target.value)} /><strong>{energy}</strong></label><label>心情（1-5）<input type="range" min="1" max="5" value={mood} onChange={(event) => setMood(event.target.value)} /><strong>{mood}</strong></label></div><label>今天最好的一件事<textarea rows={3} value={bestThing} onChange={(event) => setBestThing(event.target.value)} /></label><label>遇到的问题<textarea rows={3} value={problem} onChange={(event) => setProblem(event.target.value)} /></label><label>明天最重要的事<input value={tomorrow} onChange={(event) => setTomorrow(event.target.value)} /></label><label>补充记录<textarea rows={4} value={note} onChange={(event) => setNote(event.target.value)} /></label><button className="hx-btn primary" disabled={!online}>{existing ? "更新复盘" : "保存复盘"}</button></form></Panel><Panel eyebrow="HISTORY" title="最近复盘"><div className="hx-list">{sorted(reviews).slice(0, 20).map((item) => <article className="hx-row" key={item.meta.id}><span className="hx-row-icon">复</span><div><strong>{text(item, "reviewDate")}</strong><small>精力 {number(item, "energy") || "-"} · 心情 {number(item, "mood") || "-"} · {text(item, "tomorrowPriority") || text(item, "bestThing") || "已完成复盘"}</small></div><button className="hx-btn ghost" onClick={() => setReviewDate(text(item, "reviewDate"))}>查看</button></article>)}{!reviews.length && <Empty title="还没有复盘" description="完成今天的第一次两分钟复盘。" />}</div></Panel></div></PageStack>;
}

export function AssistantPage({ session, state, online }: CloudPageProps) {
  const api = useMemo(() => new AssistantApi(), []);
  const [prompt, setPrompt] = useState("");
  const [messages, setMessages] = useState<Array<{ role: "user" | "assistant"; content: string; provider?: string }>>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const suggestions = ["总结我最近七天的状态", "分析我的消费结构", "我最近的坚持情况怎么样", "根据训练和复盘给出下周建议"];

  async function ask(value = prompt) {
    const question = value.trim();
    if (!question || busy) return;
    setMessages((current) => [...current, { role: "user", content: question }]); setPrompt(""); setBusy(true); setError("");
    try {
      const result = await api.ask(question, state, session.csrfToken);
      setMessages((current) => [...current, { role: "assistant", content: result.reply, provider: result.provider }]);
    } catch (cause) { setError(cause instanceof Error ? cause.message : "AI 服务不可用"); }
    finally { setBusy(false); }
  }

  return <PageStack><div className="hx-assistant-layout"><Panel eyebrow="PERSONAL CONTEXT" title="你的云端数据概况"><div className="hx-metrics compact"><Metric label="坚持" value={String(entities(state, "habit.log").length)} detail="条记录" /><Metric label="训练" value={String(entities(state, "workout.workout").length)} detail="次训练" /><Metric label="账单" value={String(entities(state, "finance.transaction").length)} detail="笔流水" /><Metric label="笔记" value={String(entities(state, "note.note").length)} detail="篇笔记" /></div><p className="hx-muted">请求只发送精简后的近期云端记录。DeepSeek 密钥保存在服务器环境变量中，不进入浏览器。</p></Panel><Panel eyebrow="AI ASSISTANT" title="LifeTrace AI 管家"><div className="hx-chat">{messages.map((message, index) => <article key={`${message.role}-${index}`} className={message.role}><span>{message.role === "user" ? "你" : "AI"}</span><div><p>{message.content}</p>{message.provider && <small>{message.provider === "deepseek" ? "DeepSeek" : "本地分析"}</small>}</div></article>)}{!messages.length && <Empty title="从一个问题开始" description="AI 会基于你已经同步的个人记录回答。" />}</div>{error && <Notice kind="error">{error}</Notice>}<div className="hx-suggestions">{suggestions.map((item) => <button key={item} onClick={() => void ask(item)}>{item}</button>)}</div><form className="hx-chat-input" onSubmit={(event) => { event.preventDefault(); void ask(); }}><textarea rows={3} value={prompt} onChange={(event) => setPrompt(event.target.value)} placeholder="询问你的习惯、训练、财务、英语或复盘…" /><button className="hx-btn primary" disabled={!online || busy}>{busy ? "分析中…" : "发送"}</button></form></Panel></div></PageStack>;
}

const IMPORTABLE_TYPES = ENTITY_TYPES.filter((value) => value !== "english.article") as EntityType[];

export function SettingsPage({ session, state, run, online }: CloudPageProps) {
  const preferences = entities(state, "user.preference");
  const themePreference = preferences.find((item) => text(item, "preferenceKey") === "appearance.theme");
  const themeValue = themePreference?.value === "dark" ? "dark" : "light";
  const [message, setMessage] = useState("");
  const importRef = useRef<HTMLInputElement>(null);

  async function setTheme(theme: "light" | "dark") {
    const value = themePreference
      ? { ...themePreference, value: theme }
      : createPreference(session.user.id, session.session.deviceId, "appearance.theme", theme);
    await run((store) => store.upsert("user.preference", value));
    document.documentElement.dataset.theme = theme;
  }

  function exportBackup() {
    const blob = new Blob([JSON.stringify({ version: 1, exportedAt: new Date().toISOString(), entities: state.entities }, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob); const anchor = document.createElement("a");
    anchor.href = url; anchor.download = `lifetrace-cloud-backup-${localDate()}.json`; anchor.click(); URL.revokeObjectURL(url);
  }

  async function importBackup(file?: File) {
    if (!file) return;
    try {
      const payload = JSON.parse(await file.text()) as { entities?: Record<string, Record<string, JsonEntity>> };
      if (!payload.entities || typeof payload.entities !== "object") throw new Error("备份文件缺少 entities");
      let saved = 0;
      await run(async (store) => {
        let next = store.snapshot();
        for (const entityType of IMPORTABLE_TYPES) {
          const values = Object.values(payload.entities?.[entityType] ?? {});
          if (!values.length) continue;
          const result = await store.batchUpsert(entityType, values); next = result.state; saved += result.saved;
        }
        return next;
      });
      setMessage(`备份导入完成，共保存 ${saved} 条记录`);
    } catch (cause) { setMessage(cause instanceof Error ? cause.message : "备份导入失败"); }
    finally { if (importRef.current) importRef.current.value = ""; }
  }

  const total = Object.values(state.entities).reduce((sum, collection) => sum + Object.keys(collection ?? {}).length, 0);
  return <PageStack><div className="hx-content-grid two"><Panel eyebrow="APPEARANCE" title="界面外观"><div className="hx-setting-row"><div><strong>颜色模式</strong><small>浏览器端与桌面应用使用同一套视觉层级。</small></div><div className="hx-inline-actions"><button className={`hx-btn ${themeValue === "light" ? "primary" : "secondary"}`} onClick={() => void setTheme("light")}>浅色</button><button className={`hx-btn ${themeValue === "dark" ? "primary" : "secondary"}`} onClick={() => void setTheme("dark")}>深色</button></div></div></Panel><Panel eyebrow="CLOUD DATA" title="云端数据"><div className="hx-metrics compact"><Metric label="实体记录" value={String(total)} detail="当前账户" /><Metric label="同步游标" value={state.cursor ?? "-"} detail="服务器位置" /><Metric label="冲突" value={String(state.conflicts.length)} detail="待检查" /></div><div className="hx-inline-actions"><button className="hx-btn secondary" onClick={exportBackup}>导出 JSON 备份</button><input hidden ref={importRef} type="file" accept="application/json,.json" onChange={(event) => void importBackup(event.target.files?.[0])} /><button className="hx-btn secondary" disabled={!online} onClick={() => importRef.current?.click()}>导入云端备份</button></div>{message && <Notice kind="neutral">{message}</Notice>}</Panel><Panel eyebrow="SECURITY" title="账号与安全"><div className="hx-setting-row"><div><strong>{session.user.displayName || "LifeTrace 用户"}</strong><small>{session.user.email}</small></div><span className="hx-status success">已登录</span></div><p className="hx-muted">设备、活动会话、退出登录和撤销操作在“设备与会话”页面管理。</p></Panel><Panel eyebrow="BOUNDARY" title="浏览器功能边界"><Notice kind="neutral">浏览器端包含除相册外的云端功能。照片同步、私密相册、局域网上传、证书和本地密钥仍只属于桌面应用，不会进入浏览器包或云端备份。</Notice></Panel></div></PageStack>;
}
