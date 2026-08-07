import { useState } from "react";
import { X } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type {
  Activity,
  ActivityLog,
  FinanceAccount,
  Transaction,
} from "@/src/types";
import PersistProjectDialog from "@/src/components/persist-project/PersistProjectDialog";
import { getAccountBalanceSnapshot } from "@/src/utils/finance";
import { dateTimeLocal, money } from "@/src/utils/format";
import { notify } from "@/src/ui/feedback/toastBus";

export type EditorModalState =
  | null
  | { kind: "activity"; value?: Activity }
  | { kind: "record"; value: Activity }
  | { kind: "transaction"; value?: Transaction }
  | { kind: "account"; value?: FinanceAccount };

export default function EditorModal({
  modal,
  close,
}: {
  modal: EditorModalState;
  close: () => void;
}) {
  if (modal?.kind === "record") return <RecordForm activity={modal.value} close={close} />;
  if (modal?.kind === "activity")
    return <PersistProjectDialog activity={modal.value} onClose={close} />;
  if (modal?.kind === "transaction")
    return <TransactionForm value={modal.value} close={close} />;
  if (modal?.kind === "account")
    return <AccountForm value={modal.value} close={close} />;
  return null;
}

function ModalFrame({
  title,
  close,
  children,
}: {
  title: string;
  close: () => void;
  children: React.ReactNode;
}) {
  return (
    <div
      className="hx-overlay"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) close();
      }}
    >
      <div className="hx-modal" role="dialog" aria-modal="true" aria-label={title}>
        <header>
          <div>
            <span className="hx-kicker">编辑内容</span>
            <h2>{title}</h2>
          </div>
          <button type="button" aria-label="关闭" onClick={close}>
            <X />
          </button>
        </header>
        {children}
      </div>
    </div>
  );
}

function RecordForm({
  activity,
  close,
}: {
  activity: Activity;
  close: () => void;
}) {
  const { addLog } = useLifeStore();
  const [value, setValue] = useState(activity.normalTarget ?? 1);
  const [status, setStatus] =
    useState<NonNullable<ActivityLog["status"]>>("completed");
  const [state, setState] =
    useState<NonNullable<ActivityLog["metadata"]>["state"]>("stable");
  const [urgeLevel, setUrgeLevel] = useState(5);
  const [triggers, setTriggers] = useState<string[]>([]);
  const [actions, setActions] = useState<string[]>([]);
  const [note, setNote] = useState("");
  const triggerOptions = ["压力", "疲劳", "无聊", "社交场景", "环境诱因"];
  const actionOptions = ["离开现场", "喝水", "短暂散步", "呼吸放松", "联系支持者"];
  const toggle = (
    list: string[],
    item: string,
    set: (value: string[]) => void,
  ) => set(list.includes(item) ? list.filter((value) => value !== item) : [...list, item]);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (activity.type === "control") {
      await addLog(
        activity.id,
        undefined,
        "completed",
        {
          state,
          urgeLevel: state === "stable" ? undefined : urgeLevel,
          triggers: state === "stable" ? [] : triggers,
          actions: state === "stable" ? [] : actions,
        },
        note,
      );
    } else if (activity.type === "completion") {
      await addLog(activity.id, undefined, status, undefined, note);
    } else {
      await addLog(activity.id, value, "completed", undefined, note);
    }
    notify(`${activity.name}已记录`);
    close();
  };

  return (
    <ModalFrame title={`记录：${activity.name}`} close={close}>
      <form className="hx-form hx-record-form" onSubmit={submit}>
        {activity.type === "control" ? (
          <>
            <label>
              当前状态
              <div className="hx-choice-row">
                {(
                  [
                    ["stable", "保持稳定"],
                    ["urge", "出现冲动"],
                    ["relapse", "发生偏离"],
                  ] as const
                ).map(([id, label]) => (
                  <button
                    type="button"
                    key={id}
                    className={state === id ? "active" : ""}
                    onClick={() => setState(id)}
                  >
                    {label}
                  </button>
                ))}
              </div>
            </label>
            {state !== "stable" ? (
              <>
                <label className="hx-range">
                  <span>
                    冲动强度<b>{urgeLevel}/10</b>
                  </span>
                  <input
                    type="range"
                    min="1"
                    max="10"
                    value={urgeLevel}
                    onChange={(event) => setUrgeLevel(Number(event.target.value))}
                  />
                </label>
                <fieldset className="hx-choice-group">
                  <legend>可能诱因（可多选）</legend>
                  <div>
                    {triggerOptions.map((item) => (
                      <button
                        type="button"
                        key={item}
                        className={triggers.includes(item) ? "active" : ""}
                        onClick={() => toggle(triggers, item, setTriggers)}
                      >
                        {item}
                      </button>
                    ))}
                  </div>
                </fieldset>
                <fieldset className="hx-choice-group">
                  <legend>已采取行动（可多选）</legend>
                  <div>
                    {actionOptions.map((item) => (
                      <button
                        type="button"
                        key={item}
                        className={actions.includes(item) ? "active" : ""}
                        onClick={() => toggle(actions, item, setActions)}
                      >
                        {item}
                      </button>
                    ))}
                  </div>
                </fieldset>
              </>
            ) : null}
          </>
        ) : activity.type === "completion" ? (
          <label>
            完成情况
            <div className="hx-choice-row">
              {(
                [
                  ["completed", "已完成"],
                  ["partial", "部分完成"],
                  ["skipped", "今天跳过"],
                ] as const
              ).map(([id, label]) => (
                <button
                  type="button"
                  key={id}
                  className={status === id ? "active" : ""}
                  onClick={() => setStatus(id)}
                >
                  {label}
                </button>
              ))}
            </div>
          </label>
        ) : (
          <label>
            本次完成量
            <input
              autoFocus
              required
              type="number"
              min="0"
              step={activity.type === "duration" ? "1" : "0.1"}
              value={value}
              onChange={(event) => setValue(Number(event.target.value))}
            />
            <small>
              单位：{activity.unit}，目标：{activity.normalTarget ?? 1} {activity.unit}
            </small>
          </label>
        )}
        <label>
          备注（可选）
          <textarea
            value={note}
            onChange={(event) => setNote(event.target.value)}
            placeholder="记录当时的情况或感受"
          />
        </label>
        <footer>
          <button type="button" className="hx-btn secondary" onClick={close}>
            取消
          </button>
          <button type="submit" className="hx-btn primary">
            保存记录
          </button>
        </footer>
      </form>
    </ModalFrame>
  );
}

