import type { Activity, ActivityCheckinMethod, ActivityColorKey, ActivityScheduleType, ActivitySyncSource, ActivityType } from "@/src/types";

export const WEEKDAYS = [
  { value: 1, short: "一", label: "周一" },
  { value: 2, short: "二", label: "周二" },
  { value: 3, short: "三", label: "周三" },
  { value: 4, short: "四", label: "周四" },
  { value: 5, short: "五", label: "周五" },
  { value: 6, short: "六", label: "周六" },
  { value: 7, short: "日", label: "周日" },
] as const;

export const PROJECT_TYPE_DEFAULTS: Record<"completion" | "duration" | "count", { target: number; unit: string }> = {
  completion: { target: 1, unit: "次" },
  duration: { target: 30, unit: "分钟" },
  count: { target: 10, unit: "次" },
};

export const PROJECT_UNITS: Record<"completion" | "duration" | "count", string[]> = {
  completion: ["次", "项"],
  duration: ["分钟", "小时"],
  count: ["次", "个", "页", "组", "杯"],
};

export const PROJECT_ICON_IDS = [
  "fitness", "running", "reading", "study", "english", "piano",
  "meditation", "sleep", "water", "target", "journal", "custom",
] as const;

export type ProjectIconId = typeof PROJECT_ICON_IDS[number];

export const PROJECT_COLOR_KEYS: ActivityColorKey[] = [
  "emerald", "blue", "cyan", "violet", "rose", "orange", "amber", "slate",
];

export interface PersistProjectDraft {
  name: string;
  type: ActivityType;
  unit: string;
  target: number | "";
  description: string;
  icon: ProjectIconId;
  color: ActivityColorKey;
  scheduleType: ActivityScheduleType;
  targetDays: number[];
  startDate: string;
  checkinMethod: ActivityCheckinMethod;
  syncSource: ActivitySyncSource | "";
}

export type ProjectField = "name" | "target" | "unit" | "targetDays" | "startDate" | "syncSource";
export type ProjectErrors = Partial<Record<ProjectField, string>>;

const localDate = () => {
  const date = new Date();
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 10);
};

const inferIcon = (activity?: Activity): ProjectIconId => {
  if (activity?.icon && PROJECT_ICON_IDS.includes(activity.icon as ProjectIconId)) return activity.icon as ProjectIconId;
  const text = `${activity?.name ?? ""} ${activity?.description ?? ""}`;
  if (/健身|训练|力量/.test(text)) return "fitness";
  if (/跑步|跑/.test(text)) return "running";
  if (/英语|英文/.test(text)) return "english";
  if (/钢琴|练琴|音乐/.test(text)) return "piano";
  if (/阅读|读书/.test(text)) return "reading";
  if (/学习|课程/.test(text)) return "study";
  if (/冥想|呼吸/.test(text)) return "meditation";
  if (/睡眠|早睡/.test(text)) return "sleep";
  if (/喝水|饮水/.test(text)) return "water";
  if (/日记|复盘|记录/.test(text)) return "journal";
  return "target";
};

const inferAutomaticSource = (activity: Activity): ActivitySyncSource | "" => {
  if (activity.syncSource) return activity.syncSource;
  if (activity.id === "system-daily-english" || /英语/.test(activity.name)) return "english";
  if (activity.id === "system-fitness-training" || /健身|训练/.test(activity.name)) return "fitness";
  return "";
};

export const createProjectDraft = (activity?: Activity): PersistProjectDraft => {
  if (!activity) {
    return {
      name: "",
      type: "completion",
      unit: "次",
      target: 1,
      description: "",
      icon: "fitness",
      color: "emerald",
      scheduleType: "daily",
      targetDays: WEEKDAYS.map((day) => day.value),
      startDate: localDate(),
      checkinMethod: "manual",
      syncSource: "",
    };
  }

  const syncSource = inferAutomaticSource(activity);
  const type = activity.type === "weekly" ? "count" : activity.type;
  const scheduleType = activity.scheduleType
    ?? (activity.targetPeriod === "daily" ? "daily" : activity.targetDays?.length ? "custom" : "weekly");

  return {
    name: activity.name,
    type,
    unit: activity.unit,
    target: activity.normalTarget ?? (type === "control" ? 1 : PROJECT_TYPE_DEFAULTS[type].target),
    description: activity.description ?? "",
    icon: inferIcon(activity),
    color: PROJECT_COLOR_KEYS.includes(activity.color ?? "emerald") ? activity.color ?? "emerald" : "emerald",
    scheduleType,
    targetDays: activity.targetDays?.length
      ? [...activity.targetDays]
      : scheduleType === "daily"
        ? WEEKDAYS.map((day) => day.value)
        : [1, 3, 5],
    startDate: activity.startDate ?? activity.createdAt.slice(0, 10),
    checkinMethod: activity.checkinMethod ?? (syncSource ? "automatic" : "manual"),
    syncSource,
  };
};

export const validateProjectDraft = (draft: PersistProjectDraft): ProjectErrors => {
  const errors: ProjectErrors = {};
  if (!draft.name.trim()) errors.name = "请输入项目名称";
  else if (draft.name.trim().length > 30) errors.name = "项目名称不能超过 30 个字符";

  if (draft.target === "" || !Number.isFinite(draft.target) || draft.target <= 0) errors.target = "目标值必须大于 0";
  if (!draft.unit.trim()) errors.unit = "请输入或选择单位";
  if (draft.unit.trim().length > 20) errors.unit = "单位不能超过 20 个字符";
  if (draft.scheduleType !== "daily" && !draft.targetDays.length) errors.targetDays = "每周至少选择一天";
  if (!draft.startDate || Number.isNaN(new Date(`${draft.startDate}T00:00:00`).getTime())) errors.startDate = "请选择有效的开始日期";
  if (draft.checkinMethod === "automatic" && !draft.syncSource) errors.syncSource = "请选择自动同步的数据来源";
  return errors;
};

export const projectDraftToActivity = (draft: PersistProjectDraft) => ({
  name: draft.name.trim(),
  type: draft.type,
  unit: draft.unit.trim(),
  normalTarget: Number(draft.target),
  targetPeriod: draft.scheduleType === "daily" ? "daily" as const : "weekly" as const,
  targetDays: draft.scheduleType === "daily" ? WEEKDAYS.map((day) => day.value) : [...draft.targetDays].sort((a, b) => a - b),
  icon: draft.icon,
  color: draft.color,
  description: draft.description.trim() || undefined,
  scheduleType: draft.scheduleType,
  startDate: draft.startDate,
  checkinMethod: draft.checkinMethod,
  syncSource: draft.checkinMethod === "automatic" ? draft.syncSource || undefined : undefined,
});

