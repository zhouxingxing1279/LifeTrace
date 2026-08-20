import { expect, test, type Page, type Route } from "@playwright/test";

type MockOptions = {
  sessionStatus?: number;
  snapshotDelayMs?: number;
  snapshotError?: boolean;
  empty?: boolean;
};

const now = "2026-08-19T02:00:00.000Z";

function meta(id: string) {
  return { id, userId: "user-1", createdAt: now, updatedAt: now, localVersion: 1, serverVersion: "1", modifiedByDevice: "web-test" };
}

function snapshotItems(empty = false) {
  if (empty) return [];
  const rows = [
    ["execution.task", "task-1", { meta: meta("task-1"), title: "Finish redesign", status: "todo", priority: "high", dueAt: "2026-08-19T15:00:00.000Z", scheduledStartAt: "2026-08-19T10:00:00.000Z" }],
    ["execution.calendar_event", "event-1", { meta: meta("event-1"), title: "Design review", status: "scheduled", startLocalDate: "2026-08-19", endLocalDate: "2026-08-19", isAllDay: true }],
    ["habit.activity", "habit-1", { meta: meta("habit-1"), name: "Read 30 minutes", isArchived: false }],
    ["habit.log", "habit-log-1", { meta: meta("habit-log-1"), activityId: "habit-1", logDate: "2026-08-19", status: "completed", value: 1 }],
    ["workout.workout", "workout-1", { meta: meta("workout-1"), name: "Upper body", localDate: "2026-08-19", occurredAt: now, durationSeconds: 3600, volumeKg: 6200 }],
    ["note.note", "note-1", { meta: meta("note-1"), title: "Architecture", contentText: "Independent apps/web", summary: "Independent apps/web", isArchived: false }],
    ["english.article", "article-1", { meta: meta("article-1"), title: "Sample Article", summary: "A short reading sample.", contentText: "Daily practice improves fluency.\n\nFocused reading also builds vocabulary." }],
    ["finance.account", "account-1", { meta: meta("account-1"), name: "Cash", accountType: "cash", currency: "CNY", openingBalanceCents: 100000 }],
    ["finance.category", "category-1", { meta: meta("category-1"), name: "Food", categoryType: "expense" }],
    ["finance.transaction", "tx-1", { meta: meta("tx-1"), transactionType: "expense", amountCents: 2300, currency: "CNY", merchant: "Coffee Shop", note: "breakfast", localDate: "2026-08-19", occurredAt: now, status: "confirmed" }],
    ["review.daily", "review-1", { meta: meta("review-1"), reviewDate: "2026-08-18", energy: 4, mood: 4, bestThing: "Finished the migration plan", tomorrowPriority: "Ship new Web" }],
  ] as const;
  return rows.map(([entityType, entityId, payload]) => ({ entityType, entityId, serverVersion: "1", payload }));
}

async function json(route: Route, payload: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(payload) });
}

