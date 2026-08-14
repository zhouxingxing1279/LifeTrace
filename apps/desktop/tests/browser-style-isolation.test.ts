import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const browserEntry = readFileSync(new URL("../web-client/src/main.tsx", import.meta.url), "utf8");

test("browser entry never imports desktop application css", () => {
  assert.doesNotMatch(browserEntry, /import "\.\.\/\.\.\/app\/[^\"]+\.css";/);
});

test("browser entry owns an explicit token -> primitive -> shell -> feature layer", () => {
  const legacy = browserEntry.indexOf('import "./browser.css";');
  const tokens = browserEntry.indexOf('import "./web-tokens.css";');
  const primitives = browserEntry.indexOf('import "./web-primitives.css";');
  const shell = browserEntry.indexOf('import "./web-shell.css";');
  const features = browserEntry.indexOf('import "./web-features.css";');

  assert.ok(legacy >= 0, "browser compatibility stylesheet must remain explicit while migration is in progress");
  assert.ok(tokens > legacy, "design tokens must override compatibility variables");
  assert.ok(primitives > tokens, "shared primitives must load after tokens");
  assert.ok(shell > primitives, "application shell must compose primitives");
  assert.ok(features > shell, "feature composition must be the final browser-owned layer");
});
