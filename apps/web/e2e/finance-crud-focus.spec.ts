import { expect, test, type Page, type Route } from "@playwright/test";

const now = "2026-08-19T02:00:00.000Z";

async function json(route: Route, payload: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(payload) });
}

const ledger = {
  id: "beecount:ledger-1", sourceId: "ledger-1", name: "日常账本", currency: "CNY", monthStartDay: 1,
  transactionCount: 2, incomeTotalCents: 120000, expenseTotalCents: 4300, balanceCents: 115700,
  updatedAt: now, role: "owner", isShared: false, memberCount: 1, readOnly: true,
};

const beeCountSnapshot = {
  source: "beecount-cloud", readOnly: true, fetchedAt: now, ledger,
  transactions: { items: [
    { id: "beecount:tx-1", externalTransactionId: "tx-1", transactionType: "expense", amountCents: 2300, currency: "CNY", occurredAt: now, localDate: "2026-08-19", status: "confirmed", sourceType: "beecount-cloud", note: "Coffee Shop", ledgerId: "ledger-1", ledgerName: "日常账本", accountId: "account-1", categoryId: "category-1", accountName: "现金", categoryName: "餐饮", tags: ["早餐"], tagIds: ["tag-1"], attachments: [], excludeFromStats: false, excludeFromBudget: false, readOnly: true },
    { id: "beecount:tx-2", externalTransactionId: "tx-2", transactionType: "income", amountCents: 120000, currency: "CNY", occurredAt: now, localDate: "2026-08-19", status: "confirmed", sourceType: "beecount-cloud", note: "工资", ledgerId: "ledger-1", ledgerName: "日常账本", accountId: "account-1", categoryId: "category-2", accountName: "现金", categoryName: "工资", tags: [], tagIds: [], attachments: [], excludeFromStats: false, excludeFromBudget: false, readOnly: true },
  ], total: 2, limit: 500, offset: 0 },
  accounts: [{ id: "beecount:account-1", sourceId: "account-1", name: "现金", accountType: "cash", currency: "CNY", balanceCents: 115700, transactionCount: 2, source: "beecount-cloud", readOnly: true }],
  categories: [
    { id: "beecount:category-1", sourceId: "category-1", name: "餐饮", categoryType: "expense", transactionCount: 1, source: "beecount-cloud", readOnly: true },
    { id: "beecount:category-2", sourceId: "category-2", name: "工资", categoryType: "income", transactionCount: 1, source: "beecount-cloud", readOnly: true },
  ],
  tags: [{ id: "beecount:tag-1", sourceId: "tag-1", name: "早餐", transactionCount: 1, expenseTotalCents: 2300, source: "beecount-cloud", readOnly: true }],
  budgets: [{ id: "beecount:budget-1", sourceId: "budget-1", budgetType: "category", categoryId: "category-1", categoryName: "餐饮", amountCents: 100000, period: "monthly", startDay: 1, enabled: true, source: "beecount-cloud", readOnly: true }],
};

async function installMocks(page: Page) {
  await page.route("**/api/v1/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/v1/web/session") return json(route, { user: { id: "user-1", email: "tester@example.com", displayName: "Web Tester" }, session: { id: "session-1", appId: "lifetrace-web", deviceId: "web-test", scopes: ["sync:read", "sync:write"], idleExpiresAt: "2026-08-20T00:00:00.000Z", absoluteExpiresAt: "2026-08-26T00:00:00.000Z", publicDevice: false }, csrfToken: "csrf-test" });
    if (path === "/api/v1/sync/snapshot") return json(route, { snapshotId: "snapshot-1", snapshotCursor: "cursor-1", items: [], nextPageToken: null, completed: true });
    if (path === "/api/v1/sync/pull") return json(route, { changes: [], nextCursor: "cursor-2", hasMore: false });
    if (path === "/api/v1/integrations/beecount/status") return json(route, { enabled: true, readOnly: true, source: "beecount-cloud", upstreamReachable: true, upstreamVersion: { version: "test" } });
    if (path === "/api/v1/integrations/beecount/ledgers") return json(route, { source: "beecount-cloud", readOnly: true, items: [ledger], fetchedAt: now });
    if (path === "/api/v1/integrations/beecount/ledgers/ledger-1/snapshot") return json(route, beeCountSnapshot);
    return json(route, {});
  });
}

test("finance transactions use BeeCount as the only runtime source", async ({ page }) => {
  await installMocks(page);
  await page.goto("/app/finance/transactions");
  await expect(page.getByRole("heading", { name: "财务", level: 1 })).toBeVisible();
  await expect(page.getByText("唯一财务数据源")).toBeVisible();
  await expect(page.getByText("Coffee Shop")).toBeVisible();
  await expect(page.getByText("LifeTrace Native")).toHaveCount(0);
  await expect(page.getByText("适配器未启用")).toHaveCount(0);
  const filter = page.getByPlaceholder("筛选备注、账户、分类、标签或日期");
  await filter.fill("does-not-exist");
  await expect(page.getByText("没有匹配的 BeeCount 交易")).toBeVisible();
  await filter.fill("早餐");
  await expect(page.getByText("Coffee Shop")).toBeVisible();
});

test("Dialog autofocuses, traps Tab, closes on Escape, and restores focus", async ({ page }) => {
  await installMocks(page);
  await page.goto("/app/system/ui");
  await page.getByRole("tab", { name: "Overlay / Feedback" }).click();
  const trigger = page.getByRole("button", { name: "Dialog", exact: true });
  await trigger.focus(); await trigger.click();
  const dialog = page.getByRole("dialog", { name: "Dialog" });
  const done = dialog.getByRole("button", { name: "完成" });
  await expect(done).toBeFocused();
  await page.keyboard.press("Tab"); await expect(done).toBeFocused();
  await page.keyboard.press("Shift+Tab"); await expect(done).toBeFocused();
  await page.keyboard.press("Escape"); await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
});
