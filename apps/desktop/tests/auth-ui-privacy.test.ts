import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

test("signed-out desktop gates the local business workspace", async () => {
  const source = await readFile("src/components/DesktopApp.tsx", "utf8");
  assert.match(source, /const hasIdentity = Boolean\(user && \(authenticated \|\| phase === "offline"\)\)/);
  assert.match(source, /if \(!hasIdentity\)[\s\S]*return <SignedOutShell restoring=\{restoring\}\/>/);

  const identityGate = source.indexOf("if (!hasIdentity)");
  const signedOutReturn = source.indexOf("return <SignedOutShell", identityGate);
  const providers = source.indexOf("<DesktopProviders>", signedOutReturn);
  const localWorkspace = source.indexOf("<HengXuShell", signedOutReturn);

  assert.ok(identityGate >= 0, "desktop must gate business UI on an authenticated identity");
  assert.ok(signedOutReturn > identityGate, "signed-out users must return before business workspace renders");
  assert.ok(providers > signedOutReturn, "desktop runtime providers must stay behind the identity gate");
  assert.ok(localWorkspace > signedOutReturn, "local workspace must stay behind the identity gate");
  assert.doesNotMatch(source, /DesktopCloudWorkspace|CloudDataStore|DesktopFeatureRouter/);
});

test("desktop updater is available before login without a web workspace lifecycle", async () => {
  const [desktop, shell] = await Promise.all([
    readFile("src/components/DesktopApp.tsx", "utf8"),
    readFile("src/components/HengXuShell.tsx", "utf8"),
  ]);
  const signedOutStart = desktop.indexOf("function SignedOutShell");
  const desktopStart = desktop.indexOf("export default function DesktopApp");
  const signedOut = desktop.slice(signedOutStart, desktopStart);

  assert.match(signedOut, /<AppUpdaterHost \/>/, "signed-out desktop must still check for updates");
  assert.match(shell, /<AppUpdaterHost \/>/, "signed-in local shell must own the updater lifecycle");
  assert.doesNotMatch(desktop, /DesktopCloudWorkspace/, "updater must not depend on a web workspace bridge");
});

test("signed-out shell keeps the account entry visible and opens login automatically", async () => {
  const [desktop, account] = await Promise.all([
    readFile("src/components/DesktopApp.tsx", "utf8"),
    readFile("src/components/account/AccountEntry.tsx", "utf8"),
  ]);
  assert.match(desktop, /hx-sidebar-foot"><AccountEntry autoOpen=\{!restoring\}\/><\/div>/);
  assert.match(account, /export function AccountEntry\(\{ autoOpen = false \}/);
  assert.match(account, /if \(autoOpen\) setDialog\(\(current\) => current \?\? "login"\)/);
});

test("signed-out shell contains no business navigation or data widgets", async () => {
  const source = await readFile("src/components/DesktopApp.tsx", "utf8");
  const signedOutStart = source.indexOf("function SignedOutShell");
  const desktopStart = source.indexOf("export default function DesktopApp");
  const signedOut = source.slice(signedOutStart, desktopStart);
  for (const forbidden of ["Dashboard", "Finance", "NotesModule", "PhotoSyncModule", "navGroups"]) {
    assert.equal(signedOut.includes(forbidden), false, `${forbidden} must not render while signed out`);
  }
});

test("auth dialog cannot overflow horizontally and topbar actions are a real entry point", async () => {
  const css = await readFile("app/auth-shell-fixes.css", "utf8");
  assert.equal(css.includes(".hx-page-actions{display:none!important}"), false);
  assert.match(css, /\.hx-account-dialog\{[^}]*overflow-x:hidden!important/);
  assert.match(css, /\.hx-account-form\{[^}]*flex-direction:column!important/);
  assert.match(css, /\.hx-account-form input,[\s\S]*width:100%!important/);
  const shell = await readFile("src/components/layout/AppShell.tsx", "utf8");
  assert.match(shell, /className="hx-page-actions"/);
  assert.match(shell, /CommandPalette/);
});