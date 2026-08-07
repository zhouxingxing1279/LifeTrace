import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

test("signed-out desktop does not mount the business shell", async () => {
  const source = await readFile("src/components/DesktopApp.tsx", "utf8");
  assert.match(source, /const hasIdentity = Boolean\(user && \(authenticated \|\| phase === "offline"\)\)/);
  assert.match(source, /if \(!hasIdentity\)[\s\S]*SignedOutShell/);
  assert.match(source, /return <><HengXuShell\/><AccountEntryHost\/><\/>/);
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
