import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repoUrl = (path: string) => new URL(`../../../${path}`, import.meta.url);
const repo = (path: string) => readFileSync(repoUrl(path), "utf8");

test("production web image packages browser, photo challenge and HTTP Caddy", () => {
  const dockerfile = repo("deploy/cloud/Dockerfile.web");
  assert.match(dockerfile, /FROM node:22-alpine AS browser-builder/);
  assert.match(dockerfile, /RUN npm ci/);
  assert.match(dockerfile, /RUN npm run browser:build/);
  assert.match(dockerfile, /FROM caddy:2\.11\.4-alpine/);
  assert.match(dockerfile, /dist-browser \/srv/);
  assert.match(dockerfile, /apps\/photo-challenge-pwa \/srv-photo-challenge/);
  assert.match(dockerfile, /Caddyfile\.production \/etc\/caddy\/Caddyfile/);
  assert.match(dockerfile, /EXPOSE 80 8869/);
  assert.doesNotMatch(dockerfile, /EXPOSE 80 443 8869/);
});

test("production compose uses simple direct-IP HTTP profile", () => {
  const production = repo("deploy/cloud/docker-compose.production.yml");
  assert.match(production, /ghcr\.io\/zhouxingxing1279\/lifetrace-web:main/);
  assert.match(production, /lifetrace-execution-worker:/);
  assert.match(production, /PUBLIC_WEB_BASE_URL:-http:\/\/8\.148\.75\.45/);
  assert.match(production, /CORS_ALLOWED_ORIGINS:-http:\/\/8\.148\.75\.45/);
  assert.match(production, /AUTH_COOKIE_SECURE: "false"/);
  assert.match(production, /AUTH_COOKIE_NAME: "lifetrace_session"/);
  assert.match(production, /DEV_AUTH_ENABLED: "false"/);
  assert.match(production, /"8869:8869"/);
  assert.doesNotMatch(production, /"443:443"/);
  assert.match(production, /127\.0\.0\.1:2019\/config\//);
  assert.doesNotMatch(production, /\.\.\/\.\.\/apps\/desktop\/dist-browser/);
  assert.doesNotMatch(production, /\.\.\/\.\.\/apps\/photo-challenge-pwa/);
  assert.doesNotMatch(production, /sunxiao0721\/beecount-cloud/);
  assert.doesNotMatch(production, /^\s{2}beecount-cloud:/m);
  assert.doesNotMatch(production, /^\s{2}beecount_data:/m);
  assert.doesNotMatch(production, /sslip\.io/);
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
  assert.match(workflow, /Validate production Caddy config/);
  assert.match(workflow, /Validate production Compose config/);
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
  assert.match(script, /wait_for_service caddy true/);
  assert.match(script, /wait_for_public_url "LifeTrace public endpoint"/);
  assert.match(script, /wait_for_public_url "BeeCount compatibility endpoint"/);
  assert.match(script, /http:\/\/8\.148\.75\.45:8869/);
  assert.doesNotMatch(script, /https:\/\/8\.148\.75\.45/);
  assert.match(script, /last public endpoint error/);
  assert.match(script, /verify inbound TCP 80\/8869/);
  assert.match(script, /--skip-git-update/);
  assert.doesNotMatch(script, /npm (ci|run)/);
});
