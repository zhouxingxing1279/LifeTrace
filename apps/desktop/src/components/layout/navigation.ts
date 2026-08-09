import {
  BarChart3,
  BookOpen,
  Bot,
  CalendarDays,
  Check,
  CircleDollarSign,
  Dumbbell,
  FileUp,
  Home,
  Images,
  Languages,
  ListChecks,
  Mail,
  NotebookPen,
  WalletCards,
} from "lucide-react";
import type { NavGroup } from "./AppShell";

export type PlatformView =
  | "dashboard"
  | "execution"
  | "assistant"
  | "mail"
  | "habits"
  | "english"
  | "fitness"
  | "photos"
  | "finance"
  | "transactions"
  | "accounts"
  | "import"
  | "calendar"
  | "review"
  | "notes"
  | "settings"
  | "gallery";

export const navGroups: NavGroup[] = [
  {
    label: "今天",
    items: [
      { id: "dashboard", label: "今天", icon: Home },
      { id: "execution", label: "执行", icon: ListChecks },
    ],
  },
  {
    label: "生活",
    items: [
      { id: "habits", label: "坚持", icon: Check },
      { id: "fitness", label: "健身", icon: Dumbbell },
      { id: "english", label: "英语", icon: Languages },
    ],
  },
  {
    label: "记录",
    items: [
      { id: "notes", label: "笔记", icon: NotebookPen },
      { id: "photos", label: "照片", icon: Images },
      { id: "mail", label: "邮件", icon: Mail },
    ],
  },
  {
    label: "财务",
    items: [
      { id: "finance", label: "概览", icon: BarChart3 },
      { id: "transactions", label: "账单", icon: CircleDollarSign },
      { id: "accounts", label: "账户", icon: WalletCards },
      { id: "import", label: "导入", icon: FileUp },
    ],
  },
  {
    label: "回顾",
    items: [
      { id: "calendar", label: "日历", icon: CalendarDays },
      { id: "review", label: "复盘", icon: BookOpen },
    ],
  },
  {
    label: "助手",
    items: [{ id: "assistant", label: "AI 管家", icon: Bot }],
  },
];

export const pageTitles: Record<PlatformView, string> = {
  dashboard: "今天",
  execution: "执行中心",
  assistant: "AI 管家",
  mail: "邮件行动中心",
  habits: "坚持",
  english: "每日英语",
  fitness: "健身训练",
  photos: "照片",
  finance: "财务",
  transactions: "账单",
  accounts: "账户",
  import: "导入账单",
  calendar: "生活日历",
  review: "每日复盘",
  notes: "笔记",
  settings: "设置",
  gallery: "设计系统",
};

export function isPlatformView(value: string): value is PlatformView {
  return value in pageTitles;
}
