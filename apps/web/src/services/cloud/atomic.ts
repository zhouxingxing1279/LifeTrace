import { uuid, type CloudState, type EntityType, type JsonEntity, type PushResult, type SyncChange } from "./types";
import type { CloudDataStore } from "./api";

export type AtomicMutation =
  | { operation: "upsert"; entityType: EntityType; entity: JsonEntity }
  | { operation: "delete"; entityType: EntityType; entityId: string };

interface PreparedUpsert { entity: JsonEntity; change: SyncChange; }
interface AtomicStoreInternals {
  prepareUpsert(entityType: EntityType, entity: JsonEntity): PreparedUpsert;
  prepareDelete(entityType: EntityType, entityId: string): SyncChange;
  push(changes: SyncChange[]): Promise<PushResult[]>;
  put(entityType: EntityType, entity: JsonEntity): void;
  remove(entityType: EntityType, entityId: string): void;
  state: CloudState;
  snapshot(): CloudState;
}

/**
 * Push a heterogeneous set of mutations as one sync atomic group.
 *
 * Both the in-memory protocol store and the production PostgreSQL repository
 * already enforce `atomicGroupId` transaction semantics. This helper activates
 * that existing protocol capability for browser workflows such as Goal + first
 * Project creation and conversion flows that must never be partially saved.
 */
export async function atomicMutate(store: CloudDataStore, mutations: AtomicMutation[]): Promise<CloudState> {
  if (mutations.length < 2) throw new Error("原子写入至少需要两个变更");
  const internals = store as unknown as AtomicStoreInternals;
  const atomicGroupId = uuid();
  const prepared = mutations.map((mutation) => {
    if (mutation.operation === "upsert") {
      const value = internals.prepareUpsert(mutation.entityType, mutation.entity);
      value.change.atomicGroupId = atomicGroupId;
      return { mutation, change: value.change, entity: value.entity };
    }
    const change = internals.prepareDelete(mutation.entityType, mutation.entityId);
    change.atomicGroupId = atomicGroupId;
    return { mutation, change, entity: null };
  });

  const results = await internals.push(prepared.map((item) => item.change));
  const byChangeId = new Map(results.map((result) => [result.changeId, result]));
  for (const item of prepared) {
    const result = byChangeId.get(item.change.changeId);
    if (!result) throw new Error(`原子写入缺少返回结果：${item.change.entityId}`);
    if (result.status !== "accepted" && result.status !== "duplicate") {
      throw new Error(result.message || result.reason || "原子写入失败，整组变更已回滚");
    }
  }

  for (const item of prepared) {
    const result = byChangeId.get(item.change.changeId)!;
    if (item.mutation.operation === "upsert" && item.entity) {
      item.entity.meta.serverVersion = result.serverVersion ?? item.entity.meta.serverVersion ?? "0";
      internals.put(item.mutation.entityType, item.entity);
    } else if (item.mutation.operation === "delete") {
      internals.remove(item.mutation.entityType, item.mutation.entityId);
    }
  }
  internals.state.lastLoadedAt = new Date().toISOString();
  return internals.snapshot();
}
