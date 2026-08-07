import type { ReactNode } from "react";
import { AlertTriangle, Inbox, RefreshCw } from "lucide-react";
import { Button, Skeleton } from "@/src/components/ui";

/* ------------------------------------------------------------------ */
/* Empty state                                                         */
/* ------------------------------------------------------------------ */

export function EmptyState({
  title,
  hint,
  icon,
  action,
}: {
  title: string;
  hint?: string;
  icon?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="lt-empty">
      <span className="lt-empty-icon">{icon ?? <Inbox aria-hidden="true" />}</span>
      <strong>{title}</strong>
      {hint ? <p>{hint}</p> : null}
      {action ? <div className="lt-empty-action">{action}</div> : null}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Error state                                                         */
/* ------------------------------------------------------------------ */

export function ErrorState({
  title,
  message,
  detail,
  onRetry,
  retryLabel = "重试",
}: {
  title: string;
  message?: string;
  detail?: string;
  onRetry?: () => void;
  retryLabel?: string;
}) {
  return (
    <div className="lt-error" role="alert">
      <span className="lt-error-icon">
        <AlertTriangle aria-hidden="true" />
      </span>
      <strong>{title}</strong>
      {message ? <p>{message}</p> : null}
      {detail ? (
        <details>
          <summary>详细信息</summary>
          <pre>{detail}</pre>
        </details>
      ) : null}
      {onRetry ? (
        <Button variant="primary" icon={<RefreshCw aria-hidden="true" />} onClick={onRetry}>
          {retryLabel}
        </Button>
      ) : null}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Loading skeleton                                                    */
/* ------------------------------------------------------------------ */

export function LoadingState({
  rows = 4,
  label = "正在加载",
}: {
  rows?: number;
  label?: string;
}) {
  return (
    <div className="lt-loading" role="status" aria-label={label}>
      {Array.from({ length: rows }, (_, index) => (
        <div className="lt-loading-row" key={index}>
          <Skeleton width={38} height={38} radius={8} />
          <div className="lt-loading-copy">
            <Skeleton width="70%" height={13} />
            <Skeleton width="45%" height={11} />
          </div>
        </div>
      ))}
    </div>
  );
}

export function LoadingPanel({
  label = "正在加载",
}: {
  label?: string;
}) {
  return (
    <div className="hx-panel" role="status" aria-label={label}>
      <div className="hx-panel-body">
        <LoadingState rows={4} />
      </div>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Stat display                                                        */
/* ------------------------------------------------------------------ */

export function StatDisplay({
  label,
  value,
  sub,
  tone,
}: {
  label: string;
  value: string;
  sub?: string;
  tone?: "positive" | "negative";
}) {
  return (
    <div className="lt-stat">
      <label>{label}</label>
      <strong className={tone}>{value}</strong>
      {sub ? <small>{sub}</small> : null}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Panel header                                                        */
/* ------------------------------------------------------------------ */

export function PanelHead({
  kicker,
  title,
  action,
  onClick,
}: {
  kicker: string;
  title: string;
  action?: string;
  onClick?: () => void;
}) {
  return (
    <header className="hx-panel-head">
      <div>
        <span className="hx-kicker">{kicker}</span>
        <h2>{title}</h2>
      </div>
      {action ? (
        <button type="button" onClick={onClick}>
          {action}
        </button>
      ) : null}
    </header>
  );
}
