import type { ReactNode } from "react";
import type { CloudDataStore, CloudState, EntityType, JsonEntity, WebSession } from "./core";

export type Route =
  | "/"
  | "/search"
  | "/devices"
  | "/finance"
  | "/finance/transactions"
  | "/finance/accounts"
  | "/finance/categories"
  | "/finance/budgets"
  | "/finance/import"
  | "/notes"
  | "/english/articles"
  | "/english/vocabulary"
  | "/english/stats";

const ROUTES = new Set<Route>([
  "/", "/search", "/devices", "/finance", "/finance/transactions",
  "/finance/accounts", "/finance/categories", "/finance/budgets",
  "/finance/import", "/notes", "/english/articles",
  "/english/vocabulary", "/english/stats",
]);

export interface CloudPageProps {
  session: WebSession;
  state: CloudState;
  privacy: boolean;
  online: boolean;
  run: (action: (store: CloudDataStore) => Promise<CloudState>) => Promise<CloudState>;
}

export function currentRoute(): Route {
  const path = window.location.pathname.replace(/\/$/, "") || "/";
  return ROUTES.has(path as Route) ? (path as Route) : "/";
}

export function navigate(route: Route): void {
  if (window.location.pathname !== route) window.history.pushState({}, "", route);
  window.dispatchEvent(new PopStateEvent("popstate"));
}

export function text(entity: JsonEntity, key: string): string {
  return typeof entity[key] === "string" ? String(entity[key]) : "";
}

export function number(entity: JsonEntity, key: string): number {
  return typeof entity[key] === "number" ? Number(entity[key]) : 0;
}

export function entities(state: CloudState, entityType: EntityType): JsonEntity[] {
  return Object.values(state.entities[entityType] ?? {});
}

export function Notice({ kind, children }: { kind: "error" | "warning" | "neutral"; children: ReactNode }) {
  return <div className={`notice ${kind}`}>{children}</div>;
}

export function PageStack({ children }: { children: ReactNode }) {
  return <div className="page-stack">{children}</div>;
}

export function Panel({ title, eyebrow, actions, children }: { title: string; eyebrow: string; actions?: ReactNode; children: ReactNode }) {
  return <section className="panel"><div className="panel-heading"><div><p className="eyebrow">{eyebrow}</p><h3>{title}</h3></div>{actions}</div>{children}</section>;
}

export function Metric({ label, value, detail }: { label: string; value: string; detail: string }) {
  return <article className="metric-card"><span>{label}</span><strong>{value}</strong><small>{detail}</small></article>;
}

export function Empty({ title, description }: { title: string; description: string }) {
  return <div className="empty-state"><span>·</span><h4>{title}</h4><p>{description}</p></div>;
}

export function TabBar({ items }: { items: Array<{ route: Route; label: string }> }) {
  return <nav className="tab-bar">{items.map((item) => <button key={item.route} className={window.location.pathname === item.route ? "active" : ""} onClick={() => navigate(item.route)}>{item.label}</button>)}</nav>;
}

export function FinanceTabs() {
  return <TabBar items={[
    { route: "/finance", label: "概览" },
    { route: "/finance/transactions", label: "账单" },
    { route: "/finance/accounts", label: "账户" },
    { route: "/finance/categories", label: "分类" },
    { route: "/finance/budgets", label: "预算" },
    { route: "/finance/import", label: "导入与对账" },
  ]} />;
}

export function EnglishTabs() {
  return <TabBar items={[
    { route: "/english/articles", label: "文章" },
    { route: "/english/vocabulary", label: "生词" },
    { route: "/english/stats", label: "统计" },
  ]} />;
}
