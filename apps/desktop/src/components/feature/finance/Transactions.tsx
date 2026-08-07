import { useMemo, useState } from "react";
import { Copy, NotebookPen, Pencil, Plus, Trash2 } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { Transaction } from "@/src/types";
import ContextMenu from "@/src/ui/menu/ContextMenu";
import MoreMenu from "@/src/ui/menu/MoreMenu";
import type { AppAction } from "@/src/ui/actions/types";
import { confirmAction } from "@/src/ui/feedback/confirm";
import { notify } from "@/src/ui/feedback/toastBus";
import { transactionAmountText } from "@/src/utils/format";
import { Button, SearchInput, Select } from "@/src/components/ui";

export default function Transactions({
  edit,
  note,
}: {
  edit: (value?: Transaction) => void;
  note: (value: Transaction) => void;
}) {
  const { transactions, deleteTransaction } = useLifeStore();
  const [search, setSearch] = useState("");
  const [direction, setDirection] = useState<"all" | Transaction["type"]>("all");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const rows = transactions
    .filter(
      (item) =>
        (direction === "all" || item.type === direction) &&
        `${item.counterparty ?? ""}${item.category}${item.note ?? ""}`
          .toLowerCase()
          .includes(search.toLowerCase()),
    )
    .sort(
      (left, right) =>
        new Date(right.occurredAt).getTime() - new Date(left.occurredAt).getTime(),
    );
  const selected = useMemo(
    () => rows.find((item) => item.id === selectedId) ?? rows[0] ?? null,
    [rows, selectedId],
  );

  const actionsFor = (item: Transaction): AppAction<Transaction>[] => [
    {
      id: "note",
      label: "添加消费笔记",
      icon: NotebookPen,
      group: "primary",
      execute: note,
    },
    {
      id: "copy",
      label: "复制交易摘要",
      icon: Copy,
      group: "related",
      execute: async (context) => {
        await navigator.clipboard?.writeText(
          `${context.counterparty || context.category} ${transactionAmountText(context)} ${context.account}`,
        );
        notify("交易摘要已复制");
      },
    },
    {
      id: "edit",
      label: "编辑交易",
      icon: Pencil,
      group: "organize",
      execute: edit,
    },
    {
      id: "delete",
      label: "删除交易",
      icon: Trash2,
      group: "danger",
      danger: true,
      execute: async (context) => {
        if (
          await confirmAction({
            title: "删除这笔交易？",
            description: `${context.counterparty || context.category} · ${transactionAmountText(context)}。删除后无法恢复。`,
            confirmLabel: "删除交易",
          })
        ) {
          await deleteTransaction(context.id);
          if (selectedId === context.id) setSelectedId(null);
        }
      },
    },
  ];

  return (
    <div className="hx-view">
      <div className="hx-toolbar hx-tx-tools">
        <SearchInput
          value={search}
          onChange={setSearch}
          placeholder="搜索交易对象、分类或备注"
        />
        <Select
          value={direction}
          onChange={(event) =>
            setDirection(event.target.value as "all" | Transaction["type"])
          }
          aria-label="筛选流水类型"
        >
          <option value="all">全部流水</option>
          <option value="expense">支出</option>
          <option value="income">收入</option>
          <option value="transfer">转账</option>
        </Select>
        <span className="hx-toolbar-summary">{rows.length} 笔</span>
        <Button
          variant="primary"
          icon={<Plus aria-hidden="true" />}
          onClick={() => edit()}
        >
          手动记账
        </Button>
      </div>

      <div className="lt-record-workspace">
        <section className="lt-record-list" aria-label="账单列表">
          <div className="hx-table-wrap">
            <table>
              <thead>
                <tr>
                  <th>时间</th>
                  <th>交易</th>
                  <th>分类</th>
                  <th>账户</th>
                  <th>金额</th>
                  <th aria-label="操作" />
                </tr>
              </thead>
              <tbody>
                {rows.map((item) => (
                  <ContextMenu
                    as="tr"
                    actions={actionsFor(item)}
                    context={item}
                    ariaLabel={`${item.counterparty || item.category}操作`}
                    className={selected?.id === item.id ? "selected" : undefined}
                    onClick={() => setSelectedId(item.id)}
                    onDoubleClick={() => edit(item)}
                    tabIndex={0}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") edit(item);
                      if (event.key === " ") {
                        event.preventDefault();
                        setSelectedId(item.id);
                      }
                    }}
                    key={item.id}
                  >
                    <td>
                      {new Date(item.occurredAt).toLocaleString("zh-CN", {
                        month: "2-digit",
                        day: "2-digit",
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </td>
                    <td>
                      <strong>{item.counterparty || item.category}</strong>
                      <small>{item.item || item.note || "手动记录"}</small>
                    </td>
                    <td><span className="hx-tag">{item.category}</span></td>
                    <td>
                      {item.type === "transfer"
                        ? `${item.account} → ${item.toAccount ?? "未匹配账户"}`
                        : item.account}
                    </td>
                    <td className={item.type}>{transactionAmountText(item)}</td>
                    <td className="hx-table-actions">
                      <MoreMenu
                        actions={actionsFor(item)}
                        context={item}
                        label={`${item.counterparty || item.category}更多操作`}
                      />
                    </td>
                  </ContextMenu>
                ))}
                {!rows.length ? (
                  <tr>
                    <td className="hx-table-empty" colSpan={6}>没有符合当前条件的账单。</td>
                  </tr>
                ) : null}
              </tbody>
            </table>
          </div>
        </section>

        <aside className="lt-record-inspector" aria-label="账单详情">
          {selected ? (
            <>
              <header>
                <div>
                  <small>{selected.type === "expense" ? "支出" : selected.type === "income" ? "收入" : "转账"}</small>
                  <h2>{selected.counterparty || selected.category}</h2>
                </div>
                <strong className={selected.type}>{transactionAmountText(selected)}</strong>
              </header>
              <dl>
                <div><dt>时间</dt><dd>{new Date(selected.occurredAt).toLocaleString("zh-CN")}</dd></div>
                <div><dt>分类</dt><dd>{selected.category}</dd></div>
                <div><dt>账户</dt><dd>{selected.account}</dd></div>
                {selected.toAccount ? <div><dt>转入</dt><dd>{selected.toAccount}</dd></div> : null}
                {selected.item ? <div><dt>商品</dt><dd>{selected.item}</dd></div> : null}
                {selected.note ? <div><dt>备注</dt><dd>{selected.note}</dd></div> : null}
              </dl>
              <div className="lt-inspector-actions">
                <Button variant="secondary" icon={<NotebookPen aria-hidden="true" />} onClick={() => note(selected)}>
                  添加笔记
                </Button>
                <Button variant="primary" icon={<Pencil aria-hidden="true" />} onClick={() => edit(selected)}>
                  编辑
                </Button>
              </div>
              <p className="lt-inspector-hint">单击选择 · 双击或 Enter 编辑 · 右键查看更多操作</p>
            </>
          ) : (
            <div className="lt-inspector-empty">选择一笔账单查看详情。</div>
          )}
        </aside>
      </div>
    </div>
  );
}
