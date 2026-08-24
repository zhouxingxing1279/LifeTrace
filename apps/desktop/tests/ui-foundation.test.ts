import assert from "node:assert/strict";
import test from "node:test";
import {
  dayKey,
  dateTimeLocal,
  escapeHtml,
  money,
  transactionAmountText,
} from "../src/utils/format";
import {
  filterCommandItems,
  groupCommandItems,
} from "../src/components/layout/commandModel";
import type { CommandItem } from "../src/components/layout/CommandPalette";
import { navGroups, pageTitles } from "../src/components/layout/navigation";

const item = (overrides: Partial<CommandItem>): CommandItem => ({
  id: "id",
  label: "示例命令",
  group: "操作",
  execute: () => undefined,
  ...overrides,
});

test("dayKey formats local date with zero padding", () => {
  assert.equal(dayKey(new Date(2026, 7, 8)), "2026-08-08");
  assert.equal(dayKey(new Date(2026, 0, 3)), "2026-01-03");
});

test("money formats cents with ¥ and grouping", () => {
  assert.equal(money(1234.5), "¥1,234.50");
  assert.equal(money(0), "¥0.00");
});

test("transactionAmountText renders direction-aware amounts", () => {
  assert.equal(transactionAmountText({ type: "expense", amount: 12 }), "-¥12.00");
  assert.equal(transactionAmountText({ type: "income", amount: 12 }), "+¥12.00");
  assert.equal(transactionAmountText({ type: "transfer", amount: 12 }), "¥12.00");
});

test("dateTimeLocal keeps a local date string", () => {
  const value = dateTimeLocal("2026-08-08T00:00:00.000Z");
  assert.match(value, /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/);
});

test("escapeHtml escapes angle brackets, ampersands and quotes", () => {
  assert.equal(escapeHtml(`<b>"&"</b>`), "&lt;b&gt;&quot;&amp;&quot;&lt;/b&gt;");
});

test("command filter matches label, hint and keywords case-insensitively", () => {
  const commands = [
    item({ id: "a", label: "前往账单管理", hint: "资产与账单", group: "跳转" }),
    item({ id: "b", label: "新建坚持项目", keywords: "habit 习惯", group: "新建" }),
    item({ id: "c", label: "打开设置", group: "操作" }),
  ];
  assert.deepEqual(
    filterCommandItems(commands, "").map((c) => c.id),
    ["a", "b", "c"],
  );
  assert.deepEqual(
    filterCommandItems(commands, "账单").map((c) => c.id),
    ["a"],
  );
  assert.deepEqual(
    filterCommandItems(commands, "HABIT").map((c) => c.id),
    ["b"],
  );
  assert.deepEqual(filterCommandItems(commands, "不存在"), []);
});

test("command groups preserve order and stay contiguous", () => {
  const commands = [
    item({ id: "a", group: "跳转" }),
    item({ id: "b", group: "跳转" }),
    item({ id: "c", group: "新建" }),
    item({ id: "d", group: "操作" }),
    item({ id: "e", group: "新建" }),
  ];
  const groups = groupCommandItems(commands);
  assert.deepEqual(
    groups.map((group) => group.group),
    ["跳转", "新建", "操作", "新建"],
  );
  assert.equal(groups[0].items.length, 2);
});

test("every sidebar navigation target has a page title", () => {
  for (const group of navGroups) {
    for (const item of group.items) {
      assert.ok(
        item.id in pageTitles,
        `navigation target "${item.id}" is missing a page title`,
      );
    }
  }
});