async function installMocks(page: Page, options: MockOptions = {}) {
  await page.route("**/api/v1/**", async (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname;

    if (path === "/api/v1/web/session") {
      if ((options.sessionStatus ?? 200) !== 200) return json(route, { message: "session expired" }, options.sessionStatus ?? 401);
      return json(route, {
        user: { id: "user-1", email: "tester@example.com", displayName: "Web Tester" },
        session: {
          id: "session-1",
          appId: "lifetrace-web",
          deviceId: "web-test",
          scopes: ["sync:read", "sync:write"],
          idleExpiresAt: "2026-08-20T00:00:00.000Z",
          absoluteExpiresAt: "2026-08-26T00:00:00.000Z",
          publicDevice: false,
        },
        csrfToken: "csrf-test",
      });
    }

    if (path === "/api/v1/sync/snapshot") {
      if (options.snapshotDelayMs) await new Promise((resolve) => setTimeout(resolve, options.snapshotDelayMs));
      if (options.snapshotError) return json(route, { message: "snapshot unavailable" }, 503);
      return json(route, { snapshotId: "snapshot-1", snapshotCursor: "cursor-1", items: snapshotItems(options.empty), nextPageToken: null, completed: true });
    }

    if (path === "/api/v1/sync/pull") {
      return json(route, { changes: [], nextCursor: "cursor-2", hasMore: false });
    }

    if (path === "/api/v1/sync/push") {
      const body = route.request().postDataJSON() as { changes?: Array<Record<string, unknown>> };
      const results = (body.changes ?? []).map((change, index) => ({
        changeId: change.changeId,
        entityType: change.entityType,
        entityId: change.entityId,
        status: "accepted",
        serverVersion: `server-${index + 2}`,
      }));
      return json(route, { results });
    }

    if (path === "/api/v1/integrations/beecount/status") {
      return json(route, { enabled: false, readOnly: true, source: "beecount-cloud", upstreamReachable: false });
    }
    if (path === "/api/v1/integrations/beecount/ledgers") return json(route, { items: [], total: 0 });
    if (path === "/api/v1/web/devices") return json(route, { devices: [] });
    if (path === "/api/v1/web/sessions") return json(route, { sessions: [] });
    if (path === "/api/v1/web/session/logout") return json(route, {});
    if (path === "/api/v1/web/csrf") return json(route, { csrfToken: "csrf-test" });

    return json(route, {});
  });
}

const viewportMatrix = [
  { width: 360, height: 800 },
  { width: 390, height: 844 },
  { width: 430, height: 932 },
  { width: 768, height: 1024 },
  { width: 1024, height: 768 },
  { width: 1366, height: 768 },
  { width: 1440, height: 900 },
  { width: 1920, height: 1080 },
];

test("responsive matrix keeps the Personal OS usable without horizontal overflow", async ({ page }) => {
  await installMocks(page);
  for (const viewport of viewportMatrix) {
    await page.setViewportSize(viewport);
    await page.goto("/app/today");
    await expect(page.getByRole("heading", { name: "今天", level: 1 })).toBeVisible();
    if (viewport.width < 1024) {
      await expect(page.getByRole("navigation", { name: "移动端导航" })).toBeVisible();
      await expect(page.getByRole("navigation", { name: "主导航" })).toBeHidden();
    } else {
      await expect(page.getByRole("navigation", { name: "主导航" })).toBeVisible();
      await expect(page.getByRole("navigation", { name: "移动端导航" })).toBeHidden();
    }
    const hasOverflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1);
    expect(hasOverflow, `${viewport.width}x${viewport.height} must not overflow horizontally`).toBe(false);
  }
});

test("light dark and system theme modes resolve from the same semantic token system", async ({ page, context }) => {
  await installMocks(page);
  await context.addCookies([{ name: "lifetrace_theme", value: "light", url: "http://127.0.0.1:4173" }]);
  await page.goto("/app/today");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");

  await context.addCookies([{ name: "lifetrace_theme", value: "dark", url: "http://127.0.0.1:4173" }]);
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");

  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await context.addCookies([{ name: "lifetrace_theme", value: "system", url: "http://127.0.0.1:4173" }]);
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
});

test("direct route refresh back forward and command keyboard navigation work", async ({ page }) => {
  await installMocks(page);
  await page.setViewportSize({ width: 1366, height: 768 });
  await page.goto("/app/calendar");
  await expect(page.getByRole("heading", { name: "日历", level: 1 })).toBeVisible();
  await page.reload();
  await expect(page.getByRole("heading", { name: "日历", level: 1 })).toBeVisible();
  await page.getByRole("link", { name: "今日" }).click();
  await expect(page.getByRole("heading", { name: "今天", level: 1 })).toBeVisible();
  await page.goBack();
  await expect(page.getByRole("heading", { name: "日历", level: 1 })).toBeVisible();
  await page.goForward();
  await expect(page.getByRole("heading", { name: "今天", level: 1 })).toBeVisible();

  await page.keyboard.press("Control+K");
  const commandDialog = page.getByRole("dialog", { name: "全局命令" });
  await expect(commandDialog).toBeVisible();
  await commandDialog.getByRole("button", { name: /新建任务/ }).click();
  await expect(page.getByRole("heading", { name: "计划与待办", level: 1 })).toBeVisible();

  await page.keyboard.press("Tab");
  const activeTag = await page.evaluate(() => document.activeElement?.tagName ?? "BODY");
  expect(activeTag).not.toBe("BODY");
});

