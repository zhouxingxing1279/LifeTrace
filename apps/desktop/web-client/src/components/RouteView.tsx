import type { AuthApi, CloudDataStore, CloudState, WebSession } from "../core";
import type { Route } from "../navigation";
import { AccountsPage, BudgetsPage, CategoriesPage, FinanceOverview, ImportPage, TransactionsPage } from "../pages/FinancePages";
import { BeeCountFinancePage } from "../pages/BeeCountFinancePage";
import { NotesPage } from "../pages/NotesPage";
import { ArticlesPage, EnglishStatsPage, VocabularyPage } from "../pages/EnglishPages";
import { DevicesPage } from "../pages/DevicesPage";
import { AssistantPage, CalendarPage, FitnessPage, HabitsPage, ReviewPage, SettingsPage } from "../pages/GrowthPages";
import { DashboardPage } from "../pages/DashboardPage";
import { ExecutionPage } from "../pages/ExecutionPage";
import { SearchPage } from "../pages/SearchPage";

interface RouteViewProps {
  route: Route;
  auth: AuthApi;
  session: WebSession;
  state: CloudState;
  privacy: boolean;
  online: boolean;
  run: (action: (store: CloudDataStore) => Promise<CloudState>) => Promise<CloudState>;
}

export function RouteView({ route, auth, session, state, privacy, online, run }: RouteViewProps) {
  const common = { session, state, privacy, online, run };

  switch (route) {
    case "/": return <DashboardPage state={state} privacy={privacy} />;
    case "/assistant": return <AssistantPage {...common} />;
    case "/execution": return <ExecutionPage {...common} />;
    case "/habits": return <HabitsPage {...common} />;
    case "/fitness": return <FitnessPage {...common} />;
    case "/calendar": return <CalendarPage {...common} />;
    case "/review": return <ReviewPage {...common} />;
    case "/search": return <SearchPage state={state} />;
    case "/devices": return <DevicesPage session={session} auth={auth} online={online} />;
    case "/settings": return <SettingsPage {...common} />;
    case "/finance": return <FinanceOverview {...common} />;
    case "/finance/transactions": return <TransactionsPage {...common} />;
    case "/finance/accounts": return <AccountsPage {...common} />;
    case "/finance/categories": return <CategoriesPage {...common} />;
    case "/finance/budgets": return <BudgetsPage {...common} />;
    case "/finance/import": return <ImportPage {...common} />;
    case "/finance/beecount": return <BeeCountFinancePage privacy={privacy} online={online} />;
    case "/notes": return <NotesPage {...common} />;
    case "/english/articles": return <ArticlesPage {...common} />;
    case "/english/vocabulary": return <VocabularyPage {...common} />;
    case "/english/stats": return <EnglishStatsPage state={state} />;
  }
}
