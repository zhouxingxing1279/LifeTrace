import { Navigate, createBrowserRouter } from "react-router-dom";
import { useApp } from "./AppContext";
import { AppShell } from "../layouts/AppShell";
import { LoginPage } from "../features/auth/LoginPage";
import { TodayPage } from "../features/dashboard/TodayPage";
import { ExecutionPage } from "../features/execution/ExecutionPage";
import { CalendarPage } from "../features/calendar/CalendarPage";
import { HabitsPage } from "../features/habits/HabitsPage";
import { FitnessPage } from "../features/fitness/FitnessPage";
import { HealthPage } from "../features/health/HealthPage";
import { NotesPage } from "../features/notes/NotesPage";
import { EnglishPage } from "../features/english/EnglishPage";
import { ReviewPage } from "../features/review/ReviewPage";
import { AssistantPage } from "../features/assistant/AssistantPage";
import { SearchPage } from "../features/search/SearchPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { WorkspacePlaceholder } from "../features/system/WorkspacePlaceholder";

function ProtectedShell(){const {session,authLoading,online}=useApp();if(authLoading)return <div className="flex min-h-screen items-center justify-center text-sm text-muted-foreground">正在验证 LifeTrace Cloud 会话…</div>;if(!session)return <Navigate to="/login" replace state={{offline:!online}}/>;return <AppShell/>}
export const router=createBrowserRouter([{path:"/",element:<Navigate to="/app/today" replace/>},{path:"/login",element:<LoginPage/>},{path:"/app",element:<ProtectedShell/>,children:[{index:true,element:<Navigate to="today" replace/>},{path:"today",element:<TodayPage/>},{path:"execution",element:<ExecutionPage/>},{path:"calendar",element:<CalendarPage/>},{path:"habits",element:<HabitsPage/>},{path:"fitness",element:<FitnessPage/>},{path:"health",element:<HealthPage/>},{path:"notes",element:<NotesPage/>},{path:"english/*",element:<EnglishPage/>},{path:"review",element:<ReviewPage/>},{path:"finance/*",element:<WorkspacePlaceholder title="财务" description="BeeCount Cloud Web 源码派生的财务工作区。" references="BeeCount Cloud Web · LifeTrace AppShell"/>},{path:"assistant",element:<AssistantPage/>},{path:"search",element:<SearchPage/>},{path:"settings/*",element:<SettingsPage/>},{path:"*",element:<Navigate to="today" replace/>}]},{path:"*",element:<Navigate to="/app/today" replace/>}]);
