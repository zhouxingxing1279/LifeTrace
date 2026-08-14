import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const repo = (path: string) => readFileSync(new URL(`../../../${path}`, import.meta.url), "utf8");

test("production web image packages browser, photo challenge and Caddy", () => {
  const dockerfile = repo("deploy/cloud/Dockerfile.web");
  assert.match(dockerfile, /FROM node:22-alpine AS browser-builder/);
  assert.match(dockerfile, /RUN npm ci/);
  assert.match(dockerfile, /RUN npm run browser:build/);
  assert.match(dockerfile, /FROM caddy:2-alpine/);
  assert.match(dockerfile, /dist-browser \/srv/);
  assert.match(dockerfile, /apps\/photo-challenge-pwa \/srv-photo-challenge/);
  assert.match(dockerfile, /Caddyfile\.production \/etc\/caddy\/Caddyfile/);
});

test("production compose uses only packaged LifeTrace services", () => {
  const production = repo("deploy/cloud/docker-compose.production.yml");
  assert.match(production, /ghcr\.io\/zhouxingxing1279\/lifetrace-web:main/);
  assert.match(production, /lifetrace-execution-worker:/);
  assert.doesNotMatch(production, /\.\.\/\.\.\/apps\/desktop\/dist-browser/);
  assert.doesNotMatch(production, /\.\.\/\.\.\/apps\/photo-challenge-pwa/);
  assert.doesNotMatch(production, /sunxiao0721\/beecount-cloud/);
  assert.doesNotMatch(production, /^\s{2}beecount-cloud:/m);
  assert.doesNotMatch(production, /^\s{2}beecount_data:/m);
});

test("WSL compatibility stack no longer pulls legacy BeeCount Cloud", () => {
  const wsl = repo("deploy/cloud/docker-compose.wsl.yml");
  assert.doesNotMatch(wsl, /sunxiao0721\/beecount-cloud/);
  assert.doesNotMatch(wsl, /^\s{2}beecount-cloud:/m);
  assert.match(wsl, /Caddyfile\.wsl/);
});

test("CI publishes a dedicated LifeTrace Web image", () => {
  const workflow = repo(".github/workflows/web-image.yml");
  assert.match(workflow, /IMAGE_NAME: \$\{\{ github\.repository_owner \}\}\/lifetrace-web/);
  assert.match(workflow, /file: deploy\/cloud\/Dockerfile\.web/);
  assert.match(workflow, /type=raw,value=main/);
  assert.match(workflow, /apps\/photo-challenge-pwa\/\*\*/);
});
