import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

import { CloudAuthClient, type CloudTokenResponse } from "../src/services/cloudAuth";

class MemoryStorage {
  private values = new Map<string, string>();
  getItem(key: string) { return this.values.get(key) ?? null; }
  setItem(key: string, value: string) { this.values.set(key, String(value)); }
  removeItem(key: string) { this.values.delete(key); }
  clear() { this.values.clear(); }
  key(index: number) { return [...this.values.keys()][index] ?? null; }
  get length() { return this.values.size; }
}

const tokenFor = (userId: string): CloudTokenResponse => ({
  accessToken: `access-${userId}`,
  refreshToken: `refresh-${userId}`,
  tokenType: "Bearer",
  expiresIn: 900,
  refreshExpiresIn: 86400,
  user: { id: userId, email: `${userId}@example.com`, displayName: userId.toUpperCase(), state: "active" },
  session: {
    id: `session-${userId}`,
    appId: "lifetrace-desktop",
    deviceId: "device",
    status: "active",
    createdAt: new Date(0).toISOString(),
    lastSeenAt: new Date(0).toISOString(),
    absoluteExpiresAt: new Date(86400000).toISOString(),
  },
  scopes: [],
});

function installBrowserMocks(syncApi: NonNullable<Window["syncApi"]>, token: CloudTokenResponse) {
  const storage = new MemoryStorage() as unknown as Storage;
  const originalWindow = globalThis.window;
  const originalFetch = globalThis.fetch;
  const originalLocalStorage = globalThis.localStorage;
  const credentials = { value: null as string | null };
  const mockedWindow = {
    localStorage: storage,
    cloudCredentialApi: {
      async set(value: string) { credentials.value = value; },
      async get() { return credentials.value; },
      async clear() { credentials.value = null; },
    },
    syncApi,
  } as unknown as Window & typeof globalThis;
  Object.defineProperty(globalThis, "window", { configurable: true, value: mockedWindow });
  Object.defineProperty(globalThis, "localStorage", { configurable: true, value: storage });
  globalThis.fetch = async () => new Response(JSON.stringify(token), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
  return () => {
    Object.defineProperty(globalThis, "window", { configurable: true, value: originalWindow });
    Object.defineProperty(globalThis, "localStorage", { configurable: true, value: originalLocalStorage });
    globalThis.fetch = originalFetch;
  };
}

test("cloud auth module does not persist access or refresh tokens in web storage", async () => {
  const source = await readFile("src/services/cloudAuth.ts", "utf8");
  assert.equal(source.includes('localStorage.setItem("access'), false);
  assert.equal(source.includes('localStorage.setItem("refresh'), false);
  assert.match(source, /cloudCredentialApi/);
});

test("cloud auth exposes registration and capability discovery", async () => {
  const source = await readFile("src/services/cloudAuth.ts", "utf8");
  assert.match(source, /\/api\/v1\/auth\/register/);
  assert.match(source, /\/api\/v1\/auth\/capabilities/);
});

test("cloud auth client starts unauthenticated with tokens outside serializable state", () => {
  const client = new CloudAuthClient();
  assert.deepEqual(client.state(), { scopes: [], authenticated: false });
  assert.equal(JSON.stringify(client.state()).includes("Token"), false);
});

test("existing account switches to its own profile before establishing sync session", async () => {
  const calls: string[] = [];
  let active = "local";
  const profiles = [
    { id: "profile-a", displayName: "A", cloudUserId: "user-a", cloudBindingState: "bound", active: false, createdAt: "", updatedAt: "" },
    { id: "profile-b", displayName: "B", cloudUserId: "user-b", cloudBindingState: "bound", active: false, createdAt: "", updatedAt: "" },
    { id: "local", displayName: "Local", cloudUserId: null, cloudBindingState: "local_only", active: true, createdAt: "", updatedAt: "" },
  ];
  const syncApi = {
    async profiles() { calls.push("profiles"); return profiles; },
    async setActiveProfile(profileId: string) { calls.push(`active:${profileId}`); active = profileId; },
    async setSession() { calls.push(`session:${active}`); return { profileId: active, cloudUserId: "user-a", bindingRequired: false, alreadyBound: true }; },
    async clearSession() {}, async bindCurrentProfile() { return active; }, async createCloudProfile() { throw new Error("should not create"); },
    async status() { throw new Error("unused"); }, async now() { throw new Error("unused"); }, async conflicts() { return []; }, async resolveConflict() {},
  } as NonNullable<Window["syncApi"]>;
  const restore = installBrowserMocks(syncApi, tokenFor("user-a"));
  try {
    const client = new CloudAuthClient();
    client.configure("https://cloud.example.com");
    await client.login("a@example.com", "password");
    assert.deepEqual(calls.slice(0, 3), ["profiles", "active:profile-a", "session:profile-a"]);
    assert.equal(client.state().binding?.profileId, "profile-a");
  } finally { restore(); }
});

test("new account creates a new profile instead of taking another user's profile", async () => {
  const calls: string[] = [];
  let active = "profile-a";
  const profiles = [
    { id: "profile-a", displayName: "A", cloudUserId: "user-a", cloudBindingState: "bound", active: true, createdAt: "", updatedAt: "" },
    { id: "local", displayName: "Local", cloudUserId: null, cloudBindingState: "local_only", active: false, createdAt: "", updatedAt: "" },
  ];
  const syncApi = {
    async profiles() { calls.push("profiles"); return profiles; },
    async setActiveProfile(profileId: string) { calls.push(`active:${profileId}`); active = profileId; },
    async setSession() {
      calls.push(`session:${active}`);
      return { profileId: active, cloudUserId: "user-c", bindingRequired: active === "local", alreadyBound: active === "profile-c" };
    },
    async createCloudProfile() { calls.push("create:user-c"); active = "profile-c"; return active; },
    async clearSession() {}, async bindCurrentProfile() { return active; },
    async status() { throw new Error("unused"); }, async now() { throw new Error("unused"); }, async conflicts() { return []; }, async resolveConflict() {},
  } as NonNullable<Window["syncApi"]>;
  const restore = installBrowserMocks(syncApi, tokenFor("user-c"));
  try {
    const client = new CloudAuthClient();
    client.configure("https://cloud.example.com");
    await client.login("c@example.com", "password");
    assert.deepEqual(calls.slice(0, 5), ["profiles", "active:local", "session:local", "create:user-c", "session:profile-c"]);
    assert.equal(calls.includes("active:profile-a"), false);
    assert.equal(client.state().binding?.profileId, "profile-c");
  } finally { restore(); }
});