function TransactionForm({
  value,
  close,
}: {
  value?: Transaction;
  close: () => void;
}) {
  const { accounts, addTransaction, updateTransaction } = useLifeStore();
  const [type, setType] = useState<Transaction["type"]>(value?.type ?? "expense");
  const [amount, setAmount] = useState(value?.amount ?? 0);
  const [category, setCategory] = useState(value?.category ?? "餐饮");
  const [accountId, setAccountId] = useState(
    value?.accountId ?? accounts[0]?.id ?? "",
  );
  const [toAccountId, setToAccountId] = useState(
    value?.toAccountId ??
      accounts.find((item) => item.id !== accountId)?.id ??
      "",
  );
  const [counterparty, setCounterparty] = useState(value?.counterparty ?? "");
  const [item, setItem] = useState(value?.item ?? "");
  const [occurredAt, setOccurredAt] = useState(dateTimeLocal(value?.occurredAt));
  const account = accounts.find((i) => i.id === accountId);
  const toAccount = accounts.find((i) => i.id === toAccountId);

  return (
    <ModalFrame title={value ? "编辑账单" : "手动记账"} close={close}>
      <form
        className="hx-form"
        onSubmit={async (event) => {
          event.preventDefault();
          const data = {
            type,
            amount,
            category: type === "transfer" ? "账户转账" : category,
            account: account?.name ?? "未分配",
            accountId,
            toAccount: type === "transfer" ? toAccount?.name : undefined,
            toAccountId: type === "transfer" ? toAccountId : undefined,
            counterparty:
              type === "transfer"
                ? counterparty || toAccount?.name || "账户转账"
                : counterparty,
            item,
            occurredAt: new Date(occurredAt).toISOString(),
          };
          if (value) await updateTransaction(value.id, data);
          else await addTransaction(data);
          close();
        }}
      >
        <div>
          <label>
            流水类型
            <select
              value={type}
              onChange={(event) =>
                setType(event.target.value as Transaction["type"])
              }
            >
              <option value="expense">支出</option>
              <option value="income">收入</option>
              <option value="transfer">账户转账</option>
            </select>
          </label>
          <label>
            金额
            <input
              required
              min="0.01"
              step="0.01"
              type="number"
              value={amount}
              onChange={(event) => setAmount(Number(event.target.value))}
            />
          </label>
        </div>
        {type === "transfer" ? (
          <div>
            <label>
              转出账户
              <select
                required
                value={accountId}
                onChange={(event) => setAccountId(event.target.value)}
              >
                {accounts.map((i) => (
                  <option value={i.id} key={i.id}>
                    {i.name}
                  </option>
                ))}
              </select>
            </label>
            <label>
              转入账户
              <select
                required
                value={toAccountId}
                onChange={(event) => setToAccountId(event.target.value)}
              >
                {accounts.map((i) => (
                  <option value={i.id} key={i.id}>
                    {i.name}
                  </option>
                ))}
              </select>
            </label>
          </div>
        ) : (
          <div>
            <label>
              分类
              <input
                required
                value={category}
                onChange={(event) => setCategory(event.target.value)}
              />
            </label>
            <label>
              账户
              <select
                required
                value={accountId}
                onChange={(event) => setAccountId(event.target.value)}
              >
                {accounts.map((i) => (
                  <option value={i.id} key={i.id}>
                    {i.name}
                  </option>
                ))}
              </select>
            </label>
          </div>
        )}
        <label>
          交易对象{type === "transfer" ? "（可选）" : ""}
          <input
            required={type !== "transfer"}
            value={counterparty}
            onChange={(event) => setCounterparty(event.target.value)}
          />
        </label>
        <label>
          商品 / 说明
          <input value={item} onChange={(event) => setItem(event.target.value)} />
        </label>
        <label>
          交易时间
          <input
            required
            type="datetime-local"
            value={occurredAt}
            onChange={(event) => setOccurredAt(event.target.value)}
          />
        </label>
        <footer>
          <button type="button" className="hx-btn secondary" onClick={close}>
            取消
          </button>
          <button type="submit" className="hx-btn primary">
            保存
          </button>
        </footer>
      </form>
    </ModalFrame>
  );
}

