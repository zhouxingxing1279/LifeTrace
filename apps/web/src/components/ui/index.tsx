import type { ButtonHTMLAttributes, HTMLAttributes, InputHTMLAttributes, PropsWithChildren, ReactNode, TextareaHTMLAttributes } from "react";
import { forwardRef } from "react";
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

export function Progress({ value, className }: { value: number; className?: string }) {
  const safe = Math.max(0, Math.min(100, Number.isFinite(value) ? value : 0));
  return <div className={cn("h-2 overflow-hidden rounded-full bg-muted", className)} aria-label={`完成度 ${Math.round(safe)}%`} role="progressbar" aria-valuenow={safe} aria-valuemin={0} aria-valuemax={100}>
    <div className="h-full rounded-full bg-primary transition-[width]" style={{ width: `${safe}%` }} />
  </div>;
}

export function EmptyState({ icon, title, description, action }: { icon?: ReactNode; title: string; description?: string; action?: ReactNode }) {
  return <div className="empty-state">
    {icon ? <div className="mb-3 text-muted-foreground">{icon}</div> : null}
    <div className="font-medium text-foreground">{title}</div>
    {description ? <p className="mt-1 max-w-md leading-6">{description}</p> : null}
    {action ? <div className="mt-4">{action}</div> : null}
  </div>;
}

export function PageHeader({ title, description, action }: { title: string; description?: string; action?: ReactNode }) {
  return <div className="page-header">
    <div>
      <h1 className="page-title">{title}</h1>
      {description ? <p className="page-description">{description}</p> : null}
    </div>
    {action ? <div className="flex flex-wrap items-center gap-2">{action}</div> : null}
  </div>;
}

export function MetricCard({ label, value, hint, icon }: { label: string; value: ReactNode; hint?: ReactNode; icon?: ReactNode }) {
  return <Card className="min-w-0">
    <CardContent className="pt-5">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-xs font-medium text-muted-foreground">{label}</div>
          <div className="metric-value mt-2 truncate">{value}</div>
          {hint ? <div className="mt-2 text-xs text-muted-foreground">{hint}</div> : null}
        </div>
        {icon ? <div className="rounded-md bg-muted p-2 text-muted-foreground">{icon}</div> : null}
      </div>
    </CardContent>
  </Card>;
}

export function Section({ title, description, action, children, className }: PropsWithChildren<{ title?: string; description?: string; action?: ReactNode; className?: string }>) {
  return <section className={className}>
    {title ? <div className="mb-3 flex items-end justify-between gap-4">
      <div>
        <h2 className="section-title">{title}</h2>
        {description ? <p className="mt-1 text-xs leading-5 text-muted-foreground">{description}</p> : null}
      </div>
      {action}
    </div> : null}
    {children}
  </section>;
}
