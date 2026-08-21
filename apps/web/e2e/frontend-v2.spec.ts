import { expect, test } from "@playwright/test";

const routes = ["/app/today", "/app/execution", "/app/calendar", "/app/habits", "/app/fitness", "/app/health", "/app/notes", "/app/english/articles", "/app/review", "/app/finance", "/app/finance/transactions", "/app/finance/calendar", "/app/finance/ledgers", "/app/finance/budgets", "/app/finance/accounts", "/app/finance/categories", "/app/finance/tags", "/app/finance/import", "/app/settings", "/app/system/ui"];

test("all v2 routes render the shared LifeTrace shell", async ({ page }) => {
  for (const route of routes) {
    await page.goto(route);
    await expect(page.locator("body")).toContainText("LifeTrace");
    await expect(page.locator("main")).toBeVisible();
  }
});

test("quick capture persists a task across navigation", async ({ page }) => {
  await page.goto("/app/today");
  await page.getByRole("button", { name: "Quick Capture" }).click();
  await page.getByPlaceholder("下一步要做什么？").fill("Validate clean-room rewrite");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("Validate clean-room rewrite")).toBeVisible();
  await page.goto("/app/execution");
  await expect(page.getByText("Validate clean-room rewrite")).toBeVisible();
});

test("keyboard command palette is usable", async ({ page }) => {
  await page.goto("/app/today");
  await page.keyboard.press(process.platform === "darwin" ? "Meta+K" : "Control+K");
  const dialog = page.getByRole("dialog", { name: "命令菜单" });
  await expect(dialog).toBeVisible();
  await dialog.getByPlaceholder("搜索页面、任务、笔记…").fill("Finance");
  await dialog.getByRole("button", { name: "Finance" }).click();
  await expect(page).toHaveURL(/\/app\/finance$/);
});

test("theme and responsive layout have no horizontal page overflow", async ({ page }) => {
  await page.goto("/app/settings");
  await page.getByLabel("切换主题").click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", /dark|light/);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
  expect(overflow).toBe(false);
});
