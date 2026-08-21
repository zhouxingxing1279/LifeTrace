import { expect, test, type Page, type Route } from "@playwright/test";

const routes = ["/app/today", "/app/execution", "/app/calendar", "/app/habits", "/app/fitness", "/app/health", "/app/notes", "/app/english/articles", "/app/review", "/app/finance", "/app/finance/transactions", "/app/finance/calendar", "/app/finance/ledgers", "/app/finance/budgets", "/app/finance/accounts", "/app/finance/categories", "/app/finance/tags", "/app/finance/import", "/app/settings", "/app/system/ui"];

function sessionBody() {
  return {
    user: { id: "user-e2e", email: "e2e@lifetrace.test", displayName: "E2E", state: "active", emailVerifiedAt: null, createdAt: "2026-08-22T00:00:00Z", passwordChangedAt: null },
    session: { id: "session-e2e", appId: "web", deviceId: "device-e2e", sessionType: "web", status: "active", scopes: ["sync:read", "sync:write", "execution:read", "execution:write"], publicDevice: false, createdAt: "2026-08-22T00:00:00Z", lastSeenAt: "2026-08-22T00:00:00Z", idleExpiresAt: "2026-08-23T00:00:00Z", absoluteExpiresAt: "2026-09-22T00:00:00Z", revokedAt: null, current: true },
    csrfToken: "csrf-e2e"
  };
}

type StoredEntity = { entityType: string; entityId: string; serverVersion: string; payload: unknown };

async function mockCloud(page: Page, startAuthenticated = true) {
  let authenticated = startAuthenticated;
  let version = 1;
  const entities = new Map<string, StoredEntity>();
  const key = (type: string, id: string) => `${type}:${id}`;

  await page.route("**/api/v1/**", async (route: Route) => {
    const request = route.request();
    const url = new URL(request.url());
    const pathname = url.pathname;

    if (pathname === "/api/v1/web/session" && request.method() === "GET") {
      if (!authenticated) return route.fulfill({ status: 401, contentType: "application/json", body: JSON.stringify({ code: "AUTH_INVALID", message: "sign in required" }) });
      return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(sessionBody()) });
    }
    if (pathname === "/api/v1/web/session/login" && request.method() === "POST") {
      authenticated = true;
      return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(sessionBody()) });
    }
    if (pathname === "/api/v1/web/session/logout" && request.method() === "POST") {
      authenticated = false;
      return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ accepted: true }) });
    }
    if (pathname === "/api/v1/sync/snapshot" && request.method() === "POST") {
      const body = request.postDataJSON() as { entityTypes?: string[] | null };
      const allowed = body.entityTypes ? new Set(body.entityTypes) : null;
      const items = [...entities.values()].filter((item) => !allowed || allowed.has(item.entityType));
      return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ requestId: "snapshot-e2e", snapshotId: "snapshot-1", snapshotCursor: String(version), items, nextPageToken: null, completed: true, serverTime: "2026-08-22T00:00:00Z" }) });
    }
    if (pathname === "/api/v1/sync/push" && request.method() === "POST") {
      const body = request.postDataJSON() as { requestId: string; changes: Array<{ changeId: string; entityType: string; entityId: string; operation: string; payload: unknown }> };
      const results = body.changes.map((change) => {
        version += 1;
        if (change.operation === "delete") entities.delete(key(change.entityType, change.entityId));
        else entities.set(key(change.entityType, change.entityId), { entityType: change.entityType, entityId: change.entityId, serverVersion: String(version), payload: change.payload });
        return { status: "accepted", changeId: change.changeId, entityType: change.entityType, entityId: change.entityId, serverVersion: String(version), cursor: String(version), serverModifiedAt: "2026-08-22T00:00:00Z" };
      });
      return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ requestId: body.requestId, serverTime: "2026-08-22T00:00:00Z", results, latestCursor: String(version) }) });
    }
    return route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ message: `unmocked ${pathname}` }) });
  });

  return { entities, isAuthenticated: () => authenticated };
}

test("all v2 routes render the shared LifeTrace shell", async ({ page }) => {
  await mockCloud(page);
  for (const route of routes) {
    await page.goto(route);
    await expect(page.locator("body")).toContainText("LifeTrace");
    await expect(page.locator("main")).toBeVisible();
  }
});

test("cloud login, task sync, reload and logout form one real user path", async ({ page }) => {
  const cloud = await mockCloud(page, false);
  await page.goto("/login");
  await page.getByLabel("Email").fill("e2e@lifetrace.test");
  await page.getByLabel("Password").fill("correct-password");
  await page.getByRole("button", { name: "Continue" }).click();
  await expect(page).toHaveURL(/\/app\/today$/);

  await page.getByRole("button", { name: "Quick Capture" }).click();
  await page.getByPlaceholder("下一步要做什么？").fill("Validate cloud-backed rewrite");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.locator(".lt-list-item").filter({ hasText: "Validate cloud-backed rewrite" })).toHaveCount(1);
  await expect.poll(() => [...cloud.entities.values()].some((item) => item.entityType === "execution.task")).toBe(true);

  await page.reload();
  await expect(page.locator(".lt-list-item").filter({ hasText: "Validate cloud-backed rewrite" })).toHaveCount(1);
  await page.getByRole("button", { name: "Settings" }).first().click();
  await page.getByRole("button", { name: "Log out" }).click();
  await expect(page).toHaveURL(/\/login$/);
  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
  expect(cloud.isAuthenticated()).toBe(false);
});

test("quick capture keeps a task across SPA navigation", async ({ page }) => {
  await mockCloud(page);
  await page.goto("/app/today");
  await page.getByRole("button", { name: "Quick Capture" }).click();
  await page.getByPlaceholder("下一步要做什么？").fill("Validate clean-room rewrite");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.locator(".lt-list-item").filter({ hasText: "Validate clean-room rewrite" })).toHaveCount(1);
  await page.getByRole("button", { name: "Plan" }).first().click();
  await expect(page).toHaveURL(/\/app\/execution$/);
});

test("keyboard command palette is usable", async ({ page }) => {
  await mockCloud(page);
  await page.goto("/app/today");
  await page.keyboard.press(process.platform === "darwin" ? "Meta+K" : "Control+K");
  const dialog = page.getByRole("dialog", { name: "命令菜单" });
  await expect(dialog).toBeVisible();
  await dialog.getByPlaceholder("搜索页面、任务、笔记…").fill("Finance");
  await dialog.getByRole("button", { name: "Finance" }).click();
  await expect(page).toHaveURL(/\/app\/finance$/);
});

test("theme and responsive layout have no horizontal page overflow", async ({ page }) => {
  await mockCloud(page);
  await page.goto("/app/settings");
  await page.getByLabel("切换主题").click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", /dark|light/);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
  expect(overflow).toBe(false);
});
