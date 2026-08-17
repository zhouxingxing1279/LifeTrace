import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const indexHtml = () => readFileSync(new URL("../web-client/index.html", import.meta.url), "utf8");
const appSource = () => readFileSync(new URL("../web-client/src/App.tsx", import.meta.url), "utf8");

test("cached appearance theme is restored in the document head before the app bundle starts", () => {
  const html = indexHtml();
  const bootstrapTheme = html.indexOf("lifetrace.appearance.theme");
  const appBundle = html.indexOf('/src/main.tsx');

  assert.ok(bootstrapTheme >= 0, "theme cache bootstrap is missing");
  assert.ok(appBundle > bootstrapTheme, "theme must be restored before main.tsx is requested");
  assert.match(html, /document\.documentElement\.dataset\.theme = theme/);
  assert.match(html, /html\[data-theme="dark"\] \{ background: #101613; color-scheme: dark; \}/);
  assert.match(html, /theme === "dark" \? "#101613" : "#f4f6f4"/);
});

test("react waits for cloud preferences before reconciling the cached first-paint theme", () => {
  const source = appSource();

  assert.match(source, /const THEME_CACHE_KEY = "lifetrace\.appearance\.theme"/);
  assert.match(source, /const \[cloudLoaded, setCloudLoaded\] = useState\(false\)/);
  assert.match(source, /if \(!session \|\| !cloudLoaded\) return/);
  assert.match(source, /localStorage\.setItem\(THEME_CACHE_KEY, theme\)/);
  assert.match(source, /applyTheme\(preference\?\.value === "dark" \? "dark" : "light"\)/);
});
