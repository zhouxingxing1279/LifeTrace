import { Pencil, Plus, Trash2 } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { FinanceAccount } from "@/src/types";
import ContextMenu from "@/src/ui/menu/ContextMenu";
import MoreMenu from "@/src/ui/menu/MoreMenu";
import type { AppAction } from "@/src/ui/actions/types";
import { confirmAction } from "@/src/ui/feedback/confirm";
import { getAccountBalanceSnapshot } from "@/src/utils/finance";
import { money } from "@/src/utils/format";
import { EmptyState } from "@/src/components/common";
import { Button } from "@/src/components/ui";

export default function Accounts({
  edit,
}: {
  edit: (value?: FinanceAccount) => void;
}) {
  const { accounts, transactions, deleteAccount } = useLifeStore();
  const actionsFor = (account: FinanceAccount): AppAction<FinanceAccount>[] => [
    {
      id: "edit",
      label: "编辑账户",
      icon: Pencil,
      group: "primary",
      execute: edit,
    },
    {
      id: "delete",
      label: "删除账户",
      icon: Trash2,
      group: "danger",
      danger: true,
      execute: async (context) => {
        if (
          await confirmAction({
            title: "删除这个账户？",
            description: `账户“${context.name}”将被删除。请先确认相关账单不再需要该账户。`,
            confirmLabel: "删除账户",
          })
        ) {
          await deleteAccount(context.id);
        }
      },
    },
  ];

  return (
    <div className="hx-view">
      <div className="hx-toolbar">
        <span className="hx-toolbar-summary">
          余额按基准时间和后续流水自动计算
        </span>
        <Button
          variant="primary"
          icon={<Plus aria-hidden="true" />}
          onClick={() => edit()}
        >
          添加账户
        </Button>
      </div>
      <div className="hx-account-list">
        {accounts.map((account) => {
          const snapshot = getAccountBalanceSnapshot(account, transactions);
          return (
            <ContextMenu
              as="article"
              className="hx-account-row"
              actions={actionsFor(account)}
              context={account}
              ariaLabel={`${account.name}操作`}
              key={account.id}
            >
              <i
                className="hx-account-icon"
                style={{ background: account.color }}
              >
                {account.icon}
              </i>
              <div className="hx-account-copy">
                <h3>{account.name}</h3>
                <p>
                  {account.type}
                  {account.last4 ? ` · 尾号 ${account.last4}` : ""}
                </p>
              </div>
              <div className="hx-account-balance">
                <strong>
                  {snapshot.currentBalance === null
                    ? "未设置"
                    : money(snapshot.currentBalance)}
                </strong>
                <small>
                  {snapshot.hasBaseline && account.balanceAt
                    ? `基准 ${new Date(account.balanceAt).toLocaleDateString("zh-CN")} · 后续 ${snapshot.transactionCount} 笔`
                    : "尚未设置余额基准"}
                </small>
              </div>
              <div className="hx-account-actions">
                <Button variant="secondary" onClick={() => edit(account)}>
                  编辑
                </Button>
                <MoreMenu
                  actions={actionsFor(account)}
                  context={account}
                  label={`${account.name}更多操作`}
                />
              </div>
            </ContextMenu>
          );
        })}
        {!accounts.length ? (
          <EmptyState
            title="还没有账户"
            hint="添加账户后，余额和流水会在这里统一计算。"
          />
        ) : null}
      </div>
    </div>
  );
}
