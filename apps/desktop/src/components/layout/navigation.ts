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
  NotebookPen,
  WalletCards,
} from "lucide-react";
import type { NavGroup } from "./AppShell";

export type PlatformView =
  | "dashboard"
  | "assistant"
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
    label: "概览",
    items: [
      { id: "dashboard", label: "总览", icon: Home },
      { id: "assistant", label: "AI 管家", icon: Bot },
    ],
  },
  {
    label: "成长与生活",
    items: [
      { id: "habits", label: "坚持项目", icon: Check },
      { id: "english", label: "每日英语", icon: Languages },
      { id: "fitness", label: "健身训练", icon: Dumbbell },
      { id: "photos", label: "照片", icon: Images },
      { id: "notes", label: "笔记", icon: NotebookPen },
      { id: "calendar", label: "生活日历", icon: CalendarDays },
      { id: "review", label: "每日复盘", icon: BookOpen },
    ],
  },
  {
    label: "资产与账单",
    items: [
      { id: "finance", label: "财务概览", icon: BarChart3 },
      { id: "transactions", label: "账单管理", icon: CircleDollarSign },
      { id: "accounts", label: "账户管理", icon: WalletCards },
      { id: "import", label: "账单导入", icon: FileUp },
    ],
  },
];

export const pageTitles: Record<PlatformView, string> = {
  dashboard: "总览",
  assistant: "AI 管家",
  habits: "坚持项目",
  english: "每日英语",
  fitness: "健身训练",
  photos: "照片",
  finance: "财务概览",
  transactions: "账单管理",
  accounts: "账户管理",
  import: "账单导入",
  calendar: "生活日历",
  review: "每日复盘",
  notes: "笔记",
  settings: "设置",
  gallery: "设计系统",
};

export function isPlatformView(value: string): value is PlatformView {
  return value in pageTitles;
}
