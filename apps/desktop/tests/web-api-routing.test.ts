import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { normalizeApiBase } from "../web-client/src/cloud/base";

const repo = (path: string) => readFileSync(new URL(`../../../${path}`, import.meta.url), "utf8");

test("production browser API defaults to same-origin instead of public port 8787", () => {
  assert.equal(normalizeApiBase(undefined), "");
  assert.equal(normalizeApiBase(""), "");
  assert.equal(normalizeApiBase("   "), "");
  assert.equal(normalizeApiBase("https://cloud.example.test/"), "https://cloud.example.test");

  const base = repo("apps/desktop/web-client/src/cloud/base.ts");
  assert.doesNotMatch(base, /window\.location\.hostname.*8787/);
  assert.doesNotMatch(base, /window\.location\.host.*8787/);
});

test("local browser development proxies same-origin API paths to the Cloud listener", () => {
  const vite = repo("apps/desktop/vite.browser.config.ts");
  assert.match(vite, /DEFAULT_LIFETRACE_CLOUD_URL = "http:\/\/127\.0\.0\.1:8787"/);
  assert.match(vite, /"\/api"\s*:\s*\{/);
  assert.match(vite, /"\/health"\s*:\s*\{/);
  assert.match(vite, /server:\s*\{[\s\S]*proxy:\s*cloudProxy/);
  assert.match(vite, /preview:\s*\{[\s\S]*proxy:\s*cloudProxy/);
});

test("production Caddy remains the public API boundary", () => {
  const caddy = repo("deploy/cloud/Caddyfile.production");
  const compose = repo("deploy/cloud/docker-compose.production.yml");
  assert.match(caddy, /handle \/api\/\* \{[\s\S]*reverse_proxy lifetrace-cloud:8787/);
  assert.match(caddy, /handle \/health\/\* \{[\s\S]*reverse_proxy lifetrace-cloud:8787/);
  assert.doesNotMatch(compose, /8787:8787/);
});
