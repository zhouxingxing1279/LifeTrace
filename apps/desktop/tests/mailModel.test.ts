import assert from "node:assert/strict";
import test from "node:test";
import type { MailMessageSummary } from "../src/services/mailApi";
import { groupBySender, shouldAggregateBySender, sourceQuery } from "../src/components/feature/mail/mailModel";

function message(id: string, from: unknown, receivedAt: string, isRead = true): MailMessageSummary {
  return {
    id,
    accountId: "11111111-1111-1111-1111-111111111111",
    folderId: "22222222-2222-2222-2222-222222222222",
    threadId: "33333333-3333-3333-3333-333333333333",
    subject: `subject-${id}`,
    fromJson: from,
    toJson: [],
    sentAt: null,
    receivedAt,
    isRead,
    isArchived: false,
    snippet: null,
    hasAttachments: false,
  };
}

test("unified source omits account and folder filters", () => {
  assert.deepEqual(sourceQuery({ kind: "unified" }, " hello "), { q: "hello" });
});

test("unread source enables unread filter", () => {
  assert.deepEqual(sourceQuery({ kind: "unread" }, ""), { q: undefined, unreadOnly: true });
});

test("account source scopes account id", () => {
  assert.deepEqual(sourceQuery({ kind: "account", accountId: "account-1" }, ""), {
    q: undefined,
    accountId: "account-1",
  });
});

test("folder source scopes account and folder ids", () => {
  assert.deepEqual(sourceQuery({ kind: "folder", accountId: "account-1", folderId: "folder-1", role: "sent", name: "已发送" }, "x"), {
    q: "x",
    accountId: "account-1",
    folderId: "folder-1",
  });
});

test("sender grouping canonicalizes email case and keeps newest message", () => {
  const groups = groupBySender([
    message("old", { address: "Team@Example.com", name: "Team" }, "2026-08-10T09:00:00Z"),
    message("new", { address: "team@example.com", name: "Team" }, "2026-08-11T09:00:00Z", false),
  ]);
  assert.equal(groups.length, 1);
  assert.equal(groups[0]?.key, "team@example.com");
  assert.equal(groups[0]?.messages.length, 2);
  assert.equal(groups[0]?.latest.id, "new");
  assert.equal(groups[0]?.unreadCount, 1);
});

test("messages without a usable sender address do not collapse together", () => {
  const groups = groupBySender([
    message("one", {}, "2026-08-10T09:00:00Z"),
    message("two", {}, "2026-08-11T09:00:00Z"),
  ]);
  assert.equal(groups.length, 2);
});

test("sent and other non-inbox folders are concrete-message lists", () => {
  assert.equal(shouldAggregateBySender({ kind: "folder", accountId: "a", folderId: "f", role: "sent", name: "已发送" }), false);
  assert.equal(shouldAggregateBySender({ kind: "folder", accountId: "a", folderId: "f", role: "inbox", name: "收件箱" }), true);
  assert.equal(shouldAggregateBySender({ kind: "unified" }), true);
});
