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
  return <div className={`notice ${kind} hx-notice`} role={kind === "error" ? "alert" : "status"}>{children}</div>;
}

export function PageStack({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <div className={`page-stack hx-view lt-page-stack ${className}`.trim()}>{children}</div>;
}

export function MetricGrid({ children, compact = false }: { children: ReactNode; compact?: boolean }) {
  return <div className={`hx-metrics lt-metric-grid ${compact ? "compact" : ""}`.trim()}>{children}</div>;
}

export function ContentGrid({ children, aside = false, className = "" }: { children: ReactNode; aside?: boolean; className?: string }) {
  return <div className={`hx-content-grid ${aside ? "two" : ""} lt-content-grid ${className}`.trim()}>{children}</div>;
}

export function Toolbar({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <div className={`hx-inline-actions lt-toolbar ${className}`.trim()}>{children}</div>;
}

export function Panel({ title, eyebrow, actions, children, className = "" }: { title: string; eyebrow: string; actions?: ReactNode; children: ReactNode; className?: string }) {
  return <section className={`panel hx-panel lt-panel ${className}`.trim()}>
    <div className="panel-heading hx-panel-head">
      <div><p className="eyebrow">{eyebrow}</p><h3>{title}</h3></div>
      {actions && <div className="lt-panel-actions">{actions}</div>}
    </div>
    <div className="hx-panel-body">{children}</div>
  </section>;
}

export function Metric({ label, value, detail, positive = false }: { label: string; value: string; detail: string; positive?: boolean }) {
  return <article className="metric-card hx-metric lt-metric">
    <span>{label}</span>
    <strong>{value}</strong>
    <small className={positive ? "positive" : ""}>{detail}</small>
  </article>;
}

export function Empty({ title, description }: { title: string; description: string }) {
  return <div className="empty-state hx-empty lt-empty" role="status"><span>—</span><h4>{title}</h4><p>{description}</p></div>;
}

export function TabBar({ items, label = "页面切换" }: { items: Array<{ route: Route; label: string }>; label?: string }) {
  const route = currentRoute();
  return <nav className="tab-bar hx-tab-bar lt-tabs" aria-label={label}>
    {items.map((item) => <button
      key={item.route}
      className={route === item.route ? "active" : ""}
      aria-current={route === item.route ? "page" : undefined}
      onClick={() => navigate(item.route)}
    >{item.label}</button>)}
  </nav>;
}

export function FinanceTabs() {
  return <TabBar label="财务功能" items={[
    { route: "/finance", label: "概览" },
    { route: "/finance/transactions", label: "账单" },
    { route: "/finance/accounts", label: "账户" },
    { route: "/finance/categories", label: "分类" },
    { route: "/finance/budgets", label: "预算" },
    { route: "/finance/import", label: "导入与对账" },
    { route: "/finance/beecount", label: "BeeCount" },
  ]} />;
}

export function EnglishTabs() {
  return <TabBar label="英语学习" items={[
    { route: "/english/articles", label: "文章" },
    { route: "/english/vocabulary", label: "生词" },
    { route: "/english/stats", label: "统计" },
  ]} />;
}
