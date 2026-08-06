import { entityText, type CloudState, type EntityType, type JsonEntity, type SearchHit } from "./types";

export function searchEntities(state: CloudState, query: string): SearchHit[] {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return [];
  const hits: SearchHit[] = [];
  const add = (entityType: EntityType, entity: JsonEntity, title: string, subtitle: string, route: string) => {
    if (`${title} ${subtitle}`.toLocaleLowerCase().includes(needle)) hits.push({ id: entity.meta.id, entityType, title: title || "未命名记录", subtitle, updatedAt: entity.meta.updatedAt, route });
  };
  for (const entity of Object.values(state.entities["finance.transaction"] ?? {})) add("finance.transaction", entity, entityText(entity, "merchant") || entityText(entity, "item") || entityText(entity, "note") || "财务流水", `${entityText(entity, "localDate")} ${entityText(entity, "counterparty")}`, "/finance/transactions");
  for (const entity of Object.values(state.entities["note.note"] ?? {})) add("note.note", entity, entityText(entity, "title") || "无标题笔记", entityText(entity, "contentText") || entityText(entity, "summary"), "/notes");
  for (const entity of Object.values(state.entities["english.article"] ?? {})) add("english.article", entity, entityText(entity, "title") || "English article", entityText(entity, "summary") || entityText(entity, "content"), "/english/articles");
  for (const entity of Object.values(state.entities["english.vocabulary"] ?? {})) add("english.vocabulary", entity, entityText(entity, "displayWord"), entityText(entity, "definition"), "/english/vocabulary");
  return hits.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)).slice(0, 50);
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
