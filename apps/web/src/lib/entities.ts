import type { CloudState, EntityType, JsonEntity } from "../services/core";

export function entities(state: CloudState, type: EntityType): JsonEntity[] {
  return Object.values(state.entities[type] ?? {}).filter((item) => !item.meta.deletedAt);
}

export function text(entity: JsonEntity | undefined, key: string, fallback = ""): string {
  if (!entity) return fallback;
  const value = entity[key];
  return typeof value === "string" ? value : fallback;
}

export function number(entity: JsonEntity | undefined, key: string, fallback = 0): number {
  if (!entity) return fallback;
  const value = entity[key];
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

export function boolean(entity: JsonEntity | undefined, key: string, fallback = false): boolean {
  if (!entity) return fallback;
  const value = entity[key];
  return typeof value === "boolean" ? value : fallback;
}

export function dateKey(value: unknown): string {
  if (typeof value !== "string" || !value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value.slice(0, 10);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function todayKey(): string {
  return dateKey(new Date().toISOString());
}

export function recentDays(days: number): string[] {
  return Array.from({ length: days }, (_, index) => {
    const date = new Date();
    date.setDate(date.getDate() - (days - index - 1));
    return dateKey(date.toISOString());
  });
}

export function formatDateTime(value: unknown): string {
  if (typeof value !== "string" || !value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" }).format(date);
}

export function sum(values: number[]): number {
  return values.reduce((total, value) => total + value, 0);
}

export function titleOf(entity: JsonEntity): string {
  for (const key of ["title", "name", "displayWord", "merchant", "item", "content", "summary"]) {
    const value = text(entity, key);
    if (value) return value;
  }
  return entity.meta.id.slice(0, 8);
}
