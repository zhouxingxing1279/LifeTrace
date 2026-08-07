export const pad = (value: number) => String(value).padStart(2, "0");

export const dayKey = (date = new Date()) =>
  `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;

export const money = (value: number) =>
  `¥${value.toLocaleString("zh-CN", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;

export const transactionAmountText = (
  transaction: Pick<TransactionLike, "type" | "amount">,
) => {
  if (transaction.type === "expense") return `-${money(transaction.amount)}`;
  if (transaction.type === "income") return `+${money(transaction.amount)}`;
  return money(transaction.amount);
};

interface TransactionLike {
  type: "expense" | "income" | "transfer";
  amount: number;
}

export const dateTimeLocal = (value?: string) => {
  const date = value ? new Date(value) : new Date();
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60000);
  return local.toISOString().slice(0, 16);
};

export const escapeHtml = (value: string) =>
  value.replace(
    /[&<>"]/g,
    (character) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[character]!,
  );
