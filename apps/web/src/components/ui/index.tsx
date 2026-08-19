import {
  forwardRef,
  useEffect,
  useRef,
  type ButtonHTMLAttributes,
  type DetailsHTMLAttributes,
  type HTMLAttributes,
  type InputHTMLAttributes,
  type PropsWithChildren,
  type ReactNode,
  type SelectHTMLAttributes,
  type TableHTMLAttributes,
  type TextareaHTMLAttributes,
} from "react";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...values: Array<string | false | null | undefined>): string {
  return twMerge(clsx(values));
}

export function Card({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("surface", className)} {...props} />;
}

export function CardHeader({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("flex items-start justify-between gap-4 px-5 pt-5", className)} {...props} />;
}

export function CardContent({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("px-5 pb-5 pt-4", className)} {...props} />;
}

export function Badge({ className, ...props }: HTMLAttributes<HTMLSpanElement>) {
  return <span className={cn("inline-flex min-h-6 items-center rounded-md border bg-muted/45 px-2 py-0.5 text-[11px] font-medium text-muted-foreground", className)} {...props} />;
}

export type ButtonVariant = "default" | "secondary" | "ghost" | "outline" | "destructive";

export const Button = forwardRef<HTMLButtonElement, ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant; size?: "sm" | "md" | "icon" }>(
  ({ className, variant = "default", size = "md", type = "button", ...props }, ref) => {
    const variants: Record<ButtonVariant, string> = {
      default: "bg-primary text-primary-foreground hover:bg-primary/90",
      secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/75",
      ghost: "bg-transparent hover:bg-muted",
      outline: "border bg-background hover:bg-muted",
      destructive: "bg-destructive text-destructive-foreground hover:bg-destructive/90",
    };
    const sizes = { sm: "h-8 px-3 text-xs", md: "h-9 px-3.5 text-sm", icon: "h-9 w-9 p-0" };
    return <button ref={ref} type={type} className={cn("inline-flex shrink-0 items-center justify-center gap-2 rounded-md font-medium transition-colors disabled:pointer-events-none disabled:opacity-50", variants[variant], sizes[size], className)} {...props} />;
  },
);
Button.displayName = "Button";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(({ className, ...props }, ref) => (
  <input ref={ref} className={cn("h-10 w-full rounded-md border bg-background px-3 text-sm shadow-none outline-none placeholder:text-muted-foreground/70 focus:border-ring focus:ring-2 focus:ring-ring/15", className)} {...props} />
));
Input.displayName = "Input";

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement>>(({ className, ...props }, ref) => (
  <textarea ref={ref} className={cn("min-h-28 w-full resize-y rounded-md border bg-background px-3 py-2.5 text-sm outline-none placeholder:text-muted-foreground/70 focus:border-ring focus:ring-2 focus:ring-ring/15", className)} {...props} />
));
Textarea.displayName = "Textarea";

export const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement>>(({ className, ...props }, ref) => (
  <select ref={ref} className={cn("h-10 w-full rounded-md border bg-background px-3 text-sm outline-none focus:border-ring focus:ring-2 focus:ring-ring/15", className)} {...props} />
));
Select.displayName = "Select";

export function Checkbox({ className, ...props }: Omit<InputHTMLAttributes<HTMLInputElement>, "type">) {
  return <input type="checkbox" className={cn("h-4 w-4 rounded border accent-[hsl(var(--primary))]", className)} {...props} />;
}

export function Switch({ checked, onCheckedChange, disabled, label }: { checked: boolean; onCheckedChange(value: boolean): void; disabled?: boolean; label: string }) {
  return <button type="button" role="switch" aria-checked={checked} aria-label={label} disabled={disabled} onClick={() => onCheckedChange(!checked)} className={cn("relative h-6 w-11 rounded-full border transition-colors", checked ? "border-primary bg-primary" : "bg-muted")}><span className={cn("absolute top-[3px] h-4 w-4 rounded-full bg-background shadow-sm transition-transform", checked ? "translate-x-[22px]" : "translate-x-[3px]")} /></button>;
}

export type TabItem<T extends string> = { value: T; label: string };
export function Tabs<const T extends string>({ value, onValueChange, items, className }: { value: T; onValueChange(value: T): void; items: readonly TabItem<T>[]; className?: string }) {
  return <div role="tablist" className={cn("flex w-fit max-w-full overflow-x-auto rounded-md border p-0.5", className)}>{items.map((item) => <button key={item.value} role="tab" aria-selected={value === item.value} className={cn("shrink-0 rounded px-3 py-1.5 text-xs text-muted-foreground", value === item.value && "bg-muted font-medium text-foreground")} onClick={() => onValueChange(item.value)}>{item.label}</button>)}</div>;
}

