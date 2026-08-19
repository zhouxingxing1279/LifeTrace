import { Navigate, createBrowserRouter } from "react-router-dom";
import { useApp } from "./AppContext";
import { AppShell } from "../layouts/AppShell";
import { LoginPage } from "../features/auth/LoginPage";
import { WorkspacePlaceholder } from "../features/system/WorkspacePlaceholder";

function ProtectedShell() {
  const { session, authLoading, online } = useApp();
  if (authLoading) return <div className="flex min-h-screen items-center justify-center text-sm text-muted-foreground">正在验证 LifeTrace Cloud 会话…</div>;
  if (!session) return <Navigate to="/login" replace state={{ offline: !online }} />;
  return <AppShell />;
}

function page(title: string, description: string, references: string) {
  return <WorkspacePlaceholder title={title} description={description} references={references} />;
}

export const router = createBrowserRouter([
  { path: "/", element: <Navigate to="/app/today" replace /> },
  { path: "/login", element: <LoginPage /> },
  {
    path: "/app",
    element: <ProtectedShell />,
    children: [
      { index: true, element: <Navigate to="today" replace /> },
      { path: "today", element: page("今日", "今天最重要的事项、完成度、下一步和关键趋势。", "Shadcnblocks Dashboard · Tremor · Catalyst") },
      { path: "execution", element: page("计划与待办", "从 Inbox、Today、Upcoming 到 Project 的执行闭环。", "Shadcnblocks Todo · Catalyst Lists · Preline") },
      { path: "calendar", element: page("日历", "Month / Week / Day / Agenda，移动端优先 Agenda。", "Shadcnblocks Calendar · Catalyst Toolbar · Preline") },
      { path: "habits", element: page("坚持", "今日打卡、streak、7/30 天趋势和完成率。", "Tremor · Shadcnblocks") },
      { path: "fitness", element: page("健身", "本周训练、训练量趋势、动作分布和训练记录。", "Tremor Analytics · Shadcnblocks") },
      { path: "health", element: page("健康", "以趋势为中心组织身体指标与长期变化。", "Tremor Analytics · Apple Health hierarchy") },
      { path: "notes", element: page("笔记", "内容工作区、笔记列表和编辑器。", "Catalyst workspace · shadcn/ui · Preline") },
      { path: "english/*", element: page("英语学习", "阅读、高亮、快捷笔记、生词和学习历史。", "Catalyst content · shadcn/ui · Shadcnblocks") },
      { path: "review", element: page("复盘", "日复盘与 7/30 天总结。", "Shadcnblocks Dashboard · Tremor · Catalyst") },
      { path: "finance/*", element: page("财务", "BeeCount Cloud Web 源码派生的财务工作区。", "BeeCount Cloud Web · LifeTrace AppShell") },
      { path: "assistant", element: page("AI 助手", "基于 LifeTrace 云端记录提供摘要与下一步建议。", "shadcn Command/Dialog · Aceternity/Magic UI micro-interactions") },
      { path: "search", element: page("全局搜索", "搜索任务、坚持、训练、交易、笔记、英语与设置。", "shadcn Command · Catalyst Search") },
      { path: "settings/*", element: page("设置", "Profile、Appearance、Cloud & Sync、Devices、Privacy、Security、Data。", "Catalyst Settings · shadcn Form · Preline") },
      { path: "*", element: <Navigate to="today" replace /> },
    ],
  },
  { path: "*", element: <Navigate to="/app/today" replace /> },
]);
