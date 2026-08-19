import { expect, test, type Page, type Route } from "@playwright/test";

const now = "2026-08-19T02:00:00.000Z";
const pushes: Array<Record<string, unknown>> = [];

function meta(id: string) {
  return { id, userId: "user-1", createdAt: now, updatedAt: now, localVersion: 1, serverVersion: "1", modifiedByDevice: "web-test" };
}

async function json(route: Route, payload: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(payload) });
}

async function installMocks(page: Page) {
  pushes.length = 0;
  await page.route("**/api/v1/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/v1/web/session") {
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
      return json(route, {
        snapshotId: "snapshot-1",
        snapshotCursor: "cursor-1",
        nextPageToken: null,
        completed: true,
        items: [{
          entityType: "note.note",
          entityId: "note-1",
          serverVersion: "1",
          payload: {
            meta: meta("note-1"),
            title: "Markdown Notes",
            contentText: "# Existing\n\n- [ ] Task",
            contentMarkdown: "# Existing\n\n- [ ] Task",
            summary: "Existing Task",
            isArchived: false,
          },
        }],
      });
    }
    if (path === "/api/v1/sync/push") {
      const body = route.request().postDataJSON() as { changes?: Array<Record<string, unknown>> };
      pushes.push(...(body.changes ?? []));
      return json(route, {
        results: (body.changes ?? []).map((change, index) => ({
          changeId: change.changeId,
          entityType: change.entityType,
          entityId: change.entityId,
          status: "accepted",
          serverVersion: `server-${index + 2}`,
        })),
      });
    }
    if (path === "/api/v1/sync/pull") return json(route, { changes: [], nextCursor: "cursor-2", hasMore: false });
    if (path === "/api/v1/integrations/beecount/status") return json(route, { enabled: false, readOnly: true, source: "beecount-cloud", upstreamReachable: false });
    if (path === "/api/v1/web/devices") return json(route, { devices: [] });
    if (path === "/api/v1/web/sessions") return json(route, { sessions: [] });
    return json(route, {});
  });
}

test("notes provide Markdown formatting, split preview and Cloud save", async ({ page }) => {
  await installMocks(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/app/notes");

  await expect(page.getByRole("heading", { name: "笔记", level: 1 })).toBeVisible();
  const editor = page.getByLabel("Markdown 正文");
  await expect(editor).toHaveValue("# Existing\n\n- [ ] Task");
  await expect(page.getByLabel("Markdown 预览").getByRole("heading", { name: "Existing", level: 1 })).toBeVisible();

  await editor.fill("# Markdown Editor\n\nhello");
  await editor.evaluate((element) => {
    const textarea = element as HTMLTextAreaElement;
    textarea.focus();
    textarea.setSelectionRange(19, 24);
  });
  await page.getByRole("button", { name: "粗体 (Ctrl/Cmd+B)" }).click();
  await expect(editor).toHaveValue("# Markdown Editor\n\n**hello**");
  await expect(page.getByLabel("Markdown 预览").getByText("hello", { exact: true })).toBeVisible();
  await expect(page.getByText("未保存更改")).toBeVisible();

  await editor.press("Control+S");
  await expect.poll(() => pushes.length).toBeGreaterThan(0);
  expect(JSON.stringify(pushes.at(-1))).toContain("contentMarkdown");
  expect(JSON.stringify(pushes.at(-1))).toContain("**hello**");
});
