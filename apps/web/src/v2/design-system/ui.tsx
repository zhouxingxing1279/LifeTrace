import type { ButtonHTMLAttributes, HTMLAttributes, InputHTMLAttributes, ReactNode, SelectHTMLAttributes, TextareaHTMLAttributes } from "react";
import { Search } from "lucide-react";

export const cx = (...parts: Array<string | false | null | undefined>) => parts.filter(Boolean).join(" ");

export function Button({ className, children, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button className={cx("lt-button", className)} {...props}>{children}</button>;
}

export function IconButton({ className, children, "aria-label": ariaLabel, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button className={cx("lt-icon-button", className)} aria-label={ariaLabel} {...props}>{children}</button>;
}

export function Input({ className, ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={cx("lt-input", className)} {...props} />;
}

export function Textarea({ className, ...props }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea className={cx("lt-textarea", className)} {...props} />;
}

export function SearchField({ className, ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return <label className={cx("lt-search", className)}><Search size={17} aria-hidden="true" /><input aria-label={props["aria-label"] ?? "搜索"} {...props} /></label>;
}

export function Select({ className, children, ...props }: SelectHTMLAttributes<HTMLSelectElement>) {
  return <select className={cx("lt-select", className)} {...props}>{children}</select>;
}

export function SegmentedControl<T extends string>({ value, options, onChange, label }: { value: T; options: Array<{ value: T; label: string }>; onChange: (value: T) => void; label: string }) {
  return <div className="lt-segmented" role="group" aria-label={label}>{options.map((option) => <button key={option.value} className={cx(option.value === value && "is-active")} aria-pressed={option.value === value} onClick={() => onChange(option.value)}>{option.label}</button>)}</div>;
}

export function Switch({ checked, onChange, label }: { checked: boolean; onChange: (checked: boolean) => void; label: string }) {
  return <button type="button" className={cx("lt-switch", checked && "is-on")} role="switch" aria-checked={checked} onClick={() => onChange(!checked)}><span /><span className="sr-only">{label}</span></button>;
}

export function Checkbox({ label, ...props }: InputHTMLAttributes<HTMLInputElement> & { label?: string }) {
  return <label className="lt-checkbox"><input type="checkbox" {...props} />{label ? <span>{label}</span> : null}</label>;
}

export function Card({ className, children, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <section className={cx("lt-card", className)} {...props}>{children}</section>;
}

export function StatCard({ label, value, detail }: { label: string; value: ReactNode; detail?: ReactNode }) {
  return <div className="lt-stat"><span>{label}</span><strong>{value}</strong>{detail ? <small>{detail}</small> : null}</div>;
}

export function List({ children, className }: { children: ReactNode; className?: string }) { return <div className={cx("lt-list", className)}>{children}</div>; }
export function ListItem({ children, className }: { children: ReactNode; className?: string }) { return <div className={cx("lt-list-item", className)}>{children}</div>; }
export function Table({ children }: { children: ReactNode }) { return <div className="lt-table-scroll"><table className="lt-table">{children}</table></div>; }
export function Badge({ children, tone = "neutral" }: { children: ReactNode; tone?: "neutral" | "success" | "warning" | "danger" | "accent" }) { return <span className={cx("lt-badge", `tone-${tone}`)}>{children}</span>; }
export function Progress({ value, label }: { value: number; label: string }) { const safe = Math.max(0, Math.min(100, value)); return <div className="lt-progress" aria-label={label} role="progressbar" aria-valuenow={safe} aria-valuemin={0} aria-valuemax={100}><span style={{ width: `${safe}%` }} /></div>; }

export function Tabs<T extends string>(props: { value: T; options: Array<{ value: T; label: string }>; onChange: (value: T) => void; label: string }) { return <SegmentedControl {...props} />; }

export function Popover({ trigger, children }: { trigger: ReactNode; children: ReactNode }) { return <details className="lt-popover"><summary>{trigger}</summary><div className="lt-popover-panel">{children}</div></details>; }
export function Dropdown({ trigger, children }: { trigger: ReactNode; children: ReactNode }) { return <Popover trigger={trigger}>{children}</Popover>; }
export function ContextMenu({ children }: { children: ReactNode }) { return <div className="lt-menu" role="menu">{children}</div>; }

export function Modal({ open, title, children, onClose }: { open: boolean; title: string; children: ReactNode; onClose: () => void }) {
  if (!open) return null;
  return <div className="lt-overlay" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}><section className="lt-modal" role="dialog" aria-modal="true" aria-labelledby="lt-modal-title"><header><h2 id="lt-modal-title">{title}</h2><IconButton aria-label="关闭" onClick={onClose}>×</IconButton></header>{children}</section></div>;
}
export const AlertDialog = Modal;
export function Sheet({ open, title, children, onClose }: { open: boolean; title: string; children: ReactNode; onClose: () => void }) { if (!open) return null; return <div className="lt-overlay"><aside className="lt-sheet" role="dialog" aria-modal="true"><header><h2>{title}</h2><IconButton aria-label="关闭" onClick={onClose}>×</IconButton></header>{children}</aside></div>; }
export function Toast({ children }: { children: ReactNode }) { return <div className="lt-toast" role="status">{children}</div>; }
export function EmptyState({ title, detail, action }: { title: string; detail: string; action?: ReactNode }) { return <div className="lt-empty"><strong>{title}</strong><p>{detail}</p>{action}</div>; }
export function Skeleton({ width = "100%" }: { width?: string }) { return <span className="lt-skeleton" style={{ width }} aria-hidden="true" />; }
export function Tooltip({ label, children }: { label: string; children: ReactNode }) { return <span className="lt-tooltip" title={label}>{children}</span>; }
export function ChartContainer({ title, children }: { title: string; children: ReactNode }) { return <section className="lt-chart" aria-label={title}><header><h3>{title}</h3></header>{children}</section>; }

export function CommandPalette({ open, query, onQuery, children, onClose }: { open: boolean; query: string; onQuery: (value: string) => void; children: ReactNode; onClose: () => void }) {
  if (!open) return null;
  return <div className="lt-command-overlay" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}><section className="lt-command" role="dialog" aria-modal="true" aria-label="命令菜单"><SearchField autoFocus value={query} onChange={(event) => onQuery(event.target.value)} placeholder="搜索页面、任务、笔记…" /><div className="lt-command-results">{children}</div></section></div>;
}
