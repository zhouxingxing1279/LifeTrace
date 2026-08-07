import { API_BASE } from "./base";
import { browserFetch } from "./http";
import type { CloudState, FetchLike, JsonEntity } from "./types";

export interface AssistantReply {
  reply: string;
  provider: "deepseek" | "local";
}

function compactEntity(entity: JsonEntity): Record<string, unknown> {
  const entries = Object.entries(entity)
    .filter(([key]) => key !== "contentHtml" && key !== "contentJson")
    .slice(0, 24);
  return Object.fromEntries(entries);
}

export function buildAssistantContext(state: CloudState): Record<string, unknown> {
  const context: Record<string, unknown> = {};
  for (const [entityType, collection] of Object.entries(state.entities)) {
    if (!collection) continue;
    context[entityType] = Object.values(collection)
      .sort((left, right) => right.meta.updatedAt.localeCompare(left.meta.updatedAt))
      .slice(0, 40)
      .map(compactEntity);
  }
  return context;
}

export class AssistantApi {
  constructor(private readonly fetcher: FetchLike = browserFetch) {}

  async ask(prompt: string, state: CloudState, csrfToken: string): Promise<AssistantReply> {
    const response = await this.fetcher(`${API_BASE}/api/v1/web/assistant`, {
      method: "POST",
      credentials: "include",
      headers: { "content-type": "application/json", "x-csrf-token": csrfToken },
      body: JSON.stringify({ prompt: prompt.trim(), context: buildAssistantContext(state) }),
    });
    const payload = await response.json() as Partial<AssistantReply> & { message?: string; error?: { message?: string } };
    if (!response.ok) throw new Error(payload.message || payload.error?.message || `AI 请求失败 (${response.status})`);
    if (!payload.reply) throw new Error("AI 服务未返回内容");
    return { reply: payload.reply, provider: payload.provider === "deepseek" ? "deepseek" : "local" };
  }
}