export function Table({ className, ...props }: TableHTMLAttributes<HTMLTableElement>) {
  return <div className="scrollbar-thin w-full overflow-x-auto"><table className={cn("w-full border-collapse text-sm", className)} {...props} /></div>;
}
export function TableHeader({ className, ...props }: HTMLAttributes<HTMLTableSectionElement>) { return <thead className={cn("border-b bg-muted/30 text-xs text-muted-foreground", className)} {...props} />; }
export function TableBody({ className, ...props }: HTMLAttributes<HTMLTableSectionElement>) { return <tbody className={cn("divide-y", className)} {...props} />; }
export function TableRow({ className, ...props }: HTMLAttributes<HTMLTableRowElement>) { return <tr className={cn("hover:bg-muted/30", className)} {...props} />; }
export function TableHead({ className, ...props }: HTMLAttributes<HTMLTableCellElement>) { return <th className={cn("px-3 py-2 text-left font-medium", className)} {...props} />; }
export function TableCell({ className, ...props }: HTMLAttributes<HTMLTableCellElement>) { return <td className={cn("px-3 py-2.5", className)} {...props} />; }

const FOCUSABLE = 'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])';

function useFocusTrap(open: boolean) {
  const ref = useRef<HTMLDivElement>(null);
  const previous = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (!open) return;
    previous.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const timer = window.setTimeout(() => ref.current?.querySelector<HTMLElement>(FOCUSABLE)?.focus(), 0);
    return () => {
      window.clearTimeout(timer);
      previous.current?.focus();
    };
  }, [open]);
  return ref;
}

