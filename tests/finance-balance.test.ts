import assert from "node:assert/strict";
import test from "node:test";
import { getAccountBalanceSnapshot } from "../src/utils/finance";
import type { FinanceAccount, Transaction } from "../src/types";

const account = (id: string, balance: number, balanceAt?: string): FinanceAccount => ({
  id,
  userId: "local-user",
  name: id,
  type: "bank",
  balance,
  balanceAt,
  color: "#000000",
  icon: "账",
  isArchived: false,
  createdAt: "2026-08-01T00:00:00.000Z",
  updatedAt: "2026-08-01T00:00:00.000Z",
});

const transaction = (overrides: Partial<Transaction>): Transaction => ({
  id: Math.random().toString(),
  userId: "local-user",
  type: "expense",
  amount: 0,
  category: "测试",
  account: "bank-a",
  accountId: "bank-a",
  occurredAt: "2026-08-02T00:00:00.000Z",
  createdAt: "2026-08-02T00:00:00.000Z",
  updatedAt: "2026-08-02T00:00:00.000Z",
  ...overrides,
});

test("uses only matched transactions after the balance baseline", () => {
  const result = getAccountBalanceSnapshot(account("bank-a", 1000, "2026-08-01T00:00:00.000Z"), [
    transaction({ amount: 200 }),
    transaction({ type: "income", amount: 50 }),
    transaction({ amount: 999, occurredAt: "2026-07-31T00:00:00.000Z" }),
    transaction({ amount: 999, accountId: "bank-b", account: "bank-b" }),
  ]);
  assert.equal(result.currentBalance, 850);
  assert.equal(result.income, 50);
  assert.equal(result.expense, 200);
  assert.equal(result.transactionCount, 2);
});

test("keeps legacy manual balances unchanged until a baseline time is set", () => {
  const result = getAccountBalanceSnapshot(account("bank-a", 123.45), [transaction({ amount: 20 })]);
  assert.equal(result.currentBalance, 123.45);
  assert.equal(result.hasBaseline, false);
});

test("moves money between two matched accounts without changing total assets", () => {
  const transfer = transaction({ type: "transfer", amount: 125.25, toAccount: "wallet", toAccountId: "wallet" });
  const source = getAccountBalanceSnapshot(account("bank-a", 500, "2026-08-01T00:00:00.000Z"), [transfer]);
  const destination = getAccountBalanceSnapshot(account("wallet", 100, "2026-08-01T00:00:00.000Z"), [transfer]);
  assert.equal(source.currentBalance, 374.75);
  assert.equal(destination.currentBalance, 225.25);
  assert.equal((source.currentBalance ?? 0) + (destination.currentBalance ?? 0), 600);
});

test("does not change balances for a transfer with an unmatched endpoint", () => {
  const result = getAccountBalanceSnapshot(account("bank-a", 500, "2026-08-01T00:00:00.000Z"), [
    transaction({ type: "transfer", amount: 125.25, toAccount: "unknown", toAccountId: undefined }),
  ]);
  assert.equal(result.currentBalance, 500);
  assert.equal(result.transactionCount, 0);
});
