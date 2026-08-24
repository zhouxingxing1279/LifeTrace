import type {
  ButtonHTMLAttributes,
  InputHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from "react";
import { LoaderCircle, Search } from "lucide-react";

/* ------------------------------------------------------------------ */
/* Button                                                              */
/* ------------------------------------------------------------------ */

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type ButtonSize = "sm" | "md";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: ReactNode;
  loading?: boolean;
}

export function Button({
  variant = "secondary",
  size = "md",
  icon,
  loading,
  children,
  className,
  disabled,
  type = "button",
  ...rest
}: ButtonProps) {
  const classes = [
    "hx-btn",
    variant,
    size === "sm" ? "sm" : "",
    className ?? "",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <button
      type={type}
      className={classes}
      disabled={disabled || loading}
      {...rest}
    >
      {loading ? <LoaderCircle className="spin" aria-hidden="true" /> : icon}
      {children}
    </button>
  );
}

export interface IconButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
}

export function IconButton({
  label,
  children,
  className,
  ...rest
}: IconButtonProps) {
  return (
    <button
      type="button"
      className={`hx-icon-btn ${className ?? ""}`.trim()}
      aria-label={label}
      title={label}
      {...rest}
    >
      {children}
    </button>
  );
}

/* ------------------------------------------------------------------ */
/* Inputs                                                              */
/* ------------------------------------------------------------------ */

export function Input(props: InputHTMLAttributes<HTMLInputElement>) {
  return <input {...props} />;
}

export function Select(props: SelectHTMLAttributes<HTMLSelectElement>) {
  return <select {...props} />;
}

export function Textarea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea {...props} />;
}

export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <label className="lt-field">
      <span>{label}</span>
      {children}
      {hint ? <small className="lt-field-hint">{hint}</small> : null}
    </label>
  );
}

export function SearchInput({
  value,
  onChange,
  placeholder = "搜索",
  className,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
}) {
  return (
    <div className={`lt-search ${className ?? ""}`.trim()}>
      <Search aria-hidden="true" />
      <input
        type="search"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        aria-label={placeholder}
      />
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Badge & status                                                      */
/* ------------------------------------------------------------------ */

export type BadgeTone =
  | "neutral"
  | "success"
  | "warning"
  | "danger"
  | "info"
  | "primary";

export function Badge({
  tone = "neutral",
  dot = false,
  children,
  className,
}: {
  tone?: BadgeTone;
  dot?: boolean;
  children: ReactNode;
  className?: string;
}) {
  return (
    <span
      className={`lt-badge ${tone === "neutral" ? "" : tone} ${className ?? ""}`.trim()}
    >
      {dot ? <span className={`lt-dot ${tone}`} aria-hidden="true" /> : null}
      {children}
    </span>
  );
}

/* ------------------------------------------------------------------ */
/* Switch & checkbox                                                   */
/* ------------------------------------------------------------------ */

export function Switch({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: string;
}) {
  return (
    <label className="lt-switch" title={label}>
      <input
        type="checkbox"
        role="switch"
        checked={checked}
        aria-label={label}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span aria-hidden="true" />
    </label>
  );
}

export function Checkbox({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: string;
}) {
  return (
    <label className="lt-checkbox">
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span>{label}</span>
    </label>
  );
}

/* ------------------------------------------------------------------ */
/* Tabs                                                                */
/* ------------------------------------------------------------------ */

export interface TabItem<T extends string> {
  value: T;
  label: string;
}

export function Tabs<T extends string>({
  items,
  value,
  onChange,
  ariaLabel,
}: {
  items: TabItem<T>[];
  value: T;
  onChange: (value: T) => void;
  ariaLabel?: string;
}) {
  return (
    <div className="lt-tabs" role="tablist" aria-label={ariaLabel}>
      {items.map((item) => (
        <button
          key={item.value}
          type="button"
          role="tab"
          aria-selected={value === item.value}
          className={value === item.value ? "active" : ""}
          onClick={() => onChange(item.value)}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Skeleton, spinner, kbd, tooltip                                     */
/* ------------------------------------------------------------------ */

export function Skeleton({
  width = "100%",
  height = 14,
  radius = 6,
  className,
  style,
}: {
  width?: number | string;
  height?: number | string;
  radius?: number;
  className?: string;
  style?: React.CSSProperties;
}) {
  return (
    <span
      className={`lt-skeleton ${className ?? ""}`.trim()}
      style={{ width, height, borderRadius: radius, ...style }}
      aria-hidden="true"
    />
  );
}

export function Spinner({ label = "加载中" }: { label?: string }) {
  return (
    <span role="status" aria-label={label}>
      <LoaderCircle className="spin" aria-hidden="true" />
    </span>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return <kbd className="lt-kbd">{children}</kbd>;
}

export function Tooltip({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <span className="lt-tooltip" data-tooltip={label} tabIndex={0}>
      {children}
    </span>
  );
}
