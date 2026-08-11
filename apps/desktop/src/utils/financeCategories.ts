import type { FinanceCategory, Transaction } from "@/src/types";

export const DEFAULT_FINANCE_CATEGORIES: Record<"expense" | "income", string[]> = {
  expense: ["餐饮", "交通", "购物", "居住", "水电通信", "医疗健康", "教育", "娱乐", "人情往来", "旅行", "保险", "其他支出"],
  income: ["工资", "奖金", "兼职", "投资收益", "退款", "礼金", "其他收入"],
};

export function categoryNames(
  categories: FinanceCategory[],
  type: Transaction["type"],
  current?: string,
) {
  if (type === "transfer") return ["账户转账"];
  const persisted = categories
    .filter((item) => item.type === type && !item.isArchived)
    .map((item) => item.name);
  return Array.from(new Set([
    ...(current ? [current] : []),
    ...DEFAULT_FINANCE_CATEGORIES[type],
    ...persisted,
  ]));
}
