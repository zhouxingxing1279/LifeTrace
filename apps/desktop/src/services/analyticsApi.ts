import { instrumentedFetch } from "@/src/services/clientObservability";
import type {
  InsightSnapshot,
  ProjectionStatus,
  ReportSnapshot,
  SearchHit,
  TimelinePage,
} from "@/src/types/analytics";

type ApiErrorPayload = { error?: string };

async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const method = (init?.method || "GET").toUpperCase();
  const response = await instrumentedFetch(globalThis.fetch, url, init, {
    module: "analytics",
    action: `${method} ${url.split("?", 1)[0]}`,
    userMessage: "分析服务请求失败",
  });
  const raw = await response.text();
  let payload: unknown = null;
  if (raw) {
    try {
      payload = JSON.parse(raw);
    } catch {
      payload = raw;
    }
  }
  if (!response.ok) {
    const error = payload as ApiErrorPayload | string | null;
    const message = typeof error === "string" ? error : error?.error;
    throw new Error(message || `分析服务请求失败（${response.status}）`);
  }
  return payload as T;
}

function query(path: string, values: Record<string, string | number | null | undefined>) {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(values)) {
    if (value !== undefined && value !== null && value !== "") params.set(key, String(value));
  }
  const suffix = params.toString();
  return suffix ? `${path}?${suffix}` : path;
}

export const analyticsApi = {
  status: () => request<ProjectionStatus>("/api/analytics/status"),
  rebuild: () => request<ProjectionStatus>("/api/analytics/rebuild", { method: "POST" }),
  timeline: (options: {
    from?: string;
    to?: string;
    domain?: string;
    eventType?: string;
    keyword?: string;
    cursor?: string;
    limit?: number;
  } = {}) => request<TimelinePage>(query("/api/analytics/timeline", options)),
  search: (options: {
    q: string;
    domain?: string;
    from?: string;
    to?: string;
    limit?: number;
  }) => request<SearchHit[]>(query("/api/analytics/search", options)),
  report: (options: {
    reportType: "weekly" | "monthly" | "custom";
    periodStart: string;
    periodEnd: string;
    timezone: string;
  }) => request<ReportSnapshot>(query("/api/analytics/report", options)),
  insights: (options: { periodStart: string; periodEnd: string }) =>
    request<InsightSnapshot[]>(query("/api/analytics/insights", options)),
};
