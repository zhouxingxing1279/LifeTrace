import type { ReactNode } from "react";
import type { CloudDataStore, CloudState, EntityType, JsonEntity, WebSession } from "./core";
import { currentRoute, navigate, type Route } from "./navigation";

export type { Route } from "./navigation";
export { currentRoute, navigate } from "./navigation";

export interface CloudPageProps {
  session: WebSession;
  state: CloudState;
  privacy: boolean;
  online: boolean;
  run: (action: (store: CloudDataStore) => Promise<CloudState>) => Promise<CloudState>;
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

export function Notice({ kind, children }: { kind: "error" | "warning" | "neutral" | "success"; children: ReactNode }) {
  return <div className={`notice ${kind} hx-notice`}>{children}</div>;
}

export function PageStack({ children }: { children: ReactNode }) {
  return <div className="page-stack hx-view">{children}</div>;
}

export function Panel({ title, eyebrow, actions, children, className = "" }: { title: string; eyebrow: string; actions?: ReactNode; children: ReactNode; className?: string }) {
  return <section className={`panel hx-panel ${className}`.trim()}>
    <div className="panel-heading hx-panel-head"><div><p className="eyebrow">{eyebrow}</p><h3>{title}</h3></div>{actions}</div>
    <div className="hx-panel-body">{children}</div>
  </section>;
}

export function Metric({ label, value, detail, positive = false }: { label: string; value: string; detail: string; positive?: boolean }) {
  return <article className="metric-card hx-metric"><span>{label}</span><strong>{value}</strong><small className={positive ? "positive" : ""}>{detail}</small></article>;
}

export function Empty({ title, description }: { title: string; description: string }) {
  return <div className="empty-state hx-empty"><span>—</span><h4>{title}</h4><p>{description}</p></div>;
}

export function TabBar({ items }: { items: Array<{ route: Route; label: string }> }) {
  const route = currentRoute();
  return <nav className="tab-bar hx-tab-bar">{items.map((item) => <button key={item.route} className={route === item.route ? "active" : ""} onClick={() => navigate(item.route)}>{item.label}</button>)}</nav>;
}

export function FinanceTabs() {
  return <TabBar items={[
    { route: "/finance", label: "概览" },
    { route: "/finance/transactions", label: "账单" },
    { route: "/finance/accounts", label: "账户" },
    { route: "/finance/categories", label: "分类" },
    { route: "/finance/budgets", label: "预算" },
    { route: "/finance/import", label: "导入与对账" },
    { route: "/finance/beecount", label: "BeeCount 云账本" },
  ]} />;
}

export function EnglishTabs() {
  return <TabBar items={[
    { route: "/english/articles", label: "文章" },
    { route: "/english/vocabulary", label: "生词" },
    { route: "/english/stats", label: "统计" },
  ]} />;
}
