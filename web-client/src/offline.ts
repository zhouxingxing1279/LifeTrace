import type { PersistedState, StorageLike } from "./core";

const DB_NAME = "lifetrace-web";
const DB_VERSION = 1;
const STATE_STORE = "state";
const ATTACHMENT_STORE = "attachments";

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onerror = () => reject(request.error ?? new Error("无法打开 IndexedDB"));
    request.onupgradeneeded = () => {
      const database = request.result;
      if (!database.objectStoreNames.contains(STATE_STORE)) database.createObjectStore(STATE_STORE);
      if (!database.objectStoreNames.contains(ATTACHMENT_STORE)) database.createObjectStore(ATTACHMENT_STORE);
    };
    request.onsuccess = () => resolve(request.result);
  });
}

async function idbGet<T>(storeName: string, key: string): Promise<T | null> {
  const database = await openDatabase();
  return new Promise((resolve, reject) => {
    const transaction = database.transaction(storeName, "readonly");
    const request = transaction.objectStore(storeName).get(key);
    request.onsuccess = () => resolve((request.result as T | undefined) ?? null);
    request.onerror = () => reject(request.error);
    transaction.oncomplete = () => database.close();
  });
}

async function idbSet(storeName: string, key: string, value: unknown) {
  const database = await openDatabase();
  return new Promise<void>((resolve, reject) => {
    const transaction = database.transaction(storeName, "readwrite");
    transaction.objectStore(storeName).put(value, key);
    transaction.oncomplete = () => { database.close(); resolve(); };
    transaction.onerror = () => { database.close(); reject(transaction.error); };
  });
}

async function idbDelete(storeName: string, key: string) {
  const database = await openDatabase();
  return new Promise<void>((resolve, reject) => {
    const transaction = database.transaction(storeName, "readwrite");
    transaction.objectStore(storeName).delete(key);
    transaction.oncomplete = () => { database.close(); resolve(); };
    transaction.onerror = () => { database.close(); reject(transaction.error); };
  });
}

export class MemoryStorage implements StorageLike {
  private readonly values = new Map<string, string>();
  getItem(key: string) { return this.values.get(key) ?? null; }
  setItem(key: string, value: string) { this.values.set(key, value); }
  removeItem(key: string) { this.values.delete(key); }
}

/**
 * WebSyncStore remains synchronous for deterministic optimistic UI. This
 * adapter keeps a localStorage mirror and writes the authoritative cache and
 * outbox to IndexedDB. `hydrate` must be awaited before constructing the
 * store so a browser restart restores the latest durable state.
 */
export class IndexedDbStateStorage implements StorageLike {
  private readonly mirror: StorageLike;
  constructor(private readonly userId: string, mirror: StorageLike = localStorage) {
    this.mirror = mirror;
  }

  private stateKey() { return `lifetrace:web:${this.userId}`; }

  async hydrate() {
    if (!globalThis.indexedDB) return;
    const state = await idbGet<PersistedState>(STATE_STORE, this.stateKey());
    if (state) this.mirror.setItem(this.stateKey(), JSON.stringify(state));
  }

  getItem(key: string) { return this.mirror.getItem(key); }

  setItem(key: string, value: string) {
    this.mirror.setItem(key, value);
    if (globalThis.indexedDB) {
      let state: unknown = value;
      try { state = JSON.parse(value); } catch { /* preserve string */ }
      void idbSet(STATE_STORE, key, state);
    }
  }

  removeItem(key: string) {
    this.mirror.removeItem(key);
    if (globalThis.indexedDB) void idbDelete(STATE_STORE, key);
  }
}

export interface StoredAttachment {
  id: string;
  userId: string;
  noteId: string;
  name: string;
  mimeType: string;
  size: number;
  sha256: string;
  blob: Blob;
  createdAt: string;
}

export class AttachmentStore {
  async put(attachment: StoredAttachment) {
    if (!globalThis.indexedDB) throw new Error("当前浏览器不支持离线附件存储");
    await idbSet(ATTACHMENT_STORE, attachment.id, attachment);
  }

  get(id: string) { return idbGet<StoredAttachment>(ATTACHMENT_STORE, id); }
  remove(id: string) { return idbDelete(ATTACHMENT_STORE, id); }

  async list(ids: string[]) {
    const values = await Promise.all(ids.map((id) => this.get(id)));
    return values.filter((value): value is StoredAttachment => Boolean(value));
  }
}
