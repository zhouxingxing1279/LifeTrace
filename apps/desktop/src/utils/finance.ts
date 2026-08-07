import type { FinanceAccount, Transaction } from "../types";

export interface AccountBalanceSnapshot {
  currentBalance: number | null;
  income: number;
  expense: number;
  transactionCount: number;
  hasBaseline: boolean;
}

const toCents = (value: number) => Math.round(value * 100);

export const transactionBelongsToAccount = (transaction: Transaction, account: FinanceAccount) =>
  transaction.accountId
    ? transaction.accountId === account.id
    : transaction.account === account.name;

export const getAccountBalanceSnapshot = (
  account: FinanceAccount,
  transactions: Transaction[],
): AccountBalanceSnapshot => {
  if (account.balance === null) {
    return { currentBalance: null, income: 0, expense: 0, transactionCount: 0, hasBaseline: false };
  }

  const baselineTime = account.balanceAt ? Date.parse(account.balanceAt) : Number.NaN;
  if (!Number.isFinite(baselineTime)) {
    return { currentBalance: account.balance, income: 0, expense: 0, transactionCount: 0, hasBaseline: false };
  }

  let balanceCents = toCents(account.balance);
  let incomeCents = 0;
  let expenseCents = 0;
  let transactionCount = 0;

  for (const transaction of transactions) {
    const occurredAt = Date.parse(transaction.occurredAt);
    if (!Number.isFinite(occurredAt) || occurredAt <= baselineTime) continue;
    const amountCents = toCents(transaction.amount);
    if (transaction.type === "transfer") {
      if (!transaction.accountId || !transaction.toAccountId) continue;
      const isSource = transaction.accountId === account.id;
      const isDestination = transaction.toAccountId === account.id;
      if (!isSource && !isDestination) continue;
      if (isSource) balanceCents -= amountCents;
      if (isDestination) balanceCents += amountCents;
    } else if (!transactionBelongsToAccount(transaction, account)) {
      continue;
    } else if (transaction.type === "income") {
      incomeCents += amountCents;
      balanceCents += amountCents;
    } else {
      expenseCents += amountCents;
      balanceCents -= amountCents;
    }
    transactionCount += 1;
  }

  return {
    currentBalance: balanceCents / 100,
    income: incomeCents / 100,
    expense: expenseCents / 100,
    transactionCount,
    hasBaseline: true,
  };
};

export const getTotalAccountBalance = (accounts: FinanceAccount[], transactions: Transaction[]) =>
  accounts.reduce((total, account) => total + (getAccountBalanceSnapshot(account, transactions).currentBalance ?? 0), 0);
