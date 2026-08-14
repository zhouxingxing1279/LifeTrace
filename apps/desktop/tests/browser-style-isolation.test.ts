import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const browserEntry = readFileSync(new URL("../web-client/src/main.tsx", import.meta.url), "utf8");

test("browser entry never imports desktop application css", () => {
  assert.doesNotMatch(browserEntry, /import "\.\.\/\.\.\/app\/[^\"]+\.css";/);
});

test("catch-all browser stylesheet has been eliminated", () => {
  assert.equal(existsSync(new URL("../web-client/src/browser.css", import.meta.url)), false);
  assert.doesNotMatch(browserEntry, /browser\.css/);
});

test("browser entry owns an explicit token -> primitive -> shell -> feature layer", () => {
  const tokens = browserEntry.indexOf('import "./web-tokens.css";');
  const primitives = browserEntry.indexOf('import "./web-primitives.css";');
  const shell = browserEntry.indexOf('import "./web-shell.css";');
  const auth = browserEntry.indexOf('import "./web-auth.css";');
  const workspaces = browserEntry.indexOf('import "./web-workspaces.css";');
  const beecount = browserEntry.indexOf('import "./web-beecount.css";');
  const features = browserEntry.indexOf('import "./web-features.css";');

  assert.ok(tokens >= 0, "design token layer is required");
  assert.ok(primitives > tokens, "shared primitives must load after tokens");
  assert.ok(shell > primitives, "application shell must compose primitives");
  assert.ok(auth > shell && workspaces > shell && beecount > shell, "specialized feature styles must load after shell");
  assert.ok(features > auth && features > workspaces && features > beecount, "feature composition must be the final browser-owned layer");
});