test("auth expiry redirects to login", async ({ page }) => {
  await installMocks(page, { sessionStatus: 401 });
  await page.goto("/app/today");
  await expect(page).toHaveURL(/\/login$/);
  await expect(page.getByRole("heading", { name: "登录你的生活工作台" })).toBeVisible();
});

test("loading empty and API error states are visible", async ({ page }) => {
  const options: MockOptions = { snapshotDelayMs: 500, empty: true };
  await installMocks(page, options);
  await page.goto("/app/today");
  await expect(page.getByText("同步中")).toBeVisible();
  await expect(page.getByRole("heading", { name: "今天", level: 1 })).toBeVisible();
  await expect(page.getByText("今天没有待办")).toBeVisible();

  options.snapshotDelayMs = undefined;
  options.empty = false;
  options.snapshotError = true;
  await page.reload();
  await expect(page.getByText("snapshot unavailable")).toBeVisible();
});

test("calendar exposes Month Week Day and Agenda", async ({ page }) => {
  await installMocks(page);
  // This test's snapshot is intentionally pinned to 2026-08-19. Freeze the
  // browser clock to the same fixture date so the calendar's initial visible
  // range does not drift as CI's real wall clock advances.
  await page.clock.setFixedTime(new Date(now));
  await page.setViewportSize({ width: 1366, height: 768 });
  await page.goto("/app/calendar");
  for (const label of ["Month", "Week", "Day", "Agenda"]) {
    const button = page.getByRole("button", { name: label, exact: true });
    await expect(button).toBeVisible();
    await button.click();
  }
  await expect(page.getByText("Design review")).toBeVisible();
});

test("English reader supports visual highlights quick notes and read completion", async ({ page }) => {
  await installMocks(page);
  await page.goto("/app/english/articles");
  await page.getByRole("button", { name: /Sample Article/ }).click();
  await expect(page.getByRole("heading", { name: "Sample Article", level: 1 })).toBeVisible();
  const article = page.getByTestId("reader-article");
  await expect(article.locator("mark")).toHaveCount(0);
  const phrase = article.getByText("Daily practice improves fluency.");
  await phrase.evaluate((node) => {
    const selection = window.getSelection();
    selection?.removeAllRanges();
    const range = document.createRange();
    range.selectNodeContents(node);
    selection?.addRange(range);
    node.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));
  });
  await expect(page.getByRole("button", { name: "高亮" })).toBeVisible();
  await page.getByRole("button", { name: "高亮" }).click();
  await expect(article.locator("mark")).toHaveCount(1);
  await page.getByRole("button", { name: "快捷笔记" }).click();
  await page.getByPlaceholder("记一句想法…").fill("Remember this phrase");
  await page.getByRole("button", { name: "保存笔记" }).click();
  await expect(page.getByText("Remember this phrase")).toBeVisible();
  await page.getByRole("button", { name: "标记已读" }).click();
  await expect(page.getByRole("button", { name: "已读" })).toBeVisible();
});

test("UI showcase keeps dialog accessible and reduced-motion compatible", async ({ page }) => {
  await installMocks(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/app/system/ui");
  await page.getByRole("tab", { name: "Overlay / Feedback" }).click();
  await page.getByRole("button", { name: "Dialog", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Dialog" });
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveAttribute("aria-modal", "true");
});
