import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const composePath = "../../deploy/cloud/docker-compose.production.yml";
const wslComposePath = "../../deploy/cloud/docker-compose.wsl.yml";
const caddyPath = "../../deploy/cloud/Caddyfile.production";

test("legacy BeeCount Cloud is absent from active deployment stacks", async () => {
  const [production, wsl] = await Promise.all([
    readFile(composePath, "utf8"),
    readFile(wslComposePath, "utf8"),
  ]);

  for (const compose of [production, wsl]) {
    assert.doesNotMatch(compose, /sunxiao0721\/beecount-cloud/);
    assert.doesNotMatch(compose, /^\s{2}beecount-cloud:/m);
    assert.doesNotMatch(compose, /beecount_data:\/data/);
  }

  assert.match(production, /ghcr\.io\/zhouxingxing1279\/lifetrace-web:main/);
});

test("Caddy exposes stock BeeCount compatibility on the direct public IP", async () => {
  const [compose, caddy] = await Promise.all([
    readFile(composePath, "utf8"),
    readFile(caddyPath, "utf8"),
  ]);

  const caddyService = compose.match(/\n  caddy:\n([\s\S]*?)\n\nvolumes:/)?.[1] ?? "";
  assert.match(caddyService, /"8869:8869"/);
  assert.match(caddyService, /lifetrace-cloud:[\s\S]*?condition: service_healthy/);
  assert.match(caddyService, /wget[\s\S]*?127\.0\.0\.1:2019\/config\//);
  assert.doesNotMatch(caddyService, /BEECOUNT_DOMAIN:/);
  assert.doesNotMatch(caddyService, /beecount-cloud:/);

  assert.match(caddy, /https:\/\/8\.148\.75\.45:8869/);
  assert.match(caddy, /issuer acme https:\/\/acme-v02\.api\.letsencrypt\.org\/directory/);
  assert.match(caddy, /profile shortlived/);
  assert.match(caddy, /disable_tlsalpn_challenge/);
  assert.doesNotMatch(caddy, /sslip\.io/);
  assert.match(caddy, /handle \/ready/);
  assert.match(caddy, /rewrite \* \/health\/ready/);
  assert.match(caddy, /handle \/api\/v1\/\*/);
  assert.match(caddy, /uri replace \/api\/v1\/ \/api\/v1\/integrations\/beecount\/compat\//);
  assert.match(caddy, /handle \/ws/);
  assert.match(caddy, /rewrite \* \/api\/v1\/integrations\/beecount\/compat\/ws/);
  assert.match(caddy, /reverse_proxy lifetrace-cloud:8787/);
  assert.doesNotMatch(caddy, /reverse_proxy beecount-cloud:8080/);
});
