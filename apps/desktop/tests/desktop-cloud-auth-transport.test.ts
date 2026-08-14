import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

const root = resolve(import.meta.dirname, "..");
const authClient = readFileSync(resolve(root, "src/services/cloudAuth.ts"), "utf8");
const authCommand = readFileSync(resolve(root, "src-tauri/src/cloud_auth.rs"), "utf8");
const tauriLib = readFileSync(resolve(root, "src-tauri/src/lib.rs"), "utf8");

test("packaged desktop cloud auth uses Tauri native HTTP instead of WebView CORS", () => {
  assert.match(authClient, /invoke<NativeCloudAuthResponse>\("cloud_auth_http_request"/);
  assert.match(authClient, /if \(!isTauriRuntime\(\)\) return fetch\(input, init\);/);
  assert.equal((authClient.match(/\bfetch\(/g) ?? []).length, 1, "only the non-Tauri fallback may call browser fetch");
  assert.match(tauriLib, /cloud_auth::cloud_auth_http_request/);
});

test("native auth transport is constrained to authentication endpoints", () => {
  assert.match(authCommand, /const AUTH_PATHS: &\[&str\]/);
  assert.match(authCommand, /"\/api\/v1\/auth\/login"/);
  assert.match(authCommand, /"\/api\/v1\/auth\/refresh"/);
  assert.match(authCommand, /AUTH_PATHS\.contains\(&path\)/);
  assert.match(authCommand, /redirect\(Policy::none\(\)\)/);
});
