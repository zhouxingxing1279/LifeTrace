import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const browserEntry = readFileSync(new URL("../web-client/src/main.tsx", import.meta.url), "utf8");

test("browser entry uses only browser-owned styles", () => {
  assert.match(browserEntry, /import "\.\/web-shell\.css";/);
  assert.match(browserEntry, /import "\.\/browser\.css";/);
  assert.match(browserEntry, /import "\.\/styles\.css";/);
  assert.match(browserEntry, /import "\.\/cloud-pages\.css";/);
  assert.doesNotMatch(browserEntry, /import "\.\.\/\.\.\/app\/[^\"]+\.css";/);
});

test("browser shell loads before browser feature overrides", () => {
  const shell = browserEntry.indexOf('import "./web-shell.css";');
  const featureOverrides = browserEntry.indexOf('import "./browser.css";');
  assert.ok(shell >= 0 && featureOverrides > shell, "browser.css must remain the final web override layer");
});
