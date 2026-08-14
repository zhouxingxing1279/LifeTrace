import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(new URL(`../web-client/src/${path}`, import.meta.url), "utf8");

const app = read("App.tsx");
const shell = read("components/AppShell.tsx");
const routes = read("components/RouteView.tsx");
const dashboard = read("pages/DashboardPage.tsx");
const dependencyAwareDashboard = read("pages/DependencyAwareDashboardPage.tsx");
const navigation = read("navigation.ts");
const shellCss = read("web-shell.css");
const finance = read("pages/FinancePages.tsx");
const growth = read("pages/GrowthPages.tsx");
const notes = read("pages/NotesPage.tsx");
const english = read("pages/EnglishPages.tsx");
const devices = read("pages/DevicesPage.tsx");

test("App remains orchestration-only", () => {
  assert.match(app, /<AppShell/);
  assert.match(app, /<RouteView/);
  assert.doesNotMatch(app, /function Dashboard/);
  assert.doesNotMatch(app, /function SearchPage/);
  assert.doesNotMatch(app, /NAV_GROUPS/);
  assert.ok(app.split("\n").length < 190, "App.tsx should not grow back into a monolith");
});

test("shell owns global navigation and responsive chrome", () => {
  assert.match(shell, /NAV_GROUPS/);
  assert.match(shell, /MOBILE_NAV/);
  assert.match(shell, /aria-label="功能导航"/);
  assert.match(shell, /lt-sidebar-toggle/);
});

test("shell stylesheet stays single-purpose", () => {
  assert.match(shellCss, /\.hx-shell/);
  assert.match(shellCss, /\.hx-topbar/);
  assert.match(shellCss, /\.browser-mobile-nav/);
  assert.doesNotMatch(shellCss, /\.hx-btn\s*\{/);
  assert.doesNotMatch(shellCss, /\.hx-panel\s*[,\{]/);
  assert.doesNotMatch(shellCss, /\.lt-dashboard/);
  assert.doesNotMatch(shellCss, /\.hx-form\s/);
});

test("route rendering is centralized and dashboard remains a composable page module", () => {
  assert.match(routes, /switch \(route\)/);
  assert.match(routes, /<DependencyAwareDashboardPage/);
  assert.match(dependencyAwareDashboard, /<DashboardPage/);
  assert.match(dependencyAwareDashboard, /DEPENDENCY-AWARE TODAY/);
  assert.match(dashboard, /lt-dashboard-focus/);
  assert.match(dashboard, /lt-dashboard-layout/);
  assert.match(dashboard, /MetricGrid/);
});

test("major domains share the browser page primitives", () => {
  for (const [name, source] of [
    ["finance", finance],
    ["growth", growth],
    ["notes", notes],
    ["english", english],
    ["devices", devices],
  ] as const) {
    assert.match(source, /PageStack/, `${name} must use PageStack`);
    assert.match(source, /Panel/, `${name} must use Panel`);
  }
  assert.match(finance, /Metric/);
  assert.match(growth, /Metric/);
  assert.match(english, /Metric/);
});

test("global navigation exposes domains rather than every feature subroute", () => {
  assert.match(navigation, /label: "成长健康"/);
  assert.match(navigation, /label: "知识学习"/);
  assert.match(navigation, /label: "财务"/);
  const navBlock = navigation.slice(navigation.indexOf("export const NAV_GROUPS"), navigation.indexOf("export const SECONDARY_NAV"));
  assert.doesNotMatch(navBlock, /\/finance\/transactions/);
  assert.doesNotMatch(navBlock, /\/finance\/accounts/);
  assert.doesNotMatch(navBlock, /\/finance\/import/);
  assert.doesNotMatch(navBlock, /\/execution\/goals/);
  assert.doesNotMatch(navBlock, /\/execution\/control/);
});
