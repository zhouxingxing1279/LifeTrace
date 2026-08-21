import { lazy, Suspense, type ReactNode } from "react";
import { BrowserRouter, Navigate, Route, Routes, useLocation, useNavigate } from "react-router-dom";

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

export type DesktopRouteBridge = {
  path: string;
  navigate(path: string): void;
  content: ReactNode;
};

type DesktopFeatureRouterProps = {
  render(bridge: DesktopRouteBridge): ReactNode;
};

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

function FeatureRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/app/today" replace />} />
      <Route path="/app" element={<Navigate to="/app/today" replace />} />
      <Route path="/app/today" element={withSuspense(<TodayPage />)} />
      <Route path="/app/execution" element={withSuspense(<ExecutionPage />)} />
      <Route path="/app/calendar" element={withSuspense(<CalendarPage />)} />
      <Route path="/app/habits" element={withSuspense(<HabitsPage />)} />
      <Route path="/app/fitness" element={withSuspense(<FitnessPage />)} />
      <Route path="/app/health" element={withSuspense(<HealthPage />)} />
      <Route path="/app/notes" element={withSuspense(<NotesPage />)} />
      <Route path="/app/english/*" element={withSuspense(<EnglishPage />)} />
      <Route path="/app/review" element={withSuspense(<ReviewPage />)} />
      <Route path="/app/finance/transactions" element={withSuspense(<FinanceTransactionsPage />)} />
      <Route path="/app/finance/*" element={withSuspense(<FinanceWorkspace />)} />
      <Route path="/app/assistant" element={withSuspense(<AssistantPage />)} />
      <Route path="/app/search" element={withSuspense(<SearchPage />)} />
      <Route path="/app/settings/*" element={withSuspense(<SettingsPage />)} />
      <Route path="/app/system/ui" element={withSuspense(<UiShowcasePage />)} />
      <Route path="*" element={<Navigate to="/app/today" replace />} />
    </Routes>
  );
}

function DesktopRouterBridge({ render }: DesktopFeatureRouterProps) {
  const location = useLocation();
  const navigate = useNavigate();
  return render({
    path: location.pathname,
    navigate: (path) => navigate(path),
    content: <FeatureRoutes />,
  });
}

export function DesktopFeatureRouter({ render }: DesktopFeatureRouterProps) {
  return (
    <BrowserRouter>
      <DesktopRouterBridge render={render} />
    </BrowserRouter>
  );
}
