import type { ReactNode } from "react";

/** 页面内容容器：统一宽度、内边距与进入动画。 */
export function PageContainer({ children }: { children: ReactNode }) {
  return <div className="hx-view">{children}</div>;
}

/** 页面头部：标题 + 可选说明 + 右侧操作区。 */
export function PageHeader({
  title,
  description,
  actions,
}: {
  title: string;
  description?: string;
  actions?: ReactNode;
}) {
  return (
    <header className="lt-page-header">
      <div className="lt-page-header-copy">
        <h1>{title}</h1>
        {description ? <p>{description}</p> : null}
      </div>
      {actions ? <div className="lt-page-actions">{actions}</div> : null}
    </header>
  );
}

/** 页面工具栏：左侧筛选/搜索，右侧操作。 */
export function Toolbar({
  left,
  right,
  summary,
}: {
  left?: ReactNode;
  right?: ReactNode;
  summary?: string;
}) {
  return (
    <div className="lt-toolbar">
      <div className="lt-toolbar-left">{left}</div>
      {summary ? <span className="hx-toolbar-summary">{summary}</span> : null}
      <div className="lt-toolbar-right">{right}</div>
    </div>
  );
}

/** 内容分区：带标题与操作的独立区块。 */
export function Section({
  title,
  description,
  actions,
  children,
  className,
}: {
  title?: string;
  description?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`lt-section ${className ?? ""}`.trim()}>
      {title ? (
        <header className="lt-section-head">
          <div>
            <h2>{title}</h2>
            {description ? <p>{description}</p> : null}
          </div>
          {actions ? <div>{actions}</div> : null}
        </header>
      ) : null}
      {children}
    </section>
  );
}
