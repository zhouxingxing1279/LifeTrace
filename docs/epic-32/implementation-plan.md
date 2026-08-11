# EPIC-32 — Cypht-inspired Mail V2

## Goal

Refactor LifeTrace Mail from a single-account inbox page into a multi-account mail aggregator inspired by Cypht's core product model: combined views across accounts, per-account folder browsing, modular mail surfaces, and standard mail compose/read flows.

This is an architectural and UX reference only. Do not copy Cypht source code.

## Product principles

1. **Combined first** — the default entry is a unified inbox across all connected accounts.
2. **Accounts remain visible** — users can browse each account and its folders independently.
3. **List first** — inbox/folder screens are mail lists; message bodies only appear in the message detail view.
4. **Sender aggregation is preserved** — inbox-style views aggregate messages by canonical sender address; the group row shows the number of messages. Clicking a group opens that sender's list, then a concrete mail opens detail.
5. **Folder semantics matter** — sent/draft/trash/spam/other folders show individual messages instead of sender aggregation.
6. **Compose is a first-class action** — `写邮件` opens a modal composer. Reply uses the same modal composition model.
7. **LifeTrace actions stay in detail** — archive/read state/reply and conversions to Task/Calendar/Memo/Waiting remain on the concrete message detail page.
8. **Existing sync policy stays unchanged** — initial mail sync remains limited to the latest 30 days, followed by UID-based incremental sync. This EPIC does not introduce full-history backfill.

## Target information architecture

```text
LifeTrace main navigation
└── 邮件
    ├── Mail sidebar
    │   ├── 写邮件
    │   ├── 统一收件箱
    │   ├── 未读邮件
    │   └── 邮箱账户
    │       ├── QQ account
    │       │   ├── 收件箱
    │       │   ├── 已发送
    │       │   ├── 归档
    │       │   └── other synced folders
    │       └── 126 account
    │           └── ...
    └── Content
        ├── list view
        ├── sender aggregate list
        └── message detail
```

## View model

Introduce an explicit collection source abstraction instead of using `selectedAccountId` as the application state:

- `unified-inbox`: all accounts, inbox role
- `unread`: all accounts, inbox role, unread only
- `account-inbox`: one account, inbox role
- `folder`: one account + concrete folder id

A list screen can optionally carry `senderKey` for the sender aggregation drill-down. A detail screen stores its previous list context so Back returns to the exact source/sender list.

## UI scope

### Mail sidebar

- Primary `写邮件` action.
- Unified inbox and unread combined views.
- Connected accounts listed below.
- Each account can expose its discovered/synced IMAP folders.
- Account state is visible without occupying the mail list toolbar.
- `添加邮箱` remains available.

### Mail list

- Search across the active source.
- Sync action: selected account when browsing one account; all active accounts from combined views.
- Inbox-like sources aggregate by sender email.
- Aggregate row shows sender, message count, latest subject/snippet/time, unread state, attachment indicator, and target account when the source is combined.
- Non-inbox folders show one row per concrete message.
- 30-day scope remains visible.

### Sender list

- Opens only from a multi-message sender aggregate.
- Shows all matching concrete messages within the parent source/search result.
- Clicking a row opens concrete detail.
- Back returns to the parent collection.

### Message detail

- Back to exact previous list context.
- Subject, sender, recipients, account, sent/received time.
- Sanitized HTML/plain-text body with safe clickable links.
- Attachment download.
- Read/unread, archive, reply.
- LifeTrace conversions: Task, Calendar Event, Memo, Waiting.

### Compose / reply

- Modal composer; never append a form below a long mail body.
- New mail: From account, To, optional Cc/Bcc, Subject, Body.
- Reply: prefilled account, recipient, `Re:` subject and `inReplyToMessageId`.
- Reuse existing SMTP send endpoint and confirmation flow.
- Outbound attachments are explicitly out of scope until the backend send contract supports them.

## Backend/API reuse

No protocol rewrite is required for this EPIC:

- `GET /api/v1/mail/messages` already supports omitting `accountId`, enabling unified inbox queries.
- `accountId`, `folderId`, `q`, `unreadOnly`, `limit`, and `offset` cover the required collection views.
- `GET /api/v1/mail/accounts/{id}/folders` supplies the account folder tree.
- existing send/read/archive/attachment endpoints are reused.

Provider-specific IMAP logic, including the already-fixed NetEase IMAP ID handshake, must not be changed unless CI reveals a regression.

## Implementation structure

Break the existing ~38 KB `MailActionCenter.tsx` monolith into focused mail module files:

- `MailActionCenter.tsx` — orchestration/state/data loading
- `mailViewModel.ts` — source/query/sender/address helpers
- `MailSidebar.tsx` — combined views + account/folder navigation
- `MailMessageList.tsx` — aggregate rows and concrete rows
- `MailMessageDetail.tsx` — message reading and LifeTrace actions
- `MailComposerDialog.tsx` — new mail and reply modal
- `MailAccountDialog.tsx` — account connection form

## Tests

Add pure TypeScript unit tests for the view model:

- unified inbox query omits account/folder filters
- unread source sets unread filter
- account inbox scopes account id
- folder source scopes folder id
- sender grouping canonicalizes email case
- unknown sender does not incorrectly combine unrelated messages
- sent/non-inbox source is not sender-aggregated

Formal merge gates:

1. `Browser Web`
2. `EPIC-05 Windows Sync`
3. `EPIC-03 PostgreSQL` only if backend/schema files are changed

Merge to `main` only after all applicable workflows pass on the exact final head.

## Non-goals

- Full historical sync beyond 30 days
- JMAP/EWS protocol implementation
- AI mail summarization or autonomous actions
- outbound attachment upload/send
- server-side contacts/address-book implementation
- copying Cypht PHP/JS source

## Acceptance criteria

- Mail opens into a unified inbox across connected accounts.
- User can switch to unread combined view, one account inbox, or a discovered folder.
- Inbox-style lists preserve same-sender aggregation with visible counts.
- Sender group → concrete list → concrete detail navigation works with correct Back behavior.
- Message detail uses the message's own account for reply, even when opened from unified inbox.
- `写邮件` can send a new message through a selected connected account.
- Reply remains a centered modal composer.
- Existing archive/read/attachments/LifeTrace conversion functions remain available on detail.
- Existing 30-day initial sync behavior remains unchanged.
- Final diff is modular and no longer concentrates the whole mail UI in one component.