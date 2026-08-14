import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repoUrl = (path: string) => new URL(`../../../${path}`, import.meta.url);
const repo = (path: string) => readFileSync(repoUrl(path), "utf8");

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

test("production deploy script updates main, deploys images and verifies health", () => {
  const scriptPath = fileURLToPath(repoUrl("deploy/cloud/deploy-production.sh"));
  const script = readFileSync(scriptPath, "utf8");
  const syntax = spawnSync("bash", ["-n", scriptPath], { encoding: "utf8" });

  assert.equal(syntax.status, 0, syntax.stderr || syntax.stdout);
  assert.match(script, /git -C "\$\{REPO_ROOT\}" pull --ff-only origin main/);
  assert.match(script, /compose config --quiet/);
  assert.match(script, /compose pull/);
  assert.match(script, /compose up -d --remove-orphans/);
  assert.match(script, /wait_for_migration/);
  assert.match(script, /wait_for_service lifetrace-cloud true/);
  assert.match(script, /wait_for_service lifetrace-execution-worker true/);
  assert.match(script, /--skip-git-update/);
  assert.doesNotMatch(script, /npm (ci|run)/);
});
