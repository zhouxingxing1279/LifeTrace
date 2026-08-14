import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(new URL(`../web-client/src/${path}`, import.meta.url), "utf8");

const app = read("App.tsx");
const shell = read("components/AppShell.tsx");
const routes = read("components/RouteView.tsx");
const dashboard = read("pages/DashboardPage.tsx");
const navigation = read("navigation.ts");

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

test("route rendering is centralized and dashboard is a real page module", () => {
  assert.match(routes, /switch \(route\)/);
  assert.match(routes, /<DashboardPage/);
  assert.match(dashboard, /lt-dashboard-focus/);
  assert.match(dashboard, /lt-dashboard-layout/);
  assert.match(dashboard, /MetricGrid/);
});

test("global navigation exposes domains rather than every feature subroute", () => {
  assert.match(navigation, /label: "成长健康"/);
  assert.match(navigation, /label: "知识学习"/);
  assert.match(navigation, /label: "财务"/);
  const navBlock = navigation.slice(navigation.indexOf("export const NAV_GROUPS"), navigation.indexOf("export const SECONDARY_NAV"));
  assert.doesNotMatch(navBlock, /\/finance\/transactions/);
  assert.doesNotMatch(navBlock, /\/finance\/accounts/);
  assert.doesNotMatch(navBlock, /\/finance\/import/);
});
