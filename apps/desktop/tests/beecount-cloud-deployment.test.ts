import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const composePath = "../../deploy/cloud/docker-compose.production.yml";
const caddyPath = "../../deploy/cloud/Caddyfile.production";

test("production deployment exposes BeeCount through Caddy only", async () => {
  const compose = await readFile(composePath, "utf8");
  const service = compose.match(/\n  beecount-cloud:\n([\s\S]*?)\n  caddy:/)?.[1] ?? "";

  assert.match(service, /sunxiao0721\/beecount-cloud/);
  assert.match(service, /beecount_data:\/data/);
  assert.match(service, /127\.0\.0\.1:8080\/ready/);
  assert.match(service, /REGISTRATION_ENABLED: "false"/);
  assert.doesNotMatch(service, /^\s+ports:/m);
});

test("Caddy routes the dedicated finance host to BeeCount Cloud", async () => {
  const [compose, caddy] = await Promise.all([
    readFile(composePath, "utf8"),
    readFile(caddyPath, "utf8"),
  ]);

  assert.match(compose, /BEECOUNT_DOMAIN:/);
  assert.match(compose, /beecount-cloud:[\s\S]*?condition: service_started/);
  assert.match(caddy, /\{\$BEECOUNT_DOMAIN:finance\.8-148-75-45\.sslip\.io\}/);
  assert.match(caddy, /reverse_proxy beecount-cloud:8080/);
});
