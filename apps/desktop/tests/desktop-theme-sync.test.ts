import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const cloudWorkspace = () => readFileSync(new URL("../src/components/DesktopCloudWorkspace.tsx", import.meta.url), "utf8");
const tauriIndex = () => readFileSync(new URL("../tauri-ui/index.html", import.meta.url), "utf8");
const tauriMain = () => readFileSync(new URL("../tauri-ui/main.tsx", import.meta.url), "utf8");
const bootstrapScript = () => readFileSync(new URL("../public/desktop-theme-bootstrap.js", import.meta.url), "utf8");
const bootstrapStyles = () => readFileSync(new URL("../public/desktop-theme-bootstrap.css", import.meta.url), "utf8");

test("desktop cloud workspace applies the loaded cloud appearance preference", () => {
  const source = cloudWorkspace();
  assert.match(source, /const \[cloudLoaded, setCloudLoaded\] = useState\(false\)/);
  assert.match(source, /if \(!session \|\| !cloudLoaded\) return/);
  assert.match(source, /preferenceKey"\) === "appearance\.theme"/);
  assert.match(source, /setAppThemePreference\(preference\?\.value === "dark" \? "dark" : "light"\)/);
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
