import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const appPreferences = () => readFileSync(new URL("../src/services/appPreferences.ts", import.meta.url), "utf8");
const tauriIndex = () => readFileSync(new URL("../tauri-ui/index.html", import.meta.url), "utf8");
const tauriMain = () => readFileSync(new URL("../tauri-ui/main.tsx", import.meta.url), "utf8");
const bootstrapScript = () => readFileSync(new URL("../public/desktop-theme-bootstrap.js", import.meta.url), "utf8");
const bootstrapStyles = () => readFileSync(new URL("../public/desktop-theme-bootstrap.css", import.meta.url), "utf8");
const designTokens = () => readFileSync(new URL("../app/tokens.css", import.meta.url), "utf8");

test("desktop appearance is owned by local app preferences", () => {
  const source = appPreferences();
  assert.match(source, /APP_PREFERENCES_STORAGE_KEY = "lifetrace\.app-preferences\.v1"/);
  assert.match(source, /export function readAppPreferences/);
  assert.match(source, /export function applyAppPreferences/);
  assert.match(source, /export function setAppThemePreference/);
  assert.match(source, /target\.dataset\.theme = resolvedTheme/);
  assert.doesNotMatch(source, /CloudDataStore|appearance\.theme.*cloud/i);
});

test("tauri restores the cached theme before the react entrypoint", () => {
  const html = tauriIndex();
  const bootstrap = html.indexOf('/desktop-theme-bootstrap.js');
  const main = html.indexOf('/main.tsx');
  assert.ok(bootstrap >= 0, "desktop theme bootstrap is missing");
  assert.ok(main > bootstrap, "desktop theme must be restored before the React bundle starts");
  assert.ok(html.indexOf('/desktop-theme-bootstrap.css') < bootstrap, "first-paint theme CSS must load before the bootstrap script");

  const script = bootstrapScript();
  assert.match(script, /lifetrace\.app-preferences\.v1/);
  assert.match(script, /lifetrace_theme=/);
  assert.match(script, /document\.documentElement\.dataset\.theme = resolved/);

  const styles = bootstrapStyles();
  assert.match(styles, /html\[data-theme="dark"\]/);
  assert.match(styles, /background: #171a18/);
  assert.match(styles, /color-scheme: dark/);
});

test("legacy sqlite dark state and desktop DOM theme stay synchronized", () => {
  const source = tauriMain();
  assert.match(source, /useLifeStore\.subscribe\(\(state, previous\) =>/);
  assert.match(source, /if \(!previous\.ready\)/);
  assert.match(source, /useLifeStore\.setState\(\{ dark: true \}\)/);
  assert.match(source, /state\.dark !== previous\.dark/);
  assert.match(source, /setAppThemePreference\(state\.dark \? "dark" : "light"\)/);
});

test("native desktop shell colors inherit the shared semantic theme", () => {
  const tokens = designTokens();
  assert.match(tokens, /--lt-color-bg:\s*var\(--ui-bg-app\)/);
  assert.match(tokens, /--lt-color-surface:\s*var\(--ui-bg-surface\)/);
  assert.match(tokens, /--lt-color-text:\s*var\(--ui-foreground\)/);
  assert.match(tokens, /--lt-color-border:\s*var\(--ui-border\)/);
  assert.match(tokens, /--lt-color-primary:\s*var\(--ui-primary\)/);
});