function AccountForm({
  value,
  close,
}: {
  value?: FinanceAccount;
  close: () => void;
}) {
  const { saveAccount, transactions } = useLifeStore();
  const [name, setName] = useState(value?.name ?? "");
  const [type, setType] = useState<FinanceAccount["type"]>(value?.type ?? "bank");
  const [balance, setBalance] = useState(value?.balance ?? 0);
  const [balanceAt, setBalanceAt] = useState(dateTimeLocal(value?.balanceAt));
  const [last4, setLast4] = useState(value?.last4 ?? "");
  const [color, setColor] = useState(value?.color ?? "#2a7a5e");
  const [icon, setIcon] = useState(value?.icon ?? "账");
  const parsedBalanceAt = Date.parse(balanceAt);
  const balanceAtIso = Number.isFinite(parsedBalanceAt)
    ? new Date(parsedBalanceAt).toISOString()
    : undefined;
  const previewAccount: FinanceAccount = {
    id: value?.id ?? "preview",
    userId: "local-user",
    name,
    type,
    balance,
    balanceAt: balanceAtIso,
    last4,
    color,
    icon,
    isArchived: false,
    createdAt: value?.createdAt ?? new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
  const preview = getAccountBalanceSnapshot(previewAccount, transactions);

  return (
    <ModalFrame title={value ? "编辑账户" : "添加账户"} close={close}>
      <form
        className="hx-form"
        onSubmit={async (event) => {
          event.preventDefault();
          if (!balanceAtIso) return;
          await saveAccount({
            id: value?.id,
            name,
            type,
            balance,
            balanceAt: balanceAtIso,
            last4,
            color,
            icon,
          });
          close();
        }}
      >
        <label>
          账户名称
          <input
            required
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <fieldset className="hx-balance-baseline">
          <legend>余额基准</legend>
          <p>
            填写这个时间点账户中真实存在的金额，之后的收入和支出会自动计入当前余额。
          </p>
          <div>
            <label>
              基准余额
              <input
                required
                type="number"
                step="0.01"
                value={balance ?? 0}
                onChange={(event) => setBalance(Number(event.target.value))}
              />
            </label>
            <label>
              基准时间
              <input
                required
                type="datetime-local"
                value={balanceAt}
                onChange={(event) => setBalanceAt(event.target.value)}
              />
            </label>
          </div>
          {value ? (
            <small>
              按当前账单预估余额：
              <b>
                {preview.currentBalance === null
                  ? "未设置"
                  : money(preview.currentBalance)}
              </b>
              （基准后 {preview.transactionCount} 笔）
            </small>
          ) : null}
        </fieldset>
        <div>
          <label>
            账户类型
            <select
              value={type}
              onChange={(event) =>
                setType(event.target.value as FinanceAccount["type"])
              }
            >
              <option value="bank">银行卡</option>
              <option value="wechat">微信</option>
              <option value="alipay">支付宝</option>
              <option value="cash">现金</option>
              <option value="investment">投资账户</option>
              <option value="other">其他</option>
            </select>
          </label>
          <label>
            尾号
            <input
              maxLength={4}
              value={last4}
              onChange={(event) => setLast4(event.target.value)}
            />
          </label>
        </div>
        <div>
          <label>
            标识
            <input
              maxLength={2}
              value={icon}
              onChange={(event) => setIcon(event.target.value)}
            />
          </label>
          <label>
            颜色
            <input
              type="color"
              value={color}
              onChange={(event) => setColor(event.target.value)}
            />
          </label>
        </div>
        <footer>
          <button type="button" className="hx-btn secondary" onClick={close}>
            取消
          </button>
          <button type="submit" className="hx-btn primary">
            保存
          </button>
        </footer>
      </form>
    </ModalFrame>
  );
}
