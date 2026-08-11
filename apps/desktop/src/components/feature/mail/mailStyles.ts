import type { CSSProperties } from "react";

export const panelStyle: CSSProperties = {
  border: "1px solid var(--line, rgba(128,128,128,.18))",
  borderRadius: 12,
  background: "var(--panel, rgba(255,255,255,.72))",
  overflow: "hidden",
};

export const inputStyle: CSSProperties = {
  width: "100%",
  minHeight: 38,
  border: "1px solid var(--line, rgba(128,128,128,.25))",
  borderRadius: 9,
  background: "var(--surface, transparent)",
  color: "inherit",
  padding: "8px 10px",
  font: "inherit",
  boxSizing: "border-box",
};

export const actionButton: CSSProperties = {
  minHeight: 34,
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  gap: 6,
  border: "1px solid var(--line, rgba(128,128,128,.22))",
  borderRadius: 9,
  background: "transparent",
  color: "inherit",
  padding: "6px 10px",
  cursor: "pointer",
};

export function toast(message: string, type: "success" | "error" = "success") {
  window.dispatchEvent(new CustomEvent("hengxu-toast", {
    detail: { message, type, duration: type === "error" ? 4500 : 2500 },
  }));
}

export function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "邮件操作失败";
}

export function formatTime(value?: string | null) {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const today = new Date();
  const sameDay = date.getFullYear() === today.getFullYear()
    && date.getMonth() === today.getMonth()
    && date.getDate() === today.getDate();
  return new Intl.DateTimeFormat("zh-CN", sameDay
    ? { hour: "2-digit", minute: "2-digit" }
    : { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" }).format(date);
}

export function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}
