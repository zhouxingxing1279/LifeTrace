export type ThemePreference = "system" | "light" | "dark";
export type Priority = "low" | "normal" | "high";

export interface TaskItem {
  id: string;
  title: string;
  dueDate: string;
  project: string;
  priority: Priority;
  completed: boolean;
}

export interface HabitItem {
  id: string;
  name: string;
  targetDays: number;
  streak: number;
  completedDates: string[];
}

export interface WorkoutItem {
  id: string;
  date: string;
  title: string;
  durationMinutes: number;
  volume: number;
}

export interface FinanceTransaction {
  id: string;
  date: string;
  title: string;
  category: string;
  account: string;
  amountCents: number;
  direction: "income" | "expense";
}

export interface NoteItem {
  id: string;
  title: string;
  content: string;
  updatedAt: string;
  pinned: boolean;
}

export interface ReadingItem {
  id: string;
  title: string;
  source: string;
  progress: number;
  completed: boolean;
  highlights: string[];
  note: string;
}

export interface ReviewEntry {
  date: string;
  bestThing: string;
  problem: string;
  tomorrowPriority: string;
}

export interface LifeTraceSettings {
  theme: ThemePreference;
  reducedMotion: boolean;
  accent: "blue";
}

export interface LifeTraceState {
  tasks: TaskItem[];
  habits: HabitItem[];
  workouts: WorkoutItem[];
  transactions: FinanceTransaction[];
  notes: NoteItem[];
  reading: ReadingItem[];
  reviews: ReviewEntry[];
  settings: LifeTraceSettings;
}

export const isoDate = (date = new Date()) => date.toISOString().slice(0, 10);

export const initialState = (): LifeTraceState => ({
  tasks: [],
  habits: [],
  workouts: [],
  transactions: [],
  notes: [],
  reading: [],
  reviews: [],
  settings: { theme: "system", reducedMotion: false, accent: "blue" }
});

export const newId = (prefix: string) => `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

export function financeSummary(transactions: FinanceTransaction[]) {
  const income = transactions.filter((item) => item.direction === "income").reduce((sum, item) => sum + item.amountCents, 0);
  const expense = transactions.filter((item) => item.direction === "expense").reduce((sum, item) => sum + item.amountCents, 0);
  return { income, expense, balance: income - expense };
}

export function reviewMetrics(state: LifeTraceState, date = isoDate()) {
  const tasks = state.tasks.filter((task) => task.dueDate === date);
  const completedTasks = tasks.filter((task) => task.completed).length;
  const habits = state.habits.length;
  const completedHabits = state.habits.filter((habit) => habit.completedDates.includes(date)).length;
  return {
    taskCompletion: tasks.length ? completedTasks / tasks.length : 0,
    habitCompletion: habits ? completedHabits / habits : 0,
    completedTasks,
    totalTasks: tasks.length,
    completedHabits,
    totalHabits: habits
  };
}

export function searchState(state: LifeTraceState, query: string) {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return [];
  const hits: Array<{ id: string; type: string; title: string; path: string }> = [];
  for (const task of state.tasks) if (`${task.title} ${task.project}`.toLowerCase().includes(normalized)) hits.push({ id: task.id, type: "任务", title: task.title, path: "/app/execution" });
  for (const note of state.notes) if (`${note.title} ${note.content}`.toLowerCase().includes(normalized)) hits.push({ id: note.id, type: "笔记", title: note.title || "未命名笔记", path: "/app/notes" });
  for (const item of state.reading) if (`${item.title} ${item.source} ${item.note}`.toLowerCase().includes(normalized)) hits.push({ id: item.id, type: "阅读", title: item.title, path: "/app/english/articles" });
  for (const item of state.transactions) if (`${item.title} ${item.category} ${item.account}`.toLowerCase().includes(normalized)) hits.push({ id: item.id, type: "财务", title: item.title, path: "/app/finance" });
  return hits.slice(0, 24);
}

export const money = (cents: number, currency = "CNY") => new Intl.NumberFormat("zh-CN", { style: "currency", currency }).format(cents / 100);