function trapKeyDown(event: React.KeyboardEvent<HTMLDivElement>, onClose?: () => void) {
  if (event.key === "Escape" && onClose) {
    event.preventDefault();
    onClose();
    return;
  }
  if (event.key !== "Tab") return;
  const focusable = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((element) => !element.hasAttribute("disabled") && element.tabIndex !== -1);
  if (!focusable.length) {
    event.preventDefault();
    event.currentTarget.focus();
    return;
  }
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

export function Dialog({ open, onOpenChange, title, description, children }: PropsWithChildren<{ open: boolean; onOpenChange(open: boolean): void; title: string; description?: string }>) {
  const ref = useFocusTrap(open);
  if (!open) return null;
  return <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4" role="presentation" onMouseDown={(event) => { if (event.currentTarget === event.target) onOpenChange(false); }}><div ref={ref} tabIndex={-1} onKeyDown={(event) => trapKeyDown(event, () => onOpenChange(false))} className="max-h-[85vh] w-full max-w-lg overflow-y-auto rounded-xl border bg-popover shadow-2xl" role="dialog" aria-modal="true" aria-label={title}><div className="border-b px-5 py-4"><div className="font-semibold">{title}</div>{description ? <p className="mt-1 text-xs leading-5 text-muted-foreground">{description}</p> : null}</div><div className="p-5">{children}</div></div></div>;
}

export function AlertDialog(props: PropsWithChildren<{ open: boolean; onOpenChange(open: boolean): void; title: string; description?: string }>) {
  return <Dialog {...props} />;
}

export function Sheet({ open, onOpenChange, title, children, side = "right" }: PropsWithChildren<{ open: boolean; onOpenChange(open: boolean): void; title: string; side?: "right" | "bottom" }>) {
  const ref = useFocusTrap(open);
  if (!open) return null;
  return <div className={cn("fixed inset-0 z-50 flex bg-black/40", side === "bottom" ? "items-end" : "justify-end")} role="presentation" onMouseDown={(event) => { if (event.currentTarget === event.target) onOpenChange(false); }}><div ref={ref} tabIndex={-1} onKeyDown={(event) => trapKeyDown(event, () => onOpenChange(false))} className={cn("border bg-popover shadow-2xl", side === "bottom" ? "max-h-[80vh] w-full rounded-t-xl" : "h-full w-full max-w-md border-l")} role="dialog" aria-modal="true" aria-label={title}><div className="border-b px-5 py-4 font-semibold">{title}</div><div className="scrollbar-thin max-h-full overflow-y-auto p-5">{children}</div></div></div>;
}
export const Drawer = Sheet;

export function DropdownMenu({ summary, children, className, ...props }: PropsWithChildren<{ summary: ReactNode } & DetailsHTMLAttributes<HTMLDetailsElement>>) {
  return <details className={cn("relative", className)} {...props}><summary className="cursor-pointer list-none">{summary}</summary><div className="absolute right-0 z-40 mt-2 min-w-44 rounded-md border bg-popover p-1 shadow-lg">{children}</div></details>;
}

export function Popover({ trigger, children, className }: PropsWithChildren<{ trigger: ReactNode; className?: string }>) {
  return <details className={cn("relative", className)}><summary className="cursor-pointer list-none">{trigger}</summary><div className="absolute z-40 mt-2 min-w-64 rounded-md border bg-popover p-3 shadow-lg">{children}</div></details>;
}

export function Tooltip({ content, children }: PropsWithChildren<{ content: string }>) {
  return <span title={content} className="inline-flex">{children}</span>;
}

export function Command({ children, className }: PropsWithChildren<{ className?: string }>) {
  return <div role="listbox" className={cn("overflow-hidden rounded-lg border bg-popover", className)}>{children}</div>;
}

export function Skeleton({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div aria-hidden="true" className={cn("animate-pulse rounded-md bg-muted", className)} {...props} />;
}

export function Separator({ className, orientation = "horizontal" }: { className?: string; orientation?: "horizontal" | "vertical" }) {
  return <div role="separator" aria-orientation={orientation} className={cn("shrink-0 bg-border", orientation === "horizontal" ? "h-px w-full" : "h-full w-px", className)} />;
}

export function ScrollArea({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("scrollbar-thin overflow-auto", className)} {...props} />;
}

export function Toast({ title, description, tone = "default", onDismiss }: { title: string; description?: string; tone?: "default" | "success" | "destructive"; onDismiss?(): void }) {
  return <div role="status" className={cn("rounded-lg border bg-popover px-4 py-3 shadow-lg", tone === "success" && "border-success/30", tone === "destructive" && "border-destructive/30")}><div className="flex items-start justify-between gap-4"><div><div className="text-sm font-medium">{title}</div>{description ? <div className="mt-1 text-xs text-muted-foreground">{description}</div> : null}</div>{onDismiss ? <button className="text-xs text-muted-foreground" onClick={onDismiss}>关闭</button> : null}</div></div>;
}

export function Progress({ value, className }: { value: number; className?: string }) {
  const safe = Math.max(0, Math.min(100, Number.isFinite(value) ? value : 0));
  return <div className={cn("h-2 overflow-hidden rounded-full bg-muted", className)} aria-label={`完成度 ${Math.round(safe)}%`} role="progressbar" aria-valuenow={safe} aria-valuemin={0} aria-valuemax={100}><div className="h-full rounded-full bg-primary transition-[width]" style={{ width: `${safe}%` }} /></div>;
}

export function EmptyState({ icon, title, description, action }: { icon?: ReactNode; title: string; description?: string; action?: ReactNode }) {
  return <div className="empty-state">{icon ? <div className="mb-3 text-muted-foreground">{icon}</div> : null}<div className="font-medium text-foreground">{title}</div>{description ? <p className="mt-1 max-w-md leading-6">{description}</p> : null}{action ? <div className="mt-4">{action}</div> : null}</div>;
}

export function PageHeader({ title, description, action }: { title: string; description?: string; action?: ReactNode }) {
  return <div className="page-header"><div><h1 className="page-title">{title}</h1>{description ? <p className="page-description">{description}</p> : null}</div>{action ? <div className="flex flex-wrap items-center gap-2">{action}</div> : null}</div>;
}

export function MetricCard({ label, value, hint, icon }: { label: string; value: ReactNode; hint?: ReactNode; icon?: ReactNode }) {
  return <Card className="min-w-0"><CardContent className="pt-5"><div className="flex items-start justify-between gap-3"><div className="min-w-0"><div className="text-xs font-medium text-muted-foreground">{label}</div><div className="metric-value mt-2 truncate">{value}</div>{hint ? <div className="mt-2 text-xs text-muted-foreground">{hint}</div> : null}</div>{icon ? <div className="rounded-md bg-muted p-2 text-muted-foreground">{icon}</div> : null}</div></CardContent></Card>;
}

export function Section({ title, description, action, children, className }: PropsWithChildren<{ title?: string; description?: string; action?: ReactNode; className?: string }>) {
  return <section className={className}>{title ? <div className="mb-3 flex items-end justify-between gap-4"><div><h2 className="section-title">{title}</h2>{description ? <p className="mt-1 text-xs leading-5 text-muted-foreground">{description}</p> : null}</div>{action}</div> : null}{children}</section>;
}
