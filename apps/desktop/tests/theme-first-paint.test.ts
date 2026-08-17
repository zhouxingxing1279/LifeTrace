import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const indexHtml = () => readFileSync(new URL("../web-client/index.html", import.meta.url), "utf8");
const appSource = () => readFileSync(new URL("../web-client/src/App.tsx", import.meta.url), "utf8");
const bootstrapScript = () => readFileSync(new URL("../public/theme-bootstrap.js", import.meta.url), "utf8");
const bootstrapStyles = () => readFileSync(new URL("../public/theme-bootstrap.css", import.meta.url), "utf8");

test("appearance theme cookie is restored before the app bundle starts", () => {
  const html = indexHtml();
  const bootstrapTheme = html.indexOf('/theme-bootstrap.js');
  const appBundle = html.indexOf('/src/main.tsx');

  assert.ok(bootstrapTheme >= 0, "theme bootstrap script is missing");
  assert.ok(appBundle > bootstrapTheme, "theme must be restored before main.tsx is requested");
  assert.ok(html.indexOf('/theme-bootstrap.css') < bootstrapTheme, "first-paint theme styles must load before the bootstrap script");

  const script = bootstrapScript();
  assert.match(script, /document\.cookie/);
  assert.match(script, /lifetrace_theme=/);
  assert.match(script, /document\.documentElement\.dataset\.theme = theme/);
  assert.match(script, /theme === "dark" \? "#101613" : "#f4f6f4"/);

  const styles = bootstrapStyles();
  assert.match(styles, /html\[data-theme="dark"\]/);
  assert.match(styles, /background: #101613/);
  assert.match(styles, /color-scheme: dark/);
});

test("react waits for cloud preferences before reconciling the first-paint theme", () => {
  const source = appSource();

  assert.match(source, /const THEME_COOKIE_NAME = "lifetrace_theme"/);
  assert.match(source, /const \[cloudLoaded, setCloudLoaded\] = useState\(false\)/);
  assert.match(source, /if \(!session \|\| !cloudLoaded\) return/);
  assert.match(source, /document\.cookie = `\$\{THEME_COOKIE_NAME\}=\$\{theme\}; Path=\/; Max-Age=31536000; SameSite=Lax`/);
  assert.match(source, /applyTheme\(preference\?\.value === "dark" \? "dark" : "light"\)/);
  assert.doesNotMatch(source, /localStorage|sessionStorage|indexedDB/);
});
