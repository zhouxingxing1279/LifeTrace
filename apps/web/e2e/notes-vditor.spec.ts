import { expect, test, type Page, type Route } from "@playwright/test";

const now = "2026-08-19T02:00:00.000Z";
const cacheKey = "lifetrace:vditor:user-1:note-1";

type PushBody = { changes?: Array<{ entityType?: string; payload?: Record<string, unknown> }> };

function meta(id: string) {
  return { id, userId: "user-1", createdAt: now, updatedAt: now, localVersion: 1, serverVersion: "1", modifiedByDevice: "web-test" };
}

async function json(route: Route, payload: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(payload) });
}

async function installMocks(page: Page, pushes: PushBody[], cloudMarkdown = "# Cloud note\n\nCloud body") {
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
        items: [{
          entityType: "note.note",
          entityId: "note-1",
          serverVersion: "1",
          payload: {
            meta: meta("note-1"),
            noteType: "quick",
            title: "Vditor note",
            contentMarkdown: cloudMarkdown,
            contentText: "Cloud note Cloud body",
            contentHtml: "",
            contentJson: { type: "markdown", source: cloudMarkdown },
            summary: "Cloud note Cloud body",
            isPinned: false,
            isFavorite: false,
            isArchived: false,
            folderId: null,
          },
        }],
        nextPageToken: null,
        completed: true,
      });
    }
    if (path === "/api/v1/sync/pull") return json(route, { changes: [], nextCursor: "cursor-2", hasMore: false });
    if (path === "/api/v1/sync/push") {
      const body = route.request().postDataJSON() as PushBody;
      pushes.push(body);
      return json(route, {
        results: (body.changes ?? []).map((change, index) => ({
          changeId: `change-${index}`,
          entityType: change.entityType,
          entityId: "note-1",
          status: "accepted",
          serverVersion: `server-${index + 2}`,
        })),
      });
    }
    return json(route, {});
  });
}

function noteMarkdownPush(pushes: PushBody[]) {
  for (const body of pushes) {
    for (const change of body.changes ?? []) {
      if (change.entityType === "note.note" && typeof change.payload?.contentMarkdown === "string") {
        return change.payload.contentMarkdown;
      }
    }
  }
  return null;
}

test("Vditor edits Markdown and autosaves the note to LifeTrace Cloud", async ({ page }) => {
  const pushes: PushBody[] = [];
  await installMocks(page, pushes);
  await page.goto("/app/notes");

  await expect(page.getByRole("heading", { name: "笔记", level: 1 })).toBeVisible();
  const editor = page.locator(".vditor-ir pre[contenteditable='true']");
  await expect(editor).toBeVisible();
  await expect(editor).toContainText("Cloud body");
  await expect(page.getByRole("button", { name: /编辑模式/ })).toBeVisible();

  await editor.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.type("# Local Vditor edit\n\n- [x] cloud autosave");

  await expect.poll(() => noteMarkdownPush(pushes), { timeout: 6_000 }).toContain("Local Vditor edit");
  expect(noteMarkdownPush(pushes)).toContain("cloud autosave");
  await expect(page.getByText("已保存", { exact: true })).toBeVisible();
});

test("dirty Vditor localStorage draft is restored and promoted to Cloud autosave", async ({ page }) => {
  const pushes: PushBody[] = [];
  await page.addInitScript(({ key }) => {
    localStorage.setItem(key, "# Recovered draft\n\nLocal unsaved text");
    localStorage.setItem(`${key}:meta`, JSON.stringify({ dirty: true, updatedAt: new Date().toISOString() }));
  }, { key: cacheKey });
  await installMocks(page, pushes, "# Cloud version\n\nOlder cloud text");
  await page.goto("/app/notes");

  const editor = page.locator(".vditor-ir pre[contenteditable='true']");
  await expect(editor).toBeVisible();
  await expect(editor).toContainText("Recovered draft");
  await expect.poll(() => noteMarkdownPush(pushes), { timeout: 6_000 }).toContain("Recovered draft");
  expect(noteMarkdownPush(pushes)).toContain("Local unsaved text");
});
