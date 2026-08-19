import { expect, test, type Page, type Route } from "@playwright/test";

const now = "2026-08-19T02:00:00.000Z";

function meta(id: string) {
  return { id, userId: "user-1", createdAt: now, updatedAt: now, localVersion: 1, serverVersion: "1", modifiedByDevice: "web-test" };
}

async function json(route: Route, payload: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(payload) });
}

async function installMocks(page: Page, pushes: Array<Record<string, unknown>>) {
  await page.route("**/api/v1/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/v1/web/session") {
      return json(route, {
        user: { id: "user-1", email: "tester@example.com", displayName: "Web Tester" },
        session: { id: "session-1", appId: "lifetrace-web", deviceId: "web-test", scopes: ["sync:read", "sync:write"], idleExpiresAt: "2026-08-20T00:00:00.000Z", absoluteExpiresAt: "2026-08-26T00:00:00.000Z", publicDevice: false },
        csrfToken: "csrf-test",
      });
    }
    if (path === "/api/v1/sync/snapshot") {
      return json(route, {
        snapshotId: "snapshot-1",
        snapshotCursor: "cursor-1",
        items: [
          { entityType: "finance.account", entityId: "account-1", serverVersion: "1", payload: { meta: meta("account-1"), name: "Cash", accountType: "cash", currency: "CNY", openingBalanceCents: 100000 } },
          { entityType: "finance.category", entityId: "category-1", serverVersion: "1", payload: { meta: meta("category-1"), name: "Food", categoryType: "expense" } },
          { entityType: "finance.transaction", entityId: "tx-1", serverVersion: "1", payload: { meta: meta("tx-1"), transactionType: "expense", amountCents: 2300, currency: "CNY", merchant: "Coffee Shop", note: "breakfast", localDate: "2026-08-19", occurredAt: now, status: "confirmed", accountId: "account-1", categoryId: "category-1" } },
        ],
        nextPageToken: null,
        completed: true,
      });
    }
    if (path === "/api/v1/sync/pull") return json(route, { changes: [], nextCursor: "cursor-2", hasMore: false });
    if (path === "/api/v1/sync/push") {
      const body = route.request().postDataJSON() as Record<string, unknown>;
      pushes.push(body);
      const changes = Array.isArray(body.changes) ? body.changes as Array<Record<string, unknown>> : [];
      return json(route, {
        results: changes.map((change, index) => ({ changeId: change.changeId, entityType: change.entityType, entityId: change.entityId, status: "accepted", serverVersion: `server-${index + 2}` })),
      });
    }
    if (path === "/api/v1/integrations/beecount/status") return json(route, { enabled: false, readOnly: true, source: "beecount-cloud", upstreamReachable: false });
    if (path === "/api/v1/web/devices") return json(route, { devices: [] });
    if (path === "/api/v1/web/sessions") return json(route, { sessions: [] });
    if (path === "/api/v1/web/csrf") return json(route, { csrfToken: "csrf-test" });
    if (path === "/api/v1/web/session/logout") return json(route, {});
    return json(route, {});
  });
}

function flattenedChanges(pushes: Array<Record<string, unknown>>) {
  return pushes.flatMap((body) => Array.isArray(body.changes) ? body.changes as Array<Record<string, unknown>> : []);
}

test("native finance transaction supports create edit delete and filtering", async ({ page }) => {
  const pushes: Array<Record<string, unknown>> = [];
  await installMocks(page, pushes);
  await page.goto("/app/finance/transactions");
  await expect(page.getByRole("heading", { name: "财务 · 交易", level: 1 })).toBeVisible();
  await expect(page.getByText("Coffee Shop")).toBeVisible();

  await page.getByPlaceholder("筛选商户、备注或日期").fill("does-not-exist");
  await expect(page.getByText("没有匹配的 LifeTrace 交易")).toBeVisible();
  await page.getByPlaceholder("筛选商户、备注或日期").fill("");

  await page.getByRole("button", { name: "编辑交易" }).click();
  const editDialog = page.getByRole("dialog", { name: "编辑交易" });
  await expect(editDialog).toBeVisible();
  await editDialog.getByLabel("金额").fill("28.50");
  await editDialog.getByLabel("商户 / 对象").fill("Coffee Lab");
  await editDialog.getByRole("button", { name: "保存修改" }).click();
  await expect(page.getByText("Coffee Lab")).toBeVisible();
  expect(flattenedChanges(pushes).some((change) => change.entityType === "finance.transaction")).toBe(true);

  await page.getByRole("button", { name: "记一笔" }).click();
  const createDialog = page.getByRole("dialog", { name: "新增交易" });
  await createDialog.getByLabel("金额").fill("66.00");
  await createDialog.getByLabel("商户 / 对象").fill("Dinner");
  await createDialog.getByRole("button", { name: "新增交易" }).click();
  await expect(page.getByText("Dinner")).toBeVisible();

  const deleteButtons = page.getByRole("button", { name: "删除交易" });
  await deleteButtons.first().click();
  expect(flattenedChanges(pushes).length).toBeGreaterThanOrEqual(3);
});

test("Dialog autofocuses, traps Tab, closes on Escape, and restores focus", async ({ page }) => {
  const pushes: Array<Record<string, unknown>> = [];
  await installMocks(page, pushes);
  await page.goto("/app/system/ui");
  await page.getByRole("tab", { name: "Overlay / Feedback" }).click();
  const trigger = page.getByRole("button", { name: "Dialog", exact: true });
  await trigger.focus();
  await trigger.click();

  const dialog = page.getByRole("dialog", { name: "Dialog" });
  await expect(dialog).toBeVisible();
  const done = dialog.getByRole("button", { name: "完成" });
  await expect(done).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(done).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(done).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
});
