import { lazy, Suspense, type ReactNode } from "react";
import { Navigate, createBrowserRouter } from "react-router-dom";
import { useApp } from "./AppContext";
import { AppShell } from "../layouts/AppShell";

const LoginPage = lazy(() => import("../features/auth/LoginPage").then((module) => ({ default: module.LoginPage })));
const TodayPage = lazy(() => import("../features/dashboard/TodayPage").then((module) => ({ default: module.TodayPage })));
const ExecutionPage = lazy(() => import("../features/execution/ExecutionPage").then((module) => ({ default: module.ExecutionPage })));
const CalendarPage = lazy(() => import("../features/calendar/CalendarPage").then((module) => ({ default: module.CalendarPage })));
const HabitsPage = lazy(() => import("../features/habits/HabitsPage").then((module) => ({ default: module.HabitsPage })));
const FitnessPage = lazy(() => import("../features/fitness/FitnessPage").then((module) => ({ default: module.FitnessPage })));
const HealthPage = lazy(() => import("../features/health/HealthPage").then((module) => ({ default: module.HealthPage })));
const NotesPage = lazy(() => import("../features/notes/NotesPage").then((module) => ({ default: module.NotesPage })));
const EnglishPage = lazy(() => import("../features/english/EnglishPage").then((module) => ({ default: module.EnglishPage })));
const ReviewPage = lazy(() => import("../features/review/ReviewPage").then((module) => ({ default: module.ReviewPage })));
const FinanceWorkspace = lazy(() => import("../features/finance/FinanceWorkspace").then((module) => ({ default: module.FinanceWorkspace })));
const FinanceTransactionsPage = lazy(() => import("../features/finance/FinanceTransactionsPage").then((module) => ({ default: module.FinanceTransactionsPage })));
const AssistantPage = lazy(() => import("../features/assistant/AssistantPage").then((module) => ({ default: module.AssistantPage })));
const SearchPage = lazy(() => import("../features/search/SearchPage").then((module) => ({ default: module.SearchPage })));
const SettingsPage = lazy(() => import("../features/settings/SettingsPage").then((module) => ({ default: module.SettingsPage })));
const UiShowcasePage = lazy(() => import("../features/system/UiShowcasePage").then((module) => ({ default: module.UiShowcasePage })));

function PageFallback() {
  return (
    <div className="page-shell">
      <div className="space-y-3" aria-label="页面加载中" aria-busy="true">
        <div className="h-8 w-40 animate-pulse rounded-md bg-muted" />
        <div className="h-4 w-72 max-w-full animate-pulse rounded bg-muted" />
        <div className="mt-6 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {Array.from({ length: 4 }, (_, index) => <div key={index} className="h-28 animate-pulse rounded-lg border bg-card" />)}
        </div>
      </div>
    </div>
  );
}

function withSuspense(element: ReactNode) {
  return <Suspense fallback={<PageFallback />}>{element}</Suspense>;
}

function ProtectedShell() {
  const { session, authLoading, online } = useApp();
  if (authLoading) {
    return <div className="flex min-h-screen items-center justify-center text-sm text-muted-foreground">正在验证 LifeTrace Cloud 会话…</div>;
  }
  if (!session) return <Navigate to="/login" replace state={{ offline: !online }} />;
  return <AppShell />;
}

export const router = createBrowserRouter([
  { path: "/", element: <Navigate to="/app/today" replace /> },
  { path: "/login", element: withSuspense(<LoginPage />) },
  {
    path: "/app",
    element: <ProtectedShell />,
    children: [
      { index: true, element: <Navigate to="today" replace /> },
      { path: "today", element: withSuspense(<TodayPage />) },
      { path: "execution", element: withSuspense(<ExecutionPage />) },
      { path: "calendar", element: withSuspense(<CalendarPage />) },
      { path: "habits", element: withSuspense(<HabitsPage />) },
      { path: "fitness", element: withSuspense(<FitnessPage />) },
      { path: "health", element: withSuspense(<HealthPage />) },
      { path: "notes", element: withSuspense(<NotesPage />) },
      { path: "english/*", element: withSuspense(<EnglishPage />) },
      { path: "review", element: withSuspense(<ReviewPage />) },
      { path: "finance/transactions", element: withSuspense(<FinanceTransactionsPage />) },
      { path: "finance/*", element: withSuspense(<FinanceWorkspace />) },
      { path: "assistant", element: withSuspense(<AssistantPage />) },
      { path: "search", element: withSuspense(<SearchPage />) },
      { path: "settings/*", element: withSuspense(<SettingsPage />) },
      { path: "system/ui", element: withSuspense(<UiShowcasePage />) },
      { path: "*", element: <Navigate to="today" replace /> },
    ],
  },
  { path: "*", element: <Navigate to="/app/today" replace /> },
]);
