import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const composePath = "../../deploy/cloud/docker-compose.production.yml";
const caddyPath = "../../deploy/cloud/Caddyfile.production";

test("legacy BeeCount Cloud stays private as a rollback/data-export source", async () => {
  const compose = await readFile(composePath, "utf8");
  const service = compose.match(/\n  beecount-cloud:\n([\s\S]*?)\n  caddy:/)?.[1] ?? "";

  assert.match(service, /sunxiao0721\/beecount-cloud/);
  assert.match(service, /beecount_data:\/data/);
  assert.match(service, /127\.0\.0\.1:8080\/ready/);
  assert.match(service, /REGISTRATION_ENABLED: "false"/);
  assert.doesNotMatch(service, /^\s+ports:/m);
});

test("Caddy cuts stock BeeCount traffic over to the unified LifeTrace backend", async () => {
  const [compose, caddy] = await Promise.all([
    readFile(composePath, "utf8"),
    readFile(caddyPath, "utf8"),
  ]);

  assert.match(compose, /BEECOUNT_DOMAIN:/);
  const caddyService = compose.match(/\n  caddy:\n([\s\S]*?)\n\nvolumes:/)?.[1] ?? "";
  assert.match(caddyService, /lifetrace-cloud:[\s\S]*?condition: service_healthy/);
  assert.doesNotMatch(caddyService, /beecount-cloud:/);

  assert.match(caddy, /\{\$BEECOUNT_DOMAIN:finance\.8-148-75-45\.sslip\.io\}/);
  assert.match(caddy, /handle \/ready/);
  assert.match(caddy, /rewrite \* \/health\/ready/);
  assert.match(caddy, /handle \/api\/v1\/\*/);
  assert.match(caddy, /uri replace \/api\/v1\/ \/api\/v1\/integrations\/beecount\/compat\//);
  assert.match(caddy, /handle \/ws/);
  assert.match(caddy, /rewrite \* \/api\/v1\/integrations\/beecount\/compat\/ws/);
  assert.match(caddy, /reverse_proxy lifetrace-cloud:8787/);
  assert.doesNotMatch(caddy, /reverse_proxy beecount-cloud:8080/);
});
