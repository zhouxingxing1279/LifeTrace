import { entityText, type CloudState, type EntityType, type JsonEntity, type SearchHit } from "./types";

export function searchEntities(state: CloudState, query: string): SearchHit[] {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return [];
  const hits: SearchHit[] = [];
  const add = (entityType: EntityType, entity: JsonEntity, title: string, subtitle: string, route: string) => {
    if (`${title} ${subtitle}`.toLocaleLowerCase().includes(needle)) hits.push({ id: entity.meta.id, entityType, title: title || "未命名记录", subtitle, updatedAt: entity.meta.updatedAt, route });
  };
  for (const entity of Object.values(state.entities["execution.goal"] ?? {})) add("execution.goal", entity, entityText(entity, "name") || "目标", `${entityText(entity, "description")} ${entityText(entity, "status")}`, "/execution/goals");
  for (const entity of Object.values(state.entities["execution.task"] ?? {})) add("execution.task", entity, entityText(entity, "title") || "任务", `${entityText(entity, "description")} ${entityText(entity, "context")} ${entityText(entity, "priority")}`, "/execution");
  for (const entity of Object.values(state.entities["execution.project"] ?? {})) add("execution.project", entity, entityText(entity, "name") || "计划", entityText(entity, "description"), "/execution");
  for (const entity of Object.values(state.entities["execution.memo"] ?? {})) add("execution.memo", entity, entityText(entity, "plainText") || entityText(entity, "content") || "备忘", entityText(entity, "context"), "/execution");
  for (const entity of Object.values(state.entities["execution.waiting_item"] ?? {})) add("execution.waiting_item", entity, entityText(entity, "title") || "等待事项", `${entityText(entity, "description")} ${entityText(entity, "waitingFor")}`, "/execution");
  for (const entity of Object.values(state.entities["habit.activity"] ?? {})) add("habit.activity", entity, entityText(entity, "name") || "坚持项目", `${entityText(entity, "description")} ${entityText(entity, "unit")}`, "/habits");
  for (const entity of Object.values(state.entities["habit.log"] ?? {})) add("habit.log", entity, `坚持记录 ${entityText(entity, "logDate")}`, entityText(entity, "note"), "/habits");
  for (const entity of Object.values(state.entities["workout.workout"] ?? {})) add("workout.workout", entity, entityText(entity, "name") || "训练记录", `${entityText(entity, "localDate")} ${entityText(entity, "source")}`, "/fitness");
  for (const entity of Object.values(state.entities["workout.training_note"] ?? {})) add("workout.training_note", entity, entityText(entity, "title") || "训练笔记", entityText(entity, "content"), "/fitness");
  for (const entity of Object.values(state.entities["finance.transaction"] ?? {})) add("finance.transaction", entity, entityText(entity, "merchant") || entityText(entity, "item") || entityText(entity, "note") || "财务流水", `${entityText(entity, "localDate")} ${entityText(entity, "counterparty")}`, "/finance/transactions");
  for (const entity of Object.values(state.entities["finance.account"] ?? {})) add("finance.account", entity, entityText(entity, "name") || "资金账户", `${entityText(entity, "accountType")} ${entityText(entity, "last4")}`, "/finance/accounts");
  for (const entity of Object.values(state.entities["note.note"] ?? {})) add("note.note", entity, entityText(entity, "title") || "无标题笔记", entityText(entity, "contentText") || entityText(entity, "summary"), "/notes");
  for (const entity of Object.values(state.entities["english.article"] ?? {})) add("english.article", entity, entityText(entity, "title") || "English article", entityText(entity, "summary") || entityText(entity, "content"), "/english/articles");
  for (const entity of Object.values(state.entities["english.vocabulary"] ?? {})) add("english.vocabulary", entity, entityText(entity, "displayWord"), `${entityText(entity, "definition")} ${entityText(entity, "notes")}`, "/english/vocabulary");
  for (const entity of Object.values(state.entities["english.learning_record"] ?? {})) add("english.learning_record", entity, `英语阅读 ${entityText(entity, "recordDate")}`, entityText(entity, "summary"), "/english/stats");
  for (const entity of Object.values(state.entities["review.daily"] ?? {})) add("review.daily", entity, `每日复盘 ${entityText(entity, "reviewDate")}`, `${entityText(entity, "bestThing")} ${entityText(entity, "problem")} ${entityText(entity, "tomorrowPriority")} ${entityText(entity, "note")}`, "/review");
  return hits.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)).slice(0, 80);
}

export function findProbableDuplicate(transaction: JsonEntity, existing: JsonEntity[]): JsonEntity | null {
  const externalId = entityText(transaction, "externalTransactionId");
  if (externalId) {
    const exact = existing.find((item) => entityText(item, "externalTransactionId") === externalId);
    if (exact) return exact;
  }
  const amount = Number(transaction.amountCents ?? 0);
  const date = entityText(transaction, "localDate");
  const merchant = entityText(transaction, "merchant").toLocaleLowerCase();
  return existing.find((item) => {
    if (Number(item.amountCents ?? 0) !== amount || entityText(item, "localDate") !== date) return false;
    const candidate = entityText(item, "merchant").toLocaleLowerCase();
    return !merchant || !candidate || merchant === candidate;
  }) ?? null;
}
