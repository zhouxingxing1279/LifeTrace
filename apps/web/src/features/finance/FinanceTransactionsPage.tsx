import { FinanceWorkspace } from "./FinanceWorkspace";

/**
 * Transactions is part of the BeeCount-backed finance workspace.
 * Keeping this route component preserves direct-route compatibility without
 * maintaining a second LifeTrace-native finance implementation.
 */
export function FinanceTransactionsPage() {
  return <FinanceWorkspace />;
}
